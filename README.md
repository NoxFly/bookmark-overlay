# Bookmark Overlay

Un panneau Windows, appelé par un raccourci clavier, pour retrouver une fiche en
deux secondes et ouvrir ses ressources — Azure DevOps, GitHub, Admin Center Business
Central, liens web, dossiers et espaces de travail Visual Studio Code — sans quitter
le clavier.

L'application vit discrètement dans la zone de notification : aucune fenêtre ni
entrée dans la barre des tâches tant que le raccourci n'est pas pressé.

## Fonctionnalités

- **Recherche instantanée** sur le nom, le tenant et les mots-clés, sans tenir compte
  des accents ni de la casse.
- **Liens calculés** à partir de gabarits d'URL, plus des liens et des espaces de
  travail propres à chaque fiche.
- **Profils de navigation** : chaque lien s'ouvre dans le bon navigateur et le bon
  profil selon son hôte — Edge pour l'un, Firefox pour l'autre. Edge, Chrome,
  Firefox, Firefox Developer Edition, Brave, Vivaldi, Opera et le navigateur par
  défaut sont pris en charge.
- **Données chiffrées** sur le poste, avec export et import pour partager une base.
- **Mise à jour automatique** en un clic, sans rien réinstaller à la main.
- Thème clair, sombre ou système ; démarrage avec Windows.

## Installation

Télécharger la dernière version depuis les
[releases GitHub](https://github.com/NoxFly/bookmark-overlay/releases/latest) :

| Fichier                                  | Taille  | Pour qui                          |
| ---------------------------------------- | ------- | --------------------------------- |
| `BookmarkOverlay-<version>-setup.exe`    | ~1,4 Mo | installation classique            |
| `BookmarkOverlay-<version>-portable.exe` | ~3,9 Mo | copie simple, aucune installation |

- **Windows SmartScreen affiche un avertissement** au premier lancement, les binaires
  n'étant pas signés : « Informations complémentaires » puis « Exécuter quand même ».
- L'application s'appuie sur WebView2, présent d'origine sur Windows 11 et installé
  avec Edge sur Windows 10 ; l'installateur le télécharge au besoin.
- Elle s'inscrit au démarrage de Windows dès le premier lancement. Pour la version
  portable, placer l'exécutable à son emplacement définitif **avant** de le lancer.

## Utilisation

`Ctrl+Alt+D` ouvre et ferme le panneau. On tape pour chercher, puis :

| Touche        | Action                                                          |
| ------------- | --------------------------------------------------------------- |
| `↑` `↓`       | naviguer dans les résultats                                     |
| `Entrée`      | ouvrir le dossier local, ou le premier espace de travail        |
| `Ctrl+Entrée` | ouvrir Azure DevOps                                             |
| `Maj+Entrée`  | ouvrir GitHub                                                   |
| `Alt+Entrée`  | ouvrir l'Admin Center                                           |
| `F2`          | modifier la fiche sélectionnée                                  |
| `Ctrl+N`      | nouvelle fiche                                                  |
| `Ctrl+,`      | réglages                                                        |
| `Échap`       | fermer ce qui est ouvert par-dessus, revenir, fermer le panneau |

Chaque fiche n'affiche que les boutons qu'elle sait ouvrir ; un clic sur l'un d'eux
ouvre la ressource et referme le panneau.

Les **réglages** (icône d'engrenage) regroupent les gabarits d'URL, les profils de
navigation, l'apparence, le raccourci global et la gestion des données. Le raccourci
se change en le tapant directement au clavier.

Quand une nouvelle version est publiée, une icône apparaît dans l'en-tête du
panneau : un clic affiche la version, sa date et son poids, et « Mettre à jour et
redémarrer » s'occupe du reste.

## Données

Les fiches restent sur le poste, chiffrées, dans
`%APPDATA%\fr.capvision.bookmark-overlay\data`. Réglages → Données permet d'ouvrir
ce dossier, d'exporter la base (chiffrée pour un collègue, ou lisible en JSON) et
d'en importer une, en fusion ou en remplacement. Une sauvegarde est écrite avant
chaque import.

## Documentation

- [Guide d'utilisation](docs/guide.md) : fiches, gabarits, profils de navigation,
  réglages, import et export en détail.
- [Architecture](docs/architecture.md) : fonctionnement interne et choix techniques.
- [Développement](docs/development.md) : build, clés de chiffrement, pipeline de
  release et mise à jour automatique.
