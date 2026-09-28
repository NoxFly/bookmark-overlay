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

//! Modèle de données persisté : clients et réglages de l'overlay.

use serde::{Deserialize, Serialize};

use crate::error::{validation, AppError, AppResult};

/// Longueur maximale acceptée pour un champ texte libre.
///
/// Borne volontairement large, mais finie : le fichier de données est relu à chaque
/// démarrage, une entrée aberrante ne doit pas peser sur le temps de boot.
const MAX_FIELD_LEN: usize = 512;

/// Nombre maximal de mots-clés conservés par client.
const MAX_KEYWORDS: usize = 64;

/// Nombre maximal d'espaces de travail par client.
const MAX_WORKSPACES: usize = 32;

/// Nombre maximal de liens personnalisés par client.
const MAX_LINKS: usize = 32;

/// Un lien personnalisé : un nom à afficher et l'URL qu'il ouvre.
///
/// Complète les trois liens calculés (DevOps, GitHub, Admin Center) pour tout ce
/// qui n'entre pas dans un gabarit : un extranet, une supervision, un ticket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CustomLink {
    /// Libellé affiché sur le bouton.
    pub name: String,
    /// URL ouverte dans le navigateur.
    pub url: String,
}

impl CustomLink {
    /// Normalise les deux champs et vérifie que l'URL est bien en http(s).
    fn sanitized(self) -> AppResult<Self> {
        let name = self.name.trim().to_owned();
        let url = self.url.trim().to_owned();

        if name.is_empty() {
            return Err(validation("Un lien personnalisé doit porter un nom."));
        }
        if !is_web_url(&url) {
            return Err(validation(format!(
                "L'URL du lien « {name} » doit commencer par http:// ou https://."
            )));
        }
        if name.chars().count() > MAX_FIELD_LEN || url.chars().count() > MAX_FIELD_LEN {
            return Err(validation(format!(
                "Le lien « {name} » dépasse {MAX_FIELD_LEN} caractères."
            )));
        }
        Ok(Self { name, url })
    }

    /// Compare deux noms de lien sans tenir compte de la casse.
    pub fn has_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name.trim())
    }
}

/// Un espace de travail : un nom à afficher et le dossier local qu'il ouvre.
///
/// Sert aux clients dont le travail est éclaté en plusieurs dossiers (l'extension,
/// le harnais de test, la documentation...) : chacun apparaît comme un bouton
/// distinct dans la liste et s'ouvre dans Visual Studio Code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// Libellé affiché sur le bouton.
    pub name: String,
    /// Chemin du dossier local à ouvrir.
    pub path: String,
}

impl Workspace {
    /// Normalise les deux champs et vérifie qu'aucun n'est vide.
    fn sanitized(self) -> AppResult<Self> {
        let name = self.name.trim().to_owned();
        let path = self.path.trim().to_owned();

        if name.is_empty() {
            return Err(validation("Un espace de travail doit porter un nom."));
        }
        if path.is_empty() {
            return Err(validation(format!(
                "L'espace de travail « {name} » n'a pas de dossier."
            )));
        }
        if name.chars().count() > MAX_FIELD_LEN || path.chars().count() > MAX_FIELD_LEN {
            return Err(validation(format!(
                "L'espace de travail « {name} » dépasse {MAX_FIELD_LEN} caractères."
            )));
        }
        Ok(Self { name, path })
    }

    /// Compare deux noms d'espace de travail sans tenir compte de la casse.
    pub fn has_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name.trim())
    }
}

/// Une fiche client.
///
/// La clé est `id`, un identifiant technique attribué à la création et jamais
/// réutilisé. Le tenant a longtemps joué ce rôle, mais il est facultatif : tous
/// les clients n'en ont pas, et une fiche sans tenant reste une fiche.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    /// Clé technique, opaque et stable.
    #[serde(default)]
    pub id: String,
    /// Tenant Business Central, facultatif ; alimente l'URL de l'Admin Center.
    #[serde(default)]
    pub tenant: String,
    /// Raison sociale affichée dans la liste.
    pub name: String,
    /// Mots-clés libres, exploités par la recherche.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Chemin du dossier de travail local.
    #[serde(default)]
    pub folder_path: String,
    /// Identifiant du projet Azure DevOps.
    #[serde(default)]
    pub devops_id: String,
    /// Identifiant du dépôt GitHub.
    #[serde(default)]
    pub github_id: String,
    /// Dossiers supplémentaires, ouverts dans Visual Studio Code.
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    /// Liens supplémentaires, ouverts dans le navigateur.
    #[serde(default)]
    pub links: Vec<CustomLink>,
}

