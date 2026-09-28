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

//! Navigateurs : détection de ceux installés et de leurs profils, puis ouverture
//! d'un lien dans le profil de navigation qui revendique son hôte.
//!
//! Aucun navigateur n'est imposé : chaque profil de navigation désigne le sien, si
//! bien qu'un lien peut partir dans Edge et le suivant dans Firefox.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Serialize;

use crate::error::{validation, AppResult};
use crate::launcher::{open_with_default_browser, spawn};
use crate::model::{is_web_url, BrowserKind, Settings};

/// Taille au-delà de laquelle une photo de profil n'est pas transmise au front.
const MAX_AVATAR_BYTES: u64 = 512 * 1024;

/// Fragment du chemin du profil que Firefox Developer Edition crée pour elle-même.
const DEVELOPER_PROFILE_MARKER: &str = "dev-edition";

/// Dossier de profil Chromium créé à l'installation, et donc profil par défaut.
const CHROMIUM_DEFAULT_PROFILE: &str = "Default";

/// Racine à laquelle se rapporte un emplacement d'installation.
#[derive(Debug, Clone, Copy)]
enum Root {
    /// `%LOCALAPPDATA%`, installations propres à l'utilisateur.
    LocalAppData,
    /// `%ProgramFiles%`.
    ProgramFiles,
    /// `%ProgramFiles(x86)%`.
    ProgramFilesX86,
}

/// Où trouver un navigateur et ses profils.
struct BrowserSpec {
    kind: BrowserKind,
    /// Emplacements de l'exécutable, testés dans l'ordre.
    executables: &'static [(Root, &'static str)],
    /// Nom de l'exécutable, cherché dans le `PATH` en dernier recours.
    executable_name: &'static str,
    /// Dossier `User Data` des navigateurs Chromium, sous `%LOCALAPPDATA%`.
    chromium_user_data: Option<&'static str>,
}

const SPECS: [BrowserSpec; 7] = [
    BrowserSpec {
        kind: BrowserKind::Edge,
        executables: &[
            (
                Root::ProgramFilesX86,
                r"Microsoft\Edge\Application\msedge.exe",
            ),
            (Root::ProgramFiles, r"Microsoft\Edge\Application\msedge.exe"),
        ],
        executable_name: "msedge.exe",
        chromium_user_data: Some(r"Microsoft\Edge\User Data"),
    },
    BrowserSpec {
        kind: BrowserKind::Chrome,
        executables: &[
            (Root::ProgramFiles, r"Google\Chrome\Application\chrome.exe"),
            (
                Root::ProgramFilesX86,
                r"Google\Chrome\Application\chrome.exe",
            ),
            (Root::LocalAppData, r"Google\Chrome\Application\chrome.exe"),
        ],
        executable_name: "chrome.exe",
        chromium_user_data: Some(r"Google\Chrome\User Data"),
    },
    BrowserSpec {
        kind: BrowserKind::Firefox,
        executables: &[
            (Root::ProgramFiles, r"Mozilla Firefox\firefox.exe"),
            (Root::ProgramFilesX86, r"Mozilla Firefox\firefox.exe"),
            (Root::LocalAppData, r"Mozilla Firefox\firefox.exe"),
        ],
        executable_name: "firefox.exe",
        chromium_user_data: None,
    },
    BrowserSpec {
        kind: BrowserKind::FirefoxDev,
        executables: &[
            (Root::ProgramFiles, r"Firefox Developer Edition\firefox.exe"),
            (
                Root::ProgramFilesX86,
                r"Firefox Developer Edition\firefox.exe",
            ),
            (Root::LocalAppData, r"Firefox Developer Edition\firefox.exe"),
        ],
        // Même nom que Firefox : le `PATH` désignerait n'importe laquelle des deux.
        executable_name: "",
        chromium_user_data: None,
    },
    BrowserSpec {
        kind: BrowserKind::Brave,
        executables: &[
            (
                Root::ProgramFiles,
                r"BraveSoftware\Brave-Browser\Application\brave.exe",
            ),
            (
                Root::ProgramFilesX86,
                r"BraveSoftware\Brave-Browser\Application\brave.exe",
            ),
            (
                Root::LocalAppData,
                r"BraveSoftware\Brave-Browser\Application\brave.exe",
            ),
        ],
        executable_name: "brave.exe",
        chromium_user_data: Some(r"BraveSoftware\Brave-Browser\User Data"),
    },
    BrowserSpec {
        kind: BrowserKind::Vivaldi,
        executables: &[
            (Root::LocalAppData, r"Vivaldi\Application\vivaldi.exe"),
            (Root::ProgramFiles, r"Vivaldi\Application\vivaldi.exe"),
            (Root::ProgramFilesX86, r"Vivaldi\Application\vivaldi.exe"),
        ],
        executable_name: "vivaldi.exe",
        chromium_user_data: Some(r"Vivaldi\User Data"),
    },
    BrowserSpec {
        kind: BrowserKind::Opera,
        executables: &[
            (Root::LocalAppData, r"Programs\Opera\opera.exe"),
            (Root::ProgramFiles, r"Opera\opera.exe"),
            (Root::ProgramFilesX86, r"Opera\opera.exe"),
        ],
        executable_name: "opera.exe",
        chromium_user_data: None,
    },
];

