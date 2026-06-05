# Brocolis sur Linux

Scripts d'installation système et test d'intégration spécifiques à Linux.
L'intégration native (`brocolis auto`) reste macOS-only ; sur Linux, ce
dossier fournit l'équivalent en shell.

## Scripts

| Script           | Rôle                                                                   |
| ---------------- | ---------------------------------------------------------------------- |
| `install.sh`     | Génère le CA, l'ajoute au magasin de confiance, dépose la politique Firefox et génère un `brocolis.toml` prêt à l'emploi. |
| `uninstall.sh`   | Annule ce que `install.sh` a fait, en respectant ce qui préexistait.  |
| `test-linux.sh`  | Test d'intégration équivalent à `scripts/test-macos.sh` : install + flux HTTPS via le proxy + chiffrement sélectif + uninstall + vérif de la restauration. |

## Usage

```sh
# Compilation (si pas déjà fait)
cargo build --release --bin brocolis

# Installation système
sudo linux/install.sh

# Lancement du proxy
brocolis run --config /etc/brocolis/brocolis.toml

# Configurez votre navigateur sur le proxy HTTP+HTTPS 127.0.0.1:8080
#   - Firefox : la politique entreprise déposée fait confiance au CA
#     automatiquement (aucune action manuelle).
#   - Chrome/Chromium : utilise le magasin de confiance système — OK.
#     Configurez le proxy via les paramètres système ou --proxy-server.

# Désinstallation
sudo linux/uninstall.sh           # garde /etc/brocolis/ (CA + clé)
sudo linux/uninstall.sh --purge   # supprime aussi /etc/brocolis/
```

Le port d'écoute par défaut est 8080 ; surchargez-le avec
`BROCOLIS_PORT=9000 sudo --preserve-env=BROCOLIS_PORT linux/install.sh`.

## Distributions supportées

`install.sh` détecte la famille de distribution via les outils du magasin
de confiance :

| Famille          | Magasin de confiance                         | Mise à jour            |
| ---------------- | -------------------------------------------- | ---------------------- |
| Debian / Ubuntu  | `/usr/local/share/ca-certificates/`          | `update-ca-certificates` |
| Fedora / RHEL    | `/etc/pki/ca-trust/source/anchors/`          | `update-ca-trust`       |
| Arch / dérivées  | `/etc/ca-certificates/trust-source/anchors/` | `trust extract-compat`  |

Sur une distribution inconnue, le script s'arrête proprement avec un
message explicite — installez `ca-certificates` ou équivalent.

## Limites volontaires

- **Proxy système** : pas d'automatisation. La diversité des
  environnements de bureau (GNOME, KDE, Sway, …) et des packagings
  (deb, rpm, Flatpak, Snap) rend une approche universelle peu fiable.
  Configurez-le côté navigateur ou via `gsettings set
  org.gnome.system.proxy mode 'manual'` à la main.
- **Firefox Snap / Flatpak** : la politique entreprise est déposée dans
  `/etc/firefox/policies/policies.json`, lu par les paquets natifs et
  les paquets Mozilla. Les versions confinées (Snap, Flatpak) ne lisent
  pas toujours cet emplacement — référez-vous à la doc de votre paquet.
- **Pas de service systemd** : le proxy se lance en avant-plan
  (Ctrl-C pour quitter). Ajoutez une unité `brocolis.service` si vous
  voulez en faire un démon.

## État côté Rust

`src/system.rs` ne touche au système qu'en macOS. Sur Linux,
`brocolis auto` lance le proxy en mode `--no-system` ; ces scripts
font le reste.