impl Customer {
    /// Normalise les champs (trim, mots-clés vides supprimés) et vérifie les
    /// invariants métier. Toute donnée venant du front passe obligatoirement par ici.
    pub fn sanitized(self) -> AppResult<Self> {
        let tenant = self.tenant.trim().to_owned();
        let name = self.name.trim().to_owned();

        if name.is_empty() {
            return Err(validation("Le nom du client est obligatoire."));
        }

        let id = match self.id.trim() {
            "" => new_id(),
            existing => existing.to_owned(),
        };

        let keywords: Vec<String> = self
            .keywords
            .into_iter()
            .map(|keyword| keyword.trim().to_owned())
            .filter(|keyword| !keyword.is_empty())
            .take(MAX_KEYWORDS)
            .collect();

        let mut workspaces: Vec<Workspace> = Vec::with_capacity(self.workspaces.len());
        for workspace in self.workspaces.into_iter().take(MAX_WORKSPACES) {
            let workspace = workspace.sanitized()?;
            if workspaces.iter().any(|kept| kept.has_name(&workspace.name)) {
                return Err(validation(format!(
                    "Deux espaces de travail portent le nom « {} ».",
                    workspace.name
                )));
            }
            workspaces.push(workspace);
        }

        let mut links: Vec<CustomLink> = Vec::with_capacity(self.links.len());
        for link in self.links.into_iter().take(MAX_LINKS) {
            let link = link.sanitized()?;
            if links.iter().any(|kept| kept.has_name(&link.name)) {
                return Err(validation(format!(
                    "Deux liens personnalisés portent le nom « {} ».",
                    link.name
                )));
            }
            links.push(link);
        }

        let sanitized = Self {
            id,
            tenant,
            name,
            keywords,
            folder_path: self.folder_path.trim().to_owned(),
            devops_id: self.devops_id.trim().to_owned(),
            github_id: self.github_id.trim().to_owned(),
            workspaces,
            links,
        };
        sanitized.check_lengths()?;
        Ok(sanitized)
    }

    /// Rejette les champs anormalement longs (fichier importé, copier-coller accidentel).
    fn check_lengths(&self) -> AppResult<()> {
        let fields = [
            ("tenant", &self.tenant),
            ("nom", &self.name),
            ("chemin du dossier", &self.folder_path),
            ("identifiant DevOps", &self.devops_id),
            ("identifiant GitHub", &self.github_id),
        ];
        for (label, value) in fields {
            if value.chars().count() > MAX_FIELD_LEN {
                return Err(validation(format!(
                    "Le champ « {label} » dépasse {MAX_FIELD_LEN} caractères."
                )));
            }
        }
        if let Some(keyword) = self
            .keywords
            .iter()
            .find(|keyword| keyword.chars().count() > MAX_FIELD_LEN)
        {
            return Err(validation(format!(
                "Le mot-clé « {} … » dépasse {MAX_FIELD_LEN} caractères.",
                keyword.chars().take(32).collect::<String>()
            )));
        }
        Ok(())
    }

    /// Compare deux tenants sans tenir compte de la casse.
    ///
    /// Un tenant vide ne « correspond » à rien : plusieurs fiches peuvent s'en
    /// passer sans être considérées comme des doublons.
    pub fn has_tenant(&self, tenant: &str) -> bool {
        let tenant = tenant.trim();
        !tenant.is_empty() && self.tenant.eq_ignore_ascii_case(tenant)
    }

    /// Compare la clé technique.
    pub fn has_id(&self, id: &str) -> bool {
        self.id == id.trim()
    }
}

/// Navigateur dans lequel un profil de navigation ouvre ses liens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub enum BrowserKind {
    /// Navigateur par défaut de Windows, sans choix de profil.
    System,
    /// Microsoft Edge.
    Edge,
    /// Google Chrome.
    Chrome,
    /// Mozilla Firefox.
    Firefox,
    /// Firefox Developer Edition, installée à part mais qui partage les profils de Firefox.
    FirefoxDev,
    /// Brave.
    Brave,
    /// Vivaldi.
    Vivaldi,
    /// Opera, qui n'expose pas ses profils en ligne de commande.
    Opera,
}

