//! Ouverture des cibles externes : navigateur (par profil de navigation), Visual
//! Studio Code et l'explorateur Windows.
//!
//! Aucune chaîne fournie par le front n'atteint un processus externe sans avoir été
//! validée ici : les URL doivent être en http(s), les valeurs injectées dans les
//! gabarits sont encodées, et les arguments sont passés un par un (jamais via un shell),
//! ce qui exclut toute injection de commande.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{validation, AppError, AppResult};
use crate::model::{is_web_url, Customer, Settings};

/// Empêche l'apparition d'une fenêtre de console lors du lancement d'un processus.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Emplacements d'installation « pour tous les utilisateurs » de Visual Studio Code.
///
/// L'installation par défaut, elle, est propre à l'utilisateur et se trouve sous
/// `%LOCALAPPDATA%` ; elle est construite à part, ce chemin n'étant pas constant.
#[cfg(windows)]
const VSCODE_MACHINE_LOCATIONS: [&str; 4] = [
    r"C:\Program Files\Microsoft VS Code\Code.exe",
    r"C:\Program Files (x86)\Microsoft VS Code\Code.exe",
    r"C:\Program Files\Microsoft VS Code Insiders\Code - Insiders.exe",
    r"C:\Program Files (x86)\Microsoft VS Code Insiders\Code - Insiders.exe",
];

/// Cible que l'overlay sait ouvrir pour un client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkKind {
    /// Page du projet Azure DevOps.
    Devops,
    /// Page du dépôt GitHub.
    Github,
    /// Admin Center Business Central.
    AdminCenter,
    /// Dossier de travail local.
    Folder,
}

/// Construit l'URL d'une cible web pour un client donné, ou `None` si la fiche
/// ne porte pas l'identifiant nécessaire.
pub fn build_url(customer: &Customer, settings: &Settings, kind: LinkKind) -> Option<String> {
    let (template, required) = match kind {
        LinkKind::Devops => (&settings.devops_url_template, &customer.devops_id),
        LinkKind::Github => (&settings.github_url_template, &customer.github_id),
        LinkKind::AdminCenter => (&settings.admin_center_url_template, &customer.tenant),
        LinkKind::Folder => return None,
    };
    if template.trim().is_empty() || required.trim().is_empty() {
        return None;
    }

    let url = template
        .replace("{tenant}", &encode_segment(&customer.tenant))
        .replace("{devopsId}", &encode_segment(&customer.devops_id))
        .replace("{githubId}", &encode_segment(&customer.github_id));

    is_web_url(&url).then_some(url)
}

/// Ouvre une cible pour un client, dans le profil de navigation qui revendique
/// l'hôte de son URL.
pub fn open(customer: &Customer, settings: &Settings, kind: LinkKind) -> AppResult<()> {
    match kind {
        LinkKind::Folder => open_folder(&customer.folder_path),
        LinkKind::Devops | LinkKind::Github | LinkKind::AdminCenter => {
            let url = build_url(customer, settings, kind).ok_or_else(|| {
                validation("Cette fiche n'a pas l'identifiant requis pour ouvrir ce lien.")
            })?;
            crate::browser::open_web(&url, settings)
        }
    }
}

/// Extension des fichiers d'espace de travail de Visual Studio Code.
pub const WORKSPACE_EXTENSION: &str = "code-workspace";

/// Ouvre un dossier ou un fichier `.code-workspace` dans Visual Studio Code.
///
/// L'exécutable est invoqué directement plutôt que par le script `code.cmd` du
/// dossier `bin` : ce dernier impose de passer par un interpréteur de commandes,
/// ce qu'on s'interdit pour toute donnée venant du front.
pub fn open_in_vscode(target_path: &str) -> AppResult<()> {
    let path = existing_vscode_target(target_path)?;
    let executable = vscode_executable().ok_or_else(|| {
        AppError::Launch("Visual Studio Code est introuvable sur cette machine.".to_owned())
    })?;

    let mut command = Command::new(&executable);
    // `--new-window` évite qu'une instance déjà lancée se contente de repasser au
    // premier plan sans ouvrir la cible demandée.
    command.arg("--new-window").arg(path);

    // Ces deux variables sont posées par le terminal intégré de Visual Studio Code
    // et héritées par tout ce qu'on y lance, overlay compris. `ELECTRON_RUN_AS_NODE`
    // fait démarrer `Code.exe` en simple interpréteur Node : l'éditeur ne s'ouvre
    // pas, et seule la fenêtre déjà présente revient au premier plan. Le script
    // `code.cmd` fourni par l'éditeur les neutralise de la même façon.
    command.env_remove("ELECTRON_RUN_AS_NODE");
    command.env_remove("VSCODE_DEV");

    apply_no_window(&mut command);
    command.spawn().map_err(|error| {
        AppError::Launch(format!(
            "Impossible de lancer {} : {error}",
            executable.display()
        ))
    })?;
    Ok(())
}

