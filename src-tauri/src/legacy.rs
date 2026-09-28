//! Reprise d'une installation antérieure au renommage « Customers Overlay » →
//! « Bookmark Overlay ».
//!
//! L'identifiant de l'application nomme son dossier de configuration : le changer
//! fait démarrer la nouvelle version sur un dossier vide. Ses fichiers sont donc
//! recopiés depuis l'ancien dossier, qui est laissé intact en guise de sauvegarde,
//! et l'ancienne entrée de démarrage automatique est remplacée par la nouvelle.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

/// Dossier de configuration de l'ancienne identité, voisin du nouveau.
const LEGACY_CONFIG_DIR: &str = "fr.capvision.customers-overlay";

/// Nom de l'ancienne entrée sous `HKCU\…\CurrentVersion\Run`.
const LEGACY_AUTOSTART_ENTRY: &str = "Customers Overlay";

/// Clé de registre des programmes lancés à l'ouverture de session.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

/// Recopie les fichiers de l'ancien dossier de configuration si le nouveau ne
/// contient pas encore de fichier de données. Retourne vrai si une reprise a eu lieu.
pub fn adopt_legacy_config(config_dir: &Path, data_file: &str) -> io::Result<bool> {
    let Some(legacy) = config_dir
        .parent()
        .map(|parent| parent.join(LEGACY_CONFIG_DIR))
    else {
        return Ok(false);
    };
    if config_dir.join(data_file).exists() || !legacy.join(data_file).is_file() {
        return Ok(false);
    }

    fs::create_dir_all(config_dir)?;
    for entry in fs::read_dir(&legacy)? {
        let entry = entry?;
        // Seuls les fichiers de premier niveau comptent : données, sauvegardes.
        // Le reste (cache de la webview…) est propre à l'ancienne identité.
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), config_dir.join(entry.file_name()))?;
        }
    }
    Ok(true)
}

/// Vrai si l'ancienne entrée de démarrage automatique existe ; elle est alors
/// supprimée, pour que l'ancien exécutable ne soit plus lancé à l'ouverture de
/// session.
pub fn take_legacy_autostart() -> bool {
    let exists = reg(&["query", RUN_KEY, "/v", LEGACY_AUTOSTART_ENTRY]);
    if exists {
        // Une suppression refusée laisse une entrée orpheline, sans autre effet que
        // de relancer l'ancienne version : rien qui justifie de bloquer le démarrage.
        let _ = reg(&["delete", RUN_KEY, "/v", LEGACY_AUTOSTART_ENTRY, "/f"]);
    }
    exists
}

/// Exécute `reg.exe` avec des arguments fixes et indique s'il a réussi.
fn reg(arguments: &[&str]) -> bool {
    let mut command = Command::new("reg.exe");
    command.args(arguments);
    crate::launcher::apply_no_window(&mut command);
    command.output().is_ok_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(label: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("overlay-legacy-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("dossier de test");
        root
    }

    #[test]
    fn legacy_files_are_copied_into_the_new_folder() {
        let root = temporary_root("copy");
        let legacy = root.join(LEGACY_CONFIG_DIR);
        fs::create_dir_all(legacy.join("EBWebView")).expect("dossier");
        fs::write(legacy.join("data"), "chiffré").expect("écriture");
        fs::write(legacy.join("data.backup-1.json"), "{}").expect("écriture");
        let current = root.join("fr.capvision.bookmark-overlay");

        assert!(adopt_legacy_config(&current, "data").expect("reprise"));

        assert_eq!(
            fs::read_to_string(current.join("data")).expect("lecture"),
            "chiffré"
        );
        assert!(current.join("data.backup-1.json").is_file());
        assert!(!current.join("EBWebView").exists());
        assert!(
            legacy.join("data").is_file(),
            "l'ancien dossier reste intact"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn existing_data_is_never_overwritten() {
        let root = temporary_root("keep");
        let legacy = root.join(LEGACY_CONFIG_DIR);
        let current = root.join("fr.capvision.bookmark-overlay");
        fs::create_dir_all(&legacy).expect("dossier");
        fs::create_dir_all(&current).expect("dossier");
        fs::write(legacy.join("data"), "ancien").expect("écriture");
        fs::write(current.join("data"), "actuel").expect("écriture");

        assert!(!adopt_legacy_config(&current, "data").expect("reprise"));
        assert_eq!(
            fs::read_to_string(current.join("data")).expect("lecture"),
            "actuel"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn nothing_happens_without_a_legacy_folder() {
        let root = temporary_root("none");
        let current = root.join("fr.capvision.bookmark-overlay");
        assert!(!adopt_legacy_config(&current, "data").expect("reprise"));
        assert!(!current.exists());
        let _ = fs::remove_dir_all(root);
    }
}
