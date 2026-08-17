use serde::{Deserialize, Serialize};

use crate::classifier::AssetRegistry;
use crate::domain::EntityId;
use crate::ipc::{CallerIdentity, CommandResult, panic_boundary};
use crate::notebook::NotebookRuntime;
use crate::notebook::repository::NotebookRepository;
use crate::providers::decks::OfficialDeckProvider;
use crate::services::decks::{
    ClassificationRunView, DeckService, ReclassificationPriority, ReclassificationProgress,
    ReclassificationService,
};

pub struct DeckEnrichmentRuntime {
    pub provider: OfficialDeckProvider,
    pub assets: AssetRegistry,
    pub reclassification: ReclassificationService,
    pub reclassification_priority: ReclassificationPriority,
}

impl DeckEnrichmentRuntime {
    pub fn builtin() -> Result<Self, crate::domain::RepoError> {
        let reclassification_priority = ReclassificationPriority::default();
        Ok(Self {
            provider: OfficialDeckProvider::default(),
            assets: AssetRegistry::builtin()?,
            reclassification: ReclassificationService::new(reclassification_priority.clone()),
            reclassification_priority,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationRequest {
    pub deck_revision_id: String,
}

pub fn get_classification_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &DeckEnrichmentRuntime,
    request: ClassificationRequest,
) -> CommandResult<Vec<ClassificationRunView>> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let result = EntityId::parse(request.deck_revision_id).and_then(|revision_id| {
        DeckService::new(repository, &runtime.assets).get_classification(&revision_id)
    });
    match result {
        Ok(runs) => CommandResult::success(runs, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

pub fn start_reclassification_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &DeckEnrichmentRuntime,
) -> CommandResult<ReclassificationProgress> {
    if let Err(error) = caller.require(&[CallerIdentity::Main]) {
        return CommandResult::failure(error);
    }
    let result = runtime
        .assets
        .current()
        .and_then(|assets| runtime.reclassification.start(repository, &assets));
    match result {
        Ok(progress) => CommandResult::success(progress, 1),
        Err(error) => CommandResult::failure(error.to_app_error()),
    }
}

#[tauri::command]
pub fn get_classification(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, DeckEnrichmentRuntime>,
    request: ClassificationRequest,
) -> CommandResult<Vec<ClassificationRunView>> {
    panic_boundary("get-classification-command", || {
        with_caller(&window, |caller| {
            get_classification_for(caller, &notebook.repository, &runtime, request)
        })
    })
}

#[tauri::command]
pub fn start_reclassification(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, DeckEnrichmentRuntime>,
) -> CommandResult<ReclassificationProgress> {
    panic_boundary("start-reclassification-command", || {
        with_caller(&window, |caller| {
            start_reclassification_for(caller, &notebook.repository, &runtime)
        })
    })
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
