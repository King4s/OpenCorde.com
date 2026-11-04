#!/usr/bin/env python3
"""Controlled Linux Docker egress acceptance; import and call smoke(directory).

Requires a running, explicitly testOnly ocqa-* installation. Uses its already
built netguard image; never pulls images or contacts public destinations. Only
uniquely labelled helper containers are created/killed/removed. Existing guards,
applications, volumes, daemon configuration and host firewall remain untouched.
Guard-death tests use disposable replicas, not the live application guards.
"""
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import time


LABEL = "io.opencorde.phase1.egress"
PORTS = {"tcp": 19080, "udp": 19081, "dns_udp": 53, "dns_tcp": 53}
SERVICES = ("backendguard", "webguard", "mediaguard")

# Packet observation records only addresses/protocol/port at this disposable
# destination. In particular, TCP SYN is evidence even if no handshake finishes.
SINK = r'''
import json, socket, threading, time
from pathlib import Path
events=Path('/tmp/packets.jsonl')
events.write_text('')
lock=threading.Lock()
def capture():
    raw=socket.socket(socket.AF_PACKET,socket.SOCK_RAW,socket.htons(3))
    raw.settimeout(.2)
    while True:
        pending=Path('/tmp/capture-heartbeat-next')
        pending.write_text(str(time.monotonic()))
        pending.replace('/tmp/capture-heartbeat')
        Path('/tmp/capture-ready').touch()
        try: frame=raw.recv(65535)
        except socket.timeout: continue
        if len(frame)<34: continue
        ether=int.from_bytes(frame[12:14],'big')
        if ether==0x0800:
            offset=14+(frame[14]&15)*4
            proto=frame[23]
            src=socket.inet_ntop(socket.AF_INET,frame[26:30])
            dst=socket.inet_ntop(socket.AF_INET,frame[30:34])
        elif ether==0x86dd and len(frame)>=54:
            offset=54; proto=frame[20]
            src=socket.inet_ntop(socket.AF_INET6,frame[22:38])
            dst=socket.inet_ntop(socket.AF_INET6,frame[38:54])
        else: continue
        if proto not in (6,17) or len(frame)<offset+20: continue
        port=int.from_bytes(frame[offset+2:offset+4],'big')
        if port not in (53,19080,19081): continue
        if proto==6 and not frame[offset+13]&2: continue
        record={'src':src,'dst':dst,'protocol':'tcp' if proto==6 else 'udp','port':port}
        with lock:
            with events.open('a') as output:
                output.write(json.dumps(record)+'\n'); output.flush()
def dns(data):
    # Valid one-question A query; authoritative answer at the test sink only.
    if len(data)<17 or data[4:6]!=b'\x00\x01': return b''
    end=12
    while end<len(data) and data[end]: end+=data[end]+1
    end+=5
    if end>len(data): return b''
    return data[:2]+b'\x81\x80\x00\x01\x00\x01\x00\x00\x00\x00'+data[12:end]+b'\xc0\x0c\x00\x01\x00\x01\x00\x00\x00\x00\x00\x04\xc0\x00\x02\x01'
def exact(conn,size):
    data=b''
    while len(data)<size:
        part=conn.recv(size-len(data))
        if not part: raise OSError('short request')
        data+=part
    return data
def serve_tcp(listener,port):
    while True:
        conn,_=listener.accept()
        with conn:
            conn.settimeout(2)
            try:
                if port==53:
                    data=exact(conn,int.from_bytes(exact(conn,2),'big'))
                    reply=dns(data); conn.sendall(len(reply).to_bytes(2,'big')+reply)
                else:
                    data=conn.recv(1024); conn.sendall(b'echo:'+data)
            except OSError: pass
def serve_udp(listener,port):
    while True:
        data,addr=listener.recvfrom(2048)
        listener.sendto(dns(data) if port==53 else b'echo:'+data,addr)
threading.Thread(target=capture,daemon=True).start()
listeners=[]
for family,host in ((socket.AF_INET,'0.0.0.0'),(socket.AF_INET6,'::')):
    for kind,port in ((socket.SOCK_STREAM,19080),(socket.SOCK_DGRAM,19081),(socket.SOCK_STREAM,53),(socket.SOCK_DGRAM,53)):
        try:
            listener=socket.socket(family,kind)
            if family==socket.AF_INET6: listener.setsockopt(socket.IPPROTO_IPV6,socket.IPV6_V6ONLY,1)
            listener.bind((host,port))
            if kind==socket.SOCK_STREAM: listener.listen(32)
        except OSError:
            if family==socket.AF_INET: raise
            continue
        listeners.append(listener)
        threading.Thread(target=serve_tcp if kind==socket.SOCK_STREAM else serve_udp,args=(listener,port),daemon=True).start()
while not Path('/tmp/capture-ready').exists(): time.sleep(.01)
Path('/tmp/ready').touch()
while True: time.sleep(30)
'''

