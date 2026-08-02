use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::domain::{CardCertainty, EntityId, RepoError, Revision, UtcMillis};
use crate::notebook::repository::NotebookRepository;
use crate::services::database_error;
use crate::services::profiles::{normalize_card_name, normalize_tag};

const MAX_OBSERVATION_CHARS: usize = 4_000;
const MAX_STRUCTURED_ITEMS: usize = 500;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardObservationInput {
    #[serde(default)]
    pub oracle_id: Option<String>,
    pub display_name: String,
    #[serde(default = "default_quantity")]
    pub quantity: u16,
    pub certainty: CardCertainty,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardObservationView {
    pub oracle_id: String,
    pub display_name: String,
    pub quantity: u16,
    pub certainty: CardCertainty,
    pub context: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagView {
    pub id: String,
    pub display_label: String,
    pub normalized_label: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDetail {
    pub id: String,
    pub encounter_id: String,
    pub text: String,
    pub encounter_started_at: i64,
    pub created_at: i64,
    pub edited_at: Option<i64>,
    pub revision: u64,
    pub cards: Vec<CardObservationView>,
    pub tags: Vec<TagView>,
    pub user_deck_label: Option<String>,
    pub source: String,
}

pub struct ObservationService<'a> {
    repository: &'a NotebookRepository,
}

impl<'a> ObservationService<'a> {
    pub fn new(repository: &'a NotebookRepository) -> Self {
        Self { repository }
    }

    pub fn create(
        &self,
        encounter_id: &EntityId,
        text: &str,
    ) -> Result<ObservationDetail, RepoError> {
        let text = validate_observation_text(text)?;
        let exists = self.repository.with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM encounters
                        WHERE id = ?1 AND deleted_at IS NULL
                    )",
                    [encounter_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)
        })?;
        if exists == 0 {
            return Err(RepoError::NotFound);
        }
        let observation_id = EntityId::new();
        self.repository.add_observation(
            &observation_id,
            encounter_id,
            &text,
            UtcMillis::now(),
            true,
        )?;
        self.get(&observation_id)
    }

    pub fn validate_enrichment(
        cards: &[CardObservationInput],
        tags: &[String],
        user_deck_label: Option<&str>,
    ) -> Result<(), RepoError> {
        normalize_cards(cards.to_vec())?;
        for tag in tags {
            normalize_tag(tag)?;
        }
        if let Some(label) = user_deck_label {
            normalize_tag(label)?;
        }
        Ok(())
    }

    pub fn update_text(
        &self,
        observation_id: &EntityId,
        expected_revision: Revision,
        text: &str,
    ) -> Result<ObservationDetail, RepoError> {
        let text = validate_observation_text(text)?;
        self.repository.update_observation(
            observation_id,
            expected_revision,
            &text,
            UtcMillis::now(),
        )?;
        self.get(observation_id)
    }

    pub fn set_cards(
        &self,
        observation_id: &EntityId,
        expected_revision: Revision,
        cards: Vec<CardObservationInput>,
    ) -> Result<ObservationDetail, RepoError> {
        let cards = normalize_cards(cards)?;
        let now = UtcMillis::now();
        self.repository.transact_domain(|transaction| {
            require_observation_revision(transaction, observation_id, expected_revision)?;
            transaction
                .execute(
                    "DELETE FROM card_observations WHERE observation_id = ?1",
                    [observation_id.as_str()],
                )
                .map_err(database_error)?;
            for card in &cards {
                transaction
                    .execute(
                        "INSERT INTO card_observations(
                            observation_id, oracle_id, display_name, quantity, certainty, context
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            observation_id.as_str(),
                            card.oracle_id,
                            card.display_name,
                            i64::from(card.quantity),
                            certainty_name(card.certainty),
                            card.context
                        ],
                    )
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE observations
                     SET edited_at = ?1, revision = revision + 1
                     WHERE id = ?2",
                    params![now.get(), observation_id.as_str()],
                )
                .map_err(database_error)?;
            Ok(())
        })?;
        self.get(observation_id)
    }

    pub fn set_tags(
        &self,
        observation_id: &EntityId,
        expected_revision: Revision,
        tags: Vec<String>,
    ) -> Result<ObservationDetail, RepoError> {
        if tags.len() > MAX_STRUCTURED_ITEMS {
            return Err(RepoError::InvalidRequest);
        }
        let mut normalized = BTreeMap::new();
        for tag in tags {
            let tag = normalize_tag(&tag)?;
            normalized.entry(tag.key).or_insert(tag.display);
        }
        let now = UtcMillis::now();
        self.repository.transact_domain(|transaction| {
            require_observation_revision(transaction, observation_id, expected_revision)?;
            transaction
                .execute(
                    "DELETE FROM observation_tags WHERE observation_id = ?1",
                    [observation_id.as_str()],
                )
                .map_err(database_error)?;
            for (key, display) in &normalized {
                let tag_id = transaction
                    .query_row(
                        "SELECT id FROM tendency_tags WHERE normalized_label = ?1",
                        [key],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(database_error)?
                    .unwrap_or_else(|| EntityId::new().to_string());
                transaction
                    .execute(
                        "INSERT OR IGNORE INTO tendency_tags(id, normalized_label, display_label)
                         VALUES (?1, ?2, ?3)",
                        params![tag_id, key, display],
                    )
                    .map_err(database_error)?;
                transaction
                    .execute(
                        "INSERT INTO observation_tags(observation_id, tag_id) VALUES (?1, ?2)",
                        params![observation_id.as_str(), tag_id],
                    )
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE observations
                     SET edited_at = ?1, revision = revision + 1
                     WHERE id = ?2",
                    params![now.get(), observation_id.as_str()],
                )
                .map_err(database_error)?;
            Ok(())
        })?;
        self.get(observation_id)
    }

    pub fn save_user_deck_label(
        &self,
        observation_id: &EntityId,
        expected_revision: Revision,
        label: Option<&str>,
    ) -> Result<ObservationDetail, RepoError> {
        let label = label
            .map(normalize_tag)
            .transpose()?
            .map(|value| value.display);
        let now = UtcMillis::now();
        self.repository.transact_domain(|transaction| {
            let (_, encounter_id, profile_id) =
                require_observation_revision(transaction, observation_id, expected_revision)?;
            if let Some(label) = &label {
                transaction
                    .execute(
                        "INSERT INTO deck_records(
                            id, profile_id, source_class, format, completeness, user_label,
                            current_revision, revision, created_at
                         ) VALUES (?1, ?2, 'user', 'Modern', 'partial', ?3, 1, 1, ?4)",
                        params![EntityId::new().as_str(), profile_id, label, now.get()],
                    )
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE observations SET edited_at = ?1, revision = revision + 1
                     WHERE id = ?2 AND encounter_id = ?3",
                    params![now.get(), observation_id.as_str(), encounter_id],
                )
                .map_err(database_error)?;
            Ok(())
        })?;
        self.get(observation_id)
    }

    pub fn get(&self, observation_id: &EntityId) -> Result<ObservationDetail, RepoError> {
        self.repository.with_connection(|connection| {
            let base = connection
                .connection
                .query_row(
                    "SELECT observation.id, observation.encounter_id, observation.text,
                            encounter.started_at, observation.created_at, observation.edited_at,
                            observation.revision, encounter.profile_id
                     FROM observations observation
                     JOIN encounters encounter ON encounter.id = observation.encounter_id
                     JOIN opponent_profiles profile ON profile.id = encounter.profile_id
                     WHERE observation.id = ?1
                       AND observation.deleted_at IS NULL
                       AND encounter.deleted_at IS NULL
                       AND profile.deleted_at IS NULL",
                    [observation_id.as_str()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, i64>(4)?,
                            row.get::<_, Option<i64>>(5)?,
                            row.get::<_, i64>(6)?,
                            row.get::<_, String>(7)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?
                .ok_or(RepoError::NotFound)?;
            let cards = {
                let mut statement = connection
                    .connection
                    .prepare(
                        "SELECT oracle_id, display_name, quantity, certainty, context
                         FROM card_observations
                         WHERE observation_id = ?1
                         ORDER BY certainty, display_name, oracle_id",
                    )
                    .map_err(database_error)?;
                statement
                    .query_map([observation_id.as_str()], |row| {
                        Ok(CardObservationView {
                            oracle_id: row.get(0)?,
                            display_name: row.get(1)?,
                            quantity: u16::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                            certainty: parse_certainty(&row.get::<_, String>(3)?)
                                .map_err(|_| rusqlite::Error::InvalidQuery)?,
                            context: row.get(4)?,
                        })
                    })
                    .map_err(database_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(database_error)?
            };
            let tags = {
                let mut statement = connection
                    .connection
                    .prepare(
                        "SELECT tag.id, tag.display_label, tag.normalized_label
                         FROM observation_tags link
                         JOIN tendency_tags tag ON tag.id = link.tag_id
                         WHERE link.observation_id = ?1
                         ORDER BY tag.normalized_label, tag.id",
                    )
                    .map_err(database_error)?;
                statement
                    .query_map([observation_id.as_str()], |row| {
                        Ok(TagView {
                            id: row.get(0)?,
                            display_label: row.get(1)?,
                            normalized_label: row.get(2)?,
                        })
                    })
                    .map_err(database_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(database_error)?
            };
            let user_deck_label = connection
                .connection
                .query_row(
                    "SELECT user_label FROM deck_records
                     WHERE profile_id = ?1 AND source_class = 'user'
                       AND deleted_at IS NULL AND created_at >= ?2
                     ORDER BY created_at DESC, id DESC LIMIT 1",
                    params![base.7, base.3],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(database_error)?
                .flatten();
            Ok(ObservationDetail {
                id: base.0,
                encounter_id: base.1,
                text: base.2,
                encounter_started_at: base.3,
                created_at: base.4,
                edited_at: base.5,
                revision: u64::try_from(base.6).map_err(|_| RepoError::NotebookInvalid)?,
                cards,
                tags,
                user_deck_label,
                source: "player_observation".to_owned(),
            })
        })
    }

    pub fn tag_suggestions(&self, query: &str, limit: usize) -> Result<Vec<TagView>, RepoError> {
        let normalized = normalize_tag(query)?;
        let limit = limit.min(50);
        self.repository.with_connection(|connection| {
            let mut statement = connection
                .connection
                .prepare(
                    "SELECT id, display_label, normalized_label
                     FROM tendency_tags
                     WHERE retired_at IS NULL
                       AND normalized_label LIKE ?1 || '%'
                     ORDER BY normalized_label, id
                     LIMIT ?2",
                )
                .map_err(database_error)?;
            statement
                .query_map(
                    params![
                        normalized.key,
                        i64::try_from(limit).map_err(|_| RepoError::InvalidRequest)?
                    ],
                    |row| {
                        Ok(TagView {
                            id: row.get(0)?,
                            display_label: row.get(1)?,
                            normalized_label: row.get(2)?,
                        })
                    },
                )
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)
        })
    }

    pub fn retire_tag(&self, tag_id: &EntityId, now: UtcMillis) -> Result<(), RepoError> {
        self.repository.transact_domain(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE tendency_tags SET retired_at = ?1
                     WHERE id = ?2 AND retired_at IS NULL",
                    params![now.get(), tag_id.as_str()],
                )
                .map_err(database_error)?;
            if changed == 0 {
                return Err(RepoError::NotFound);
            }
            Ok(())
        })
    }
}

fn default_quantity() -> u16 {
    1
}

fn validate_observation_text(value: &str) -> Result<String, RepoError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RepoError::BlankObservation);
    }
    if value.chars().count() > MAX_OBSERVATION_CHARS {
        return Err(RepoError::InvalidRequest);
    }
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(RepoError::InvalidRequest);
    }
    Ok(trimmed.to_owned())
}

