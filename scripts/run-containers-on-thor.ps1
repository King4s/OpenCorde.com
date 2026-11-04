param(
    [ValidatePattern('^[a-z][a-z0-9-]{0,62}$')][string]$QaHost = 'loki',
    [ValidatePattern('^/home/mb/opencorde-phase1-20261104(?:/[a-z0-9_-]+)?$')][string]$RemoteBase = '/home/mb/opencorde-phase1-20261104/containers',
    [ValidatePattern('^/home/mb/opencorde-phase1-20261104/media-runtime(?:/[a-z0-9_-]+)?$')][string]$MediaRuntime = '/home/mb/opencorde-phase1-20261104/media-runtime'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$remoteBase = $RemoteBase
$exitCode = 1

function Invoke-Native([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Command failed ($LASTEXITCODE)" }
}

try {
    # Inventory includes the checked out source, including the explicitly reviewed
    # untracked Phase 1 files. Never walk the repository or transfer ignored files.
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = 'git'
    $psi.Arguments = 'ls-files -z --cached --others --exclude-standard'
    $psi.WorkingDirectory = $repo
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $process = [Diagnostics.Process]::Start($psi)
    $names = $process.StandardOutput.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw 'Git source inventory failed' }
    $files = New-Object 'System.Collections.Generic.List[string]'
    foreach ($name in ($names.Split([char]0) | Sort-Object -Unique)) {
        if (-not $name) { continue }
        $allow = $name -match '^(Cargo\.(toml|lock)|rust-toolchain(\.toml)?|\.cargo/config(\.toml)?|\.dockerignore|Dockerfile\.(api|web|netguard|minio)|compose\.yaml)$' -or
                 $name -match '^crates/.+\.(rs|toml|sql)$' -or
                 $name -match '^client/(package\.json|pnpm-lock\.yaml|svelte\.config\.js|tsconfig\.json|vite\.config\.ts|src/.+|scripts/.+|static/.+)$' -or
                 $name -match '^deploy/(containers|nginx)/.+' -or
                 $name -in @('scripts/ops.py', 'scripts/phase1-smoke.py',
                             'tests/ops/phase1-media.mjs', 'tests/ops/phase1_egress.py',
                             'tests/ops/test_operations.py', 'tests/ops/test_egress.py')
        if (-not $allow) { continue }
        if ($name -match '(^|/)(\.git|node_modules|target|build|runtime|data|private|secrets|reports|\.svelte-kit|drafts)/' -or
            $name -match '(^|/)\.env[^/]*$|\.(pem|key|pfx|p12|crt|cer)$|(^|/)(credentials|secrets)(\.[^/]*)?$') { continue }
        if ($name -notmatch '^[A-Za-z0-9_. /@+\[\]-]+$' -or $name -match '(^|/)\.\.(/|$)|(^|/)\.(/|$)' -or $name.Contains('\')) { throw "Unsafe archive name: $name" }
        $full = [IO.Path]::GetFullPath((Join-Path $repo $name))
        if (-not $full.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Source escaped repository' }
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { continue }
        $ancestor = Get-Item -LiteralPath $full -Force
        while ($null -ne $ancestor -and $ancestor.FullName -ne $repo) {
            if (($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Symlink/reparse source rejected: $name" }
            $ancestor = Get-Item -LiteralPath (Split-Path -Parent $ancestor.FullName) -Force
        }
        $files.Add($name)
    }
    foreach ($required in @('Cargo.toml', 'Cargo.lock', '.dockerignore', 'compose.yaml', 'Dockerfile.api', 'Dockerfile.web', 'Dockerfile.netguard', 'Dockerfile.minio', 'scripts/ops.py', 'scripts/phase1-smoke.py', 'tests/ops/phase1-media.mjs', 'tests/ops/phase1_egress.py')) {
        if (-not $files.Contains($required)) { throw "Missing QA source: $required" }
    }
    $temp = Join-Path $repo ('target/qa-container-sync/' + [Guid]::NewGuid().ToString('N'))
    foreach ($part in @($repo, (Join-Path $repo 'target'), (Join-Path $repo 'target/qa-container-sync'))) {
        if ((Test-Path -LiteralPath $part) -and (((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'Staging path is a reparse point' }
    }
    New-Item -ItemType Directory -Path $temp -Force | Out-Null
    $list = Join-Path $temp 'files.txt'
    [IO.File]::WriteAllLines($list, $files, (New-Object Text.UTF8Encoding($false)))
    $archive = Join-Path $temp 'sources.tar'
    Invoke-Native 'tar' @('-cf', $archive, '-C', $repo, '-T', $list)
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $runId = [Guid]::NewGuid().ToString('N')
    $snapshot = "$remoteBase/$hash-$runId"
    $upload = "$remoteBase/upload-$runId.tar"
    $sshOptions = @('-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10')
    $prepare = "set -eu; test `"`$(id -un)`" = mb; for p in /home /home/mb /home/mb/opencorde-phase1-20261104 '$remoteBase'; do test ! -L `"`$p`" || exit 1; done; mkdir -p '$remoteBase'; mkdir '$snapshot'"
    Invoke-Native 'ssh' ($sshOptions + @($QaHost, $prepare))
    Invoke-Native 'scp' ($sshOptions + @($archive, "${QaHost}:$upload"))
    $remote = @'
set -eu
test "$(id -un)" = mb
test ! -L '__BASE__'
test ! -L '__SNAPSHOT__'
test ! -L '__UPLOAD__'
printf '%s  %s\n' '__HASH__' '__UPLOAD__' | sha256sum -c -
tar -tf '__UPLOAD__' | python3 -c 'import sys; from pathlib import PurePosixPath; names=sys.stdin.read().splitlines(); assert names and all(not x.startswith("/") and ".." not in PurePosixPath(x).parts for x in names)'
tar -xf '__UPLOAD__' -C '__SNAPSHOT__' --no-same-owner --no-same-permissions
rm -- '__UPLOAD__'
cd '__SNAPSHOT__'
python3 -B -m unittest discover -s tests/ops -v
printf 'QA source snapshot: %s\nSource SHA256: %s\n' '__SNAPSHOT__' '__HASH__'
env -u DATABASE_URL -u REDIS_URL -u OPENCORDE_TEST_DATABASE_URL OPENCORDE_QA_HOST='__QA_HOST__' OPENCORDE_QA_MEDIA_RUNTIME='__MEDIA_RUNTIME__' python3 scripts/phase1-smoke.py run --source-hash '__HASH__' --run-id '__RUNID__'
'@
    $remote = $remote.Replace('__BASE__', $remoteBase).Replace('__SNAPSHOT__', $snapshot).Replace('__UPLOAD__', $upload).Replace('__HASH__', $hash).Replace('__RUNID__', $runId).Replace('__QA_HOST__', $QaHost).Replace('__MEDIA_RUNTIME__', $MediaRuntime)
    $remote.Replace("`r`n", "`n") | & ssh @sshOptions $QaHost "tr -d '\r' | sh -s"
    $exitCode = $LASTEXITCODE
    $reportTemp = Join-Path $temp 'phase1-runtime.json'
    & scp @sshOptions "${QaHost}:$snapshot/reports/phase1-runtime.json" $reportTemp
    if ($LASTEXITCODE -eq 0) {
        $report = Get-Content -LiteralPath $reportTemp -Raw | ConvertFrom-Json
        if ($report.source_sha256 -ne $hash -or $report.run_id -ne $runId -or $report.release_ready -ne $false) {
            throw 'QA runtime report identity or release status is invalid'
        }
        $reportDir = Join-Path $repo 'reports'
        if ((Test-Path -LiteralPath $reportDir) -and (((Get-Item -LiteralPath $reportDir -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'Report directory is a reparse point' }
        New-Item -ItemType Directory -Path $reportDir -Force | Out-Null
        $reportFinal = Join-Path $reportDir 'phase1-runtime.json'
        if ((Test-Path -LiteralPath $reportFinal) -and (((Get-Item -LiteralPath $reportFinal -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'Report path is a reparse point' }
        Move-Item -LiteralPath $reportTemp -Destination $reportFinal -Force
        Write-Host "QA runtime report: $reportFinal"
    } elseif ($exitCode -eq 0) {
        throw 'Remote QA succeeded but its runtime report could not be retrieved'
    }
} catch {
    $exitCode = 1
    Write-Error $_ -ErrorAction Continue
}
exit $exitCode
