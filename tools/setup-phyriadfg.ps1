$ErrorActionPreference = "Stop"

# Install the official, pinned PhyriadFG 0.6.0 experimental CLI release.
# SHA-256 is the digest published by the upstream GitHub release API.
$root = Split-Path -Parent $PSScriptRoot
$version = "v0.6.0-experimental"
$url = "https://github.com/Swately/PhyriadFG/releases/download/$version/phyriad_fg.exe"
$expectedSha256 = "52c0389fb48d0a789825aab7c682eaf7b49da301d6b8925cf04dee8a3deae6b2"

$dest = Join-Path $root "target\release\phyriad_fg.exe"
New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
$temp = Join-Path $env:TEMP "aetherx-phyriad_fg.exe"

Write-Host "Downloading PhyriadFG $version..."
Invoke-WebRequest -Uri $url -OutFile $temp -UseBasicParsing
$actualSha256 = (Get-FileHash -Path $temp -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha256 -ne $expectedSha256) {
    Remove-Item -Force $temp -ErrorAction SilentlyContinue
    throw "PhyriadFG checksum mismatch. Expected $expectedSha256, got $actualSha256."
}

Move-Item -Force $temp $dest
Write-Host "Verified and installed PhyriadFG: $dest"
Write-Host "AetherX will start it automatically with each game."
