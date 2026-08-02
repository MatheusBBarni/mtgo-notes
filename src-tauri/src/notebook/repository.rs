use std::path::Path;
use std::sync::Mutex;

use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::disclosure::{DisclosurePolicy, QueryKind};
use crate::domain::{EntityId, IdempotencyKey, InternalPhase, RepoError, Revision, UtcMillis};
use crate::notebook::connection::EncryptedConnection;
use crate::notebook::fts::{HistoryHit, Page, search};
use crate::notebook::key::DatabaseKey;
use crate::notebook::migrations::current_version;
use crate::operations::OperationRecord;

pub struct NotebookRepository {
    connection: Mutex<Option<EncryptedConnection>>,
    cursor_secret: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadSnapshot {
    pub schema_version: i64,
    pub profile_count: i64,
    pub encounter_count: i64,
    pub observation_count: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEncounterRecord {
    pub id: String,
    pub profile_id: String,
    pub phase: InternalPhase,
    pub generation: u64,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmedEncounterRecord {
    pub profile_id: EntityId,
    pub started_new: bool,
    pub replaced_encounter_id: Option<EntityId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrityOutcome {
    Clean,
    ValidAfterUncleanShutdown,
}

impl NotebookRepository {
    pub fn upsert_capture_draft(
        &self,
        encounter_id: &EntityId,
        encrypted_text: &[u8],
        updated_at: UtcMillis,
        claimed_window_instance: &str,
    ) -> Result<Revision, RepoError> {
        self.transact_domain(|transaction| {
            transaction
                .execute(
                    "INSERT INTO capture_drafts(
                        encounter_id, encrypted_text, updated_at, claimed_window_instance, revision
                     ) VALUES (?1, ?2, ?3, ?4, 1)
                     ON CONFLICT(encounter_id) DO UPDATE SET
                        encrypted_text = excluded.encrypted_text,
                        updated_at = excluded.updated_at,
                        claimed_window_instance = excluded.claimed_window_instance,
                        revision = capture_drafts.revision + 1",
                    params![
                        encounter_id.as_str(),
                        encrypted_text,
                        updated_at.get(),
                        claimed_window_instance
                    ],
                )
                .map_err(map_database_error)?;
            let revision = transaction
                .query_row(
                    "SELECT revision FROM capture_drafts WHERE encounter_id = ?1",
                    [encounter_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(map_database_error)?;
            Revision::new(u64::try_from(revision).map_err(|_| RepoError::NotebookInvalid)?)
        })
    }

    pub fn capture_draft(
        &self,
        encounter_id: &EntityId,
    ) -> Result<Option<(Vec<u8>, String, Revision)>, RepoError> {
        self.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT encrypted_text, claimed_window_instance, revision
                     FROM capture_drafts WHERE encounter_id = ?1",
                    [encounter_id.as_str()],
                    |row| {
                        let revision =
                            Revision::new(u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default())
                                .map_err(|_| rusqlite::Error::InvalidQuery)?;
                        Ok((row.get(0)?, row.get(1)?, revision))
                    },
                )
                .optional()
                .map_err(map_database_error)
        })
    }

    pub fn delete_capture_draft(&self, encounter_id: &EntityId) -> Result<(), RepoError> {
        self.transact_domain(|transaction| {
            transaction
                .execute(
                    "DELETE FROM capture_drafts WHERE encounter_id = ?1",
                    [encounter_id.as_str()],
                )
                .map_err(map_database_error)?;
            Ok(())
        })
    }

    pub fn active_encounter(&self) -> Result<Option<ActiveEncounterRecord>, RepoError> {
        self.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT id, profile_id, phase, generation, revision
                     FROM encounters
                     WHERE status = 'active' AND deleted_at IS NULL
                     LIMIT 1",
                    [],
                    |row| {
                        let phase = parse_phase(&row.get::<_, String>(2)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?;
                        Ok(ActiveEncounterRecord {
                            id: row.get(0)?,
                            profile_id: row.get(1)?,
                            phase,
                            generation: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                            revision: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        })
                    },
                )
                .optional()
                .map_err(map_database_error)
        })
    }