impl BrowserKind {
    /// Vrai pour les navigateurs Chromium, qui partagent `--profile-directory`.
    pub fn is_chromium(self) -> bool {
        match self {
            Self::Edge | Self::Chrome | Self::Brave | Self::Vivaldi => true,
            Self::System | Self::Firefox | Self::FirefoxDev | Self::Opera => false,
        }
    }

    /// Vrai si le navigateur sait ouvrir un lien dans un profil désigné.
    pub fn supports_profiles(self) -> bool {
        self.is_chromium() || self.is_firefox()
    }

    /// Vrai pour les éditions de Firefox, qui partagent `-P <nom>` et `profiles.ini`.
    pub fn is_firefox(self) -> bool {
        matches!(self, Self::Firefox | Self::FirefoxDev)
    }
}

/// Un profil de navigation : un navigateur, un de ses profils, et les hôtes qui
/// lui reviennent.
///
/// Chaque lien web part dans le premier profil dont un hôte correspond ; faute de
/// correspondance, dans le premier profil de la liste.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    /// Libellé affiché dans les réglages.
    pub name: String,
    /// Navigateur lancé.
    pub browser: BrowserKind,
    /// Profil du navigateur : dossier pour Chromium, nom pour Firefox. Vide, le
    /// navigateur est lancé sans désigner de profil.
    #[serde(default)]
    pub profile: String,
    /// Hôtes revendiqués ; chacun couvre aussi ses sous-domaines.
    #[serde(default)]
    pub hosts: Vec<String>,
}

impl BrowserProfile {
    /// Normalise le profil et vérifie qu'il est sûr à passer en ligne de commande.
    fn sanitized(self) -> AppResult<Self> {
        let name = self.name.trim().to_owned();
        if name.is_empty() {
            return Err(validation("Un profil de navigation doit porter un nom."));
        }
        if name.chars().count() > MAX_FIELD_LEN {
            return Err(validation(format!(
                "Le nom du profil « {name} » dépasse {MAX_FIELD_LEN} caractères."
            )));
        }

        let profile = if self.browser.supports_profiles() {
            self.profile.trim().to_owned()
        } else {
            String::new()
        };
        let is_safe = profile.is_empty()
            || if self.browser.is_chromium() {
                is_safe_profile_directory(&profile)
            } else {
                is_safe_profile_name(&profile)
            };
        if !is_safe {
            return Err(validation(format!(
                "Le profil du navigateur de « {name} » contient des caractères interdits."
            )));
        }

        let mut hosts: Vec<String> = Vec::with_capacity(self.hosts.len());
        for raw in self.hosts.iter().take(MAX_HOSTS) {
            if raw.trim().is_empty() {
                continue;
            }
            let host = normalize_host(raw).ok_or_else(|| {
                validation(format!(
                    "L'hôte « {} » du profil « {name} » est invalide.",
                    raw.trim()
                ))
            })?;
            if !hosts.contains(&host) {
                hosts.push(host);
            }
        }

        Ok(Self {
            name,
            browser: self.browser,
            profile,
            hosts,
        })
    }

    /// Vrai si l'hôte (déjà en minuscules) relève de ce profil, sous-domaines compris.
    pub fn claims(&self, host: &str) -> bool {
        self.hosts.iter().any(|claimed| {
            host == claimed
                || host
                    .strip_suffix(claimed.as_str())
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
    }
}

/// Réglages modifiables depuis l'overlay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase", from = "StoredSettings")]
pub struct Settings {
    /// Gabarit d'URL Azure DevOps ; jetons acceptés : `{devopsId}`, `{githubId}`, `{tenant}`.
    pub devops_url_template: String,
    /// Gabarit d'URL GitHub ; mêmes jetons.
    pub github_url_template: String,
    /// Gabarit d'URL de l'Admin Center Business Central ; mêmes jetons.
    pub admin_center_url_template: String,
    /// Profils de navigation, par ordre de priorité.
    pub browser_profiles: Vec<BrowserProfile>,
    /// Raccourci global d'ouverture de l'overlay, au format accélérateur Tauri.
    pub hotkey: String,
    /// Apparence de l'interface : `system`, `light` ou `dark`.
    #[cfg_attr(test, ts(type = r#""system" | "light" | "dark""#))]
    pub theme: String,
    /// Installe les mises à jour dès qu'elles sont trouvées, sans rien demander.
    pub auto_update: bool,
}

/// Forme lue sur disque : accepte encore les deux profils Edge des versions
/// antérieures aux profils de navigation, pour les convertir.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    devops_url_template: String,
    github_url_template: String,
    admin_center_url_template: String,
    #[serde(default)]
    browser_profiles: Vec<BrowserProfile>,
    #[serde(default)]
    edge_profile_primary: Option<String>,
    #[serde(default)]
    edge_profile_admin: Option<String>,
    hotkey: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default)]
    auto_update: bool,
}