/// Un navigateur tel que vu sur la machine.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct InstalledBrowser {
    /// Navigateur concerné.
    pub kind: BrowserKind,
    /// Vrai si son exécutable a été trouvé.
    pub installed: bool,
    /// Profils déclarés ; vide pour un navigateur qui n'en expose pas.
    pub profiles: Vec<DetectedProfile>,
}

/// Un profil de navigateur détecté sur la machine.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct DetectedProfile {
    /// Valeur attendue en ligne de commande : dossier (Chromium) ou nom (Firefox).
    pub id: String,
    /// Nom affiché dans le navigateur.
    pub name: String,
    /// Compte associé, quand le navigateur le connaît.
    pub account: String,
    /// Vrai pour le profil que le navigateur ouvre de lui-même.
    pub is_default: bool,
    /// Photo du profil, en URL `data:`, quand le navigateur en conserve une.
    pub avatar: Option<String>,
    /// Couleur du profil (`#rrggbb`), quand le navigateur en attribue une.
    pub color: Option<String>,
}

/// Énumère les navigateurs connus, installés ou non, avec leurs profils.
///
/// Un fichier de profils illisible n'est pas une erreur : le navigateur est
/// simplement présenté sans profil, et reste utilisable.
pub fn detect() -> Vec<InstalledBrowser> {
    let mut browsers: Vec<InstalledBrowser> = SPECS
        .iter()
        .map(|spec| {
            let profiles = match (spec.kind, spec.chromium_user_data) {
                (_, Some(user_data)) => chromium_profiles(user_data),
                (kind, None) if kind.is_firefox() => firefox_profiles(kind),
                _ => Vec::new(),
            };
            InstalledBrowser {
                kind: spec.kind,
                installed: executable(spec).is_some(),
                profiles,
            }
        })
        .collect();
    browsers.push(InstalledBrowser {
        kind: BrowserKind::System,
        installed: true,
        profiles: Vec::new(),
    });
    browsers
}

/// Profil Edge par défaut de la machine, s'il en existe un.
pub fn default_edge_profile() -> Option<String> {
    let user_data = SPECS
        .iter()
        .find(|spec| spec.kind == BrowserKind::Edge)
        .and_then(|spec| spec.chromium_user_data)?;
    let profiles = chromium_profiles(user_data);
    profiles
        .iter()
        .find(|profile| profile.is_default)
        .or_else(|| profiles.first())
        .map(|profile| profile.id.clone())
}

/// Ouvre une URL dans le profil de navigation qui la revendique.
///
/// Si le navigateur choisi est introuvable, on retombe sur le navigateur par
/// défaut du système plutôt que d'échouer : le lien s'ouvre, sans le bon profil.
pub fn open_web(url: &str, settings: &Settings) -> AppResult<()> {
    if !is_web_url(url) {
        return Err(validation("URL refusée : seul http(s) est autorisé."));
    }
    match settings.profile_for(url) {
        Some(profile) => launch(profile.browser, &profile.profile, url),
        None => open_with_default_browser(url),
    }
}

/// Lance un navigateur sur une URL déjà validée.
fn launch(kind: BrowserKind, profile: &str, url: &str) -> AppResult<()> {
    let Some(spec) = SPECS.iter().find(|spec| spec.kind == kind) else {
        return open_with_default_browser(url);
    };
    let Some(executable) = executable(spec) else {
        return open_with_default_browser(url);
    };
    spawn(&executable, &launch_arguments(kind, profile, url))
}

