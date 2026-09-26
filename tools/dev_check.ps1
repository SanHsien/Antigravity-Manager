[CmdletBinding()]
param(
    [switch]$Quick
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repoRoot

$env:NODE_NO_WARNINGS = "1"
$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Write-Host "==> Checking Frontend Build (npm run build)"
npm run build
if ($LASTEXITCODE -ne 0) {
    throw "Frontend build failed with exit code $LASTEXITCODE"
}

Write-Host "==> Compiling Python tools"
python -m compileall -q tools
if ($LASTEXITCODE -ne 0) {
    throw "Python compile failed with exit code $LASTEXITCODE"
}

Write-Host "==> Checking fork document links"
python tools/check_links.py
if ($LASTEXITCODE -ne 0) {
    throw "Link check failed with exit code $LASTEXITCODE"
}

Write-Host "==> Checking zh-TW locale key completeness"
python tools/check_locale_keys.py
if ($LASTEXITCODE -ne 0) {
    throw "zh-TW locale key check failed with exit code $LASTEXITCODE"
}

Write-Host "==> Checking upstream updates"
python tools/check_upstream_updates.py --strict
if ($LASTEXITCODE -ne 0) {
    throw "Upstream check failed with exit code $LASTEXITCODE"
}

Write-Host "WINDOWS DEV CHECK GREEN"
