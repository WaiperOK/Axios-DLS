param(
    [string]$Prefix = "C:\Program Files\Axion"
)

$BinDir = Join-Path $Prefix "bin"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

Copy-Item ".\bin\axion.exe" -Destination (Join-Path $BinDir "axion.exe") -Force
Copy-Item ".\tools" -Destination (Join-Path $Prefix "tools") -Recurse -Force
Copy-Item ".\examples" -Destination (Join-Path $Prefix "examples") -Recurse -Force

Write-Host "[axion] Installed to $Prefix"
Write-Host "[axion] Add $BinDir to PATH if not already configured."
