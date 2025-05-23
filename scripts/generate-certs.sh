#!/bin/bash

# Generate self-signed certificates for development/testing
# This script creates a certificate authority and then generates a server certificate

set -e

CERT_DIR="./certs"
DAYS=365
KEY_SIZE=2048

# Create certs directory if it doesn't exist
mkdir -p "$CERT_DIR"

echo "Generating self-signed certificates for development..."

# Generate CA private key
echo "1. Generating CA private key..."
openssl genrsa -out "$CERT_DIR/ca-key.pem" $KEY_SIZE

# Generate CA certificate
echo "2. Generating CA certificate..."
openssl req -new -x509 -key "$CERT_DIR/ca-key.pem" -out "$CERT_DIR/ca-cert.pem" -days $DAYS -subj "/C=US/ST=CA/L=San Francisco/O=IAM Service/OU=Development/CN=IAM-CA"

# Generate server private key
echo "3. Generating server private key..."
openssl genrsa -out "$CERT_DIR/key.pem" $KEY_SIZE

# Generate server certificate signing request
echo "4. Generating server certificate signing request..."
openssl req -new -key "$CERT_DIR/key.pem" -out "$CERT_DIR/server.csr" -subj "/C=US/ST=CA/L=San Francisco/O=IAM Service/OU=Development/CN=localhost" -config <(
cat <<EOF
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
EOF
)

# Generate server certificate signed by CA
echo "5. Generating server certificate..."
openssl x509 -req -in "$CERT_DIR/server.csr" -CA "$CERT_DIR/ca-cert.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial -out "$CERT_DIR/cert.pem" -days $DAYS -extensions v3_req -extfile <(
cat <<EOF
[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1 = 127.0.0.1
IP.2 = ::1
EOF
)

# Clean up CSR file
rm "$CERT_DIR/server.csr"

# Set appropriate permissions
chmod 600 "$CERT_DIR"/*.pem

echo ""
echo "✅ Certificates generated successfully!"
echo ""
echo "Generated files:"
echo "  📁 $CERT_DIR/"
echo "    🔑 key.pem       - Server private key"
echo "    📜 cert.pem      - Server certificate"
echo "    🔑 ca-key.pem    - CA private key"
echo "    📜 ca-cert.pem   - CA certificate"
echo ""
echo "To enable HTTPS in your service:"
echo "1. Update config.toml: set tls_enabled = true"
echo "2. Ensure cert paths point to: $CERT_DIR/cert.pem and $CERT_DIR/key.pem"
echo "3. For browsers to trust the certificate, import $CERT_DIR/ca-cert.pem as a trusted CA"
echo ""
echo "⚠️  These are self-signed certificates for development only!"
echo "   For production, use certificates from a trusted CA like Let's Encrypt." 