/// Arguments de lancement, un par un : aucun interpréteur n'est impliqué.
fn launch_arguments(kind: BrowserKind, profile: &str, url: &str) -> Vec<String> {
    let mut arguments = Vec::with_capacity(4);
    if !profile.is_empty() {
        if kind.is_chromium() {
            arguments.push(format!("--profile-directory={profile}"));
        } else if kind.is_firefox() {
            arguments.push("-P".to_owned());
            arguments.push(profile.to_owned());
        }
    }
    if kind.is_firefox() {
        // Sans cela, une instance déjà ouverte sur ce profil ouvrirait une fenêtre.
        arguments.push("-new-tab".to_owned());
    }
    arguments.push(url.to_owned());
    arguments
}

/// Localise l'exécutable d'un navigateur : emplacements standards, puis `PATH`.
fn executable(spec: &BrowserSpec) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(found) = spec
            .executables
            .iter()
            .filter_map(|(root, relative)| root_path(*root).map(|base| base.join(relative)))
            .find(|candidate| candidate.is_file())
        {
            return Some(found);
        }
        if spec.executable_name.is_empty() {
            return None;
        }
        let paths = std::env::var_os("PATH")?;
        std::env::split_paths(&paths)
            .map(|directory| directory.join(spec.executable_name))
            .find(|candidate| candidate.is_file())
    }
    #[cfg(not(windows))]
    {
        let _ = spec;
        None
    }
}

/// Résout une racine d'installation depuis l'environnement.
fn root_path(root: Root) -> Option<PathBuf> {
    let variable = match root {
        Root::LocalAppData => "LOCALAPPDATA",
        Root::ProgramFiles => "ProgramFiles",
        Root::ProgramFilesX86 => "ProgramFiles(x86)",
    };
    std::env::var_os(variable).map(PathBuf::from)
}

/// Profils d'un navigateur Chromium, lus dans son fichier `Local State`.
///
/// Le nom affiché (« 2 professionnel ») n'est pas celui du dossier attendu par
/// `--profile-directory` (« Profile 3 ») : les deux sont remontés.
fn chromium_profiles(user_data_relative: &str) -> Vec<DetectedProfile> {
    let Some(user_data) = root_path(Root::LocalAppData).map(|base| base.join(user_data_relative))
    else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(user_data.join("Local State")) else {
        return Vec::new();
    };
    parse_chromium_local_state(&contents, &user_data)
}

/// Extrait les profils du contenu d'un `Local State`.
fn parse_chromium_local_state(contents: &str, user_data: &Path) -> Vec<DetectedProfile> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(contents) else {
        return Vec::new();
    };
    let Some(cache) = parsed
        .get("profile")
        .and_then(|profile| profile.get("info_cache"))
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };

    let text = |entry: &serde_json::Value, key: &str| {
        entry
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };

    let mut profiles: Vec<DetectedProfile> = cache
        .iter()
        .map(|(directory, entry)| {
            let name = match text(entry, "name") {
                name if name.is_empty() => directory.clone(),
                name => name,
            };
            let color = ["profile_highlight_color", "default_avatar_fill_color"]
                .iter()
                .find_map(|key| entry.get(*key).and_then(serde_json::Value::as_i64))
                .map(argb_to_hex);
            DetectedProfile {
                id: directory.clone(),
                name,
                account: text(entry, "user_name"),
                is_default: directory == CHROMIUM_DEFAULT_PROFILE,
                avatar: chromium_avatar(
                    &user_data.join(directory),
                    &text(entry, "gaia_picture_file_name"),
                ),
                color,
            }
        })
        .collect();
    profiles.sort_by(|left, right| left.id.cmp(&right.id));
    if !profiles.iter().any(|profile| profile.is_default) {
        if let Some(first) = profiles.first_mut() {
            first.is_default = true;
        }
    }
    profiles
}

/// Photo d'un profil Chromium : celle du compte connecté, quand elle existe.
fn chromium_avatar(profile_directory: &Path, gaia_file: &str) -> Option<String> {
    let mut candidates: Vec<&str> = Vec::with_capacity(3);
    // Le nom vient d'un fichier du navigateur : on refuse tout ce qui sortirait du
    // dossier du profil.
    if !gaia_file.is_empty() && !gaia_file.contains(['/', '\\']) && gaia_file != ".." {
        candidates.push(gaia_file);
    }
    candidates.extend(["Edge Profile Picture.png", "Google Profile Picture.png"]);

    candidates.into_iter().find_map(|file| {
        let path = profile_directory.join(file);
        let size = std::fs::metadata(&path).ok()?.len();
        if size == 0 || size > MAX_AVATAR_BYTES {
            return None;
        }
        let bytes = std::fs::read(&path).ok()?;
        let mime = image_mime(&bytes)?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Some(format!("data:{mime};base64,{encoded}"))
    })
}

