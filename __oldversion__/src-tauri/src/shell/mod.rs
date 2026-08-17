pub mod autostart;
pub mod updater;
mod windows;

pub use windows::{
    apply_tray_choice, create_configured_windows, handle_window_event, is_allowed_navigation,
    register_quick_capture_shortcut, request_safe_shutdown, set_overlay_expanded, show_main_window,
};

use crate::domain::RepoError;
use crate::providers::decks::validate_official_url;

pub fn open_official_mtgo_url(value: &str) -> Result<(), RepoError> {
    let url = validate_official_url(value)?;
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(url.as_str())
            .spawn()
            .map_err(|_| RepoError::ProviderUnavailable)?;
    }
    #[cfg(not(windows))]
    {
        let _ = url;
    }
    Ok(())
}
