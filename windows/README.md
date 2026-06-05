# Brocolis sur Windows

Dossier réservé aux scripts, installeurs et fichiers de packaging
**spécifiques à Windows**. Vide pour l'instant — l'intégration système
automatique (`brocolis auto`) ne cible que macOS aujourd'hui.

## À mettre ici (à terme)

- Scripts PowerShell d'installation / désinstallation
  (`install.ps1`, `uninstall.ps1`)
- Configuration d'installeur MSI ou NSIS
- Étapes d'ajout du CA racine au magasin **« Autorités de certification
  racines de confiance »** (`certutil -addstore Root <cert>`)
- Configuration du proxy système via `netsh winhttp set proxy` ou les
  registres `HKCU\…\Internet Settings`
- Tests d'intégration équivalents au `scripts/test-macos.sh`
  (`scripts\test-windows.ps1`)

## État actuel

Côté code Rust, `src/system.rs` détecte la cible et ne touche au système
qu'en macOS ; sur Windows, `brocolis auto` lance le proxy en mode
`--no-system` et indique de configurer manuellement
`127.0.0.1:<port>` comme proxy HTTP/HTTPS.
