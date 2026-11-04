param(
    [ValidateSet('Api')][string]$Mode = 'Api',
    [ValidatePattern('^[a-z][a-z0-9-]{0,62}$')][string]$QaHost = 'loki',
    [ValidatePattern('^/home/mb/opencorde-phase1-20261104(?:/[a-z0-9_-]+)?$')][string]$RemoteBase = '/home/mb/opencorde-phase1-20261104/rust'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# Developer QA only. No production services, credentials, or database configuration.
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$remoteBase = $RemoteBase
$exitCode = 1
function Invoke-Native([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { $script:exitCode = $LASTEXITCODE; throw "$Command failed ($LASTEXITCODE)" }
}
try {
    # Read NUL-delimited Git names without PowerShell's line-oriented native pipeline.
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = 'git'
    $psi.Arguments = 'ls-files -z --cached --others --exclude-standard'
    $psi.WorkingDirectory = $repo
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $process = [Diagnostics.Process]::Start($psi)
    $names = $process.StandardOutput.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { $exitCode = $process.ExitCode; throw 'Git inventory failed' }
    $files = New-Object 'System.Collections.Generic.List[string]'
    foreach ($name in ($names.Split([char]0) | Sort-Object -Unique)) {
        if (-not $name) { continue }
        if ($name -notmatch '^(Cargo\.(toml|lock)|Dockerfile\.qa-rust|rust-toolchain(\.toml)?|\.cargo/config(\.toml)?|crates/.+)$') { continue }
        # Match forbidden directories and credential payloads, not words in Rust module names.
        if ($name -match '(^|/)(\.git|node_modules|target|build|runtime|data|private|private-policies|private_policies|policies|search-index)/') { continue }
        if ($name -match '(^|/)\.env[^/]*$|(^|/)(credentials|secrets)(\.(json|toml|yaml|yml|txt|ini))?$|\.(pem|key|pfx|p12|crt|cer)$') { continue }
        if ($name -notmatch '^[A-Za-z0-9_. /-]+$' -or $name -match '(^|/)\.\.(/|$)|(^|/)\.(/|$)' -or $name.Contains('\')) { throw "Unsafe archive name: $name" }
        $full = [IO.Path]::GetFullPath((Join-Path $repo $name))
        if (-not $full.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Input escaped repository' }
        if (-not (Test-Path -LiteralPath $full)) { continue } # Tracked deletion.
        $item = Get-Item -LiteralPath $full -Force
        if ($item.PSIsContainer) { throw "Non-file input: $name" }
        $ancestor = $item
        while ($null -ne $ancestor -and $ancestor.FullName -ne $repo) {
            if (($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Symlink/reparse input rejected: $name" }
            $ancestor = Get-Item -LiteralPath (Split-Path -Parent $ancestor.FullName) -Force
        }
        $files.Add($name)
    }
    if (-not $files.Contains('Cargo.toml') -or -not $files.Contains('Cargo.lock') -or -not $files.Contains('Dockerfile.qa-rust')) { throw 'Cargo manifests and Dockerfile.qa-rust are required' }
    $temp = Join-Path $repo ('target/qa-sync/' + [Guid]::NewGuid().ToString('N'))
    # Do not traverse a pre-existing symlink when creating the local staging directory.
    foreach ($part in @($repo, (Join-Path $repo 'target'), (Join-Path $repo 'target/qa-sync'))) {
        if ((Test-Path -LiteralPath $part) -and (((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'Staging path is a reparse point' }
    }
    New-Item -ItemType Directory -Path $temp -Force | Out-Null
    $list = Join-Path $temp 'files.txt'
    [IO.File]::WriteAllLines($list, $files, (New-Object Text.UTF8Encoding($false)))
    $archive = Join-Path $temp 'sources.tar'
    Invoke-Native 'tar' @('-cf', $archive, '-C', $repo, '-T', $list)
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $sshOptions = @('-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10')
    # All shell paths are constants or lowercase SHA256/GUID; no user text enters shell code.
    $runId = [Guid]::NewGuid().ToString('N')
    $snapshot = "$remoteBase/snapshots/$hash-$runId"
    $upload = "$remoteBase/sources-$runId.tar"
    $prepare = "set -eu; test `"`$(id -un)`" = mb; for p in /home /home/mb /home/mb/opencorde-phase1-20261104 $remoteBase $remoteBase/src $remoteBase/snapshots $remoteBase/target $remoteBase/cargo; do test ! -L `"`$p`" || exit 1; done; mkdir -p '$remoteBase/snapshots' '$remoteBase/target' '$remoteBase/cargo'; mkdir '$snapshot'"
    Invoke-Native 'ssh' ($sshOptions + @($QaHost, $prepare))
    Invoke-Native 'scp' ($sshOptions + @($archive, "${QaHost}:$upload"))
    $remote = @'
set -eu
test "$(id -un)" = mb
for p in /home /home/mb /home/mb/opencorde-phase1-20261104 __BASE__ __BASE__/src __BASE__/snapshots __BASE__/target __BASE__/cargo __SNAPSHOT__; do
  test ! -L "$p" || exit 1
done
test ! -L '__UPLOAD__'
printf '%s  %s\n' '__HASH__' '__UPLOAD__' | sha256sum -c -
tar -xf '__UPLOAD__' -C '__SNAPSHOT__' --no-same-owner --no-same-permissions
cd '__SNAPSHOT__'
printf 'QA snapshot: %s\nSource archive SHA256: %s\n' '__SNAPSHOT__' '__HASH__'
unset DATABASE_URL REDIS_URL OPENCORDE_TEST_DATABASE_URL
export PATH=/usr/local/bin:/usr/bin:/bin
qa_image='opencorde-qa-rust:__HASH16__'
docker build --file Dockerfile.qa-rust --tag "$qa_image" .
container_name='opencorde-qa-postgres-__RUNID__'
owner='__RUNID__'
container_id=''
qa_index=0
cleanup() {
  status=$?
  trap - EXIT HUP INT TERM
  for qa_id in $(docker ps -aq --no-trunc --filter "label=opencorde.qa.owner=$owner" --filter 'label=opencorde.qa.kind=rust' 2>/dev/null); do
    case "$qa_id" in ''|*[!0-9a-f]*) continue ;; esac
    [ "${#qa_id}" -eq 64 ] || continue
    qa_name=$(docker inspect --format '{{.Name}}' "$qa_id" 2>/dev/null) || qa_name=''
    qa_owner=$(docker inspect --format '{{index .Config.Labels "opencorde.qa.owner"}}' "$qa_id" 2>/dev/null) || qa_owner=''
    qa_kind=$(docker inspect --format '{{index .Config.Labels "opencorde.qa.kind"}}' "$qa_id" 2>/dev/null) || qa_kind=''
    case "$qa_name" in "/opencorde-qa-rust-$owner-"*) ;; *) continue ;; esac
    if [ "$qa_owner" = "$owner" ] && [ "$qa_kind" = rust ]; then
      if ! docker rm -f "$qa_id" >/dev/null; then
        printf '%s\n' 'Failed to remove owned Rust QA container' >&2
        [ "$status" -ne 0 ] || status=1
      fi
    fi
  done
  # A unique name alone is insufficient: verify the label and immutable ID too.
  case "$container_name" in opencorde-qa-postgres-"$owner") ;; *) exit "$status" ;; esac
  actual_id=$(docker inspect --format '{{.Id}}' "$container_name" 2>/dev/null) || actual_id=''
  case "$actual_id" in ''|*[!0-9a-f]*) exit "$status" ;; esac
  if [ "${#actual_id}" -eq 64 ] &&
     [ "$(docker inspect --format '{{.Name}}' "$actual_id" 2>/dev/null)" = "/$container_name" ] &&
     [ "$(docker inspect --format '{{index .Config.Labels "opencorde.qa.owner"}}' "$actual_id" 2>/dev/null)" = "$owner" ] &&
     { [ -z "$container_id" ] || [ "$actual_id" = "$container_id" ]; }; then
    if ! docker rm -f "$actual_id" >/dev/null; then
      printf '%s\n' 'Failed to remove owned QA database container' >&2
      [ "$status" -ne 0 ] || status=1
    fi
  fi
  exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
qa_rust() {
  qa_index=$((qa_index + 1))
  qa_name="opencorde-qa-rust-$owner-$qa_index"
  docker run --rm --name "$qa_name" --label "opencorde.qa.owner=$owner" \
    --label 'opencorde.qa.kind=rust' --network host --read-only \
    --cap-drop ALL --security-opt no-new-privileges \
    --tmpfs /tmp:rw,exec,nosuid,size=2g --user "$(id -u):$(id -g)" \
    --mount 'type=bind,src=__SNAPSHOT__,dst=/qa/src,readonly' \
    --mount 'type=bind,src=__BASE__/target,dst=/qa/target' \
    --mount 'type=bind,src=__BASE__/cargo,dst=/qa/cargo' \
    --env CARGO_HOME=/qa/cargo --env HOME=/qa/cargo \
    --env CARGO_TARGET_DIR=/qa/target --env CARGO_BUILD_JOBS=4 \
    --env OPENCORDE_TEST_DATABASE_URL "$qa_image" "$@"
}
cargo() { qa_rust cargo "$@"; }
rustc() { qa_rust rustc "$@"; }
cargo --version
rustc --version
# Complete the default suite before creating the disposable database.
cargo test --workspace --locked
# No mounts or production configuration; credentials belong only to this empty DB.
container_id=$(docker run -d --rm --name "$container_name" \
  --label "opencorde.qa.owner=$owner" --tmpfs /var/lib/postgresql/data:rw \
  -e POSTGRES_USER=opencorde_qa -e POSTGRES_PASSWORD=opencorde_qa_ephemeral \
  -e POSTGRES_DB=opencorde_qa -p 127.0.0.1::5432 postgres:16-alpine@sha256:0cefaabd882e42bcb2a2f3dc56d3ce4770560ba62fd7f197f1a2e008fd5731ab)
case "$container_id" in ''|*[!0-9a-f]*) printf '%s\n' 'Invalid QA container ID' >&2; exit 1 ;; esac
[ "${#container_id}" -eq 64 ] || exit 1
ready=0
for attempt in $(seq 1 60); do
  if docker exec "$container_id" pg_isready -h 127.0.0.1 -U opencorde_qa -d opencorde_qa >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
[ "$ready" -eq 1 ] || { printf '%s\n' 'QA database readiness timed out' >&2; exit 1; }
binding=$(docker port "$container_id" 5432/tcp)
case "$binding" in 127.0.0.1:*) port=${binding#127.0.0.1:} ;; *) exit 1 ;; esac
case "$port" in ''|*[!0-9]*) exit 1 ;; esac
[ "$port" -ge 1 ] && [ "$port" -le 65535 ] || exit 1
export OPENCORDE_TEST_DATABASE_URL="postgres://opencorde_qa:opencorde_qa_ephemeral@127.0.0.1:$port/opencorde_qa"
# The ignored test applies the full ../opencorde-db/migrations set to this empty DB.
cargo test --locked -p opencorde-api ws::dispatch::tests::current_permissions_gate_every_delivery -- --ignored --exact
cargo test --locked -p opencorde-api routes::messages::notifications::tests::current_access_gates_notification_side_effects -- --ignored --exact
cargo test --locked -p opencorde-api routes::messages::send::tests::timeouts_gate_channel_persistence_and_events -- --ignored --exact
cargo test --locked -p opencorde-api --test persistent_websocket persistent_websocket_rechecks_current_authorization -- --ignored --exact --nocapture --test-threads=1
cargo test --locked -p opencorde-api --test offline_maintenance -- --ignored --test-threads=1
cargo test --locked -p opencorde-api maintenance::tests::object_inventory_includes_avatars_and_files_across_keyset_batches -- --ignored --exact --test-threads=1
'@
    $remote = $remote.Replace('__BASE__', $remoteBase).Replace('__SNAPSHOT__', $snapshot).Replace('__UPLOAD__', $upload).Replace('__HASH__', $hash).Replace('__HASH16__', $hash.Substring(0,16)).Replace('__RUNID__', $runId)
    # Send script on stdin, avoiding multi-line command quoting by Windows OpenSSH.
    # Windows PowerShell appends CRLF to native pipeline input. Strip CR at the
    # remote stdin boundary before sh parses commands and their final arguments.
    $remote.Replace("`r`n", "`n") | & ssh @sshOptions $QaHost "tr -d '\r' | sh -s"
    $exitCode = $LASTEXITCODE
} catch {
    $exitCode = 1
    Write-Error $_ -ErrorAction Continue
}
exit $exitCode
