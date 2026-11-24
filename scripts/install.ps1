# Install zap on Windows to %LOCALAPPDATA%\zap
# Prefers the GitHub release binary; builds from source if that asset is missing.
$ErrorActionPreference = "Stop"
# irm … | iex leaves $PSScriptRoot empty — fail clearly instead of Resolve-Path noise (#69).
if (-not $PSScriptRoot) {
  Write-Error @"
PSScriptRoot is empty (common with irm|iex).
Download the release: https://github.com/tobbikz/zap/releases/latest/download/zap-windows-x86_64.exe
Or clone the repo and run: .\scripts\install.ps1
"@
  exit 1
}
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

$DestDir = Join-Path $env:LOCALAPPDATA "zap"
New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
$Dest = Join-Path $DestDir "zap.exe"
$Url = "https://github.com/tobbikz/zap/releases/latest/download/zap-windows-x86_64.exe"

$downloaded = $false
try {
  Write-Host "-> downloading latest Windows release..."
  Invoke-WebRequest -Uri $Url -OutFile $Dest -UseBasicParsing
  if ((Get-Item $Dest).Length -lt 1MB) {
    throw "download was too small to be zap.exe"
  }
  $downloaded = $true
  Write-Host "-> installed: $Dest"
} catch {
  Write-Host "-> download failed ($($_.Exception.Message)); building from source..."
  if (Test-Path $Dest) { Remove-Item -Force $Dest }
}

if (-not $downloaded) {
  Set-Location $Root
  Write-Host "-> building release..."
  cargo build -p ide-app --release
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
  $Src = Join-Path $Root "target\release\zap.exe"
  if (-not (Test-Path $Src)) {
    throw "binary not found at $Src (need Rust toolchain + cargo)"
  }
  Copy-Item -Force $Src $Dest
  Write-Host "-> installed: $Dest"
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$DestDir*") {
  Write-Host "-> adding $DestDir to user PATH"
  [Environment]::SetEnvironmentVariable("Path", "$UserPath;$DestDir", "User")
  Write-Host "   reopen the terminal for PATH to apply"
}

Write-Host "done. Run: zap"
Write-Host "PowerShell uses Consolas-safe icons automatically."
Write-Host "Nerd Font icons: install a Nerd Font, or set [theme] icons = `"nerd`" in config.toml."
