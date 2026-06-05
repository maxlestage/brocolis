# Brocolis sur Linux

`brocolis auto` fonctionne **nativement sur Linux** (cf.
`src/system/linux.rs`) — Debian/Ubuntu, Fedora/RHEL et Arch détectés
automatiquement. Ce dossier complète avec :
des installeurs shell (pour les usages manuels), une unité systemd
durcie, des paquets `.deb` / `.rpm` (avec maintainer-scripts),
et un test d'intégration équivalent à `scripts/test-macos.sh`.

## Fichiers

| Fichier                 | Rôle                                                                                              |
| ----------------------- | ------------------------------------------------------------------------------------------------- |
| `install.sh`            | Setup CA + politique Firefox + config. Flags `--env-proxy` et `--enable-systemd` opt-in.          |
| `uninstall.sh`          | Annule ce que `install.sh` a fait, en respectant ce qui préexistait.                              |
| `brocolis.service`      | Unité systemd (durcie via `Protect*` / `Restrict*`) qui tourne `brocolis run` comme service.      |
| `test-linux.sh`         | Test d'intégration équivalent à `scripts/test-macos.sh` (install, flux HTTPS réel, uninstall).    |
| `build-packages.sh`     | Construit `.deb` (cargo-deb) + `.rpm` (cargo-generate-rpm) dans `linux/dist/`.                    |
| `debian/{postinst,prerm,postrm}` | Maintainer scripts utilisés par le `.deb` (créent l'utilisateur, posent CA + Firefox + config). |
| `rpm/{post-install,pre-uninstall,post-uninstall}.sh` | Scriptlets utilisés par le `.rpm` (équivalents, magasin Fedora/RHEL).            |

## Trois modes au choix

| Mode                             | Pour qui ?                                                                              |
| -------------------------------- | --------------------------------------------------------------------------------------- |
| **`sudo brocolis auto`**         | Test rapide / utilisation ad hoc. Tout est configuré au lancement, restauré à Ctrl-C.   |
| **Paquet (`.deb` / `.rpm`)**     | Installation persistante avec service systemd. Idéal sur poste de travail.              |
| **Scripts (`install.sh`)**       | Depuis les sources, ou contrôle fin sur chaque étape.                                   |

## Usage : `brocolis auto`

Strictement le même que sur macOS — la même CLI, la même UX :

```sh
sudo brocolis auto --config brocolis.toml
```

`engage` détecte la famille de distribution (Debian/Ubuntu, Fedora/RHEL
ou Arch), dépose le CA dans le magasin de confiance, pose la politique
entreprise Firefox dans `/etc/firefox/policies/policies.json`, et écrit
`/etc/profile.d/brocolis-proxy.sh` (variables `http_proxy`/`https_proxy`
sur les nouvelles sessions shell). `revert` (Ctrl-C) annule tout ce qui
a été réellement modifié — les fichiers préexistants sont conservés.

## Usage : paquet

Téléchargez depuis la release rolling [`cli-latest-linux`](https://github.com/maxlestage/brocolis/releases/tag/cli-latest-linux) (générée à chaque push sur `master`) ou construisez localement :

```sh
bash linux/build-packages.sh        # produit .deb et .rpm dans linux/dist/
bash linux/build-packages.sh deb    # juste le .deb
bash linux/build-packages.sh rpm    # juste le .rpm
```

Installation :

```sh
# Debian / Ubuntu
sudo apt install ./brocolis_*_amd64.deb

# Fedora / RHEL
sudo dnf install ./brocolis-*.x86_64.rpm
```

Le `postinst` (`.deb`) ou `%post` (`.rpm`) :
- crée l'utilisateur système `brocolis` ;
- génère le CA + un `brocolis.toml` avec une clé fraîche dans `/etc/brocolis/` ;
- dépose le CA dans le magasin de confiance de la distribution ;
- pose la politique entreprise Firefox dans `/etc/firefox/policies/` ;
- démarre et active le service `brocolis.service`.

Désinstallation :

```sh
# Debian / Ubuntu — retire CA, politique Firefox, service ; garde /etc/brocolis/.
sudo apt remove brocolis
# Purge complète (supprime aussi /etc/brocolis/ et l'utilisateur).
sudo apt purge brocolis

# Fedora / RHEL — équivalent direct (un seul mode = purge).
sudo dnf remove brocolis
```

## Usage : scripts (depuis les sources)

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

## CI / déploiement

`.github/workflows/linux-cli-release.yml` builde le binaire, le `.deb` et
le `.rpm` à chaque push sur `master`, et publie tout dans la release
rolling `cli-latest-linux`. Les tags `v*` produisent en plus une release
versionnée `vX.Y.Z-linux`.

Les tests d'intégration (`linux/test-linux.sh`) ne tournent **pas** en CI :
ils exigent root et modifient l'état du système (magasin de confiance,
politique Firefox, proxy). À jouer sur une vraie machine.

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

## Côté Rust

`src/system/` est découpé par plateforme :

- `mod.rs` : code partagé (`ProxyState`, `parse_getproxy`,
  `firefox_policy_json`, `firefox_mobileconfig`, `SystemProxy`,
  dispatch `engage` / `revert` via `cfg(target_os)`) + tests.
- `macos.rs` : trousseau Système + `networksetup` + variantes Firefox
  + `.mobileconfig`.
- `linux.rs` : détection Debian/RHEL/Arch + magasin de confiance distro
  + politique Firefox + `/etc/profile.d/brocolis-proxy.sh`.

Les scripts shell et les paquets `.deb`/`.rpm` ne sont **pas** rendus
obsolètes par le support natif : ils restent l'option idéale pour une
installation persistante avec service systemd, là où `brocolis auto`
est conçu pour un usage interactif (Ctrl-C restaure tout).