    pub fn correct_encounter_phase(
        &self,
        encounter_id: &EntityId,
        expected_revision: Revision,
        phase: InternalPhase,
        changed_at: UtcMillis,
        trigger: &str,
    ) -> Result<Revision, RepoError> {
        let expected =
            i64::try_from(expected_revision.get()).map_err(|_| RepoError::InvalidRequest)?;
        self.transact_domain(|transaction| {
            let (current_phase, current_revision) = transaction
                .query_row(
                    "SELECT phase, revision FROM encounters
                     WHERE id = ?1 AND status = 'active' AND deleted_at IS NULL",
                    [encounter_id.as_str()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(map_database_error)?
                .ok_or(RepoError::InvalidTransition)?;
            if current_revision != expected {
                return Err(RepoError::RevisionConflict);
            }
            let next_phase = phase_name(phase);
            if current_phase == next_phase {
                return Revision::new(u64::try_from(current_revision).unwrap_or_default());
            }
            transaction
                .execute(
                    "UPDATE encounters SET phase = ?1, revision = revision + 1 WHERE id = ?2",
                    params![next_phase, encounter_id.as_str()],
                )
                .map_err(map_database_error)?;
            transaction
                .execute(
                    "INSERT INTO encounter_transitions(
                        id, encounter_id, sequence, from_phase, to_phase, trigger,
                        confidence_class, created_at
                     ) VALUES (
                        ?1, ?2,
                        coalesce((SELECT max(sequence) + 1 FROM encounter_transitions WHERE encounter_id = ?2), 0),
                        ?3, ?4, ?5, 'player_confirmed', ?6
                     )",
                    params![
                        EntityId::new().as_str(),
                        encounter_id.as_str(),
                        current_phase,
                        next_phase,
                        trigger,
                        changed_at.get()
                    ],
                )
                .map_err(map_database_error)?;
            Revision::new(
                u64::try_from(current_revision + 1).map_err(|_| RepoError::NotebookInvalid)?,
            )
        })
    }

    pub fn undo_encounter_replacement(
        &self,
        undo_group_id: &EntityId,
    ) -> Result<ActiveEncounterRecord, RepoError> {
        self.transact_domain(|transaction| {
            let mut statement = transaction
                .prepare(
                    "SELECT encounter_id, from_phase, to_phase
                     FROM encounter_transitions
                     WHERE undo_group_id = ?1
                     ORDER BY created_at, sequence, id",
                )
                .map_err(map_database_error)?;
            let transitions = statement
                .query_map([undo_group_id.as_str()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(map_database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_database_error)?;
            drop(statement);
            if transitions.len() != 2 {
                return Err(RepoError::InvalidTransition);
            }
            let previous = transitions
                .iter()
                .find(|(_, _, to)| to == "finished")
                .ok_or(RepoError::InvalidTransition)?;
            let current = transitions
                .iter()
                .find(|(_, from, to)| from == "idle" && to == "pre_match")
                .ok_or(RepoError::InvalidTransition)?;
            let current_has_data = transaction
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM observations WHERE encounter_id = ?1 AND deleted_at IS NULL
                        UNION ALL
                        SELECT 1 FROM public_snapshots WHERE encounter_id = ?1
                    )",
                    [&current.0],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(map_database_error)?;
            if current_has_data == 1 {
                return Err(RepoError::InvalidTransition);
            }
            transaction
                .execute("DELETE FROM encounters WHERE id = ?1", [&current.0])
                .map_err(map_database_error)?;
            transaction
                .execute(
                    "UPDATE encounters
                     SET ended_at = NULL, status = 'active', phase = ?1, revision = revision + 1
                     WHERE id = ?2 AND status = 'finished'",
                    params![previous.1, previous.0],
                )
                .map_err(map_database_error)?;
            transaction
                .execute(
                    "DELETE FROM encounter_transitions WHERE undo_group_id = ?1",
                    [undo_group_id.as_str()],
                )
                .map_err(map_database_error)?;
            transaction
                .query_row(
                    "SELECT id, profile_id, phase, generation, revision FROM encounters WHERE id = ?1",
                    [&previous.0],
                    |row| {
                        let phase = parse_phase(&row.get::<_, String>(2)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?;
                        Ok(ActiveEncounterRecord {
                            id: row.get(0)?,
                            profile_id: row.get(1)?,
                            phase,
                            generation: u64::try_from(row.get::<_, i64>(3)?)
                                .unwrap_or_default(),
                            revision: u64::try_from(row.get::<_, i64>(4)?)
                                .unwrap_or_default(),
                        })
                    },
                )
                .map_err(map_database_error)
        })
    }
    pub fn set_provider_consent(
        &self,
        provider_id: &str,
        granted: bool,
        disclosed_fields_json: &str,
        changed_at: UtcMillis,
    ) -> Result<(), RepoError> {
        self.transact_domain(|transaction| {
            transaction
                .execute(
                    "INSERT INTO provider_consents(
                        provider_id, version, granted_at, revoked_at, disclosed_fields_json
                     ) VALUES (?1, 1, ?2, ?3, ?4)
                     ON CONFLICT(provider_id) DO UPDATE SET
                       version = provider_consents.version + 1,
                       granted_at = excluded.granted_at,
                       revoked_at = excluded.revoked_at,
                       disclosed_fields_json = excluded.disclosed_fields_json",
                    params![
                        provider_id,
                        granted.then_some(changed_at.get()),
                        (!granted).then_some(changed_at.get()),
                        disclosed_fields_json
                    ],
                )
                .map_err(map_database_error)?;
            Ok(())
        })
    }

    pub fn provider_consent(&self, provider_id: &str) -> Result<Option<(bool, String)>, RepoError> {
        self.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT granted_at IS NOT NULL AND revoked_at IS NULL,
                            disclosed_fields_json
                     FROM provider_consents WHERE provider_id = ?1",
                    [provider_id],
                    |row| Ok((row.get::<_, i64>(0)? == 1, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(map_database_error)
        })
    }

    pub fn open(path: impl AsRef<Path>, key: &DatabaseKey) -> Result<Self, RepoError> {
        let connection = EncryptedConnection::open(path, key)?;
        let mut cursor_secret = [0_u8; 32];
        getrandom::fill(&mut cursor_secret).map_err(|_| RepoError::NotebookInvalid)?;
        Ok(Self {
            connection: Mutex::new(Some(connection)),
            cursor_secret,
        })
    }

    pub fn begin_runtime(&self) -> Result<IntegrityOutcome, RepoError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let connection = connection.as_ref().ok_or(RepoError::NotebookInvalid)?;
        let clean = connection
            .connection
            .query_row(
                "SELECT clean_shutdown FROM runtime_state WHERE singleton = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        let outcome = if clean == 1 {
            IntegrityOutcome::Clean
        } else {
            connection.integrity_check()?;
            IntegrityOutcome::ValidAfterUncleanShutdown
        };
        connection
            .connection
            .execute(
                "UPDATE runtime_state
                 SET clean_shutdown = 0, last_integrity_at = ?1
                 WHERE singleton = 1",
                [UtcMillis::now().get()],
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        Ok(outcome)
    }

    pub fn mark_clean_shutdown(&self) -> Result<(), RepoError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let connection = connection.as_ref().ok_or(RepoError::NotebookInvalid)?;
        connection
            .connection
            .execute(
                "UPDATE runtime_state SET clean_shutdown = 1 WHERE singleton = 1",
                [],
            )
            .map_err(|_| RepoError::NotebookInvalid)?;
        Ok(())
    }

    pub fn create_profile(
        &self,
        id: &EntityId,
        primary_handle: &str,
        normalized_handle: &str,
        created_at: UtcMillis,
    ) -> Result<(), RepoError> {
        self.transact(|transaction| {
            let conflict = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM opponent_profiles
                    WHERE normalized_handle = ?1 AND deleted_at IS NULL
                    UNION ALL
                    SELECT 1 FROM opponent_aliases alias
                    JOIN opponent_profiles profile ON profile.id = alias.profile_id
                    WHERE alias.normalized_handle = ?1 AND profile.deleted_at IS NULL
                )",
                [normalized_handle],
                |row| row.get::<_, i64>(0),
            )?;
            if conflict == 1 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            transaction.execute(
                "INSERT INTO opponent_profiles(
                    id, primary_handle, normalized_handle, created_at, revision
                 ) VALUES (?1, ?2, ?3, ?4, 1)",
                params![
                    id.as_str(),
                    primary_handle,
                    normalized_handle,
                    created_at.get()
                ],
            )?;
            Ok(())
        })
        .map_err(|error| {
            if error == RepoError::InvalidRequest {
                RepoError::IdentityConflict
            } else {
                error
            }
        })
    }

    pub fn add_alias(
        &self,
        id: &EntityId,
        profile_id: &EntityId,
        display_handle: &str,
        normalized_handle: &str,
        created_at: UtcMillis,
    ) -> Result<(), RepoError> {
        self.transact(|transaction| {
            let conflict = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM opponent_profiles
                    WHERE normalized_handle = ?1 AND deleted_at IS NULL
                    UNION ALL
                    SELECT 1 FROM opponent_aliases alias
                    JOIN opponent_profiles profile ON profile.id = alias.profile_id
                    WHERE alias.normalized_handle = ?1
                      AND alias.profile_id <> ?2
                      AND profile.deleted_at IS NULL
                )",
                params![normalized_handle, profile_id.as_str()],
                |row| row.get::<_, i64>(0),
            )?;
            if conflict == 1 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            transaction.execute(
                "INSERT INTO opponent_aliases(
                    id, profile_id, display_handle, normalized_handle, provenance, created_at
                 ) VALUES (?1, ?2, ?3, ?4, 'player_confirmed', ?5)",
                params![
                    id.as_str(),
                    profile_id.as_str(),
                    display_handle,
                    normalized_handle,
                    created_at.get()
                ],
            )?;
            Ok(())
        })
        .map_err(|error| {
            if error == RepoError::InvalidRequest {
                RepoError::IdentityConflict
            } else {
                error
            }
        })
    }

    pub fn start_encounter(
        &self,
        encounter_id: &EntityId,
        profile_id: &EntityId,
        started_at: UtcMillis,
        generation: u64,
    ) -> Result<(), RepoError> {
        self.start_encounter_with_source(encounter_id, profile_id, started_at, generation, "manual")
    }

    pub fn start_encounter_with_source(
        &self,
        encounter_id: &EntityId,
        profile_id: &EntityId,
        started_at: UtcMillis,
        generation: u64,
        source: &str,
    ) -> Result<(), RepoError> {
        let generation = i64::try_from(generation).map_err(|_| RepoError::InvalidRequest)?;
        if !matches!(source, "manual" | "uia" | "ocr") {
            return Err(RepoError::InvalidRequest);
        }
        self.transact(|transaction| {
            transaction.execute(
                "INSERT INTO encounters(
                    id, profile_id, format, started_at, status, phase,
                    source, generation, revision
                 ) VALUES (?1, ?2, 'Modern', ?3, 'active', 'pre_match', ?4, ?5, 1)",
                params![
                    encounter_id.as_str(),
                    profile_id.as_str(),
                    started_at.get(),
                    source,
                    generation,
                ],
            )?;
            Ok(())
        })
    }

    pub fn replace_active_encounter(
        &self,
        encounter_id: &EntityId,
        profile_id: &EntityId,
        started_at: UtcMillis,
        generation: u64,
        undo_group_id: &EntityId,
    ) -> Result<Option<EntityId>, RepoError> {
        self.replace_active_encounter_with_source(
            encounter_id,
            profile_id,
            started_at,
            generation,
            undo_group_id,
            "manual",
        )
    }

    pub fn replace_active_encounter_with_source(
        &self,
        encounter_id: &EntityId,
        profile_id: &EntityId,
        started_at: UtcMillis,
        generation: u64,
        undo_group_id: &EntityId,
        source: &str,
    ) -> Result<Option<EntityId>, RepoError> {
        let generation = i64::try_from(generation).map_err(|_| RepoError::InvalidRequest)?;
        if !matches!(source, "manual" | "uia" | "ocr") {
            return Err(RepoError::InvalidRequest);
        }
        self.transact_domain(|transaction| {
            let previous = transaction
                .query_row(
                    "SELECT id, phase FROM encounters
                     WHERE status = 'active' AND deleted_at IS NULL",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(map_database_error)?;

            if let Some((previous_id, previous_phase)) = previous.as_ref() {
                transaction
                    .execute(
                        "UPDATE encounters
                         SET ended_at = ?1, status = 'finished', phase = 'finished',
                             revision = revision + 1
                         WHERE id = ?2 AND status = 'active' AND deleted_at IS NULL",
                        params![started_at.get(), previous_id],
                    )
                    .map_err(map_database_error)?;
                transaction
                    .execute(
                        "INSERT INTO encounter_transitions(
                            id, encounter_id, sequence, from_phase, to_phase, trigger,
                            confidence_class, created_at, undo_group_id
                         ) VALUES (
                            ?1, ?2,
                            coalesce((
                                SELECT max(sequence) + 1 FROM encounter_transitions
                                WHERE encounter_id = ?2
                            ), 0),
                            ?3, 'finished', 'confirmed_new_opponent',
                            'player_confirmed', ?4, ?5
                         )",
                        params![
                            EntityId::new().as_str(),
                            previous_id,
                            previous_phase,
                            started_at.get(),
                            undo_group_id.as_str()
                        ],
                    )
                    .map_err(map_database_error)?;
            }

            transaction
                .execute(
                    "INSERT INTO encounters(
                        id, profile_id, format, started_at, status, phase,
                        source, generation, revision
                     ) VALUES (?1, ?2, 'Modern', ?3, 'active', 'pre_match', ?4, ?5, 1)",
                    params![
                        encounter_id.as_str(),
                        profile_id.as_str(),
                        started_at.get(),
                        source,
                        generation,
                    ],
                )
                .map_err(map_database_error)?;
            transaction
                .execute(
                    "INSERT INTO encounter_transitions(
                        id, encounter_id, sequence, from_phase, to_phase, trigger,
                        confidence_class, created_at, undo_group_id
                     ) VALUES (
                        ?1, ?2, 0, 'idle', 'pre_match', 'confirmed_opponent',
                        'player_confirmed', ?3, ?4
                     )",
                    params![
                        EntityId::new().as_str(),
                        encounter_id.as_str(),
                        started_at.get(),
                        undo_group_id.as_str()
                    ],
                )
                .map_err(map_database_error)?;

            previous
                .map(|(previous_id, _)| EntityId::parse(previous_id))
                .transpose()
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn confirm_opponent_encounter(
        &self,
        encounter_id: &EntityId,
        display_handle: &str,
        normalized_handle: &str,
        started_at: UtcMillis,
        generation: u64,
        undo_group_id: &EntityId,
        source: &str,
    ) -> Result<ConfirmedEncounterRecord, RepoError> {
        let generation = i64::try_from(generation).map_err(|_| RepoError::InvalidRequest)?;
        if !matches!(source, "manual" | "uia" | "ocr") {
            return Err(RepoError::InvalidRequest);
        }
        self.transact_domain(|transaction| {
            let existing_profile = transaction
                .query_row(
                    "SELECT id FROM opponent_profiles
                     WHERE normalized_handle = ?1 AND deleted_at IS NULL
                     UNION ALL
                     SELECT alias.profile_id FROM opponent_aliases alias
                     JOIN opponent_profiles profile ON profile.id = alias.profile_id
                     WHERE alias.normalized_handle = ?1 AND profile.deleted_at IS NULL
                     LIMIT 1",
                    [normalized_handle],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(map_database_error)?;
            let profile_id = match existing_profile {
                Some(id) => EntityId::parse(id)?,
                None => {
                    let id = EntityId::new();
                    transaction
                        .execute(
                            "INSERT INTO opponent_profiles(
                                id, primary_handle, normalized_handle, created_at, revision
                             ) VALUES (?1, ?2, ?3, ?4, 1)",
                            params![
                                id.as_str(),
                                display_handle,
                                normalized_handle,
                                started_at.get(),
                            ],
                        )
                        .map_err(map_database_error)?;
                    id
                }
            };
            let previous = transaction
                .query_row(
                    "SELECT id, profile_id, phase FROM encounters
                     WHERE status = 'active' AND deleted_at IS NULL",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(map_database_error)?;
            if previous
                .as_ref()
                .is_some_and(|(_, previous_profile, _)| previous_profile == profile_id.as_str())
            {
                return Ok(ConfirmedEncounterRecord {
                    profile_id,
                    started_new: false,
                    replaced_encounter_id: None,
                });
            }
            let replaced_encounter_id =
                if let Some((previous_id, _, previous_phase)) = previous.as_ref() {
                    transaction
                        .execute(
                            "UPDATE encounters
                         SET ended_at = ?1, status = 'finished', phase = 'finished',
                             revision = revision + 1
                         WHERE id = ?2 AND status = 'active' AND deleted_at IS NULL",
                            params![started_at.get(), previous_id],
                        )
                        .map_err(map_database_error)?;
                    transaction
                        .execute(
                            "INSERT INTO encounter_transitions(
                            id, encounter_id, sequence, from_phase, to_phase, trigger,
                            confidence_class, created_at, undo_group_id
                         ) VALUES (
                            ?1, ?2,
                            coalesce((
                                SELECT max(sequence) + 1 FROM encounter_transitions
                                WHERE encounter_id = ?2
                            ), 0),
                            ?3, 'finished', 'confirmed_new_opponent',
                            'player_confirmed', ?4, ?5
                         )",
                            params![
                                EntityId::new().as_str(),
                                previous_id,
                                previous_phase,
                                started_at.get(),
                                undo_group_id.as_str(),
                            ],
                        )
                        .map_err(map_database_error)?;
                    Some(EntityId::parse(previous_id)?)
                } else {
                    None
                };
            transaction
                .execute(
                    "INSERT INTO encounters(
                        id, profile_id, format, started_at, status, phase,
                        source, generation, revision
                     ) VALUES (?1, ?2, 'Modern', ?3, 'active', 'pre_match', ?4, ?5, 1)",
                    params![
                        encounter_id.as_str(),
                        profile_id.as_str(),
                        started_at.get(),
                        source,
                        generation,
                    ],
                )
                .map_err(map_database_error)?;
            transaction
                .execute(
                    "INSERT INTO encounter_transitions(
                        id, encounter_id, sequence, from_phase, to_phase, trigger,
                        confidence_class, created_at, undo_group_id
                     ) VALUES (
                        ?1, ?2, 0, 'idle', 'pre_match', 'confirmed_opponent',
                        'player_confirmed', ?3, ?4
                     )",
                    params![
                        EntityId::new().as_str(),
                        encounter_id.as_str(),
                        started_at.get(),
                        replaced_encounter_id
                            .as_ref()
                            .map(|_| undo_group_id.as_str()),
                    ],
                )
                .map_err(map_database_error)?;
            Ok(ConfirmedEncounterRecord {
                profile_id,
                started_new: true,
                replaced_encounter_id,
            })
        })
    }

    pub fn add_observation(
        &self,
        observation_id: &EntityId,
        encounter_id: &EntityId,
        text: &str,
        created_at: UtcMillis,
        searchable: bool,
    ) -> Result<(), RepoError> {
        self.transact(|transaction| {
            transaction.execute(
                "INSERT INTO observations(
                    id, encounter_id, text, created_at, revision, searchable
                 ) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                params![
                    observation_id.as_str(),
                    encounter_id.as_str(),
                    text,
                    created_at.get(),
                    i64::from(searchable)
                ],
            )?;
            Ok(())
        })
    }

    pub fn finish_encounter(
        &self,
        encounter_id: &EntityId,
        ended_at: UtcMillis,
    ) -> Result<(), RepoError> {
        self.transact_domain(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE encounters
                     SET ended_at = ?1, status = 'finished', phase = 'finished',
                         revision = revision + 1
                     WHERE id = ?2 AND status = 'active' AND deleted_at IS NULL",
                    params![ended_at.get(), encounter_id.as_str()],
                )
                .map_err(map_database_error)?;
            if changed == 0 {
                return Err(RepoError::InvalidTransition);
            }
            Ok(())
        })
    }

    pub fn mark_active_encounter_incomplete(
        &self,
        reason: &str,
        changed_at: UtcMillis,
    ) -> Result<Option<EntityId>, RepoError> {
        self.transact_domain(|transaction| {
            let active = transaction
                .query_row(
                    "SELECT id, phase FROM encounters
                     WHERE status = 'active' AND deleted_at IS NULL",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(map_database_error)?;
            let Some((encounter_id, phase)) = active else {
                return Ok(None);
            };
            transaction
                .execute(
                    "UPDATE encounters
                     SET status = 'incomplete', phase = 'incomplete', incomplete_reason = ?1,
                         revision = revision + 1
                     WHERE id = ?2 AND status = 'active'",
                    params![reason, encounter_id],
                )
                .map_err(map_database_error)?;
            transaction
                .execute(
                    "INSERT INTO encounter_transitions(
                        id, encounter_id, sequence, from_phase, to_phase, trigger,
                        confidence_class, created_at
                     ) VALUES (
                        ?1, ?2,
                        coalesce((SELECT max(sequence) + 1 FROM encounter_transitions WHERE encounter_id = ?2), 0),
                        ?3, 'incomplete', 'shutdown_without_confident_end', 'uncertain', ?4
                     )",
                    params![
                        EntityId::new().as_str(),
                        encounter_id,
                        phase,
                        changed_at.get()
                    ],
                )
                .map_err(map_database_error)?;
            EntityId::parse(encounter_id).map(Some)
        })
    }

    pub fn update_observation(
        &self,
        observation_id: &EntityId,
        expected_revision: Revision,
        text: &str,
        edited_at: UtcMillis,
    ) -> Result<Revision, RepoError> {
        let expected_revision =
            i64::try_from(expected_revision.get()).map_err(|_| RepoError::InvalidRequest)?;
        self.transact_domain(|transaction| {
            let current = transaction
                .query_row(
                    "SELECT revision, deleted_at FROM observations WHERE id = ?1",
                    [observation_id.as_str()],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
                )
                .optional()
                .map_err(map_database_error)?
                .ok_or(RepoError::NotFound)?;
            if current.1.is_some() {
                return Err(RepoError::NotFound);
            }
            if current.0 != expected_revision {
                return Err(RepoError::RevisionConflict);
            }
            let changed = transaction
                .execute(
                    "UPDATE observations
                 SET text = ?1, edited_at = ?2, revision = revision + 1
                 WHERE id = ?3 AND revision = ?4 AND deleted_at IS NULL",
                    params![
                        text,
                        edited_at.get(),
                        observation_id.as_str(),
                        expected_revision
                    ],
                )
                .map_err(map_database_error)?;
            if changed == 0 {
                return Err(RepoError::RevisionConflict);
            }
            transaction
                .query_row(
                    "SELECT revision FROM observations WHERE id = ?1",
                    [observation_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(map_database_error)
        })
        .and_then(|revision| {
            u64::try_from(revision)
                .map_err(|_| RepoError::NotebookInvalid)
                .and_then(Revision::new)
        })
    }

    pub fn set_observation_searchable(
        &self,
        observation_id: &EntityId,
        searchable: bool,
        deleted_at: Option<UtcMillis>,
    ) -> Result<(), RepoError> {
        self.transact(|transaction| {
            transaction.execute(
                "UPDATE observations
                 SET searchable = ?1, deleted_at = ?2, revision = revision + 1
                 WHERE id = ?3",
                params![
                    i64::from(searchable),
                    deleted_at.map(UtcMillis::get),
                    observation_id.as_str()
                ],
            )?;
            Ok(())
        })
    }

    pub fn search_history(
        &self,
        policy: &DisclosurePolicy,
        phase: InternalPhase,
        query: &str,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<Page<HistoryHit>, RepoError> {
        policy.authorize(QueryKind::SearchHistory, phase)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let connection = connection.as_ref().ok_or(RepoError::NotebookInvalid)?;
        search(
            &connection.connection,
            query,
            cursor,
            page_size,
            &self.cursor_secret,
        )
    }

    pub fn save_public_snapshot_once(
        &self,
        snapshot_id: &EntityId,
        encounter_id: &EntityId,
        deck_revision_id: &EntityId,
        source_token: &str,
        created_at: UtcMillis,
    ) -> Result<EntityId, RepoError> {
        self.transact(|transaction| {
            if let Some(existing) = transaction
                .query_row(
                    "SELECT id FROM public_snapshots
                     WHERE encounter_id = ?1 AND source_token = ?2",
                    params![encounter_id.as_str(), source_token],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                return Ok(existing);
            }
            transaction.execute(
                "INSERT INTO public_snapshots(
                    id, encounter_id, deck_revision_id, provider, event, format,
                    publication_date, source_url, confirmed, source_token, created_at
                 ) VALUES (
                    ?1, ?2, ?3, 'official_mtgo', 'Fixture Event', 'Modern',
                    ?4, 'https://www.mtgo.com/decklists', 1, ?5, ?4
                 )",
                params![
                    snapshot_id.as_str(),
                    encounter_id.as_str(),
                    deck_revision_id.as_str(),
                    created_at.get(),
                    source_token
                ],
            )?;
            Ok(snapshot_id.as_str().to_owned())
        })
        .and_then(EntityId::parse)
    }

    pub fn run_idempotent_operation(
        &self,
        operation_id: &EntityId,
        kind: &str,
        idempotency_key: &IdempotencyKey,
        operation: impl FnOnce(&Transaction<'_>) -> Result<Value, rusqlite::Error>,
    ) -> Result<Value, RepoError> {
        self.transact(|transaction| {
            if let Some(result) = transaction
                .query_row(
                    "SELECT result_json FROM operation_records
                     WHERE idempotency_key = ?1 AND state = 'completed'",
                    [idempotency_key.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                return serde_json::from_str(&result).map_err(|_| rusqlite::Error::InvalidQuery);
            }
            let result = operation(transaction)?;
            transaction.execute(
                "INSERT INTO operation_records(
                    id, kind, idempotency_key, state, requested_at,
                    completed_at, result_json, revision
                 ) VALUES (?1, ?2, ?3, 'completed', ?4, ?4, ?5, 1)",
                params![
                    operation_id.as_str(),
                    kind,
                    idempotency_key.as_str(),
                    UtcMillis::now().get(),
                    result.to_string()
                ],
            )?;
            Ok(result)
        })
    }

    pub fn persist_operation_record(&self, record: &OperationRecord) -> Result<(), RepoError> {
        let kind = serde_json::to_value(record.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or(RepoError::InvalidRequest)?;
        let state = serde_json::to_value(record.state)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or(RepoError::InvalidRequest)?;
        let result_json = serde_json::json!({
            "completed": record.completed,
            "total": record.total,
        })
        .to_string();
        let revision =
            i64::try_from(record.revision.get()).map_err(|_| RepoError::InvalidRequest)?;
        self.transact_domain(|transaction| {
            transaction
                .execute(
                    "INSERT INTO operation_records(
                        id, kind, idempotency_key, state, requested_at,
                        completed_at, result_json, rollback_location, revision
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(id) DO UPDATE SET
                        kind = excluded.kind,
                        state = excluded.state,
                        completed_at = excluded.completed_at,
                        result_json = excluded.result_json,
                        rollback_location = excluded.rollback_location,
                        revision = excluded.revision",
                    rusqlite::params![
                        record.id.as_str(),
                        kind,
                        record.idempotency_key.as_str(),
                        state,
                        record.requested_at.get(),
                        record.completed_at.map(UtcMillis::get),
                        result_json,
                        record.rollback_location,
                        revision,
                    ],
                )
                .map_err(map_database_error)?;
            Ok(())
        })
    }

    pub fn recover_interrupted_operations(&self) -> Result<usize, RepoError> {
        self.transact_domain(|transaction| {
            transaction
                .execute(
                    "UPDATE operation_records
                     SET state = CASE
                           WHEN rollback_location IS NULL THEN 'failed'
                           ELSE 'recoverable'
                         END,
                         completed_at = ?1,
                         revision = revision + 1
                     WHERE state IN (
                       'requested', 'running', 'awaiting_confirmation', 'committing'
                     )",
                    [UtcMillis::now().get()],
                )
                .map_err(map_database_error)
        })
    }

    pub fn snapshot(&self) -> Result<ReadSnapshot, RepoError> {
        self.transact(|transaction| {
            Ok(ReadSnapshot {
                schema_version: transaction.query_row(
                    "SELECT coalesce(max(version), 0) FROM schema_migrations",
                    [],
                    |row| row.get(0),
                )?,
                profile_count: transaction.query_row(
                    "SELECT count(*) FROM opponent_profiles WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )?,
                encounter_count: transaction.query_row(
                    "SELECT count(*) FROM encounters WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )?,
                observation_count: transaction.query_row(
                    "SELECT count(*) FROM observations WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )?,
            })
        })
    }

    pub(crate) fn transact<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, rusqlite::Error>,
    ) -> Result<T, RepoError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let connection = connection.as_mut().ok_or(RepoError::NotebookInvalid)?;
        let transaction = connection
            .connection
            .transaction()
            .map_err(map_database_error)?;
        let result = operation(&transaction).map_err(map_database_error)?;
        transaction.commit().map_err(map_database_error)?;
        Ok(result)
    }

    pub(crate) fn transact_domain<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, RepoError>,
    ) -> Result<T, RepoError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let connection = connection.as_mut().ok_or(RepoError::NotebookInvalid)?;
        let transaction = connection
            .connection
            .transaction()
            .map_err(map_database_error)?;
        let result = operation(&transaction)?;
        transaction.commit().map_err(map_database_error)?;
        Ok(result)
    }

    pub(crate) fn with_connection<T>(
        &self,
        operation: impl FnOnce(&EncryptedConnection) -> Result<T, RepoError>,
    ) -> Result<T, RepoError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let connection = connection.as_ref().ok_or(RepoError::NotebookInvalid)?;
        operation(connection)
    }

    pub fn schema_version(&self) -> Result<i64, RepoError> {
        self.with_connection(current_version)
    }

    pub(crate) fn encrypted_backup_to(
        &self,
        destination: &Path,
        key: &DatabaseKey,
    ) -> Result<(), RepoError> {
        self.with_connection(|connection| connection.encrypted_backup_to(destination, key))
    }

    pub(crate) fn database_path(&self) -> Result<std::path::PathBuf, RepoError> {
        self.with_connection(|connection| Ok(connection.path().to_owned()))
    }

    pub(crate) fn atomic_replace_from(
        &self,
        staging_path: &Path,
        displaced_path: &Path,
        key: &DatabaseKey,
    ) -> Result<(), RepoError> {
        EncryptedConnection::open(staging_path, key)?.integrity_check()?;
        let mut slot = self
            .connection
            .lock()
            .map_err(|_| RepoError::NotebookInvalid)?;
        let live = slot.take().ok_or(RepoError::NotebookInvalid)?;
        let live_path = live.path().to_owned();
        live.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|_| RepoError::NotebookInvalid)?;
        drop(live);
        remove_database_family(displaced_path);
        remove_sidecars(&live_path);
        std::fs::rename(&live_path, displaced_path).map_err(|_| RepoError::NotebookInvalid)?;
        if std::fs::rename(staging_path, &live_path).is_err() {
            let _ = std::fs::rename(displaced_path, &live_path);
            *slot = Some(EncryptedConnection::open(&live_path, key)?);
            return Err(RepoError::NotebookInvalid);
        }
        remove_sidecars(&live_path);
        match EncryptedConnection::open(&live_path, key) {
            Ok(connection) => {
                connection.integrity_check()?;
                *slot = Some(connection);
                Ok(())
            }
            Err(error) => {
                let failed_path = live_path.with_extension("failed-restore");
                let _ = std::fs::rename(&live_path, &failed_path);
                let _ = std::fs::rename(displaced_path, &live_path);
                *slot = Some(EncryptedConnection::open(&live_path, key)?);
                remove_database_family(&failed_path);
                Err(error)
            }
        }
    }
}

