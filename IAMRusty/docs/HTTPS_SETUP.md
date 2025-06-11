# HTTPS Setup Guide

This guide explains how to configure your IAM service to run on HTTPS instead of HTTP.

## Table of Contents

- [Development Setup](#development-setup)
- [Production Setup](#production-setup)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

## Development Setup

For development and testing, you can use self-signed certificates.

### Prerequisites

- OpenSSL (recommended) or PowerShell (Windows)
- Rust toolchain

### Quick Start

1. **Generate Certificates**

   **Option A: Using the provided script (Unix/macOS/Linux with OpenSSL)**
   ```bash
   chmod +x scripts/generate-certs.sh
   ./scripts/generate-certs.sh
   ```

   **Option B: Using PowerShell (Windows)**
   ```powershell
   .\scripts\generate-certs.ps1
   ```

   **Option C: Manual generation with OpenSSL**
   ```bash
   mkdir -p certs

   # Generate private key
   openssl genrsa -out certs/key.pem 2048

   # Generate certificate
   openssl req -new -x509 -key certs/key.pem -out certs/cert.pem -days 365 \
     -subj "/C=US/ST=CA/L=San Francisco/O=IAM Service/CN=localhost" \
     -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
   ```

2. **Update Configuration**

   Edit your `config.toml`:
   ```toml
   [server]
   host = "127.0.0.1"
   port = 8080
   tls_enabled = true
   tls_cert_path = "./certs/cert.pem"
   tls_key_path = "./certs/key.pem"
   tls_port = 8443
   ```

   Or use environment variables:
   ```bash
   export APP_SERVER_TLS_ENABLED=true
   export APP_SERVER_TLS_CERT_PATH=./certs/cert.pem
   export APP_SERVER_TLS_KEY_PATH=./certs/key.pem
   export APP_SERVER_TLS_PORT=8443
   ```

3. **Update OAuth Redirect URLs**

   Update your OAuth provider configurations to use HTTPS:
   ```toml
   [oauth.github]
   redirect_uri = "https://localhost:8443/auth/github/callback"

   [oauth.gitlab]
   redirect_uri = "https://localhost:8443/auth/gitlab/callback"
   ```

4. **Run the Service**
   ```bash
   cargo run
   ```

   The service will now be available at `https://localhost:8443`

5. **Trust the Certificate (Optional)**

   For browsers to trust your self-signed certificate:
   - **Chrome/Edge**: Visit `https://localhost:8443`, click "Advanced", then "Proceed to localhost"
   - **Firefox**: Visit the URL, click "Advanced", then "Accept the Risk"
   - **System-wide**: Import `certs/ca-cert.pem` (if generated with the script) to your system's trusted CAs

## Production Setup

For production, use certificates from a trusted Certificate Authority (CA).

### Let's Encrypt (Recommended)

1. **Install Certbot**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install certbot

   # CentOS/RHEL
   sudo yum install certbot

   # macOS
   brew install certbot
   ```

2. **Generate Certificates**
   ```bash
   # Replace example.com with your domain
   sudo certbot certonly --standalone -d iam.example.com
   ```

3. **Configure the Service**
   ```toml
   [server]
   host = "0.0.0.0"
   port = 80
   tls_enabled = true
   tls_cert_path = "/etc/letsencrypt/live/iam.example.com/fullchain.pem"
   tls_key_path = "/etc/letsencrypt/live/iam.example.com/privkey.pem"
   tls_port = 443
   ```

4. **Set Up Auto-renewal**
   ```bash
   # Add to crontab
   0 12 * * * /usr/bin/certbot renew --quiet --post-hook "systemctl restart iam-service"
   ```

### Commercial Certificates

1. **Purchase a Certificate** from providers like:
   - DigiCert
   - Comodo
   - GlobalSign
   - Cloudflare

2. **Install the Certificate**
   - Place the certificate file (usually `.crt` or `.pem`) in a secure location
   - Place the private key file (usually `.key` or `.pem`) in a secure location
   - Set appropriate permissions: `chmod 600 /path/to/certificates/*`

3. **Configure the Service**
   ```toml
   [server]
   host = "0.0.0.0"
   port = 80
   tls_enabled = true
   tls_cert_path = "/etc/ssl/certs/iam.example.com.crt"
   tls_key_path = "/etc/ssl/private/iam.example.com.key"
   tls_port = 443
   ```

## Configuration

### Configuration File Options

```toml
[server]
# Server host (use "0.0.0.0" for production to bind to all interfaces)
host = "127.0.0.1"

# HTTP port (used when TLS is disabled)
port = 8080

# Enable/disable HTTPS
tls_enabled = false

# Path to TLS certificate file (PEM format)
tls_cert_path = "./certs/cert.pem"

# Path to TLS private key file (PEM format)
tls_key_path = "./certs/key.pem"

# HTTPS port (used when TLS is enabled)
tls_port = 8443
```

### Environment Variables

You can override any configuration using environment variables with the `APP_` prefix:

```bash
APP_SERVER_HOST=0.0.0.0
APP_SERVER_PORT=8080
APP_SERVER_TLS_ENABLED=true
APP_SERVER_TLS_CERT_PATH=/etc/ssl/certs/cert.pem
APP_SERVER_TLS_KEY_PATH=/etc/ssl/private/key.pem
APP_SERVER_TLS_PORT=443
```

### Docker Configuration

Update your `docker-compose.yml`:

```yaml
services:
  iam-service:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"   # HTTP port
      - "8443:8443"   # HTTPS port
    environment:
      - APP_SERVER_TLS_ENABLED=true
      - APP_SERVER_TLS_CERT_PATH=/app/certs/cert.pem
      - APP_SERVER_TLS_KEY_PATH=/app/certs/key.pem
    volumes:
      - ./certs:/app/certs
```

## Security Considerations

### File Permissions

Ensure certificate files have restrictive permissions:
```bash
chmod 600 /path/to/certificates/*.pem
chown app:app /path/to/certificates/*.pem  # If running as non-root user
```

### Firewall Configuration

Update your firewall rules to allow HTTPS traffic:
```bash
# UFW (Ubuntu)
sudo ufw allow 443/tcp

# iptables
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# firewalld (CentOS/RHEL)
sudo firewall-cmd --add-service=https --permanent
sudo firewall-cmd --reload
```

### HTTP to HTTPS Redirect

You may want to redirect HTTP traffic to HTTPS. You can do this at the:

1. **Reverse Proxy Level** (Nginx, Apache, Traefik)
2. **Load Balancer Level** (AWS ALB, CloudFlare)
3. **Application Level** (by running both HTTP and HTTPS servers)

Example Nginx configuration:
```nginx
server {
    listen 80;
    server_name iam.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name iam.example.com;
    
    ssl_certificate /etc/ssl/certs/iam.example.com.crt;
    ssl_certificate_key /etc/ssl/private/iam.example.com.key;
    
    location / {
        proxy_pass https://localhost:8443;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Troubleshooting

### Common Issues

1. **"Certificate file not found" error**
   - Verify the certificate file paths in your configuration
   - Ensure the application has read permissions to the certificate files

2. **"Invalid certificate" browser warning**
   - For development: This is expected with self-signed certificates
   - For production: Verify your certificate is from a trusted CA and covers the correct domain

3. **"Connection refused" when accessing HTTPS**
   - Verify `tls_enabled = true` in your configuration
   - Check that the HTTPS port is correct
   - Ensure the service is binding to the correct interface

4. **OAuth callback errors**
   - Update your OAuth provider redirect URLs to use HTTPS
   - Ensure the ports match your configuration

### Debugging Commands

```bash
# Test certificate validity
openssl x509 -in certs/cert.pem -text -noout

# Test HTTPS connection
curl -k https://localhost:8443/health

# Check if service is listening on HTTPS port
netstat -tlnp | grep :8443

# View service logs
docker-compose logs iam-service
```

### Support

If you encounter issues:
1. Check the application logs for error messages
2. Verify your certificate and key files are valid
3. Ensure all configuration parameters are correctly set
4. Test with a simple curl command first before using a browser

For additional help, please create an issue in the project repository. 