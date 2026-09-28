//! Commandes exposées à la webview.
//!
//! C'est l'unique frontière entre le front et le système : la webview n'a aucune
//! permission Tauri au-delà de l'IPC, donc tout accès disque, réseau ou processus
//! passe par une commande typée et validée ci-dessous.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use crate::browser::{self, InstalledBrowser};
use crate::error::{validation, AppError, AppResult};
use crate::launcher::{self, LinkKind};
use crate::model::{Customer, Database, Settings};
use crate::overlay::{self, OverlayState};
use crate::shortcut;
use crate::store::{ImportMode, ImportReport, Store};
use crate::updater::{self, UpdateInfo, UpdateState};

/// Une fiche client enrichie des URL calculées à partir des gabarits.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerView {
    /// Les champs stockés de la fiche.
    #[serde(flatten)]
    customer: Customer,
    /// URL Azure DevOps, si la fiche porte un identifiant DevOps.
    devops_url: Option<String>,
    /// URL GitHub, si la fiche porte un identifiant GitHub.
    github_url: Option<String>,
    /// URL de l'Admin Center Business Central.
    admin_center_url: Option<String>,
}

impl CustomerView {
    /// Calcule les liens dynamiques d'une fiche.
    fn new(customer: Customer, settings: &Settings) -> Self {
        let devops_url = launcher::build_url(&customer, settings, LinkKind::Devops);
        let github_url = launcher::build_url(&customer, settings, LinkKind::Github);
        let admin_center_url = launcher::build_url(&customer, settings, LinkKind::AdminCenter);
        Self {
            customer,
            devops_url,
            github_url,
            admin_center_url,
        }
    }
}

/// Tout ce dont le front a besoin au premier rendu, en un seul aller-retour.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    /// Fiches clients avec leurs liens calculés.
    pub customers: Vec<CustomerView>,
    /// Réglages courants.
    pub settings: Settings,
    /// Chemin du fichier de données, affiché dans les réglages.
    pub data_path: String,
    /// Lancement au démarrage de Windows actif ou non.
    pub autostart_enabled: bool,
    /// Navigateurs connus, installés ou non, et leurs profils.
    pub browsers: Vec<InstalledBrowser>,
    /// Version de l'application en cours d'exécution.
    pub version: &'static str,
    /// Mise à jour déjà trouvée, s'il y en a une.
    pub update: Option<UpdateInfo>,
}

/// Assemble la vue de tous les clients.
fn views(store: &Store) -> AppResult<Vec<CustomerView>> {
    let database = store.snapshot()?;
    Ok(database
        .customers
        .into_iter()
        .map(|customer| CustomerView::new(customer, &database.settings))
        .collect())
}

/// Charge l'état complet nécessaire au démarrage de l'interface.
#[tauri::command]
pub fn bootstrap<R: Runtime>(app: AppHandle<R>, store: State<'_, Store>) -> AppResult<Bootstrap> {
    // Le front vient de se charger : si l'overlay reste masqué, la webview n'a
    // plus rien à faire tant que le raccourci n'a pas été pressé.
    if let Ok(window) = overlay::window(&app) {
        overlay::suspend_when_idle(window);
    }

    Ok(Bootstrap {
        customers: views(&store)?,
        settings: store.settings()?,
        data_path: store.path().to_string_lossy().into_owned(),
        autostart_enabled: autostart_enabled(&app),
        browsers: browser::detect(),
        version: updater::CURRENT_VERSION,
        update: app.state::<UpdateState>().available()?,
    })
}

/// Crée une fiche client.
#[tauri::command]
pub fn create_customer(
    store: State<'_, Store>,
    customer: Customer,
) -> AppResult<Vec<CustomerView>> {
    store.create(customer)?;
    views(&store)
}

/// Met à jour la fiche identifiée par `id`.
#[tauri::command]
pub fn update_customer(
    store: State<'_, Store>,
    id: String,
    customer: Customer,
) -> AppResult<Vec<CustomerView>> {
    store.update(&id, customer)?;
    views(&store)
}

/// Supprime la fiche identifiée par `id`.
#[tauri::command]
pub fn delete_customer(store: State<'_, Store>, id: String) -> AppResult<Vec<CustomerView>> {
    store.delete(&id)?;
    views(&store)
}

/// Enregistre les réglages et réapplique le raccourci global si besoin.
///
/// Si le nouveau raccourci est refusé par le système, l'ancien est restauré et
/// les réglages ne sont pas conservés : l'overlay reste toujours accessible.
#[tauri::command]
pub fn save_settings<R: Runtime>(
    app: AppHandle<R>,
    store: State<'_, Store>,
    settings: Settings,
) -> AppResult<Bootstrap> {
    let previous = store.settings()?;
    let hotkey_changed = previous.hotkey != settings.hotkey.trim();

    let saved = store.save_settings(settings)?;

    if hotkey_changed {
        if let Err(error) = shortcut::register(&app, &saved.hotkey) {
            store.save_settings(previous.clone())?;
            shortcut::register(&app, &previous.hotkey)?;
            return Err(error);
        }
    }

    bootstrap(app, store)
}

