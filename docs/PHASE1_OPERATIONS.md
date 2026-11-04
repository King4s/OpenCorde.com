# Phase 1 operations

Status: implementation under validation. No production promotion has occurred.
Do not interpret Dockerfiles or the image workflow as evidence that a release has
already been published. Use only the `release-images.json` artifact from a
successful, reviewed server-image workflow.

## Web dependency lock

`client/pnpm-lock.yaml` is the authoritative released web dependency lock for
the Phase 1 images, CI and Tauri. Use Node 22, Corepack-pinned pnpm 10.32.1,
and `pnpm install --frozen-lockfile --ignore-scripts`. The retired npm lock is
preserved byte-for-byte at `reports/legacy-client-package-lock.json` for source
provenance; it is not used for dependency installation. The root
`package-lock.json` belongs to the separate Playwright test boundary.
The server-image workflow publishes SHA-addressed image artifacts on the
Phase 1 branch for validation; it does not deploy them to production.

## Supported installation boundary

Linux Docker Engine with the Compose plugin and Python 3. Docker builds run in
the release pipeline, not on the installation host. The initial image download
requires network access. Runtime services cannot initiate external connections
by default. HTTPS replies are permitted; this is not a claim of zero outbound
bytes. The first supported networking configuration is IPv4, with IPv6 output
denied inside service namespaces.

The production entry point is **compose.yaml**. The older docker-compose.yml
is retained as historical development configuration and must not be used for
this installation. The operator script always names compose.yaml explicitly.

Prepare two DNS names pointing to the same public IPv4: the application name
and a distinct TURN name (default `turn.<application hostname>`). Obtain a trusted
TLS certificate whose subject alternative names cover **both names**. Place
`fullchain.pem` and `privkey.pem` in a private staging directory. Public ACME
automation is not enabled by this package. Certificate renewal is an explicit
operator task; renew the mounted files and restart web and LiveKit together.
LiveKit clients require a trusted certificate for production use; test-only
self-signed certificates do not establish public browser media acceptance.

```sh
python3 scripts/ops.py prepare \
  --images release-images.json \
  --domain chat.example.org \
  --turn-domain turn.chat.example.org \
  --public-ip 203.0.113.10 \
  --tls-dir /private/opencorde-certificate \
  --project opencorde
python3 scripts/ops.py up
```

Use the real public IPv4 address in place of the documentation address. The
private subnet defaults to 172.30.90.0/24; use `--private-subnet` for a different
unused private /24. Inspect existing Docker and host networks first. Generated
credentials are stored in `.env`; the registration invitation code is there too.
Registration defaults to invite-only. Never commit this file or the secrets
directory. Existing production data is not imported by these commands.

Only HTTPS and media ports are published. PostgreSQL, Redis, the MinIO API and
console, and the API HTTP port have no host port mappings. Application data
services share an internal network namespace; they are not mutually hostile
tenants. Network guards alone receive NET_ADMIN, scoped to their container
namespace. They have no Docker socket or host networking.

## Privacy and media

`PUBLIC_REGISTRY_SYNC=false` is enforced by the backend. The packaged backend
also sets `ALLOW_EXTERNAL_INTEGRATIONS=false`: registry, federation, SMTP,
push, external previews, Steam and command integrations are disabled. Setting
an API flag alone cannot bypass the network firewall. Enabling external
integrations needs a separately reviewed network configuration; arbitrary
internet access is not an advertised configuration of this release.

Every guard installs a dual-family nftables OUTPUT/FORWARD default-drop policy
before dependent services start. DNS is denied before Docker's output DNAT,
including its embedded resolver. The backend uses loopback for its data
services; the edge proxy uses configured fixed private addresses. No public
STUN lookup or automatic external-IP discovery is enabled.

To enable consented media connections, set `MEDIA_EGRESS=true` in `.env` and
recreate the media guard and LiveKit as a pair. This permits UDP to configured
`MEDIA_CLIENT_CIDRS` only in the media namespace. Keep client CIDRs as narrow
as practical for private deployments. HTTPS and backend namespaces remain
blocked. The public ports are TCP 443 (HTTPS and TURN/TLS), RTC TCP 7881,
UDP 50000–50100 and TURN/STUN UDP 3478. TURN uses LiveKit's authenticated embedded
server. Nginx examines TLS SNI on 443: `TURN_DOMAIN` passes encrypted TCP to
LiveKit's internal port 5349; other names pass to a loopback HTTPS listener.
Port 5349 is never published. Both upstreams require a PROXY header to preserve
the actual client address. HTTPS trusts only the loopback proxy; LiveKit trusts
only the configured web guard's private `/32`. TURN TLS terminates at LiveKit,
so both services mount the same certificate covering both DNS names.

