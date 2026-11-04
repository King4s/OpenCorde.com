#!/usr/bin/env python3
"""Phase 1 operator commands. Python 3 + Docker Compose; no host compiler needed."""
import argparse
import contextlib
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tarfile
import time
import uuid

ROOT = Path(__file__).resolve().parent.parent
SERVICES = {"backendguard", "webguard", "mediaguard", "volume-init", "postgres", "redis", "minio", "api", "maintenance", "web", "livekit"}
DATA = ("postgres", "objects", "identity", "search")
BACKUP_READ_SERVICES = {"postgres", "minio", "backendguard", "webguard", "mediaguard"}


def run(argv, *, data=None, output=None, check=True):
    p = subprocess.run(argv, input=data, stdout=output or subprocess.PIPE,
                       stderr=subprocess.PIPE, cwd=ROOT)
    if check and p.returncode:
        # Commands can contain credentials through Compose; never echo argv or
        # raw service logs on failure. Operator can inspect their own local logs.
        raise RuntimeError(f"{argv[0]} operation failed (exit {p.returncode})")
    return p


def dc(*args, **kwargs):
    return run(["docker", "compose", "--env-file", str(ROOT / ".env"), "-f", str(ROOT / "compose.yaml"), *args], **kwargs)


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def write_private(path, value, mode=0o600):
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode), "w") as f:
        f.write(value)


@contextlib.contextmanager
def operation_lock():
    # flock is released after a crash; an old empty lock file does not block recovery.
    import fcntl
    with (ROOT / ".operations.lock").open("a") as f:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield


def refuse_pending_restore():
    # lstat also detects dangling links. Any error other than a missing marker
    # must stop startup rather than silently treating its state as clear.
    try:
        (ROOT / ".restore-pending.json").lstat()
    except FileNotFoundError:
        return
    raise RuntimeError("Incomplete restore. Keep this target offline and restore into a fresh installation.")