/// Valide une cible Visual Studio Code : un dossier, ou un fichier `.code-workspace`.
fn existing_vscode_target(target_path: &str) -> AppResult<&Path> {
    let trimmed = target_path.trim();
    if trimmed.is_empty() {
        return Err(validation("Aucun dossier n'est renseigné."));
    }
    let path = Path::new(trimmed);

    if path.is_dir() {
        return Ok(path);
    }
    if path.is_file() {
        let is_workspace = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(WORKSPACE_EXTENSION));
        if is_workspace {
            return Ok(path);
        }
        return Err(validation(format!(
            "« {trimmed} » n'est ni un dossier ni un fichier .{WORKSPACE_EXTENSION}."
        )));
    }

    Err(AppError::Launch(format!("« {trimmed} » est introuvable.")))
}

/// Localise `Code.exe` : installation utilisateur, installations machine, puis `PATH`.
fn vscode_executable() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let user_install = Path::new(&local_app_data)
                .join("Programs")
                .join("Microsoft VS Code")
                .join("Code.exe");
            if user_install.is_file() {
                return Some(user_install);
            }
        }
        if let Some(found) = VSCODE_MACHINE_LOCATIONS
            .iter()
            .map(PathBuf::from)
            .find(|candidate| candidate.is_file())
        {
            return Some(found);
        }
        let paths = std::env::var_os("PATH")?;
        std::env::split_paths(&paths)
            .map(|directory| directory.join("Code.exe"))
            .find(|candidate| candidate.is_file())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Valide qu'un chemin est renseigné et pointe bien sur un dossier existant.
fn existing_directory(folder_path: &str) -> AppResult<&Path> {
    let trimmed = folder_path.trim();
    if trimmed.is_empty() {
        return Err(validation("Aucun dossier n'est renseigné."));
    }
    let path = Path::new(trimmed);
    if !path.is_dir() {
        return Err(AppError::Launch(format!(
            "Le dossier « {trimmed} » est introuvable."
        )));
    }
    Ok(path)
}

/// Ouvre un dossier dans l'explorateur Windows.
pub fn open_folder(folder_path: &str) -> AppResult<()> {
    let path = existing_directory(folder_path)?;
    reveal_in_explorer(&[path.as_os_str().to_owned()])
}

/// Ouvre l'explorateur sur le dossier contenant le fichier et l'y sélectionne.
pub fn reveal_file(file_path: &Path) -> AppResult<()> {
    if file_path.is_file() {
        let mut argument = std::ffi::OsString::from("/select,");
        argument.push(file_path.as_os_str());
        return reveal_in_explorer(&[argument]);
    }
    match file_path.parent() {
        Some(parent) if parent.is_dir() => reveal_in_explorer(&[parent.as_os_str().to_owned()]),
        _ => Err(AppError::Launch(
            "Le dossier de données n'existe pas encore.".to_owned(),
        )),
    }
}

/// Démarre un processus détaché, sans fenêtre de console.
pub(crate) fn spawn(executable: &Path, arguments: &[String]) -> AppResult<()> {
    let mut command = Command::new(executable);
    command.args(arguments);
    apply_no_window(&mut command);
    command.spawn().map_err(|error| {
        AppError::Launch(format!(
            "Impossible de lancer {} : {error}",
            executable.display()
        ))
    })?;
    Ok(())
}

/// Ouvre l'explorateur avec les arguments fournis.
///
/// `explorer.exe` renvoie fréquemment un code de sortie non nul même en cas de
/// succès : on ne teste donc que le démarrage du processus.
fn reveal_in_explorer(arguments: &[std::ffi::OsString]) -> AppResult<()> {
    let mut command = Command::new("explorer.exe");
    command.args(arguments);
    apply_no_window(&mut command);
    command.spawn().map_err(|error| {
        AppError::Launch(format!("Impossible d'ouvrir l'explorateur : {error}"))
    })?;
    Ok(())
}

