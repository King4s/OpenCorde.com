#!/usr/bin/env python3
"""Disposable Linux container acceptance. Run only through the remote QA runner."""
import argparse
import base64
import hashlib
import hmac
import importlib.util
import http.client
import ipaddress
import json
import os
from pathlib import Path
import pwd
import re
import secrets
import shutil
import socket
import ssl
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlsplit


SOURCE = Path(__file__).resolve().parent.parent
RUN_ID = re.compile(r"[0-9a-f]{32}\Z")
HASH = re.compile(r"[0-9a-f]{64}\Z")
TLS = ssl._create_unverified_context()  # Only the generated, loopback QA certificate.
FEATURE_LIMITS = {
    "attachment_avatar_display": "blocked: private bucket has no authorized download route with channel provenance",
    "host_reboot_restart": "not tested: no host reboot or daemon restart in isolated QA",
    "media_end_to_end": "not tested: no external media session in isolated QA",
    "guard_interruption": "supervised replicas tested below; container teardown and host reboot are not covered",
}


def utc_now():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def hardware_summary():
    memory_kib = None
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                memory_kib = int(line.split()[1])
                break
    except OSError:
        pass
    disk = shutil.disk_usage(SOURCE)
    return {"host": socket.gethostname(), "qa_host": os.environ.get("OPENCORDE_QA_HOST", "unspecified"),
            "cpu_count": os.cpu_count(), "memory_kib": memory_kib,
            "disk_free_gib_at_start": round(disk.free / 1024**3, 2)}


def write_report(report):
    directory = SOURCE / "reports"
    directory.mkdir(mode=0o700, exist_ok=True)
    path = directory / "phase1-runtime.json"
    if path.is_symlink():
        raise RuntimeError("Refusing a symlinked QA report")
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600), "w") as output:
        json.dump(report, output, indent=2, sort_keys=True)
        output.write("\n")


def mark(report, name, status, **details):
    report["checks"][name] = {"status": status, **details}
    write_report(report)


def timed_check(report, name, action):
    started = time.monotonic()
    try:
        value = action()
    except Exception:
        mark(report, name, "failed", seconds=round(time.monotonic() - started, 1))
        raise
    mark(report, name, "passed", seconds=round(time.monotonic() - started, 1))
    return value


def private_log(path, data):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as output:
        output.write(data)


def build_log_tail(path):
    with path.open("rb") as source:
        source.seek(0, os.SEEK_END)
        source.seek(max(0, source.tell() - 8192))
        lines = source.read().decode("utf-8", "replace").splitlines()
    for line in reversed(lines):
        if line.strip():
            return line.strip()[-160:]
    return "waiting for build output"


def command(*args, cwd=SOURCE, capture=False, env=None):
    result = subprocess.run(args, cwd=cwd, env=env,
                            stdout=subprocess.PIPE if capture else None,
                            stderr=subprocess.PIPE if capture else None,
                            check=False)
    if result.returncode:
        if capture:
            logs = SOURCE / "qa-command-logs"
            logs.mkdir(mode=0o700, exist_ok=True)
            log = logs / f"failure-{secrets.token_hex(8)}.log"
            # Error output may contain application details. Keep it private and
            # bounded, and never print arguments that could hold configuration.
            output = (result.stdout or b"") + b"\n--- stderr ---\n" + (result.stderr or b"")
            private_log(log, output[-2 * 1024 * 1024:])
            raise RuntimeError(f"{args[0]} operation failed (exit {result.returncode}); private log: {log}")
        raise RuntimeError(f"{args[0]} operation failed (exit {result.returncode})")
    return result.stdout if capture else b""


def ops(directory, *args):
    return command("python3", "scripts/ops.py", *args, cwd=directory, capture=True)


def compose(directory, *args):
    return command("docker", "compose", "--env-file", ".env", "-f", "compose.yaml",
                   *args, cwd=directory, capture=True)


def compose_argv(*args):
    return ("docker", "compose", "--env-file", ".env", "-f", "compose.yaml", *args)