impl From<StoredSettings> for Settings {
    fn from(stored: StoredSettings) -> Self {
        // Une liste vide ne peut venir que d'un fichier antérieur : l'application
        // refuse d'enregistrer des réglages sans aucun profil.
        let browser_profiles = if stored.browser_profiles.is_empty() {
            default_browser_profiles(
                [
                    &stored.devops_url_template,
                    &stored.github_url_template,
                    &stored.admin_center_url_template,
                ],
                stored
                    .edge_profile_primary
                    .unwrap_or_else(|| FALLBACK_EDGE_PROFILE.to_owned()),
                stored
                    .edge_profile_admin
                    .unwrap_or_else(|| FALLBACK_EDGE_PROFILE.to_owned()),
            )
        } else {
            stored.browser_profiles
        };
        Self {
            devops_url_template: stored.devops_url_template,
            github_url_template: stored.github_url_template,
            admin_center_url_template: stored.admin_center_url_template,
            browser_profiles,
            hotkey: stored.hotkey,
            theme: stored.theme,
            auto_update: stored.auto_update,
        }
    }
}

/// Les deux profils historiques : DevOps et GitHub d'un côté, l'Admin Center de
/// l'autre, chacun revendiquant l'hôte de ses gabarits.
fn default_browser_profiles(
    [devops, github, admin]: [&str; 3],
    primary: String,
    admin_profile: String,
) -> Vec<BrowserProfile> {
    let mut primary_hosts: Vec<String> = Vec::new();
    for host in [devops, github].into_iter().filter_map(template_host) {
        if !primary_hosts.contains(&host) {
            primary_hosts.push(host);
        }
    }
    vec![
        BrowserProfile {
            name: "DevOps et GitHub".to_owned(),
            browser: BrowserKind::Edge,
            profile: primary,
            hosts: primary_hosts,
        },
        BrowserProfile {
            name: "Admin Center".to_owned(),
            browser: BrowserKind::Edge,
            profile: admin_profile,
            hosts: template_host(admin).into_iter().collect(),
        },
    ]
}

/// Hôte fixe d'un gabarit d'URL, s'il n'est pas lui-même construit par un jeton.
fn template_host(template: &str) -> Option<String> {
    host_of(template).filter(|host| !host.contains(['{', '}', '%']))
}

/// Hôte d'une URL, en minuscules.
pub fn host_of(url: &str) -> Option<String> {
    let parsed = tauri::Url::parse(url.trim()).ok()?;
    parsed.host_str().map(str::to_ascii_lowercase)
}

/// Ramène une saisie libre à un nom d'hôte : schéma, chemin, port et joker
/// initial retirés. `None` si ce qui reste n'est pas un nom d'hôte.
///
/// Coller une URL entière dans le champ est le geste naturel : il est accepté.
pub fn normalize_host(raw: &str) -> Option<String> {
    let lower = raw.trim().to_ascii_lowercase();
    let without_scheme = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    let authority = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    let without_port = authority.split(':').next().unwrap_or_default();
    let host = without_port
        .trim_start_matches("*.")
        .trim_matches('.')
        .to_owned();

    let is_valid = !host.is_empty()
        && host.len() <= 253
        && !host.contains("..")
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'));
    is_valid.then_some(host)
}

/// Fabrique une clé technique, unique sur la machine.
///
/// L'horloge donne l'unicité entre exécutions, le compteur celle entre deux
/// créations dans la même nanoseconde. Une dépendance à un générateur d'UUID
/// serait disproportionnée pour un fichier local de quelques centaines de lignes.
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{sequence:x}")
}

/// Dossier de profil Edge présent sur toute installation, utilisé tant qu'aucun
/// profil n'a été détecté sur la machine.
pub const FALLBACK_EDGE_PROFILE: &str = "Default";

/// Nombre maximal de profils de navigation.
const MAX_BROWSER_PROFILES: usize = 32;

/// Nombre maximal d'hôtes par profil de navigation.
const MAX_HOSTS: usize = 64;

/// Apparence par défaut : celle du système.
fn default_theme() -> String {
    "system".to_owned()
}

/// Apparences acceptées.
const THEMES: [&str; 3] = ["system", "light", "dark"];

