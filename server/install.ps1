# Questline Installer for Windows
# Uso: irm https://raw.githubusercontent.com/gibranlp/Questline-cli/main/server/install.ps1 | iex
#
# Installs questline.exe to %LOCALAPPDATA%\Questline\ and adds it to the user PATH.

$ErrorActionPreference = "Stop"

$InstallDir  = "$env:LOCALAPPDATA\Questline"
$ConfigDir   = "$env:APPDATA\questline"
$BinaryName  = "questline-windows-x86_64.exe"
$DownloadUrl = "https://github.com/gibranlp/Questline-cli/releases/latest/download/$BinaryName"
$InstallPath = "$InstallDir\questline.exe"
$TempPath    = "$InstallDir\questline-update.exe"
$OldPath     = "$InstallDir\questline-old.exe"

Write-Host ""
Write-Host "  QUESTLINE - Windows Installer" -ForegroundColor Cyan
Write-Host "  ----------------------------------------"
Write-Host "  Platform  : Windows/x86_64"
Write-Host "  Binary    : $BinaryName"
Write-Host "  Install   : $InstallPath"
Write-Host "  Config    : $ConfigDir"
Write-Host ""

# ── Create directories ────────────────────────────────────────────────────────
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}
if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null
}

# ── Clean up any leftover temp files from a previous interrupted update ───────
if (Test-Path $TempPath) { Remove-Item -Force $TempPath -ErrorAction SilentlyContinue }
if (Test-Path $OldPath)  { Remove-Item -Force $OldPath  -ErrorAction SilentlyContinue }

# ── Download with progress bar ────────────────────────────────────────────────
function Draw-ProgressBar {
    param([int]$Percent, [long]$Received, [long]$Total)

    $barWidth = 38
    $filled   = [math]::Floor($barWidth * $Percent / 100)
    $empty    = $barWidth - $filled

    if ($Percent -ge 100) {
        $bar = "=" * $barWidth
    } elseif ($filled -gt 0) {
        $bar = ("=" * ($filled - 1)) + ">" + (" " * $empty)
    } else {
        $bar = " " * $barWidth
    }

    $recvMB = [math]::Round($Received / 1MB, 1)
    if ($Total -gt 0) {
        $totMB   = [math]::Round($Total / 1MB, 1)
        $sizeStr = "${recvMB} MB / ${totMB} MB"
    } else {
        $sizeStr = "${recvMB} MB"
    }

    $pctStr = $Percent.ToString().PadLeft(3)
    Write-Host -NoNewline "`r  [$bar] $pctStr%  $sizeStr   "
}

Write-Host "  Downloading $BinaryName..." -ForegroundColor DarkGray

$fileStream = $null
try {
    $request = [System.Net.HttpWebRequest]::Create($DownloadUrl)
    $request.UserAgent = "Questline-Installer/1.0"
    $response  = $request.GetResponse()
    $totalBytes = $response.ContentLength

    $stream     = $response.GetResponseStream()
    $fileStream = [System.IO.File]::Open($TempPath, [System.IO.FileMode]::Create)
    $buffer     = New-Object byte[] 65536
    $received   = 0L

    Draw-ProgressBar -Percent 0 -Received 0 -Total $totalBytes

    while ($true) {
        $read = $stream.Read($buffer, 0, $buffer.Length)
        if ($read -le 0) { break }
        $fileStream.Write($buffer, 0, $read)
        $received += $read
        $pct = if ($totalBytes -gt 0) { [int][math]::Min(100, [math]::Floor($received * 100 / $totalBytes)) } else { 0 }
        Draw-ProgressBar -Percent $pct -Received $received -Total $totalBytes
    }

    $fileStream.Close(); $fileStream = $null
    $stream.Close()
    $response.Close()

    Draw-ProgressBar -Percent 100 -Received $received -Total $received
    Write-Host ""
} catch {
    Write-Host ""
    Write-Host "  Error: Download failed." -ForegroundColor Red
    Write-Host "  URL: $DownloadUrl"
    Write-Host "  $_"
    if ($null -ne $fileStream) { $fileStream.Close() }
    if (Test-Path $TempPath) { Remove-Item -Force $TempPath -ErrorAction SilentlyContinue }
    exit 1
}

Write-Host "  " -NoNewline
Write-Host "✓ Download complete" -ForegroundColor Green

# ── Swap binaries ─────────────────────────────────────────────────────────────
# Windows locks exe data but permits renaming a running file, so we rename the
# old binary out of the way before moving the freshly downloaded one into place.
if (Test-Path $InstallPath) {
    try {
        Rename-Item -Path $InstallPath -NewName "questline-old.exe" -Force -ErrorAction Stop
    } catch {
        Write-Host "  Error: Could not move existing binary. Close Questline and try again." -ForegroundColor Red
        Remove-Item -Force $TempPath -ErrorAction SilentlyContinue
        exit 1
    }
}
Move-Item -Path $TempPath -Destination $InstallPath -Force
# Best-effort cleanup; old binary may still be held if the process is running
Remove-Item -Force $OldPath -ErrorAction SilentlyContinue

Write-Host "  " -NoNewline
Write-Host "✓ Installed" -ForegroundColor Green -NoNewline
Write-Host " → $InstallPath"

# ── Update user PATH ──────────────────────────────────────────────────────────
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    $newPath = "$currentPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    Write-Host ""
    Write-Host "  Added $InstallDir to your PATH." -ForegroundColor Yellow
    Write-Host "  PATH update takes effect in new terminals." -ForegroundColor DarkGray
}

# ── Launch ────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  Installation complete! Starting Questline..." -ForegroundColor Green
Write-Host "  ----------------------------------------"
Write-Host ""

& $InstallPath
