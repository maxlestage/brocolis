# Brocolis sur Linux

Scripts d'installation système, unité systemd et test d'intégration
spécifiques à Linux. L'intégration native (`brocolis auto`) reste
macOS-only ; sur Linux, ce dossier fournit l'équivalent en shell.

## Fichiers

| Fichier              | Rôle                                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------------- |
| `install.sh`         | Setup CA + politique Firefox + config. Flags `--env-proxy` et `--enable-systemd` opt-in.          |
| `uninstall.sh`       | Annule ce que `install.sh` a fait, en respectant ce qui préexistait.                              |
| `brocolis.service`   | Unité systemd (durcie via `Protect*` / `Restrict*`) qui tourne `brocolis run` comme service.      |
| `test-linux.sh`      | Test d'intégration équivalent à `scripts/test-macos.sh` (install, flux HTTPS réel, uninstall).    |

## Usage

```sh
# Compilation (si pas déjà fait)
cargo build --release --bin brocolis

# Installation minimale : CA + Firefox + config, sans toucher au reste.
sudo linux/install.sh
brocolis run --config /etc/brocolis/brocolis.toml

# Installation complète : + variables proxy + service systemd.
sudo linux/install.sh --env-proxy --enable-systemd
# Le proxy tourne déjà en service ; rien d'autre à lancer.

# Désinstallation
sudo linux/uninstall.sh           # garde /etc/brocolis/ (CA + clé)
sudo linux/uninstall.sh --purge   # supprime aussi /etc/brocolis/
```

Le port d'écoute par défaut est 8080 ; surchargez-le avec
`BROCOLIS_PORT=9000 sudo --preserve-env=BROCOLIS_PORT linux/install.sh`.

`install.sh` est **idempotent** : ré-exécutez-le sans dommage, l'état
(`/var/lib/brocolis/state.env`) est préservé pour que `uninstall.sh` ne
retire **que** ce que vous aviez installé.

## Flags d'`install.sh`

### `--env-proxy`

Écrit `/etc/profile.d/brocolis-proxy.sh` avec :

```sh
export http_proxy="http://127.0.0.1:8080"
export https_proxy="http://127.0.0.1:8080"
export no_proxy="localhost,127.0.0.1,::1"
```

Effet sur les **nouvelles** sessions shell. Pour la session courante :

```sh
source /etc/profile.d/brocolis-proxy.sh
```

Utile pour `curl`, `wget`, `git`, `npm`, `pip`, etc. Les **navigateurs**
ont leurs propres réglages — Firefox suit la politique entreprise
déposée, Chromium/Chrome lit le proxy système (cf. `gsettings` GNOME ou
les options de lancement `--proxy-server`).

### `--enable-systemd`

- Crée l'utilisateur système `brocolis` (`useradd --system`).
- Copie le binaire dans `/usr/local/bin/brocolis`.
- Installe `linux/brocolis.service` dans `/etc/systemd/system/`.
- `systemctl daemon-reload && systemctl enable --now brocolis.service`.

L'unité tourne en `User=brocolis Group=brocolis` avec un durcissement
systemd standard (`ProtectSystem=strict`, `PrivateTmp`, `NoNewPrivileges`,
`CapabilityBoundingSet=`, etc.).

Commandes utiles :

```sh
systemctl status brocolis        # état
journalctl -u brocolis -f        # logs en direct
systemctl reload brocolis        # recharge la clé partagée (SIGUSR1)
systemctl restart brocolis       # régénère la clé (rotation, SIGHUP)
systemctl stop brocolis          # arrêt
```

## Distributions supportées

`install.sh` détecte la famille via les outils du magasin de confiance :

| Famille          | Magasin de confiance                         | Mise à jour              |
| ---------------- | -------------------------------------------- | ------------------------ |
| Debian / Ubuntu  | `/usr/local/share/ca-certificates/`          | `update-ca-certificates` |
| Fedora / RHEL    | `/etc/pki/ca-trust/source/anchors/`          | `update-ca-trust`        |
| Arch / dérivées  | `/etc/ca-certificates/trust-source/anchors/` | `trust extract-compat`   |

Sur une distribution inconnue, le script s'arrête proprement avec un
message explicite.

## Limites volontaires

- **Proxy GUI** (GSettings GNOME, KWrite KDE) : non automatisé.
  La diversité des environnements de bureau rend une approche
  universelle peu fiable — utilisez `--env-proxy` pour la ligne de
  commande, et configurez votre navigateur GUI à la main (ou via la
  politique entreprise Firefox déjà déposée).
- **Firefox Snap / Flatpak** : la politique entreprise est déposée
  dans `/etc/firefox/policies/policies.json`, lu par les paquets
  natifs et Mozilla. Les versions confinées (Snap, Flatpak) ne lisent
  pas toujours cet emplacement.
- **Packaging `.deb` / `.rpm`** : pas fourni — installation par script
  pour l'instant.

## État côté Rust

`src/system.rs` ne touche au système qu'en macOS. Sur Linux,
`brocolis auto` lance le proxy en mode `--no-system` ; ces scripts
font le reste.