impl Default for Settings {
    fn default() -> Self {
        let devops_url_template = "https://dev.azure.com/ORGANISATION/{devopsId}".to_owned();
        let github_url_template = "https://github.com/ORGANISATION/{githubId}".to_owned();
        let admin_center_url_template =
            "https://businesscentral.dynamics.com/{tenant}/admin".to_owned();
        let browser_profiles = default_browser_profiles(
            [
                &devops_url_template,
                &github_url_template,
                &admin_center_url_template,
            ],
            FALLBACK_EDGE_PROFILE.to_owned(),
            FALLBACK_EDGE_PROFILE.to_owned(),
        );
        Self {
            devops_url_template,
            github_url_template,
            admin_center_url_template,
            browser_profiles,
            hotkey: "Ctrl+Alt+D".to_owned(),
            theme: default_theme(),
            auto_update: false,
        }
    }
}

impl Settings {
    /// Normalise et valide les réglages fournis par le front.
    pub fn sanitized(self) -> AppResult<Self> {
        if self.browser_profiles.is_empty() {
            return Err(validation("Au moins un profil de navigation est requis."));
        }
        if self.browser_profiles.len() > MAX_BROWSER_PROFILES {
            return Err(validation(format!(
                "Au plus {MAX_BROWSER_PROFILES} profils de navigation sont acceptés."
            )));
        }
        let browser_profiles = self
            .browser_profiles
            .into_iter()
            .map(BrowserProfile::sanitized)
            .collect::<AppResult<Vec<_>>>()?;

        let sanitized = Self {
            devops_url_template: self.devops_url_template.trim().to_owned(),
            github_url_template: self.github_url_template.trim().to_owned(),
            admin_center_url_template: self.admin_center_url_template.trim().to_owned(),
            browser_profiles,
            hotkey: self.hotkey.trim().to_owned(),
            theme: self.theme.trim().to_ascii_lowercase(),
            auto_update: self.auto_update,
        };

        if !THEMES.contains(&sanitized.theme.as_str()) {
            return Err(validation(format!(
                "Apparence inconnue : « {} ».",
                sanitized.theme
            )));
        }

        let templates = [
            ("DevOps", &sanitized.devops_url_template),
            ("GitHub", &sanitized.github_url_template),
            ("Admin Center", &sanitized.admin_center_url_template),
        ];
        for (label, template) in templates {
            if !template.is_empty() && !is_web_url(template) {
                return Err(validation(format!(
                    "L'URL {label} doit commencer par http:// ou https://."
                )));
            }
        }

        if sanitized.hotkey.is_empty() {
            return Err(validation("Le raccourci clavier ne peut pas être vide."));
        }
        Ok(sanitized)
    }

    /// Profil de navigation dans lequel ouvrir une URL : le premier qui revendique
    /// son hôte, à défaut le premier de la liste.
    pub fn profile_for(&self, url: &str) -> Option<&BrowserProfile> {
        host_of(url)
            .and_then(|host| {
                self.browser_profiles
                    .iter()
                    .find(|profile| profile.claims(&host))
            })
            .or_else(|| self.browser_profiles.first())
    }
}

/// Vérifie qu'une chaîne est bien une URL http(s).
///
/// Tout autre schéma est refusé avant transmission à un processus externe :
/// `file:`, `javascript:` ou une URL contenant un saut de ligne n'ont rien à y faire.
pub fn is_web_url(candidate: &str) -> bool {
    let trimmed = candidate.trim();
    let lower = trimmed.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !trimmed.contains(['\n', '\r', '\0'])
}

/// Un dossier de profil Edge ne contient ni séparateur de chemin ni tiret initial,
/// faute de quoi il pourrait être interprété comme une option de ligne de commande.
fn is_safe_profile_directory(profile: &str) -> bool {
    !profile.starts_with('-')
        && profile != "."
        && profile != ".."
        && profile
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'))
}

/// Un nom de profil Firefox est libre (accents compris), mais ne doit ni passer
/// pour une option ni contenir de caractère de contrôle.
fn is_safe_profile_name(profile: &str) -> bool {
    !profile.starts_with('-')
        && profile.chars().count() <= MAX_FIELD_LEN
        && !profile.chars().any(char::is_control)
}

/// Contenu complet du fichier de données.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Database {
    /// Fiches clients.
    #[serde(default)]
    pub customers: Vec<Customer>,
    /// Réglages de l'overlay.
    #[serde(default)]
    pub settings: Settings,
}

