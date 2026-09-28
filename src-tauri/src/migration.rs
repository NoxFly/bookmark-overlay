// Bookmark Overlay
// Copyright (C) 2026 NoxFly
//
// FR : Ce programme est un logiciel libre ; vous pouvez le redistribuer ou le
// modifier selon les termes de la GNU Affero General Public License, version 3,
// telle que publiée par la Free Software Foundation. Il est distribué dans
// l'espoir d'être utile, mais SANS AUCUNE GARANTIE. Voir le fichier LICENSE.
//
// EN : This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License, version 3, as
// published by the Free Software Foundation. It is distributed in the hope that
// it will be useful, but WITHOUT ANY WARRANTY. See the LICENSE file.
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reprise des données d'un ancien identifiant d'application.
//!
//! L'identifiant nomme le dossier de configuration (`%APPDATA%\<identifiant>`) :
//! le changer fait démarrer l'application sur un dossier vide. Tant que le dossier
//! courant n'a pas de fichier de données, on cherche donc, à côté de lui, un autre
//! dossier `*.bookmark-overlay` qui en a un, et on en déplace les données. Aucun
//! ancien identifiant n'est écrit ici : seul le suffixe, commun à tous, sert à les
//! reconnaître.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Suffixe commun à tous les identifiants de l'application.
const IDENTIFIER_SUFFIX: &str = ".bookmark-overlay";

/// Préfixe des sauvegardes écrites avant chaque import.
const BACKUP_PREFIX: &str = "data.backup-";

/// Déplace les données d'un ancien dossier de configuration vers `config_dir`, si
/// celui-ci n'a pas encore de fichier de données. Retourne le dossier repris.
///
/// Parmi plusieurs candidats, le fichier de données modifié le plus récemment
/// l'emporte : c'est celui de la dernière version utilisée.
pub fn adopt_previous_identifier(
    config_dir: &Path,
    data_file: &str,
) -> io::Result<Option<PathBuf>> {
    if config_dir.join(data_file).exists() {
        return Ok(None);
    }
    let Some(parent) = config_dir.parent() else {
        return Ok(None);
    };
    let Some(source) = latest_candidate(parent, config_dir, data_file)? else {
        return Ok(None);
    };

    fs::create_dir_all(config_dir)?;
    for entry in fs::read_dir(&source)? {
        let entry = entry?;
        let name = entry.file_name();
        let is_data = name == data_file
            || name
                .to_str()
                .is_some_and(|name| name.starts_with(BACKUP_PREFIX));
        if is_data && entry.file_type()?.is_file() {
            move_file(&entry.path(), &config_dir.join(&name))?;
        }
    }
    Ok(Some(source))
}

/// Dossier `*.bookmark-overlay` voisin, autre que le courant, dont le fichier de
/// données est le plus récent.
fn latest_candidate(parent: &Path, current: &Path, data_file: &str) -> io::Result<Option<PathBuf>> {
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let matches = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(IDENTIFIER_SUFFIX));
        if !matches || path == current {
            continue;
        }
        let Ok(metadata) = fs::metadata(path.join(data_file)) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if best.as_ref().is_none_or(|(time, _)| modified > *time) {
            best = Some((modified, path));
        }
    }
    Ok(best.map(|(_, path)| path))
}

/// Déplace un fichier ; à défaut de renommage possible, le copie puis supprime
/// l'original.
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to)?;
    fs::remove_file(from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(label: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("overlay-migration-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("dossier de test");
        root
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("dossier");
        fs::write(path, contents).expect("écriture");
    }

    #[test]
    fn data_and_backups_move_from_a_previous_identifier() {
        let root = temporary_root("move");
        let previous = root.join("org.ancien.bookmark-overlay");
        write(&previous.join("data"), "chiffré");
        write(&previous.join("data.backup-1.json"), "{}");
        write(&previous.join("EBWebView").join("cache"), "x");
        let current = root.join("io.github.noxfly.bookmark-overlay");

        let adopted = adopt_previous_identifier(&current, "data").expect("reprise");

        assert_eq!(adopted, Some(previous.clone()));
        assert_eq!(
            fs::read_to_string(current.join("data")).expect("lecture"),
            "chiffré"
        );
        assert!(current.join("data.backup-1.json").is_file());
        assert!(
            !previous.join("data").exists(),
            "le fichier est déplacé, pas copié"
        );
        assert!(
            !current.join("EBWebView").exists(),
            "le cache de la webview reste en place"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn existing_data_is_never_replaced() {
        let root = temporary_root("keep");
        write(
            &root.join("org.ancien.bookmark-overlay").join("data"),
            "ancien",
        );
        let current = root.join("io.github.noxfly.bookmark-overlay");
        write(&current.join("data"), "actuel");

        assert_eq!(
            adopt_previous_identifier(&current, "data").expect("reprise"),
            None
        );
        assert_eq!(
            fs::read_to_string(current.join("data")).expect("lecture"),
            "actuel"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unrelated_folders_are_ignored() {
        let root = temporary_root("unrelated");
        write(&root.join("une-autre-application").join("data"), "étranger");
        write(
            &root.join("org.ancien.bookmark-overlay-bis").join("data"),
            "étranger",
        );
        let current = root.join("io.github.noxfly.bookmark-overlay");

        assert_eq!(
            adopt_previous_identifier(&current, "data").expect("reprise"),
            None
        );
        assert!(!current.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_previous_folder_without_data_is_skipped() {
        let root = temporary_root("empty");
        fs::create_dir_all(root.join("org.ancien.bookmark-overlay")).expect("dossier");
        let current = root.join("io.github.noxfly.bookmark-overlay");

        assert_eq!(
            adopt_previous_identifier(&current, "data").expect("reprise"),
            None
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn the_most_recent_data_wins() {
        let root = temporary_root("recent");
        let older = root.join("org.a.bookmark-overlay");
        let newer = root.join("org.b.bookmark-overlay");
        write(&older.join("data"), "ancien");
        std::thread::sleep(std::time::Duration::from_millis(50));
        write(&newer.join("data"), "récent");
        let current = root.join("io.github.noxfly.bookmark-overlay");

        let adopted = adopt_previous_identifier(&current, "data").expect("reprise");

        assert_eq!(adopted, Some(newer));
        assert_eq!(
            fs::read_to_string(current.join("data")).expect("lecture"),
            "récent"
        );
        assert!(
            older.join("data").is_file(),
            "l'autre candidat n'est pas touché"
        );
        let _ = fs::remove_dir_all(root);
    }
}
