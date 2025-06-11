# JWT Configuration Guide

## Overview

The IAM service supports flexible JWT token generation and validation using either symmetric (HMAC) or asymmetric (RSA) cryptographic algorithms. The configuration system is designed to be extensible, supporting multiple secret storage backends while keeping the JWT encoder agnostic to the secret source.

## Table of Contents

- [Secret Storage Architecture](#secret-storage-architecture)
- [Configuration Options](#configuration-options)
- [HMAC (Symmetric) Configuration](#hmac-symmetric-configuration)
- [RSA (Asymmetric) Configuration](#rsa-asymmetric-configuration)
- [Certificate Management](#certificate-management)
- [Security Considerations](#security-considerations)
- [Future Extensions](#future-extensions)
- [Migration Guide](#migration-guide)
- [Troubleshooting](#troubleshooting)

## Secret Storage Architecture

The JWT configuration uses a layered architecture that separates secret storage from JWT operations:

```
┌─────────────────────────────────────────────────────────────┐
│                    JWT Token Service                        │
│                   (Algorithm Agnostic)                      │
├─────────────────────────────────────────────────────────────┤
│                   Secret Resolution                         │
│              (SecretStorage → JwtSecret)                    │
├─────────────────────────────────────────────────────────────┤
│                  Secret Storage Backend                     │
│           (PlainText | PemFile | Vault | GCP)               │
└─────────────────────────────────────────────────────────────┘
```

### Key Components

- **SecretStorage**: Configuration enum defining where/how secrets are stored
- **JwtSecret**: Resolved secret enum (HMAC key or RSA key pair)
- **JwtTokenService**: Algorithm-agnostic JWT encoder/decoder
- **Secret Resolution**: Converts storage config to usable secrets

## Configuration Options

### Base Structure

```toml
[jwt]
expiration_seconds = 3600

[jwt.secret_storage]
# Secret storage configuration (see options below)
```

### Available Storage Types

#### 1. Plain Text (Development/Legacy)

```toml
[jwt.secret_storage]
type = "PlainText"
secret = "your-hmac-secret-at-least-32-bytes-long"
```

#### 2. PEM Files (Recommended for Production)

```toml
[jwt.secret_storage]
type = "PemFile"
private_key_path = "config/certs/key.pem"
public_key_path = "config/certs/public-key.pem"
```

#### 3. HashiCorp Vault (Future)

```toml
[jwt.secret_storage]
type = "Vault"
vault_url = "https://vault.example.com"
secret_path = "secret/jwt-keys"
role = "iam-service"
```

#### 4. GCP Secret Manager (Future)

```toml
[jwt.secret_storage]
type = "GcpSecretManager"
project_id = "my-gcp-project"
secret_name = "jwt-rsa-keys"
version = "latest"
```

## HMAC (Symmetric) Configuration

HMAC uses a shared secret for both signing and verification.

### Configuration

```toml
[jwt]
expiration_seconds = 3600

[jwt.secret_storage]
type = "PlainText"
secret = "your-super-secure-hmac-secret-key-at-least-32-bytes-long"
```

### Security Requirements

- **Minimum Length**: 32 bytes (256 bits)
- **Randomness**: Use cryptographically secure random generation
- **Storage**: Store securely (environment variables, secret management)

### Generation Example

```bash
# Generate secure HMAC secret
openssl rand -base64 48
```

### Use Cases

- **Development**: Easy setup and testing
- **Simple Deployments**: Single-service architectures
- **Legacy Compatibility**: Existing HMAC-based systems

## RSA (Asymmetric) Configuration

RSA uses a private key for signing and public key for verification, enabling distributed verification.

### Configuration

```toml
[jwt]
expiration_seconds = 3600

[jwt.secret_storage]
type = "PemFile"
private_key_path = "config/certs/key.pem"
public_key_path = "config/certs/public-key.pem"
```

### Key Generation

Create RSA key pair using OpenSSL:

```bash
# Navigate to certs directory
cd config/certs

# Generate private key (2048-bit minimum, 4096-bit recommended)
openssl genrsa -out key.pem 4096

# Extract public key
openssl rsa -in key.pem -pubout -out public-key.pem

# Verify the keys
openssl rsa -in key.pem -check
openssl rsa -pubin -in public-key.pem -text -noout
```

### Security Requirements

- **Key Size**: Minimum 2048 bits, recommended 4096 bits
- **Format**: PKCS#8 PEM format for compatibility
- **Permissions**: Private key should be readable only by the service user

### File Permissions

```bash
# Set secure permissions
chmod 600 config/certs/key.pem       # Private key - owner read/write only
chmod 644 config/certs/public-key.pem # Public key - world readable
```

### Use Cases

- **Microservices**: Distributed token verification
- **Multi-Service**: Shared public key for verification
- **Zero-Trust**: Public key distribution without private key exposure
- **Compliance**: Meeting asymmetric cryptography requirements

## Certificate Management

### Directory Structure

```
config/certs/
├── README.md           # Instructions and documentation
├── .gitignore         # Prevent committing private keys
├── key.pem            # Private key (RSA, PKCS#8 PEM)
└── public-key.pem     # Public key (PKCS#8 PEM)
```

### Key Rotation

1. **Generate New Keys**:
   ```bash
   # Backup existing keys
   cp key.pem key.pem.backup
   cp public-key.pem public-key.pem.backup
   
   # Generate new keys
   openssl genrsa -out key.pem 4096
   openssl rsa -in key.pem -pubout -out public-key.pem
   ```

2. **Update Configuration**: No changes needed if using same file paths

3. **Restart Service**: The new keys will be loaded on restart

4. **Verify Deployment**: Check logs for successful secret resolution

### Backup and Recovery

```bash
# Create encrypted backup
tar -czf jwt-certs-backup-$(date +%Y%m%d).tar.gz config/certs/
gpg --symmetric --cipher-algo AES256 jwt-certs-backup-*.tar.gz

# Restore from backup
gpg --decrypt jwt-certs-backup-*.tar.gz.gpg | tar -xzf -
```

## Security Considerations

### Private Key Security

- **File Permissions**: 600 (owner read/write only)
- **User Isolation**: Run service as dedicated user
- **No Version Control**: Use .gitignore to prevent committing
- **Encrypted Storage**: Consider disk encryption for additional protection

### Public Key Distribution

- **JWKS Endpoint**: Public keys automatically exposed at `/.well-known/jwks.json`
- **Service Discovery**: Share public key with consuming services
- **Key Versioning**: Include kid (key ID) for multiple key support

### Operational Security

- **Secret Scanning**: Monitor for accidental exposure
- **Access Logging**: Log secret access and resolution
- **Key Rotation**: Regular rotation schedule (recommended: quarterly)
- **Monitoring**: Alert on secret resolution failures

### Network Security

- **TLS in Transit**: Always use HTTPS for JWT transmission
- **Secure Storage**: Use dedicated secret management systems in production
- **Network Isolation**: Restrict access to certificate storage

## Future Extensions

The architecture is designed to support additional secret storage backends:

### HashiCorp Vault Integration

```rust
pub struct VaultConfig {
    pub vault_url: String,
    pub secret_path: String,
    pub role: String,
    pub auth_method: VaultAuthMethod,
}
```

### GCP Secret Manager Integration

```rust
pub struct GcpSecretManagerConfig {
    pub project_id: String,
    pub secret_name: String,
    pub version: String,
}
```

### AWS Secrets Manager Integration

```rust
pub struct AwsSecretsManagerConfig {
    pub region: String,
    pub secret_name: String,
    pub version_stage: String,
}
```

## Migration Guide

### From HMAC to RSA

1. **Generate RSA Keys**:
   ```bash
   mkdir -p config/certs
   openssl genrsa -out config/certs/key.pem 4096
   openssl rsa -in config/certs/key.pem -pubout -out config/certs/public-key.pem
   ```

2. **Update Configuration**:
   ```toml
   # Before (HMAC)
   [jwt.secret_storage]
   type = "PlainText"
   secret = "hmac-secret"
   
   # After (RSA)
   [jwt.secret_storage]
   type = "PemFile"
   private_key_path = "config/certs/key.pem"
   public_key_path = "config/certs/public-key.pem"
   ```

3. **Update Consuming Services**: Share new public key via JWKS endpoint

4. **Deploy and Verify**: Check logs for successful secret resolution

### Zero-Downtime Migration

For production systems requiring zero downtime:

1. **Dual-Key Support**: Deploy service with both old and new keys
2. **Gradual Rollout**: Issue new tokens with new algorithm
3. **Validation Period**: Accept both token types during transition
4. **Complete Migration**: Remove old key support after validation period

## Troubleshooting

### Common Issues

#### Secret Resolution Failures

```bash
# Check logs for secret resolution errors
tail -f logs/app.log | grep "secret_resolution"

# Verify file permissions
ls -la config/certs/

# Test key validity
openssl rsa -in config/certs/key.pem -check
openssl rsa -pubin -in config/certs/public-key.pem -text -noout
```

#### JWT Validation Failures

```bash
# Check algorithm configuration
curl -s https://localhost:8443/.well-known/jwks.json | jq

# Verify token structure
echo "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9..." | jwt decode -

# Check expiration times
grep "token_expired" logs/app.log
```

#### Performance Issues

```bash
# Monitor secret loading time
grep "secret_loading_duration" logs/app.log

# Check memory usage for large keys
ps aux | grep iam-service
```

### Debug Configuration

Enable debug logging for JWT operations:

```toml
[logging]
level = "debug"
modules = ["jwt_encoder", "secret_storage"]
```

### Health Checks

The service provides health endpoints for JWT configuration:

```bash
# Check overall health
curl https://localhost:8443/health

# Check JWT-specific health (if available)
curl https://localhost:8443/health/jwt
```

### Testing Configuration

Test your JWT configuration:

```bash
# Test token generation (requires running service)
curl -X POST https://localhost:8443/auth/test-token \
  -H "Content-Type: application/json" \
  -d '{"user_id": "test-user"}'

# Validate generated token
curl -X GET https://localhost:8443/auth/validate \
  -H "Authorization: Bearer <token>"
``` 