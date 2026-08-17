use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::disclosure::{DisclosurePolicy, QueryKind};
use crate::domain::{EntityId, InternalPhase, RepoError};
use crate::notebook::fts::HistoryHit;
use crate::notebook::repository::NotebookRepository;
use crate::services::database_error;
use crate::services::identity::IdentityPlanRecord;
use crate::services::observations::{ObservationDetail, ObservationService};
use crate::services::profiles::{ProfileAggregate, ProfileService};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFilters {
    #[serde(default)]
    pub entity_types: Vec<String>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub certainty: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    pub text: String,
    #[serde(default)]
    pub filters: HistoryFilters,
    pub cursor: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<HistoryHit>,
    pub next_cursor: Option<String>,
    pub replacement: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterSummary {
    pub id: String,
    pub format: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: String,
    pub phase: String,
    pub source: String,
    pub incomplete_reason: Option<String>,
    pub revision: u64,
    pub observation_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastDeckSeen {
    pub label: String,
    pub source_class: String,
    pub source_label: String,
    pub format: String,
    pub seen_at: i64,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDetail {
    pub profile: ProfileAggregate,
    pub encounters: Vec<EncounterSummary>,
    pub last_deck_seen: Option<LastDeckSeen>,
    pub canonical_profile_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterDetail {
    pub summary: EncounterSummary,
    pub profile_id: String,
    pub observations: Vec<ObservationDetail>,
}

pub struct HistoryService<'a> {
    repository: &'a NotebookRepository,
    policy: &'a DisclosurePolicy,
}

impl<'a> HistoryService<'a> {
    pub fn new(repository: &'a NotebookRepository, policy: &'a DisclosurePolicy) -> Self {
        Self { repository, policy }
    }

    pub fn search(
        &self,
        phase: InternalPhase,
        query: HistoryQuery,
    ) -> Result<HistoryPage, RepoError> {
        let page = self.repository.search_history(
            self.policy,
            phase,
            &query.text,
            query.cursor.as_deref(),
            query.page_size,
        )?;
        let items = page
            .items
            .into_iter()
            .filter(|hit| {
                query.filters.entity_types.is_empty()
                    || query.filters.entity_types.contains(&hit.entity_type)
            })
            .filter(|hit| {
                query
                    .filters
                    .date_from
                    .is_none_or(|from| hit.sort_ms >= from)
            })
            .filter(|hit| query.filters.date_to.is_none_or(|to| hit.sort_ms <= to))
            .filter(|hit| {
                query
                    .filters
                    .certainty
                    .as_ref()
                    .is_none_or(|certainty| hit.content.contains(certainty))
            })
            .collect();
        Ok(HistoryPage {
            items,
            next_cursor: page.next_cursor,
            replacement: true,
        })
    }

    pub fn get_profile(
        &self,
        phase: InternalPhase,
        profile_id: &EntityId,
    ) -> Result<ProfileDetail, RepoError> {
        self.policy.authorize(QueryKind::GetProfile, phase)?;
        match ProfileService::new(self.repository).get(profile_id) {
            Ok(profile) => self.active_profile_detail(profile),
            Err(RepoError::NotFound) => {
                if let Some(canonical_id) = self.canonical_profile_for(profile_id)? {
                    let canonical = ProfileService::new(self.repository)
                        .get(&EntityId::parse(canonical_id.clone())?)?;
                    let mut detail = self.active_profile_detail(canonical)?;
                    detail.canonical_profile_id = Some(canonical_id);
                    Ok(detail)
                } else {
                    Err(RepoError::NotFound)
                }
            }
            Err(error) => Err(error),
        }
    }

    pub fn get_encounter(
        &self,
        phase: InternalPhase,
        encounter_id: &EntityId,
    ) -> Result<EncounterDetail, RepoError> {
        self.policy.authorize(QueryKind::GetEncounter, phase)?;
        let (summary, profile_id, ids) = self.repository.with_connection(|connection| {
            let (summary, profile_id) = connection
                .connection
                .query_row(
                    "SELECT encounter.id, encounter.format, encounter.started_at,
                            encounter.ended_at, encounter.status, encounter.phase,
                            encounter.source, encounter.incomplete_reason, encounter.revision,
                            (SELECT count(*) FROM observations observation
                             WHERE observation.encounter_id = encounter.id
                               AND observation.deleted_at IS NULL),
                            encounter.profile_id
                     FROM encounters encounter
                     JOIN opponent_profiles profile ON profile.id = encounter.profile_id
                     WHERE encounter.id = ?1
                       AND encounter.deleted_at IS NULL
                       AND profile.deleted_at IS NULL",
                    [encounter_id.as_str()],
                    |row| Ok((map_encounter_summary(row)?, row.get::<_, String>(10)?)),
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT id FROM observations
                     WHERE encounter_id = ?1 AND deleted_at IS NULL
                     ORDER BY created_at DESC, id DESC",
                )
                .map_err(database_error)?;
            let ids = statement
                .query_map([encounter_id.as_str()], |row| row.get::<_, String>(0))
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            Ok((summary, profile_id, ids))
        })?;
        let observations = ids
            .into_iter()
            .map(EntityId::parse)
            .map(|id| id.and_then(|id| ObservationService::new(self.repository).get(&id)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(EncounterDetail {
            summary,
            profile_id,
            observations,
        })
    }

    fn active_profile_detail(&self, profile: ProfileAggregate) -> Result<ProfileDetail, RepoError> {
        let profile_id = profile.profile.id.clone();
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT encounter.id, encounter.format, encounter.started_at,
                            encounter.ended_at, encounter.status, encounter.phase,
                            encounter.source, encounter.incomplete_reason, encounter.revision,
                            (SELECT count(*) FROM observations observation
                             WHERE observation.encounter_id = encounter.id
                               AND observation.deleted_at IS NULL)
                     FROM encounters encounter
                     WHERE encounter.profile_id = ?1 AND encounter.deleted_at IS NULL
                     ORDER BY encounter.started_at DESC, encounter.id DESC",
                )
                .map_err(database_error)?;
            let encounters = statement
                .query_map([profile_id.as_str()], map_encounter_summary)
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            let last_deck_seen = last_deck_seen(&connection.connection, &profile_id)?;
            Ok(ProfileDetail {
                profile,
                encounters,
                last_deck_seen,
                canonical_profile_id: None,
            })
        })
    }

    fn canonical_profile_for(&self, profile_id: &EntityId) -> Result<Option<String>, RepoError> {
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT reassignment_plan_json FROM profile_merges
                     WHERE state = 'applied' ORDER BY created_at DESC",
                )
                .map_err(database_error)?;
            let plans = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            for plan in plans {
                let plan: IdentityPlanRecord =
                    serde_json::from_str(&plan).map_err(|_| RepoError::NotebookInvalid)?;
                if plan.secondary_profile_id == profile_id.as_str() {
                    return Ok(Some(plan.primary_profile_id));
                }
            }
            Ok(None)
        })
    }
}

