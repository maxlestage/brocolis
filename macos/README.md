# Brocolis sur macOS

`brocolis auto` est la voie **native** sur macOS (cf. `src/system/macos.rs`).
Ce dossier complète avec des installeurs shell pour les usages persistants
(LaunchDaemon), un plist durci, et un test d'intégration équivalent à
`linux/test-linux.sh`.

## Fichiers

| Fichier                              | Rôle                                                                                              |
| ------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `install.sh`                         | Setup CA + politique Firefox + `.mobileconfig` + config. Flag `--enable-launchd` opt-in.          |
| `uninstall.sh`                       | Annule ce que `install.sh` a fait, en respectant ce qui préexistait.                              |
| `com.maxlestage.brocolis.plist`      | LaunchDaemon (tourne `brocolis run` comme service système).                                       |
| `test-install.sh`                    | Test d'intégration du chemin `install.sh` (équivalent de `linux/test-linux.sh`).                  |

Note : `scripts/test-macos.sh` (à la racine) teste le chemin natif
`brocolis auto`. Les deux tests sont complémentaires.

## Trois modes au choix

| Mode                             | Pour qui ?                                                                              |
| -------------------------------- | --------------------------------------------------------------------------------------- |
| **`sudo brocolis auto`**         | Test rapide / utilisation ad hoc. Tout configuré, restauré à Ctrl-C. **Recommandé** pour démarrer. |
| **`install.sh --enable-launchd`** | Installation persistante : CA + Firefox + service LaunchDaemon. Idéal sur poste perso.  |
| **`install.sh` (sans flag)**     | Setup persistant côté trousseau + Firefox, on lance `brocolis run` à la main quand on veut. |
| **`Brocolis.app`**               | L'application de bureau : un interrupteur, les clés, les messages. **Le plus simple.**   |

## Application `Brocolis.app`

L'archive `brocolis-macos-universal.tar.gz` contient l'application et la ligne
de commande. Glissez `Brocolis.app` dans **Applications**. Elle n'est pas
signée par un compte développeur Apple : au premier lancement, clic droit →
**Ouvrir**, ou Réglages Système → Confidentialité et sécurité → **Ouvrir quand
même**.

Activer la protection demande le mot de passe administrateur (ou Touch ID) :
l'application fait alors exécuter `brocolis on` par la ligne de commande
embarquée dans le bundle (`Contents/MacOS/brocolis`). Si le service
LaunchDaemon tourne déjà, elle l'utilise ; sinon elle fait tourner le proxy
elle-même tant qu'elle est ouverte. Ses données sont dans
`~/Library/Application Support/Brocolis`.

## Usage : `brocolis auto`

```sh
cargo build --release --bin brocolis
sudo target/release/brocolis auto --config brocolis.toml
```

Le proxy tourne en foreground, tout est configuré au lancement (CA dans
le trousseau Système, politique Firefox dans chaque variante détectée,
`.mobileconfig` généré à côté du CA, proxies web/HTTPS système réglés
sur `127.0.0.1:<port>`). `Ctrl-C` restaure exactement l'état initial.

## Usage : `install.sh`

```sh
# Compilation (si pas déjà fait)
cargo build --release --bin brocolis

# Installation minimale : CA + politique Firefox + .mobileconfig + config.
sudo macos/install.sh
brocolis run --config /etc/brocolis/brocolis.toml

# Installation complète : + service LaunchDaemon (tourne en arrière-plan).
sudo macos/install.sh --enable-launchd

# Désinstallation
sudo macos/uninstall.sh           # garde /etc/brocolis/ (CA + clé)
sudo macos/uninstall.sh --purge   # supprime aussi /etc/brocolis/
```

Port d'écoute par défaut : 8080. Surchargez avec
`BROCOLIS_PORT=9000 sudo --preserve-env=BROCOLIS_PORT macos/install.sh`.

`install.sh` est **idempotent** : ré-exécutez-le sans dommage, l'état
(`/var/lib/brocolis/state.env`) est préservé pour que `uninstall.sh` ne
retire que ce que vous aviez installé.

## Flag : `--enable-launchd`

- Copie le binaire dans `/usr/local/bin/brocolis`.
- Installe `com.maxlestage.brocolis.plist` dans `/Library/LaunchDaemons/`
  (tourne en `root`, comme les LaunchDaemons système classiques).
- `launchctl bootstrap system <plist> && launchctl enable && kickstart -k`.

Commandes utiles une fois lancé :

```sh
sudo launchctl print system/com.maxlestage.brocolis | head     # état
tail -f /var/log/brocolis/brocolis.log                         # logs stdout
tail -f /var/log/brocolis/brocolis.err                         # logs stderr
sudo launchctl kill USR1 system/com.maxlestage.brocolis        # recharge la clé partagée
sudo launchctl kill HUP  system/com.maxlestage.brocolis        # rotation de clé
sudo launchctl bootout system/com.maxlestage.brocolis          # arrêt
```

Configurez votre navigateur sur le proxy HTTP **et** HTTPS
`127.0.0.1:8080` :
- **Safari / Chrome / Edge** : trousseau Système → CA reconnu automatiquement ;
  réglez le proxy dans *Réglages Système → Réseau → Détails → Proxies*.
- **Firefox** : la politique entreprise (`Certificates.ImportEnterpriseRoots`)
  ou le `.mobileconfig` fait confiance au CA automatiquement.
- **CLI** : `export http_proxy=http://127.0.0.1:8080 https_proxy=http://127.0.0.1:8080`.

## Limites volontaires

- **Proxy système non touché** : `install.sh` ne règle pas `networksetup`
  (le réglage des proxies par service réseau). Si vous voulez ça
  automatisé, utilisez `brocolis auto` (qui fait exactement ça et le
  restaure à l'arrêt). Sinon, configurez-le côté navigateur ou via
  *Réglages Système*.
- **Approbation manuelle du `.mobileconfig`** : macOS impose un
  double-clic puis approbation dans *Réglages Système → Confidentialité
  et sécurité → Profils*. C'est une contrainte de l'OS, non
  contournable. La politique entreprise déposée dans le bundle Firefox
  (méthode par défaut) fonctionne, elle, sans étape manuelle — mais
  modifie le bundle.
- **Notarisation / signature** : le binaire copié dans `/usr/local/bin/`
  n'est pas notarisé. macOS Gatekeeper ne bloque pas une exécution
  par `launchd` mais affichera un avertissement si vous le lancez à la
  main depuis Finder.

## CI / déploiement

`.github/workflows/macos-cli-release.yml` builde, à chaque push sur
`master`, le binaire **universel** (Intel + Apple Silicon via `lipo`),
puis le publie en `.tar.gz` sur la release rolling `cli-latest`. Les
tags `v*` produisent en plus une release versionnée `vX.Y.Z`.

Pas de packaging `.pkg` automatisé — le binaire est livré seul, les
utilisateurs lancent `brocolis auto` ou `macos/install.sh` à partir de
là.

## Côté Rust

`src/system/macos.rs` implémente nativement `brocolis auto` (trousseau
Système via `security add-trusted-cert`, proxies via `networksetup`,
politique Firefox dans chaque variante détectée, profil `.mobileconfig`,
restauration à l'arrêt). Les scripts shell de ce dossier sont une
alternative persistante au mode interactif, pas un remplacement —
`brocolis auto` reste la voie courte pour un test ou un usage ponctuel.
