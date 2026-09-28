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

//! Script de build : contexte Tauri, et clés de chiffrement injectées à la compilation.
//!
//! Le dépôt est public : aucune clé n'y est écrite. Elles viennent, dans l'ordre :
//!
//! 1. des variables d'environnement `BOOKMARK_OVERLAY_DATA_KEY` et
//!    `BOOKMARK_OVERLAY_LEGACY_KEY` (secrets GitHub en CI) ;
//! 2. du fichier `.env.local`, ignoré par git, pour développer et tester en local
//!    sans passer par la pipeline ;
//! 3. à défaut, d'une clé de développement publique — un tel binaire ne lit pas
//!    les données d'un binaire officiel, et un avertissement le rappelle.
//!
//! Chaque clé fait 32 octets, écrits en 64 caractères hexadécimaux.

use std::collections::HashMap;
use std::path::PathBuf;

/// Variable de la clé courante.
const DATA_KEY: &str = "BOOKMARK_OVERLAY_DATA_KEY";

/// Variable de la clé des versions Customers Overlay, facultative.
const LEGACY_KEY: &str = "BOOKMARK_OVERLAY_LEGACY_KEY";

/// Fichier local de clés, au format `NOM=valeur`.
const LOCAL_FILE: &str = ".env.local";

/// Clé de développement : publique, donc sans valeur de secret.
const DEVELOPMENT_KEY: &str = "6465762d6f6e6c792d6b65792d6e6f742d612d7365637265742d2d2d2d2d2d21";

fn main() {
    // Le dépôt suivi par la mise à jour automatique est capturé à la compilation
    // (`option_env!`) : il faut recompiler s'il change.
    println!("cargo:rerun-if-env-changed=GITHUB_REPOSITORY");
    println!("cargo:rerun-if-env-changed={DATA_KEY}");
    println!("cargo:rerun-if-env-changed={LEGACY_KEY}");
    println!("cargo:rerun-if-changed={LOCAL_FILE}");

    if let Err(message) = write_keys() {
        // Une clé mal formée doit arrêter le build : un binaire qui chiffrerait
        // avec une clé inattendue rendrait les données illisibles ailleurs.
        eprintln!("{message}");
        std::process::exit(1);
    }

    tauri_build::build()
}

/// Écrit `keys.rs` dans `OUT_DIR`, inclus par le module de chiffrement.
fn write_keys() -> Result<(), String> {
    let local = read_local_file()?;
    let lookup = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| local.get(name).cloned())
    };

    let data_key = match lookup(DATA_KEY) {
        Some(value) => parse_key(DATA_KEY, &value)?,
        None => {
            println!(
                "cargo:warning=Aucune clé {DATA_KEY} (ni variable, ni {LOCAL_FILE}) : clé de développement utilisée, ce binaire ne lira pas les données d'un binaire officiel."
            );
            parse_key("clé de développement", DEVELOPMENT_KEY)?
        }
    };
    let legacy_key = lookup(LEGACY_KEY)
        .map(|value| parse_key(LEGACY_KEY, &value))
        .transpose()?;

    let legacy = match legacy_key {
        Some(key) => format!("Some({})", as_array(&key)),
        None => "None".to_owned(),
    };
    let source = format!(
        "/// Clé de chiffrement courante.
         const KEY: [u8; 32] = {};
         /// Clé des fichiers Customers Overlay, si le build la connaît.
         const LEGACY_KEY: Option<[u8; 32]> = {legacy};
",
        as_array(&data_key)
    );

    let out_dir = std::env::var("OUT_DIR").map_err(|error| format!("OUT_DIR absent : {error}"))?;
    std::fs::write(PathBuf::from(out_dir).join("keys.rs"), source)
        .map_err(|error| format!("écriture de keys.rs impossible : {error}"))
}

/// Lit `.env.local` s'il existe.
fn read_local_file() -> Result<HashMap<String, String>, String> {
    let contents = match std::fs::read_to_string(LOCAL_FILE) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(format!("{LOCAL_FILE} illisible : {error}")),
    };
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect())
}

/// Décode une clé de 64 caractères hexadécimaux.
fn parse_key(name: &str, value: &str) -> Result<[u8; 32], String> {
    let value = value.trim();
    if value.len() != 64 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "{name} doit faire 64 caractères hexadécimaux (32 octets)."
        ));
    }
    let mut key = [0_u8; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        let pair = &value[index * 2..index * 2 + 2];
        *byte = u8::from_str_radix(pair, 16).map_err(|error| format!("{name} : {error}"))?;
    }
    Ok(key)
}

/// Représentation Rust d'un tableau d'octets.
fn as_array(key: &[u8; 32]) -> String {
    let bytes: Vec<String> = key.iter().map(|byte| format!("0x{byte:02x}")).collect();
    format!("[{}]", bytes.join(", "))
}
