use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

use crate::domain::{EntityId, OpponentAlias, OpponentProfile, RepoError, Revision, UtcMillis};
use crate::notebook::repository::NotebookRepository;
use crate::services::database_error;

const MAX_HANDLE_CHARS: usize = 128;
const MAX_SUGGESTIONS: usize = 50;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedIdentity {
    pub display: String,
    pub key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub primary_handle: String,
    pub matched_handle: String,
    pub matched_as_alias: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAggregate {
    pub profile: OpponentProfile,
    pub aliases: Vec<OpponentAlias>,
}

pub struct ProfileService<'a> {
    repository: &'a NotebookRepository,
}

impl<'a> ProfileService<'a> {
    pub fn new(repository: &'a NotebookRepository) -> Self {
        Self { repository }
    }

    pub fn create(&self, handle: &str) -> Result<ProfileAggregate, RepoError> {
        let normalized = normalize_handle(handle)?;
        if let Some(existing) = self.resolve_exact(handle)? {
            return self.get(&EntityId::parse(existing.id)?);
        }
        let id = EntityId::new();
        self.repository.create_profile(
            &id,
            &normalized.display,
            &normalized.key,
            UtcMillis::now(),
        )?;
        self.get(&id)
    }

    pub fn resolve_exact(&self, handle: &str) -> Result<Option<ProfileSummary>, RepoError> {
        let normalized = normalize_handle(handle)?;
        self.repository.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT profile.id, profile.primary_handle, profile.revision,
                            profile.primary_handle, 0
                     FROM opponent_profiles profile
                     WHERE profile.normalized_handle = ?1 AND profile.deleted_at IS NULL
                     UNION ALL
                     SELECT profile.id, profile.primary_handle, profile.revision,
                            alias.display_handle, 1
                     FROM opponent_aliases alias
                     JOIN opponent_profiles profile ON profile.id = alias.profile_id
                     WHERE alias.normalized_handle = ?1 AND profile.deleted_at IS NULL
                     LIMIT 1",
                    [normalized.key],
                    |row| {
                        Ok(ProfileSummary {
                            id: row.get(0)?,
                            primary_handle: row.get(1)?,
                            revision: u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                            matched_handle: row.get(3)?,
                            matched_as_alias: row.get::<_, i64>(4)? == 1,
                        })
                    },
                )
                .optional()
                .map_err(database_error)
        })
    }

    pub fn suggestions(&self, query: &str, limit: usize) -> Result<Vec<ProfileSummary>, RepoError> {
        let limit = limit.min(MAX_SUGGESTIONS);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let normalized = normalize_handle(query)?;
        let prefix = format!("{}%", escape_like(&normalized.key));
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT profile.id, profile.primary_handle, profile.revision,
                            profile.primary_handle, 0, profile.normalized_handle
                     FROM opponent_profiles profile
                     WHERE profile.deleted_at IS NULL
                       AND profile.normalized_handle LIKE ?1 ESCAPE '\\'
                     UNION ALL
                     SELECT profile.id, profile.primary_handle, profile.revision,
                            alias.display_handle, 1, alias.normalized_handle
                     FROM opponent_aliases alias
                     JOIN opponent_profiles profile ON profile.id = alias.profile_id
                     WHERE profile.deleted_at IS NULL
                       AND alias.normalized_handle LIKE ?1 ESCAPE '\\'
                     ORDER BY 6, 2, 1
                     LIMIT ?2",
                )
                .map_err(database_error)?;
            statement
                .query_map(
                    params![
                        prefix,
                        i64::try_from(limit).map_err(|_| RepoError::InvalidRequest)?
                    ],
                    |row| {
                        Ok(ProfileSummary {
                            id: row.get(0)?,
                            primary_handle: row.get(1)?,
                            revision: u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                            matched_handle: row.get(3)?,
                            matched_as_alias: row.get::<_, i64>(4)? == 1,
                        })
                    },
                )
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)
        })
    }

    pub fn add_alias(
        &self,
        profile_id: &EntityId,
        display_handle: &str,
    ) -> Result<ProfileAggregate, RepoError> {
        let normalized = normalize_handle(display_handle)?;
        self.repository.add_alias(
            &EntityId::new(),
            profile_id,
            &normalized.display,
            &normalized.key,
            UtcMillis::now(),
        )?;
        self.get(profile_id)
    }

    pub fn update_primary_handle(
        &self,
        profile_id: &EntityId,
        expected_revision: Revision,
        handle: &str,
    ) -> Result<ProfileAggregate, RepoError> {
        let normalized = normalize_handle(handle)?;
        let now = UtcMillis::now();
        self.repository.transact_domain(|transaction| {
            let current = transaction
                .query_row(
                    "SELECT primary_handle, normalized_handle, revision
                     FROM opponent_profiles
                     WHERE id = ?1 AND deleted_at IS NULL",
                    [profile_id.as_str()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            if current.2
                != i64::try_from(expected_revision.get())
                    .map_err(|_| RepoError::InvalidRequest)?
            {
                return Err(RepoError::RevisionConflict);
            }
            let conflict = transaction
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM opponent_profiles
                        WHERE normalized_handle = ?1
                          AND id <> ?2 AND deleted_at IS NULL
                        UNION ALL
                        SELECT 1 FROM opponent_aliases alias
                        JOIN opponent_profiles profile ON profile.id = alias.profile_id
                        WHERE alias.normalized_handle = ?1
                          AND alias.profile_id <> ?2 AND profile.deleted_at IS NULL
                    )",
                    params![normalized.key, profile_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            if conflict == 1 {
                return Err(RepoError::IdentityConflict);
            }
            transaction
                .execute(
                    "UPDATE opponent_profiles
                     SET primary_handle = ?1, normalized_handle = ?2, revision = revision + 1
                     WHERE id = ?3",
                    params![normalized.display, normalized.key, profile_id.as_str()],
                )
                .map_err(database_error)?;
            if current.1 != normalized.key {
                transaction
                    .execute(
                        "INSERT OR IGNORE INTO opponent_aliases(
                            id, profile_id, display_handle, normalized_handle, provenance, created_at
                         ) VALUES (?1, ?2, ?3, ?4, 'prior_primary', ?5)",
                        params![
                            EntityId::new().as_str(),
                            profile_id.as_str(),
                            current.0,
                            current.1,
                            now.get()
                        ],
                    )
                    .map_err(database_error)?;
            }
            Ok(())
        })?;
        self.get(profile_id)
    }

    pub fn get(&self, profile_id: &EntityId) -> Result<ProfileAggregate, RepoError> {
        self.repository.with_connection(|connection| {
            let profile = connection
                .connection
                .query_row(
                    "SELECT id, primary_handle, normalized_handle, created_at, revision, deleted_at
                     FROM opponent_profiles WHERE id = ?1 AND deleted_at IS NULL",
                    [profile_id.as_str()],
                    map_profile,
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT id, profile_id, display_handle, normalized_handle, provenance
                     FROM opponent_aliases WHERE profile_id = ?1
                     ORDER BY normalized_handle, id",
                )
                .map_err(database_error)?;
            let aliases = statement
                .query_map([profile_id.as_str()], map_alias)
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            Ok(ProfileAggregate { profile, aliases })
        })
    }
}