/// Type d'une image d'après sa signature ; `None` pour tout autre contenu.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// Convertit une couleur ARGB (entier signé, tel que stocké par Chromium) en `#rrggbb`.
fn argb_to_hex(argb: i64) -> String {
    // Seuls les 24 bits de poids faible portent la couleur ; la troncature est voulue.
    let rgb = (argb as u32) & 0x00FF_FFFF;
    format!("#{rgb:06x}")
}

/// Profils Firefox, lus dans `profiles.ini`, commun aux deux éditions.
fn firefox_profiles(kind: BrowserKind) -> Vec<DetectedProfile> {
    let Some(app_data) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
    let path = Path::new(&app_data)
        .join("Mozilla")
        .join("Firefox")
        .join("profiles.ini");
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_firefox_profiles(&contents, kind == BrowserKind::FirefoxDev)
}

/// Extrait les profils d'un `profiles.ini`.
///
/// Le profil par défaut est celui désigné par la section `[Install…]` (Firefox
/// 67 et suivants), à défaut celui marqué `Default=1`, à défaut le premier.
///
/// Chaque installation a sa section `[Install<empreinte du dossier>]`, empreinte
/// qu'on ne sait pas recalculer. Developer Edition se reconnaît autrement : elle
/// crée elle-même son profil `dev-edition-default`, dont le chemin porte le nom.
fn parse_firefox_profiles(contents: &str, developer: bool) -> Vec<DetectedProfile> {
    let sections = parse_ini(contents);
    let is_developer_path = |path: &str| path.contains(DEVELOPER_PROFILE_MARKER);
    let install_default = sections
        .iter()
        .filter(|(name, _)| name.starts_with("Install"))
        .filter_map(|(_, entries)| ini_value(entries, "Default"))
        .find(|path| is_developer_path(path) == developer);

    let mut profiles: Vec<(DetectedProfile, Option<&str>, bool)> = sections
        .iter()
        .filter(|(name, _)| name.starts_with("Profile"))
        .filter_map(|(_, entries)| {
            let name = ini_value(entries, "Name")?;
            let profile = DetectedProfile {
                id: name.to_owned(),
                name: name.to_owned(),
                account: String::new(),
                is_default: false,
                avatar: None,
                color: None,
            };
            let flagged = ini_value(entries, "Default") == Some("1");
            Some((profile, ini_value(entries, "Path"), flagged))
        })
        .collect();

    let default_index = install_default
        .and_then(|path| {
            profiles
                .iter()
                .position(|(_, profile_path, _)| *profile_path == Some(path))
        })
        .or_else(|| {
            if developer {
                profiles
                    .iter()
                    .position(|(_, path, _)| path.is_some_and(is_developer_path))
            } else {
                profiles.iter().position(|(_, _, flagged)| *flagged)
            }
        })
        .or_else(|| (!profiles.is_empty()).then_some(0));
    if let Some(entry) = default_index.and_then(|index| profiles.get_mut(index)) {
        entry.0.is_default = true;
    }
    profiles
        .into_iter()
        .map(|(profile, _, _)| profile)
        .collect()
}

/// Découpe un fichier INI en sections `(nom, [(clé, valeur)])`.
fn parse_ini(contents: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            sections.push((name.trim().to_owned(), Vec::new()));
        } else if let (Some((key, value)), Some((_, entries))) =
            (line.split_once('='), sections.last_mut())
        {
            entries.push((key.trim().to_owned(), value.trim().to_owned()));
        }
    }
    sections
}

