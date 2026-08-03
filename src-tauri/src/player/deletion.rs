//! Player-owned deletion with explicit, revision-bound confirmation.
//!
//! This is deliberately separate from opponent reversible deletion.  The only
//! durable remnants are content-free Player tombstones used to prevent a later
//! archive merge from resurrecting deleted identity subtrees.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::{InternalPhase, RepoError, Revision, UtcMillis};
use crate::ipc::CallerIdentity;
use crate::notebook::repository::NotebookRepository;

use super::models::{
    PlayerEmptyOutcomeId, PlayerEvidenceId, PlayerId, PlayerPreviewToken, canonical_digest,
};
use super::runtime::{
    PlayerCommandKind, PlayerError, PlayerErrorCode, PlayerPublicResultsRuntime, PlayerRecovery,
    authorize_command,
};

const DELETION_TTL_MILLIS: i64 = 15 * 60 * 1_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerDeletionTarget {
    Identity(PlayerId),
    Evidence(PlayerEvidenceId),
    EmptyOutcome(PlayerEmptyOutcomeId),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDeletionCounts {
    pub evidence: u64,
    pub cards: u64,
    pub selections: u64,
    pub classifications: u64,
    pub empty_outcomes: u64,
    pub consents: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDeletionPreview {
    pub token: PlayerPreviewToken,
    pub target: PlayerDeletionTarget,
    pub player_identity_id: PlayerId,
    pub identity_revision: Revision,
    pub counts: PlayerDeletionCounts,
    pub digest: String,
    pub expires_at: UtcMillis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDeletionOutcome {
    pub target: PlayerDeletionTarget,
    pub player_identity_id: PlayerId,
    pub deleted: bool,
    pub tombstones_written: u64,
}

pub struct PlayerDeletionService<'a> {
    repository: &'a NotebookRepository,
    runtime: &'a PlayerPublicResultsRuntime,
    previews: Mutex<BTreeMap<PlayerPreviewToken, PlayerDeletionPreview>>,
}

impl<'a> PlayerDeletionService<'a> {
    pub fn new(
        repository: &'a NotebookRepository,
        runtime: &'a PlayerPublicResultsRuntime,
    ) -> Self {
        Self {
            repository,
            runtime,
            previews: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn preview(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        payload_bytes: usize,
        target: PlayerDeletionTarget,
        now: UtcMillis,
    ) -> Result<PlayerDeletionPreview, PlayerError> {
        authorize_command(caller, phase, PlayerCommandKind::Delete, payload_bytes)?;
        let preview = self
            .build_preview(target, now)
            .map_err(|_| PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::None))?;
        self.previews
            .lock()
            .map_err(|_| {
                PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
            })?
            .insert(preview.token.clone(), preview.clone());
        Ok(preview)
    }

    pub fn confirm(
        &self,
        caller: CallerIdentity,
        phase: InternalPhase,
        payload_bytes: usize,
        token: &PlayerPreviewToken,
        digest: &str,
        now: UtcMillis,
    ) -> Result<PlayerDeletionOutcome, PlayerError> {
        authorize_command(caller, phase, PlayerCommandKind::Delete, payload_bytes)?;
        let preview = self
            .previews
            .lock()
            .map_err(|_| {
                PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
            })?
            .get(token)
            .cloned()
            .ok_or_else(|| {
                PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::None)
            })?;
        if now >= preview.expires_at || digest != preview.digest {
            return Err(PlayerError::new(
                PlayerErrorCode::DeletionPreviewStale,
                PlayerRecovery::Retry,
            ));
        }
        let current = self
            .build_preview(preview.target.clone(), now)
            .map_err(|_| {
                PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::Retry)
            })?;
        if current.digest != preview.digest
            || current.identity_revision != preview.identity_revision
        {
            return Err(PlayerError::new(
                PlayerErrorCode::DeletionPreviewStale,
                PlayerRecovery::Retry,
            ));
        }
        let outcome = self.delete(&preview, now).map_err(|_| {
            PlayerError::new(PlayerErrorCode::InvalidRequest, PlayerRecovery::Retry)
        })?;
        self.previews
            .lock()
            .map_err(|_| {
                PlayerError::new(PlayerErrorCode::ProviderUnavailable, PlayerRecovery::Retry)
            })?
            .remove(token);
        if matches!(outcome.target, PlayerDeletionTarget::Identity(_)) {
            self.runtime.reset_disabled()?;
        }
        Ok(outcome)
    }

    fn build_preview(
        &self,
        target: PlayerDeletionTarget,
        now: UtcMillis,
    ) -> Result<PlayerDeletionPreview, RepoError> {
        let (player_identity_id, identity_revision, counts) =
            self.repository
                .with_connection(|connection| match &target {
                    PlayerDeletionTarget::Identity(player_id) => {
                        let row = connection
                            .connection
                            .query_row(
                                "SELECT id, revision FROM player_identities WHERE id = ?1",
                                [player_id.as_str()],
                                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                            )
                            .map_err(|_| RepoError::NotFound)?;
                        Ok((
                            PlayerId::parse(row.0)?,
                            Revision::new(
                                u64::try_from(row.1).map_err(|_| RepoError::NotebookInvalid)?,
                            )?,
                            counts_for_identity(&connection.connection, player_id)?,
                        ))
                    }
                    PlayerDeletionTarget::Evidence(evidence_id) => {
                        let row = connection
                            .connection
                            .query_row(
                                "SELECT player_identity_id FROM player_evidence WHERE id = ?1",
                                [evidence_id.as_str()],
                                |row| row.get::<_, String>(0),
                            )
                            .map_err(|_| RepoError::NotFound)?;
                        let identity = PlayerId::parse(row)?;
                        let revision = identity_revision(&connection.connection, &identity)?;
                        Ok((
                            identity,
                            revision,
                            counts_for_evidence(&connection.connection, evidence_id)?,
                        ))
                    }
                    PlayerDeletionTarget::EmptyOutcome(empty_id) => {
                        let row = connection.connection.query_row(
                        "SELECT player_identity_id FROM player_empty_outcomes WHERE id = ?1",
                        [empty_id.as_str()],
                        |row| row.get::<_, String>(0),
                    ).map_err(|_| RepoError::NotFound)?;
                        let identity = PlayerId::parse(row)?;
                        let revision = identity_revision(&connection.connection, &identity)?;
                        Ok((
                            identity,
                            revision,
                            PlayerDeletionCounts {
                                empty_outcomes: 1,
                                ..PlayerDeletionCounts::default()
                            },
                        ))
                    }
                })?;
        let digest = canonical_digest(&json!({
            "target": target,
            "playerIdentityId": player_identity_id,
            "identityRevision": identity_revision,
            "counts": counts,
        }))?;
        Ok(PlayerDeletionPreview {
            token: PlayerPreviewToken::new(),
            target,
            player_identity_id,
            identity_revision,
            counts,
            digest,
            expires_at: UtcMillis::new(
                now.get()
                    .checked_add(DELETION_TTL_MILLIS)
                    .ok_or(RepoError::InvalidRequest)?,
            )?,
        })
    }

    fn delete(
        &self,
        preview: &PlayerDeletionPreview,
        now: UtcMillis,
    ) -> Result<PlayerDeletionOutcome, RepoError> {
        self.repository.transact_domain(|transaction| {
            let mut tombstones = Vec::new();
            match &preview.target {
                PlayerDeletionTarget::Identity(player_id) => {
                    let mut evidence = transaction.prepare("SELECT id FROM player_evidence WHERE player_identity_id = ?1 ORDER BY id").map_err(|_| RepoError::NotebookInvalid)?;
                    let ids = evidence.query_map([player_id.as_str()], |row| row.get::<_, String>(0)).map_err(|_| RepoError::NotebookInvalid)?.collect::<Result<Vec<_>, _>>().map_err(|_| RepoError::NotebookInvalid)?;
                    for id in ids { tombstones.push(("player_evidence", id)); }
                    let mut empty = transaction.prepare("SELECT id FROM player_empty_outcomes WHERE player_identity_id = ?1 ORDER BY id").map_err(|_| RepoError::NotebookInvalid)?;
                    let ids = empty.query_map([player_id.as_str()], |row| row.get::<_, String>(0)).map_err(|_| RepoError::NotebookInvalid)?.collect::<Result<Vec<_>, _>>().map_err(|_| RepoError::NotebookInvalid)?;
                    for id in ids { tombstones.push(("player_empty_outcome", id)); }
                    tombstones.push(("player_identity", player_id.as_str().to_owned()));
                    transaction.execute("UPDATE player_evidence SET supersedes_evidence_id = NULL WHERE player_identity_id = ?1", [player_id.as_str()]).map_err(|_| RepoError::NotebookInvalid)?;
                    transaction.execute("DELETE FROM player_identities WHERE id = ?1", [player_id.as_str()]).map_err(|_| RepoError::NotebookInvalid)?;
                }
                PlayerDeletionTarget::Evidence(evidence_id) => {
                    tombstones.push(("player_evidence", evidence_id.as_str().to_owned()));
                    transaction.execute("UPDATE player_evidence SET supersedes_evidence_id = NULL WHERE id = ?1", [evidence_id.as_str()]).map_err(|_| RepoError::NotebookInvalid)?;
                    transaction.execute("DELETE FROM player_evidence WHERE id = ?1", [evidence_id.as_str()]).map_err(|_| RepoError::NotebookInvalid)?;
                }
                PlayerDeletionTarget::EmptyOutcome(empty_id) => {
                    tombstones.push(("player_empty_outcome", empty_id.as_str().to_owned()));
                    transaction.execute("DELETE FROM player_empty_outcomes WHERE id = ?1", [empty_id.as_str()]).map_err(|_| RepoError::NotebookInvalid)?;
                }
            }
            for (kind, id) in &tombstones {
                transaction.execute(
                    "INSERT OR IGNORE INTO player_tombstones(entity_kind, entity_id, player_identity_id, deleted_at) VALUES (?1, ?2, ?3, ?4)",
                    (kind, id, preview.player_identity_id.as_str(), now.get()),
                ).map_err(|_| RepoError::NotebookInvalid)?;
            }
            Ok(PlayerDeletionOutcome {
                target: preview.target.clone(),
                player_identity_id: preview.player_identity_id.clone(),
                deleted: true,
                tombstones_written: tombstones.len() as u64,
            })
        })
    }
}

fn identity_revision(
    connection: &rusqlite::Connection,
    id: &PlayerId,
) -> Result<Revision, RepoError> {
    connection
        .query_row(
            "SELECT revision FROM player_identities WHERE id = ?1",
            [id.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| RepoError::NotFound)
        .and_then(|value| {
            Revision::new(u64::try_from(value).map_err(|_| RepoError::NotebookInvalid)?)
        })
}

fn counts_for_identity(
    connection: &rusqlite::Connection,
    id: &PlayerId,
) -> Result<PlayerDeletionCounts, RepoError> {
    let count = |sql: &str| -> Result<u64, RepoError> {
        connection
            .query_row(sql, [id.as_str()], |row| row.get::<_, i64>(0))
            .map_err(|_| RepoError::NotebookInvalid)
            .and_then(|value| u64::try_from(value).map_err(|_| RepoError::NotebookInvalid))
    };
    Ok(PlayerDeletionCounts {
        evidence: count("SELECT count(*) FROM player_evidence WHERE player_identity_id = ?1")?,
        cards: count(
            "SELECT count(*) FROM player_evidence_cards WHERE evidence_id IN (SELECT id FROM player_evidence WHERE player_identity_id = ?1)",
        )?,
        selections: count(
            "SELECT count(*) FROM player_selection_revisions WHERE evidence_id IN (SELECT id FROM player_evidence WHERE player_identity_id = ?1)",
        )?,
        classifications: count(
            "SELECT count(*) FROM player_classification_runs WHERE evidence_id IN (SELECT id FROM player_evidence WHERE player_identity_id = ?1)",
        )?,
        empty_outcomes: count(
            "SELECT count(*) FROM player_empty_outcomes WHERE player_identity_id = ?1",
        )?,
        consents: count(
            "SELECT count(*) FROM player_source_consents WHERE player_identity_id = ?1",
        )?,
    })
}

fn counts_for_evidence(
    connection: &rusqlite::Connection,
    id: &PlayerEvidenceId,
) -> Result<PlayerDeletionCounts, RepoError> {
    let count = |table: &str| -> Result<u64, RepoError> {
        let sql = format!("SELECT count(*) FROM {table} WHERE evidence_id = ?1");
        connection
            .query_row(&sql, [id.as_str()], |row| row.get::<_, i64>(0))
            .map_err(|_| RepoError::NotebookInvalid)
            .and_then(|value| u64::try_from(value).map_err(|_| RepoError::NotebookInvalid))
    };
    Ok(PlayerDeletionCounts {
        evidence: 1,
        cards: count("player_evidence_cards")?,
        selections: count("player_selection_revisions")?,
        classifications: count("player_classification_runs")?,
        ..PlayerDeletionCounts::default()
    })
}
