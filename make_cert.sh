#!/usr/bin/env bash
# One-time: creates a self-signed "Swatchbar Dev" code-signing certificate in your login keychain.
# Signing with a stable identity lets macOS keep the Screen Recording grant across rebuilds.
set -euo pipefail
NAME="Swatchbar Dev"
if security find-identity -p codesigning | grep -q "$NAME"; then
  echo "'$NAME' already exists."; exit 0
fi
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT; cd "$D"
cat > c.cnf <<EOF
[req]
distinguished_name=dn
x509_extensions=ext
prompt=no
[dn]
CN=$NAME
[ext]
basicConstraints=critical,CA:false
keyUsage=critical,digitalSignature
extendedKeyUsage=critical,codeSigning
EOF
openssl req -x509 -newkey rsa:2048 -nodes -keyout k.pem -out c.pem -days 3650 -config c.cnf 2>/dev/null
openssl pkcs12 -export -legacy -inkey k.pem -in c.pem -out c.p12 -passout pass:swb
KC="$HOME/Library/Keychains/login.keychain-db"
security import c.p12 -k "$KC" -P swb -T /usr/bin/codesign
security add-trusted-cert -p codeSign -k "$KC" c.pem   # macOS will ask for your password
security find-identity -p codesigning | grep "$NAME"
echo "Done. Now run ./bundle.sh /Applications/Swatchbar.app"