/// Valeur d'une clé dans une section INI.
fn ini_value<'a>(entries: &'a [(String, String)], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_web_refuses_a_non_http_url() {
        assert!(open_web("file:///C:/windows", &Settings::default()).is_err());
    }

    #[test]
    fn chromium_arguments_designate_the_profile_directory() {
        assert_eq!(
            launch_arguments(BrowserKind::Chrome, "Profile 2", "https://a.test"),
            vec![
                "--profile-directory=Profile 2".to_owned(),
                "https://a.test".to_owned()
            ]
        );
    }

    #[test]
    fn firefox_arguments_designate_the_profile_by_name() {
        assert_eq!(
            launch_arguments(BrowserKind::Firefox, "travail", "https://a.test"),
            vec![
                "-P".to_owned(),
                "travail".to_owned(),
                "-new-tab".to_owned(),
                "https://a.test".to_owned()
            ]
        );
    }

    #[test]
    fn an_empty_profile_launches_the_browser_on_its_own_default() {
        assert_eq!(
            launch_arguments(BrowserKind::Edge, "", "https://a.test"),
            vec!["https://a.test".to_owned()]
        );
    }

    #[test]
    fn chromium_local_state_lists_profiles_and_flags_the_default() {
        let local_state = r#"{"profile":{"info_cache":{
            "Profile 3":{"name":"Pro","user_name":"moi@exemple.test","profile_highlight_color":-16776961},
            "Default":{"name":"Perso"}
        }}}"#;
        let profiles = parse_chromium_local_state(local_state, Path::new("Z:\\absent"));

        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].id, "Default");
        assert!(profiles[0].is_default);
        assert_eq!(profiles[1].name, "Pro");
        assert_eq!(profiles[1].account, "moi@exemple.test");
        assert_eq!(profiles[1].color.as_deref(), Some("#0000ff"));
    }

    #[test]
    fn without_a_default_folder_the_first_chromium_profile_is_the_default() {
        let local_state = r#"{"profile":{"info_cache":{"Profile 1":{"name":"A"}}}}"#;
        let profiles = parse_chromium_local_state(local_state, Path::new("Z:\\absent"));
        assert!(profiles[0].is_default);
    }

    #[test]
    fn firefox_default_comes_from_the_install_section() {
        let ini = "[Profile1]\nName=default\nIsRelative=1\nPath=Profiles/a.default\nDefault=1\n\n\
                   [Profile0]\nName=Développement\nIsRelative=1\nPath=Profiles/b.dev\n\n\
                   [Install308046B0AF4A39CB]\nDefault=Profiles/b.dev\nLocked=1\n";
        let profiles = parse_firefox_profiles(ini, false);

        assert_eq!(profiles.len(), 2);
        let default: Vec<&str> = profiles
            .iter()
            .filter(|profile| profile.is_default)
            .map(|profile| profile.id.as_str())
            .collect();
        assert_eq!(default, vec!["Développement"]);
    }

    #[test]
    fn firefox_falls_back_on_the_flagged_profile() {
        let ini = "[Profile0]\nName=a\nPath=x\n[Profile1]\nName=b\nPath=y\nDefault=1\n";
        let profiles = parse_firefox_profiles(ini, false);
        assert!(!profiles[0].is_default);
        assert!(profiles[1].is_default);
    }

    /// Tel qu'écrit sur un poste où seule Developer Edition a été lancée.
    const DEVELOPER_INI: &str = "[Profile1]\nName=default\nPath=Profiles/q.default\nDefault=1\n\
                                 [Profile0]\nName=dev-edition-default\nPath=Profiles/n.dev-edition-default\n\
                                 [InstallCA9422711AE1A81C]\nDefault=Profiles/n.dev-edition-default\n";

    fn default_of(profiles: &[DetectedProfile]) -> Vec<&str> {
        profiles
            .iter()
            .filter(|profile| profile.is_default)
            .map(|profile| profile.id.as_str())
            .collect()
    }

    #[test]
    fn developer_edition_defaults_to_its_own_profile() {
        let profiles = parse_firefox_profiles(DEVELOPER_INI, true);
        assert_eq!(default_of(&profiles), vec!["dev-edition-default"]);
    }

    #[test]
    fn regular_firefox_ignores_the_developer_install_section() {
        let profiles = parse_firefox_profiles(DEVELOPER_INI, false);
        assert_eq!(default_of(&profiles), vec!["default"]);
    }

    #[test]
    fn developer_edition_arguments_match_firefox() {
        assert_eq!(
            launch_arguments(
                BrowserKind::FirefoxDev,
                "dev-edition-default",
                "https://a.test"
            ),
            launch_arguments(
                BrowserKind::Firefox,
                "dev-edition-default",
                "https://a.test"
            )
        );
    }

    #[test]
    fn an_avatar_must_be_an_image() {
        assert_eq!(image_mime(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
        assert_eq!(image_mime(b"<svg"), None);
    }
}
