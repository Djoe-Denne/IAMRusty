# Generate certificates and JWT signing keys for development/testing
# This script creates TLS certificates and RSA keys for JWT signing

param(
    [string]$CertDir = "./config/certs",
    [int]$Days = 365,
    [int]$JwtKeySize = 4096,  # Larger key size for JWT validation
    [int]$TlsKeySize = 2048
)

$ErrorActionPreference = "Stop"

# Create certs directory if it doesn't exist
if (!(Test-Path $CertDir)) {
    New-Item -ItemType Directory -Path $CertDir -Force | Out-Null
    Write-Host "Created directory: $CertDir" -ForegroundColor Green
}

Write-Host "Generating certificates and JWT signing keys..." -ForegroundColor Green
Write-Host "   Target directory: $CertDir" -ForegroundColor Cyan
Write-Host "   JWT key size: $JwtKeySize bits" -ForegroundColor Cyan
Write-Host "   TLS key size: $TlsKeySize bits" -ForegroundColor Cyan
Write-Host ""

try {
    # Check if OpenSSL is available
    $null = Get-Command openssl -ErrorAction Stop
    Write-Host "Using OpenSSL for key generation..." -ForegroundColor Yellow
    
    # Generate JWT signing RSA key pair (high priority)
    Write-Host "Generating JWT RSA private key ($JwtKeySize bits)..."
    & openssl genrsa -out "$CertDir/key.pem" $JwtKeySize
    if ($LASTEXITCODE -ne 0) { throw "Failed to generate JWT private key" }
    
    Write-Host "Extracting JWT RSA public key..."
    & openssl rsa -in "$CertDir/key.pem" -pubout -out "$CertDir/public_key.pem"
    if ($LASTEXITCODE -ne 0) { throw "Failed to extract JWT public key" }
    
    # Verify the key pair
    Write-Host "Verifying JWT key pair..."
    & openssl rsa -in "$CertDir/key.pem" -check -noout
    if ($LASTEXITCODE -ne 0) { throw "JWT key verification failed" }
    
    Write-Host "JWT signing keys generated successfully!" -ForegroundColor Green
    Write-Host ""
    
    # Generate TLS certificates (for HTTPS)
    Write-Host "Generating TLS certificates..." -ForegroundColor Yellow
    
    # Generate CA private key
    Write-Host "1. Generating CA private key..."
    & openssl genrsa -out "$CertDir/ca-key.pem" $TlsKeySize
    
    # Generate CA certificate
    Write-Host "2. Generating CA certificate..."
    & openssl req -new -x509 -key "$CertDir/ca-key.pem" -out "$CertDir/ca-cert.pem" -days $Days -subj "/C=FR/ST=FR/L=Nice/O=IAM Service/OU=Development/CN=IAM-CA"
    
    # Generate server private key for TLS (separate from JWT keys)
    Write-Host "3. Generating TLS server private key..."
    & openssl genrsa -out "$CertDir/tls-key.pem" $TlsKeySize
    
    # Create temporary config file for certificate generation
    $configContent = @"
[req]
distinguished_name = req_distinguished_name
req_extensions     = v3_req
prompt             = no

[req_distinguished_name]
C  = FR
ST = FR
L  = Nice
O  = IAM Service
OU = Development
CN = localhost

[v3_req]
keyUsage        = keyEncipherment, dataEncipherment
extendedKeyUsage = clientAuth, serverAuth
subjectAltName  = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1  = 127.0.0.1
IP.2  = ::1
"@

    $configFile = "$CertDir/temp-config.conf"
    Set-Content -Path $configFile -Value $configContent
    
    # Generate server certificate signing request
    Write-Host "4. Generating TLS certificate signing request..."
    & openssl req -new -key "$CertDir/tls-key.pem" -out "$CertDir/server.csr" -config $configFile
    
    # Generate server certificate signed by CA
    Write-Host "5. Generating TLS server certificate..."
    & openssl x509 -req -in "$CertDir/server.csr" -CA "$CertDir/ca-cert.pem" -CAkey "$CertDir/ca-key.pem" -CAcreateserial -out "$CertDir/tls-cert.pem" -days $Days -extensions v3_req -extfile $configFile
    
    # Clean up temporary files
    Remove-Item "$CertDir/server.csr" -Force -ErrorAction SilentlyContinue
    Remove-Item $configFile -Force -ErrorAction SilentlyContinue

} catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "OpenSSL might not be installed or accessible." -ForegroundColor Yellow
    Write-Host "Please install OpenSSL from: https://slproweb.com/products/Win32OpenSSL.html" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "All keys and certificates generated successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Generated files in $CertDir/:" -ForegroundColor Cyan
$files = Get-ChildItem $CertDir | Sort-Object Name
foreach ($file in $files) {
    $size = [math]::Round($file.Length / 1KB, 1)
    switch ($file.Name) {
        "key.pem" { Write-Host "  $($file.Name) - JWT private key ($size KB)" -ForegroundColor Yellow }
        "public_key.pem" { Write-Host "  $($file.Name) - JWT public key ($size KB)" -ForegroundColor Yellow }
        "tls-key.pem" { Write-Host "  $($file.Name) - TLS private key ($size KB)" -ForegroundColor Blue }
        "tls-cert.pem" { Write-Host "  $($file.Name) - TLS certificate ($size KB)" -ForegroundColor Blue }
        "ca-key.pem" { Write-Host "  $($file.Name) - CA private key ($size KB)" -ForegroundColor Magenta }
        "ca-cert.pem" { Write-Host "  $($file.Name) - CA certificate ($size KB)" -ForegroundColor Magenta }
        default { Write-Host "  $($file.Name) ($size KB)" -ForegroundColor White }
    }
}

Write-Host ""
Write-Host "Usage Instructions:" -ForegroundColor Cyan
Write-Host ""
Write-Host "For JWT Configuration (test.toml):" -ForegroundColor Yellow
Write-Host "  [jwt.secret]" -ForegroundColor White
Write-Host "  type = `"pem_file`"" -ForegroundColor White
Write-Host "  private_key_path = `"$CertDir/key.pem`"" -ForegroundColor White
Write-Host "  public_key_path = `"$CertDir/public_key.pem`"" -ForegroundColor White
Write-Host "  key_id = `"jwt-key-test`"" -ForegroundColor White
Write-Host ""
Write-Host "For TLS Configuration (production.toml):" -ForegroundColor Yellow
Write-Host "  [server]" -ForegroundColor White
Write-Host "  tls_enabled = true" -ForegroundColor White
Write-Host "  tls_cert_path = `"$CertDir/tls-cert.pem`"" -ForegroundColor White
Write-Host "  tls_key_path = `"$CertDir/tls-key.pem`"" -ForegroundColor White
Write-Host ""
Write-Host "To run tests:" -ForegroundColor Green
Write-Host "  `$env:RUN_ENV=`"test`"; cargo test --test token" -ForegroundColor White
Write-Host ""
Write-Host "Security Notes:" -ForegroundColor Red
Write-Host "  • These are development/test keys only!" -ForegroundColor White
Write-Host "  • JWT private keys should be kept secure in production" -ForegroundColor White
Write-Host "  • For production, use proper key management (Vault, GCP, etc.)" -ForegroundColor White
