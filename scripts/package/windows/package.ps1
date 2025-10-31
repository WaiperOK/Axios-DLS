#requires -version 5.0
[CmdletBinding()]
param(
    [string]$Version = "dev",
    [string]$Configuration = "release",
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$OutputDir = "build\\package\\windows"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..\..") | Select-Object -ExpandProperty Path
Set-Location $RepoRoot

Write-Host "[package] building axion-cli ($Configuration, $Target)"
cargo build --$Configuration -p axion-cli `
    --target $Target | Out-Null

$stageRoot = Join-Path $RepoRoot ("build\\stage\\windows-" + $Version)
if (Test-Path $stageRoot) {
    Remove-Item $stageRoot -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stageRoot "bin") | Out-Null

$binaryPath = Join-Path $RepoRoot ("target\$Target\$Configuration\axion-cli.exe")
if (-not (Test-Path $binaryPath)) {
    throw "Compiled binary not found at $binaryPath"
}
Copy-Item $binaryPath -Destination (Join-Path $stageRoot "bin\axion.exe")

$copyDirs = @("examples", "docs", "tools", "ui\react-flow-prototype\dist")
foreach ($dir in $copyDirs) {
    $src = Join-Path $RepoRoot $dir
    if (Test-Path $src) {
        Copy-Item $src -Destination (Join-Path $stageRoot $dir) -Recurse
    } else {
        Write-Warning "[package] source directory not found: $dir"
    }
}

$supportFiles = @("LICENSE", "README.md")
foreach ($file in $supportFiles) {
    $src = Join-Path $RepoRoot $file
    if (Test-Path $src) {
        Copy-Item $src -Destination (Join-Path $stageRoot $file)
    }
}

$installScript = @"
param(
    [string]`$Prefix = "$env:ProgramFiles\Axion"
)

`$BinDir = Join-Path `$Prefix "bin"
New-Item -ItemType Directory -Force -Path `$BinDir | Out-Null

Copy-Item ".\bin\axion.exe" -Destination (Join-Path `$BinDir "axion.exe") -Force
Copy-Item ".\tools" -Destination (Join-Path `$Prefix "tools") -Recurse -Force
Copy-Item ".\examples" -Destination (Join-Path `$Prefix "examples") -Recurse -Force

Write-Host "[axion] Installed to `$Prefix"
Write-Host "[axion] Add `$BinDir to PATH if not already configured."
"@
Set-Content -Path (Join-Path $stageRoot "install.ps1") -Value $installScript -Encoding UTF8

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$zipPath = Join-Path $RepoRoot ("$OutputDir\axion-" + $Version + "-windows-x64.zip")
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $stageRoot "*") -DestinationPath $zipPath

Write-Host "[package] archive created at $zipPath"