fn default_page_size() -> usize {
    50
}

fn map_encounter_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<EncounterSummary> {
    Ok(EncounterSummary {
        id: row.get(0)?,
        format: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        status: row.get(4)?,
        phase: row.get(5)?,
        source: row.get(6)?,
        incomplete_reason: row.get(7)?,
        revision: u64::try_from(row.get::<_, i64>(8)?).unwrap_or_default(),
        observation_count: u64::try_from(row.get::<_, i64>(9)?).unwrap_or_default(),
    })
}

fn last_deck_seen(
    connection: &rusqlite::Connection,
    profile_id: &EntityId,
) -> Result<Option<LastDeckSeen>, RepoError> {
    let public = connection
        .query_row(
            "SELECT coalesce(deck.provider_label, 'Official deck'),
                    snapshot.provider, snapshot.format, snapshot.publication_date
             FROM public_snapshots snapshot
             JOIN encounters encounter ON encounter.id = snapshot.encounter_id
             JOIN deck_revisions revision ON revision.id = snapshot.deck_revision_id
             JOIN deck_records deck ON deck.id = revision.deck_id
             WHERE encounter.profile_id = ?1
               AND encounter.deleted_at IS NULL
               AND encounter.status <> 'incomplete'
               AND deck.deleted_at IS NULL
               AND snapshot.confirmed = 1
             ORDER BY snapshot.publication_date DESC, snapshot.id DESC LIMIT 1",
            [profile_id.as_str()],
            |row| {
                Ok(LastDeckSeen {
                    label: row.get(0)?,
                    source_class: "public".to_owned(),
                    source_label: row.get(1)?,
                    format: row.get(2)?,
                    seen_at: row.get(3)?,
                    confirmed: true,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    let user = connection
        .query_row(
            "SELECT coalesce(user_label, 'User-entered deck'), format, created_at
             FROM deck_records
             WHERE profile_id = ?1 AND source_class = 'user' AND deleted_at IS NULL
             ORDER BY created_at DESC, id DESC LIMIT 1",
            [profile_id.as_str()],
            |row| {
                Ok(LastDeckSeen {
                    label: row.get(0)?,
                    source_class: "user".to_owned(),
                    source_label: "Player entered".to_owned(),
                    format: row.get(1)?,
                    seen_at: row.get(2)?,
                    confirmed: true,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    Ok(match (public, user) {
        (Some(public), Some(user)) if user.seen_at > public.seen_at => Some(user),
        (Some(public), _) => Some(public),
        (None, user) => user,
    })
}
