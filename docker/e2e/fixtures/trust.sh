#!/usr/bin/env bash
# Mints the CA and leaf certificate the upstream fixture serves under, and
# makes the container trust the CA.
#
# Go reads the system pool on Linux (/etc/ssl/certs), which is what
# update-ca-certificates maintains, so quiver.core needs no flag, no env var
# and no build change to accept these -- which is the whole point: the binary
# under test stays a production binary.
set -euo pipefail

TLS_DIR="${TLS_DIR:-/workspace/run/tls}"
mkdir -p "$TLS_DIR"

if [ -f "$TLS_DIR/server.pem" ] && [ -f "$TLS_DIR/server.key" ]; then
	echo "[trust] certificates already present in $TLS_DIR"
else
	echo "[trust] minting a CA and a leaf for the GitHub hostnames"
	openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
		-keyout "$TLS_DIR/ca.key" -out "$TLS_DIR/ca.pem" \
		-subj "/CN=Quiver E2E Local CA" 2>/dev/null

	cat >"$TLS_DIR/leaf.cnf" <<'EOF'
[req]
distinguished_name = dn
req_extensions     = ext
prompt             = no

[dn]
CN = github.com

[ext]
subjectAltName = @alt

[alt]
DNS.1 = github.com
DNS.2 = raw.githubusercontent.com
DNS.3 = api.github.com
DNS.4 = objects.githubusercontent.com
DNS.5 = codeload.github.com
DNS.6 = localhost
IP.1  = 127.0.0.1
EOF

	openssl req -newkey rsa:2048 -nodes \
		-keyout "$TLS_DIR/server.key" -out "$TLS_DIR/server.csr" \
		-config "$TLS_DIR/leaf.cnf" 2>/dev/null

	openssl x509 -req -in "$TLS_DIR/server.csr" \
		-CA "$TLS_DIR/ca.pem" -CAkey "$TLS_DIR/ca.key" -CAcreateserial \
		-out "$TLS_DIR/server.pem" -days 3650 \
		-extensions ext -extfile "$TLS_DIR/leaf.cnf" 2>/dev/null
fi

install -m 0644 "$TLS_DIR/ca.pem" /usr/local/share/ca-certificates/quiver-e2e-ca.crt
update-ca-certificates >/dev/null
echo "[trust] CA installed into the system trust store"