def rejected(argv, directory, reason):
    result = subprocess.run(argv, cwd=directory, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode == 0 or reason not in result.stderr.decode("utf-8", "replace").lower():
        logs = SOURCE / "qa-command-logs"
        logs.mkdir(mode=0o700, exist_ok=True)
        log = logs / f"unexpected-{secrets.token_hex(8)}.log"
        private_log(log, ((result.stdout or b"") + b"\n--- stderr ---\n" + (result.stderr or b""))[-2 * 1024 * 1024:])
        raise RuntimeError(f"Expected rejection did not match ({result.returncode}); private log: {log}")


def identity_hash(directory):
    output = compose(directory, "exec", "-T", "api", "sha256sum", "/data/server.key").decode().split()
    if len(output) < 2 or output[1] != "/data/server.key" or not HASH.fullmatch(output[0]):
        raise RuntimeError("Identity key hash was not returned by the API container")
    return output[0]


def object_bytes(directory, path):
    if not re.fullmatch(r"/files/opencorde/attachments/[0-9a-f-]{36}/proof\.txt", path):
        raise RuntimeError("Unexpected attachment object path")
    path = path.removeprefix("/files")
    # Curl signs with credentials expanded inside the MinIO container. Neither
    # the command line nor the report receives the credential values.
    script = ('exec curl --fail --silent --show-error '
              '--aws-sigv4 "aws:amz:us-east-1:s3" '
              '--user "$MINIO_ROOT_USER:$MINIO_ROOT_PASSWORD" '
              '"http://127.0.0.1:9000$1"')
    return compose(directory, "exec", "-T", "minio", "sh", "-c", script, "sh", path)


def verify_objects(directory):
    result = json.loads(compose(directory, "run", "--rm", "--no-deps", "maintenance", "verify-objects"))
    if not result.get("ok") or result.get("checked_objects", 0) < 1 or not HASH.fullmatch(result.get("manifest_sha256", "")):
        raise RuntimeError("S3 object verification did not cover the QA upload")
    return {"checked_objects": result["checked_objects"],
            "verified_bytes": result["verified_bytes"], "manifest_sha256": result["manifest_sha256"]}


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def unused_subnets():
    ids = command("docker", "network", "ls", "-q", capture=True).decode().split()
    used = []
    if ids:
        networks = json.loads(command("docker", "network", "inspect", *ids, capture=True))
        for network in networks:
            for item in network.get("IPAM", {}).get("Config") or []:
                try:
                    used.append(ipaddress.ip_network(item["Subnet"], strict=False))
                except (KeyError, ValueError):
                    pass
    for octet in range(100, 240):
        candidate = ipaddress.ip_network(f"172.30.{octet}.0/24")
        if not any(candidate.overlaps(other) for other in used):
            yield str(candidate)


def build_images(tag, report):
    # No prune, pull of mutable tags, or change to host-wide Docker settings.
    if shutil.disk_usage(SOURCE).free < 14 * 1024**3:
        raise RuntimeError("QA host has less than 14 GiB free for isolated image builds")
    env = os.environ.copy()
    env["DOCKER_BUILDKIT"] = "1"
    env["CARGO_BUILD_JOBS"] = "2"
    images = {name: f"opencorde-qa-{name}:{tag}" for name in ("api", "web", "netguard", "minio")}
    build_evidence = {}
    for name, image in images.items():
        log = SOURCE / f"qa-build-{name}.log"
        tag_preexisting = subprocess.run(("docker", "image", "inspect", image),
                                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
        report["build"][name] = {"status": "running", "tag_preexisting": tag_preexisting,
                                  "log": str(log)}
        write_report(report)
        print(f"Building {image}; live log: {log}", flush=True)
        with os.fdopen(os.open(log, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as output:
            process = subprocess.Popen(("docker", "build", "--progress=plain", "--build-arg", "CARGO_BUILD_JOBS=4", "--tag", image,
                                        "--file", f"Dockerfile.{name}", "."),
                                       cwd=SOURCE, env=env, stdout=output, stderr=subprocess.STDOUT)
            started = time.monotonic()
            while process.poll() is None:
                time.sleep(20)
                if process.poll() is None:
                    print(f"Building {image}: {int(time.monotonic() - started)}s; {build_log_tail(log)}; log {log}", flush=True)
            if process.returncode:
                report["build"][name]["status"] = "failed"
                report["build"][name]["seconds"] = round(time.monotonic() - started, 1)
                write_report(report)
                raise RuntimeError(f"Image build failed ({name}, exit {process.returncode}); private log: {log}")
        print(f"Built {image} in {int(time.monotonic() - started)}s", flush=True)
        build_evidence[name] = {"status": "passed", "seconds": round(time.monotonic() - started, 1),
                                "tag_preexisting": tag_preexisting,
                                "cached_build_steps_observed": log.read_text(errors="replace").count(" CACHED"),
                                "log": str(log)}
        report["build"][name] = build_evidence[name]
        write_report(report)
    return images, build_evidence


def make_install(name, images, subnet, port):
    directory = SOURCE / name
    directory.mkdir(mode=0o700)
    (directory / "scripts").mkdir(mode=0o700)
    for relative in ("compose.yaml", "scripts/ops.py"):
        target = directory / relative
        shutil.copyfile(SOURCE / relative, target)
    (directory / "images.json").write_text(json.dumps(images), encoding="utf-8")
    certs = directory / "qa-cert-input"
    certs.mkdir(mode=0o700)
    command("openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
            "-subj", "/CN=localhost", "-addext", "subjectAltName=DNS:localhost,DNS:turn.localhost",
            "-keyout", str(certs / "privkey.pem"),
            "-out", str(certs / "fullchain.pem"), cwd=directory, capture=True)
    os.chmod(certs / "privkey.pem", 0o600)
    ops(directory, "prepare", "--images", "images.json", "--domain", f"localhost:{port}",
        "--turn-domain", "turn.localhost",
        "--public-ip", "127.0.0.1", "--tls-dir", "qa-cert-input", "--project", name,
        "--private-subnet", subnet, "--https-port", str(port), "--test-local-images")
    return directory


def api(port, method, path, *, token=None, body=None, content_type="application/json", headers=None):
    headers = {"Accept": "application/json", **(headers or {})}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if body is not None:
        if not isinstance(body, bytes):
            body = json.dumps(body).encode()
        headers["Content-Type"] = content_type
    conn = http.client.HTTPSConnection("127.0.0.1", port, context=TLS, timeout=15)
    try:
        conn.request(method, path, body=body, headers=headers)
        response = conn.getresponse()
        data = response.read(8 * 1024 * 1024)
        return response.status, data
    finally:
        conn.close()


def expect_json(port, method, path, expected, **kwargs):
    status, raw = api(port, method, path, **kwargs)
    if status != expected:
        raise RuntimeError(f"{method} {path} returned HTTP {status}, expected {expected}")
    return json.loads(raw)


def ready(port):
    for _ in range(90):
        try:
            status, _ = api(port, "GET", "/api/v1/health")
            if status == 200:
                return
        except (OSError, ssl.SSLError, http.client.HTTPException):
            pass
        time.sleep(2)
    raise RuntimeError("Loopback HTTPS health timed out")


def verify_proxy_log_privacy(directory, port):
    # Exercise signaling with disposable values. Never copy container logs to the report.
    query, referrer = (secrets.token_hex(18) for _ in range(2))
    status, _ = api(port, "GET", f"/livekit/rtc?access_token={query}",
                    headers={"Referer": f"https://fixture.invalid/?token={referrer}"})
    if not 400 <= status < 500:
        raise RuntimeError(f"Unexpected response to invalid signaling credential (HTTP {status})")
    entry = f"route=media method=GET status={status} ".encode()
    for _ in range(12):
        logs = compose(directory, "logs", "--no-color", "--tail", "300", "web")
        if any(value.encode() in logs for value in (query, referrer)):
            raise RuntimeError("Web container logs exposed a signaling query or referrer")
        if entry in logs:
            return
        time.sleep(0.25)
    raise RuntimeError("Missing access record for signaling privacy probe")


def websocket(port, token):
    key = base64.b64encode(secrets.token_bytes(16)).decode()
    raw = socket.create_connection(("127.0.0.1", port), timeout=10)
    sock = TLS.wrap_socket(raw, server_hostname="localhost")
    sock.settimeout(10)
    try:
        request = ("GET /api/v1/gateway HTTP/1.1\r\nHost: localhost\r\n"
                   "Connection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\n"
                   f"Sec-WebSocket-Key: {key}\r\n\r\n")
        sock.sendall(request.encode())
        reply = b""
        while b"\r\n\r\n" not in reply and len(reply) < 8192:
            reply += sock.recv(1)
        if not reply.startswith(b"HTTP/1.1 101 "):
            raise RuntimeError("WebSocket upgrade failed")
        hello = read_frame(sock)
        if hello.get("type") != "Hello":
            raise RuntimeError("WebSocket Hello missing")
        send_frame(sock, json.dumps({"type": "Identify", "data": {"token": token}}).encode())
        response = read_frame(sock)
        if response.get("type") != "Ready":
            raise RuntimeError("WebSocket Ready missing")
    finally:
        sock.close()


def recv_exact(sock, n):
    data = b""
    while len(data) < n:
        part = sock.recv(n - len(data))
        if not part:
            raise RuntimeError("WebSocket closed early")
        data += part
    return data


def read_frame(sock):
    first, second = recv_exact(sock, 2)
    if first & 0x0f != 1 or second & 0x80:
        raise RuntimeError("Unexpected WebSocket frame")
    length = second & 0x7f
    if length == 126:
        length = int.from_bytes(recv_exact(sock, 2), "big")
    elif length == 127:
        length = int.from_bytes(recv_exact(sock, 8), "big")
    if length > 65536:
        raise RuntimeError("Oversized WebSocket frame")
    return json.loads(recv_exact(sock, length))


def send_frame(sock, payload):
    mask = secrets.token_bytes(4)
    length = len(payload)
    prefix = bytes([0x81, 0x80 | length]) if length < 126 else bytes([0x81, 0xfe]) + length.to_bytes(2, "big")
    sock.sendall(prefix + mask + bytes(byte ^ mask[i % 4] for i, byte in enumerate(payload)))


def egress_checks(directory):
    spec = importlib.util.spec_from_file_location("phase1_egress", SOURCE / "tests/ops/phase1_egress.py")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    return helper.smoke(directory)


def initial_smoke(directory, port):
    ready(port)
    config = dict(line.split("=", 1) for line in (directory / ".env").read_text().splitlines())
    suffix = secrets.token_hex(5)
    email = f"phase1qa_{suffix}@example.invalid"
    password = secrets.token_urlsafe(24)
    username = f"phase1qa_{suffix}"
    auth = expect_json(port, "POST", "/api/v1/auth/register", 201,
                       body={"email": email, "username": username, "password": password,
                             "invite_code": config["REGISTRATION_INVITE_CODE"]})
    login = expect_json(port, "POST", "/api/v1/auth/login", 200,
                        body={"email": email, "password": password})
    token = login["access_token"]
    if login["user"]["id"] != auth["user"]["id"]:
        raise RuntimeError("Login identity differs from registration")
    server = expect_json(port, "POST", "/api/v1/servers", 201, token=token,
                         body={"name": f"QA {suffix}"})
    channels = expect_json(port, "GET", f"/api/v1/servers/{server['id']}/channels", 200, token=token)
    channel = next(item for item in channels if item["name"] == "general")
    message_text = f"phase1 recovery {suffix}"
    message = expect_json(port, "POST", f"/api/v1/channels/{channel['id']}/messages", 201,
                          token=token, body={"content": message_text})
    listed = expect_json(port, "GET", f"/api/v1/channels/{channel['id']}/messages", 200, token=token)
    if not any(item["id"] == message["id"] and item["content"] == message_text for item in listed):
        raise RuntimeError("Text message missing from channel")
    payload = f"Isolated Phase 1 QA object {suffix}\n".encode()
    boundary = "phase1qa" + suffix
    multipart = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"proof.txt\"\r\n"
                 "Content-Type: text/plain\r\n\r\n").encode() + payload + f"\r\n--{boundary}--\r\n".encode()
    attachment = expect_json(port, "POST", f"/api/v1/channels/{channel['id']}/attachments", 201,
                             token=token, body=multipart, content_type=f"multipart/form-data; boundary={boundary}")
    path = urlsplit(attachment["url"]).path
    status, _ = api(port, "GET", path)
    if status != 403:
        raise RuntimeError(f"Private bucket anonymous GET returned HTTP {status}, expected 403")
    if object_bytes(directory, path) != payload:
        raise RuntimeError("Authorized S3 read differs from uploaded QA bytes")
    verified = verify_objects(directory)
    websocket(port, token)
    egress = egress_checks(directory)
    return {"email": email, "password": password, "user_id": auth["user"]["id"],
            "server_id": server["id"], "channel_id": channel["id"], "message_id": message["id"],
            "message_text": message_text, "object_id": attachment["id"], "object_path": path,
            "object_sha256": hashlib.sha256(payload).hexdigest(), "objects_verification": verified,
            "egress": egress}


def media_smoke(directory, port):
    """Use short-lived, room-scoped synthetic identities; never report credentials."""
    runtime = Path(os.environ["OPENCORDE_QA_MEDIA_RUNTIME"])
    if not runtime.is_absolute() or runtime.is_symlink() or not runtime.is_dir():
        raise RuntimeError("Media QA runtime is not a regular absolute directory")
    for parent in runtime.parents:
        if parent.is_symlink():
            raise RuntimeError("Media QA runtime has a symlinked ancestor")
    config = dict(line.split("=", 1) for line in (directory / ".env").read_text().splitlines())
    room = "phase1-media-" + secrets.token_hex(8)
    def encoded(value):
        return base64.urlsafe_b64encode(json.dumps(value, separators=(",", ":")).encode()).rstrip(b"=")
    def token(identity):
        now = int(time.time())
        message = encoded({"alg": "HS256", "typ": "JWT"}) + b"." + encoded({
            "iss": config["LIVEKIT_API_KEY"], "sub": identity, "nbf": now - 5, "exp": now + 600,
            "video": {"roomJoin": True, "room": room, "canPublish": True, "canSubscribe": True}})
        signature = hmac.new(config["LIVEKIT_API_SECRET"].encode(), message, hashlib.sha256).digest()
        return (message + b"." + base64.urlsafe_b64encode(signature).rstrip(b"=")).decode()
    env = os.environ.copy()
    env.update(PHASE1_LIVEKIT_URL=f"wss://localhost:{port}/livekit",
               PHASE1_MEDIA_TOKEN_A=token("probe-a"), PHASE1_MEDIA_TOKEN_B=token("probe-b"),
               PHASE1_MEDIA_MODE="both", PHASE1_TURN_HOST=config["TURN_DOMAIN"],
               PHASE1_TURN_TLS_PORT=str(port), PHASE1_MEDIA_INSECURE_TLS="1",
               PHASE1_LIVEKIT_BUNDLE=str(runtime / "livekit-client.umd.js"),
               PLAYWRIGHT_BROWSERS_PATH=str(runtime / "browsers"))
    with tempfile.TemporaryDirectory(prefix="session-", dir=runtime) as temporary:
        helper = Path(temporary) / "phase1-media.mjs"
        shutil.copyfile(SOURCE / "tests/ops/phase1-media.mjs", helper)
        runner = json.loads((runtime / "runner.json").read_text())
        image = runner.get("image", "")
        if not re.fullmatch(r"mcr\.microsoft\.com/playwright@sha256:[0-9a-f]{64}", image):
            raise RuntimeError("Media QA browser image must be pinned to its official digest")
        env["PLAYWRIGHT_BROWSERS_PATH"] = "/ms-playwright"
        # Pass names, not values, to Docker; room tokens stay out of argv/logs.
        owner = secrets.token_hex(16)
        container_name = f"ocqa-media-{owner}"
        argv = ["docker", "run", "--rm", "--pull=never", "--network=host",
                "--name", container_name, "--label", f"opencorde.qa.media={owner}",
                "--add-host", f"{config['TURN_DOMAIN']}:127.0.0.1",
                "--user", f"{os.getuid()}:{os.getgid()}", "--cap-drop=ALL",
                "--security-opt=no-new-privileges", "--read-only", "--tmpfs", "/tmp:rw,size=1g",
                "--shm-size=256m", "--mount", f"type=bind,src={runtime},dst={runtime},readonly",
                "--workdir", str(runtime)]
        for name in ("PHASE1_LIVEKIT_URL", "PHASE1_MEDIA_TOKEN_A", "PHASE1_MEDIA_TOKEN_B",
                     "PHASE1_MEDIA_MODE", "PHASE1_TURN_HOST", "PHASE1_TURN_TLS_PORT",
                     "PHASE1_MEDIA_INSECURE_TLS", "PHASE1_LIVEKIT_BUNDLE", "PLAYWRIGHT_BROWSERS_PATH"):
            argv.extend(("--env", name))
        argv.extend((image, "node", str(helper)))
        try:
            output = command(*argv, cwd=runtime, env=env, capture=True)
        finally:
            inspected = subprocess.run(("docker", "inspect", container_name),
                                       stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            if inspected.returncode == 0:
                item = json.loads(inspected.stdout)[0]
                if (item.get("Name") != f"/{container_name}" or
                        (item.get("Config", {}).get("Labels") or {}).get("opencorde.qa.media") != owner):
                    raise RuntimeError("Media helper ownership changed; refusing cleanup")
                command("docker", "rm", "--force", item["Id"], capture=True)
        result = json.loads(output)
    if result.get("status") != "passed":
        raise RuntimeError("Two-client media acceptance did not pass")
    return result


def restored_smoke(directory, port, evidence):
    ready(port)
    login = expect_json(port, "POST", "/api/v1/auth/login", 200,
                        body={"email": evidence["email"], "password": evidence["password"]})
    if login["user"]["id"] != evidence["user_id"]:
        raise RuntimeError("Restored login identity differs")
    token = login["access_token"]
    server = expect_json(port, "GET", f"/api/v1/servers/{evidence['server_id']}", 200, token=token)
    if server["id"] != evidence["server_id"]:
        raise RuntimeError("Restored server differs")
    listed = expect_json(port, "GET", f"/api/v1/channels/{evidence['channel_id']}/messages", 200, token=token)
    if not any(item["id"] == evidence["message_id"] and item["content"] == evidence["message_text"] for item in listed):
        raise RuntimeError("Restored message differs")
    status, _ = api(port, "GET", evidence["object_path"])
    if status != 403:
        raise RuntimeError(f"Restored private bucket anonymous GET returned HTTP {status}, expected 403")
    downloaded = object_bytes(directory, evidence["object_path"])
    if hashlib.sha256(downloaded).hexdigest() != evidence["object_sha256"]:
        raise RuntimeError("Restored object SHA256 differs")
    restored_objects = verify_objects(directory)
    if restored_objects != evidence["objects_verification"]:
        raise RuntimeError("Restored object manifest differs")
    websocket(port, token)
    query = evidence["message_text"]
    result = json.loads(compose(directory, "run", "--rm", "--no-deps", "maintenance",
                                "search-verify", query))
    if not result.get("ok") or result.get("matched_messages", 0) < 1:
        raise RuntimeError("Restored search index did not contain QA message")
    egress = egress_checks(directory)
    return {"restored_object_sha256": hashlib.sha256(downloaded).hexdigest(),
            "objects_verification": restored_objects, "search_matches": result["matched_messages"],
            "anonymous_attachment_get_http": 403, "egress": egress}


def assert_owned_oneoff(container_id, project):
    if not HASH.fullmatch(container_id):
        raise RuntimeError("Invalid detached QA container ID")
    inspected = json.loads(command("docker", "inspect", container_id, capture=True))
    if len(inspected) != 1 or inspected[0].get("Id") != container_id:
        raise RuntimeError("Detached QA container identity changed")
    labels = inspected[0].get("Config", {}).get("Labels") or {}
    if (labels.get("com.docker.compose.project") != project or
            labels.get("com.docker.compose.service") != "maintenance" or
            str(labels.get("com.docker.compose.oneoff", "")).lower() != "true"):
        raise RuntimeError("Detached container does not carry expected QA labels")


def oneoff_writer_rejection(directory, project, port):
    container_id = compose(directory, "run", "--detach", "--no-deps", "--entrypoint", "sleep",
                           "maintenance", "300").decode().strip()
    assert_owned_oneoff(container_id, project)
    try:
        negative_backup = SOURCE / f"negative-backup-{project}"
        rejected(("python3", "scripts/ops.py", "backup", "--output", str(negative_backup)), directory,
                 "one-off or unclassified project container")
        if negative_backup.exists():
            raise RuntimeError("Rejected backup created an output directory")
        status, _ = api(port, "GET", "/api/v1/health")
        if status != 200 or not compose(directory, "ps", "--status", "running", "-q", "api").strip():
            raise RuntimeError("API did not remain running after rejected backup")
    finally:
        assert_owned_oneoff(container_id, project)
        command("docker", "rm", "--force", container_id, capture=True)


def assert_owned_volume(name, project):
    inspected = json.loads(command("docker", "volume", "inspect", name, capture=True))
    if len(inspected) != 1 or inspected[0].get("Name") != name:
        raise RuntimeError("QA volume identity changed")
    labels = inspected[0].get("Labels") or {}
    if (labels.get("com.docker.compose.project") != project or
            labels.get("com.docker.compose.volume") != "postgres"):
        raise RuntimeError("QA volume labels changed")


def nonempty_restore_rejection(directory, project, backup):
    config = json.loads(compose(directory, "config", "--format", "json"))
    if config.get("name") != project:
        raise RuntimeError("Restore target project identity changed")
    volume = config["volumes"]["postgres"]["name"]
    if not re.fullmatch(re.escape(project) + r"_postgres", volume):
        raise RuntimeError("Unexpected restore target volume name")
    command("docker", "volume", "create", "--label", f"com.docker.compose.project={project}",
            "--label", "com.docker.compose.volume=postgres", volume, capture=True)
    assert_owned_volume(volume, project)
    try:
        rejected(("python3", "scripts/ops.py", "restore", "--backup", str(backup)),
                 directory, "target volumes must not exist")
        if compose(directory, "ps", "-aq").strip():
            raise RuntimeError("Rejected restore created target containers")
        assert_owned_volume(volume, project)
        for key in ("objects", "identity", "search"):
            other = config["volumes"][key]["name"]
            if subprocess.run(("docker", "volume", "inspect", other),
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
                raise RuntimeError("Rejected restore created another target volume")
    finally:
        assert_owned_volume(volume, project)
        command("docker", "volume", "rm", volume, capture=True)


def guard_start_failure(directory, project):
    original = (directory / "compose.yaml").read_bytes()
    marker = b"GUARD_ROLE: backend"
    if original.count(marker) != 1:
        raise RuntimeError("Cannot isolate backend guard configuration")
    stop_owned(directory, project)
    try:
        (directory / "compose.yaml").write_bytes(original.replace(marker, b"GUARD_ROLE: invalid", 1))
        result = subprocess.run(compose_argv("up", "-d", "--wait", "--wait-timeout", "90"), cwd=directory,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if result.returncode == 0:
            raise RuntimeError("Invalid backend guard configuration unexpectedly started")
        guard_id = compose(directory, "ps", "-aq", "backendguard").decode().strip()
        if not HASH.fullmatch(guard_id):
            raise RuntimeError("Invalid guard test did not create an inspectable guard")
        inspected = json.loads(command("docker", "inspect", guard_id, capture=True))[0]
        labels = inspected.get("Config", {}).get("Labels") or {}
        if labels.get("com.docker.compose.project") != project:
            raise RuntimeError("Invalid guard test inspected another project")
        output = subprocess.run(("docker", "logs", guard_id), cwd=directory,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        logs = (output.stdout + output.stderr).decode("utf-8", "replace")
        if "invalid GUARD_ROLE" not in logs:
            raise RuntimeError("Guard did not reject the invalid role")
        for service in ("postgres", "redis", "minio", "api"):
            if compose(directory, "ps", "--status", "running", "-q", service).strip():
                raise RuntimeError(f"{service} started behind an unhealthy backend guard")
    finally:
        # Only this prepared test installation is touched, and its source
        # compose file is restored even if the negative case fails.
        try:
            stop_owned(directory, project)
        finally:
            (directory / "compose.yaml").write_bytes(original)


def stop_owned(directory, name):
    marker = json.loads((directory / ".phase1-installation.json").read_text())
    if marker.get("project") != name or marker.get("testOnly") is not True or not name.startswith("ocqa-"):
        raise RuntimeError("Refusing to stop a project without matching QA identity")
    compose(directory, "stop", "--timeout", "45")


def remove_owned(directory, name):
    marker = json.loads((directory / ".phase1-installation.json").read_text())
    if marker.get("project") != name or marker.get("testOnly") is not True or not name.startswith("ocqa-"):
        raise RuntimeError("Refusing to remove a project without matching QA identity")
    ids = compose(directory, "ps", "-aq").decode().split()
    if ids:
        for container in json.loads(command("docker", "inspect", *ids, capture=True)):
            labels = container.get("Config", {}).get("Labels") or {}
            if labels.get("com.docker.compose.project") != name:
                raise RuntimeError("QA project inventory contains another project's container")
    compose(directory, "down", "--volumes", "--timeout", "45")


def run(source_hash, run_id):
    if not HASH.fullmatch(source_hash) or not RUN_ID.fullmatch(run_id):
        raise ValueError("Invalid QA run identity")
    if os.geteuid() == 0 or pwd.getpwuid(os.geteuid()).pw_name != "mb":
        raise RuntimeError("Run only as the QA host's unprivileged mb user")
    started = time.monotonic()
    report = {"format": 1, "started_at_utc": utc_now(), "source_sha256": source_hash,
              "run_id": run_id, "status": "running", "release_ready": False,
              "hardware": hardware_summary(), "build": {}, "checks": {},
              "feature_limits": FEATURE_LIMITS,
              "verified_scope": [],
              "pending_scope": ["host reboot or Docker daemon restart", "external media session",
                                "guard process interruption while application namespace remains live"]}
    report["checks"]["guard_process_interruption"] = {"status": "pending", "reason": FEATURE_LIMITS["guard_interruption"]}
    write_report(report)
    try:
        tag = source_hash[:16]
        images, builds = timed_check(report, "image_build", lambda: build_images(tag, report))
        report["build"] = builds
        write_report(report)
        subnets = unused_subnets()
        subnet_a, subnet_b = next(subnets), next(subnets)
        port_a = free_port()
        port_b = free_port()
        while port_b == port_a:
            port_b = free_port()
        primary_name = f"ocqa-{run_id[:12]}-a"
        restore_name = f"ocqa-{run_id[:12]}-b"
        report["projects"] = [primary_name, restore_name]
        primary = timed_check(report, "prepare_source", lambda: make_install(primary_name, images, subnet_a, port_a))
        print(f"Starting isolated project {primary_name}", flush=True)
        timed_check(report, "source_start", lambda: ops(primary, "up"))
        timed_check(report, "proxy_log_privacy", lambda: verify_proxy_log_privacy(primary, port_a))
        source_identity = timed_check(report, "source_identity_hash", lambda: identity_hash(primary))
        evidence = timed_check(report, "source_api_storage_and_egress", lambda: initial_smoke(primary, port_a))
        report["checks"]["source_api_storage_and_egress"]["objects"] = evidence["objects_verification"]
        report["checks"]["source_api_storage_and_egress"]["egress"] = evidence["egress"]
        report["checks"]["guard_process_interruption"] = {
            "status": "passed_on_supervised_replicas", "evidence": evidence["egress"]["guard_process_death"]}
        report["checks"]["anonymous_attachment_get"] = {"status": "blocked_as_expected", "http_status": 403}
        write_report(report)
        media = timed_check(report, "two_client_media", lambda: media_smoke(primary, port_a))
        report["checks"]["two_client_media"]["evidence"] = media
        write_report(report)
        timed_check(report, "oneoff_writer_backup_rejection", lambda: oneoff_writer_rejection(primary, primary_name, port_a))
        backup = SOURCE / f"backup-{run_id}"
        timed_check(report, "coordinated_backup", lambda: ops(primary, "backup", "--output", str(backup)))
        print("Source installation backed up and verified", flush=True)
        timed_check(report, "invalid_guard_start_rejection", lambda: guard_start_failure(primary, primary_name))
        restored = timed_check(report, "prepare_restore", lambda: make_install(restore_name, images, subnet_b, port_b))
        timed_check(report, "nonempty_restore_rejection", lambda: nonempty_restore_rejection(restored, restore_name, backup))
        timed_check(report, "restore", lambda: ops(restored, "restore", "--backup", str(backup)))
        timed_check(report, "restored_start", lambda: ops(restored, "up"))
        restored_identity = timed_check(report, "restored_identity_hash", lambda: identity_hash(restored))
        if restored_identity != source_identity:
            raise RuntimeError("Restored identity key SHA256 differs from source")
        mark(report, "identity_key_preserved", "passed", sha256=restored_identity)
        restored_evidence = timed_check(report, "restored_api_storage_search_and_egress",
                                        lambda: restored_smoke(restored, port_b, evidence))
        report["checks"]["restored_api_storage_search_and_egress"].update(restored_evidence)
        write_report(report)
        timed_check(report, "cleanup_restore", lambda: remove_owned(restored, restore_name))
        timed_check(report, "cleanup_source", lambda: remove_owned(primary, primary_name))
        report["verified_scope"] = ["isolated image builds", "loopback HTTPS health and authentication",
                                     "text message persistence", "attachment upload and authorized S3 bytes",
                                     "private bucket anonymous GET denied", "WebSocket identify",
                                     "controlled TCP, UDP and DNS egress probes with positive controls",
                                     "guard process-death replicas and media opt-in isolation",
                                     "two-client synthetic audio/video including remapped TURN/TLS",
                                     "one-off writer rejection without API outage",
                                     "coordinated backup and restore", "nonempty target refusal before restore mutation",
                                     "identity key SHA256 preserved", "restored object SHA256 and manifest",
                                     "search reindex query", "invalid backend guard prevents consumers starting"]
        report["status"] = "operational_checks_passed"
        report["completed_at_utc"] = utc_now()
        report["duration_seconds"] = round(time.monotonic() - started, 1)
        write_report(report)
        print(json.dumps({"operational_checks_passed": True, "release_ready": False,
                          "source_sha256": source_hash, "report": str(SOURCE / "reports/phase1-runtime.json")}), flush=True)
    except Exception as error:
        report["status"] = "failed"
        report["failure"] = str(error)
        report["completed_at_utc"] = utc_now()
        report["duration_seconds"] = round(time.monotonic() - started, 1)
        write_report(report)
        raise


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["run"])
    parser.add_argument("--source-hash", required=True)
    parser.add_argument("--run-id", required=True)
    args = parser.parse_args()
    try:
        run(args.source_hash, args.run_id)
    except Exception as error:
        print(f"QA FAILED: {error}; snapshot retained at {SOURCE}", file=sys.stderr)
        sys.exit(1)
