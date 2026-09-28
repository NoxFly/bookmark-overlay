//! Pilotage de la fenêtre d'overlay : placement, affichage, masquage.
//!
//! La fenêtre est créée une fois au démarrage puis simplement masquée/affichée.
//! Recréer la webview à chaque ouverture coûterait plusieurs centaines de
//! millisecondes ; la garder vivante rend le raccourci instantané, et le working
//! set du processus et de ses moteurs WebView2 est rendu au système à chaque
//! masquage pour que le coût au repos reste bas.
//!
//! `ICoreWebView2_3::TrySuspend` a été essayé en complément : il impose de masquer
//! le contrôleur de la vue, si bien que la fenêtre réapparaissait vide le temps que
//! le réveil soit traité — un clignotement visible à chaque ouverture. Ordonner
//! réveil puis affichage aurait demandé de piloter la fenêtre depuis le rappel
//! `with_webview`, ce qui bloque le thread principal. La purge du working set
//! apporte de toute façon l'essentiel du gain.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

use crate::error::{AppError, AppResult};

/// Étiquette de la fenêtre unique de l'application.
pub const MAIN_WINDOW: &str = "main";

/// Événement émis vers le front quand l'overlay devient visible.
pub const EVENT_SHOWN: &str = "overlay://shown";

/// Événement émis juste avant de masquer l'overlay.
///
/// Le front en profite pour remettre le panneau dans son état d'avant-ouverture.
/// La webview continue de composer alors que la fenêtre est masquée : l'image
/// conservée est donc celle du départ de l'animation, et non celle de l'état
/// final — sans quoi la réapparition afficherait le panneau en place avant de le
/// faire sauter à sa position de départ pour jouer le glissement.
pub const EVENT_HIDDEN: &str = "overlay://hidden";

/// État partagé du pilotage de l'overlay.
#[derive(Debug, Default)]
pub struct OverlayState {
    /// Vrai pendant qu'une boîte de dialogue native est ouverte.
    ///
    /// La fenêtre perd alors le focus sans que l'utilisateur ait quitté l'overlay :
    /// le masquage automatique doit être suspendu, sinon la sélection de dossier
    /// se retrouverait orpheline.
    modal_open: AtomicBool,

    /// Vrai le temps que la fenêtre s'installe au premier plan.
    ///
    /// Windows émet des changements de focus transitoires pendant qu'une fenêtre
    /// devient active. Sans ce délai, le masquage automatique les prend pour un
    /// clic ailleurs et referme l'overlay dans la seconde qui suit son ouverture.
    settling: AtomicBool,
}

impl OverlayState {
    /// Signale l'ouverture ou la fermeture d'une boîte de dialogue native.
    pub fn set_modal_open(&self, open: bool) {
        self.modal_open.store(open, Ordering::SeqCst);
    }

    /// Marque le début et la fin de la phase d'installation au premier plan.
    fn set_settling(&self, settling: bool) {
        self.settling.store(settling, Ordering::SeqCst);
    }

    /// Indique si le masquage automatique sur perte de focus doit être ignoré.
    pub fn ignores_focus_loss(&self) -> bool {
        self.modal_open.load(Ordering::SeqCst) || self.settling.load(Ordering::SeqCst)
    }
}

/// Place la fenêtre sur l'écran dès le démarrage, avant toute ouverture.
///
/// Sans cela, la première image composée est celle de la fenêtre initiale, centrée
/// et plus petite : au premier raccourci, le panneau apparaissait une fraction de
/// seconde au centre avant de rejoindre le bord droit en pleine hauteur.
pub fn prepare<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    stretch_over_active_monitor(window)
}

/// Récupère la fenêtre d'overlay.
pub fn window<R: Runtime, M: Manager<R>>(manager: &M) -> AppResult<WebviewWindow<R>> {
    manager
        .get_webview_window(MAIN_WINDOW)
        .ok_or(AppError::Tauri(tauri::Error::WebviewNotFound))
}

/// Affiche l'overlay sur l'écran où se trouve le curseur, puis lui donne le focus.
pub fn show<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    // Windows émet des changements de focus transitoires pendant qu'une fenêtre
    // devient active ; sans ce sursis, le masquage automatique les prendrait pour
    // un clic ailleurs et refermerait l'overlay aussitôt ouvert.
    window.state::<OverlayState>().set_settling(true);

    stretch_over_active_monitor(window)?;
    window.show()?;
    window.set_focus()?;
    window.emit(EVENT_SHOWN, ())?;
    stop_settling_later(window.clone());
    Ok(())
}

/// Rétablit le masquage automatique une fois la fenêtre installée au premier plan.
fn stop_settling_later<R: Runtime>(window: WebviewWindow<R>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        window.state::<OverlayState>().set_settling(false);
    });
}

/// Masque l'overlay et rend sa mémoire au système.
pub fn hide<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    if !window.is_visible().unwrap_or(false) {
        return Ok(());
    }
    // Émis avant le masquage, pour que le front ait le temps de repeindre.
    window.emit(EVENT_HIDDEN, ())?;
    window.hide()?;
    release_working_set();
    Ok(())
}

/// Suspend la webview si l'overlay est toujours masqué dans un instant.
///
/// Appelé quand le front a fini de charger : au démarrage de Windows, la fenêtre
/// n'a jamais été affichée, et rien ne justifie de garder le moteur de rendu
/// pleinement résident en attendant le premier raccourci.
pub fn suspend_when_idle<R: Runtime>(window: WebviewWindow<R>) {
    std::thread::spawn(move || {
        // Laisse la réponse IPC en cours atteindre la webview avant de la suspendre.
        std::thread::sleep(std::time::Duration::from_millis(600));
        if !window.is_visible().unwrap_or(false) {
            release_working_set();
        }
    });
}