impl Database {
    /// Valide un jeu de données complet et refuse les tenants dupliqués.
    /// Utilisé à l'import et au chargement du fichier.
    pub fn sanitized(self) -> AppResult<Self> {
        let mut customers: Vec<Customer> = Vec::with_capacity(self.customers.len());
        for customer in self.customers {
            let customer = customer.sanitized()?;
            if customers
                .iter()
                .any(|existing| existing.has_tenant(&customer.tenant))
            {
                return Err(AppError::DuplicateTenant(customer.tenant));
            }
            customers.push(customer);
        }
        Ok(Self {
            customers,
            settings: self.settings.sanitized()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn customer(tenant: &str, name: &str) -> Customer {
        Customer {
            id: String::new(),
            tenant: tenant.to_owned(),
            name: name.to_owned(),
            keywords: Vec::new(),
            folder_path: String::new(),
            devops_id: String::new(),
            github_id: String::new(),
            workspaces: Vec::new(),
            links: Vec::new(),
        }
    }

    #[test]
    fn sanitized_trims_and_drops_empty_keywords() {
        let mut input = customer("  acme  ", "  Acme SA  ");
        input.keywords = vec!["  erp ".to_owned(), "   ".to_owned(), "bc".to_owned()];

        let result = input.sanitized().expect("la fiche est valide");

        assert_eq!(result.tenant, "acme");
        assert_eq!(result.name, "Acme SA");
        assert_eq!(result.keywords, vec!["erp".to_owned(), "bc".to_owned()]);
    }

    #[test]
    fn sanitized_accepts_an_empty_tenant() {
        let result = customer("   ", "Acme")
            .sanitized()
            .expect("le tenant est facultatif");
        assert!(result.tenant.is_empty());
    }

    #[test]
    fn sanitized_assigns_an_id_when_missing() {
        let result = customer("acme", "Acme")
            .sanitized()
            .expect("la fiche est valide");
        assert!(!result.id.is_empty());
    }

    #[test]
    fn sanitized_keeps_an_existing_id() {
        let mut input = customer("acme", "Acme");
        input.id = "deja-la".to_owned();
        assert_eq!(input.sanitized().expect("valide").id, "deja-la");
    }

    #[test]
    fn two_customers_without_tenant_are_not_duplicates() {
        let database = Database {
            customers: vec![customer("", "Alpha"), customer("", "Beta")],
            settings: Settings::default(),
        };
        assert!(database.sanitized().is_ok());
    }

    #[test]
    fn sanitized_rejects_empty_name() {
        assert!(customer("acme", "  ").sanitized().is_err());
    }

    #[test]
    fn sanitized_rejects_oversized_field() {
        let mut input = customer("acme", "Acme");
        input.devops_id = "x".repeat(MAX_FIELD_LEN + 1);
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_rejects_a_workspace_without_a_name() {
        let mut input = customer("acme", "Acme");
        input.workspaces = vec![Workspace {
            name: "  ".to_owned(),
            path: r"C:\dev".to_owned(),
        }];
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_rejects_a_workspace_without_a_path() {
        let mut input = customer("acme", "Acme");
        input.workspaces = vec![Workspace {
            name: "Extension".to_owned(),
            path: "   ".to_owned(),
        }];
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_rejects_two_workspaces_sharing_a_name() {
        let mut input = customer("acme", "Acme");
        input.workspaces = vec![
            Workspace {
                name: "Extension".to_owned(),
                path: r"C:".to_owned(),
            },
            Workspace {
                name: "EXTENSION".to_owned(),
                path: r"C:".to_owned(),
            },
        ];
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_trims_workspaces() {
        let mut input = customer("acme", "Acme");
        input.workspaces = vec![Workspace {
            name: "  Extension ".to_owned(),
            path: "  C:/dev  ".to_owned(),
        }];

        let result = input.sanitized().expect("la fiche est valide");

        assert_eq!(result.workspaces[0].name, "Extension");
        assert_eq!(result.workspaces[0].path, "C:/dev");
    }

    #[test]
    fn sanitized_rejects_a_link_without_a_name() {
        let mut input = customer("acme", "Acme");
        input.links = vec![CustomLink {
            name: "  ".to_owned(),
            url: "https://exemple.test".to_owned(),
        }];
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_rejects_a_link_that_is_not_http() {
        let mut input = customer("acme", "Acme");
        input.links = vec![CustomLink {
            name: "Extranet".to_owned(),
            url: "file:///C:/secret".to_owned(),
        }];
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_rejects_two_links_sharing_a_name() {
        let mut input = customer("acme", "Acme");
        input.links = vec![
            CustomLink {
                name: "Extranet".to_owned(),
                url: "https://a.test".to_owned(),
            },
            CustomLink {
                name: "EXTRANET".to_owned(),
                url: "https://b.test".to_owned(),
            },
        ];
        assert!(input.sanitized().is_err());
    }

    #[test]
    fn sanitized_trims_links() {
        let mut input = customer("acme", "Acme");
        input.links = vec![CustomLink {
            name: "  Extranet ".to_owned(),
            url: "  https://exemple.test/a  ".to_owned(),
        }];

        let result = input.sanitized().expect("la fiche est valide");

        assert_eq!(result.links[0].name, "Extranet");
        assert_eq!(result.links[0].url, "https://exemple.test/a");
    }

    #[test]
    fn has_tenant_ignores_case_and_spaces() {
        assert!(customer("Acme", "Acme").has_tenant(" acme "));
    }

    #[test]
    fn database_sanitized_rejects_duplicate_tenant() {
        let database = Database {
            customers: vec![customer("acme", "Acme"), customer("ACME", "Acme bis")],
            settings: Settings::default(),
        };
        assert!(database.sanitized().is_err());
    }

    #[test]
    fn is_web_url_rejects_non_http_schemes() {
        assert!(is_web_url("https://example.test/x"));
        assert!(is_web_url("http://example.test"));
        assert!(!is_web_url("file:///C:/windows"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("https://example.test\nautre"));
    }

    fn profile(browser: BrowserKind, profile: &str, hosts: &[&str]) -> BrowserProfile {
        BrowserProfile {
            name: "Profil".to_owned(),
            browser,
            profile: profile.to_owned(),
            hosts: hosts.iter().map(|host| (*host).to_owned()).collect(),
        }
    }

    fn settings_with(profiles: Vec<BrowserProfile>) -> Settings {
        Settings {
            browser_profiles: profiles,
            ..Settings::default()
        }
    }

    #[test]
    fn settings_sanitized_rejects_a_chromium_profile_with_separator() {
        let settings = settings_with(vec![profile(BrowserKind::Edge, r"..\..\Default", &[])]);
        assert!(settings.sanitized().is_err());
    }

    #[test]
    fn settings_sanitized_rejects_a_firefox_profile_looking_like_an_option() {
        let settings = settings_with(vec![profile(BrowserKind::Firefox, "-safe-mode", &[])]);
        assert!(settings.sanitized().is_err());
    }

    #[test]
    fn settings_sanitized_accepts_an_accented_firefox_profile() {
        let settings = settings_with(vec![profile(BrowserKind::Firefox, "Développement", &[])]);
        assert!(settings.sanitized().is_ok());
    }

    #[test]
    fn settings_sanitized_drops_the_profile_of_a_browser_without_profiles() {
        let settings = settings_with(vec![profile(BrowserKind::Opera, "Default", &[])]);
        let sanitized = settings.sanitized().expect("valide");
        assert!(sanitized.browser_profiles[0].profile.is_empty());
    }

    #[test]
    fn settings_sanitized_requires_at_least_one_browser_profile() {
        assert!(settings_with(Vec::new()).sanitized().is_err());
    }

    #[test]
    fn settings_sanitized_normalizes_and_dedupes_hosts() {
        let settings = settings_with(vec![profile(
            BrowserKind::Edge,
            "Default",
            &[
                "https://Dev.Azure.com/org/projet",
                "*.dev.azure.com",
                "  ",
                "github.com:443",
            ],
        )]);
        let sanitized = settings.sanitized().expect("valide");
        assert_eq!(
            sanitized.browser_profiles[0].hosts,
            vec!["dev.azure.com".to_owned(), "github.com".to_owned()]
        );
    }

    #[test]
    fn settings_sanitized_rejects_an_invalid_host() {
        let settings = settings_with(vec![profile(
            BrowserKind::Edge,
            "Default",
            &["exemple .com"],
        )]);
        assert!(settings.sanitized().is_err());
    }

    #[test]
    fn a_profile_claims_its_hosts_and_their_subdomains_only() {
        let claimed = profile(BrowserKind::Edge, "Default", &["dynamics.com"]);
        assert!(claimed.claims("dynamics.com"));
        assert!(claimed.claims("businesscentral.dynamics.com"));
        assert!(!claimed.claims("notdynamics.com"));
    }

    #[test]
    fn profile_for_picks_the_first_matching_profile() {
        let settings = settings_with(vec![
            profile(BrowserKind::Edge, "A", &["github.com"]),
            profile(BrowserKind::Firefox, "B", &["github.com"]),
        ]);
        let chosen = settings.profile_for("https://github.com/org/depot");
        assert_eq!(chosen.map(|profile| profile.profile.as_str()), Some("A"));
    }

    #[test]
    fn profile_for_falls_back_to_the_first_profile() {
        let settings = settings_with(vec![
            profile(BrowserKind::Edge, "A", &["github.com"]),
            profile(BrowserKind::Chrome, "B", &["dev.azure.com"]),
        ]);
        let chosen = settings.profile_for("https://extranet.exemple.test/");
        assert_eq!(chosen.map(|profile| profile.profile.as_str()), Some("A"));
    }

    #[test]
    fn default_settings_route_each_template_to_its_historical_profile() {
        let settings = Settings::default();
        let admin = settings.profile_for("https://businesscentral.dynamics.com/t/admin");
        let github = settings.profile_for("https://github.com/org/depot");
        assert_eq!(
            admin.map(|profile| profile.name.as_str()),
            Some("Admin Center")
        );
        assert_eq!(
            github.map(|profile| profile.name.as_str()),
            Some("DevOps et GitHub")
        );
    }

    #[test]
    fn legacy_edge_settings_become_two_browser_profiles() {
        let legacy = r#"{
            "devopsUrlTemplate": "https://dev.azure.com/org/{devopsId}",
            "githubUrlTemplate": "https://github.com/org/{githubId}",
            "adminCenterUrlTemplate": "https://businesscentral.dynamics.com/{tenant}/admin",
            "edgeProfilePrimary": "Profile 3",
            "edgeProfileAdmin": "Profile 1",
            "hotkey": "Ctrl+Alt+D"
        }"#;
        let settings: Settings = serde_json::from_str(legacy).expect("réglages hérités");

        assert_eq!(settings.browser_profiles.len(), 2);
        assert_eq!(settings.browser_profiles[0].profile, "Profile 3");
        assert_eq!(
            settings.browser_profiles[0].hosts,
            vec!["dev.azure.com".to_owned(), "github.com".to_owned()]
        );
        assert_eq!(settings.browser_profiles[1].profile, "Profile 1");
        assert_eq!(
            settings.browser_profiles[1].hosts,
            vec!["businesscentral.dynamics.com".to_owned()]
        );
    }

    #[test]
    fn automatic_updates_are_off_by_default_and_for_older_files() {
        assert!(!Settings::default().auto_update);
        let older = r#"{
            "devopsUrlTemplate": "https://dev.azure.com/org/{devopsId}",
            "githubUrlTemplate": "https://github.com/org/{githubId}",
            "adminCenterUrlTemplate": "https://businesscentral.dynamics.com/{tenant}/admin",
            "hotkey": "Ctrl+Alt+D"
        }"#;
        let settings: Settings = serde_json::from_str(older).expect("réglages anciens");
        assert!(!settings.auto_update);
    }

    #[test]
    fn a_host_built_from_a_token_is_not_claimed() {
        assert_eq!(template_host("https://{tenant}.exemple.com/admin"), None);
    }

    #[test]
    fn serialized_settings_no_longer_carry_edge_fields() {
        let json = serde_json::to_string(&Settings::default()).expect("sérialisation");
        assert!(!json.contains("edgeProfile"));
        let back: Settings = serde_json::from_str(&json).expect("relecture");
        assert_eq!(back, Settings::default());
    }

    #[test]
    fn settings_sanitized_rejects_an_unknown_theme() {
        let settings = Settings {
            theme: "neon".to_owned(),
            ..Settings::default()
        };
        assert!(settings.sanitized().is_err());
    }

    #[test]
    fn settings_sanitized_accepts_the_three_themes() {
        for theme in ["system", "Light", " DARK "] {
            let settings = Settings {
                theme: theme.to_owned(),
                ..Settings::default()
            };
            assert!(settings.sanitized().is_ok(), "apparence refusée : {theme}");
        }
    }

    #[test]
    fn settings_sanitized_rejects_non_http_template() {
        let settings = Settings {
            devops_url_template: "file:///C:/".to_owned(),
            ..Settings::default()
        };
        assert!(settings.sanitized().is_err());
    }
}
