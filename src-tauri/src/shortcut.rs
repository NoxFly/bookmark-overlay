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

//! Enregistrement du raccourci clavier global qui ouvre l'overlay.

use std::str::FromStr;

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{validation, AppResult};
use crate::overlay;

/// Remplace le raccourci global courant par `accelerator`.
///
/// Le handler est volontairement minimal : il ne fait que basculer la fenêtre.
/// Tout travail supplémentaire ici retarderait l'apparition de l'overlay.
pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> AppResult<()> {
    let shortcut = Shortcut::from_str(accelerator).map_err(|error| {
        validation(format!(
            "Raccourci « {accelerator} » invalide : {error}. Exemple attendu : Ctrl+Alt+D."
        ))
    })?;

    let manager = app.global_shortcut();
    manager.unregister_all().map_err(|error| {
        validation(format!(
            "Impossible de libérer l'ancien raccourci : {error}"
        ))
    })?;

    manager
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if let Ok(window) = overlay::window(app) {
                // Un échec de bascule ne doit pas faire tomber le handler du
                // raccourci : on l'ignore, l'utilisateur pourra réessayer.
                let _ = overlay::toggle(&window);
            }
        })
        .map_err(|error| {
            validation(format!(
                "Le raccourci « {accelerator} » est refusé par le système : {error}. \
                 Il est probablement déjà utilisé par une autre application."
            ))
        })?;

    Ok(())
}

/// Libère le raccourci global, le temps d'en capturer un nouveau au clavier.
///
/// Sans cela, taper la combinaison en vigueur basculerait l'overlay au lieu
/// d'être capturée.
pub fn unregister_all<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|error| validation(format!("Impossible de suspendre le raccourci : {error}")))
}
