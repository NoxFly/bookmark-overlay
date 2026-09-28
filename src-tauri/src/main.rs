// Aucune console en release : l'application démarre avec Windows et doit rester
// totalement silencieuse.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    bookmark_overlay_lib::run()?;
    Ok(())
}