The pinned LiveKit advertises TURN/TLS port 443 regardless of its internal TLS
listen port. Production `prepare` therefore requires public HTTPS port 443.
Test-local port remapping is allowed for isolated HTTP/TLS checks, but does not
prove the advertised public TURN endpoint works. `rtc.stun_servers` explicitly
names this installation's public IPv4 on UDP 3478, avoiding the public STUN
fallback that an empty list can select for TLS-only clients. Keep the published
TURN UDP port at 3478 to match this generated configuration.

SNI routing requires clients that send the configured TURN hostname in their
TLS handshake. Validate an authenticated TURN allocation and a real media session
through port 443 before claiming restrictive-network media acceptance; a TLS
handshake alone proves only routing and certificate presentation.

Do not recreate a guard while leaving its consumers running in an old network
namespace. Stop the complete installation before network/guard configuration
changes, then start it through the operator command. Guard process failure
does not remove existing firewall rules, but Docker health failures do not
automatically stop dependent containers.

All engine restart policies are `no`. This prevents Docker daemon startup from
racing application startup against installation of the firewall. For boot
startup, install the supplied `deploy/opencorde.service` unit with the release
under `/opt/opencorde`, then enable that unit. It invokes the ordered operator
startup. Service crashes require operator recovery: stop the affected stack and
run `ops.py up` after diagnosis. Direct `docker start/restart` bypasses the
supported startup procedure. Automatic crash recovery is not implemented by
this release. Isolated stop/start tests do not establish an actual host-reboot test.

## Coordinated backup

```sh
python3 scripts/ops.py backup --output /private/backups/opencorde-2026-11-03
```

This is an offline coordinated checkpoint with downtime. The command locks
operations, verifies project/volume ownership, stops web/API/media writers and
waits for their processes to exit. The API includes scheduled-message and
asynchronous background work, so stopping it is part of the write barrier.
Other database sessions cause refusal. It verifies each referenced object,
dumps PostgreSQL, stops MinIO, and archives objects and the server identity.
It also saves configuration, TLS material and exact image references. Checksums
are finalized only after all artifacts exist. Previously running services are
resumed even if capture fails; an incomplete directory is not a valid backup.

Backup directories contain secrets and personal data. They are mode 0700 with
0600 files; filesystem permissions are not encryption. Store copies on protected
storage under the operator's existing encryption/retention policy. The script
does not upload backups to an external provider or delete old backups.

Unreferenced objects from interrupted uploads are retained. Search is rebuilt
from PostgreSQL on restore because live indexing is asynchronous and a cold
index copy could be stale. Redis is intentionally ephemeral and resets.

## Restore into a fresh installation

Prepare a second installation directory with the same release images, a new
project name, unused private subnet and its intended domain/certificate. Do
not run `up` first. Then:

```sh
python3 scripts/ops.py restore --backup /private/backups/opencorde-2026-11-03
python3 scripts/ops.py up
```

Restore refuses the original installation identity, any existing target
containers or volumes, mismatched images, incomplete/checksum-invalid backups,
and unsafe archive paths/links. PostgreSQL and object data are restored while
ingress stays stopped. The restored object inventory must match the recorded
checkpoint. Tantivy is rebuilt in a new directory from existing plaintext
messages, excluding encrypted channels. Only after those steps pass is a local
restore-verification record written. Restore creates `.restore-pending.json`
before changing any volumes. While this file exists, `ops.py up` refuses to
start the installation, including on a systemd restart. If restore fails or
is interrupted, keep that target offline for diagnosis and retry into a newly
prepared installation. A completed restore atomically replaces the pending
marker with `.restore-verified.json`.

New installation credentials and URLs remain explicit; the server identity
key is restored. A new JWT secret invalidates previous access tokens. The
backed-up configuration is available for deliberate recovery, not silently
executed or overlaid onto a new host. Verify login, membership-filtered search,
message history and referenced objects before directing users to the target.

## Upgrade and rollback

Take and verify a coordinated backup first. Record the current image manifest.
Stop all services before replacing image pins or guard configuration, retain
the previous manifest, then start the updated installation. Never use mutable
`latest` tags. Phase 1 preserves database migrations through 073; it does not
automatically promote the separately frozen security candidate to production.

If the new images fail and no schema/data transformation occurred, stop all
services, restore the previous image pins and start them again. If a later
release changes schema or stored formats, restore its pre-upgrade checkpoint
into a fresh installation with matching old images. Do not run an older binary
against an incompatible database or copy volumes while writers are active.

