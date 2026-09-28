# Guide d'utilisation

## Recherche

La recherche porte sur le nom, le tenant et les mots-clés, sans tenir compte des
accents ni de la casse ; tous les mots tapés doivent correspondre. Les fiches dont
le nom commence par la recherche passent en tête.

Sa valeur survit à la fermeture du panneau : à la réouverture elle est
sélectionnée, donc une frappe la remplace et `Entrée` la réutilise.

## Fiches

| Champ                  | Rôle                                                              |
| ---------------------- | ----------------------------------------------------------------- |
| Nom                    | nom affiché, obligatoire                                          |
| Tenant                 | facultatif ; unique s'il est renseigné ; alimente l'Admin Center  |
| Mots-clés              | termes de recherche supplémentaires, séparés par des virgules     |
| ID projet DevOps       | alimente le lien Azure DevOps                                     |
| ID dépôt GitHub        | alimente le lien GitHub                                           |
| Dossier local          | ouvert dans l'explorateur                                         |
| Espaces de travail     | dossiers ou fichiers `.code-workspace` nommés, ouverts dans VS Code |
| Liens personnalisés    | paires nom + URL, ouvertes dans le navigateur                     |

Le formulaire affiche en direct les URL que produiront les identifiants saisis.

Dans la liste, **les liens occupent une rangée et ce qui s'ouvre en local la
suivante** ; chaque rangée défile pour elle-même. Une fiche n'affiche que les
boutons qu'elle sait ouvrir : sans identifiant DevOps, pas de bouton DevOps ; sans
dossier, pas de bouton « Dossier ».

### Espaces de travail

Une fiche dont le travail est éclaté en plusieurs endroits (l'extension, le harnais
de test, la documentation…) peut déclarer autant d'espaces de travail que
nécessaire. Chacun pointe soit vers un **dossier**, soit vers un fichier
**`.code-workspace`** — le formulaire propose un sélecteur pour chacun — et s'ouvre
dans une nouvelle fenêtre de Visual Studio Code. Sans dossier local, `Entrée` ouvre
le premier espace de travail.

### Liens personnalisés

Tout ce qui n'entre pas dans un gabarit — un extranet, une supervision, un outil de
tickets — s'ajoute en paire nom + URL. Seul `http(s)` est accepté. Ces liens partent,
comme les autres, dans le profil de navigation qui revendique leur hôte.

## Réglages

Les réglages s'ouvrent sur un menu ; chaque entrée affiche sa section seule, et
`Retour` ou `Échap` reviennent au menu puis à la liste. Gabarits et profils de
navigation s'enregistrent avec leur bouton **Enregistrer** — quitter la section sans
enregistrer annule la saisie. Tout le reste s'applique immédiatement.

### Gabarits d'URL

Trois gabarits produisent les liens DevOps, GitHub et Admin Center, avec les jetons
`{tenant}`, `{devopsId}` et `{githubId}`. Les jetons reconnus sont colorés, un jeton
inconnu apparaît en rouge. Les valeurs injectées sont encodées ; seul `http(s)` est
accepté.

Valeurs par défaut :

```
https://dev.azure.com/ORGANISATION/{devopsId}
https://github.com/ORGANISATION/{githubId}
https://businesscentral.dynamics.com/{tenant}/admin
```

### Profils de navigation

Chaque lien web part dans un **profil de navigation** : un nom, un navigateur, un
profil de ce navigateur, et une liste d'hôtes. Deux profils existent au départ,
« DevOps et GitHub » et « Admin Center », et l'on peut en ajouter autant que
nécessaire, chacun sur le navigateur de son choix.

| Navigateur                                    | Choix du profil    |
| --------------------------------------------- | ------------------ |
| Microsoft Edge, Google Chrome, Brave, Vivaldi | oui                |
| Mozilla Firefox, Firefox Developer Edition    | oui                |
| Opera                                         | non, profil unique |
| Navigateur par défaut de Windows              | non                |

Safari n'existe pas sous Windows.

- Un lien part dans le **premier** profil dont un hôte correspond, sous-domaines
  compris : `dynamics.com` couvre `businesscentral.dynamics.com`. Un hôte présent
  dans deux profils revient au premier ; les flèches de chaque carte changent
  l'ordre.
- Aucun profil ne correspond : le lien part dans le premier de la liste. Il en faut
  toujours au moins un.
- Le champ des hôtes accepte une URL collée telle quelle.
- La liste des navigateurs présente d'abord ceux détectés sur le poste, puis les
  autres, puis le navigateur par défaut. Choisir un navigateur présélectionne son
  profil par défaut ; la photo du compte connecté est affichée quand le navigateur en
  conserve une.
- Si le navigateur choisi est introuvable, le lien part dans le navigateur par
  défaut.

### Apparence

Clair, sombre ou système. En mode système, l'interface suit le réglage de Windows
en direct.

### Système

- **Raccourci global** : **Modifier…** ouvre une fenêtre qui écoute le clavier et
  affiche la combinaison tapée. `Entrée` valide, `Échap` annule. Il faut au moins un
  modificateur (Ctrl, Alt, Maj, Win). Un raccourci déjà pris par une autre
  application est refusé et l'ancien reste en place.
- **Version** : la version en cours, et **Rechercher une mise à jour** pour
  vérifier sans attendre la vérification automatique. Un build local, qui ne suit
  aucune release, grise le bouton.
- **Installer les mises à jour automatiquement** (désactivé par défaut) : une
  mise à jour trouvée est installée sans rien demander, dès que le panneau est
  fermé ; l'application redémarre d'elle-même.
- **Démarrer avec Windows** : activé au premier lancement, vers l'emplacement de
  l'exécutable à ce moment-là. Après avoir déplacé l'exécutable, couper puis
  rallumer l'option pour corriger le chemin.

### Données

Le fichier de données est propre à chaque poste :
`%APPDATA%\io.github.noxfly.bookmark-overlay\data`.

- **Exporter** produit un fichier chiffré `.customers`, importable tel quel par un
  collègue ; **Exporter en clair** produit un JSON lisible, pour inspecter ou
  modifier la base en masse. L'import reconnaît les deux.
- **Importer (fusion)** conserve les fiches existantes, ajoute les nouvelles et met à
  jour celles déjà connues ; **Importer (remplacer)** écrase toute la base.
- Avant chaque import, une sauvegarde `data.backup-<horodatage>.json` est écrite à
  côté du fichier courant.
- Un fichier importé est refusé s'il dépasse 16 Mio, s'il ne respecte pas le schéma,
  ou s'il contient deux fois le même tenant.

## Zone de notification

Le menu de l'icône rappelle la version en cours, puis propose : ouvrir le panneau, ouvrir le dossier des données,
**Redémarrer** et **Quitter**. Un clic gauche sur l'icône ouvre ou ferme le panneau.

## Mise à jour

Quand une nouvelle version est publiée, une icône apparaît dans l'en-tête. La
fenêtre de mise à jour affiche la version, la date de publication, le poids et un
lien vers la release sur GitHub ; **Mettre à jour et redémarrer** télécharge, installe et relance
l'application sans autre intervention, qu'elle ait été installée ou copiée en
version portable. La version portable doit se trouver dans un dossier accessible en
écriture.
