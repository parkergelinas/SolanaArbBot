Param()
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function Ensure-Tool([string]$Name, [string]$Version) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Host "$Name not found — installing $Version..."
        cargo install $Name --locked --version $Version
    }
}

Ensure-Tool "cargo-audit" "0.22.2"
Ensure-Tool "cargo-deny" "0.19.8"

Write-Host "Running cargo audit..."
cargo audit

Write-Host "Running cargo deny..."
cargo deny check advisories licenses bans sources

Write-Host "Security checks passed"
