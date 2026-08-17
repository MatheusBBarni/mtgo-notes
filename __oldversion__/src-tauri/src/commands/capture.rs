use std::sync::Mutex;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{Emitter, Manager};
use zeroize::Zeroizing;

use crate::commands::notes::require_idempotency_key;
use crate::domain::{EntityId, RepoError, Revision, UtcMillis};
use crate::ipc::{
    AppError, CallerIdentity, CommandResult, ErrorCode, EventName, ReplacementEvent,
    next_event_revision, panic_boundary,
};
use crate::notebook::key::DatabaseKey;
use crate::notebook::{NotebookRuntime, repository::NotebookRepository};

const DRAFT_VERSION: u8 = 1;
const NONCE_BYTES: usize = 12;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureDraftView {
    pub encounter_id: String,
    pub window_instance: String,
    pub text: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCaptureRequest {
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardDraftRequest {
    pub encounter_id: String,
    pub window_instance: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureClaim {
    encounter_id: EntityId,
    window_instance: String,
}

#[derive(Default)]
pub struct CaptureRuntime {
    claim: Mutex<Option<CaptureClaim>>,
}

impl CaptureRuntime {
    fn release(&self, encounter_id: &EntityId, window_instance: &str) {
        if let Ok(mut claim) = self.claim.lock()
            && claim.as_ref().is_some_and(|claim| {
                &claim.encounter_id == encounter_id && claim.window_instance == window_instance
            })
        {
            *claim = None;
        }
    }

    pub fn preserve(
        &self,
        repository: &NotebookRepository,
        key: &DatabaseKey,
        encounter_id: &EntityId,
        text: &str,
    ) -> Result<Revision, RepoError> {
        let claim = self
            .claim
            .lock()
            .map_err(|_| RepoError::SaveFailed)?
            .clone()
            .ok_or(RepoError::NoActiveEncounter)?;
        if &claim.encounter_id != encounter_id {
            return Err(RepoError::InvalidTransition);
        }
        let encrypted = DraftCipher::new(key).encrypt(text)?;
        let revision = repository.upsert_capture_draft(
            encounter_id,
            &encrypted,
            UtcMillis::now(),
            &claim.window_instance,
        )?;
        if repository
            .active_encounter()?
            .as_ref()
            .is_none_or(|active| active.id != encounter_id.as_str())
        {
            return Err(RepoError::CandidateStale);
        }
        Ok(revision)
    }

    pub fn complete(
        &self,
        repository: &NotebookRepository,
        encounter_id: &EntityId,
    ) -> Result<(), RepoError> {
        repository.delete_capture_draft(encounter_id)
    }
}

pub fn open_capture_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    key: &DatabaseKey,
    runtime: &CaptureRuntime,
    request: OpenCaptureRequest,
) -> CommandResult<CaptureDraftView> {
    if let Err(error) = caller.require(&[CallerIdentity::Main, CallerIdentity::Overlay]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let active = match repository.active_encounter() {
        Ok(Some(active)) => active,
        Ok(None) => return CommandResult::failure(RepoError::NoActiveEncounter.to_app_error()),
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let encounter_id = match EntityId::parse(&active.id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let mut claim = match runtime.claim.lock() {
        Ok(claim) => claim,
        Err(_) => return CommandResult::failure(RepoError::AlreadyOpen.to_app_error()),
    };
    if let Some(existing) = claim.as_ref() {
        return CommandResult::failure(AppError::new(
            ErrorCode::AlreadyOpen,
            format!(
                "Quick capture is already open in window instance {}.",
                existing.window_instance
            ),
            false,
        ));
    }
    let window_instance = EntityId::new().to_string();
    *claim = Some(CaptureClaim {
        encounter_id: encounter_id.clone(),
        window_instance: window_instance.clone(),
    });
    drop(claim);
    let recovered = match repository.capture_draft(&encounter_id) {
        Ok(Some((encrypted, _, revision))) => match DraftCipher::new(key).decrypt(&encrypted) {
            Ok(text) => (text, revision.get()),
            Err(error) => {
                runtime.release(&encounter_id, &window_instance);
                return CommandResult::failure(error.to_app_error());
            }
        },
        Ok(None) => (String::new(), 1),
        Err(error) => {
            runtime.release(&encounter_id, &window_instance);
            return CommandResult::failure(error.to_app_error());
        }
    };
    CommandResult::success(
        CaptureDraftView {
            encounter_id: encounter_id.to_string(),
            window_instance,
            text: recovered.0,
            revision: recovered.1,
        },
        recovered.1,
    )
}

pub fn discard_draft_for(
    caller: CallerIdentity,
    repository: &NotebookRepository,
    runtime: &CaptureRuntime,
    request: DiscardDraftRequest,
) -> CommandResult<CaptureDraftView> {
    if let Err(error) = caller.require(&[CallerIdentity::Capture]) {
        return CommandResult::failure(error);
    }
    if let Err(error) = require_idempotency_key(&request.idempotency_key) {
        return CommandResult::failure(error);
    }
    let encounter_id = match EntityId::parse(request.encounter_id) {
        Ok(id) => id,
        Err(error) => return CommandResult::failure(error.to_app_error()),
    };
    let mut claim = match runtime.claim.lock() {
        Ok(claim) => claim,
        Err(_) => return CommandResult::failure(RepoError::SaveFailed.to_app_error()),
    };
    let matches = claim.as_ref().is_some_and(|claim| {
        claim.encounter_id == encounter_id && claim.window_instance == request.window_instance
    });
    if !matches {
        return CommandResult::failure(RepoError::CandidateStale.to_app_error());
    }
    if let Err(error) = repository.delete_capture_draft(&encounter_id) {
        return CommandResult::failure(error.to_app_error());
    }
    *claim = None;
    CommandResult::success(
        CaptureDraftView {
            encounter_id: encounter_id.to_string(),
            window_instance: request.window_instance,
            text: String::new(),
            revision: 1,
        },
        1,
    )
}

#[tauri::command]
pub fn open_capture(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, CaptureRuntime>,
    request: OpenCaptureRequest,
) -> CommandResult<CaptureDraftView> {
    panic_boundary("open-capture-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        let result = open_capture_for(
            caller,
            &notebook.repository,
            &notebook.key,
            runtime.inner(),
            request,
        );
        if let CommandResult::Success { data, .. } = &result
            && let Some(capture) = window.app_handle().get_webview_window("capture")
        {
            let _ = capture.emit(
                "capture://draft-v1",
                ReplacementEvent::v1(EventName::CaptureDraft, next_event_revision(), data.clone()),
            );
            let _ = capture.show();
            let _ = capture.set_ignore_cursor_events(false);
            let _ = capture.set_focus();
        } else if matches!(
            &result,
            CommandResult::Failure {
                error: AppError {
                    code: ErrorCode::AlreadyOpen,
                    ..
                },
                ..
            }
        ) && let Some(capture) = window.app_handle().get_webview_window("capture")
        {
            let _ = capture.show();
            let _ = capture.set_focus();
        }
        result
    })
}

#[tauri::command]
pub fn discard_draft(
    window: tauri::WebviewWindow,
    notebook: tauri::State<'_, NotebookRuntime>,
    runtime: tauri::State<'_, CaptureRuntime>,
    request: DiscardDraftRequest,
) -> CommandResult<CaptureDraftView> {
    panic_boundary("discard-draft-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        let result = discard_draft_for(caller, &notebook.repository, runtime.inner(), request);
        if result.is_success() {
            let _ = window.hide();
        }
        result
    })
}

struct DraftCipher {
    cipher: ChaCha20Poly1305,
}

impl DraftCipher {
    fn new(database_key: &DatabaseKey) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"mtgo-notes-capture-draft-v1");
        hasher.update(database_key.expose());
        let key = hasher.finalize();
        Self {
            cipher: ChaCha20Poly1305::new_from_slice(&key)
                .expect("SHA-256 output is a valid ChaCha20 key"),
        }
    }

    fn encrypt(&self, text: &str) -> Result<Vec<u8>, RepoError> {
        let mut nonce = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| RepoError::SaveFailed)?;
        let nonce_value = Nonce::from(nonce);
        let encrypted = self
            .cipher
            .encrypt(&nonce_value, text.as_bytes())
            .map_err(|_| RepoError::SaveFailed)?;
        let mut output = Vec::with_capacity(1 + NONCE_BYTES + encrypted.len());
        output.push(DRAFT_VERSION);
        output.extend_from_slice(&nonce);
        output.extend_from_slice(&encrypted);
        Ok(output)
    }

    fn decrypt(&self, value: &[u8]) -> Result<String, RepoError> {
        if value.len() <= 1 + NONCE_BYTES || value[0] != DRAFT_VERSION {
            return Err(RepoError::SaveFailed);
        }
        let nonce_bytes: [u8; NONCE_BYTES] = value[1..1 + NONCE_BYTES]
            .try_into()
            .map_err(|_| RepoError::SaveFailed)?;
        let nonce = Nonce::from(nonce_bytes);
        let plaintext = Zeroizing::new(
            self.cipher
                .decrypt(&nonce, &value[1 + NONCE_BYTES..])
                .map_err(|_| RepoError::SaveFailed)?,
        );
        String::from_utf8(plaintext.to_vec()).map_err(|_| RepoError::SaveFailed)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::notebook::migrations::MigrationManager;
    use crate::services::profiles::ProfileService;

    #[test]
    fn encrypted_draft_round_trip_contains_no_plaintext() {
        let key = DatabaseKey::generate().expect("key");
        let cipher = DraftCipher::new(&key);
        let plaintext = "preserve this private note";
        let encrypted = cipher.encrypt(plaintext).expect("encrypt");
        assert!(!String::from_utf8_lossy(&encrypted).contains(plaintext));
        assert_eq!(cipher.decrypt(&encrypted).expect("decrypt"), plaintext);
    }

    #[test]
    fn wrong_key_cannot_decrypt_draft() {
        let first = DatabaseKey::generate().expect("first");
        let second = DatabaseKey::generate().expect("second");
        let encrypted = DraftCipher::new(&first)
            .encrypt("private")
            .expect("encrypt");
        assert_eq!(
            DraftCipher::new(&second).decrypt(&encrypted),
            Err(RepoError::SaveFailed)
        );
    }

    #[test]
    fn ut_099_repeated_open_claims_only_one_capture_window_instance() {
        let directory = TempDir::new().expect("temp");
        let key = DatabaseKey::generate().expect("key");
        MigrationManager::default()
            .migrate(directory.path().join("notebook.db"), &key)
            .expect("migrate");
        let repository =
            NotebookRepository::open(directory.path().join("notebook.db"), &key).expect("open");
        let profile = ProfileService::new(&repository)
            .create("CaptureOpponent")
            .expect("profile");
        repository
            .start_encounter(&EntityId::new(), &profile.profile.id, UtcMillis::now(), 1)
            .expect("encounter");
        let runtime = CaptureRuntime::default();

        let first = open_capture_for(
            CallerIdentity::Overlay,
            &repository,
            &key,
            &runtime,
            OpenCaptureRequest {
                idempotency_key: EntityId::new().to_string(),
            },
        );
        assert!(first.is_success());
        let second = open_capture_for(
            CallerIdentity::Main,
            &repository,
            &key,
            &runtime,
            OpenCaptureRequest {
                idempotency_key: EntityId::new().to_string(),
            },
        );
        assert!(matches!(
            second,
            CommandResult::Failure {
                error: AppError {
                    code: ErrorCode::AlreadyOpen,
                    ..
                },
                ..
            }
        ));
    }
}