PROBE = r'''
import json,secrets,socket,sys
args=json.loads(sys.argv[1]); mode=args['mode']; token=args['token'].encode()
family=socket.AF_INET6 if ':' in args['address'] else socket.AF_INET
kind=socket.SOCK_STREAM if mode in ('tcp','dns_tcp') else socket.SOCK_DGRAM
data=token
if mode.startswith('dns'):
    qname=args.get('qname',args['token']+'.invalid')
    query_id=secrets.token_bytes(2)
    data=query_id+b'\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00'+b''.join(bytes([len(label)])+label.encode() for label in qname.split('.'))+b'\x00\x00\x01\x00\x01'
result={'sent':False,'reply':False,'error':None}
def exact(sock,size):
    result=b''
    while len(result)<size:
        part=sock.recv(size-len(result))
        if not part: raise OSError('short response')
        result+=part
    return result
with socket.socket(family,kind) as sock:
    sock.settimeout(1.5)
    try:
        sock.connect((args['address'],args['port']))
        sock.sendall(len(data).to_bytes(2,'big')+data if mode=='dns_tcp' else data)
        result['sent']=True
        if mode=='dns_tcp': reply=exact(sock,int.from_bytes(exact(sock,2),'big'))
        elif mode=='tcp': reply=exact(sock,len(data)+5)
        else: reply=sock.recv(2048)
        if mode.startswith('dns'):
            result['reply']=len(reply)>=12 and reply[:2]==query_id and bool(reply[2]&128) and reply[3]&15==0 and int.from_bytes(reply[6:8],'big')>0
        else: result['reply']=reply==b'echo:'+token
    except OSError as error: result['error']=type(error).__name__
print(json.dumps(result))
'''

POLICY = "import json,runpy; print(json.dumps(runpy.run_path('/usr/local/bin/netguard')['current_policy']()))"

# Keep Docker's namespace/endpoints alive while the actual guard process dies.
# Killing a container PID1 instead could remove routes and produce a false pass.
SUPERVISOR = r'''
import json,subprocess,time
from pathlib import Path
child=subprocess.Popen(['python3','/usr/local/bin/netguard'])
Path('/tmp/guard-child').write_text(str(child.pid))
code=child.wait()
Path('/tmp/guard-exit').write_text(json.dumps({'returncode':code}))
while True: time.sleep(30)
'''


def _run(*args, directory=None, check=True, timeout=30):
    result = subprocess.run(args, cwd=directory, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=timeout)
    if check and result.returncode:
        # Never print environment, Docker inspect Config.Env, or service logs.
        raise RuntimeError(f"Egress QA {args[0]} operation failed ({result.returncode})")
    return result


