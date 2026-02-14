param(
    [switch]$Sign,
    [string]$CertThumbprint = "",
    [string]$CertPath = "",
    [string]$CertPassword = ""
)

$ErrorActionPreference = "Stop"

Write-Host "=== Kaptik Build & Package ===" -ForegroundColor Cyan

Write-Host "`n[1/6] Configuring CMake..." -ForegroundColor Yellow
cmake --preset release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "`n[2/6] Building project..." -ForegroundColor Yellow
cmake --build --preset release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($Sign) {
    Write-Host "`n[3/6] Signing executables..." -ForegroundColor Yellow
    
    . "$PSScriptRoot\sign-binaries.ps1"
    
    $signParams = @{
        TimestampServer = "http://timestamp.digicert.com"
    }
    
    if ($CertThumbprint) {
        $signParams.CertThumbprint = $CertThumbprint
    } elseif ($CertPath) {
        $signParams.CertPath = $CertPath
        if ($CertPassword) {
            $signParams.CertPassword = $CertPassword
        }
    } else {
        Write-Error "When using -Sign, provide -CertThumbprint or -CertPath"
        exit 1
    }
    
    $uiExe = "build\release\app\kaptik-ui.exe"
    if (-not (Sign-File -FilePath $uiExe `
                       -CertThumbprint $CertThumbprint `
                       -CertPath $CertPath `
                       -CertPassword $CertPassword `
                       -TimestampServer "http://timestamp.digicert.com")) {
        exit 1
    }

    $coreExe = "kaptik-core\target\release\kaptik-core.exe"
    if (-not (Sign-File -FilePath $coreExe `
                       -CertThumbprint $CertThumbprint `
                       -CertPath $CertPath `
                       -CertPassword $CertPassword `
                       -TimestampServer "http://timestamp.digicert.com")) {
        exit 1
    }
    
    Write-Host "All executables signed" -ForegroundColor Green
} else {
    Write-Host "`n[3/6] Skipping code signing (use -Sign to enable)" -ForegroundColor Gray
}

Write-Host "`n[4/6] Installing files..." -ForegroundColor Yellow
cmake --install build/release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "`n[5/6] Creating installer package..." -ForegroundColor Yellow
cpack --config build/release/CPackConfig.cmake
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($Sign) {
    Write-Host "`n[6/6] Signing installer..." -ForegroundColor Yellow
    
    $installer = Get-ChildItem -Path "build\release" -Filter "Kaptik-*-win64.exe" | Select-Object -First 1
    
    if ($installer) {
        if (-not (Sign-File -FilePath $installer.FullName @signParams)) {
            exit 1
        }
        Write-Host "Installer signed: $($installer.Name)" -ForegroundColor Green
    } else {
        Write-Warning "Installer not found for signing"
    }
} else {
    Write-Host "`n[6/6] Skipping installer signing" -ForegroundColor Gray
}

Write-Host "`n=== Build Complete ===" -ForegroundColor Green