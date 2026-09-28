//! Mise à jour automatique depuis les releases GitHub du dépôt.
//!
//! Le dépôt est celui qui a produit le binaire : la variable `GITHUB_REPOSITORY`,
//! posée par GitHub Actions, est capturée à la compilation. Un binaire construit
//! en local n'en a pas, et ne cherche donc jamais de mise à jour.
//!
//! Deux livrables coexistent, et chacun se remplace par le sien :
//!
//! - **installateur** : le nouveau `setup.exe` est lancé en mode silencieux, il
//!   ferme l'application, la remplace puis la relance (`/R`) ;
//! - **portable** : le nouvel exécutable est téléchargé à côté de l'actuel, qui
//!   est renommé (Windows l'autorise pour un exécutable en cours) pour lui laisser
//!   la place ; le nouveau est lancé et attend la fin de l'ancien.

use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::error::{validation, AppError, AppResult};
use crate::launcher;

/// Dépôt `propriétaire/nom` dont les releases sont suivies.
const REPOSITORY: Option<&str> = option_env!("GITHUB_REPOSITORY");

/// Version du binaire en cours d'exécution.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Événement émis quand une mise à jour est trouvée.
pub const EVENT_AVAILABLE: &str = "update://available";

/// Événement émis pendant le téléchargement.
pub const EVENT_PROGRESS: &str = "update://progress";

/// Argument passé au nouvel exécutable portable : l'identifiant du processus à attendre.
const WAIT_ARGUMENT: &str = "--wait-pid=";

/// Délai avant la première vérification : le démarrage de Windows a mieux à faire.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);

/// Intervalle entre deux vérifications.
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Attente maximale de la fin de l'ancienne instance, après une mise à jour portable.
const WAIT_FOR_PREVIOUS: Duration = Duration::from_secs(20);

/// Taille maximale acceptée pour la réponse de l'API GitHub.
const MAX_API_BYTES: u64 = 1024 * 1024;

/// Taille maximale acceptée pour un livrable téléchargé.
const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;

/// Dossier temporaire où l'installateur est téléchargé.
const TEMP_FOLDER: &str = "bookmark-overlay-update";

/// Livrable en cours d'exécution, qui détermine celui à télécharger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Flavor {
    /// Installé par `setup.exe`.
    Installer,
    /// Exécutable copié tel quel.
    Portable,
}

impl Flavor {
    /// Suffixe du nom de fichier du livrable dans une release.
    fn asset_suffix(self) -> &'static str {
        match self {
            Self::Installer => "-setup.exe",
            Self::Portable => "-portable.exe",
        }
    }
}

/// Une mise à jour disponible, telle que présentée au front.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// Version proposée, sans le `v` du tag.
    pub version: String,
    /// Date de publication (ISO 8601).
    pub published_at: String,
    /// Taille du livrable, en octets.
    pub size: u64,
    /// Notes de version, en texte brut.
    pub notes: String,
    /// Livrable qui sera installé.
    pub flavor: Flavor,
    /// Nom du fichier du livrable.
    #[serde(skip)]
    asset_name: String,
    /// URL de téléchargement.
    #[serde(skip)]
    download_url: String,
    /// Empreinte `sha256:<hex>` publiée par GitHub, quand elle est fournie.
    #[serde(skip)]
    digest: Option<String>,
}

/// Avancement d'un téléchargement.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: u64,
}

/// État partagé : dernière mise à jour trouvée, installation en cours.
#[derive(Debug, Default)]
pub struct UpdateState {
    available: Mutex<Option<UpdateInfo>>,
    installing: AtomicBool,
}

impl UpdateState {
    /// Dernière mise à jour trouvée, s'il y en a une.
    pub fn available(&self) -> AppResult<Option<UpdateInfo>> {
        let guard = self.available.lock().map_err(|_| AppError::PoisonedState)?;
        Ok(guard.clone())
    }

    fn set_available(&self, info: UpdateInfo) -> AppResult<()> {
        let mut guard = self.available.lock().map_err(|_| AppError::PoisonedState)?;
        *guard = Some(info);
        Ok(())
    }
}

/// Réponse de `GET /repos/{dépôt}/releases/latest`, réduite à l'utile.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// Un fichier attaché à une release.
#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

