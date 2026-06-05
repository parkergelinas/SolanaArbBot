# Start SolanaArbBot backend services (Windows PowerShell)
# Usage:
#   .\scripts\dev-services.ps1              # start all in new windows
#   .\scripts\dev-services.ps1 -Status      # check what's running
#   .\scripts\dev-services.ps1 -Stop        # stop services on known ports

param(
    [switch]$Status,
    [switch]$Stop,
    [switch]$ControlOnly,
    [switch]$StreamOnly
)

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Ports = @{
    'control-api'       = 3001
    'stream-api'        = 8080
    'intelligence-api'  = 8090
}

function Get-PortOwner([int]$Port) {
    $conn = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $conn) { return $null }
    $proc = Get-Process -Id $conn.OwningProcess -ErrorAction SilentlyContinue
    return [PSCustomObject]@{ Port = $Port; PID = $conn.OwningProcess; Name = $proc.ProcessName }
}

function Show-Status {
    Write-Host "`nService status:" -ForegroundColor Cyan
    foreach ($svc in $Ports.GetEnumerator() | Sort-Object Value) {
        $owner = Get-PortOwner $svc.Value
        if ($owner) {
            Write-Host ("  {0,-18} :{1}  RUNNING  (PID {2}, {3})" -f $svc.Key, $svc.Value, $owner.PID, $owner.Name) -ForegroundColor Green
        } else {
            Write-Host ("  {0,-18} :{1}  stopped" -f $svc.Key, $svc.Value) -ForegroundColor DarkGray
        }
    }
    Write-Host ""
}

function Stop-Services {
    foreach ($svc in $Ports.GetEnumerator()) {
        $owner = Get-PortOwner $svc.Value
        if ($owner) {
            Write-Host "Stopping $($svc.Key) on port $($svc.Value) (PID $($owner.PID))..."
            Stop-Process -Id $owner.PID -Force -ErrorAction SilentlyContinue
        }
    }
    Start-Sleep -Seconds 1
    Show-Status
}

function Start-ServiceWindow([string]$Name, [string]$Package, [string]$EnvPrefix = '') {
    $port = $Ports[$Name]
    $owner = Get-PortOwner $port
    if ($owner) {
        Write-Host "$Name already running on port $port (PID $($owner.PID))" -ForegroundColor Yellow
        return
    }

    $cmd = if ($EnvPrefix) {
        "Set-Location '$Root'; $EnvPrefix cargo run -p $Package"
    } else {
        "Set-Location '$Root'; cargo run -p $Package"
    }
    Write-Host "Starting $Name in new window..."
    Start-Process powershell -ArgumentList @('-NoExit', '-Command', $cmd)
}

if ($Status) { Show-Status; exit 0 }
if ($Stop)   { Stop-Services; exit 0 }

Write-Host "SolanaArbBot dev services" -ForegroundColor Cyan
Write-Host "Repo: $Root`n"

$ControlEnv = @(
    '$env:DATA_LAYER_INGEST = if ($env:DATA_LAYER_INGEST) { $env:DATA_LAYER_INGEST } else { "auto" }'
    'if (-not $env:SOLANA_ARB_RPC__ENDPOINTS) { Write-Host "Tip: set SOLANA_ARB_RPC__ENDPOINTS for live chain swaps" -ForegroundColor DarkYellow }'
) -join '; '

$StreamEnv = '$env:SIGNAL_HUB_URL = "http://127.0.0.1:3001"'

if (-not $StreamOnly) {
    Start-ServiceWindow 'control-api' 'control-api' $ControlEnv
}
if (-not $ControlOnly) {
    Start-ServiceWindow 'stream-api' 'stream-api' $StreamEnv
}
if (-not $ControlOnly -and -not $StreamOnly) {
    Start-ServiceWindow 'intelligence-api' 'intelligence-api'
}

Start-Sleep -Seconds 2
Show-Status

Write-Host @"
NOTES (PowerShell):
  - Do NOT use bash syntax:  cargo run -p control-api & cargo run -p stream-api
  - Use this script, OR open separate terminals, OR:
      Start-Process powershell -ArgumentList '-NoExit','-Command','cargo run -p control-api'

Dashboard:
  cd apps\dashboard
  npm run dev
  -> http://localhost:3000
"@ -ForegroundColor DarkGray
