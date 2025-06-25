#!/bin/bash

# Generate JWT RSA signing keys for development/testing
# This script creates RSA key pairs for JWT token signing

set -e

# Default values
CERT_DIR="./config/certs"
JWT_KEY_SIZE=4096

# Color codes
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
WHITE='\033[0;37m'
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
        -s|--key-size)
            JWT_KEY_SIZE="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  -d, --cert-dir DIR    Certificate directory (default: ./config/certs)"
            echo "  -s, --key-size SIZE   JWT key size in bits (default: 4096)"
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

print_color $GREEN "Generating JWT RSA signing keys..."
print_color $CYAN "   Target directory: $CERT_DIR"
print_color $CYAN "   Key size: $JWT_KEY_SIZE bits"
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

print_color $YELLOW "Using OpenSSL for key generation..."

# Generate JWT signing RSA key pair
print_color $WHITE "Generating JWT RSA private key ($JWT_KEY_SIZE bits)..."
if ! openssl genrsa -out "$CERT_DIR/key.pem" $JWT_KEY_SIZE; then
    print_color $RED "Failed to generate JWT private key"
    exit 1
fi

print_color $WHITE "Extracting JWT RSA public key..."
if ! openssl rsa -in "$CERT_DIR/key.pem" -pubout -out "$CERT_DIR/public_key.pem"; then
    print_color $RED "Failed to extract JWT public key"
    exit 1
fi

# Verify the key pair
print_color $WHITE "Verifying JWT key pair..."
if ! openssl rsa -in "$CERT_DIR/key.pem" -check -noout; then
    print_color $RED "JWT key verification failed"
    exit 1
fi

echo ""
print_color $GREEN "JWT signing keys generated successfully!"

echo ""
print_color $CYAN "Generated JWT files in $CERT_DIR/:"

# Display generated files
for file in "key.pem" "public_key.pem"; do
    if [ -f "$CERT_DIR/$file" ]; then
        size=$(du -h "$CERT_DIR/$file" | cut -f1)
        case $file in
            "key.pem")
                print_color $YELLOW "  $file - JWT private key ($size)"
                ;;
            "public_key.pem")
                print_color $YELLOW "  $file - JWT public key ($size)"
                ;;
        esac
    fi
done

echo ""
print_color $CYAN "Configuration for test.toml:"
print_color $WHITE "  [jwt.secret]"
print_color $WHITE "  type = \"pem_file\""
print_color $WHITE "  private_key_path = \"$CERT_DIR/key.pem\""
print_color $WHITE "  public_key_path = \"$CERT_DIR/public_key.pem\""
print_color $WHITE "  key_id = \"jwt-key-test\""
echo ""
print_color $RED "Security Note:"
print_color $WHITE "  • These are development/test keys only!"
print_color $WHITE "  • JWT private keys should be kept secure in production"