use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::classifier::{AssetSource, ClassifierAssets, DeckClassifier};
use crate::commands::classifier::DeckEnrichmentRuntime;
use crate::ipc::{AppError, CallerIdentity, CommandResult, ErrorCode, panic_boundary};
use crate::notebook::NotebookRuntime;
use crate::services::decks::ReclassificationProgress;
use crate::settings::AppState;
use crate::shell::updater::{
    InstallUpdateRequest, SignedRelease, UpdateCheckRequest, UpdateStatus, UpdaterService,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierAssetBundle {
    pub manifest_json: String,
    pub definitions_json: String,
    pub corpus_json: String,
    pub golden_json: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierUpdateView {
    pub classifier_version: String,
    pub digest: String,
    pub formats: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallClassifierUpdateRequest {
    pub confirmed: bool,
}

trait ClassifierUpdateEndpoint: Send + Sync {
    fn check(
        &self,
        request: &UpdateCheckRequest,
    ) -> Result<Option<ClassifierAssetBundle>, AppError>;
}

#[derive(Default)]
struct NoClassifierUpdateEndpoint;

impl ClassifierUpdateEndpoint for NoClassifierUpdateEndpoint {
    fn check(
        &self,
        _request: &UpdateCheckRequest,
    ) -> Result<Option<ClassifierAssetBundle>, AppError> {
        Ok(None)
    }
}

struct HttpClassifierUpdateEndpoint {
    client: reqwest::blocking::Client,
    endpoint_url: &'static str,
}

impl HttpClassifierUpdateEndpoint {
    fn configured() -> Option<Self> {
        let endpoint_url = option_env!("MTGO_NOTES_CLASSIFIER_UPDATE_ENDPOINT")?;
        if !endpoint_url.starts_with("https://") {
            return None;
        }
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self {
            client,
            endpoint_url,
        })
    }
}

impl ClassifierUpdateEndpoint for HttpClassifierUpdateEndpoint {
    fn check(
        &self,
        request: &UpdateCheckRequest,
    ) -> Result<Option<ClassifierAssetBundle>, AppError> {
        const MAX_CLASSIFIER_BUNDLE_BYTES: u64 = 16 * 1024 * 1024;
        let response = self
            .client
            .get(self.endpoint_url)
            .query(request)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| update_unavailable())?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_CLASSIFIER_BUNDLE_BYTES)
        {
            return Err(update_unavailable());
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_CLASSIFIER_BUNDLE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| update_unavailable())?;
        if bytes.len() as u64 > MAX_CLASSIFIER_BUNDLE_BYTES {
            return Err(update_unavailable());
        }
        serde_json::from_slice::<Option<ClassifierAssetBundle>>(&bytes)
            .map_err(|_| update_unavailable())
    }
}

pub struct UpdateRuntime {
    pub application: UpdaterService,
    classifier_endpoint: Arc<dyn ClassifierUpdateEndpoint>,
    classifier_pending: Mutex<Option<ClassifierAssetBundle>>,
}

impl Default for UpdateRuntime {
    fn default() -> Self {
        let classifier_endpoint: Arc<dyn ClassifierUpdateEndpoint> =
            HttpClassifierUpdateEndpoint::configured()
                .map(|endpoint| Arc::new(endpoint) as Arc<dyn ClassifierUpdateEndpoint>)
                .unwrap_or_else(|| Arc::new(NoClassifierUpdateEndpoint));
        Self {
            application: UpdaterService::default(),
            classifier_endpoint,
            classifier_pending: Mutex::new(None),
        }
    }
}

impl UpdateRuntime {
    #[cfg(test)]
    fn with_classifier_endpoint(endpoint: Arc<dyn ClassifierUpdateEndpoint>) -> Self {
        Self {
            application: UpdaterService::default(),
            classifier_endpoint: endpoint,
            classifier_pending: Mutex::new(None),
        }
    }

    pub fn stage_classifier_bundle(
        &self,
        enabled: bool,
        bundle: ClassifierAssetBundle,
    ) -> Result<ClassifierUpdateView, AppError> {
        if !enabled {
            return Err(update_unavailable());
        }
        let assets = validate_bundle(&bundle)?;
        let view = classifier_view(&assets);
        *self
            .classifier_pending
            .lock()
            .map_err(|_| AppError::internal("classifier-update-lock"))? = Some(bundle);
        Ok(view)
    }

    fn pending_classifier(&self) -> Result<ClassifierAssetBundle, AppError> {
        self.classifier_pending
            .lock()
            .map_err(|_| AppError::internal("classifier-update-lock"))?
            .clone()
            .ok_or_else(update_unavailable)
    }