/// Livrable en cours d'exécution : l'installateur dépose `uninstall.exe` à côté
/// de l'exécutable, la version portable n'a rien à côté d'elle.
pub fn current_flavor() -> Flavor {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("uninstall.exe")))
        .is_some_and(|uninstaller| uninstaller.is_file());
    if installed {
        Flavor::Installer
    } else {
        Flavor::Portable
    }
}

/// Termine une mise à jour portable au démarrage de la nouvelle version : attend
/// la fin de l'ancienne instance, puis supprime son exécutable renommé.
///
/// À appeler avant tout le reste : tant que l'ancienne instance vit, le verrou
/// d'instance unique renverrait la nouvelle aussitôt lancée.
pub fn finish_pending_update() {
    let pid = std::env::args()
        .find_map(|argument| argument.strip_prefix(WAIT_ARGUMENT).map(str::to_owned))
        .and_then(|pid| pid.parse::<u32>().ok());
    if let Some(pid) = pid {
        wait_for_process(pid, WAIT_FOR_PREVIOUS);
    }

    // Nettoyage opportuniste : rien de grave si l'un de ces fichiers résiste, il
    // sera retenté au prochain démarrage.
    if let Ok(exe) = std::env::current_exe() {
        let _ = remove_if_present(&sibling(&exe, "old"));
        let _ = remove_if_present(&sibling(&exe, "download"));
    }
    let _ = fs::remove_dir_all(std::env::temp_dir().join(TEMP_FOLDER));
}

/// Lance la vérification périodique en tâche de fond.
pub fn start_background_checks<R: Runtime>(app: AppHandle<R>) {
    let Some(repository) = REPOSITORY else {
        return;
    };
    let spawned = std::thread::Builder::new()
        .name("update-check".to_owned())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK_DELAY);
            loop {
                // Hors ligne, quota de l'API atteint : on retentera au cycle suivant.
                if let Ok(Some(info)) = check(repository) {
                    let state = app.state::<UpdateState>();
                    if state.set_available(info.clone()).is_ok() {
                        // Le front n'est peut-être pas encore prêt : il relira l'état
                        // au prochain amorçage.
                        let _ = app.emit(EVENT_AVAILABLE, &info);
                    }
                }
                std::thread::sleep(CHECK_INTERVAL);
            }
        });
    // Sans ce thread, l'application fonctionne : elle ne se met simplement pas à jour.
    let _ = spawned;
}

/// Interroge GitHub sur la dernière release publiée.
fn check(repository: &str) -> AppResult<Option<UpdateInfo>> {
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let mut response = agent(Some(Duration::from_secs(30)))
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()?;
    let release: Release = response
        .body_mut()
        .with_config()
        .limit(MAX_API_BYTES)
        .read_json()?;
    Ok(select_update(
        release,
        repository,
        CURRENT_VERSION,
        current_flavor(),
    ))
}

/// Retient la release si elle est plus récente et porte le livrable attendu.
fn select_update(
    release: Release,
    repository: &str,
    current: &str,
    flavor: Flavor,
) -> Option<UpdateInfo> {
    let version = release.tag_name.trim().trim_start_matches('v').to_owned();
    if !is_newer(&version, current) {
        return None;
    }
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.ends_with(flavor.asset_suffix()))?;

    // Le nom sert de nom de fichier local, l'URL de source : ni l'un ni l'autre ne
    // doit pouvoir sortir de ce qui est attendu.
    let expected_prefix = format!("https://github.com/{repository}/releases/download/");
    let safe_name = asset
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !safe_name || !asset.browser_download_url.starts_with(&expected_prefix) {
        return None;
    }

    Some(UpdateInfo {
        version,
        published_at: release.published_at.unwrap_or_default(),
        size: asset.size,
        notes: release.body.unwrap_or_default(),
        flavor,
        asset_name: asset.name,
        download_url: asset.browser_download_url,
        digest: asset.digest,
    })
}

/// Compare deux versions `majeur.mineur.correctif` ; toute étiquette de
/// pré-version est ignorée.
fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

/// Lit `1.2.3` (ou `1.2.3-beta`) en triplet comparable.
fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next()??;
    let patch = parts.next()??;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Télécharge, vérifie et installe la mise à jour retenue, puis quitte
/// l'application pour laisser la nouvelle version démarrer.
pub fn install<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let state = app.state::<UpdateState>();
    let info = state
        .available()?
        .ok_or_else(|| validation("Aucune mise à jour n'est disponible."))?;
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err(validation("Une mise à jour est déjà en cours."));
    }

    let outcome = download_and_apply(app, &info);
    if outcome.is_err() {
        state.installing.store(false, Ordering::SeqCst);
    }
    outcome
}

