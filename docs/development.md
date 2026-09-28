# Développement

## Prérequis

- Rust stable (voir `rust-version` dans [Cargo.toml](../src-tauri/Cargo.toml))
- Node.js, pour la CLI Tauri (`npx @tauri-apps/cli@2`)
- Windows 10 ou 11

## Commandes

```powershell
cd src-tauri
cargo test                                 # tests unitaires
cargo clippy --all-targets -- -D warnings  # lint
cargo fmt                                  # formatage
cargo run                                  # exécution en debug
cargo build --release                      # binaire optimisé
```

Binaire produit : `src-tauri/target/release/bookmark-overlay.exe`.

Les deux livrables (installateur NSIS et exécutable portable), depuis la racine :

```powershell
npx @tauri-apps/cli@2 build
```

Le front ([src/](../src/)) est statique : aucun bundler, aucune dépendance npm à
l'exécution. L'icône de l'application se régénère avec `node assets/gen-icon.js`
puis `npx @tauri-apps/cli@2 icon assets/icon.png -o src-tauri/icons`. Les logos des
navigateurs sont dans [src/icons/browsers/](../src/icons/browsers/) : un PNG de
128 × 128 px par navigateur, `system.svg` pour le navigateur par défaut ; un fichier
manquant est remplacé par l'initiale du navigateur.

## Clés de chiffrement

Le dépôt est public : aucune clé n'y figure. [build.rs](../src-tauri/build.rs) les
injecte à la compilation et les cherche dans cet ordre :

1. les variables d'environnement `BOOKMARK_OVERLAY_DATA_KEY` et
   `BOOKMARK_OVERLAY_LEGACY_KEY` — en CI, les secrets GitHub du même nom ;
2. le fichier `src-tauri/.env.local`, **ignoré par git**, au format `NOM=valeur`,
   pour développer et tester en local sans passer par la pipeline ;
3. à défaut, une clé de développement publique, avec un avertissement à la
   compilation. Un tel binaire ne lit pas les données d'un binaire officiel.

```
BOOKMARK_OVERLAY_DATA_KEY=<64 caractères hexadécimaux>
BOOKMARK_OVERLAY_LEGACY_KEY=<64 caractères hexadécimaux>
```

- `BOOKMARK_OVERLAY_DATA_KEY` chiffre les données et les exports. **La même valeur
  doit figurer dans `.env.local` et dans les secrets GitHub**, sinon un build local
  et un binaire de la pipeline ne se lisent pas. La perdre rend illisibles les
  données et les exports existants : la conserver dans un coffre de mots de passe.
- `BOOKMARK_OVERLAY_LEGACY_KEY`, facultative, est la clé des versions antérieures au
  renommage. Elle permet de relire leurs fichiers et exports, aussitôt rechiffrés.
  Sans elle, un tel fichier est refusé avec un message explicite.

## Pipeline de release

Chaque poussée sur `main` publie une release GitHub
([.github/workflows/release.yml](../.github/workflows/release.yml)) :

1. la version est calculée depuis le dernier tag `vX.Y.Z` et les messages de commit
   (Conventional Commits) : `BREAKING CHANGE` ou `type!:` → majeure, `feat:` →
   mineure, **tout le reste → correctif** ; sans tag, la version de `Cargo.toml` est
   publiée telle quelle ;
2. elle est injectée dans `Cargo.toml` et `tauri.conf.json` le temps du build —
   jamais recommitée, c'est le tag qui la porte ;
3. format, lint et tests ; la publication s'arrête si le secret
   `BOOKMARK_OVERLAY_DATA_KEY` manque ;
4. `npx @tauri-apps/cli@2 build`, puis création de la release `vX.Y.Z` avec ses notes
   générées et les deux livrables.

Un commit portant `[skip release]` dans son message ne publie rien.

## Mise à jour automatique

Le binaire interroge `GET /repos/<dépôt>/releases/latest` de l'API GitHub quelques
secondes après le démarrage, sur un thread à part qui ne retarde jamais l'overlay,
puis toutes les 6 heures ; une vérification ratée (réseau pas encore prêt à
l'ouverture de session, quota de l'API) est retentée 2 minutes plus tard. Le dépôt est celui qui l'a construit :
`GITHUB_REPOSITORY`, posée par GitHub Actions, est capturée à la compilation. **Un
binaire construit en local ne cherche donc jamais de mise à jour**, et le dépôt doit
être public (l'API est appelée sans jeton).

L'installation dépend du livrable en cours d'exécution :

- **installé** (repéré à l'`uninstall.exe` posé à côté de l'exécutable) : le nouveau
  `-setup.exe` est téléchargé dans `%TEMP%` puis lancé avec `/S /R /UPDATE`.
  Silencieux, il ferme l'application, la remplace et la relance ;
- **portable** : le nouveau `-portable.exe` est téléchargé à côté de l'exécutable,
  qui est renommé en `.old` (Windows l'autorise pour un programme en cours) ; le
  nouveau est lancé avec `--wait-pid=<pid>` et attend la fin de l'ancien avant de
  prendre le verrou d'instance unique. Le `.old` est supprimé au démarrage suivant.

Garde-fous : seul un livrable hébergé sous `https://github.com/<dépôt>/releases/`
est accepté, sa taille doit correspondre à celle annoncée, et son empreinte SHA-256
est vérifiée contre celle que publie GitHub. Les binaires n'étant pas signés, la
confiance repose sur le compte GitHub qui publie.