/// Suspend le raccourci global pendant la capture d'un nouveau raccourci.
#[tauri::command]
pub fn pause_shortcut<R: Runtime>(app: AppHandle<R>) -> AppResult<()> {
    shortcut::unregister_all(&app)
}

/// Réarme le raccourci global enregistré, à la fin d'une capture.
#[tauri::command]
pub fn resume_shortcut<R: Runtime>(app: AppHandle<R>, store: State<'_, Store>) -> AppResult<()> {
    shortcut::register(&app, &store.settings()?.hotkey)
}

/// Ouvre la cible demandée pour un client, puis referme l'overlay.
#[tauri::command]
pub fn open_link<R: Runtime>(
    app: AppHandle<R>,
    store: State<'_, Store>,
    id: String,
    kind: LinkKind,
) -> AppResult<()> {
    let customer = store.find(&id)?;
    let settings = store.settings()?;
    launcher::open(&customer, &settings, kind)?;

    // L'overlay s'efface dès qu'un lien part : on ne veut pas qu'il recouvre la
    // fenêtre qui vient de s'ouvrir.
    let window = overlay::window(&app)?;
    overlay::hide(&window)
}

/// Ouvre dans Visual Studio Code l'espace de travail `name` du client `id`,
/// puis referme l'overlay.
#[tauri::command]
pub fn open_workspace<R: Runtime>(
    app: AppHandle<R>,
    store: State<'_, Store>,
    id: String,
    name: String,
) -> AppResult<()> {
    let customer = store.find(&id)?;
    let workspace = customer
        .workspaces
        .iter()
        .find(|workspace| workspace.has_name(&name))
        .ok_or_else(|| validation(format!("Espace de travail « {name} » introuvable.")))?;

    launcher::open_in_vscode(&workspace.path)?;

    let window = overlay::window(&app)?;
    overlay::hide(&window)
}

/// Ouvre le lien personnalisé `name` du client `id`, puis referme l'overlay.
///
/// Comme les liens calculés, il part dans le profil de navigation de son hôte.
#[tauri::command]
pub fn open_custom_link<R: Runtime>(
    app: AppHandle<R>,
    store: State<'_, Store>,
    id: String,
    name: String,
) -> AppResult<()> {
    let customer = store.find(&id)?;
    let link = customer
        .links
        .iter()
        .find(|link| link.has_name(&name))
        .ok_or_else(|| validation(format!("Lien « {name} » introuvable.")))?;

    let settings = store.settings()?;
    browser::open_web(&link.url, &settings)?;

    let window = overlay::window(&app)?;
    overlay::hide(&window)
}

/// Télécharge et installe la mise à jour disponible, puis redémarre l'application.
///
/// Ne rend la main qu'en cas d'échec : en cas de succès, l'application se ferme
/// pour laisser la nouvelle version prendre sa place.
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>) -> AppResult<()> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || updater::install(&handle))
        .await
        .map_err(|error| AppError::Update(format!("tâche interrompue : {error}")))?
}

/// Masque l'overlay.
#[tauri::command]
pub fn hide_overlay<R: Runtime>(app: AppHandle<R>) -> AppResult<()> {
    let window = overlay::window(&app)?;
    overlay::hide(&window)
}

/// Ouvre l'explorateur sur le dossier contenant `data.json`, fichier sélectionné.
#[tauri::command]
pub fn reveal_data_folder(store: State<'_, Store>) -> AppResult<()> {
    launcher::reveal_file(store.path())
}

/// Active ou désactive le lancement au démarrage de Windows.
#[tauri::command]
pub fn set_autostart<R: Runtime>(app: AppHandle<R>, enabled: bool) -> AppResult<bool> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let outcome = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    outcome.map_err(|error| {
        AppError::Launch(format!(
            "Impossible de modifier le démarrage automatique : {error}"
        ))
    })?;
    Ok(autostart_enabled(&app))
}

/// Ouvre un sélecteur de dossier et retourne le chemin choisi.
#[tauri::command]
pub async fn pick_folder<R: Runtime>(app: AppHandle<R>) -> AppResult<Option<String>> {
    let selection = with_native_dialog(app, |app| {
        owned_dialog(app)
            .set_title("Dossier de travail du client")
            .blocking_pick_folder()
    })
    .await?;
    Ok(selection.map(|path| path.to_string()))
}

/// Ouvre un sélecteur de fichier `.code-workspace` et retourne le chemin choisi.
///
/// Un espace de travail Visual Studio Code est soit un dossier, soit un fichier
/// `.code-workspace` : les dialogues natifs ne sachant pas proposer les deux à la
/// fois, chacun a le sien.
#[tauri::command]
pub async fn pick_workspace_file<R: Runtime>(app: AppHandle<R>) -> AppResult<Option<String>> {
    let selection = with_native_dialog(app, |app| {
        owned_dialog(app)
            .set_title("Fichier d'espace de travail")
            .add_filter(
                "Espace de travail Visual Studio Code",
                &[launcher::WORKSPACE_EXTENSION],
            )
            .blocking_pick_file()
    })
    .await?;
    Ok(selection.map(|path| path.to_string()))
}

