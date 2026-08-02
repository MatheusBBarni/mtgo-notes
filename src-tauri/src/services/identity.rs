use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, IdempotencyKey, RepoError, UtcMillis};
use crate::notebook::repository::NotebookRepository;
use crate::services::{contract_token, database_error};

const MAX_CONFLICT_DETAILS: usize = 50;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCounts {
    pub profiles: u64,
    pub aliases: u64,
    pub encounters: u64,
    pub observations: u64,
    pub decks: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergePreview {
    pub primary_profile_id: String,
    pub secondary_profile_id: String,
    pub primary_handle: String,
    pub secondary_handle: String,
    pub expected_primary_revision: u64,
    pub expected_secondary_revision: u64,
    pub affected: IdentityCounts,
    pub conflicts: Vec<String>,
    pub conflict_count: u64,
    pub conflict_details_bounded: bool,
    pub irreversible_consequences: Vec<String>,
    pub plan_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub merge_id: String,
    pub canonical_profile_id: String,
    pub canonical_revision: u64,
    pub reversible: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmergePreview {
    pub merge_id: String,
    pub primary_profile_id: String,
    pub secondary_profile_id: String,
    pub restored_encounters: u64,
    pub restored_decks: u64,
    pub post_merge_encounters: u64,
    pub post_merge_decks: u64,
    pub proposed_post_merge_assignment: String,
    pub plan_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AliasPlanRecord {
    pub id: String,
    pub display_handle: String,
    pub normalized_handle: String,
    pub provenance: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityPlanRecord {
    pub primary_profile_id: String,
    pub secondary_profile_id: String,
    pub primary_handle: String,
    pub primary_normalized_handle: String,
    pub secondary_handle: String,
    pub secondary_normalized_handle: String,
    pub primary_revision_before: u64,
    pub secondary_revision_before: u64,
    pub secondary_aliases: Vec<AliasPlanRecord>,
    pub created_alias_id: Option<String>,
    pub moved_alias_ids: Vec<String>,
    pub original_primary_encounter_ids: Vec<String>,
    pub moved_encounter_ids: Vec<String>,
    pub original_primary_deck_ids: Vec<String>,
    pub moved_deck_ids: Vec<String>,
    pub merged_at: i64,
}

#[derive(Clone, Debug)]
struct ProfileRecord {
    id: String,
    primary_handle: String,
    normalized_handle: String,
    revision: u64,
}

pub struct IdentityService<'a> {
    repository: &'a NotebookRepository,
}

impl<'a> IdentityService<'a> {
    pub fn new(repository: &'a NotebookRepository) -> Self {
        Self { repository }
    }

    pub fn preview_merge(
        &self,
        left_id: &EntityId,
        right_id: &EntityId,
        primary_id: &EntityId,
    ) -> Result<MergePreview, RepoError> {
        if left_id == right_id || (primary_id != left_id && primary_id != right_id) {
            return Err(RepoError::MergeConflict);
        }
        self.repository.with_connection(|connection| {
            let left =
                load_profile(&connection.connection, left_id)?.ok_or(RepoError::MergeConflict)?;
            let right =
                load_profile(&connection.connection, right_id)?.ok_or(RepoError::MergeConflict)?;
            let (primary, secondary) = if primary_id.as_str() == left.id {
                (left, right)
            } else {
                (right, left)
            };
            let preview = build_preview(&connection.connection, &primary, &secondary)?;
            Ok(preview)
        })
    }

    pub fn apply_merge(
        &self,
        preview: &MergePreview,
        idempotency_key: &IdempotencyKey,
    ) -> Result<MergeResult, RepoError> {
        if contract_token(
            b"mtgo-notes-merge-preview-v1",
            &preview_without_token(preview),
        )? != preview.plan_token
        {
            return Err(RepoError::MergeConflict);
        }
        let now = UtcMillis::now();
        self.repository
            .transact_domain(|transaction| {
            if let Some(existing) = transaction
                .query_row(
                    "SELECT result_json FROM operation_records
                     WHERE idempotency_key = ?1 AND state = 'completed'",
                    [idempotency_key.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)?
            {
                return serde_json::from_str(&existing).map_err(|_| RepoError::NotebookInvalid);
            }
            let primary_id = EntityId::parse(preview.primary_profile_id.clone())?;
            let secondary_id = EntityId::parse(preview.secondary_profile_id.clone())?;
            let primary =
                load_profile(transaction, &primary_id)?.ok_or(RepoError::RevisionConflict)?;
            let secondary =
                load_profile(transaction, &secondary_id)?.ok_or(RepoError::RevisionConflict)?;
            if primary.revision != preview.expected_primary_revision
                || secondary.revision != preview.expected_secondary_revision
            {
                return Err(RepoError::RevisionConflict);
            }
            let live_preview = build_preview(transaction, &primary, &secondary)?;
            if live_preview.plan_token != preview.plan_token {
                return Err(RepoError::RevisionConflict);
            }
            let secondary_aliases = load_aliases(transaction, &secondary.id)?;
            let existing_primary_keys = load_aliases(transaction, &primary.id)?
                .into_iter()
                .map(|alias| alias.normalized_handle)
                .chain(std::iter::once(primary.normalized_handle.clone()))
                .collect::<BTreeSet<_>>();
            let original_primary_encounter_ids =
                load_ids(transaction, "encounters", "profile_id", &primary.id)?;
            let moved_encounter_ids =
                load_ids(transaction, "encounters", "profile_id", &secondary.id)?;
            let original_primary_deck_ids =
                load_ids(transaction, "deck_records", "profile_id", &primary.id)?;
            let moved_deck_ids =
                load_ids(transaction, "deck_records", "profile_id", &secondary.id)?;

            transaction
                .execute(
                    "DELETE FROM opponent_aliases WHERE profile_id = ?1",
                    [&secondary.id],
                )
                .map_err(database_error)?;
            let mut moved_alias_ids = Vec::new();
            for alias in &secondary_aliases {
                if existing_primary_keys.contains(&alias.normalized_handle) {
                    continue;
                }
                transaction
                    .execute(
                        "INSERT INTO opponent_aliases(
                            id, profile_id, display_handle, normalized_handle, provenance, created_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            alias.id,
                            primary.id,
                            alias.display_handle,
                            alias.normalized_handle,
                            alias.provenance,
                            alias.created_at
                        ],
                    )
                    .map_err(database_error)?;
                moved_alias_ids.push(alias.id.clone());
            }
            let created_alias_id =
                if existing_primary_keys.contains(&secondary.normalized_handle) {
                    None
                } else {
                    let alias_id = EntityId::new().to_string();
                    transaction
                        .execute(
                            "INSERT INTO opponent_aliases(
                                id, profile_id, display_handle, normalized_handle, provenance, created_at
                             ) VALUES (?1, ?2, ?3, ?4, 'merged_primary', ?5)",
                            params![
                                alias_id,
                                primary.id,
                                secondary.primary_handle,
                                secondary.normalized_handle,
                                now.get()
                            ],
                        )
                        .map_err(database_error)?;
                    Some(alias_id)
                };
            transaction
                .execute(
                    "UPDATE encounters SET profile_id = ?1 WHERE profile_id = ?2",
                    params![primary.id, secondary.id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE deck_records SET profile_id = ?1 WHERE profile_id = ?2",
                    params![primary.id, secondary.id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE opponent_profiles SET revision = revision + 1 WHERE id = ?1",
                    [&primary.id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE opponent_profiles
                     SET deleted_at = ?1, revision = revision + 1 WHERE id = ?2",
                    params![now.get(), secondary.id],
                )
                .map_err(database_error)?;
            let merge_id = EntityId::new();
            let plan = IdentityPlanRecord {
                primary_profile_id: primary.id.clone(),
                secondary_profile_id: secondary.id.clone(),
                primary_handle: primary.primary_handle.clone(),
                primary_normalized_handle: primary.normalized_handle.clone(),
                secondary_handle: secondary.primary_handle.clone(),
                secondary_normalized_handle: secondary.normalized_handle.clone(),
                primary_revision_before: primary.revision,
                secondary_revision_before: secondary.revision,
                secondary_aliases,
                created_alias_id,
                moved_alias_ids,
                original_primary_encounter_ids,
                moved_encounter_ids,
                original_primary_deck_ids,
                moved_deck_ids,
                merged_at: now.get(),
            };
            transaction
                .execute(
                    "INSERT INTO profile_merges(
                        id, primary_profile_id, state, created_at,
                        reassignment_plan_json, revision
                     ) VALUES (?1, ?2, 'applied', ?3, ?4, 1)",
                    params![
                        merge_id.as_str(),
                        primary.id,
                        now.get(),
                        serde_json::to_string(&plan).map_err(|_| RepoError::NotebookInvalid)?
                    ],
                )
                .map_err(database_error)?;
            let result = MergeResult {
                merge_id: merge_id.to_string(),
                canonical_profile_id: primary.id,
                canonical_revision: primary.revision + 1,
                reversible: true,
            };
            transaction
                .execute(
                    "INSERT INTO operation_records(
                        id, kind, idempotency_key, state, requested_at,
                        completed_at, result_json, revision
                     ) VALUES (?1, 'profile_merge', ?2, 'completed', ?3, ?3, ?4, 1)",
                    params![
                        EntityId::new().as_str(),
                        idempotency_key.as_str(),
                        now.get(),
                        serde_json::to_string(&result).map_err(|_| RepoError::NotebookInvalid)?
                    ],
                )
                .map_err(database_error)?;
                Ok(result)
            })
            .map_err(mutation_error)
    }

    pub fn preview_unmerge(&self, merge_id: &EntityId) -> Result<UnmergePreview, RepoError> {
        self.repository
            .with_connection(|connection| build_unmerge_preview(&connection.connection, merge_id))
    }

    pub fn apply_unmerge(
        &self,
        preview: &UnmergePreview,
        idempotency_key: &IdempotencyKey,
    ) -> Result<MergeResult, RepoError> {
        if contract_token(
            b"mtgo-notes-unmerge-preview-v1",
            &unmerge_without_token(preview),
        )? != preview.plan_token
        {
            return Err(RepoError::MergeConflict);
        }
        let merge_id = EntityId::parse(preview.merge_id.clone())?;
        let now = UtcMillis::now();
        self.repository
            .transact_domain(|transaction| {
            if let Some(existing) = transaction
                .query_row(
                    "SELECT result_json FROM operation_records
                     WHERE idempotency_key = ?1 AND state = 'completed'",
                    [idempotency_key.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)?
            {
                return serde_json::from_str(&existing).map_err(|_| RepoError::NotebookInvalid);
            }
            let live = build_unmerge_preview(transaction, &merge_id)?;
            if live.plan_token != preview.plan_token {
                return Err(RepoError::RevisionConflict);
            }
            let plan = load_plan(transaction, &merge_id)?;
            transaction
                .execute(
                    "UPDATE opponent_profiles
                     SET deleted_at = NULL, revision = revision + 1 WHERE id = ?1",
                    [&plan.secondary_profile_id],
                )
                .map_err(database_error)?;
            for id in &plan.moved_encounter_ids {
                transaction
                    .execute(
                        "UPDATE encounters SET profile_id = ?1 WHERE id = ?2 AND profile_id = ?3",
                        params![plan.secondary_profile_id, id, plan.primary_profile_id],
                    )
                    .map_err(database_error)?;
            }
            for id in &plan.moved_deck_ids {
                transaction
                    .execute(
                        "UPDATE deck_records SET profile_id = ?1 WHERE id = ?2 AND profile_id = ?3",
                        params![plan.secondary_profile_id, id, plan.primary_profile_id],
                    )
                    .map_err(database_error)?;
            }
            if let Some(created_alias_id) = &plan.created_alias_id {
                transaction
                    .execute(
                        "DELETE FROM opponent_aliases WHERE id = ?1 AND profile_id = ?2",
                        params![created_alias_id, plan.primary_profile_id],
                    )
                    .map_err(database_error)?;
            }
            for alias_id in &plan.moved_alias_ids {
                transaction
                    .execute(
                        "DELETE FROM opponent_aliases WHERE id = ?1 AND profile_id = ?2",
                        params![alias_id, plan.primary_profile_id],
                    )
                    .map_err(database_error)?;
            }
            for alias in &plan.secondary_aliases {
                transaction
                    .execute(
                        "INSERT OR IGNORE INTO opponent_aliases(
                            id, profile_id, display_handle, normalized_handle, provenance, created_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            alias.id,
                            plan.secondary_profile_id,
                            alias.display_handle,
                            alias.normalized_handle,
                            alias.provenance,
                            alias.created_at
                        ],
                    )
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE opponent_profiles SET revision = revision + 1 WHERE id = ?1",
                    [&plan.primary_profile_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE profile_merges
                     SET state = 'reversed', reversed_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND state = 'applied'",
                    params![now.get(), merge_id.as_str()],
                )
                .map_err(database_error)?;
            let primary_revision = transaction
                .query_row(
                    "SELECT revision FROM opponent_profiles WHERE id = ?1",
                    [&plan.primary_profile_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            let result = MergeResult {
                merge_id: merge_id.to_string(),
                canonical_profile_id: plan.primary_profile_id,
                canonical_revision: u64::try_from(primary_revision)
                    .map_err(|_| RepoError::NotebookInvalid)?,
                reversible: false,
            };
            transaction
                .execute(
                    "INSERT INTO operation_records(
                        id, kind, idempotency_key, state, requested_at,
                        completed_at, result_json, revision
                     ) VALUES (?1, 'profile_unmerge', ?2, 'completed', ?3, ?3, ?4, 1)",
                    params![
                        EntityId::new().as_str(),
                        idempotency_key.as_str(),
                        now.get(),
                        serde_json::to_string(&result).map_err(|_| RepoError::NotebookInvalid)?
                    ],
                )
                .map_err(database_error)?;
                Ok(result)
            })
            .map_err(mutation_error)
    }
}

fn mutation_error(error: RepoError) -> RepoError {
    if error == RepoError::NotebookInvalid {
        RepoError::SaveFailed
    } else {
        error
    }
}

fn build_preview(
    connection: &rusqlite::Connection,
    primary: &ProfileRecord,
    secondary: &ProfileRecord,
) -> Result<MergePreview, RepoError> {
    let primary_aliases = load_aliases(connection, &primary.id)?;
    let secondary_aliases = load_aliases(connection, &secondary.id)?;
    let primary_keys = primary_aliases
        .iter()
        .map(|alias| alias.normalized_handle.clone())
        .chain(std::iter::once(primary.normalized_handle.clone()))
        .collect::<BTreeSet<_>>();
    let mut conflicts = secondary_aliases
        .iter()
        .filter(|alias| primary_keys.contains(&alias.normalized_handle))
        .map(|alias| format!("duplicate_alias:{}", alias.display_handle))
        .collect::<Vec<_>>();
    if primary_keys.contains(&secondary.normalized_handle) {
        conflicts.push(format!("duplicate_handle:{}", secondary.primary_handle));
    }
    let conflict_count = conflicts.len() as u64;
    conflicts.truncate(MAX_CONFLICT_DETAILS);
    let encounters = count(
        connection,
        "encounters",
        "profile_id",
        &[&primary.id, &secondary.id],
    )?;
    let observations = connection
        .query_row(
            "SELECT count(*) FROM observations observation
             JOIN encounters encounter ON encounter.id = observation.encounter_id
             WHERE encounter.profile_id IN (?1, ?2)
               AND observation.deleted_at IS NULL",
            params![primary.id, secondary.id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    let decks = count(
        connection,
        "deck_records",
        "profile_id",
        &[&primary.id, &secondary.id],
    )?;
    let mut preview = MergePreview {
        primary_profile_id: primary.id.clone(),
        secondary_profile_id: secondary.id.clone(),
        primary_handle: primary.primary_handle.clone(),
        secondary_handle: secondary.primary_handle.clone(),
        expected_primary_revision: primary.revision,
        expected_secondary_revision: secondary.revision,
        affected: IdentityCounts {
            profiles: 2,
            aliases: (primary_aliases.len() + secondary_aliases.len() + 1) as u64,
            encounters: u64::try_from(encounters).map_err(|_| RepoError::NotebookInvalid)?,
            observations: u64::try_from(observations).map_err(|_| RepoError::NotebookInvalid)?,
            decks: u64::try_from(decks).map_err(|_| RepoError::NotebookInvalid)?,
        },
        conflicts,
        conflict_count,
        conflict_details_bounded: conflict_count > MAX_CONFLICT_DETAILS as u64,
        irreversible_consequences: vec![
            "Purged records cannot be restored by unmerge.".to_owned(),
            "Post-merge records require an explicit unmerge assignment.".to_owned(),
        ],
        plan_token: String::new(),
    };
    preview.plan_token = contract_token(
        b"mtgo-notes-merge-preview-v1",
        &preview_without_token(&preview),
    )?;
    Ok(preview)
}

fn preview_without_token(preview: &MergePreview) -> MergePreview {
    let mut value = preview.clone();
    value.plan_token.clear();
    value
}

fn unmerge_without_token(preview: &UnmergePreview) -> UnmergePreview {
    let mut value = preview.clone();
    value.plan_token.clear();
    value
}

fn build_unmerge_preview(
    connection: &rusqlite::Connection,
    merge_id: &EntityId,
) -> Result<UnmergePreview, RepoError> {
    let plan = load_plan(connection, merge_id)?;
    let current_encounters = load_ids(
        connection,
        "encounters",
        "profile_id",
        &plan.primary_profile_id,
    )?;
    let current_decks = load_ids(
        connection,
        "deck_records",
        "profile_id",
        &plan.primary_profile_id,
    )?;
    let known_encounters = plan
        .original_primary_encounter_ids
        .iter()
        .chain(&plan.moved_encounter_ids)
        .cloned()
        .collect::<BTreeSet<_>>();
    let known_decks = plan
        .original_primary_deck_ids
        .iter()
        .chain(&plan.moved_deck_ids)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut preview = UnmergePreview {
        merge_id: merge_id.to_string(),
        primary_profile_id: plan.primary_profile_id,
        secondary_profile_id: plan.secondary_profile_id,
        restored_encounters: plan.moved_encounter_ids.len() as u64,
        restored_decks: plan.moved_deck_ids.len() as u64,
        post_merge_encounters: current_encounters
            .iter()
            .filter(|id| !known_encounters.contains(*id))
            .count() as u64,
        post_merge_decks: current_decks
            .iter()
            .filter(|id| !known_decks.contains(*id))
            .count() as u64,
        proposed_post_merge_assignment: "retain_with_primary".to_owned(),
        plan_token: String::new(),
    };
    preview.plan_token = contract_token(
        b"mtgo-notes-unmerge-preview-v1",
        &unmerge_without_token(&preview),
    )?;
    Ok(preview)
}

fn load_profile(
    connection: &rusqlite::Connection,
    id: &EntityId,
) -> Result<Option<ProfileRecord>, RepoError> {
    connection
        .query_row(
            "SELECT id, primary_handle, normalized_handle, revision
             FROM opponent_profiles WHERE id = ?1 AND deleted_at IS NULL",
            [id.as_str()],
            |row| {
                Ok(ProfileRecord {
                    id: row.get(0)?,
                    primary_handle: row.get(1)?,
                    normalized_handle: row.get(2)?,
                    revision: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                })
            },
        )
        .optional()
        .map_err(database_error)
}

fn load_aliases(
    connection: &rusqlite::Connection,
    profile_id: &str,
) -> Result<Vec<AliasPlanRecord>, RepoError> {
    let mut statement = connection
        .prepare(
            "SELECT id, display_handle, normalized_handle, provenance, created_at
             FROM opponent_aliases WHERE profile_id = ?1
             ORDER BY normalized_handle, id",
        )
        .map_err(database_error)?;
    statement
        .query_map([profile_id], |row| {
            Ok(AliasPlanRecord {
                id: row.get(0)?,
                display_handle: row.get(1)?,
                normalized_handle: row.get(2)?,
                provenance: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)
}

fn load_ids(
    connection: &rusqlite::Connection,
    table: &str,
    column: &str,
    value: &str,
) -> Result<Vec<String>, RepoError> {
    let allowed = BTreeMap::from([
        (("encounters", "profile_id"), true),
        (("deck_records", "profile_id"), true),
    ]);
    if !allowed.contains_key(&(table, column)) {
        return Err(RepoError::InvalidRequest);
    }
    let sql = format!("SELECT id FROM {table} WHERE {column} = ?1 ORDER BY id");
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    statement
        .query_map([value], |row| row.get::<_, String>(0))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)
}

fn count(
    connection: &rusqlite::Connection,
    table: &str,
    column: &str,
    values: &[&str],
) -> Result<i64, RepoError> {
    if values.len() != 2
        || !matches!(
            (table, column),
            ("encounters", "profile_id") | ("deck_records", "profile_id")
        )
    {
        return Err(RepoError::InvalidRequest);
    }
    let sql = format!(
        "SELECT count(*) FROM {table}
         WHERE {column} IN (?1, ?2) AND deleted_at IS NULL"
    );
    connection
        .query_row(&sql, params![values[0], values[1]], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_error)
}

fn load_plan(
    connection: &rusqlite::Connection,
    merge_id: &EntityId,
) -> Result<IdentityPlanRecord, RepoError> {
    let json = connection
        .query_row(
            "SELECT reassignment_plan_json FROM profile_merges
             WHERE id = ?1 AND state = 'applied'",
            [merge_id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?
        .ok_or(RepoError::MergeConflict)?;
    serde_json::from_str(&json).map_err(|_| RepoError::NotebookInvalid)
}