fn remove_database_family(path: &Path) {
    let _ = std::fs::remove_file(path);
    remove_sidecars(path);
}

fn phase_name(phase: InternalPhase) -> &'static str {
    match phase {
        InternalPhase::Idle => "idle",
        InternalPhase::Candidate => "candidate",
        InternalPhase::PreMatch => "pre_match",
        InternalPhase::InGameRestricted => "in_game_restricted",
        InternalPhase::BetweenGames => "between_games",
        InternalPhase::CompletionPending => "completion_pending",
        InternalPhase::Finished => "finished",
        InternalPhase::Incomplete => "incomplete",
    }
}

fn parse_phase(value: &str) -> Result<InternalPhase, RepoError> {
    match value {
        "idle" => Ok(InternalPhase::Idle),
        "candidate" => Ok(InternalPhase::Candidate),
        "pre_match" => Ok(InternalPhase::PreMatch),
        "in_game_restricted" => Ok(InternalPhase::InGameRestricted),
        "between_games" => Ok(InternalPhase::BetweenGames),
        "completion_pending" => Ok(InternalPhase::CompletionPending),
        "finished" => Ok(InternalPhase::Finished),
        "incomplete" => Ok(InternalPhase::Incomplete),
        _ => Err(RepoError::NotebookInvalid),
    }
}

fn remove_sidecars(path: &Path) {
    let display = path.as_os_str().to_string_lossy();
    let _ = std::fs::remove_file(format!("{display}-wal"));
    let _ = std::fs::remove_file(format!("{display}-shm"));
}

pub(crate) fn map_database_error(error: rusqlite::Error) -> RepoError {
    match error {
        rusqlite::Error::QueryReturnedNoRows => RepoError::NotFound,
        rusqlite::Error::InvalidQuery => RepoError::InvalidRequest,
        _ => RepoError::NotebookInvalid,
    }
}