/// Bascule l'affichage de l'overlay.
pub fn toggle<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    if window.is_visible().unwrap_or(false) {
        hide(window)
    } else {
        show(window)
    }
}

/// Étire la fenêtre sur tout l'écran actif : le fond assombri doit couvrir le
/// moniteur entier, le panneau étant positionné en CSS à l'intérieur.
fn stretch_over_active_monitor<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    let monitor = window
        .cursor_position()
        .ok()
        .and_then(|cursor| window.monitor_from_point(cursor.x, cursor.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        // Aucun moniteur rapporté (session verrouillée, bureau distant) : on laisse
        // la fenêtre où elle est plutôt que de la placer à une position arbitraire.
        return Ok(());
    };

    let position = monitor.position();
    let size = monitor.size();
    window.set_position(PhysicalPosition::new(position.x, position.y))?;
    window.set_size(PhysicalSize::new(size.width, size.height))?;
    Ok(())
}

/// Coupe l'animation d'ouverture de fenêtre du gestionnaire de bureau.
///
/// Sans cela, Windows anime l'apparition de la fenêtre en la faisant grandir :
/// le voile de fond se met à zoomer avec le reste, ce qui donne l'impression que
/// c'est tout l'écran qui grossit. Le voile doit apparaître à sa taille finale,
/// et seul le panneau s'animer — ce qui est ensuite fait en CSS.
#[cfg(windows)]
pub fn disable_open_animation<R: Runtime>(window: &WebviewWindow<R>) {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
    };

    let Ok(handle) = window.hwnd() else {
        return;
    };
    let disabled: i32 = 1;

    // SAFETY: `handle` est le descripteur de fenêtre que Tauri vient de créer et
    // détient. `DwmSetWindowAttribute` lit `size_of::<i32>()` octets à l'adresse
    // fournie, qui pointe sur une variable locale vivante de cette taille exacte.
    // L'échec (DWM indisponible) est sans conséquence : l'animation reste active.
    unsafe {
        DwmSetWindowAttribute(
            handle.0 as _,
            DWMWA_TRANSITIONS_FORCEDISABLED as u32,
            std::ptr::addr_of!(disabled).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        );
    }
}

/// Ailleurs, aucune animation système ne s'interpose.
#[cfg(not(windows))]
pub fn disable_open_animation<R: Runtime>(_window: &WebviewWindow<R>) {}

/// Nom des processus enfants de WebView2, seuls candidats à la purge.
#[cfg(windows)]
const WEBVIEW_PROCESS_NAME: &str = "msedgewebview2.exe";

/// Rend au système la mémoire résidente du processus et de ses moteurs WebView2.
///
/// Le gros de l'empreinte n'est pas dans notre processus mais dans les processus
/// enfants de WebView2 : suspendre la vue libère son moteur JavaScript, pas les
/// surfaces déjà composées. Vider leur working set renvoie ces pages dans le
/// fichier d'échange ; elles reviennent à la prochaine ouverture, ce qui est le
/// bon compromis pour une fenêtre affichée quelques secondes par heure.
#[cfg(windows)]
fn release_working_set() {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, OpenProcess, SetProcessWorkingSetSize,
        PROCESS_SET_QUOTA,
    };

    // SAFETY: `GetCurrentProcess` renvoie un pseudo-handle toujours valide, qui n'a
    // pas à être fermé. `SetProcessWorkingSetSize(handle, usize::MAX, usize::MAX)` est
    // l'appel documenté pour vider le working set du processus visé ; il ne
    // déréférence aucun pointeur fourni par nous. Son échec est sans conséquence
    // (simple optimisation), d'où le retour ignoré.
    unsafe {
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }

    // SAFETY: chaque `pid` provient de l'énumération ci-dessous, filtrée sur nos
    // propres descendants portant le nom de l'exécutable WebView2. `OpenProcess`
    // renvoie soit un handle valide, soit un handle nul que l'on écarte avant tout
    // usage ; ce handle est refermé dans tous les chemins.
    for pid in webview_descendants(unsafe { GetCurrentProcessId() }) {
        unsafe {
            let handle = OpenProcess(PROCESS_SET_QUOTA, 0, pid);
            if handle.is_null() {
                continue;
            }
            SetProcessWorkingSetSize(handle, usize::MAX, usize::MAX);
            CloseHandle(handle);
        }
    }
}

/// Liste les processus WebView2 descendants de `root`.
///
/// Le filtrage sur le nom de l'exécutable est une sécurité : Windows réutilise les
/// identifiants de processus, et un identifiant parent périmé pourrait autrement
/// désigner un processus étranger à l'application.
#[cfg(windows)]
fn webview_descendants(root: u32) -> Vec<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };

    // SAFETY: le handle de l'instantané est vérifié puis refermé ; `entry` est une
    // structure locale dont le champ `dwSize` est renseigné comme l'exige l'API,
    // et `Process32NextW` n'écrit que dans cette structure.
    let processes: Vec<(u32, u32, String)> = unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..std::mem::zeroed()
        };
        let mut collected = Vec::new();
        while Process32NextW(snapshot, &mut entry) != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
            collected.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
        }
        CloseHandle(snapshot);
        collected
    };

    let mut tree = vec![root];
    let mut found = Vec::new();
    let mut growing = true;
    while growing {
        growing = false;
        for (pid, parent, name) in &processes {
            if tree.contains(parent)
                && !tree.contains(pid)
                && name.eq_ignore_ascii_case(WEBVIEW_PROCESS_NAME)
            {
                tree.push(*pid);
                found.push(*pid);
                growing = true;
            }
        }
    }
    found
}

/// Sur les autres plateformes, le système gère seul la résidence mémoire.
#[cfg(not(windows))]
fn release_working_set() {}
