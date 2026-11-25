$ErrorActionPreference = "Stop"

# Build the MIT-licensed PhyriadFG frame generator at a pinned upstream commit.
$root = Split-Path -Parent $PSScriptRoot
$vendor = Join-Path $root "vendor\PhyriadFG"
$pin = "2ea163762c5bd3d2e79506f629de2fa1a9f83e1b"

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git is required. Install Git for Windows and rerun this script."
}
if (-not $env:VULKAN_SDK) {
    throw "Vulkan SDK is required. Install it from https://vulkan.lunarg.com/sdk/home and reopen PowerShell."
}

if (-not (Test-Path (Join-Path $vendor ".git"))) {
    New-Item -ItemType Directory -Force -Path (Split-Path $vendor) | Out-Null
    git clone https://github.com/Swately/PhyriadFG.git $vendor
}
git -C $vendor fetch origin $pin
git -C $vendor checkout --detach $pin

$build = Join-Path $vendor "build-release.bat"
if (-not (Test-Path $build)) { throw "PhyriadFG build script was not found." }
& $build
if ($LASTEXITCODE -ne 0) { throw "PhyriadFG build failed with exit code $LASTEXITCODE." }

$binary = Join-Path $vendor "build-release\phyriad_fg.exe"
if (-not (Test-Path $binary)) { throw "PhyriadFG did not produce phyriad_fg.exe." }

$dest = Join-Path $root "target\release\phyriad_fg.exe"
New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
Copy-Item -Force $binary $dest
Write-Host "Installed AetherX frame-generation backend: $dest"
