#!/bin/bash

# Generate TLS certificates for HTTPS development/testing
# This script creates self-signed TLS certificates for local development

set -e

# Default values
CERT_DIR="./config/certs"
DAYS=365
TLS_KEY_SIZE=2048
COMMON_NAME="localhost"

# Color codes
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
WHITE='\033[0;37m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Function to print colored output
print_color() {
    echo -e "${1}${2}${NC}"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--cert-dir)
            CERT_DIR="$2"
            shift 2
            ;;
        --days)
            DAYS="$2"
            shift 2
            ;;
        -s|--key-size)
            TLS_KEY_SIZE="$2"
            shift 2
            ;;
        -n|--common-name)
            COMMON_NAME="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  -d, --cert-dir DIR    Certificate directory (default: ./config/certs)"
            echo "  --days DAYS          Certificate validity in days (default: 365)"
            echo "  -s, --key-size SIZE  TLS key size in bits (default: 2048)"
            echo "  -n, --common-name CN Common name for certificate (default: localhost)"
            echo "  -h, --help           Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Create certs directory if it doesn't exist
if [ ! -d "$CERT_DIR" ]; then
    mkdir -p "$CERT_DIR"
    print_color $GREEN "Created directory: $CERT_DIR"
fi

print_color $GREEN "Generating TLS certificates for HTTPS..."
print_color $CYAN "   Target directory: $CERT_DIR"
print_color $CYAN "   Key size: $TLS_KEY_SIZE bits"
print_color $CYAN "   Valid for: $DAYS days"
print_color $CYAN "   Common Name: $COMMON_NAME"
echo ""

# Check if OpenSSL is available
if ! command -v openssl &> /dev/null; then
    print_color $RED "Error: OpenSSL is not installed or not in PATH"
    echo ""
    print_color $YELLOW "Please install OpenSSL:"
    print_color $WHITE "  Ubuntu/Debian: sudo apt-get install openssl"
    print_color $WHITE "  CentOS/RHEL: sudo yum install openssl"
    print_color $WHITE "  macOS: brew install openssl"
    exit 1
fi

print_color $YELLOW "Using OpenSSL for certificate generation..."

# Generate CA private key
print_color $WHITE "1. Generating CA private key..."
if ! openssl genrsa -out "$CERT_DIR/ca-key.pem" $TLS_KEY_SIZE; then
    print_color $RED "Failed to generate CA private key"
    exit 1
fi

# Generate CA certificate
print_color $WHITE "2. Generating CA certificate..."
if ! openssl req -new -x509 -key "$CERT_DIR/ca-key.pem" -out "$CERT_DIR/ca-cert.pem" -days $DAYS -subj "/C=FR/ST=FR/L=Nice/O=IAM Service/OU=Development/CN=IAM-CA"; then
    print_color $RED "Failed to generate CA certificate"
    exit 1
fi

# Generate server private key for TLS
print_color $WHITE "3. Generating TLS server private key..."
if ! openssl genrsa -out "$CERT_DIR/tls-key.pem" $TLS_KEY_SIZE; then
    print_color $RED "Failed to generate TLS private key"
    exit 1
fi

# Create temporary config file for certificate generation
CONFIG_FILE="$CERT_DIR/temp-config.conf"
cat > "$CONFIG_FILE" << EOF
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
CN = $COMMON_NAME

[v3_req]
keyUsage        = keyEncipherment, dataEncipherment
extendedKeyUsage = clientAuth, serverAuth
subjectAltName  = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1  = 127.0.0.1
IP.2  = ::1
EOF

# Generate server certificate signing request
print_color $WHITE "4. Generating TLS certificate signing request..."
if ! openssl req -new -key "$CERT_DIR/tls-key.pem" -out "$CERT_DIR/server.csr" -config "$CONFIG_FILE"; then
    print_color $RED "Failed to generate certificate signing request"
    exit 1
fi

# Generate server certificate signed by CA
print_color $WHITE "5. Generating TLS server certificate..."
if ! openssl x509 -req -in "$CERT_DIR/server.csr" -CA "$CERT_DIR/ca-cert.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial -out "$CERT_DIR/tls-cert.pem" -days $DAYS -extensions v3_req -extfile "$CONFIG_FILE"; then
    print_color $RED "Failed to generate server certificate"
    exit 1
fi

# Clean up temporary files
rm -f "$CERT_DIR/server.csr" "$CONFIG_FILE"

echo ""
print_color $GREEN "TLS certificates generated successfully!"

echo ""
print_color $CYAN "Generated TLS files in $CERT_DIR/:"

# Display generated files
for file in "ca-key.pem" "ca-cert.pem" "tls-key.pem" "tls-cert.pem"; do
    if [ -f "$CERT_DIR/$file" ]; then
        size=$(du -h "$CERT_DIR/$file" | cut -f1)
        case $file in
            "tls-key.pem")
                print_color $BLUE "  $file - TLS private key ($size)"
                ;;
            "tls-cert.pem")
                print_color $BLUE "  $file - TLS certificate ($size)"
                ;;
            "ca-key.pem")
                print_color $MAGENTA "  $file - CA private key ($size)"
                ;;
            "ca-cert.pem")
                print_color $MAGENTA "  $file - CA certificate ($size)"
                ;;
        esac
    fi
done

echo ""
print_color $CYAN "Configuration for your service:"
print_color $WHITE "  [server]"
print_color $WHITE "  tls_enabled = true"
print_color $WHITE "  tls_cert_path = \"$CERT_DIR/tls-cert.pem\""
print_color $WHITE "  tls_key_path = \"$CERT_DIR/tls-key.pem\""
print_color $WHITE "  tls_port = 8443"
echo ""
print_color $RED "Security Notes:"
print_color $WHITE "  • These are self-signed certificates for development only!"
print_color $WHITE "  • Browsers will show security warnings"
print_color $WHITE "  • For production, use certificates from a trusted CA"
echo ""
print_color $YELLOW "To trust the CA certificate in your browser:"
print_color $WHITE "  Import $CERT_DIR/ca-cert.pem into your browser's trusted root certificates"