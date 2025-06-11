# Certificate Setup Guide

## Overview

The IAM service uses two types of certificates:

1. **TLS Certificates**: For HTTPS/TLS server connections
2. **JWT Certificates**: For JWT token signing and verification (RSA)

This guide covers setup for both types with security best practices.

## Table of Contents

- [JWT Certificates (RSA Key Pairs)](#jwt-certificates-rsa-key-pairs)
- [TLS Certificates (HTTPS)](#tls-certificates-https)
- [Certificate Storage](#certificate-storage)
- [Security Best Practices](#security-best-practices)
- [Production Deployment](#production-deployment)
- [Troubleshooting](#troubleshooting)

## JWT Certificates (RSA Key Pairs)

JWT certificates are used for signing and verifying JWT tokens. RSA certificates enable distributed token verification without sharing private keys.

### Key Generation

#### Option 1: Generate RSA Key Pair for JWT

```bash
# Create the certificates directory
mkdir -p config/certs

# Navigate to the directory
cd config/certs

# Generate private key (4096-bit recommended for production)
openssl genrsa -out key.pem 4096

# Extract public key
openssl rsa -in key.pem -pubout -out public-key.pem

# Verify the keys
openssl rsa -in key.pem -check
openssl rsa -pubin -in public-key.pem -text -noout
```

#### Option 2: Generate with Key ID (for JWKS)

```bash
# Generate with explicit key ID for better tracking
KID=$(openssl rand -hex 8)
echo "Generating keys with Key ID: $KID"

openssl genrsa -out key.pem 4096
openssl rsa -in key.pem -pubout -out public-key.pem

# Store the key ID for reference
echo "$KID" > key-id.txt
echo "Key ID stored in key-id.txt"
```

### Configuration

Update your configuration to use the generated certificates:

**config/production.toml**:
```toml
[jwt]
expiration_seconds = 86400

[jwt.secret_storage]
type = "PemFile"
private_key_path = "config/certs/key.pem"
public_key_path = "config/certs/public-key.pem"
```

**Environment Variables**:
```bash
APP_JWT_SECRET_STORAGE__TYPE=PemFile
APP_JWT_SECRET_STORAGE__PRIVATE_KEY_PATH=config/certs/key.pem
APP_JWT_SECRET_STORAGE__PUBLIC_KEY_PATH=config/certs/public-key.pem
```

### File Permissions

Set secure permissions for JWT certificates:

```bash
# Private key - owner read/write only
chmod 600 config/certs/key.pem

# Public key - world readable
chmod 644 config/certs/public-key.pem

# Verify permissions
ls -la config/certs/
```

## TLS Certificates (HTTPS)

TLS certificates secure HTTP connections between clients and the server.

### Development Certificates (Self-Signed)

For development and testing environments:

```bash
# Create TLS certificates directory
mkdir -p certs

# Generate private key for TLS
openssl genrsa -out certs/key.pem 2048

# Generate self-signed certificate
openssl req -new -x509 -key certs/key.pem -out certs/cert.pem -days 365 \
  -subj "/C=US/ST=CA/L=San Francisco/O=IAM Service/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,DNS:127.0.0.1,IP:127.0.0.1"

# Verify the certificate
openssl x509 -in certs/cert.pem -text -noout
```

### Production Certificates

#### Let's Encrypt (Recommended)

```bash
# Install Certbot
sudo apt-get update
sudo apt-get install certbot

# Generate certificate for your domain
sudo certbot certonly --standalone -d iam.yourdomain.com

# Certificates will be stored in:
# /etc/letsencrypt/live/iam.yourdomain.com/fullchain.pem (certificate)
# /etc/letsencrypt/live/iam.yourdomain.com/privkey.pem (private key)
```

#### Commercial Certificate Authority

1. **Generate Certificate Signing Request (CSR)**:
   ```bash
   # Generate private key
   openssl genrsa -out iam.yourdomain.com.key 2048
   
   # Generate CSR
   openssl req -new -key iam.yourdomain.com.key -out iam.yourdomain.com.csr \
     -subj "/C=US/ST=CA/L=San Francisco/O=Your Company/CN=iam.yourdomain.com"
   ```

2. **Submit CSR to Certificate Authority**

3. **Install received certificate**:
   ```bash
   # Place certificate and key in secure location
   sudo cp iam.yourdomain.com.crt /etc/ssl/certs/
   sudo cp iam.yourdomain.com.key /etc/ssl/private/
   sudo chmod 644 /etc/ssl/certs/iam.yourdomain.com.crt
   sudo chmod 600 /etc/ssl/private/iam.yourdomain.com.key
   ```

### TLS Configuration

**config/production.toml**:
```toml
[server]
host = "0.0.0.0"
port = 8080
tls_enabled = true
tls_cert_path = "/etc/letsencrypt/live/iam.yourdomain.com/fullchain.pem"
tls_key_path = "/etc/letsencrypt/live/iam.yourdomain.com/privkey.pem"
tls_port = 8443
```

## Certificate Storage

### Directory Structure

Recommended directory structure for certificates:

```
├── certs/                          # TLS certificates
│   ├── cert.pem                   # TLS certificate
│   └── key.pem                    # TLS private key
├── config/
│   └── certs/                     # JWT certificates
│       ├── .gitignore            # Prevent committing private keys
│       ├── README.md             # Setup instructions
│       ├── key.pem               # JWT private key
│       ├── public-key.pem        # JWT public key
│       └── key-id.txt            # Key ID for tracking
└── docker-compose.yml
```

### Environment-Specific Storage

#### Development
```
config/certs/       # JWT certificates
certs/             # TLS certificates (self-signed)
```

#### Production
```
/etc/ssl/private/   # Private keys (JWT + TLS)
/etc/ssl/certs/     # Public certificates (JWT + TLS)
```

### Container Storage

For Docker deployments:

```yaml
# docker-compose.yml
volumes:
  # TLS certificates
  - /etc/ssl/certs:/etc/ssl/certs:ro
  - /etc/ssl/private:/etc/ssl/private:ro
  
  # JWT certificates
  - ./config/certs:/app/config/certs:ro
```

## Security Best Practices

### File Permissions

```bash
# JWT certificates
chmod 600 config/certs/key.pem        # Private key
chmod 644 config/certs/public-key.pem # Public key

# TLS certificates
chmod 600 /etc/ssl/private/*.key      # Private keys
chmod 644 /etc/ssl/certs/*.pem        # Certificates
```

### User and Group Ownership

```bash
# Create dedicated user for the service
sudo useradd --system --no-create-home iam-service

# Set ownership
sudo chown iam-service:iam-service config/certs/key.pem
sudo chown root:iam-service /etc/ssl/private/jwt-key.pem
sudo chmod 640 /etc/ssl/private/jwt-key.pem
```

### Version Control Security

Create `.gitignore` files to prevent committing private keys:

**config/certs/.gitignore**:
```gitignore
# Never commit private keys
*.pem
*.key
*.p12
*.pfx

# Allow public keys and documentation
!public-key.pem
!README.md
```

### Key Rotation

#### JWT Key Rotation

1. **Generate new keys**:
   ```bash
   # Backup existing keys
   cp config/certs/key.pem config/certs/key.pem.backup
   cp config/certs/public-key.pem config/certs/public-key.pem.backup
   
   # Generate new keys
   openssl genrsa -out config/certs/key.pem 4096
   openssl rsa -in config/certs/key.pem -pubout -out config/certs/public-key.pem
   ```

2. **Deploy gradually**:
   - Deploy new public key to all verification services
   - Wait for propagation
   - Deploy new private key to signing service
   - Verify token generation and validation

3. **Monitor and rollback if needed**:
   ```bash
   # Rollback if issues occur
   cp config/certs/key.pem.backup config/certs/key.pem
   cp config/certs/public-key.pem.backup config/certs/public-key.pem
   ```

#### TLS Certificate Renewal

For Let's Encrypt auto-renewal:

```bash
# Set up automatic renewal
sudo crontab -e

# Add this line for renewal every 2 months
0 0 1 */2 * /usr/bin/certbot renew --quiet --post-hook "systemctl restart iam-service"
```

### Environment Separation

Use different certificates for different environments:

```bash
# Development
config/certs/dev-key.pem
config/certs/dev-public-key.pem

# Staging
config/certs/staging-key.pem
config/certs/staging-public-key.pem

# Production
/etc/ssl/private/prod-jwt-key.pem
/etc/ssl/certs/prod-jwt-public-key.pem
```

## Production Deployment

### Docker Production Setup

**Dockerfile additions**:
```dockerfile
# Create certificate directories
RUN mkdir -p /etc/ssl/certs /etc/ssl/private /app/config/certs

# Set permissions
RUN chown -R appuser:appuser /app/config/certs
```

**docker-compose.production.yml**:
```yaml
version: '3.8'

services:
  iam-service:
    image: iam-service:latest
    environment:
      - APP_JWT_SECRET_STORAGE__TYPE=PemFile
      - APP_JWT_SECRET_STORAGE__PRIVATE_KEY_PATH=/etc/ssl/private/jwt-key.pem
      - APP_JWT_SECRET_STORAGE__PUBLIC_KEY_PATH=/etc/ssl/certs/jwt-public-key.pem
      - APP_SERVER_TLS_ENABLED=true
      - APP_SERVER_TLS_CERT_PATH=/etc/ssl/certs/tls-cert.pem
      - APP_SERVER_TLS_KEY_PATH=/etc/ssl/private/tls-key.pem
    volumes:
      - /etc/ssl/certs:/etc/ssl/certs:ro
      - /etc/ssl/private:/etc/ssl/private:ro
    ports:
      - "443:8443"  # HTTPS only in production
```

### Kubernetes Deployment

**Certificate Secret**:
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: iam-jwt-certs
type: Opaque
data:
  private-key.pem: <base64-encoded-private-key>
  public-key.pem: <base64-encoded-public-key>
---
apiVersion: v1
kind: Secret
metadata:
  name: iam-tls-certs
type: kubernetes.io/tls
data:
  tls.crt: <base64-encoded-certificate>
  tls.key: <base64-encoded-private-key>
```

**Deployment**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: iam-service
spec:
  template:
    spec:
      containers:
      - name: iam-service
        image: iam-service:latest
        env:
        - name: APP_JWT_SECRET_STORAGE__TYPE
          value: "PemFile"
        - name: APP_JWT_SECRET_STORAGE__PRIVATE_KEY_PATH
          value: "/etc/ssl/jwt/private-key.pem"
        - name: APP_JWT_SECRET_STORAGE__PUBLIC_KEY_PATH
          value: "/etc/ssl/jwt/public-key.pem"
        volumeMounts:
        - name: jwt-certs
          mountPath: /etc/ssl/jwt
          readOnly: true
        - name: tls-certs
          mountPath: /etc/ssl/tls
          readOnly: true
      volumes:
      - name: jwt-certs
        secret:
          secretName: iam-jwt-certs
      - name: tls-certs
        secret:
          secretName: iam-tls-certs
```

## Troubleshooting

### Common Issues

#### Certificate Format Issues

```bash
# Check certificate format
openssl x509 -in cert.pem -text -noout

# Check private key format
openssl rsa -in key.pem -check

# Convert from other formats if needed
openssl pkcs12 -in cert.p12 -out cert.pem -nodes
```

#### Permission Issues

```bash
# Check file permissions
ls -la config/certs/
ls -la /etc/ssl/private/

# Fix permissions
sudo chmod 600 /etc/ssl/private/*.pem
sudo chmod 644 /etc/ssl/certs/*.pem
```

#### Key Mismatch

```bash
# Verify public/private key pair match
openssl rsa -in key.pem -pubout | openssl md5
openssl rsa -pubin -in public-key.pem | openssl md5
# Should output the same hash
```

### Validation Commands

```bash
# Test JWT certificate loading
openssl rsa -in config/certs/key.pem -check
openssl rsa -pubin -in config/certs/public-key.pem -text -noout

# Test TLS certificate
openssl s509 -in certs/cert.pem -text -noout

# Test HTTPS connection
curl -k https://localhost:8443/health

# Check JWKS endpoint
curl https://localhost:8443/.well-known/jwks.json
```

### Debug Logging

Enable debug logging for certificate loading:

```toml
[logging]
level = "debug"
modules = ["secret_storage", "jwt_encoder", "tls"]
```

```bash
# Watch logs for certificate issues
tail -f logs/app.log | grep -i "cert\|key\|tls\|jwt"
```

### Health Checks

The service provides health endpoints to verify certificate status:

```bash
# General health
curl https://localhost:8443/health

# JWT-specific health
curl https://localhost:8443/.well-known/jwks.json

# TLS certificate info
echo | openssl s_client -connect localhost:8443 2>/dev/null | openssl x509 -noout -text
``` 