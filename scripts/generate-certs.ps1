# Generate self-signed certificates for development/testing
# This script creates a certificate authority and then generates a server certificate

param(
    [string]$CertDir = "./certs",
    [int]$Days = 365
)

$ErrorActionPreference = "Stop"

# Create certs directory if it doesn't exist
if (!(Test-Path $CertDir)) {
    New-Item -ItemType Directory -Path $CertDir -Force | Out-Null
}

Write-Host "Generating self-signed certificates for development..." -ForegroundColor Green

try {
    # Check if OpenSSL is available
    $null = Get-Command openssl -ErrorAction Stop
    Write-Host "Using OpenSSL for certificate generation..." -ForegroundColor Yellow
    
    # Generate CA private key
    Write-Host "1. Generating CA private key..."
    & openssl genrsa -out "$CertDir/ca-key.pem" 2048
    
    # Generate CA certificate
    Write-Host "2. Generating CA certificate..."
    & openssl req -new -x509 -key "$CertDir/ca-key.pem" -out "$CertDir/ca-cert.pem" -days $Days -subj "/C=US/ST=CA/L=San Francisco/O=IAM Service/OU=Development/CN=IAM-CA"
    
    # Generate server private key
    Write-Host "3. Generating server private key..."
    & openssl genrsa -out "$CertDir/key.pem" 2048
    
    # Create temporary config file for certificate generation
    $configContent = @"
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = CA
L = San Francisco
O = IAM Service
OU = Development
CN = localhost

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1 = 127.0.0.1
IP.2 = ::1
"@
    
    $configFile = "$CertDir/temp-config.conf"
    Set-Content -Path $configFile -Value $configContent
    
    # Generate server certificate signing request
    Write-Host "4. Generating server certificate signing request..."
    & openssl req -new -key "$CertDir/key.pem" -out "$CertDir/server.csr" -config $configFile
    
    # Generate server certificate signed by CA
    Write-Host "5. Generating server certificate..."
    & openssl x509 -req -in "$CertDir/server.csr" -CA "$CertDir/ca-cert.pem" -CAkey "$CertDir/ca-key.pem" -CAcreateserial -out "$CertDir/cert.pem" -days $Days -extensions v3_req -extfile $configFile
    
    # Clean up temporary files
    Remove-Item "$CertDir/server.csr" -Force
    Remove-Item $configFile -Force
    
} catch {
    Write-Host "OpenSSL not found. Using PowerShell certificate generation..." -ForegroundColor Yellow
    
    # Fallback to PowerShell certificate generation
    Write-Host "1. Generating CA certificate..."
    $caCert = New-SelfSignedCertificate -Subject "CN=IAM-CA" -CertStoreLocation "Cert:\CurrentUser\My" -KeyUsage CertSign -KeyUsageProperty All -KeyLength 2048 -NotAfter (Get-Date).AddDays($Days) -HashAlgorithm SHA256
    
    Write-Host "2. Generating server certificate..."
    $serverCert = New-SelfSignedCertificate -Subject "CN=localhost" -DnsName @("localhost", "127.0.0.1") -CertStoreLocation "Cert:\CurrentUser\My" -Signer $caCert -KeyLength 2048 -NotAfter (Get-Date).AddDays($Days) -HashAlgorithm SHA256
    
    # Export certificates
    Write-Host "3. Exporting certificates..."
    $caPassword = ConvertTo-SecureString -String "password" -Force -AsPlainText
    $serverPassword = ConvertTo-SecureString -String "password" -Force -AsPlainText
    
    Export-Certificate -Cert $caCert -FilePath "$CertDir/ca-cert.cer"
    Export-Certificate -Cert $serverCert -FilePath "$CertDir/cert.cer"
    Export-PfxCertificate -Cert $caCert -FilePath "$CertDir/ca-cert.pfx" -Password $caPassword
    Export-PfxCertificate -Cert $serverCert -FilePath "$CertDir/cert.pfx" -Password $serverPassword
    
    # Convert to PEM format (requires OpenSSL or manual conversion)
    Write-Host ""
    Write-Host "⚠️  Certificates generated in Windows format (.cer, .pfx)" -ForegroundColor Yellow
    Write-Host "   For the Rust service, you'll need to convert to PEM format or install OpenSSL" -ForegroundColor Yellow
    
    # Clean up from certificate store
    Remove-Item "Cert:\CurrentUser\My\$($caCert.Thumbprint)" -Force
    Remove-Item "Cert:\CurrentUser\My\$($serverCert.Thumbprint)" -Force
}

Write-Host ""
Write-Host "✅ Certificates generated successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Generated files in $CertDir/:" -ForegroundColor Cyan
Get-ChildItem $CertDir | ForEach-Object { Write-Host "    📜 $($_.Name)" -ForegroundColor White }
Write-Host ""
Write-Host "To enable HTTPS in your service:" -ForegroundColor Yellow
Write-Host "1. Update config.toml: set tls_enabled = true" -ForegroundColor White
Write-Host "2. Ensure cert paths point to the generated .pem files" -ForegroundColor White
Write-Host "3. For browsers to trust the certificate, import the CA certificate" -ForegroundColor White
Write-Host ""
Write-Host "⚠️  These are self-signed certificates for development only!" -ForegroundColor Red
Write-Host "   For production, use certificates from a trusted CA like Let's Encrypt." -ForegroundColor Red 