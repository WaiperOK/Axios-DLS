# requires -version 5.0
param(
    [string]$Prefix = "$env:USERPROFILE\AppData\Local\Axion"
)

$BinDir = Join-Path $Prefix "bin"
$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

Copy-Item (Join-Path $RepoRoot "tools\axion_runner.py") -Destination (Join-Path $BinDir "axion_runner.py") -Force
Copy-Item (Join-Path $RepoRoot "tools\axion.cmd") -Destination (Join-Path $BinDir "axion.cmd") -Force

Write-Host "[axion] Installed runner to $BinDir"
Write-Host "[axion] Add $BinDir to your PATH, or run 'Set-ExecutionPolicy RemoteSigned' if needed for PowerShell scripts."
