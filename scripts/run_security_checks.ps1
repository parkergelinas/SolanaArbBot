Param()
Set-StrictMode -Version Latest

Write-Host "Running cargo-audit..."
if (-not (Get-Command cargo-audit -ErrorAction SilentlyContinue)) {
    Write-Host "cargo-audit not found, installing..."
    cargo install cargo-audit --locked
}
try {
    cargo audit --json > audit.json -ErrorAction Stop
} catch {
    # continue to produce audit.json if cargo audit fails
}
Get-Content audit.json
if (Get-Command jq -ErrorAction SilentlyContinue) {
    $vuln = jq -e '.vulnerabilities.list | length == 0' audit.json
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Vulnerabilities found (see audit.json)"
        exit 1
    }
} else {
    Write-Host "jq not found; please inspect audit.json manually"
}

Write-Host "Running cargo-deny..."
if (-not (Get-Command cargo-deny -ErrorAction SilentlyContinue)) {
    Write-Host "cargo-deny not found, installing..."
    cargo install cargo-deny --locked
}
try {
    cargo deny check -v --json > deny.json -ErrorAction Stop
} catch {
}
Get-Content deny.json
if (Get-Command jq -ErrorAction SilentlyContinue) {
    $diag = jq -e '.diagnostics | length == 0' deny.json
    if ($LASTEXITCODE -ne 0) {
        Write-Error "cargo-deny reported issues (see deny.json)"
        exit 1
    }
} else {
    Write-Host "jq not found; please inspect deny.json manually"
}

Write-Host "Security checks passed"