class Harness:
    def __init__(self, directory):
        self.directory = Path(directory).resolve()
        marker = json.loads((self.directory / ".phase1-installation.json").read_text())
        self.project = marker.get("project", "")
        if marker.get("testOnly") is not True or not re.fullmatch(r"ocqa-[a-z0-9-]+", self.project):
            raise ValueError("Controlled egress QA requires an explicitly testOnly ocqa-* project")
        self.run_id = secrets.token_hex(16)
        self.owned = []
        self.records = []
        self.failure_diagnostics = None
        self.guards = {}
        for service in SERVICES:
            cid = self.compose("ps", "-q", service).decode().strip()
            if not re.fullmatch(r"[0-9a-f]{64}", cid):
                raise RuntimeError(f"Exactly one running {service} is required")
            info = self.inspect(cid)
            labels = info["Config"].get("Labels") or {}
            if labels.get("com.docker.compose.project") != self.project or labels.get("com.docker.compose.service") != service:
                raise RuntimeError("Guard does not belong to the named QA installation")
            if info["HostConfig"].get("RestartPolicy", {}).get("Name") not in ("", "no"):
                raise RuntimeError("QA requires guards without automatic restart")
            self.exec(cid, "python3", "/usr/local/bin/netguard", "health")
            self.guards[service] = info
        self.image = self.guards["backendguard"]["Image"]
        if any(g["Image"] != self.image for g in self.guards.values()):
            raise RuntimeError("Guard images differ; reconcile before QA")
        self.networks = {}
        for info in self.guards.values():
            for name in info["NetworkSettings"]["Networks"]:
                network = json.loads(_run("docker", "network", "inspect", name).stdout)[0]
                labels = network.get("Labels") or {}
                if labels.get("com.docker.compose.project") != self.project:
                    raise RuntimeError("Guard uses a network outside the QA project")
                role = labels.get("com.docker.compose.network")
                if role not in ("private", "edge"):
                    raise RuntimeError("Unexpected QA network role")
                self.networks[role] = name
        if set(self.networks) != {"private", "edge"}:
            raise RuntimeError("Both private and edge QA networks are required")

    def compose(self, *args):
        return _run("docker", "compose", "--env-file", ".env", "-f", "compose.yaml", *args,
                    directory=self.directory).stdout

    def inspect(self, cid):
        return json.loads(_run("docker", "inspect", cid).stdout)[0]

    def exec(self, cid, *args):
        return _run("docker", "exec", cid, *args).stdout

    def owned_info(self, cid):
        info = self.inspect(cid)
        labels = info["Config"].get("Labels") or {}
        if labels.get(LABEL) != self.run_id or labels.get(LABEL + ".project") != self.project:
            raise RuntimeError("Refusing to mutate a helper with different ownership labels")
        return info

    def create(self, name, *, networks=None, namespace=None, caps=(), env=None, script=None):
        name = f"oc-egress-{self.run_id[:12]}-{name}"
        args = ["docker", "create", "--pull=never", "--name", name,
                "--label", f"{LABEL}={self.run_id}", "--label", f"{LABEL}.project={self.project}",
                "--restart=no", "--read-only", "--tmpfs", "/tmp", "--cap-drop=ALL",
                "--security-opt=no-new-privileges:true"]
        for cap in caps:
            args += ["--cap-add", cap]
        args += ["--network", "container:" + namespace if namespace else networks[0]]
        if namespace is None:
            # Docker-local names still resolve, but a missing local name must
            # never fall back to the host's public recursive resolvers.
            args += ["--dns", "127.0.0.1"]
        for key, value in (env or {}).items():
            args += ["--env", f"{key}={value}"]
        args += ["--entrypoint", "python3", self.image]
        args += ["-c", script] if script is not None else ["/usr/local/bin/netguard"]
        cid = _run(*args).stdout.decode().strip()
        if not re.fullmatch(r"[0-9a-f]{64}", cid):
            raise RuntimeError("Helper creation did not return an ID")
        self.owned.append(cid)
        self.owned_info(cid)
        for network in (networks or [])[1:]:
            _run("docker", "network", "connect", network, cid)
        _run("docker", "start", cid)
        return cid, name

    def holder(self, label, guard):
        # NET_ADMIN is confined to this owned test observer for read-only nft
        # inspection after replica death. No application gains capabilities.
        return self.create(label, namespace=guard, caps=("NET_ADMIN",),
                           script="import time; time.sleep(1800)")[0]

    def save_failure(self, stage, cid=None, probe=None):
        """Retain bounded helper-only evidence before finally removes helpers.

        Never persist Docker Config.Env, application logs or arbitrary command
        arguments. Helpers contain synthetic probes and five nonsecret guard
        settings only. Verify ownership before reading their logs/state files.
        """
        if self.failure_diagnostics is not None:
            return self.failure_diagnostics
        folder = self.directory / "qa-egress-diagnostics"
        if folder.is_symlink():
            raise RuntimeError("Refusing a symlinked egress diagnostics directory")
        folder.mkdir(mode=0o700, exist_ok=True)
        folder.chmod(0o700)
        evidence = {"stage": stage, "focus_helper": cid, "helpers": []}
        if probe is not None:
            evidence["last_readiness_probe"] = {"exit_code": probe.returncode,
                "stdout": probe.stdout[-4096:].decode("utf-8", "replace"),
                "stderr": probe.stderr[-4096:].decode("utf-8", "replace")}
        helpers = [cid] if cid in self.owned else self.owned[-6:]
        for helper in helpers:
            try:
                info = self.owned_info(helper)
            except Exception:
                evidence["helpers"].append({"id": helper, "inspection": "unavailable or ownership mismatch"})
                continue
            state = info.get("State", {})
            item = {"id": helper, "name": info.get("Name"), "state": {
                key: state.get(key) for key in ("Running", "ExitCode", "OOMKilled", "StartedAt", "FinishedAt")}}
            try:
                logs = _run("docker", "logs", "--tail", "40", helper, check=False, timeout=3)
                item["logs"] = (logs.stdout + logs.stderr)[-16384:].decode("utf-8", "replace")
                if state.get("Running"):
                    status = _run("docker", "exec", helper, "python3", "-c",
                        "import json; from pathlib import Path; "
                        "print(json.dumps({name:Path('/tmp',name).read_text()[:16384] "
                        "for name in ('guard-child','guard-exit','opencorde-policy.json') "
                        "if Path('/tmp',name).is_file()}))", check=False, timeout=3)
                    item["guard_state_files"] = status.stdout[-20000:].decode("utf-8", "replace")
            except Exception:
                item["detail_capture"] = "unavailable"
            evidence["helpers"].append(item)
        path = folder / f"{self.run_id}.json"
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
        with os.fdopen(os.open(path, flags, 0o600), "w") as output:
            json.dump(evidence, output, indent=2)
            output.write("\n")
        self.failure_diagnostics = path
        return path

    def wait_ready(self, cid, guard=False):
        argv = ("python3", "/usr/local/bin/netguard", "health") if guard else (
            "python3", "-c", "from pathlib import Path; assert Path('/tmp/ready').exists()")
        deadline = time.monotonic() + 15
        probe = None
        while time.monotonic() < deadline:
            probe = _run("docker", "exec", cid, *argv, check=False)
            if probe.returncode == 0:
                return
            if not self.inspect(cid)["State"]["Running"]:
                break
            time.sleep(.1)
        name = self.owned_info(cid).get("Name", cid).lstrip("/")
        stage = "guard-policy-readiness" if guard else "sink-capture-readiness"
        path = self.save_failure(stage, cid, probe)
        raise RuntimeError(f"Egress helper {name} failed at {stage}; private diagnostics: {path}")

    def policy(self, cid):
        return json.loads(self.exec(cid, "python3", "-c", POLICY))

    def audit_dns(self, cid):
        value = self.policy(cid)
        objects = value["nftables"]
        chains = [x["chain"] for x in objects if "chain" in x]
        if not any(c.get("name") == "dns" and c.get("hook") == "output" and c.get("prio") == -150 for c in chains):
            raise RuntimeError("DNS denial does not precede Docker output DNAT")
        rules = [x["rule"] for x in objects if "rule" in x and x["rule"].get("chain") == "dns"]
        expected = (("ip", "daddr", "127.0.0.11"), ("udp", "dport", 53), ("tcp", "dport", 53))
        for protocol, field, right in expected:
            if not any(any(e.get("match", {}).get("left") == {"payload": {"protocol": protocol, "field": field}}
                           and e["match"].get("op") == "==" and e["match"].get("right") == right
                           for e in rule["expr"]) and rule["expr"][-1] == {"drop": None} for rule in rules):
                raise RuntimeError("Required early DNS denial rule is absent")
        return value

    def packets(self):
        return json.loads(self.exec(self.sink, "python3", "-c",
            "import json,time; from pathlib import Path; "
            "assert time.monotonic()-float(Path('/tmp/capture-heartbeat').read_text())<2; "
            "print(json.dumps([json.loads(x) for x in open('/tmp/packets.jsonl')]))"))

    def probe(self, cid, address, mode, *, allow, label, qname=None, observe=True):
        start = len(self.packets()) if observe else 0
        args = {"address": address, "port": PORTS[mode], "mode": mode, "token": secrets.token_hex(12)}
        if qname:
            args["qname"] = qname
        result = json.loads(self.exec(cid, "python3", "-c", PROBE, json.dumps(args)))
        # The observer runs independently of the responder. No response is not
        # evidence of containment: any SYN/datagram at the sink is a failure.
        time.sleep(.15)
        protocol = "tcp" if mode in ("tcp", "dns_tcp") else "udp"
        observed = [p for p in self.packets()[start:] if p["dst"] == address and
                    p["port"] == PORTS[mode] and p["protocol"] == protocol] if observe else []
        if allow and (not result["reply"] or (observe and not observed)):
            raise RuntimeError(f"Positive control failed: {label}/{mode}")
        if not allow and (result["reply"] or observed or result["error"] is None):
            raise RuntimeError(f"Egress escaped or probe was inconclusive: {label}/{mode}")
        self.records.append({"case": label, "protocol": mode, "expected": "reachable" if allow else "blocked",
                             "status": "passed", "sink_observed": bool(observed) if observe else None})

    def targets(self, guard):
        sink_networks = self.inspect(self.sink)["NetworkSettings"]["Networks"]
        for network, endpoint in guard["NetworkSettings"]["Networks"].items():
            for key in ("IPAddress", "GlobalIPv6Address"):
                if endpoint.get(key) and sink_networks[network].get(key):
                    yield network, sink_networks[network][key]

    def matrix(self, cid, guard, label):
        for network, address in self.targets(guard):
            version = ipaddress.ip_address(address).version
            network_role = next(role for role, name in self.networks.items() if name == network)
            for mode in PORTS:
                self.probe(self.control, address, mode, allow=True, label=f"control/{label}/{network_role}/v{version}")
                self.probe(cid, address, mode, allow=False, label=f"{label}/{network_role}/v{version}")
        for mode in ("dns_udp", "dns_tcp"):
            # Query only a Docker-local helper name. No external resolver or
            # public query is involved; exact early-rule audit complements this.
            self.probe(self.control, "127.0.0.11", mode, allow=True,
                       label=f"control/{label}/embedded-dns", qname=self.sink_name, observe=False)
            self.probe(cid, "127.0.0.11", mode, allow=False,
                       label=f"{label}/embedded-dns", qname=self.sink_name, observe=False)

    def replica(self, service, suffix, overrides=None):
        original = self.guards[service]
        allowed = {"GUARD_ROLE", "BACKEND_IP", "MEDIA_IP", "MEDIA_EGRESS", "MEDIA_CLIENT_CIDRS"}
        env = {key: value for key, value in (item.split("=", 1) for item in original["Config"]["Env"])
               if key in allowed}
        env.update(overrides or {})
        cid, _ = self.create(f"{service}-{suffix}", networks=list(original["NetworkSettings"]["Networks"]),
                             caps=("NET_ADMIN",), env=env, script=SUPERVISOR)
        self.wait_ready(cid, guard=True)
        return cid

    def allowed_connection(self, cid, service):
        env = dict(item.split("=", 1) for item in self.guards[service]["Config"]["Env"])
        address, port = ((env["MEDIA_IP"], 7880) if service == "backendguard" else
                         (env["BACKEND_IP"], 3000 if service == "webguard" else 6379))
        self.exec(cid, "python3", "-c",
                  "import json,socket,sys; a=json.loads(sys.argv[1]); "
                  "s=socket.create_connection(tuple(a),timeout=3); s.close()",
                  json.dumps([address, port]))

    def kill_guard_process(self, cid):
        self.owned_info(cid)
        self.exec(cid, "python3", "-c",
                  "import os,signal; from pathlib import Path; "
                  "pid=int(Path('/tmp/guard-child').read_text()); assert pid>1; "
                  "assert b'/usr/local/bin/netguard' in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\\0'); "
                  "os.kill(pid,signal.SIGKILL)")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            result = _run("docker", "exec", cid, "python3", "-c",
                          "import json; from pathlib import Path; "
                          "assert json.loads(Path('/tmp/guard-exit').read_text())['returncode']==-9", check=False)
            if result.returncode == 0:
                return
            time.sleep(.1)
        raise RuntimeError("Guard child death was not confirmed by its supervisor")

    def cleanup(self):
        failures = []
        for cid in reversed(self.owned):
            try:
                self.owned_info(cid)
                _run("docker", "rm", "--force", cid)
            except Exception:
                failures.append(cid)
        if failures:
            raise RuntimeError("Owned egress helper cleanup failed; retained IDs: " + ", ".join(failures))


