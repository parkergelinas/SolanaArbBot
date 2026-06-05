# Frontend continuous validation gate
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dashboard = Join-Path $root "apps\dashboard"

Write-Host "==> Frontend unit + stream tests"
Set-Location $dashboard
npm test
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Frontend production build"
npm run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> Stream API contract tests"
Set-Location $root
cargo test -p stream-api
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "OK - frontend stable, regression-free"
