//! Overlay de consultation du référentiel clients.
//!
//! L'application vit dans la zone de notification, sans fenêtre visible ni entrée
//! dans la barre des tâches. Un raccourci global fait apparaître un panneau
//! par-dessus l'écran actif, le temps de retrouver un client et d'ouvrir une de
//! ses ressources (Azure DevOps, GitHub, Admin Center, dossier local).

mod browser;
mod commands;
mod crypto;
mod error;
mod launcher;
mod legacy;
mod model;
mod overlay;
mod shortcut;
mod store;
mod updater;

use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::error::{AppError, AppResult};
use crate::model::{BrowserKind, Settings};
use crate::overlay::OverlayState;
use crate::store::Store;
use crate::updater::UpdateState;

/// Nom du fichier de données chiffré, dans le dossier de configuration.
///
/// Sans extension : rien n'invite à l'ouvrir, et aucun éditeur ne s'y associe.
const DATA_FILE: &str = "data";

/// Construit et lance l'application.
///
/// # Erreurs
///
/// Échoue si le dossier de configuration est inaccessible, si le fichier de
/// données existant est corrompu, ou si le runtime Tauri ne peut pas démarrer.
pub fn run() -> AppResult<()> {
    // Avant le verrou d'instance unique : après une mise à jour portable, l'ancienne
    // instance est peut-être encore en train de se fermer.
    updater::finish_pending_update();

    tauri::Builder::default()
        // Enregistré en premier : une seconde instance (double-clic alors que
        // l'application tourne déjà au démarrage) doit rendre la main tout de suite.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Ok(window) = overlay::window(app) {
                let _ = overlay::show(&window);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::create_customer,
            commands::update_customer,
            commands::delete_customer,
            commands::save_settings,
            commands::pause_shortcut,
            commands::resume_shortcut,
            commands::open_link,
            commands::open_workspace,
            commands::open_custom_link,
            commands::hide_overlay,
            commands::reveal_data_folder,
            commands::set_autostart,
            commands::pick_folder,
            commands::pick_workspace_file,
            commands::export_data,
            commands::export_plain_data,
            commands::import_data,
            commands::install_update,
            commands::open_update_page,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let config_dir = app.path().app_config_dir()?;
            // Une installation antérieure au renommage garde ses données et son
            // démarrage automatique, faute de quoi elle repartirait de zéro.
            let adopted = legacy::adopt_legacy_config(&config_dir, DATA_FILE)?;
            let store = Store::load(config_dir.join(DATA_FILE))?;
            let fresh = store.is_fresh();
            adopt_local_edge_profile(&store, fresh);
            let settings = store.settings()?;
            app.manage(store);
            app.manage(OverlayState::default());
            app.manage(UpdateState::default());

            if let Ok(window) = overlay::window(app) {
                overlay::disable_open_animation(&window);
                // Un échec de placement laisse simplement la fenêtre à sa taille
                // initiale ; elle sera replacée à la première ouverture.
                let _ = overlay::prepare(&window);
            }

            install_shortcut(&handle, &settings);
            enable_autostart_on_first_run(
                &handle,
                fresh || (adopted && legacy::take_legacy_autostart()),
            );
            build_tray(app)?;
            updater::start_background_checks(handle);

            Ok(())
        })
        .on_window_event(handle_window_event)
        .run(tauri::generate_context!())
        .map_err(AppError::Tauri)
}

/// Enregistre le raccourci global, en retombant sur le raccourci par défaut si
/// celui configuré est refusé par le système.
///
/// Un raccourci indisponible ne doit jamais empêcher l'application de démarrer :
/// l'icône de la zone de notification reste un moyen d'ouvrir l'overlay.
fn install_shortcut(handle: &tauri::AppHandle, settings: &Settings) {
    if shortcut::register(handle, &settings.hotkey).is_ok() {
        return;
    }
    let fallback = Settings::default().hotkey;
    if settings.hotkey != fallback {
        let _ = shortcut::register(handle, &fallback);
    }
}

/// Au tout premier lancement, fait pointer les profils de navigation Edge sur le
/// profil par défaut de la machine.
///
/// Rien n'est codé en dur sur un poste particulier : l'application copiée chez un
/// collègue s'aligne sur *ses* profils. Répartir les liens sur d'autres profils ou
/// d'autres navigateurs reste un choix personnel, à faire dans les réglages.
fn adopt_local_edge_profile(store: &Store, fresh: bool) {
    if !fresh {
        return;
    }
    let Some(local) = browser::default_edge_profile() else {
        return;
    };
    let Ok(mut settings) = store.settings() else {
        return;
    };
    for profile in &mut settings.browser_profiles {
        if profile.browser == BrowserKind::Edge {
            profile.profile.clone_from(&local);
        }
    }
    // Un échec d'écriture laisse simplement le profil de repli en place.
    let _ = store.save_settings(settings);
}

/// Inscrit l'application au démarrage de Windows, au tout premier lancement.
///
/// C'est le comportement attendu d'un overlay : il doit être là sans qu'on y pense.
/// Le réglage reste ensuite entre les mains de l'utilisateur, qui peut le couper
/// depuis les réglages sans que l'application le réactive au lancement suivant.
fn enable_autostart_on_first_run(handle: &tauri::AppHandle, fresh: bool) {
    use tauri_plugin_autostart::ManagerExt;

    if !fresh {
        return;
    }
    // Un échec ici (politique d'entreprise, registre verrouillé) ne doit pas
    // empêcher l'application de démarrer : le réglage reste accessible à la main.
    let _ = handle.autolaunch().enable();
}

/// Construit l'icône de la zone de notification et son menu.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open", "Ouvrir l'overlay")
        .separator()
        .text("data", "Ouvrir le dossier des données")
        .separator()
        .text("quit", "Quitter")
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("icône de l'application".to_owned()))?;

    TrayIconBuilder::with_id("tray")
        .icon(icon)
        .tooltip("Bookmark Overlay")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                if let Ok(window) = overlay::window(app) {
                    let _ = overlay::show(&window);
                }
            }
            "data" => {
                let _ = launcher::reveal_file(app.state::<Store>().path());
            }
            "quit" => app.exit(0),
            // Le menu est construit ici : tout autre identifiant serait un oubli
            // de câblage, sans action associée.
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Ok(window) = overlay::window(tray.app_handle()) {
                    let _ = overlay::toggle(&window);
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Politique de fenêtre : l'overlay se masque, il ne se ferme jamais.
fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    let Some(overlay_window) = window.app_handle().get_webview_window(overlay::MAIN_WINDOW) else {
        return;
    };

    match event {
        // Fermer la fenêtre terminerait le processus et ferait perdre le bénéfice
        // de la webview déjà chargée : on masque à la place.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = overlay::hide(&overlay_window);
        }
        // Comportement attendu d'un overlay : cliquer ailleurs le referme. Sauf
        // pendant une boîte de dialogue native, ou pendant que la fenêtre s'installe
        // au premier plan : dans les deux cas la perte de focus n'est pas un départ.
        WindowEvent::Focused(false) if !window.state::<OverlayState>().ignores_focus_loss() => {
            let _ = overlay::hide(&overlay_window);
        }
        // `WindowEvent` est non exhaustif côté Tauri ; les autres variantes
        // (déplacement, redimensionnement, DPI...) ne concernent pas l'overlay.
        _ => {}
    }
}