/// Enchaîne téléchargement et remplacement selon le livrable.
fn download_and_apply<R: Runtime>(app: &AppHandle<R>, info: &UpdateInfo) -> AppResult<()> {
    let current = std::env::current_exe()?;
    match info.flavor {
        Flavor::Installer => {
            let folder = std::env::temp_dir().join(TEMP_FOLDER);
            fs::create_dir_all(&folder)?;
            let installer = folder.join(&info.asset_name);
            download(app, info, &installer)?;
            // `/S` : aucune fenêtre ni question ; `/R` : relance de l'application
            // une fois installée ; `/UPDATE` : pas de raccourcis recréés.
            launcher::spawn(
                &installer,
                &["/S".to_owned(), "/R".to_owned(), "/UPDATE".to_owned()],
            )?;
        }
        Flavor::Portable => {
            // Téléchargé à côté de l'exécutable : le renommage final reste sur le
            // même volume, donc atomique.
            let downloaded = sibling(&current, "download");
            download(app, info, &downloaded)?;
            swap_executable(&current, &downloaded)?;
            launcher::spawn(
                &current,
                &[format!("{WAIT_ARGUMENT}{}", std::process::id())],
            )?;
        }
    }
    app.exit(0);
    Ok(())
}

/// Télécharge le livrable dans `target`, en vérifiant taille et empreinte.
/// Un fichier partiel ou non conforme est supprimé.
fn download<R: Runtime>(app: &AppHandle<R>, info: &UpdateInfo, target: &Path) -> AppResult<()> {
    let outcome = write_download(app, info, target);
    if outcome.is_err() {
        // Le fichier partiel n'a aucune valeur ; le laisser n'aurait aucun effet.
        let _ = remove_if_present(target);
    }
    outcome
}