/// Exporte la base complète, chiffrée, vers un fichier choisi par l'utilisateur.
///
/// Le fichier reste importable par l'application chez un collègue : la clé est la
/// même dans toutes les copies.
#[tauri::command]
pub async fn export_data<R: Runtime>(
    app: AppHandle<R>,
    store: State<'_, Store>,
) -> AppResult<Option<String>> {
    let serialized = crate::store::to_encrypted(&store.snapshot()?)?;
    write_export(
        app,
        serialized,
        "Exporter le référentiel clients (chiffré)",
        &format!("{}.customers", default_export_name()),
        ("Référentiel chiffré", "customers"),
    )
    .await
}

/// Exporte la base en JSON lisible, pour l'inspecter ou la modifier en masse.
#[tauri::command]
pub async fn export_plain_data<R: Runtime>(
    app: AppHandle<R>,
    store: State<'_, Store>,
) -> AppResult<Option<String>> {
    let serialized = crate::store::to_plain_json(&store.snapshot()?)?;
    write_export(
        app,
        serialized,
        "Exporter le référentiel clients (lisible)",
        &format!("{}.json", default_export_name()),
        ("JSON", "json"),
    )
    .await
}

/// Demande où enregistrer un export, puis y écrit les octets fournis.
async fn write_export<R: Runtime>(
    app: AppHandle<R>,
    bytes: Vec<u8>,
    title: &'static str,
    file_name: &str,
    filter: (&'static str, &'static str),
) -> AppResult<Option<String>> {
    let file_name = file_name.to_owned();
    let selection = with_native_dialog(app, move |app| {
        owned_dialog(app)
            .set_title(title)
            .set_file_name(&file_name)
            .add_filter(filter.0, &[filter.1])
            .blocking_save_file()
    })
    .await?;

    let Some(target) = selection else {
        return Ok(None);
    };
    let path: PathBuf = target
        .into_path()
        .map_err(|error| validation(format!("Chemin d'export invalide : {error}")))?;
    std::fs::write(&path, bytes)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Importe un fichier JSON, après sauvegarde automatique de la base courante.
#[tauri::command]
pub async fn import_data<R: Runtime>(
    app: AppHandle<R>,
    mode: ImportMode,
) -> AppResult<Option<ImportReport>> {
    let selection = with_native_dialog(app.clone(), |app| {
        owned_dialog(app)
            .set_title("Importer un référentiel clients")
            .add_filter("Référentiel clients", &["customers", "json"])
            .blocking_pick_file()
    })
    .await?;

    let Some(source) = selection else {
        return Ok(None);
    };
    let path: PathBuf = source
        .into_path()
        .map_err(|error| validation(format!("Chemin d'import invalide : {error}")))?;

    // Le fichier importé est relu avec les mêmes garde-fous que le fichier de
    // travail : taille bornée, schéma strict, tenants uniques. Chiffré ou en
    // clair, le format est reconnu tout seul.
    let incoming: Database = crate::store::read_database(&path)?.database;

    let store = app.state::<Store>();
    let report = store.import(incoming, mode)?;
    Ok(Some(report))
}

/// Prépare une boîte de dialogue de fichiers rattachée à la fenêtre d'overlay.
///
/// Sans ce rattachement, le dialogue est une fenêtre indépendante : l'overlay,
/// qui est toujours au premier plan, la recouvre. Déclarée fille, elle passe
/// systématiquement devant son propriétaire.
fn owned_dialog<R: Runtime>(app: &AppHandle<R>) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    let builder = app.dialog().file();
    match overlay::window(app) {
        Ok(window) => builder.set_parent(&window),
        Err(_) => builder,
    }
}

/// Exécute une boîte de dialogue native hors du thread principal, en suspendant
/// le masquage automatique de l'overlay le temps de l'interaction.
///
/// L'overlay est réaffiché au retour : si l'utilisateur l'a refermé pour atteindre
/// le dialogue, son formulaire est toujours là et il doit le retrouver, sans quoi
/// la sélection qu'il vient de faire semblerait perdue.
async fn with_native_dialog<R, T, F>(app: AppHandle<R>, run: F) -> AppResult<T>
where
    R: Runtime,
    T: Send + 'static,
    F: FnOnce(&AppHandle<R>) -> T + Send + 'static,
{
    app.state::<OverlayState>().set_modal_open(true);
    let handle = app.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || run(&handle)).await;
    app.state::<OverlayState>().set_modal_open(false);

    if let Ok(window) = overlay::window(&app) {
        if !window.is_visible().unwrap_or(true) {
            let _ = overlay::show(&window);
        }
    }

    outcome.map_err(|error| AppError::Launch(format!("Dialogue interrompu : {error}")))
}

/// Racine du nom proposé à l'export, datée pour éviter les écrasements.
fn default_export_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    format!("customers-export-{stamp}")
}

/// État du lancement automatique, `false` si le plugin ne sait pas répondre.
fn autostart_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}
