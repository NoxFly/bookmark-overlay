# Développement

## Prérequis

- Rust stable (voir `rust-version` dans [Cargo.toml](../src-tauri/Cargo.toml))
- Node.js, pour TypeScript et la CLI Tauri (`npx @tauri-apps/cli@2`)
- Windows 10 ou 11

## Organisation

```
frontend/            interface
  src/               sources TypeScript
  public/            fichiers servis tels quels (HTML, CSS, logos), embarqués par Tauri
    app/             JavaScript compilé depuis src/ (ignoré par git)
src-tauri/           application Rust
  icon-source/       source de l'icône native et son générateur
  icons/             icônes générées pour l'exe, la zone de notification, l'installateur
docs/                documentation
```

## Commandes

Interface, depuis `frontend/` — `npm ci` une fois (installe TypeScript, seule
dépendance npm) :

```powershell
cd frontend
npm run build:ui   # compile src/ vers public/app/
npm run watch:ui   # idem, en continu pendant le développement
npm run check:ui   # vérifie le typage sans rien écrire
```

Backend, depuis `src-tauri` — `cargo test` régénère aussi les types TypeScript de
l'interface ; **compiler l'interface d'abord** : `cargo` embarque
`frontend/public/`, et donc `public/app/`, qu'il ne produit pas.

```powershell
cd src-tauri
cargo test                                 # tests unitaires
cargo clippy --all-targets -- -D warnings  # lint
cargo fmt                                  # formatage
cargo run                                  # exécution en debug
cargo build --release                      # binaire optimisé
```

Binaire produit : `src-tauri/target/release/bookmark-overlay.exe`.

Les deux livrables (installateur NSIS et exécutable portable), depuis la racine ;
la CLI compile l'interface d'elle-même (`beforeBuildCommand`) :

```powershell
npx @tauri-apps/cli@2 build
```

## Interface

Les sources sont en TypeScript strict dans [frontend/src/](../frontend/src/),
compilées par `tsc` seul, sans bundler, vers `frontend/public/app/`. Chaque fichier
devient un module ES chargé tel quel par WebView2 : aucune dépendance n'est
embarquée à l'exécution. Seul `frontend/public/` est embarqué dans le binaire : les
sources `.ts` n'y figurent pas.

| Dossier                                                   | Contenu                                                          |
| --------------------------------------------------------- | ---------------------------------------------------------------- |
| [src/types/](../frontend/src/types/)                      | structures du backend, API Tauri globale                          |
| [src/core/](../frontend/src/core/)                        | état, IPC typé, outils DOM et texte, icônes, constantes           |
| [src/components/](../frontend/src/components/)            | briques réutilisables : liste déroulante, toast, champ à jetons   |
| [src/features/](../frontend/src/features/)                | liste, formulaire, réglages, profils, raccourci, mise à jour…     |
| [src/main.ts](../frontend/src/main.ts)                    | point d'entrée                                                    |

- **Les types du backend sont générés** depuis les structures Rust par
  [ts-rs](https://github.com/Aleph-Alpha/ts-rs), dans
  [frontend/src/types/generated/](../frontend/src/types/generated/) : à ne jamais
  modifier à la main. Un `cargo test` les régénère ; la pipeline échoue s'ils ne sont
  pas à jour. Une structure qui traverse l'IPC porte
  `#[cfg_attr(test, derive(ts_rs::TS), ts(export))]` : ts-rs n'est compilé que pour
  les tests, jamais dans le binaire livré. Les commentaires `///` deviennent la
  documentation des types TypeScript. La configuration (dossier de sortie, `number`
  pour les `u64`, imports en `.js`) est dans
  [src-tauri/.cargo/config.toml](../src-tauri/.cargo/config.toml).
- [backend.types.ts](../frontend/src/types/backend.types.ts) réexporte ces types et
  nomme les quelques formes propres à l'interface (`Maybe`, `CustomerInput`…).
- **Les commandes sont typées** ([ipc.service.ts](../frontend/src/core/ipc.service.ts)) :
  le nom d'une commande fixe ses arguments et son résultat, à partir des types
  générés. Ajouter une commande Rust, c'est l'ajouter à la table `Commands`.
- **Aucun module n'agit à son chargement** : chacun expose une fonction `init…`,
  appelée par `main.ts` dans un ordre explicite. Les dépendances circulaires entre
  modules restent ainsi sans effet.
- Les imports portent l'extension `.js` du fichier émis : c'est ce que le navigateur
  résout.
- HTML et CSS restent dans [frontend/public/](../frontend/public/), sans
  préprocesseur : WebView2 gère nativement variables et imbrication CSS.

L'icône de l'application se régénère avec `node src-tauri/icon-source/gen-icon.js`
puis `npx @tauri-apps/cli@2 icon src-tauri/icon-source/icon.png -o src-tauri/icons`.
Les logos des navigateurs sont dans
[frontend/public/icons/browsers/](../frontend/public/icons/browsers/) : un PNG de
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

Réglages → Système → **Rechercher une mise à jour** fait la même vérification à la
demande ; ses erreurs, elles, sont affichées.

Avec **Installer les mises à jour automatiquement** (réglage `autoUpdate`, désactivé
par défaut), la vérification en arrière-plan enchaîne directement sur
l'installation. Elle attend que le panneau soit fermé, en le regardant toutes les
minutes, et relit le réglage à chaque tour ; un échec laisse la mise à jour
proposée dans l'en-tête.

L'installation dépend du livrable en cours d'exécution :

- **installé** (repéré à l'`uninstall.exe` posé à côté de l'exécutable) : le nouveau
  `-setup.exe` est téléchargé dans `%TEMP%` puis lancé avec `/S /R /UPDATE`.
  Silencieux, il ferme l'application, la remplace et la relance ;
- **portable** : le nouveau `-portable.exe` est téléchargé à côté de l'exécutable,
  qui est renommé en `.old` (Windows l'autorise pour un programme en cours) ; le
  nouveau est lancé avec `--wait-pid=<pid>` et attend la fin de l'ancien avant de
  prendre le verrou d'instance unique. Le `.old` est supprimé au démarrage suivant.

**Redémarrer**, dans le menu de la zone de notification, réutilise le mécanisme de
la version portable : une nouvelle instance est lancée avec `--wait-pid=<pid>`, puis
l'instance courante se ferme.

Garde-fous : seul un livrable hébergé sous `https://github.com/<dépôt>/releases/`
est accepté, sa taille doit correspondre à celle annoncée, et son empreinte SHA-256
est vérifiée contre celle que publie GitHub. Les binaires n'étant pas signés, la
confiance repose sur le compte GitHub qui publie.