fn write_download<R: Runtime>(
    app: &AppHandle<R>,
    info: &UpdateInfo,
    target: &Path,
) -> AppResult<()> {
    let mut response = agent(None).get(&info.download_url).call()?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_BYTES)
        .reader();
    let mut file = File::create(target)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_percent: u64 = u64::MAX;

    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(AppError::Io(error)),
        };
        let chunk = &buffer[..read];
        file.write_all(chunk)?;
        hasher.update(chunk);
        downloaded += chunk.len() as u64;

        // Un événement par point de pourcentage suffit à animer la barre.
        let percent = downloaded.saturating_mul(100) / info.size.max(1);
        if percent != last_percent {
            last_percent = percent;
            // Une barre de progression figée n'empêche pas la mise à jour.
            let _ = app.emit(
                EVENT_PROGRESS,
                Progress {
                    downloaded,
                    total: info.size,
                },
            );
        }
    }
    file.sync_all()?;

    if downloaded != info.size {
        return Err(AppError::Update(format!(
            "téléchargement incomplet ({downloaded} octets sur {}).",
            info.size
        )));
    }
    if let Some(expected) = info
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
    {
        let actual = to_hex(&hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(AppError::Update(
                "le fichier téléchargé ne correspond pas à l'empreinte publiée.".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Met le nouvel exécutable à la place de l'actuel, qui est renommé en `.old`.
fn swap_executable(current: &Path, downloaded: &Path) -> AppResult<()> {
    let previous = sibling(current, "old");
    remove_if_present(&previous)?;
    fs::rename(current, &previous)?;
    if let Err(error) = fs::rename(downloaded, current) {
        // Remettre l'ancien en place : sans cela plus aucun exécutable ne porterait
        // le nom attendu par le démarrage automatique.
        let _ = fs::rename(&previous, current);
        return Err(AppError::Io(error));
    }
    Ok(())
}

/// Chemin voisin de l'exécutable : `bookmark-overlay.exe.<extension>`.
fn sibling(exe: &Path, extension: &str) -> PathBuf {
    let mut name = exe
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_default();
    name.push(".");
    name.push(extension);
    exe.with_file_name(name)
}

/// Supprime un fichier, sans erreur s'il n'existe pas.
fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Client HTTP : TLS du système (magasin de certificats Windows, proxys
/// d'entreprise compris) et délai global facultatif.
fn agent(timeout: Option<Duration>) -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .https_only(true)
        .user_agent(format!("bookmark-overlay/{CURRENT_VERSION}"))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_global(timeout)
        .build()
        .into()
}

/// Représentation hexadécimale d'une empreinte.
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Attend la fin d'un processus, au plus `timeout`.
#[cfg(windows)]
fn wait_for_process(pid: u32, timeout: Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    let milliseconds = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    // SAFETY: `OpenProcess` renvoie soit un handle valide, soit un handle nul que
    // l'on écarte (processus déjà terminé) ; le handle obtenu n'est utilisé que pour
    // l'attente, puis refermé. Aucun pointeur fourni par nous n'est déréférencé.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return;
        }
        WaitForSingleObject(handle, milliseconds);
        CloseHandle(handle);
    }
}

#[cfg(not(windows))]
fn wait_for_process(_pid: u32, _timeout: Duration) {}

#[cfg(test)]
mod tests {
    use super::*;

    const REPO: &str = "capvision/customers";

    fn release(tag: &str, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.to_owned(),
            published_at: Some("2026-09-28T10:00:00Z".to_owned()),
            body: Some("notes".to_owned()),
            assets: assets
                .iter()
                .map(|name| Asset {
                    name: (*name).to_owned(),
                    size: 1234,
                    browser_download_url: format!(
                        "https://github.com/{REPO}/releases/download/{tag}/{name}"
                    ),
                    digest: None,
                })
                .collect(),
        }
    }

    const BOTH: [&str; 2] = [
        "BookmarkOverlay-0.2.0-setup.exe",
        "BookmarkOverlay-0.2.0-portable.exe",
    ];

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0-beta", "0.1.0"));
        assert!(!is_newer("n'importe quoi", "0.1.0"));
    }

    #[test]
    fn a_newer_release_offers_the_matching_asset() {
        let installer = select_update(release("v0.2.0", &BOTH), REPO, "0.1.0", Flavor::Installer)
            .expect("mise à jour attendue");
        assert_eq!(installer.version, "0.2.0");
        assert_eq!(installer.asset_name, "BookmarkOverlay-0.2.0-setup.exe");

        let portable = select_update(release("v0.2.0", &BOTH), REPO, "0.1.0", Flavor::Portable)
            .expect("mise à jour attendue");
        assert_eq!(portable.asset_name, "BookmarkOverlay-0.2.0-portable.exe");
    }

    #[test]
    fn the_same_version_is_not_an_update() {
        assert!(select_update(release("v0.1.0", &BOTH), REPO, "0.1.0", Flavor::Portable).is_none());
    }

    #[test]
    fn a_release_without_the_matching_asset_is_ignored() {
        let only_setup = ["BookmarkOverlay-0.2.0-setup.exe"];
        assert!(select_update(
            release("v0.2.0", &only_setup),
            REPO,
            "0.1.0",
            Flavor::Portable
        )
        .is_none());
    }

    #[test]
    fn an_asset_hosted_elsewhere_is_refused() {
        let mut foreign = release("v0.2.0", &BOTH);
        for asset in &mut foreign.assets {
            asset.browser_download_url = "https://exemple.test/piege.exe".to_owned();
        }
        assert!(select_update(foreign, REPO, "0.1.0", Flavor::Installer).is_none());
    }

    #[test]
    fn an_asset_name_escaping_the_folder_is_refused() {
        let hostile = ["..\\..\\x-setup.exe"];
        assert!(select_update(
            release("v0.2.0", &hostile),
            REPO,
            "0.1.0",
            Flavor::Installer
        )
        .is_none());
    }

    #[test]
    fn sibling_appends_an_extension_to_the_executable_name() {
        let exe = Path::new(r"C:\outils\bookmark-overlay.exe");
        assert_eq!(
            sibling(exe, "old"),
            PathBuf::from(r"C:\outils\bookmark-overlay.exe.old")
        );
    }

    #[test]
    fn swapping_keeps_the_previous_executable_aside() {
        let folder = std::env::temp_dir().join(format!("overlay-swap-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("dossier de test");
        let current = folder.join("app.exe");
        let downloaded = sibling(&current, "download");
        fs::write(&current, "ancienne").expect("écriture");
        fs::write(&downloaded, "nouvelle").expect("écriture");

        swap_executable(&current, &downloaded).expect("échange");

        assert_eq!(fs::read_to_string(&current).expect("lecture"), "nouvelle");
        assert_eq!(
            fs::read_to_string(sibling(&current, "old")).expect("lecture"),
            "ancienne"
        );
        assert!(!downloaded.exists());
        let _ = fs::remove_dir_all(folder);
    }
}
