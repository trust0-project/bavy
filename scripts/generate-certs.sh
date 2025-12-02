#!/bin/bash
# Generate self-signed certificates for the WebTransport relay
# These certificates are valid for WebTransport with serverCertificateHashes

set -e

CERTS_DIR="${1:-./certs}"
VALIDITY_DAYS=14  # Chrome requires ≤14 days for serverCertificateHashes

echo "Generating WebTransport certificates..."
echo "Output directory: $CERTS_DIR"

# Create certs directory if it doesn't exist
mkdir -p "$CERTS_DIR"

# Generate ECDSA P-256 key (required for short-lived certs in WebTransport)
openssl ecparam -name prime256v1 -genkey -noout -out "$CERTS_DIR/relay-key.pem"

# Generate self-signed certificate
# - Use ECDSA for better WebTransport compatibility
# - Short validity (Chrome enforces ≤14 days for serverCertificateHashes)
# - Add SANs for localhost and common local addresses
openssl req -new -x509 \
    -key "$CERTS_DIR/relay-key.pem" \
    -out "$CERTS_DIR/relay-cert.pem" \
    -days "$VALIDITY_DAYS" \
    -subj "/CN=localhost" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:::1,DNS:relay,IP:10.0.2.2"

echo ""
echo "✓ Generated certificates:"
echo "  - $CERTS_DIR/relay-cert.pem"
echo "  - $CERTS_DIR/relay-key.pem"
echo "  - Valid for $VALIDITY_DAYS days"
echo ""

# Calculate SHA-256 hash of the certificate (DER format)
# This is what WebTransport serverCertificateHashes expects
CERT_HASH=$(openssl x509 -in "$CERTS_DIR/relay-cert.pem" -outform DER 2>/dev/null | \
    openssl dgst -sha256 -binary | \
    xxd -p -c 32 | \
    sed 's/\(..\)/\1:/g' | \
    sed 's/:$//')

echo "Certificate SHA-256 Hash (for WebTransport):"
echo "$CERT_HASH"
echo ""

# Also output without colons for easy copy-paste
CERT_HASH_NO_COLONS=$(echo "$CERT_HASH" | tr -d ':')
echo "Hash without colons:"
echo "$CERT_HASH_NO_COLONS"
echo ""

echo "To use these certificates:"
echo ""
echo "1. Update docker-compose.yml NEXT_PUBLIC_RELAY_CERT_HASH with:"
echo "   $CERT_HASH"
echo ""
echo "2. Or start the relay manually with:"
echo "   cargo run --release -- --cert-pem $CERTS_DIR/relay-cert.pem --key-pem $CERTS_DIR/relay-key.pem"
echo ""
echo "Note: Certificates expire in $VALIDITY_DAYS days. Regenerate before expiry."


