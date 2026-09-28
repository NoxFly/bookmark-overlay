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

// Aucune console en release : l'application démarre avec Windows et doit rester
// totalement silencieuse.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    bookmark_overlay_lib::run()?;
    Ok(())
}