def record_restore_state(checkpoint, *, verified):
    pending = ROOT / ".restore-pending.json"
    flags = os.O_WRONLY | os.O_NOFOLLOW
    if verified:
        flags |= os.O_TRUNC  # The pending pathname continues to block startup.
        record = dict(checkpoint, state="verified", completedAt=int(time.time()))
    else:
        flags |= os.O_CREAT | os.O_EXCL
        record = dict(checkpoint, state="pending")
    with os.fdopen(os.open(pending, flags, 0o600), "w") as output:
        json.dump(record, output, indent=2)
        output.flush()
        os.fsync(output.fileno())
    if verified:
        # A single same-directory rename publishes success and releases the
        # startup block. A crash before it leaves even a verified payload blocked.
        os.replace(pending, ROOT / ".restore-verified.json")
    directory = os.open(ROOT, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def installation():
    marker = json.loads((ROOT / ".phase1-installation.json").read_text())
    config = json.loads(dc("config", "--format", "json").stdout)
    if config["name"] != marker["project"] or set(config["services"]) - SERVICES:
        raise RuntimeError("Installation identity or service inventory changed; reconcile before operating")
    # Fail closed if another project/container also holds any target data volume.
    ids = run(["docker", "ps", "-aq"]).stdout.decode().split()
    volumes = {v["name"] for v in config.get("volumes", {}).values()}
    if ids:
        for c in json.loads(run(["docker", "inspect", *ids]).stdout):
            shared = any(m.get("Name") in volumes for m in c.get("Mounts", []))
            labels = c["Config"].get("Labels") or {}
            if shared and labels.get("com.docker.compose.project") != marker["project"]:
                raise RuntimeError("A volume is attached to a container outside this installation")
    return marker, config


def prepare(args):
    if (ROOT / ".env").exists() or (ROOT / ".phase1-installation.json").exists():
        raise RuntimeError("Refusing to replace existing installation configuration")
    if not re.fullmatch(r"[a-z0-9][a-z0-9_-]{2,48}", args.project):
        raise ValueError("Invalid project name")
    if not re.fullmatch(r"[A-Za-z0-9.-]+(?::[0-9]{1,5})?", args.domain):
        raise ValueError("Invalid domain")
    app_host = args.domain.split(":")[0].lower()
    turn_domain = (getattr(args, "turn_domain", None) or f"turn.{app_host}").lower()
    labels = turn_domain.split(".")
    if (len(turn_domain) > 253 or len(labels) < 2 or
            any(not re.fullmatch(r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?", label) for label in labels)):
        raise ValueError("TURN domain must be a DNS hostname without a port")
    try:
        ipaddress.ip_address(turn_domain)
    except ValueError:
        pass
    else:
        raise ValueError("TURN domain must be a DNS hostname, not an IP address")
    if turn_domain == app_host:
        raise ValueError("TURN domain must differ from the application hostname for TLS SNI routing")
    if not 1 <= args.https_port <= 65535:
        raise ValueError("HTTPS port must be between 1 and 65535")
    if not args.test_local_images and (args.https_port != 443 or
                                      (":" in args.domain and args.domain.rsplit(":", 1)[1] != "443")):
        raise ValueError("Production HTTPS and advertised TURN/TLS must share public port 443")
    public_ip = str(ipaddress.IPv4Address(args.public_ip))
    # Only disposable loopback QA needs local ICE sockets and a restricted-peer
    # exception. Keep production TURN's private/loopback peer protections intact.
    test_loopback = args.test_local_images and ipaddress.IPv4Address(public_ip).is_loopback
    loopback_rtc = "  enable_loopback_candidate: true\n" if test_loopback else ""
    loopback_turn = f'  allow_restricted_peer_cidrs: ["{public_ip}/32"]\n' if test_loopback else ""
    subnet = ipaddress.IPv4Network(args.private_subnet, strict=True)
    if subnet.prefixlen != 24 or not subnet.is_private:
        raise ValueError("Private network must be an RFC1918 /24 subnet")
    backend_ip, web_ip, media_ip = (str(subnet[n]) for n in (10, 20, 30))
    images = json.loads(Path(args.images).read_text())
    values = {}
    for name in ("api", "web", "netguard", "minio"):
        ref = images[name]
        if not re.fullmatch(r"[a-zA-Z0-9./_:-]+@sha256:[0-9a-f]{64}", ref):
            if not (args.test_local_images and re.fullmatch(r"opencorde-qa-[a-z0-9_-]+:[a-z0-9_-]+", ref)):
                raise ValueError(f"{name} must be a digest-pinned image")
        values[name.upper() + "_IMAGE"] = ref
    values.update(COMPOSE_PROJECT_NAME=args.project, DOMAIN=args.domain, TURN_DOMAIN=turn_domain,
                  PUBLIC_IP=public_ip, PUBLIC_REGISTRY_SYNC="false", MEDIA_EGRESS="false",
                  REGISTRATION_MODE="invite_only", HTTPS_PORT=str(args.https_port),
                  BIND_IP="127.0.0.1" if args.test_local_images else "0.0.0.0",
                  PRIVATE_SUBNET=str(subnet), BACKEND_IP=backend_ip, WEB_IP=web_ip, MEDIA_IP=media_ip)
    for key in ("POSTGRES_PASSWORD", "JWT_SECRET", "MINIO_ACCESS_KEY", "MINIO_SECRET_KEY",
                "LIVEKIT_API_KEY", "LIVEKIT_API_SECRET", "REGISTRATION_INVITE_CODE"):
        values[key] = secrets.token_hex(32)
    secrets_dir = ROOT / "secrets"
    secrets_dir.mkdir(mode=0o700, exist_ok=False)
    tls = secrets_dir / "tls"
    tls.mkdir(mode=0o755)
    # prepare runs with umask 077: chmod is required to permit traversal by
    # both container UIDs through this dedicated bind-mount root. The host
    # secrets directory stays 0700, protecting access from other host users.
    tls.chmod(0o755)
    for name in ("fullchain.pem", "privkey.pem"):
        source = Path(args.tls_dir) / name
        if not source.is_file() or source.is_symlink():
            raise ValueError("Supply regular TLS certificate and key files")
        shutil.copyfile(source, tls / name)
        # The enclosing host directory is 0700; the two unprivileged TLS
        # consumers have different UIDs and read only this dedicated mount.
        (tls / name).chmod(0o444)
    livekit = f'''port: 7880
log_level: info
rtc:
  node_ip: {public_ip}
  use_external_ip: false
{loopback_rtc}  tcp_port: 7881
  port_range_start: 50000
  port_range_end: 50100
  # Explicit local STUN prevents LiveKit's public fallback for TLS-only clients.
  stun_servers: ["{public_ip}:3478"]
redis:
  address: {backend_ip}:6379
keys:
  {values["LIVEKIT_API_KEY"]}: {values["LIVEKIT_API_SECRET"]}
turn:
  enabled: true
{loopback_turn}  domain: {turn_domain}
  tls_port: 5349
  udp_port: 3478
  proxy_protocol: true
  proxy_protocol_trusted_cidrs: ["{web_ip}/32"]
  cert_file: /etc/opencorde/tls/fullchain.pem
  key_file: /etc/opencorde/tls/privkey.pem
'''
    write_private(secrets_dir / "livekit.yaml", livekit, 0o444)
    # The process umask is 077, so creation alone leaves this bind-mounted
    # config at 0400. LiveKit runs as UID 10001 and needs read access.
    (secrets_dir / "livekit.yaml").chmod(0o444)
    write_private(ROOT / ".env", "".join(f"{k}={v}\n" for k, v in values.items()))
    write_private(ROOT / ".phase1-installation.json", json.dumps({"version": 1, "project": args.project, "id": uuid.uuid4().hex, "testOnly": args.test_local_images}, indent=2))
    print("Configuration prepared; secrets are in the private .env and secrets directory")


def up(_args):
    refuse_pending_restore()
    installation()
    dc("up", "-d", "--wait", "backendguard", "postgres", "redis", "minio", "volume-init")
    dc("run", "--rm", "--no-deps", "maintenance", "storage-init")
    dc("up", "-d", "--wait")
    print("Installation services started")


def archive_volume(config, key, target):
    image = config["services"]["backendguard"]["image"]
    volume = config["volumes"][key]["name"]
    with target.open("xb") as f:
        run(["docker", "run", "--rm", "--network", "none", "--read-only", "--cap-drop", "ALL",
             "--cap-add", "DAC_READ_SEARCH", "--entrypoint", "tar", "-v", f"{volume}:/source:ro",
             image, "-C", "/source", "-cf", "-", "."], output=f)


def project_running_containers(project):
    """Compose ps omits one-off jobs; query Docker's project labels directly."""
    ids = run(["docker", "ps", "--filter", f"label=com.docker.compose.project={project}", "-q"]).stdout.decode().split()
    if not ids:
        return []
    containers = json.loads(run(["docker", "inspect", *ids]).stdout)
    return [c for c in containers if c.get("State", {}).get("Running") is True]


def backup_services(containers, project, configured_services, *, stopped=False):
    """Validate the complete project inventory before any writer stop/capture.

    One-off jobs and unknown services require operator reconciliation. Never stop
    them implicitly: they may be an independent restore or maintenance operation.
    """
    running = set()
    for container in containers:
        labels = container.get("Config", {}).get("Labels") or {}
        service = labels.get("com.docker.compose.service")
        if labels.get("com.docker.compose.project") != project:
            raise RuntimeError("Unexpected project container; backup stopped")
        if str(labels.get("com.docker.compose.oneoff", "")).lower() != "false":
            raise RuntimeError("A one-off or unclassified project container is running; backup stopped")
        if service not in SERVICES or service not in configured_services:
            raise RuntimeError("An unexpected project service is running; backup stopped")
        if stopped and service not in BACKUP_READ_SERVICES:
            raise RuntimeError("A writer is still running; backup stopped")
        running.add(service)
    return sorted(running)


def assert_backup_barrier(marker, config, *, objects_stopped=False):
    running = backup_services(project_running_containers(marker["project"]), marker["project"],
                              config["services"], stopped=True)
    if objects_stopped and "minio" in running:
        raise RuntimeError("Object storage is still running; archive capture stopped")


def backup(args):
    marker, config = installation()
    output = Path(args.output).resolve()
    if output.exists():
        raise RuntimeError("Backup destination must not exist")
    running = backup_services(project_running_containers(marker["project"]),
                              marker["project"], config["services"])
    output.mkdir(mode=0o700, parents=True)
    paused = [s for s in running if s not in BACKUP_READ_SERVICES]
    # Stop ingress, all API replicas and detached background jobs. Waiting for
    # Docker stop completion (including forced kill) establishes the write barrier.
    try:
        if paused:
            dc("stop", "--timeout", "45", *paused)
        assert_backup_barrier(marker, config)
        count = dc("exec", "-T", "postgres", "psql", "-U", "opencorde", "-d", "opencorde", "-Atc",
                   "SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND backend_type='client backend'").stdout.strip()
        if count != b"0":
            raise RuntimeError("Database still has client sessions; backup stopped")
        verified = dc("run", "--rm", "--no-deps", "maintenance", "verify-objects").stdout
        (output / "objects-verification.json").write_bytes(verified)
        assert_backup_barrier(marker, config)
        with (output / "database.dump").open("xb") as f:
            dc("exec", "-T", "postgres", "pg_dump", "-U", "opencorde", "-d", "opencorde", "-Fc", "--no-owner", "--no-acl", output=f)
        dc("stop", "--timeout", "45", "minio")
        assert_backup_barrier(marker, config, objects_stopped=True)
        archive_volume(config, "objects", output / "objects.tar")
        archive_volume(config, "identity", output / "identity.tar")
        # Search is intentionally rebuilt from the restored database; Redis is
        # ephemeral and is reset. Never represent a stale async index as atomic.
        shutil.copyfile(ROOT / ".env", output / "installation.env")
        shutil.copytree(ROOT / "secrets", output / "secrets")
        shutil.copyfile(ROOT / "compose.yaml", output / "compose.yaml")
        payload = {str(p.relative_to(output)).replace(os.sep, "/"): digest(p)
                   for p in output.rglob("*") if p.is_file()}
        manifest = {"format": 1, "installation": marker, "project": config["name"],
                    "images": {k: v["image"] for k, v in config["services"].items()},
                    "files": payload, "search": "rebuild", "redis": "reset", "complete": True}
        write_private(output / "manifest.json", json.dumps(manifest, indent=2))
        for file in output.rglob("*"):
            file.chmod(0o700 if file.is_dir() else 0o600)
        print(f"Coordinated backup complete: {output}")
    finally:
        # Resume exactly the pre-backup running services, including MinIO.
        if running:
            dc("start", *running)


def validate_backup(path):
    if path.is_symlink():
        raise ValueError("Backup directory cannot be a symlink")
    manifest = json.loads((path / "manifest.json").read_text())
    if manifest.get("format") != 1 or manifest.get("complete") is not True:
        raise ValueError("Incomplete or unsupported backup")
    required = {"database.dump", "objects.tar", "identity.tar", "objects-verification.json", "installation.env", "compose.yaml"}
    if not required.issubset(manifest.get("files", {})):
        raise ValueError("Incomplete backup file inventory")
    for name, expected in manifest["files"].items():
        target = path / name
        if Path(name).is_absolute() or ".." in Path(name).parts or target.is_symlink() or not target.resolve().is_relative_to(path.resolve()):
            raise ValueError("Unsafe backup path")
        if digest(target) != expected:
            raise ValueError("Backup checksum mismatch")
    for name in ("objects.tar", "identity.tar"):
        with tarfile.open(path / name) as archive:
            for member in archive:
                if member.name.startswith("/") or ".." in Path(member.name).parts or not (member.isfile() or member.isdir()):
                    raise ValueError("Unsafe volume archive entry")
    return manifest


def restore(args):
    refuse_pending_restore()
    source = Path(args.backup).resolve()
    manifest = validate_backup(source)
    marker, config = installation()
    if marker["id"] == manifest["installation"]["id"]:
        raise RuntimeError("Restore requires a new installation identity")
    if dc("ps", "-aq").stdout.strip():
        raise RuntimeError("Restore target must have no containers")
    for key in DATA:
        if run(["docker", "volume", "inspect", config["volumes"][key]["name"]], check=False).returncode == 0:
            raise RuntimeError("Restore target volumes must not exist")
    # Require exact images: raw object volume formats may not survive upgrades.
    for name, image in manifest["images"].items():
        if config["services"].get(name, {}).get("image") != image:
            raise RuntimeError("Restore must use the backup's exact images")
    if os.path.lexists(ROOT / ".restore-verified.json"):
        raise RuntimeError("Restore requires a fresh installation without a previous verification record")
    checkpoint = {"targetInstallationId": marker["id"], "startedAt": int(time.time()),
                  "backupManifestSha256": digest(source / "manifest.json")}
    # This file and its directory entry must be durable before the first Docker
    # mutation. The operation lock serializes restore and systemd/operator up.
    record_restore_state(checkpoint, verified=False)
    # Target's fresh secrets/URLs remain explicit; identity and data are restored.
    # PostgreSQL dump contains users/sessions; old sessions are invalidated by
    # the target JWT secret. MinIO owns its existing object data independently.
    stage = "volume initialization"
    try:
        # MinIO is offline during extraction. Without that dependent service,
        # Compose --wait treats the completed initializer as a failed daemon.
        dc("run", "--rm", "--no-deps", "volume-init")
        stage = "database startup"
        dc("up", "-d", "--wait", "backendguard", "postgres")
        image = config["services"]["backendguard"]["image"]
        for key in ("objects", "identity"):
            stage = f"{key} archive extraction"
            volume = config["volumes"][key]["name"]
            with (source / f"{key}.tar").open("rb") as f:
                p = subprocess.run(["docker", "run", "--rm", "-i", "--network", "none", "--read-only",
                                    "--cap-drop", "ALL", "--cap-add", "CHOWN", "--cap-add", "DAC_OVERRIDE",
                                    "--cap-add", "FOWNER", "--entrypoint", "tar", "-v", f"{volume}:/target",
                                    image, "-C", "/target", "-xf", "-"], stdin=f, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                if p.returncode:
                    raise RuntimeError("Volume restore failed")
        stage = "database import"
        with (source / "database.dump").open("rb") as f:
            p = subprocess.run(["docker", "compose", "--env-file", str(ROOT / ".env"), "-f", str(ROOT / "compose.yaml"),
                                "exec", "-T", "postgres", "pg_restore", "-U", "opencorde", "-d", "opencorde",
                                "--exit-on-error", "--no-owner", "--no-acl"], stdin=f, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if p.returncode:
                raise RuntimeError("Database restore failed; target remains offline")
        stage = "object store startup"
        dc("up", "-d", "--wait", "minio", "redis")
        stage = "object verification"
        restored = dc("run", "--rm", "--no-deps", "maintenance", "verify-objects").stdout
        if json.loads(restored) != json.loads((source / "objects-verification.json").read_bytes()):
            raise RuntimeError("Restored objects differ from the checkpoint")
        stage = "search rebuild"
        dc("run", "--rm", "--no-deps", "maintenance", "reindex")
        stage = "restore verification record"
        record_restore_state(checkpoint, verified=True)
        print("Restore verified; application ingress remains stopped. Run up after reviewing the target configuration.")
    except Exception as error:
        dc("stop", "api", "web", "livekit", check=False)
        raise RuntimeError(f"Restore failed during {stage} ({error})") from error


def main():
    os.umask(0o077)
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="command", required=True)
    prep = sub.add_parser("prepare")
    prep.add_argument("--images", required=True)
    prep.add_argument("--domain", required=True)
    prep.add_argument("--turn-domain", help="Distinct TLS SNI hostname; defaults to turn.<application host>")
    prep.add_argument("--public-ip", required=True)
    prep.add_argument("--tls-dir", required=True)
    prep.add_argument("--project", default="opencorde")
    prep.add_argument("--https-port", type=int, default=443)
    prep.add_argument("--private-subnet", default="172.30.90.0/24")
    prep.add_argument("--test-local-images", action="store_true", help=argparse.SUPPRESS)
    sub.add_parser("up")
    sub.add_parser("backup").add_argument("--output", required=True)
    sub.add_parser("restore").add_argument("--backup", required=True)
    args = p.parse_args()
    with operation_lock():
        {"prepare": prepare, "up": up, "backup": backup, "restore": restore}[args.command](args)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
