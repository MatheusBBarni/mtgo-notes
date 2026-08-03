//! Trusted host façade for the Task 03 evidence lifecycle.

use serde_json::json;

use crate::domain::{InternalPhase, Revision, UtcMillis};
use crate::ipc::CallerIdentity;

use super::evidence::{ManualEvidenceInput, ManualEvidencePreview, manual_preview};
use super::models::{
    PlayerId, PlayerOperationKey, PlayerSelectionRevision, PlayerSourceRoute, canonical_digest,
};
use super::repository::{AppendSelectionInput, ImportOutcome, PlayerStore, VerifiedImportBatch};
use super::runtime::{
    PlayerCommandKind, PlayerError, PlayerPublicResultsRuntime, authorize_command,
};

pub struct PlayerPublicResultsService<'a> {
    pub store: &'a PlayerStore<'a>,
    pub runtime: &'a PlayerPublicResultsRuntime,
}

impl<'a> PlayerPublicResultsService<'a> {
    pub fn new(store: &'a PlayerStore<'a>, runtime: &'a PlayerPublicResultsRuntime) -> Self {
        Self { store, runtime }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn manual_preview(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        payload_bytes: usize,
        player_identity_id: PlayerId,
        identity_revision: Revision,
        input: &ManualEvidenceInput,
        operation_key: PlayerOperationKey,
        now: UtcMillis,
    ) -> Result<ManualEvidencePreview, PlayerError> {
        authorize_command(
            caller,
            phase,
            PlayerCommandKind::ManualPreview,
            payload_bytes,
        )?;
        let preview = manual_preview(player_identity_id, identity_revision.get(), input, now)
            .map_err(|_| {
                PlayerError::new(
                    super::runtime::PlayerErrorCode::ManualEvidenceInvalid,
                    super::runtime::PlayerRecovery::None,
                )
            })?;
        self.runtime.bind_manual_preview(
            preview.token.clone(),
            preview.player_identity_id.clone(),
            identity_revision,
            preview.evidence.source_key.clone(),
            preview.evidence.source_digest.clone(),
            preview.evidence.preview_digest.clone(),
            operation_key,
            now,
        )?;
        Ok(preview)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn import_manual(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        payload_bytes: usize,
        preview: &ManualEvidencePreview,
        selected_fields: serde_json::Value,
        operation_key: PlayerOperationKey,
        now: UtcMillis,
    ) -> Result<ImportOutcome, PlayerError> {
        authorize_command(caller, phase, PlayerCommandKind::Import, payload_bytes)?;
        self.runtime.verify_manual_preview(
            &preview.token,
            &preview.player_identity_id,
            Revision::new(preview.identity_revision).map_err(|_| {
                PlayerError::new(
                    super::runtime::PlayerErrorCode::PreviewMismatch,
                    super::runtime::PlayerRecovery::Retry,
                )
            })?,
            &preview.evidence.source_key,
            &preview.evidence.source_digest,
            &preview.evidence.preview_digest,
            now,
        )?;
        let request_digest = canonical_digest(&json!({
            "command": "import_manual",
            "identityId": preview.player_identity_id,
            "identityRevision": preview.identity_revision,
            "token": preview.token,
            "sourceKey": preview.evidence.source_key,
            "sourceDigest": preview.evidence.source_digest,
            "previewDigest": preview.evidence.preview_digest,
            "selectedFields": selected_fields,
        }))
        .map_err(|_| {
            PlayerError::new(
                super::runtime::PlayerErrorCode::InvalidRequest,
                super::runtime::PlayerRecovery::None,
            )
        })?;
        let mut evidence = preview.evidence.clone();
        evidence.imported_at = now;
        self.store
            .import_batch(VerifiedImportBatch {
                operation_key,
                command_kind: "import_manual".into(),
                request_digest,
                cards: evidence.cards.clone(),
                evidence,
                selected_fields,
                now,
            })
            .map_err(|_| {
                PlayerError::new(
                    super::runtime::PlayerErrorCode::InvalidRequest,
                    super::runtime::PlayerRecovery::None,
                )
            })
    }

    pub fn update_selection(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        payload_bytes: usize,
        input: AppendSelectionInput,
    ) -> Result<PlayerSelectionRevision, PlayerError> {
        authorize_command(caller, phase, PlayerCommandKind::Selection, payload_bytes)?;
        self.store.append_selection(input).map_err(|_| {
            PlayerError::new(
                super::runtime::PlayerErrorCode::InvalidRequest,
                super::runtime::PlayerRecovery::None,
            )
        })
    }

    pub fn source_status(
        &self,
        caller: CallerIdentity,
        route: PlayerSourceRoute,
    ) -> Result<super::runtime::PlayerProviderStatus, PlayerError> {
        authorize_command(caller, InternalPhase::Idle, PlayerCommandKind::Status, 0)?;
        self.runtime.consent_status(route)
    }
}
