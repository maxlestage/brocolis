# Brocolis sur Windows

`brocolis auto` fonctionne **nativement sur Windows** (cf.
`src/system/windows.rs`). Ce dossier complète avec des installeurs
PowerShell pour les usages persistants, et un test rapide à jouer sur
une vraie machine.

## Fichiers

| Fichier              | Rôle                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------ |
| `install.ps1`        | Setup CA + politique Firefox + config. Flag `-SystemProxy` opt-in (WinHTTP via netsh).     |
| `uninstall.ps1`      | Annule ce que `install.ps1` a fait, en respectant ce qui préexistait.                      |
| `test-auto.ps1`      | Test d'intégration de `brocolis auto` natif (équivalent direct de `linux/test-auto.sh`).   |

## Tester `brocolis auto`

Depuis un terminal **Exécuter en tant qu'administrateur**, à la racine du dépôt :

```powershell
.\windows\test-auto.ps1
```

Le script compile la CLI, joue le cycle complet (engage → flux HTTPS
réel via le proxy → round-trip de chiffrement sélectif → revert), et
vérifie la restauration en respectant l'état initial du poste (CA
préexistant conservé, politique Firefox préexistante conservée,
WinHTTP remis à l'identique).

Le Ctrl+C est envoyé via les API Win32 `AttachConsole` +
`GenerateConsoleCtrlEvent` pour que `tokio::signal::ctrl_c()` côté
brocolis déclenche `revert()` proprement — sans ça, `Stop-Process`
tuerait le proxy sans laisser le temps à `revert()` de tourner.

## Trois modes au choix

| Mode                                  | Pour qui ?                                                                              |
| ------------------------------------- | --------------------------------------------------------------------------------------- |
| **Admin → `brocolis auto`**           | Test rapide / utilisation ad hoc. Tout configuré, restauré à Ctrl-C. **Recommandé** pour démarrer. |
| **`install.ps1 -SystemProxy`**        | Setup persistant avec proxy WinHTTP réglé. Pas de service Windows : on lance `brocolis run` à la main quand on veut. |
| **`install.ps1` (sans flag)**         | Setup persistant côté magasin de confiance + Firefox uniquement. Proxy à configurer côté navigateur. |

## Usage : `brocolis auto`

Depuis un terminal **Exécuter en tant qu'administrateur** :

```powershell
cargo build --release --bin brocolis
.\target\release\brocolis.exe auto --config brocolis.toml
```

`engage` :

1. Approuve le CA dans **Local Machine\Trusted Root Certification
   Authorities** via `certutil -addstore -f Root <ca>`.
2. Dépose la politique entreprise Firefox dans chaque variante détectée
   (`<install>\distribution\policies.json`).
3. Règle le proxy **WinHTTP** système via `netsh winhttp set proxy
   proxy-server="127.0.0.1:<port>" bypass-list="<local>"`, après avoir
   sauvegardé l'état initial.

`revert` (Ctrl-C / SIGTERM) restaure exactement l'état initial : CA
retiré du magasin, politique Firefox retirée, WinHTTP remis comme avant.

> **WinHTTP vs WinINet** : `netsh winhttp` couvre les apps qui
> utilisent l'API WinHTTP (services, certains outils CLI). Les
> navigateurs WinINet (Edge, Chrome, Internet Explorer) lisent
> `HKCU\…\Internet Settings`, qui est par utilisateur — `brocolis
> auto` ne le modifie pas (pas de chemin propre pour faire ça en
> élévation depuis un compte arbitraire). Pour ces navigateurs,
> configurez le proxy à la main, ou comptez sur la politique Firefox
> déjà déposée si vous utilisez Firefox.

## Usage : `install.ps1`

Depuis un terminal **Exécuter en tant qu'administrateur** :

```powershell
# Compilation (si pas deja faite)
cargo build --release --bin brocolis

# Installation minimale : CA + Firefox + config, sans toucher au proxy systeme.
.\windows\install.ps1
.\target\release\brocolis.exe run --config "$env:ProgramData\Brocolis\brocolis.toml"

# Installation complete : + proxy WinHTTP.
.\windows\install.ps1 -SystemProxy

# Desinstallation
.\windows\uninstall.ps1            # garde %ProgramData%\Brocolis\
.\windows\uninstall.ps1 -Purge     # supprime aussi %ProgramData%\Brocolis\
```

Port d'écoute par défaut : 8080. Surchargez via
`$env:BROCOLIS_PORT = '9000'` avant de lancer le script.

`install.ps1` est **idempotent** : ré-exécutez-le sans dommage, l'état
(`%ProgramData%\Brocolis\state.json`) est préservé pour que
`uninstall.ps1` ne retire que ce que vous aviez installé.

`install.ps1` et `uninstall.ps1` détectent automatiquement le manque
d'élévation et relancent UAC.

## Distributions / versions supportées

- **Windows 10 / 11** (x86_64) — testé avec Windows PowerShell 5.1 et
  PowerShell 7+. Le script utilise `certutil.exe` (présent depuis
  Windows 7+) et `netsh.exe` (présent depuis Windows 2000).
- Firefox détecté dans `%ProgramFiles%` et `%ProgramFiles(x86)%`
  (Mozilla Firefox, Developer Edition, Nightly). Les paquets Microsoft
  Store / winget exposent les mêmes chemins.

## Limites volontaires

- **Pas de service Windows** : `install.ps1` ne pose pas de
  service via `sc.exe create` ou NSSM. Pour faire tourner `brocolis
  run` en arrière-plan en permanence, créez le service à la main ou
  utilisez le Planificateur de tâches.
- **Proxy WinINet** (Edge/Chrome) non automatisé : voir la note
  WinHTTP vs WinINet ci-dessus.
- **Pas de paquet `.msi`** : installation par script PowerShell pour
  l'instant.

## CI / déploiement

`.github/workflows/windows-cli-release.yml` builde le binaire à
chaque push sur `master` (en `windows-latest` GitHub Actions), et
publie `brocolis-windows-x86_64.zip` (qui contient le `.exe` + les deux
`.ps1`) dans la release rolling [`cli-latest-windows`](https://github.com/maxlestage/brocolis/releases/tag/cli-latest-windows). Les
tags `v*` produisent en plus une release versionnée `vX.Y.Z-windows`.

Les tests d'intégration ne tournent **pas** en CI : ils exigent une
vraie machine Windows avec admin et un Firefox installé. À jouer sur
votre poste après téléchargement.

## Côté Rust

`src/system/windows.rs` implémente nativement `brocolis auto`
(approbation du CA via `certutil`, politique Firefox dans chaque
variante détectée, proxy WinHTTP via `netsh`, restauration à l'arrêt).
Les scripts PowerShell de ce dossier sont une alternative persistante
au mode interactif, pas un remplacement — `brocolis auto` reste la voie
courte pour un test ou un usage ponctuel.