def smoke(directory):
    """Return JSON-safe evidence; raise on failed control, leakage or cleanup.

    Run sequentially, before backup acceptance. Do not wrap this in a timeout
    shorter than the complete matrix (typically several minutes). No source
    edits, daemon restart, host reboot, public traffic or application stop occurs.
    """
    h = Harness(directory)
    try:
        h.sink, h.sink_name = h.create("sink", networks=list(h.networks.values()),
                                      caps=("NET_RAW", "NET_BIND_SERVICE"), script=SINK)
        h.wait_ready(h.sink)
        h.control, _ = h.create("control", networks=list(h.networks.values()),
                                script="import time; time.sleep(1800)")
        live_policies = {}
        ipv6 = {}
        for service, info in h.guards.items():
            live_policies[service] = h.audit_dns(info["Id"])
            ipv6[service] = {"status": "exercised" if any(":" in address for _, address in h.targets(info)) else "not_exercised",
                             "reason": "Requires IPv6 addresses on guard, sink and positive-control network"}
            holder = h.holder(f"live-{service}", info["Id"])
            h.matrix(holder, info, f"live/{service}")

        for service in SERVICES:
            replica = h.replica(service, "death")
            info = h.owned_info(replica)
            holder = h.holder(f"held-{service}", replica)
            before = h.audit_dns(replica)
            h.allowed_connection(holder, service)
            h.matrix(holder, info, f"replica-before/{service}")
            h.kill_guard_process(replica)
            if not h.inspect(replica)["State"]["Running"] or not h.inspect(holder)["State"]["Running"]:
                raise RuntimeError("Supervisor and consumer must hold the network after guard death")
            if h.policy(holder) != before:
                raise RuntimeError("Namespace firewall changed after guard process death")
            h.allowed_connection(holder, service)
            h.matrix(holder, info, f"replica-after-kill/{service}")

        sink_ips = h.inspect(h.sink)["NetworkSettings"]["Networks"]
        private_ip = sink_ips[h.networks["private"]]["IPAddress"]
        edge_ip = sink_ips[h.networks["edge"]]["IPAddress"]
        media = h.replica("mediaguard", "optin", {"MEDIA_EGRESS": "true", "MEDIA_CLIENT_CIDRS": private_ip + "/32"})
        observer = h.holder("media-optin-probe", media)
        h.audit_dns(media)
        for mode in PORTS:
            h.probe(h.control, private_ip, mode, allow=True, label="control/media-optin")
            h.probe(observer, private_ip, mode, allow=mode == "udp", label="media-optin/allowed-destination")
        h.probe(h.control, edge_ip, "udp", allow=True, label="control/media-optin-denied-cidr")
        h.probe(observer, edge_ip, "udp", allow=False, label="media-optin/denied-cidr")
        for service in SERVICES:
            if h.policy(h.guards[service]["Id"]) != live_policies[service]:
                raise RuntimeError("Live policy changed during isolated replica tests")
            holder = h.holder(f"optin-isolation-{service}", h.guards[service]["Id"])
            h.probe(holder, private_ip, "udp", allow=False, label=f"media-optin/live-isolation/{service}")
        result = {"status": "passed", "project": h.project, "guard_image_id": h.image,
                  "helper_run_id": h.run_id, "checks": h.records, "ipv6": ipv6,
                  "guard_process_death": "guard child SIGKILL on same-image supervised replicas; allowed internal TCP and consumers remain live",
                  "media_optin": "only replica UDP to the single allowed /32; live policies unchanged",
                  "embedded_dns": "valid Docker-local A queries denied; exact pre-DNAT DNS rules checked",
                  "limits": ["No public Internet or host-forwarded external DNS query sent",
                             "No host reboot, Docker daemon restart, container PID1 death, or live guard recreation tested",
                             "Controlled destinations test namespace policy, not every host routing/firewall topology",
                             "Packet observer handles direct IPv4/IPv6 TCP/UDP, not IPv6 extension headers"]}
    except Exception as error:
        # Capture before cleanup, including failures outside readiness (e.g.
        # process-death or traffic probes). Do not replace the original cause.
        try:
            path = h.save_failure("egress-smoke")
        except Exception:
            raise RuntimeError("Egress QA failed and private diagnostic capture also failed") from error
        raise RuntimeError(f"{error}; helper evidence: {path}") from error
    finally:
        h.cleanup()
    return result