## Acceptance evidence and existing limits

The development verification wrapper defaults to the SSH alias `loki`, following
the owner's 2026-11-04 move from Thor for disk capacity. Loki had 253.6 GiB free,
8 CPUs and approximately 21 GB available RAM at preflight. The two remote runner
filenames retain `on-thor` for source provenance; their `-QaHost` argument selects
the actual host. Source snapshots, build logs and disposable installations use a
dedicated Phase 1 directory. The Playwright runtime is isolated beneath that
directory; it is a QA dependency, not an operator installation requirement.

Thor's second container build was deliberately cancelled (exit 130) after the
Rust, PostgreSQL authorization, avatar inventory and offline-maintenance checks
passed. Preserve that result as interrupted evidence, not a successful image
build. Earlier Thor snapshots and logs were archived and hash-verified on Loki
under `opencorde-phase1-20261104/thor-evidence` before removing the owned Thor
QA stack and disposable debug cache. Approximately 20.9 GiB was reclaimed;
release artifacts and unrelated services were preserved. Production was not moved.

Disposable media QA uses a loopback address and a temporary HTTPS port. Only
`prepare --test-local-images` with a loopback public IP enables LiveKit loopback
ICE candidates and permits that exact IP as a TURN peer (`/32`). Production
preparation does not add either exception. This fixture can verify relay media
flow, but it does not establish public Internet reachability on port 443.

The bundled proxy records the route class, method, response and upstream
status, and elapsed time. Signaling URLs carry room tokens, so HTTP request
logs omit paths, queries and headers. Nginx HTTP error details can contain the
raw request and are suppressed. Use the status and timing fields to diagnose
upstream failures. Stream and process diagnostics still go to stderr.

Source provenance is in `reports/phase1-source-baseline.md`. The frozen backend
and deployed web fixes are combined here; their earlier tests do not by
themselves prove the combined container installation. The isolated container
suite must supply login/text/object/restore and network evidence. Installation
timing must name host, bandwidth, architecture, image-cache state and whether
downloads were included.

On 2026-11-04, Loki (Linux amd64, 8 CPUs, approximately 23.5 GiB RAM) started
a fresh installation in 56.5 seconds with the pinned dependencies and four
locally built application images already available. That timing excludes image
builds and pulls. Download bandwidth was not measured. The run verified HTTPS
login, text, private object reads and blocked outbound IPv4/DNS probes. Two
Chromium clients sent and received audio and video in 5.63 seconds on the
direct path and 4.28 seconds through forced TURN/TLS on a remapped loopback
port. Both clients selected relay candidates and decoded video frames. The
outbound-traffic tests also passed with an interrupted guard process and with
a separate media guard configured for a specific UDP destination. IPv6, a
host reboot and public Internet TURN connectivity still require separate tests.

The same run (`46ce41aa211340d2ab708eff7e3f12cd`) refused a backup with an
extra writer active, then completed a coordinated backup in 70.9 seconds. It
refused an occupied restore volume before accepting an empty installation.
Restore took 29.7 seconds, followed by 25.7 seconds to bring up the restored
stack. The original account could sign in and read its message. The server
identity hash, referenced object manifest and attachment bytes matched the
source. Anonymous attachment access returned HTTP 403. The rebuilt index
returned the test message, and egress probes passed again on the restored stack.

The inherited application still has incomplete encrypted-channel live-index
containment and a legacy attachment URL model. The maintenance rebuild excludes
encrypted channels. Bucket creation keeps objects private and does not silently
grant anonymous reads; authenticated browser attachment delivery remains an
application security limitation to resolve before claiming that workflow works.
The inherited dependency audit also contains unresolved advisories recorded by
the previous security candidate. These limits are not erased by containerization.

Implementation references: [Docker networking](https://docs.docker.com/engine/network/),
[Compose startup ordering](https://docs.docker.com/compose/how-tos/startup-order/),
[LiveKit deployment and TURN](https://docs.livekit.io/transport/self-hosting/deployment/),
[pinned LiveKit ICE server advertisement](https://github.com/livekit/livekit/blob/v1.13.11/pkg/service/roommanager.go#L994-L1083),
[pinned LiveKit TURN proxy configuration](https://github.com/livekit/livekit/blob/v1.13.11/pkg/config/config.go#L285-L308),
[nginx TLS SNI preread](https://nginx.org/en/docs/stream/ngx_stream_ssl_preread_module.html),
[nftables hooks](https://wiki.nftables.org/wiki-nftables/index.php/Netfilter_hooks).
