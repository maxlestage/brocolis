# Brocolis sur Linux

Dossier réservé aux scripts, installeurs et fichiers de packaging
**spécifiques à Linux**. Vide pour l'instant — l'intégration système
automatique (`brocolis auto`) ne cible que macOS aujourd'hui.

## À mettre ici (à terme)

- Scripts shell d'installation / désinstallation
  (`install.sh`, `uninstall.sh`)
- Packaging `.deb` (Debian/Ubuntu) et `.rpm` (Fedora/RHEL)
- Unité `systemd` (`brocolis.service`) pour démarrage en service
- Étapes d'ajout du CA racine :
  - Debian/Ubuntu : copier dans `/usr/local/share/ca-certificates/`
    puis `update-ca-certificates`
  - Fedora/RHEL : copier dans `/etc/pki/ca-trust/source/anchors/`
    puis `update-ca-trust`
  - Politiques de confiance Firefox (cf. logique macOS dans
    `src/system.rs` : `policies.json`)
- Configuration du proxy système (variables d'environnement
  `http_proxy`/`https_proxy`, ou GSettings pour GNOME, ou
  `kwriteconfig5` pour KDE)
- Tests d'intégration équivalents au `scripts/test-macos.sh`
  (`scripts/test-linux.sh`)

## État actuel

Côté code Rust, `src/system.rs` détecte la cible et ne touche au système
qu'en macOS ; sur Linux, `brocolis auto` lance le proxy en mode
`--no-system` et indique de configurer manuellement
`127.0.0.1:<port>` comme proxy HTTP/HTTPS.
