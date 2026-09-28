# Architecture

Application [Tauri 2](https://tauri.app) : un backend Rust
([src-tauri/src/](../src-tauri/src/)) et un front statique sans bundler
([src/](../src/)).

| Module                                               | Rôle                                                      |
| ---------------------------------------------------- | --------------------------------------------------------- |
| [lib.rs](../src-tauri/src/lib.rs)                    | démarrage, zone de notification, politique de fenêtre     |
| [commands.rs](../src-tauri/src/commands.rs)          | commandes exposées au front, seule frontière avec le système |
| [model.rs](../src-tauri/src/model.rs)                | fiches, réglages, profils de navigation, validation       |
| [store.rs](../src-tauri/src/store.rs)                | persistance, import, export, migrations                   |
| [crypto.rs](../src-tauri/src/crypto.rs)              | chiffrement AES-256-GCM du fichier de données              |
| [browser.rs](../src-tauri/src/browser.rs)            | détection des navigateurs et de leurs profils, lancement  |
| [launcher.rs](../src-tauri/src/launcher.rs)          | gabarits d'URL, Visual Studio Code, explorateur            |
| [overlay.rs](../src-tauri/src/overlay.rs)            | placement, affichage et masquage de la fenêtre            |
| [shortcut.rs](../src-tauri/src/shortcut.rs)          | raccourci global                                          |
| [updater.rs](../src-tauri/src/updater.rs)            | mise à jour automatique                                   |
| [legacy.rs](../src-tauri/src/legacy.rs)              | reprise d'une installation antérieure au renommage        |

## Données

Le fichier `data` du dossier de configuration contient toute la base (fiches et
réglages) en JSON chiffré. Il est chargé une fois au démarrage puis maintenu en
mémoire : la recherche ne touche jamais le disque. Chaque écriture est atomique
(fichier temporaire puis renommage).

Chaque fiche porte un `id` technique, attribué à la création et jamais réutilisé ;
le tenant, facultatif, ne peut pas servir de clé. Les URL calculées ne sont pas
stockées : elles sont recalculées à l'affichage depuis les gabarits.

| Champ JSON   | Rôle                                            |
| ------------ | ----------------------------------------------- |
| `id`         | clé technique                                   |
| `name`       | nom affiché                                     |
| `tenant`     | unique s'il est renseigné                       |
| `keywords`   | mots-clés                                       |
| `folderPath` | dossier local                                   |
| `workspaces` | `{ name, path }`, dossier ou `.code-workspace`  |
| `links`      | `{ name, url }`                                 |
| `devopsId`   | identifiant Azure DevOps                        |
| `githubId`   | identifiant GitHub                              |

### Chiffrement

AES-256-GCM, chiffrement authentifié : une altération est détectée au lieu de
produire des données fausses. La clé est injectée à la compilation (voir
[development.md](development.md#clés-de-chiffrement)) : absente du dépôt, elle est
présente dans le binaire. Elle arrête quiconque tombe sur le fichier, pas quelqu'un
qui possède l'exécutable et cherche vraiment. C'est la contrepartie d'un démarrage
sans mot de passe et d'exports lisibles par l'application d'un collègue.

Deux formats coexistent, reconnus à leur marqueur : le format courant, et celui des
versions antérieures au renommage, chiffré avec l'ancienne clé.

### Migrations à la lecture

Chacune réécrit aussitôt le fichier, pour ne pas se rejouer au démarrage suivant :

- un fichier chiffré avec l'ancienne clé est relu grâce à elle, si le build la
  connaît, puis rechiffré avec la clé courante ;
- un fichier sans `id` techniques en reçoit ;
- des réglages à deux profils Edge (primaire, Admin Center) deviennent deux profils
  de navigation, revendiquant les hôtes des gabarits ;
- un ancien `data.json` en clair est chiffré ; sa copie
  `data.json.avant-chiffrement` reste lisible et est à supprimer à la main.

### Reprise après renommage

L'identifiant de l'application nomme son dossier de configuration. Au premier
démarrage, si le dossier courant n'a pas de fichier de données et que celui de
l'ancien identifiant en a un, ses fichiers de premier niveau sont recopiés ;
l'ancien dossier est laissé intact. L'ancienne entrée de démarrage automatique est
remplacée par la nouvelle ([legacy.rs](../src-tauri/src/legacy.rs)).

## Navigateurs

Les profils proposés sont ceux réellement présents sur la machine :

- **Chromium** (Edge, Chrome, Brave, Vivaldi) : le fichier `Local State` associe le
  dossier (`Profile 3`), attendu par `--profile-directory`, au nom affiché
  (« 2 professionnel »). La photo du compte est lue dans le dossier du profil.
- **Firefox** et **Developer Edition** partagent `profiles.ini` ; le profil est
  désigné par `-P <nom>`. Chaque installation a sa section `[Install<empreinte>]`,
  empreinte qu'on ne sait pas recalculer : Developer Edition est reconnue à son
  profil `dev-edition-default`.
- **Opera** n'expose pas ses profils en ligne de commande.

Les exécutables sont cherchés sous `%ProgramFiles%`, `%ProgramFiles(x86)%` et
`%LOCALAPPDATA%`, puis dans le `PATH`. Les arguments sont passés un par un, jamais
via un interpréteur.

### Visual Studio Code

Cherché dans l'installation utilisateur (`%LOCALAPPDATA%\Programs\Microsoft VS
Code`), puis les installations machine, puis le `PATH`. `Code.exe` est invoqué
directement, sans le script `code.cmd` qui exigerait un interpréteur, mais avec ses
précautions : `--new-window`, faute de quoi une instance lancée se contente de
revenir au premier plan, et `ELECTRON_RUN_AS_NODE` retirée de l'environnement.
Posée par le terminal intégré de l'éditeur, cette variable ferait démarrer
`Code.exe` en simple interpréteur Node.

## Raccourci global

La capture d'un nouveau raccourci suspend celui en vigueur (sans cela, taper la
combinaison actuelle fermerait le panneau), puis le réarme quelle que soit l'issue,
y compris si le panneau se referme entre-temps. La touche est lue à son code
physique (`KeyD`), indépendant de la disposition du clavier.

## Empreinte

Mesures indicatives sur la machine de développement (Windows 11, deux écrans) :

| Situation                        | Mémoire privée (processus + WebView2) |
| -------------------------------- | ------------------------------------- |
| démarrage, panneau jamais ouvert | ~12 Mo                                |
| panneau affiché                  | ~115 Mo                               |
| panneau refermé                  | 11-15 Mo                              |

Lancement du processus jusqu'à la fenêtre prête : ~300 ms. Au démarrage,
l'application n'a aucune page à peindre : elle charge ses données, enregistre son
raccourci, puis se met en sommeil.

Le retour à une quinzaine de mégaoctets tient à un seul mécanisme : le working set
du processus **et de ses processus WebView2** est rendu au système
(`SetProcessWorkingSetSize`) à chaque masquage, et une fois au démarrage.
`ICoreWebView2_3::TrySuspend` a été essayé et retiré : il impose de masquer le
contrôleur de la vue, et la fenêtre réapparaissait vide un instant, sans gain
mesurable au-delà de la purge.

## Choix d'architecture

- **La webview reste vivante entre deux ouvertures.** La recréer coûterait
  plusieurs centaines de millisecondes ; le raccourci doit être instantané.
- **La recherche ne traverse jamais l'IPC.** Le référentiel est filtré en mémoire ;
  seules les écritures appellent le backend.
- **Le front n'a aucune permission Tauri au-delà de l'IPC.** Tout accès disque,
  réseau ou processus passe par une commande Rust typée, qui valide ses entrées.
- **La clé d'une fiche vient du dépôt, jamais de la charge utile.** Une mise à jour
  impose l'identifiant visé.
- **Les liens d'une fiche défilent à l'intérieur de celle-ci.** Une fiche très
  fournie élargissait sa carte et faisait défiler tout le panneau
  horizontalement.
- **Les boîtes de dialogue sont filles de la fenêtre du panneau.** Toujours au
  premier plan, le panneau recouvrait sinon le sélecteur de dossier. Il est
  réaffiché au retour du dialogue, et la vue en cours est conservée.
- **La fenêtre n'est jamais pilotée depuis le rappel `with_webview`.** Y appeler
  `show`, `set_focus` ou un redimensionnement bloque le thread principal : le
  raccourci cesse de répondre.
- **Les pertes de focus sont ignorées 350 ms après l'affichage.** Windows en émet
  de transitoires le temps que la fenêtre passe au premier plan.
- **Aucun `--disable-gpu` n'est passé à WebView2.** Il réduisait l'empreinte GPU
  mais rendait la fenêtre noire.
- **La fenêtre est étirée sur le moniteur du curseur avant d'être affichée, et
  dimensionnée dès le démarrage.** La réduire une fois masquée empêche WebView2 de
  retrouver sa surface ; ne pas la dimensionner au démarrage faisait apparaître le
  panneau un instant au centre.
- **Le panneau attend dans son état de départ pendant qu'il est masqué.** La
  webview compose encore : l'image conservée est celle du début de l'animation. Le
  backend émet `overlay://hidden` avant de masquer, pour laisser le front repeindre.
- **Aucune animation d'entrée ne touche à l'opacité**, et la classe d'animation est
  retirée par un minuteur, jamais par `animationend` : le compositeur peut être à
  l'arrêt au moment où elle est posée.
- **L'animation d'ouverture de Windows est désactivée**
  (`DWMWA_TRANSITIONS_FORCEDISABLED`) : elle faisait grandir tout l'écran.
- **Les listes déroulantes sont maison et flottent en absolu dans le formulaire.**
  Le `<select>` natif n'est pas thémable, et un menu en position fixe s'est révélé
  capricieux dans la fenêtre transparente. En absolu dans le formulaire qui défile,
  le menu suit le défilement sans calcul ; il s'ouvre dessous ou au-dessus selon la
  place, avec une hauteur bornée à 500 px et à l'espace visible.
- **Le HTTP de la mise à jour passe par le TLS de Windows** (`ureq` + `native-tls`) :
  un proxy d'entreprise qui réémet les certificats reste accepté, et aucun magasin
  de racines n'est embarqué.
