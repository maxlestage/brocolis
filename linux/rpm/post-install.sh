#!/bin/sh
# Brocolis — RPM %post.
# Arg $1 : 1 = installation neuve, 2 = upgrade.
# On ne fait le setup système qu'à l'installation neuve.
set -e
[ "${1:-1}" -eq 1 ] || exit 0

ETC=/etc/brocolis
TRUST=/etc/pki/ca-trust/source/anchors/brocolis-ca.crt
FF_POLICY_DIR=/etc/firefox/policies
FF_POLICY=$FF_POLICY_DIR/policies.json
BIN=/usr/local/bin/brocolis

# 1. Utilisateur système.
if ! id brocolis >/dev/null 2>&1; then
    useradd --system --shell /sbin/nologin \
            --home-dir /var/lib/brocolis --no-create-home brocolis
fi

# 2. Dossiers de travail.
install -d -m 755 "$ETC"
install -d -m 700 -o brocolis -g brocolis /var/lib/brocolis

# 3. CA + config par défaut si absents.
if [ ! -f "$ETC/brocolis-ca.crt" ] || [ ! -f "$ETC/brocolis-ca.key" ]; then
    "$BIN" ca --ca-cert "$ETC/brocolis-ca.crt" --ca-key "$ETC/brocolis-ca.key" >/dev/null
fi
if [ ! -f "$ETC/brocolis.toml" ]; then
    KEY="$("$BIN" keygen | head -1)"
    cat > "$ETC/brocolis.toml" <<EOF
listen = "127.0.0.1:8080"
ca_cert = "$ETC/brocolis-ca.crt"
ca_key = "$ETC/brocolis-ca.key"
key = "$KEY"
encrypt_hosts = []
EOF
fi

# 4. Permissions cibles.
chown root:brocolis "$ETC/brocolis-ca.key"
chmod 640 "$ETC/brocolis-ca.key"
chown brocolis:brocolis "$ETC/brocolis.toml"
chmod 600 "$ETC/brocolis.toml"
chmod 644 "$ETC/brocolis-ca.crt"

# 5. Magasin de confiance système (Fedora/RHEL).
if [ ! -f "$TRUST" ]; then
    cp "$ETC/brocolis-ca.crt" "$TRUST"
    chmod 644 "$TRUST"
    update-ca-trust >/dev/null || true
fi

# 6. Politique entreprise Firefox.
if [ ! -f "$FF_POLICY" ]; then
    install -d -m 755 "$FF_POLICY_DIR"
    cat > "$FF_POLICY" <<'JSON'
{
  "policies": {
    "Certificates": {
      "ImportEnterpriseRoots": true
    }
  }
}
JSON
    chmod 644 "$FF_POLICY"
fi

# 7. Activation systemd (sur RPM, à la charge des scriptlets).
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload >/dev/null 2>&1 || true
    systemctl enable --now brocolis.service >/dev/null 2>&1 || true
fi

exit 0