pub fn normalize_handle(value: &str) -> Result<NormalizedIdentity, RepoError> {
    normalize_identity(value, MAX_HANDLE_CHARS, RepoError::InvalidHandle)
}

pub fn normalize_tag(value: &str) -> Result<NormalizedIdentity, RepoError> {
    normalize_identity(value, 128, RepoError::InvalidTag)
}

pub fn normalize_card_name(value: &str) -> Result<NormalizedIdentity, RepoError> {
    normalize_identity(value, 256, RepoError::InvalidCard)
}

fn normalize_identity(
    value: &str,
    max_chars: usize,
    error: RepoError,
) -> Result<NormalizedIdentity, RepoError> {
    let display = value
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, '|' | '•' | '·')
        })
        .to_owned();
    let valid = !display.is_empty()
        && display.chars().count() <= max_chars
        && !display
            .chars()
            .any(|character| character.is_control() || matches!(character, '<' | '>'));
    if !valid {
        return Err(error);
    }
    let key = display.nfkc().case_fold().collect::<String>();
    if key.is_empty() {
        return Err(error);
    }
    Ok(NormalizedIdentity { display, key })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn map_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<OpponentProfile> {
    Ok(OpponentProfile {
        id: EntityId::parse(row.get::<_, String>(0)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        primary_handle: row.get(1)?,
        normalized_handle: row.get(2)?,
        created_at: UtcMillis::new(row.get(3)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        revision: Revision::new(
            u64::try_from(row.get::<_, i64>(4)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        )
        .map_err(|_| rusqlite::Error::InvalidQuery)?,
        deleted_at: row
            .get::<_, Option<i64>>(5)?
            .map(UtcMillis::new)
            .transpose()
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

fn map_alias(row: &rusqlite::Row<'_>) -> rusqlite::Result<OpponentAlias> {
    Ok(OpponentAlias {
        id: EntityId::parse(row.get::<_, String>(0)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        profile_id: EntityId::parse(row.get::<_, String>(1)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        display_handle: row.get(2)?,
        normalized_handle: row.get(3)?,
        provenance: row.get(4)?,
    })
}
