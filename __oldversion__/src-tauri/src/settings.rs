use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::domain::InternalPhase;
use crate::ipc::AppError;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub provider_access_enabled: bool,
    pub overlay_enabled: bool,
    pub tray_enabled: bool,
    pub launch_with_windows: bool,
    pub update_checks_enabled: bool,
    pub classifier_update_checks_enabled: bool,
    pub diagnostics_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            provider_access_enabled: false,
            overlay_enabled: true,
            tray_enabled: true,
            launch_with_windows: false,
            update_checks_enabled: false,
            classifier_update_checks_enabled: false,
            diagnostics_enabled: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    #[serde(default)]
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub settings: Settings,
}

#[derive(Debug)]
pub struct SettingsStore {
    pub settings: Settings,
    pub revision: u64,
    pub idempotent_results: HashMap<String, (Settings, u64)>,
    persistence_path: Option<PathBuf>,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            revision: 1,
            idempotent_results: HashMap::new(),
            persistence_path: None,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedSettings {
    schema_version: u32,
    revision: u64,
    settings: Settings,
}

impl SettingsStore {
    pub fn configure_persistence(&mut self, path: PathBuf) -> Result<(), AppError> {
        self.persistence_path = Some(path.clone());
        if !path.exists() {
            return self.persist_candidate(&self.settings, self.revision);
        }

        let bytes = fs::read(&path).map_err(|_| settings_io_error())?;
        let persisted: PersistedSettings =
            serde_json::from_slice(&bytes).map_err(|_| settings_io_error())?;
        if persisted.schema_version != SETTINGS_SCHEMA_VERSION
            || persisted.settings.schema_version != SETTINGS_SCHEMA_VERSION
            || persisted.revision == 0
        {
            return Err(AppError::new(
                crate::ipc::ErrorCode::InvalidRequest,
                "The local settings version is unsupported.",
                false,
            ));
        }
        self.settings = persisted.settings;
        self.revision = persisted.revision;
        self.idempotent_results.clear();
        Ok(())
    }

    pub fn persist_candidate(&self, settings: &Settings, revision: u64) -> Result<(), AppError> {
        if settings.schema_version != SETTINGS_SCHEMA_VERSION || revision == 0 {
            return Err(AppError::new(
                crate::ipc::ErrorCode::InvalidRequest,
                "The settings schema version is invalid.",
                false,
            ));
        }
        let Some(path) = self.persistence_path.as_deref() else {
            return Ok(());
        };
        persist_settings(path, settings, revision)
    }
}

fn persist_settings(path: &Path, settings: &Settings, revision: u64) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(settings_io_error)?;
    fs::create_dir_all(parent).map_err(|_| settings_io_error())?;
    let partial = path.with_extension("json.partial");
    let bytes = serde_json::to_vec_pretty(&PersistedSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        revision,
        settings: settings.clone(),
    })
    .map_err(|_| settings_io_error())?;
    fs::write(&partial, bytes).map_err(|_| settings_io_error())?;
    let previous = path.with_extension("json.previous");
    if path.exists() {
        let _ = fs::remove_file(&previous);
        fs::rename(path, &previous).map_err(|_| settings_io_error())?;
    }
    if fs::rename(&partial, path).is_err() {
        if previous.exists() {
            let _ = fs::rename(&previous, path);
        }
        let _ = fs::remove_file(&partial);
        return Err(settings_io_error());
    }
    let _ = fs::remove_file(previous);
    Ok(())
}

fn settings_io_error() -> AppError {
    AppError::new(
        crate::ipc::ErrorCode::SaveFailed,
        "Local settings could not be saved. The prior choices remain active.",
        true,
    )
}

#[derive(Debug)]
pub struct AppState {
    pub settings: Mutex<SettingsStore>,
    pub notebook_error: Mutex<Option<AppError>>,
    pub phase: Mutex<InternalPhase>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(SettingsStore::default()),
            notebook_error: Mutex::new(None),
            phase: Mutex::new(InternalPhase::Idle),
        }
    }
}

impl AppState {
    pub fn with_notebook_error(error: AppError) -> Self {
        Self {
            notebook_error: Mutex::new(Some(error)),
            ..Self::default()
        }
    }

    pub fn configure_settings_path(&self, path: PathBuf) -> Result<(), AppError> {
        self.settings
            .lock()
            .map_err(|_| AppError::internal("settings-persistence-lock"))?
            .configure_persistence(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_choices_have_privacy_preserving_defaults() {
        let settings = Settings::default();
        assert_eq!(settings.schema_version, SETTINGS_SCHEMA_VERSION);
        assert!(!settings.provider_access_enabled);
        assert!(!settings.launch_with_windows);
        assert!(!settings.update_checks_enabled);
        assert!(!settings.classifier_update_checks_enabled);
        assert!(!settings.diagnostics_enabled);
    }

    #[test]
    fn versioned_settings_survive_restart_without_secrets() {
        let directory = tempfile::tempdir().expect("temp");
        let path = directory.path().join("settings.json");
        let mut first = SettingsStore::default();
        first
            .configure_persistence(path.clone())
            .expect("configure");
        let next = Settings {
            provider_access_enabled: true,
            update_checks_enabled: true,
            ..Settings::default()
        };
        first.persist_candidate(&next, 2).expect("persist");

        let mut restarted = SettingsStore::default();
        restarted.configure_persistence(path).expect("reload");
        assert_eq!(restarted.revision, 2);
        assert!(restarted.settings.provider_access_enabled);
        assert!(restarted.settings.update_checks_enabled);
        let serialized = serde_json::to_string(&restarted.settings).expect("json");
        for prohibited in ["secret", "passphrase", "token", "handle"] {
            assert!(!serialized.to_ascii_lowercase().contains(prohibited));
        }
    }
}
