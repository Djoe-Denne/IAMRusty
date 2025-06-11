#!/bin/bash

# Generate certificates and JWT signing keys for development/testing
# This script creates TLS certificates and RSA keys for JWT signing

set -e

CERT_DIR="./config/certs"
DAYS=365
JWT_KEY_SIZE=4096  # Larger key size for JWT validation
TLS_KEY_SIZE=2048

# Create certs directory if it doesn't exist
mkdir -p "$CERT_DIR"

echo "🔐 Generating certificates and JWT signing keys..."
echo "   Target directory: $CERT_DIR"
echo "   JWT key size: $JWT_KEY_SIZE bits"
echo "   TLS key size: $TLS_KEY_SIZE bits"
echo ""

# Generate JWT signing RSA key pair (high priority)
echo "🔑 Generating JWT RSA private key ($JWT_KEY_SIZE bits)..."
openssl genrsa -out "$CERT_DIR/key.pem" $JWT_KEY_SIZE

echo "🔑 Extracting JWT RSA public key..."
openssl rsa -in "$CERT_DIR/key.pem" -pubout -out "$CERT_DIR/public_key.pem"

# Verify the key pair
echo "✅ Verifying JWT key pair..."
openssl rsa -in "$CERT_DIR/key.pem" -check -noout

echo "✅ JWT signing keys generated successfully!"
echo ""

# Generate TLS certificates (for HTTPS)
echo "🔐 Generating TLS certificates..."

# Generate CA private key
echo "1. Generating CA private key..."
openssl genrsa -out "$CERT_DIR/ca-key.pem" $TLS_KEY_SIZE

# Generate CA certificate
echo "2. Generating CA certificate..."
openssl req -new -x509 -key "$CERT_DIR/ca-key.pem" -out "$CERT_DIR/ca-cert.pem" -days $DAYS -subj "/C=FR/ST=FR/L=Nice/O=IAM Service/OU=Development/CN=IAM-CA"

# Generate server private key for TLS (separate from JWT keys)
echo "3. Generating TLS server private key..."
openssl genrsa -out "$CERT_DIR/tls-key.pem" $TLS_KEY_SIZE

# Generate server certificate signing request
echo "4. Generating TLS certificate signing request..."
openssl req -new -key "$CERT_DIR/tls-key.pem" -out "$CERT_DIR/server.csr" -subj "/C=FR/ST=FR/L=Nice/O=IAM Service/OU=Development/CN=localhost" -config <(
cat <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = FR
ST = FR
L = Nice
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
echo "5. Generating TLS server certificate..."
openssl x509 -req -in "$CERT_DIR/server.csr" -CA "$CERT_DIR/ca-cert.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial -out "$CERT_DIR/tls-cert.pem" -days $DAYS -extensions v3_req -extfile <(
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
echo "🎉 All keys and certificates generated successfully!"
echo ""
echo "Generated files in $CERT_DIR/:"
for file in "$CERT_DIR"/*; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        size=$(ls -lh "$file" | awk '{print $5}')
        case "$filename" in
            "key.pem")
                echo "  🔑 $filename - JWT private key ($size)"
                ;;
            "public_key.pem")
                echo "  🔑 $filename - JWT public key ($size)"
                ;;
            "tls-key.pem")
                echo "  🔐 $filename - TLS private key ($size)"
                ;;
            "tls-cert.pem")
                echo "  📜 $filename - TLS certificate ($size)"
                ;;
            "ca-key.pem")
                echo "  🔐 $filename - CA private key ($size)"
                ;;
            "ca-cert.pem")
                echo "  📜 $filename - CA certificate ($size)"
                ;;
            *)
                echo "  📄 $filename ($size)"
                ;;
        esac
    fi
done

echo ""
echo "🔧 Usage Instructions:"
echo ""
echo "For JWT Configuration (test.toml):"
echo "  [jwt.secret]"
echo "  type = \"pem_file\""
echo "  private_key_path = \"$CERT_DIR/key.pem\""
echo "  public_key_path = \"$CERT_DIR/public_key.pem\""
echo "  key_id = \"jwt-key-test\""
echo ""
echo "For TLS Configuration (production.toml):"
echo "  [server]"
echo "  tls_enabled = true"
echo "  tls_cert_path = \"$CERT_DIR/tls-cert.pem\""
echo "  tls_key_path = \"$CERT_DIR/tls-key.pem\""
echo ""
echo "To run tests:"
echo "  RUN_ENV=test cargo test --test token"
echo ""
echo "⚠️  Security Notes:"
echo "  • These are development/test keys only!"
echo "  • JWT private keys should be kept secure in production"
echo "  • For production, use proper key management (Vault, GCP, etc.)" 