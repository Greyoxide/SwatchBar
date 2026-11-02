# Builds a release binary and zips it as target\Swatchbar-windows.zip.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cargo build --release
$out = "target\Swatchbar-windows"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Path $out | Out-Null
$bin = if ($env:CARGO_TARGET_DIR) { "$env:CARGO_TARGET_DIR\release\swatchbar.exe" } else { "target\release\swatchbar.exe" }
Copy-Item $bin "$out\Swatchbar.exe"
Compress-Archive -Path "$out\*" -DestinationPath "$out.zip" -Force
Write-Host "Built $out.zip"
