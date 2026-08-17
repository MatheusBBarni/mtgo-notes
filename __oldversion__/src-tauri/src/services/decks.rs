use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::classifier::{
    AssetRegistry, CanonicalCard, ClassificationExplanation, ClassificationMethod,
    ClassificationResult, ClassifierAssets, CompleteDeck, DeckClassifier,
};
use crate::domain::{EntityId, RepoError, UtcMillis};
use crate::notebook::repository::NotebookRepository;
use crate::providers::decks::{
    DeckCandidate, DeckCard, DeckZone, OfficialDeckProvider, validate_candidate,
};
use crate::services::database_error;

const RECLASSIFICATION_BATCH_SIZE: usize = 25;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckSourceClass {
    Public,
    User,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationRunView {
    pub id: String,
    pub deck_revision_id: String,
    pub classifier_version: String,
    pub classifier_digest: String,
    pub result_id: String,
    pub result_name: String,
    pub method: ClassificationMethod,
    pub confidence: f64,
    pub explanation: ClassificationExplanation,
    pub status: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSnapshotView {
    pub id: String,
    pub encounter_id: String,
    pub provider: String,
    pub event: String,
    pub format: String,
    pub publication_date: i64,
    pub source_url: String,
    pub confirmed: bool,
    pub source_token: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckDetails {
    pub deck_id: String,
    pub deck_revision_id: String,
    pub revision_number: u64,
    pub canonical_digest: String,
    pub complete: bool,
    pub format: String,
    pub source_class: DeckSourceClass,
    pub provider_label: Option<String>,
    pub user_label: Option<String>,
    pub cards: Vec<DeckCard>,
    pub public_snapshot: Option<PublicSnapshotView>,
    pub current_classification: Option<ClassificationRunView>,
    pub classification_history: Vec<ClassificationRunView>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCompleteDeck {
    pub deck_id: Option<String>,
    pub profile_id: String,
    pub format: String,
    pub user_label: Option<String>,
    pub cards: Vec<DeckCard>,
}

pub struct DeckService<'a> {
    repository: &'a NotebookRepository,
    assets: &'a AssetRegistry,
}

impl<'a> DeckService<'a> {
    pub fn new(repository: &'a NotebookRepository, assets: &'a AssetRegistry) -> Self {
        Self { repository, assets }
    }

    pub fn confirm_public_snapshot(
        &self,
        provider: &OfficialDeckProvider,
        encounter_id: &EntityId,
        candidate: &DeckCandidate,
        active_generation: u64,
        active_format: &str,
    ) -> Result<DeckDetails, RepoError> {
        provider.validate_confirmation(candidate, active_generation, active_format)?;
        validate_candidate(candidate, active_format)?;
        let existing = self.repository.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT revision.deck_id
                     FROM public_snapshots snapshot
                     JOIN deck_revisions revision ON revision.id = snapshot.deck_revision_id
                     WHERE snapshot.encounter_id = ?1 AND snapshot.source_token = ?2",
                    params![encounter_id.as_str(), candidate.response_token],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(database_error)
        })?;
        if let Some(deck_id) = existing {
            return self.get_deck_details(&EntityId::parse(deck_id)?);
        }

        let (profile_id, encounter_format, generation) =
            self.repository.with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT encounter.profile_id, encounter.format, encounter.generation
                         FROM encounters encounter
                         JOIN opponent_profiles profile ON profile.id = encounter.profile_id
                         WHERE encounter.id = ?1
                           AND encounter.deleted_at IS NULL
                           AND profile.deleted_at IS NULL",
                        [encounter_id.as_str()],
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
                    .ok_or(RepoError::NotFound)
            })?;
        if u64::try_from(generation).ok() != Some(active_generation)
            || !encounter_format.eq_ignore_ascii_case(active_format)
        {
            return Err(RepoError::StaleProviderResult);
        }

        let assets = self.assets.current()?;
        let complete_deck = complete_deck(&candidate.format, &candidate.cards, true);
        let result = DeckClassifier::classify_confirmable(&complete_deck, &assets)?;
        let now = UtcMillis::now();
        let deck_id = EntityId::new();
        let deck_revision_id = EntityId::new();
        let snapshot_id = EntityId::new();
        let run_id = EntityId::new();
        let job_id = EntityId::new();
        let digest = canonical_digest(&complete_deck)?;
        let explanation =
            serde_json::to_string(&result.explanation).map_err(|_| RepoError::NotebookInvalid)?;
        self.repository.transact_domain(|transaction| {
            transaction
                .execute(
                    "INSERT INTO deck_records(
                        id, profile_id, source_class, format, completeness,
                        provider_label, user_label, current_revision, revision, created_at
                     ) VALUES (?1, ?2, 'public', ?3, 'complete', ?4, NULL, 1, 1, ?5)",
                    params![
                        deck_id.as_str(),
                        profile_id,
                        candidate.format,
                        candidate.provider_label,
                        now.get()
                    ],
                )
                .map_err(database_error)?;
            insert_revision(
                transaction,
                &deck_revision_id,
                &deck_id,
                &digest,
                &candidate.cards,
                now,
            )?;
            transaction
                .execute(
                    "INSERT INTO public_snapshots(
                        id, encounter_id, deck_revision_id, provider, event, format,
                        publication_date, source_url, confirmed, source_token, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10)",
                    params![
                        snapshot_id.as_str(),
                        encounter_id.as_str(),
                        deck_revision_id.as_str(),
                        candidate.provider,
                        candidate.event,
                        candidate.format,
                        candidate.publication_date.get(),
                        candidate.source_url,
                        candidate.response_token,
                        now.get()
                    ],
                )
                .map_err(database_error)?;
            insert_classification_run(
                transaction,
                &run_id,
                &deck_revision_id,
                &result,
                &explanation,
                now,
            )?;
            insert_completed_classification_job(transaction, &job_id, &deck_revision_id, &assets)?;
            Ok(())
        })?;
        self.get_deck_details(&deck_id)
    }

    pub fn save_complete_deck(&self, input: SaveCompleteDeck) -> Result<DeckDetails, RepoError> {
        if let Some(deck_id) = input.deck_id.as_deref() {
            return self.revise_complete_deck(&EntityId::parse(deck_id.to_owned())?, input);
        }
        let profile_id = EntityId::parse(input.profile_id)?;
        if input.format.trim().is_empty() {
            return Err(RepoError::InvalidRequest);
        }
        let deck = complete_deck(&input.format, &input.cards, true);
        validate_complete_cards(&input.cards)?;
        let assets = self.assets.current()?;
        let result = DeckClassifier::classify_confirmable(&deck, &assets)?;
        let now = UtcMillis::now();
        let deck_id = EntityId::new();
        let revision_id = EntityId::new();
        let run_id = EntityId::new();
        let job_id = EntityId::new();
        let digest = canonical_digest(&deck)?;
        let explanation =
            serde_json::to_string(&result.explanation).map_err(|_| RepoError::NotebookInvalid)?;
        self.repository.transact_domain(|transaction| {
            let profile_exists = transaction
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM opponent_profiles WHERE id = ?1 AND deleted_at IS NULL
                    )",
                    [profile_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            if profile_exists != 1 {
                return Err(RepoError::NotFound);
            }
            transaction
                .execute(
                    "INSERT INTO deck_records(
                        id, profile_id, source_class, format, completeness,
                        provider_label, user_label, current_revision, revision, created_at
                     ) VALUES (?1, ?2, 'user', ?3, 'complete', NULL, ?4, 1, 1, ?5)",
                    params![
                        deck_id.as_str(),
                        profile_id.as_str(),
                        input.format,
                        input.user_label,
                        now.get()
                    ],
                )
                .map_err(database_error)?;
            insert_revision(
                transaction,
                &revision_id,
                &deck_id,
                &digest,
                &input.cards,
                now,
            )?;
            insert_classification_run(
                transaction,
                &run_id,
                &revision_id,
                &result,
                &explanation,
                now,
            )?;
            insert_completed_classification_job(transaction, &job_id, &revision_id, &assets)?;
            Ok(())
        })?;
        self.get_deck_details(&deck_id)
    }

    fn revise_complete_deck(
        &self,
        deck_id: &EntityId,
        input: SaveCompleteDeck,
    ) -> Result<DeckDetails, RepoError> {
        validate_complete_cards(&input.cards)?;
        let deck = complete_deck(&input.format, &input.cards, true);
        let assets = self.assets.current()?;
        let result = DeckClassifier::classify_confirmable(&deck, &assets)?;
        let now = UtcMillis::now();
        let revision_id = EntityId::new();
        let run_id = EntityId::new();
        let job_id = EntityId::new();
        let digest = canonical_digest(&deck)?;
        let explanation =
            serde_json::to_string(&result.explanation).map_err(|_| RepoError::NotebookInvalid)?;
        self.repository.transact_domain(|transaction| {
            let (current_revision, stored_format) = transaction
                .query_row(
                    "SELECT current_revision, format FROM deck_records
                     WHERE id = ?1 AND source_class = 'user' AND deleted_at IS NULL",
                    [deck_id.as_str()],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            if !stored_format.eq_ignore_ascii_case(&input.format) {
                return Err(RepoError::InvalidRequest);
            }
            let next_revision = current_revision
                .checked_add(1)
                .ok_or(RepoError::RevisionConflict)?;
            transaction
                .execute(
                    "INSERT INTO deck_revisions(
                        id, deck_id, revision_number, canonical_digest, complete, created_at
                     ) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                    params![
                        revision_id.as_str(),
                        deck_id.as_str(),
                        next_revision,
                        digest,
                        now.get()
                    ],
                )
                .map_err(database_error)?;
            for card in &input.cards {
                transaction
                    .execute(
                        "INSERT INTO deck_cards(
                            deck_revision_id, oracle_id, display_name, zone, quantity, basic_land
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            revision_id.as_str(),
                            card.oracle_id,
                            card.display_name,
                            card.zone.as_str(),
                            i64::from(card.quantity),
                            i64::from(card.basic_land)
                        ],
                    )
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE deck_records
                     SET current_revision = ?1, revision = revision + 1,
                         user_label = coalesce(?2, user_label)
                     WHERE id = ?3",
                    params![next_revision, input.user_label, deck_id.as_str()],
                )
                .map_err(database_error)?;
            insert_classification_run(
                transaction,
                &run_id,
                &revision_id,
                &result,
                &explanation,
                now,
            )?;
            insert_completed_classification_job(transaction, &job_id, &revision_id, &assets)?;
            Ok(())
        })?;
        self.get_deck_details(deck_id)
    }

    pub fn get_deck_details(&self, deck_id: &EntityId) -> Result<DeckDetails, RepoError> {
        self.repository.with_connection(|connection| {
            let row = connection
                .connection
                .query_row(
                    "SELECT deck.id, revision.id, revision.revision_number,
                            revision.canonical_digest, revision.complete, deck.format,
                            deck.source_class, deck.provider_label, deck.user_label
                     FROM deck_records deck
                     JOIN deck_revisions revision
                       ON revision.deck_id = deck.id
                      AND revision.revision_number = deck.current_revision
                     WHERE deck.id = ?1 AND deck.deleted_at IS NULL",
                    [deck_id.as_str()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, i64>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, String>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<String>>(8)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            let revision_id = row.1.clone();
            let mut cards_statement = connection
                .connection
                .prepare(
                    "SELECT oracle_id, display_name, zone, quantity, basic_land
                     FROM deck_cards WHERE deck_revision_id = ?1
                     ORDER BY zone, oracle_id",
                )
                .map_err(database_error)?;
            let cards = cards_statement
                .query_map([revision_id.as_str()], |row| {
                    Ok(DeckCard {
                        oracle_id: row.get(0)?,
                        display_name: row.get(1)?,
                        zone: match row.get::<_, String>(2)?.as_str() {
                            "main" => DeckZone::Main,
                            _ => DeckZone::Sideboard,
                        },
                        quantity: u16::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                        basic_land: row.get::<_, i64>(4)? == 1,
                    })
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            let public_snapshot = load_snapshot(&connection.connection, &revision_id)?;
            let classification_history =
                load_classification_history(&connection.connection, &revision_id)?;
            let current_classification = classification_history.first().cloned();
            Ok(DeckDetails {
                deck_id: row.0,
                deck_revision_id: revision_id,
                revision_number: u64::try_from(row.2).unwrap_or_default(),
                canonical_digest: row.3,
                complete: row.4 == 1,
                format: row.5,
                source_class: if row.6 == "public" {
                    DeckSourceClass::Public
                } else {
                    DeckSourceClass::User
                },
                provider_label: row.7,
                user_label: row.8,
                cards,
                public_snapshot,
                current_classification,
                classification_history,
            })
        })
    }

    pub fn get_classification(
        &self,
        deck_revision_id: &EntityId,
    ) -> Result<Vec<ClassificationRunView>, RepoError> {
        self.repository.with_connection(|connection| {
            load_classification_history(&connection.connection, deck_revision_id.as_str())
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclassificationState {
    Requested,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReclassificationProgress {
    pub job_id: String,
    pub classifier_version: String,
    pub cursor: Option<String>,
    pub completed: u64,
    pub total: u64,
    pub state: ReclassificationState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedClassifierCursor {
    classifier_version: String,
    last_deck_revision_id: Option<String>,
}

#[derive(Clone, Default)]
pub struct ReclassificationPriority {
    mtgo_foreground: Arc<AtomicBool>,
    interactive_operation: Arc<AtomicBool>,
}

impl ReclassificationPriority {
    pub fn set_mtgo_foreground(&self, foreground: bool) {
        self.mtgo_foreground.store(foreground, Ordering::SeqCst);
    }

    pub fn set_interactive_operation(&self, active: bool) {
        self.interactive_operation.store(active, Ordering::SeqCst);
    }

    fn should_yield(&self) -> bool {
        self.mtgo_foreground.load(Ordering::SeqCst)
            || self.interactive_operation.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
pub struct ReclassificationService {
    priority: ReclassificationPriority,
    active: Arc<Mutex<Option<ReclassificationProgress>>>,
}

impl ReclassificationService {
    pub fn new(priority: ReclassificationPriority) -> Self {
        Self {
            priority,
            active: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(
        &self,
        repository: &NotebookRepository,
        assets: &ClassifierAssets,
    ) -> Result<ReclassificationProgress, RepoError> {
        let mut active = self.active.lock().map_err(|_| RepoError::OperationBusy)?;
        if let Some(job) = active.as_mut() {
            if job.state == ReclassificationState::Paused {
                job.state = ReclassificationState::Requested;
                persist_progress(repository, job)?;
                return Ok(job.clone());
            }
            if matches!(
                job.state,
                ReclassificationState::Requested | ReclassificationState::Running
            ) {
                return Err(RepoError::JobBusy);
            }
        }
        if let Some(mut resumed) =
            load_resumable_job(repository, &assets.manifest.classifier_version)?
        {
            resumed.state = ReclassificationState::Requested;
            persist_progress(repository, &resumed)?;
            *active = Some(resumed.clone());
            return Ok(resumed);
        }
        let total = count_pending(repository, &assets.manifest.classifier_version)?;
        let job = ReclassificationProgress {
            job_id: EntityId::new().to_string(),
            classifier_version: assets.manifest.classifier_version.clone(),
            cursor: None,
            completed: 0,
            total,
            state: ReclassificationState::Requested,
        };
        repository.transact_domain(|transaction| {
            transaction
                .execute(
                    "INSERT INTO background_jobs(
                        id, kind, payload_version, cursor, state, priority,
                        completed, total, revision
                     ) VALUES (?1, 'classifier_reclassification', 1, ?2, 'requested',
                               100, 0, ?3, 1)",
                    params![
                        job.job_id,
                        encode_job_cursor(&job)?,
                        i64::try_from(total).unwrap_or(i64::MAX)
                    ],
                )
                .map_err(database_error)?;
            Ok(())
        })?;
        *active = Some(job.clone());
        Ok(job)
    }

    pub fn run_next_batch(
        &self,
        repository: &NotebookRepository,
        assets: &ClassifierAssets,
    ) -> Result<ReclassificationProgress, RepoError> {
        let mut active = self.active.lock().map_err(|_| RepoError::OperationBusy)?;
        let job = active.as_mut().ok_or(RepoError::InvalidRequest)?;
        if self.priority.should_yield() {
            job.state = ReclassificationState::Paused;
            persist_progress(repository, job)?;
            return Ok(job.clone());
        }
        job.state = ReclassificationState::Running;
        let pending = load_pending_revisions(
            repository,
            &assets.manifest.classifier_version,
            RECLASSIFICATION_BATCH_SIZE,
        )?;
        for (revision_id, deck) in &pending {
            let result = DeckClassifier::classify_confirmable(deck, assets)?;
            let explanation = serde_json::to_string(&result.explanation)
                .map_err(|_| RepoError::NotebookInvalid)?;
            let run_id = EntityId::new();
            repository.transact_domain(|transaction| {
                insert_classification_run(
                    transaction,
                    &run_id,
                    &EntityId::parse(revision_id.clone())?,
                    &result,
                    &explanation,
                    UtcMillis::now(),
                )
            })?;
            job.completed = job.completed.saturating_add(1);
            job.cursor = Some(revision_id.clone());
        }
        if pending.len() < RECLASSIFICATION_BATCH_SIZE {
            job.state = ReclassificationState::Completed;
        }
        persist_progress(repository, job)?;
        Ok(job.clone())
    }
}

impl Default for ReclassificationService {
    fn default() -> Self {
        Self::new(ReclassificationPriority::default())
    }
}

fn complete_deck(format: &str, cards: &[DeckCard], complete: bool) -> CompleteDeck {
    CompleteDeck {
        format: format.to_owned(),
        complete,
        cards: cards
            .iter()
            .map(|card| CanonicalCard {
                oracle_id: card.oracle_id.clone(),
                quantity: card.quantity,
                basic_land: card.basic_land,
            })
            .collect(),
    }
}

fn validate_complete_cards(cards: &[DeckCard]) -> Result<(), RepoError> {
    let main = cards
        .iter()
        .filter(|card| card.zone == DeckZone::Main)
        .map(|card| usize::from(card.quantity))
        .sum::<usize>();
    let sideboard = cards
        .iter()
        .filter(|card| card.zone == DeckZone::Sideboard)
        .map(|card| usize::from(card.quantity))
        .sum::<usize>();
    if cards.is_empty()
        || cards.len() > 500
        || !(60..=250).contains(&main)
        || sideboard > 15
        || cards.iter().any(|card| {
            card.oracle_id.trim().is_empty()
                || card.display_name.trim().is_empty()
                || card.quantity == 0
        })
    {
        return Err(RepoError::DeckIncomplete);
    }
    Ok(())
}

fn canonical_digest(deck: &CompleteDeck) -> Result<String, RepoError> {
    let mut canonical = BTreeMap::<(&str, bool), u16>::new();
    for card in &deck.cards {
        let quantity = canonical
            .entry((card.oracle_id.as_str(), card.basic_land))
            .or_default();
        *quantity = quantity.saturating_add(card.quantity);
    }
    let canonical = canonical
        .into_iter()
        .map(|((oracle_id, basic_land), quantity)| (oracle_id, basic_land, quantity))
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(deck.format.to_ascii_lowercase(), canonical))
        .map_err(|_| RepoError::NotebookInvalid)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn insert_revision(
    transaction: &rusqlite::Transaction<'_>,
    revision_id: &EntityId,
    deck_id: &EntityId,
    digest: &str,
    cards: &[DeckCard],
    created_at: UtcMillis,
) -> Result<(), RepoError> {
    transaction
        .execute(
            "INSERT INTO deck_revisions(
                id, deck_id, revision_number, canonical_digest, complete, created_at
             ) VALUES (?1, ?2, 1, ?3, 1, ?4)",
            params![
                revision_id.as_str(),
                deck_id.as_str(),
                digest,
                created_at.get()
            ],
        )
        .map_err(database_error)?;
    for card in cards {
        transaction
            .execute(
                "INSERT INTO deck_cards(
                    deck_revision_id, oracle_id, display_name, zone, quantity, basic_land
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    revision_id.as_str(),
                    card.oracle_id,
                    card.display_name,
                    card.zone.as_str(),
                    i64::from(card.quantity),
                    i64::from(card.basic_land)
                ],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn insert_classification_run(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &EntityId,
    deck_revision_id: &EntityId,
    result: &ClassificationResult,
    explanation_json: &str,
    created_at: UtcMillis,
) -> Result<(), RepoError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO classification_runs(
                id, deck_revision_id, classifier_version, classifier_digest,
                result_id, result_name, method, confidence, explanation_json,
                status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'successful', ?10)",
            params![
                run_id.as_str(),
                deck_revision_id.as_str(),
                result.classifier_version,
                result.classifier_digest,
                result.result_id,
                result.result_name,
                method_name(&result.method),
                result.confidence,
                explanation_json,
                created_at.get()
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn insert_completed_classification_job(
    transaction: &rusqlite::Transaction<'_>,
    job_id: &EntityId,
    revision_id: &EntityId,
    assets: &ClassifierAssets,
) -> Result<(), RepoError> {
    transaction
        .execute(
            "INSERT INTO background_jobs(
                id, kind, payload_version, cursor, state, priority,
                completed, total, revision
             ) VALUES (?1, 'deck_classification', 1, ?2, 'completed', 10, 1, 1, 1)",
            params![
                job_id.as_str(),
                format!(
                    "{}:{}",
                    revision_id.as_str(),
                    assets.manifest.classifier_version
                )
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn load_snapshot(
    connection: &rusqlite::Connection,
    revision_id: &str,
) -> Result<Option<PublicSnapshotView>, RepoError> {
    connection
        .query_row(
            "SELECT id, encounter_id, provider, event, format, publication_date,
                    source_url, confirmed, source_token, created_at
             FROM public_snapshots WHERE deck_revision_id = ?1
             ORDER BY created_at DESC LIMIT 1",
            [revision_id],
            |row| {
                Ok(PublicSnapshotView {
                    id: row.get(0)?,
                    encounter_id: row.get(1)?,
                    provider: row.get(2)?,
                    event: row.get(3)?,
                    format: row.get(4)?,
                    publication_date: row.get(5)?,
                    source_url: row.get(6)?,
                    confirmed: row.get::<_, i64>(7)? == 1,
                    source_token: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(database_error)
}

fn load_classification_history(
    connection: &rusqlite::Connection,
    revision_id: &str,
) -> Result<Vec<ClassificationRunView>, RepoError> {
    let mut statement = connection
        .prepare(
            "SELECT id, deck_revision_id, classifier_version, classifier_digest,
                    result_id, result_name, method, confidence, explanation_json,
                    status, created_at
             FROM classification_runs
             WHERE deck_revision_id = ?1 AND status = 'successful'
             ORDER BY created_at DESC, classifier_version DESC, id DESC",
        )
        .map_err(database_error)?;
    statement
        .query_map([revision_id], |row| {
            let explanation_json = row.get::<_, String>(8)?;
            let explanation = serde_json::from_str(&explanation_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    explanation_json.len(),
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(ClassificationRunView {
                id: row.get(0)?,
                deck_revision_id: row.get(1)?,
                classifier_version: row.get(2)?,
                classifier_digest: row.get(3)?,
                result_id: row.get(4)?,
                result_name: row.get(5)?,
                method: parse_method(&row.get::<_, String>(6)?),
                confidence: row.get(7)?,
                explanation,
                status: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)
}

fn count_pending(repository: &NotebookRepository, version: &str) -> Result<u64, RepoError> {
    repository.with_connection(|connection| {
        connection
            .connection
            .query_row(
                "SELECT count(*)
                 FROM deck_revisions revision
                 WHERE revision.complete = 1
                   AND NOT EXISTS (
                     SELECT 1 FROM classification_runs run
                     WHERE run.deck_revision_id = revision.id
                       AND run.classifier_version = ?1
                       AND run.status = 'successful'
                   )",
                [version],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)
            .and_then(|count| u64::try_from(count).map_err(|_| RepoError::NotebookInvalid))
    })
}

fn load_pending_revisions(
    repository: &NotebookRepository,
    version: &str,
    limit: usize,
) -> Result<Vec<(String, CompleteDeck)>, RepoError> {
    repository.with_connection(|connection| {
        let mut statement = connection
            .connection
            .prepare(
                "SELECT revision.id, deck.format
                 FROM deck_revisions revision
                 JOIN deck_records deck ON deck.id = revision.deck_id
                 WHERE revision.complete = 1 AND deck.deleted_at IS NULL
                   AND NOT EXISTS (
                     SELECT 1 FROM classification_runs run
                     WHERE run.deck_revision_id = revision.id
                       AND run.classifier_version = ?1
                       AND run.status = 'successful'
                   )
                 ORDER BY revision.created_at, revision.id
                 LIMIT ?2",
            )
            .map_err(database_error)?;
        let revisions = statement
            .query_map(
                params![version, i64::try_from(limit).unwrap_or(i64::MAX)],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        revisions
            .into_iter()
            .map(|(revision_id, format)| {
                let mut cards_statement = connection
                    .connection
                    .prepare(
                        "SELECT oracle_id, quantity, basic_land FROM deck_cards
                         WHERE deck_revision_id = ?1 ORDER BY oracle_id",
                    )
                    .map_err(database_error)?;
                let cards = cards_statement
                    .query_map([revision_id.as_str()], |row| {
                        Ok(CanonicalCard {
                            oracle_id: row.get(0)?,
                            quantity: u16::try_from(row.get::<_, i64>(1)?).unwrap_or_default(),
                            basic_land: row.get::<_, i64>(2)? == 1,
                        })
                    })
                    .map_err(database_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(database_error)?;
                Ok((
                    revision_id,
                    CompleteDeck {
                        format,
                        complete: true,
                        cards,
                    },
                ))
            })
            .collect()
    })
}

fn persist_progress(
    repository: &NotebookRepository,
    progress: &ReclassificationProgress,
) -> Result<(), RepoError> {
    repository.transact_domain(|transaction| {
        transaction
            .execute(
                "UPDATE background_jobs
                 SET cursor = ?1, state = ?2, completed = ?3, total = ?4,
                     revision = revision + 1
                 WHERE id = ?5",
                params![
                    encode_job_cursor(progress)?,
                    reclassification_state_name(&progress.state),
                    i64::try_from(progress.completed).unwrap_or(i64::MAX),
                    i64::try_from(progress.total).unwrap_or(i64::MAX),
                    progress.job_id
                ],
            )
            .map_err(database_error)?;
        Ok(())
    })
}

fn load_resumable_job(
    repository: &NotebookRepository,
    classifier_version: &str,
) -> Result<Option<ReclassificationProgress>, RepoError> {
    repository.with_connection(|connection| {
        let stored = connection
            .connection
            .query_row(
                "SELECT id, cursor, completed, total
                 FROM background_jobs
                 WHERE kind = 'classifier_reclassification'
                   AND state IN ('requested', 'running', 'paused')
                 ORDER BY rowid DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let Some((job_id, cursor_json, completed, total)) = stored else {
            return Ok(None);
        };
        let cursor: PersistedClassifierCursor =
            serde_json::from_str(&cursor_json).map_err(|_| RepoError::NotebookInvalid)?;
        if cursor.classifier_version != classifier_version {
            return Ok(None);
        }
        Ok(Some(ReclassificationProgress {
            job_id,
            classifier_version: cursor.classifier_version,
            cursor: cursor.last_deck_revision_id,
            completed: u64::try_from(completed).map_err(|_| RepoError::NotebookInvalid)?,
            total: u64::try_from(total).map_err(|_| RepoError::NotebookInvalid)?,
            state: ReclassificationState::Requested,
        }))
    })
}

fn encode_job_cursor(progress: &ReclassificationProgress) -> Result<String, RepoError> {
    serde_json::to_string(&PersistedClassifierCursor {
        classifier_version: progress.classifier_version.clone(),
        last_deck_revision_id: progress.cursor.clone(),
    })
    .map_err(|_| RepoError::NotebookInvalid)
}

fn method_name(method: &ClassificationMethod) -> &'static str {
    match method {
        ClassificationMethod::Signature => "signature",
        ClassificationMethod::Knn => "knn",
        ClassificationMethod::Unsupported => "unsupported",
    }
}

fn parse_method(value: &str) -> ClassificationMethod {
    match value {
        "signature" => ClassificationMethod::Signature,
        "knn" => ClassificationMethod::Knn,
        _ => ClassificationMethod::Unsupported,
    }
}

fn reclassification_state_name(state: &ReclassificationState) -> &'static str {
    match state {
        ReclassificationState::Requested => "requested",
        ReclassificationState::Running => "running",
        ReclassificationState::Paused => "paused",
        ReclassificationState::Completed => "completed",
        ReclassificationState::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::classifier::{
        DeckEnrichmentRuntime, get_classification_for, start_reclassification_for,
    };
    use crate::commands::decks::{
        ConfirmPublicSnapshotRequest, LookupOfficialDeckRequest, SaveCompleteDeckRequest,
        confirm_public_snapshot_for, lookup_official_deck_for, save_complete_deck_for,
    };
    use crate::domain::IdempotencyKey;
    use crate::ipc::{CallerIdentity, CommandResult, EventName, ReplacementEvent};
    use crate::notebook::NotebookBootstrap;
    use crate::notebook::key::KeyProtector;
    use crate::providers::decks::{PROVIDER_ID, ProviderConsent};

    use super::*;

    #[derive(Clone)]
    struct TestProtector;

    impl KeyProtector for TestProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
            Ok(plaintext.iter().map(|byte| byte ^ 0x5a).collect())
        }

        fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
            Ok(ciphertext.iter().map(|byte| byte ^ 0x5a).collect())
        }
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        runtime: crate::notebook::NotebookRuntime,
        profile_id: EntityId,
        encounter_id: EntityId,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().expect("tempdir");
            let runtime = NotebookBootstrap::new(
                directory.path().join("notebook.db"),
                directory.path().join("notebook.key"),
                TestProtector,
            )
            .initialize()
            .expect("notebook");
            let profile_id = EntityId::new();
            let now = UtcMillis::new(1_753_689_600_000).expect("time");
            runtime
                .repository
                .create_profile(&profile_id, "Opponent_42", "opponent_42", now)
                .expect("profile");
            let encounter_id = EntityId::new();
            runtime
                .repository
                .start_encounter(&encounter_id, &profile_id, now, 1)
                .expect("encounter");
            Self {
                _directory: directory,
                runtime,
                profile_id,
                encounter_id,
            }
        }

        fn enrichment(&self) -> DeckEnrichmentRuntime {
            DeckEnrichmentRuntime::builtin().expect("enrichment")
        }
    }

    fn complete_cards(marker: &str) -> Vec<DeckCard> {
        vec![
            DeckCard {
                oracle_id: format!("oracle-{marker}"),
                display_name: format!("Fixture {marker}"),
                zone: DeckZone::Main,
                quantity: 56,
                basic_land: true,
            },
            DeckCard {
                oracle_id: "oracle-lightning-bolt".into(),
                display_name: "Lightning Bolt".into(),
                zone: DeckZone::Main,
                quantity: 4,
                basic_land: false,
            },
        ]
    }

    fn public_candidate(token: &str, generation: u64) -> DeckCandidate {
        DeckCandidate {
            provider: PROVIDER_ID.into(),
            event: "Fixture Challenge".into(),
            format: "Modern".into(),
            publication_date: UtcMillis::new(1_753_689_600_000).expect("date"),
            source_url: "https://www.mtgo.com/decklists/fixture".into(),
            provider_label: Some("Provider Burn".into()),
            response_token: token.into(),
            encounter_generation: generation,
            cards: complete_cards("mountain"),
        }
    }

    fn register(provider: &OfficialDeckProvider, token: &str, generation: u64) {
        provider
            .lookup(
                &ProviderConsent::official_decks(true),
                "Opponent_42",
                "Modern",
                generation,
                token,
            )
            .expect("binding");
    }

    fn save_user(fixture: &Fixture, runtime: &DeckEnrichmentRuntime, marker: &str) -> DeckDetails {
        DeckService::new(&fixture.runtime.repository, &runtime.assets)
            .save_complete_deck(SaveCompleteDeck {
                deck_id: None,
                profile_id: fixture.profile_id.to_string(),
                format: "Modern".into(),
                user_label: Some(format!("User {marker}")),
                cards: complete_cards(marker),
            })
            .expect("user deck")
    }

    #[test]
    fn ut_066_provider_label_and_local_result_keep_separate_provenance() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let token = EntityId::new().to_string();
        register(&runtime.provider, &token, 1);
        let details = DeckService::new(&fixture.runtime.repository, &runtime.assets)
            .confirm_public_snapshot(
                &runtime.provider,
                &fixture.encounter_id,
                &public_candidate(&token, 1),
                1,
                "Modern",
            )
            .expect("snapshot");
        assert_eq!(details.provider_label.as_deref(), Some("Provider Burn"));
        assert!(details.current_classification.is_some());
        assert_ne!(
            details.provider_label.as_deref(),
            details
                .current_classification
                .as_ref()
                .map(|run| run.result_name.as_str())
        );
    }

    #[test]
    fn it_097_repeated_confirmation_stores_one_snapshot() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let token = EntityId::new().to_string();
        register(&runtime.provider, &token, 1);
        let service = DeckService::new(&fixture.runtime.repository, &runtime.assets);
        let candidate = public_candidate(&token, 1);
        let first = service
            .confirm_public_snapshot(
                &runtime.provider,
                &fixture.encounter_id,
                &candidate,
                1,
                "Modern",
            )
            .expect("first");
        let second = service
            .confirm_public_snapshot(
                &runtime.provider,
                &fixture.encounter_id,
                &candidate,
                1,
                "Modern",
            )
            .expect("repeat");
        assert_eq!(first.deck_id, second.deck_id);
        let count = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row("SELECT count(*) FROM public_snapshots", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(database_error)
            })
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn it_099_confirmed_snapshot_survives_provider_removal() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let token = EntityId::new().to_string();
        register(&runtime.provider, &token, 1);
        let details = DeckService::new(&fixture.runtime.repository, &runtime.assets)
            .confirm_public_snapshot(
                &runtime.provider,
                &fixture.encounter_id,
                &public_candidate(&token, 1),
                1,
                "Modern",
            )
            .expect("snapshot");
        drop(runtime);
        let registry = AssetRegistry::builtin().expect("assets");
        let reloaded = DeckService::new(&fixture.runtime.repository, &registry)
            .get_deck_details(&EntityId::parse(details.deck_id).expect("id"))
            .expect("persisted");
        assert_eq!(
            reloaded.public_snapshot.expect("source").provider,
            PROVIDER_ID
        );
    }

    #[test]
    fn it_100_latest_and_historical_snapshots_are_bounded() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let token = EntityId::new().to_string();
        register(&runtime.provider, &token, 1);
        DeckService::new(&fixture.runtime.repository, &runtime.assets)
            .confirm_public_snapshot(
                &runtime.provider,
                &fixture.encounter_id,
                &public_candidate(&token, 1),
                1,
                "Modern",
            )
            .expect("snapshot");
        let count = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT count(*) FROM (
                            SELECT id FROM public_snapshots
                            ORDER BY publication_date DESC, id DESC LIMIT 50
                         )",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(database_error)
            })
            .expect("bounded query");
        assert_eq!(count, 1);
    }

    #[test]
    fn it_210_confirmation_stores_snapshot_and_queues_classification() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let token = EntityId::new().to_string();
        register(&runtime.provider, &token, 1);
        let response = confirm_public_snapshot_for(
            CallerIdentity::Main,
            &fixture.runtime.repository,
            &runtime,
            ConfirmPublicSnapshotRequest {
                encounter_id: fixture.encounter_id.to_string(),
                candidate: public_candidate(&token, 1),
                active_generation: 1,
                active_format: "Modern".into(),
                idempotency_key: IdempotencyKey::new().as_str().into(),
            },
        );
        let CommandResult::Success { data, .. } = response else {
            panic!("confirmation failed")
        };
        assert!(data.public_snapshot.is_some());
        assert!(data.current_classification.is_some());
        let jobs = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT count(*) FROM background_jobs
                         WHERE kind = 'deck_classification'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(database_error)
            })
            .expect("jobs");
        assert_eq!(jobs, 1);
    }

    #[test]
    fn it_280_host_derives_disclosed_lookup_fields_from_confirmed_encounter() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let fields = serde_json::to_string(&vec!["confirmed_handle", "format"]).expect("fields");
        fixture
            .runtime
            .repository
            .set_provider_consent(PROVIDER_ID, true, &fields, UtcMillis::now())
            .expect("consent");
        let response = lookup_official_deck_for(
            CallerIdentity::Main,
            &fixture.runtime.repository,
            &runtime.provider,
            LookupOfficialDeckRequest {
                encounter_id: fixture.encounter_id.to_string(),
                encounter_generation: 1,
                request_token: EntityId::new().to_string(),
            },
        );
        let serialized = serde_json::to_value(response).expect("response");
        let url = serialized["data"]["officialUrl"]
            .as_str()
            .unwrap_or_else(|| panic!("official URL response: {serialized}"));
        assert!(url.contains("player=Opponent_42"));
        assert!(url.contains("format=Modern"));
        assert!(!url.contains("profile_id"));
        assert!(!url.contains(fixture.encounter_id.as_str()));
    }

    #[test]
    fn it_211_complete_user_deck_creates_revision_and_job() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let response = save_complete_deck_for(
            CallerIdentity::Main,
            &fixture.runtime.repository,
            &runtime,
            SaveCompleteDeckRequest {
                deck: SaveCompleteDeck {
                    deck_id: None,
                    profile_id: fixture.profile_id.to_string(),
                    format: "Modern".into(),
                    user_label: Some("User Burn".into()),
                    cards: complete_cards("user"),
                },
                idempotency_key: IdempotencyKey::new().as_str().into(),
            },
        );
        let CommandResult::Success { data, .. } = response else {
            panic!("save failed")
        };
        assert_eq!(data.source_class, DeckSourceClass::User);
        assert_eq!(data.revision_number, 1);
        assert!(data.current_classification.is_some());
    }

    #[test]
    fn it_215_deck_details_separate_provider_and_local_classification() {
        ut_066_provider_label_and_local_result_keep_separate_provenance();
    }

    #[test]
    fn it_229_classification_returns_current_and_prior_metadata_without_editor_fields() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let details = save_user(&fixture, &runtime, "history");
        let response = get_classification_for(
            CallerIdentity::Main,
            &fixture.runtime.repository,
            &runtime,
            crate::commands::classifier::ClassificationRequest {
                deck_revision_id: details.deck_revision_id,
            },
        );
        let serialized = serde_json::to_value(response).expect("serialize");
        assert_eq!(serialized["ok"], true);
        assert!(serialized["data"][0]["classifierVersion"].is_string());
        assert!(serialized["data"][0]["classifierDigest"].is_string());
        let text = serialized.to_string();
        for prohibited in ["editor", "import", "activate", "deleteDefinition"] {
            assert!(!text.contains(prohibited));
        }
    }

    #[test]
    fn it_230_start_reclassification_schedules_one_low_priority_job() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let response =
            start_reclassification_for(CallerIdentity::Main, &fixture.runtime.repository, &runtime);
        let CommandResult::Success { data, .. } = response else {
            panic!("schedule failed")
        };
        assert_eq!(data.state, ReclassificationState::Requested);
        let priority = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT priority FROM background_jobs WHERE id = ?1",
                        [data.job_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(database_error)
            })
            .expect("priority");
        assert_eq!(priority, 100);
    }

    #[test]
    fn it_248_stale_provider_token_returns_typed_error() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let response = confirm_public_snapshot_for(
            CallerIdentity::Main,
            &fixture.runtime.repository,
            &runtime,
            ConfirmPublicSnapshotRequest {
                encounter_id: fixture.encounter_id.to_string(),
                candidate: public_candidate(&EntityId::new().to_string(), 1),
                active_generation: 1,
                active_format: "Modern".into(),
                idempotency_key: IdempotencyKey::new().as_str().into(),
            },
        );
        let serialized = serde_json::to_value(response).expect("serialize");
        assert_eq!(serialized["error"]["code"], "stale_provider_result");
    }

    #[test]
    fn it_249_partial_deck_returns_deck_incomplete() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let response = save_complete_deck_for(
            CallerIdentity::Main,
            &fixture.runtime.repository,
            &runtime,
            SaveCompleteDeckRequest {
                deck: SaveCompleteDeck {
                    deck_id: None,
                    profile_id: fixture.profile_id.to_string(),
                    format: "Modern".into(),
                    user_label: None,
                    cards: vec![DeckCard {
                        oracle_id: "oracle-partial".into(),
                        display_name: "Partial".into(),
                        zone: DeckZone::Main,
                        quantity: 4,
                        basic_land: false,
                    }],
                },
                idempotency_key: IdempotencyKey::new().as_str().into(),
            },
        );
        let serialized = serde_json::to_value(response).expect("serialize");
        assert_eq!(serialized["error"]["code"], "deck_incomplete");
    }

    #[test]
    fn it_261_invalid_assets_map_to_assets_invalid_and_preserve_active() {
        let registry = AssetRegistry::builtin().expect("registry");
        let original = registry.current().expect("active");
        let result = registry.activate(crate::classifier::AssetSource {
            manifest_json: "{}",
            definitions_json: "{}",
            corpus_json: "{}",
            golden_json: "{}",
        });
        assert_eq!(result, Err(RepoError::AssetsInvalid));
        assert_eq!(
            registry.current().expect("preserved").digest,
            original.digest
        );
        assert_eq!(
            RepoError::AssetsInvalid.to_app_error().code,
            crate::ipc::ErrorCode::AssetsInvalid
        );
    }

    #[test]
    fn it_262_duplicate_running_job_returns_job_busy() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let assets = runtime.assets.current().expect("assets");
        runtime
            .reclassification
            .start(&fixture.runtime.repository, &assets)
            .expect("first");
        assert_eq!(
            runtime
                .reclassification
                .start(&fixture.runtime.repository, &assets),
            Err(RepoError::JobBusy)
        );
    }

    #[test]
    fn it_270_progress_event_contains_no_deck_data() {
        let progress = ReclassificationProgress {
            job_id: EntityId::new().to_string(),
            classifier_version: "2026.07.2".into(),
            cursor: Some("cursor".into()),
            completed: 25,
            total: 50,
            state: ReclassificationState::Running,
        };
        let event = ReplacementEvent::v1(EventName::ClassifierProgress, 2, progress);
        let serialized = serde_json::to_value(event).expect("event");
        assert_eq!(serialized["name"], "classifier://progress-v1");
        assert_eq!(serialized["payload"]["completed"], 25);
        let text = serialized.to_string();
        assert!(!text.contains("cards"));
        assert!(!text.contains("decklist"));
    }

    #[test]
    fn it_187_repeated_version_revision_has_one_successful_run() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let details = save_user(&fixture, &runtime, "unique");
        let count = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT count(*) FROM classification_runs
                         WHERE deck_revision_id = ?1",
                        [details.deck_revision_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(database_error)
            })
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn it_188_new_deck_revision_keeps_prior_run_attached() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let original = save_user(&fixture, &runtime, "revision-one");
        let revised = DeckService::new(&fixture.runtime.repository, &runtime.assets)
            .save_complete_deck(SaveCompleteDeck {
                deck_id: Some(original.deck_id.clone()),
                profile_id: fixture.profile_id.to_string(),
                format: "Modern".into(),
                user_label: Some("Revised".into()),
                cards: complete_cards("revision-two"),
            })
            .expect("revision");
        assert_eq!(revised.revision_number, 2);
        assert_ne!(original.deck_revision_id, revised.deck_revision_id);
        let count = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT count(*) FROM classification_runs
                         WHERE deck_revision_id IN (?1, ?2)",
                        params![original.deck_revision_id, revised.deck_revision_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(database_error)
            })
            .expect("runs");
        assert_eq!(count, 2);
    }

    #[test]
    fn it_189_unsupported_format_does_not_block_complete_deck_save() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let details = DeckService::new(&fixture.runtime.repository, &runtime.assets)
            .save_complete_deck(SaveCompleteDeck {
                deck_id: None,
                profile_id: fixture.profile_id.to_string(),
                format: "Pauper".into(),
                user_label: None,
                cards: complete_cards("pauper"),
            })
            .expect("confirmable");
        assert_eq!(
            details.current_classification.expect("run").method,
            ClassificationMethod::Unsupported
        );
    }

    #[test]
    fn it_186_and_it_190_batches_pause_resume_at_twenty_five() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        for index in 0..30 {
            save_user(&fixture, &runtime, &format!("batch-{index:02}"));
        }
        let mut new_assets = runtime.assets.current().expect("assets");
        new_assets.manifest.classifier_version = "2026.07.2".into();
        let priority = ReclassificationPriority::default();
        let service = ReclassificationService::new(priority.clone());
        let scheduled = service
            .start(&fixture.runtime.repository, &new_assets)
            .expect("scheduled");
        assert_eq!(scheduled.total, 30);
        priority.set_mtgo_foreground(true);
        let paused = service
            .run_next_batch(&fixture.runtime.repository, &new_assets)
            .expect("paused");
        assert_eq!(paused.state, ReclassificationState::Paused);
        assert_eq!(paused.completed, 0);
        priority.set_mtgo_foreground(false);
        service
            .start(&fixture.runtime.repository, &new_assets)
            .expect("resume");
        let first = service
            .run_next_batch(&fixture.runtime.repository, &new_assets)
            .expect("first batch");
        assert_eq!(first.completed, 25);
        assert_eq!(first.state, ReclassificationState::Running);
        let restarted = ReclassificationService::new(ReclassificationPriority::default());
        let resumed = restarted
            .start(&fixture.runtime.repository, &new_assets)
            .expect("restart resume");
        assert_eq!(resumed.job_id, first.job_id);
        assert_eq!(resumed.completed, 25);
        let done = restarted
            .run_next_batch(&fixture.runtime.repository, &new_assets)
            .expect("second batch");
        assert_eq!(done.completed, 30);
        assert_eq!(done.state, ReclassificationState::Completed);
    }

    #[test]
    fn e2e_010_official_confirmation_refreshes_without_overwrite_or_user_label_loss() {
        let fixture = Fixture::new();
        let runtime = fixture.enrichment();
        let service = DeckService::new(&fixture.runtime.repository, &runtime.assets);
        let first_token = EntityId::new().to_string();
        register(&runtime.provider, &first_token, 1);
        let first = service
            .confirm_public_snapshot(
                &runtime.provider,
                &fixture.encounter_id,
                &public_candidate(&first_token, 1),
                1,
                "Modern",
            )
            .expect("first snapshot");
        let user = save_user(&fixture, &runtime, "personal-label");
        fixture
            .runtime
            .repository
            .finish_encounter(
                &fixture.encounter_id,
                UtcMillis::new(1_753_689_700_000).expect("end"),
            )
            .expect("finish");
        let later_encounter = EntityId::new();
        fixture
            .runtime
            .repository
            .start_encounter(
                &later_encounter,
                &fixture.profile_id,
                UtcMillis::new(1_753_689_800_000).expect("start"),
                2,
            )
            .expect("later encounter");
        let second_token = EntityId::new().to_string();
        register(&runtime.provider, &second_token, 2);
        let mut second_candidate = public_candidate(&second_token, 2);
        second_candidate.publication_date =
            UtcMillis::new(1_753_689_900_000).expect("later publication");
        let second = service
            .confirm_public_snapshot(
                &runtime.provider,
                &later_encounter,
                &second_candidate,
                2,
                "Modern",
            )
            .expect("second snapshot");

        assert_ne!(first.deck_id, second.deck_id);
        assert_ne!(
            first.public_snapshot.expect("first source").id,
            second.public_snapshot.expect("second source").id
        );
        assert_eq!(user.user_label.as_deref(), Some("User personal-label"));
        let counts = fixture
            .runtime
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .query_row(
                        "SELECT
                           (SELECT count(*) FROM public_snapshots),
                           (SELECT count(*) FROM deck_records WHERE source_class = 'user')",
                        [],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .map_err(database_error)
            })
            .expect("counts");
        assert_eq!(counts, (2, 1));
    }
}
