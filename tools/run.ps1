# Build and run Soulscale locally on Windows.
#
#   powershell -ExecutionPolicy Bypass -File tools\run.ps1            # build client + server, run, open browser
#   powershell -ExecutionPolicy Bypass -File tools\run.ps1 -NoBuild   # skip the client build
#   powershell -ExecutionPolicy Bypass -File tools\run.ps1 -Tunnel    # also open a Cloudflare tunnel (for Discord)
#   powershell -ExecutionPolicy Bypass -File tools\run.ps1 -Name Bob -NoBrowser
#
# Ctrl+C stops the server. Settings come from .env in the repo root (see .env.example).
param(
    [switch]$NoBuild,
    [switch]$NoBrowser,
    [switch]$Tunnel,
    [string]$Name = "Wofl",
    [int]$Port = 3001
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

foreach ($tool in "cargo", "npm") {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool not found. Install Rust (winget install Rustlang.Rustup) and Node (winget install OpenJS.NodeJS.LTS), then open a new terminal."
    }
}

if (-not $NoBuild) {
    Push-Location client
    try {
        if (-not (Test-Path node_modules)) {
            Write-Host "Installing client packages..." -ForegroundColor Cyan
            npm ci
            if ($LASTEXITCODE -ne 0) { throw "npm ci failed" }
        }
        Write-Host "Building client..." -ForegroundColor Cyan
        npm run build
        if ($LASTEXITCODE -ne 0) { throw "client build failed" }
    }
    finally { Pop-Location }
}

Write-Host "Building server (first time takes a few minutes)..." -ForegroundColor Cyan
cargo build --release -p soulscale-server
if ($LASTEXITCODE -ne 0) { throw "server build failed" }

$env:SOULSCALE_BIND = "127.0.0.1:$Port"
$base = "http://127.0.0.1:$Port"
# Absolute path, and run from the repo root: the server finds .env, client\dist and data\ from there.
$exe = Join-Path $root "target\release\soulscale-server.exe"
$server = Start-Process -FilePath $exe -WorkingDirectory $root -NoNewWindow -PassThru

# Wait for the server to answer before opening anything.
$up = $false
for ($i = 0; $i -lt 60; $i++) {
    if ($server.HasExited) { throw "server exited early (code $($server.ExitCode)); see the log above" }
    try {
        Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 "$base/api/health" | Out-Null
        $up = $true
        break
    }
    catch { Start-Sleep -Milliseconds 500 }
}
if (-not $up) { throw "server did not answer on $base" }
Write-Host "Soulscale is up at $base" -ForegroundColor Green

if ($Tunnel) {
    if (Get-Command cloudflared -ErrorAction SilentlyContinue) {
        Write-Host "Opening a Cloudflare tunnel in a new window. Copy its https://....trycloudflare.com host" -ForegroundColor Yellow
        Write-Host "into Developer Portal > Activities > URL Mappings (target for prefix /, no https://)." -ForegroundColor Yellow
        Start-Process -FilePath "cloudflared" -ArgumentList "tunnel", "--url", "http://localhost:$Port"
    }
    else {
        Write-Host "cloudflared not found: winget install --id Cloudflare.cloudflared" -ForegroundColor Red
    }
}

if (-not $NoBrowser) { Start-Process "$base/?name=$Name" }
Write-Host "Ctrl+C to stop." -ForegroundColor DarkGray
try { Wait-Process -Id $server.Id }
finally {
    if (-not $server.HasExited) { Stop-Process -Id $server.Id }
}