fn normalize_cards(
    cards: Vec<CardObservationInput>,
) -> Result<Vec<CardObservationView>, RepoError> {
    if cards.len() > MAX_STRUCTURED_ITEMS {
        return Err(RepoError::InvalidCard);
    }
    let mut consolidated: BTreeMap<(String, u8), CardObservationView> = BTreeMap::new();
    let mut contexts: BTreeMap<(String, u8), BTreeSet<String>> = BTreeMap::new();
    for card in cards {
        if card.quantity == 0 || card.quantity > 99 {
            return Err(RepoError::InvalidCard);
        }
        let name = normalize_card_name(&card.display_name)?;
        let oracle_id = match card.oracle_id {
            Some(value) if !value.trim().is_empty() => {
                let value = value.trim().to_owned();
                if value
                    .chars()
                    .any(|character| character.is_control() || matches!(character, '<' | '>'))
                {
                    return Err(RepoError::InvalidCard);
                }
                value
            }
            _ => format!("name:{}", name.key),
        };
        let certainty_order = match card.certainty {
            CardCertainty::Observed => 0,
            CardCertainty::Suspected => 1,
        };
        let key = (oracle_id.clone(), certainty_order);
        let entry = consolidated
            .entry(key.clone())
            .or_insert(CardObservationView {
                oracle_id,
                display_name: name.display,
                quantity: card.quantity,
                certainty: card.certainty,
                context: None,
            });
        entry.quantity = entry.quantity.max(card.quantity);
        if let Some(context) = card.context {
            let context = context.trim().to_owned();
            if context.chars().count() > 1_000
                || context
                    .chars()
                    .any(|character| character.is_control() && character != '\t')
            {
                return Err(RepoError::InvalidCard);
            }
            if !context.is_empty() {
                contexts.entry(key).or_default().insert(context);
            }
        }
    }
    for (key, values) in contexts {
        if let Some(card) = consolidated.get_mut(&key) {
            card.context = Some(values.into_iter().collect::<Vec<_>>().join(" · "));
        }
    }
    Ok(consolidated.into_values().collect())
}

fn require_observation_revision(
    transaction: &rusqlite::Transaction<'_>,
    observation_id: &EntityId,
    expected_revision: Revision,
) -> Result<(i64, String, String), RepoError> {
    let row = transaction
        .query_row(
            "SELECT observation.revision, observation.encounter_id, encounter.profile_id
             FROM observations observation
             JOIN encounters encounter ON encounter.id = observation.encounter_id
             WHERE observation.id = ?1
               AND observation.deleted_at IS NULL
               AND encounter.deleted_at IS NULL",
            [observation_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or(RepoError::NotFound)?;
    if row.0 != i64::try_from(expected_revision.get()).map_err(|_| RepoError::InvalidRequest)? {
        return Err(RepoError::RevisionConflict);
    }
    Ok(row)
}

fn certainty_name(certainty: CardCertainty) -> &'static str {
    match certainty {
        CardCertainty::Observed => "observed",
        CardCertainty::Suspected => "suspected",
    }
}

fn parse_certainty(value: &str) -> Result<CardCertainty, RepoError> {
    match value {
        "observed" => Ok(CardCertainty::Observed),
        "suspected" => Ok(CardCertainty::Suspected),
        _ => Err(RepoError::NotebookInvalid),
    }
}
