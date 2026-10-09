# Vérification du site dans un navigateur

Le site est une application WebAssembly : son HTML initial est vide, tout est
produit à l'exécution. Un test qui lirait les sources ne vérifierait donc rien
du rendu réel — d'où ce harnais, qui pilote un Chromium.

## Pourquoi il existe

La page de téléchargement a été « corrigée » trois fois d'affilée sur la base
de captures d'écran, et les trois fois le défaut était ailleurs :

- un `padding` en notation raccourcie sur `.dl-main` remettait à zéro la
  gouttière horizontale héritée de `.container` ;
- `.dl-note` était battu en spécificité par `.platform-card p`, donc sa taille
  de police ne s'appliquait jamais ;
- le palier de bascule du menu était à 761px alors que le menu déroulé demande
  904px en français, ce qui rendait toute la page défilable horizontalement
  entre les deux.

Aucun de ces trois défauts ne se voit sur une image. Les assertions portent
donc sur des **valeurs calculées par le navigateur** : `getComputedStyle`,
`getBoundingClientRect`, `scrollWidth`, `document.activeElement`.

## Ce qui est couvert

- accueil et page de téléchargement, en 390px et 1280px ;
- absence de débordement horizontal à 390, 601, 760, 905, 959, 1280px,
  **menu déroulé** — l'état dans lequel la barre débordait ;
- gouttière et largeur des cartes de la page de téléchargement, égalité de
  hauteur des deux boutons Linux ;
- ouverture/fermeture du tiroir (clic, Échap, `aria-expanded`) ;
- défilement vers une section **sans toucher au fragment d'URL**, qui
  appartient au routeur ;
- navigation entre les deux routes, accès direct par URL, redirection de
  l'ancienne URL `/telecharger`, arrivée en haut de page ;
- langue et thème : bascule, persistance, restauration au rechargement,
  détection depuis `navigator.language` (`fr-FR`, `en-GB`, `es-419`, et
  `de-DE` qui doit retomber sur le français) ;
- les 9 combinaisons langue × thème, avec l'attribution présente dans le
  pied de page ;
- lien d'évitement : premier élément focalisable, déplace réellement le focus
  sur `<main>`, sans toucher au fragment ;
- balisage des traductions interprété et non affiché en texte brut ;
- mouvement, lu dans les variables CSS et attributs écrits par le
  WebAssembly : rideau d'ouverture (transparent aux clics, levé, non rejoué
  dans la session), scène du héros épinglée dont la progression suit le
  défilement, poussière réellement peinte dans le canevas, photo révélée en
  fin de scène, apparitions au défilement, barre qui s'efface et revient,
  manifeste, curseur anneau, aimantation des boutons, volet entre les pages ;
- mouvement réduit (`reducedMotion: "reduce"`) : ni rideau ni épinglage, et
  aucun contenu caché en attente d'une animation ;
- aucune erreur de console ni requête en échec.

## Lancer

```sh
cd tests/browser
npm ci
npx playwright install chromium   # une fois
npm test                          # démarre server.js tout seul
```

`npm test` construit *ne construit pas* le site : il sert `site/dist` tel
qu'il est versionné. Après une modification de `site/`, lancer d'abord
`cd site && trunk build --release`.

Pour viser un serveur déjà lancé (ou un déploiement) :

```sh
node site.mjs https://exemple.test
```

Si un Chromium est déjà présent sur la machine, `CHROMIUM_PATH` évite de le
retélécharger :

```sh
CHROMIUM_PATH=/chemin/vers/chrome npm test
```
