param([string[]]$AllowModified = @(), [string]$QaHost = 'loki')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$phase1Edits = @(
    'Cargo.lock', 'crates/opencorde-api/Cargo.toml',
    'crates/opencorde-api/src/config.rs', 'crates/opencorde-api/src/email.rs',
    'crates/opencorde-api/src/federation.rs', 'crates/opencorde-api/src/lib.rs',
    'crates/opencorde-api/src/main.rs', 'crates/opencorde-api/src/push_sender.rs',
    'crates/opencorde-api/src/middleware/rate_limit.rs',
    'crates/opencorde-api/src/routes/auth/steam.rs', 'crates/opencorde-api/src/routes/bridge.rs',
    'crates/opencorde-api/src/routes/dms/handlers.rs', 'crates/opencorde-api/src/routes/federation/handlers.rs',
    'crates/opencorde-api/src/routes/slash_commands/dispatch.rs', 'crates/opencorde-api/src/routes/unfurl.rs',
    'crates/opencorde-api/tests/persistent_websocket.rs', 'crates/opencorde-bridge/src/main.rs',
    'crates/opencorde-db/src/repos/message_repo/mod.rs',
    'crates/opencorde-search/src/lib.rs', 'scripts/run-rust-on-thor.ps1',
    'tests/client/login-smoke.mjs'
)
$arguments = @((Join-Path $PSScriptRoot 'verify-phase1-source.py'))
foreach ($path in $phase1Edits) { $arguments += @('--allow-modified', $path) }
foreach ($path in $AllowModified) { $arguments += @('--allow-modified', $path) }
& python @arguments
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Push-Location $repo
try {
    & python -m unittest discover -s tests/ops -v
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-rust-on-thor.ps1 -Mode Api -QaHost $QaHost
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-containers-on-thor.ps1 -QaHost $QaHost
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally { Pop-Location }
Write-Host 'Container acceptance passed; published multiarch release and independent approval remain separate release gates.'