/// Dernier recours quand le navigateur choisi est introuvable : celui par défaut.
///
/// `rundll32 url.dll,FileProtocolHandler` transmet l'URL telle quelle, sans passer
/// par un interpréteur de commandes.
pub(crate) fn open_with_default_browser(url: &str) -> AppResult<()> {
    let mut command = Command::new("rundll32.exe");
    command.args(["url.dll,FileProtocolHandler", url]);
    apply_no_window(&mut command);
    command.spawn().map_err(|error| {
        AppError::Launch(format!("Aucun navigateur n'a pu être lancé : {error}"))
    })?;
    Ok(())
}

/// Ajoute l'indicateur Windows supprimant la console du processus fils.
pub(crate) fn apply_no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

/// Encode une valeur destinée à être injectée dans une URL.
///
/// Seuls les caractères non réservés de la RFC 3986 sont conservés tels quels ;
/// tout le reste est encodé, ce qui neutralise une valeur contenant `?`, `#` ou un espace.
fn encode_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.trim().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn customer() -> Customer {
        Customer {
            id: "test".to_owned(),
            tenant: "acme.onmicrosoft.com".to_owned(),
            name: "Acme".to_owned(),
            keywords: Vec::new(),
            folder_path: String::new(),
            devops_id: "Projet Acme".to_owned(),
            github_id: "acme-bc".to_owned(),
            workspaces: Vec::new(),
            links: Vec::new(),
        }
    }

    #[test]
    fn build_url_fills_every_token() {
        let settings = Settings {
            devops_url_template: "https://dev.azure.com/org/{devopsId}".to_owned(),
            ..Settings::default()
        };
        let url = build_url(&customer(), &settings, LinkKind::Devops);
        assert_eq!(
            url.as_deref(),
            Some("https://dev.azure.com/org/Projet%20Acme")
        );
    }

    #[test]
    fn build_url_uses_the_tenant_for_the_admin_center() {
        let url = build_url(&customer(), &Settings::default(), LinkKind::AdminCenter);
        assert_eq!(
            url.as_deref(),
            Some("https://businesscentral.dynamics.com/acme.onmicrosoft.com/admin")
        );
    }

    #[test]
    fn build_url_returns_none_when_the_identifier_is_missing() {
        let mut without_github = customer();
        without_github.github_id = String::new();
        assert!(build_url(&without_github, &Settings::default(), LinkKind::Github).is_none());
    }

    #[test]
    fn build_url_returns_none_for_the_folder_kind() {
        assert!(build_url(&customer(), &Settings::default(), LinkKind::Folder).is_none());
    }

    #[test]
    fn encode_segment_neutralizes_url_metacharacters() {
        assert_eq!(encode_segment("a b"), "a%20b");
        assert_eq!(encode_segment("a?b#c"), "a%3Fb%23c");
        assert_eq!(
            encode_segment("acme.onmicrosoft.com"),
            "acme.onmicrosoft.com"
        );
    }

    #[test]
    fn open_folder_refuses_an_empty_path() {
        assert!(open_folder("   ").is_err());
    }

    #[test]
    fn a_directory_is_a_valid_vscode_target() {
        let directory = std::env::temp_dir().to_string_lossy().into_owned();
        assert!(existing_vscode_target(&directory).is_ok());
    }

    #[test]
    fn a_code_workspace_file_is_a_valid_vscode_target() {
        let path = std::env::temp_dir().join("overlay-test.code-workspace");
        std::fs::write(&path, "{}").expect("écriture du fichier de test");
        let as_text = path.to_string_lossy().into_owned();

        let result = existing_vscode_target(&as_text);

        assert!(result.is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn another_file_is_refused_as_a_vscode_target() {
        let path = std::env::temp_dir().join("overlay-test.txt");
        std::fs::write(&path, "x").expect("écriture du fichier de test");
        let as_text = path.to_string_lossy().into_owned();

        assert!(existing_vscode_target(&as_text).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_missing_path_is_refused_as_a_vscode_target() {
        assert!(existing_vscode_target(r"Z:ucun\chemin\ici").is_err());
    }
}
