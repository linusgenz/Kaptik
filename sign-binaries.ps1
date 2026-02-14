$script:SignTool = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe"

function Sign-File {
    param(
        [string]$FilePath,
        [string]$CertThumbprint = "",
        [string]$CertPath = "",
        [string]$CertPassword = "",
        [string]$TimestampServer = "http://timestamp.digicert.com"
    )

    if (-not (Test-Path $FilePath)) {
        Write-Warning "File not found: $FilePath"
        return $false
    }

    Write-Host "Signing: $FilePath" -ForegroundColor Cyan

    $args = @("sign", "/fd", "SHA256", "/tr", $TimestampServer, "/td", "SHA256")

    if ($CertThumbprint) {
        $args += @("/sha1", $CertThumbprint)
    } elseif ($CertPath) {
        $args += @("/f", $CertPath)
        if ($CertPassword) {
            $args += @("/p", $CertPassword)
        }
    } else {
        Write-Error "No certificate specified. Use -CertThumbprint or -CertPath"
        return $false
    }

    $args += $FilePath

    if (-not (Test-Path $SignTool)) {
        Write-Error "SignTool not found at: $SignTool"
        return $false
    }

    & "$SignTool" @args

    if ($LASTEXITCODE -eq 0) {
        Write-Host "Signed successfully: $FilePath" -ForegroundColor Green
        return $true
    } else {
        Write-Error "Failed to sign: $FilePath"
        return $false
    }
}