    fn clear_classifier(&self) -> Result<(), AppError> {
        *self
            .classifier_pending
            .lock()
            .map_err(|_| AppError::internal("classifier-update-lock"))? = None;
        Ok(())
    }
}

pub fn check_update_for(
    caller: CallerIdentity,
    state: &AppState,
    runtime: &UpdateRuntime,
) -> CommandResult<SignedRelease> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let enabled = match state.settings.lock() {
        Ok(store) => store.settings.update_checks_enabled,
        Err(_) => return CommandResult::failure(AppError::internal("update-settings-lock")),
    };
    match runtime
        .application
        .check(enabled, UpdateCheckRequest::current())
    {
        Ok(release) => CommandResult::success(release, 1),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn install_update_for(
    caller: CallerIdentity,
    state: &AppState,
    runtime: &UpdateRuntime,
    request: InstallUpdateRequest,
) -> CommandResult<UpdateStatus> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    match state.settings.lock() {
        Ok(store) if store.settings.update_checks_enabled => {}
        Ok(_) => return CommandResult::failure(update_unavailable()),
        Err(_) => return CommandResult::failure(AppError::internal("update-settings-lock")),
    }
    match runtime.application.install(request) {
        Ok(status) => CommandResult::success(status, 1),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn check_classifier_update_for(
    caller: CallerIdentity,
    state: &AppState,
    runtime: &UpdateRuntime,
) -> CommandResult<ClassifierUpdateView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let enabled = match state.settings.lock() {
        Ok(store) => store.settings.classifier_update_checks_enabled,
        Err(_) => {
            return CommandResult::failure(AppError::internal("classifier-update-settings-lock"));
        }
    };
    if !enabled {
        return CommandResult::failure(update_unavailable());
    }
    if let Ok(bundle) = runtime.pending_classifier() {
        return match validate_bundle(&bundle) {
            Ok(assets) => CommandResult::success(classifier_view(&assets), 1),
            Err(error) => CommandResult::failure(error),
        };
    }
    match runtime
        .classifier_endpoint
        .check(&UpdateCheckRequest::current())
    {
        Ok(Some(bundle)) => match runtime.stage_classifier_bundle(true, bundle) {
            Ok(view) => CommandResult::success(view, 1),
            Err(error) => CommandResult::failure(error),
        },
        Ok(None) => CommandResult::failure(update_unavailable()),
        Err(error) => CommandResult::failure(error),
    }
}

pub fn install_classifier_update_for(
    caller: CallerIdentity,
    state: &AppState,
    update_runtime: &UpdateRuntime,
    enrichment: &DeckEnrichmentRuntime,
    notebook: &NotebookRuntime,
    request: InstallClassifierUpdateRequest,
) -> CommandResult<ReclassificationProgress> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    if !request.confirmed {
        return CommandResult::failure(AppError::new(
            ErrorCode::InvalidRequest,
            "Confirm classifier activation before applying signed assets.",
            false,
        ));
    }
    match state.settings.lock() {
        Ok(store) if store.settings.classifier_update_checks_enabled => {}
        Ok(_) => return CommandResult::failure(update_unavailable()),
        Err(_) => {
            return CommandResult::failure(AppError::internal("classifier-update-settings-lock"));
        }
    }
    let result = update_runtime
        .pending_classifier()
        .and_then(|bundle| {
            enrichment
                .assets
                .activate(AssetSource {
                    manifest_json: &bundle.manifest_json,
                    definitions_json: &bundle.definitions_json,
                    corpus_json: &bundle.corpus_json,
                    golden_json: &bundle.golden_json,
                })
                .map_err(|error| error.to_app_error())
        })
        .and_then(|assets| {
            enrichment
                .reclassification
                .start(&notebook.repository, &assets)
                .map_err(|error| error.to_app_error())
        })
        .and_then(|progress| {
            update_runtime.clear_classifier()?;
            Ok(progress)
        });
    match result {
        Ok(progress) => CommandResult::success(progress, 1),
        Err(error) => CommandResult::failure(error),
    }
}

#[tauri::command]
pub fn check_update(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, UpdateRuntime>,
) -> CommandResult<SignedRelease> {
    panic_boundary("check-update-command", || {
        with_caller(&window, |caller| {
            check_update_for(caller, state.inner(), runtime.inner())
        })
    })
}

#[tauri::command]
pub fn install_update(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, UpdateRuntime>,
    request: InstallUpdateRequest,
) -> CommandResult<UpdateStatus> {
    panic_boundary("install-update-command", || {
        with_caller(&window, |caller| {
            install_update_for(caller, state.inner(), runtime.inner(), request)
        })
    })
}

