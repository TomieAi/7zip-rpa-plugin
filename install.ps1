# Installs Rpa.64.dll into 7-Zip Formats folder
$source = Join-Path $PSScriptRoot "Rpa.64.dll"
$dest = "C:\Program Files\7-Zip\Formats\Rpa.64.dll"

Write-Host "Copying $source to $dest..." -ForegroundColor Cyan

try {
    Copy-Item -Path $source -Destination $dest -Force -ErrorAction Stop
    Write-Host "[SUCCESS] Rpa.64.dll successfully installed into $dest!" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Please run PowerShell as Administrator to install into C:\Program Files\7-Zip\Formats" -ForegroundColor Yellow
}

