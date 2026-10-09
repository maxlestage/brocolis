# Consignes pour Claude

## Livraison

- **Fusionner et pousser directement sur `master`**, sans demander de
  confirmation, une fois le travail vérifié (consigne du mainteneur). Un push
  sur `master` déploie le site sur Heroku : ne pousser que ce qui passe les
  vérifications ci-dessous.
- Après le push, vérifier la CI de `master` et corriger tout échec causé par
  le commit.

## Avant de pousser une modification du site (`site/`)

```sh
cd site
cargo fmt --check
cargo test
cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings
trunk build --release          # site/dist est versionné : le committer
cd .. && node scripts/check-dist.mjs
cd tests/browser && npm test   # harnais navigateur
```

`site/dist` est déployé tel quel (Heroku n'a pas de chaîne Rust) : un `dist`
non reconstruit sert l'ancienne version.