#[tauri::command]
pub fn check_classifier_update(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    runtime: tauri::State<'_, UpdateRuntime>,
) -> CommandResult<ClassifierUpdateView> {
    panic_boundary("check-classifier-update-command", || {
        with_caller(&window, |caller| {
            check_classifier_update_for(caller, state.inner(), runtime.inner())
        })
    })
}

#[tauri::command]
pub fn install_classifier_update(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
    update_runtime: tauri::State<'_, UpdateRuntime>,
    enrichment: tauri::State<'_, DeckEnrichmentRuntime>,
    notebook: tauri::State<'_, NotebookRuntime>,
    request: InstallClassifierUpdateRequest,
) -> CommandResult<ReclassificationProgress> {
    panic_boundary("install-classifier-update-command", || {
        with_caller(&window, |caller| {
            install_classifier_update_for(
                caller,
                state.inner(),
                update_runtime.inner(),
                enrichment.inner(),
                notebook.inner(),
                request,
            )
        })
    })
}

fn validate_bundle(bundle: &ClassifierAssetBundle) -> Result<ClassifierAssets, AppError> {
    DeckClassifier::load(AssetSource {
        manifest_json: &bundle.manifest_json,
        definitions_json: &bundle.definitions_json,
        corpus_json: &bundle.corpus_json,
        golden_json: &bundle.golden_json,
    })
    .map_err(|error| error.to_app_error())
}

fn classifier_view(assets: &ClassifierAssets) -> ClassifierUpdateView {
    ClassifierUpdateView {
        classifier_version: assets.manifest.classifier_version.clone(),
        digest: assets.digest.clone(),
        formats: assets
            .formats
            .iter()
            .map(|format| format.name.clone())
            .collect(),
    }
}

fn update_unavailable() -> AppError {
    AppError::new(
        ErrorCode::UpdateUnavailable,
        "No newer signed classifier assets are available.",
        false,
    )
}

fn with_caller<T>(
    window: &tauri::WebviewWindow,
    operation: impl FnOnce(CallerIdentity) -> CommandResult<T>,
) -> CommandResult<T> {
    match CallerIdentity::from_window_label(window.label()) {
        Ok(caller) => operation(caller),
        Err(error) => CommandResult::failure(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureClassifierEndpoint {
        bundle: ClassifierAssetBundle,
    }

    impl ClassifierUpdateEndpoint for FixtureClassifierEndpoint {
        fn check(
            &self,
            _request: &UpdateCheckRequest,
        ) -> Result<Option<ClassifierAssetBundle>, AppError> {
            Ok(Some(self.bundle.clone()))
        }
    }

    #[test]
    fn e2e_019_signed_classifier_update_has_no_user_asset_surface() {
        let runtime = UpdateRuntime::default();
        let (manifest_json, definitions_json, corpus_json, golden_json) =
            DeckClassifier::builtin_asset_json();
        let bundle = ClassifierAssetBundle {
            manifest_json,
            definitions_json,
            corpus_json,
            golden_json,
        };
        let view = runtime
            .stage_classifier_bundle(true, bundle)
            .expect("signed bundle");
        assert!(!view.formats.is_empty());
        let json = serde_json::to_string(&view).expect("json");
        for prohibited in ["editor", "import", "path", "activateDefinition", "delete"] {
            assert!(!json.contains(prohibited));
        }
    }

    #[test]
    fn tampered_classifier_update_never_replaces_pending_last_known_good() {
        let runtime = UpdateRuntime::default();
        let (manifest_json, definitions_json, mut corpus_json, golden_json) =
            DeckClassifier::builtin_asset_json();
        corpus_json.push(' ');
        let result = runtime.stage_classifier_bundle(
            true,
            ClassifierAssetBundle {
                manifest_json,
                definitions_json,
                corpus_json,
                golden_json,
            },
        );
        assert_eq!(result.expect_err("tampered").code, ErrorCode::AssetsInvalid);
        assert!(runtime.pending_classifier().is_err());
    }

    #[test]
    fn configured_classifier_endpoint_stages_validated_assets() {
        let (manifest_json, definitions_json, corpus_json, golden_json) =
            DeckClassifier::builtin_asset_json();
        let runtime =
            UpdateRuntime::with_classifier_endpoint(Arc::new(FixtureClassifierEndpoint {
                bundle: ClassifierAssetBundle {
                    manifest_json,
                    definitions_json,
                    corpus_json,
                    golden_json,
                },
            }));
        let state = AppState::default();
        state
            .settings
            .lock()
            .expect("settings")
            .settings
            .classifier_update_checks_enabled = true;

        let result = check_classifier_update_for(CallerIdentity::Main, &state, &runtime);
        assert!(result.is_success());
        assert!(runtime.pending_classifier().is_ok());
    }
}
