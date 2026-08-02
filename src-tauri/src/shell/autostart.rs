use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::ipc::{AppError, ErrorCode};

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "MTGOOpponentNotes";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartState {
    pub enabled: bool,
    pub reversible: bool,
}

trait AutostartBackend: Send + Sync {
    fn apply(&self, enabled: bool) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct AutostartController {
    backend: Arc<dyn AutostartBackend>,
    enabled: Arc<Mutex<bool>>,
}

impl Default for AutostartController {
    fn default() -> Self {
        Self {
            backend: Arc::new(OsAutostartBackend),
            enabled: Arc::new(Mutex::new(false)),
        }
    }
}

impl AutostartController {
    pub fn apply_explicit_choice(&self, enabled: bool) -> Result<AutostartState, AppError> {
        let mut current = self.enabled.lock().map_err(|_| {
            AppError::new(
                ErrorCode::SaveFailed,
                "The Windows startup choice could not be changed.",
                true,
            )
        })?;
        if *current != enabled {
            self.backend.apply(enabled)?;
            *current = enabled;
        }
        Ok(AutostartState {
            enabled,
            reversible: true,
        })
    }

    pub fn state(&self) -> Result<AutostartState, AppError> {
        self.enabled
            .lock()
            .map_err(|_| AppError::internal("autostart-state-lock"))
            .map(|enabled| AutostartState {
                enabled: *enabled,
                reversible: true,
            })
    }
}

struct OsAutostartBackend;

#[cfg(windows)]
impl AutostartBackend for OsAutostartBackend {
    fn apply(&self, enabled: bool) -> Result<(), AppError> {
        let executable = std::env::current_exe().map_err(|_| autostart_error())?;
        let status = if enabled {
            std::process::Command::new("reg.exe")
                .args(["ADD", RUN_KEY, "/v", VALUE_NAME, "/t", "REG_SZ", "/d"])
                .arg(format!("\"{}\"", executable.display()))
                .args(["/f"])
                .status()
        } else {
            let existing = std::process::Command::new("reg.exe")
                .args(["QUERY", RUN_KEY, "/v", VALUE_NAME])
                .status()
                .map_err(|_| autostart_error())?;
            if !existing.success() {
                return Ok(());
            }
            std::process::Command::new("reg.exe")
                .args(["DELETE", RUN_KEY, "/v", VALUE_NAME, "/f"])
                .status()
        }
        .map_err(|_| autostart_error())?;

        if status.success() {
            Ok(())
        } else {
            Err(autostart_error())
        }
    }
}

#[cfg(not(windows))]
impl AutostartBackend for OsAutostartBackend {
    fn apply(&self, _enabled: bool) -> Result<(), AppError> {
        Ok(())
    }
}

#[cfg(windows)]
fn autostart_error() -> AppError {
    AppError::new(
        ErrorCode::SaveFailed,
        "The Windows startup choice could not be changed.",
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryBackend {
        enabled: Mutex<bool>,
    }

    impl AutostartBackend for MemoryBackend {
        fn apply(&self, enabled: bool) -> Result<(), AppError> {
            *self.enabled.lock().expect("memory backend") = enabled;
            Ok(())
        }
    }

    #[test]
    fn ut_098_autostart_is_off_by_default_and_reversible_without_notebook_state() {
        let controller = AutostartController {
            backend: Arc::new(MemoryBackend::default()),
            enabled: Arc::new(Mutex::new(false)),
        };
        assert!(!controller.state().expect("default").enabled);
        assert!(
            controller
                .apply_explicit_choice(true)
                .expect("enable")
                .enabled
        );
        assert!(
            !controller
                .apply_explicit_choice(false)
                .expect("disable")
                .enabled
        );
    }
}
