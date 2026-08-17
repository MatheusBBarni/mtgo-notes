use tempfile::TempDir;

use super::deletion::{DeletionEntityType, DeletionService};
use super::observations::{CardObservationInput, ObservationService};
use super::profiles::ProfileService;
use crate::commands::notes::{
    SaveObservationRequest, SetCardObservationsRequest, SetTendencyTagsRequest,
    UpdateObservationRequest, save_observation_for, set_card_observations_for,
    set_tendency_tags_for, update_observation_for,
};
use crate::domain::{CardCertainty, EntityId, IdempotencyKey, RepoError, Revision, UtcMillis};
use crate::ipc::CallerIdentity;
use crate::notebook::NotebookBootstrap;
use crate::notebook::key::KeyProtector;
use crate::notebook::repository::NotebookRepository;

#[derive(Clone)]
struct Protector;

impl KeyProtector for Protector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Ok(plaintext.iter().map(|byte| byte ^ 0xa5).collect())
    }

    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Ok(ciphertext.iter().map(|byte| byte ^ 0xa5).collect())
    }
}

struct Fixture {
    _directory: TempDir,
    repository: NotebookRepository,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = NotebookBootstrap::new(
            directory.path().join("notebook.db"),
            directory.path().join("notebook.key"),
            Protector,
        )
        .initialize()
        .expect("notebook");
        Self {
            _directory: directory,
            repository: runtime.repository,
        }
    }

    fn encounter(&self, handle: &str) -> EntityId {
        let profile = ProfileService::new(&self.repository)
            .create(handle)
            .expect("profile");
        let encounter = EntityId::new();
        self.repository
            .start_encounter(&encounter, &profile.profile.id, UtcMillis::now(), 1)
            .expect("encounter");
        encounter
    }

    fn observation(&self, handle: &str, text: &str) -> super::observations::ObservationDetail {
        let encounter = self.encounter(handle);
        ObservationService::new(&self.repository)
            .create(&encounter, text)
            .expect("observation")
    }
}

fn revision(value: u64) -> Revision {
    Revision::new(value).expect("revision")
}

fn observation_id(value: &str) -> EntityId {
    EntityId::parse(value).expect("observation id")
}

fn card(name: impl Into<String>, certainty: CardCertainty) -> CardObservationInput {
    CardObservationInput {
        oracle_id: None,
        display_name: name.into(),
        quantity: 1,
        certainty,
        context: None,
    }
}

#[test]
fn it_071_invalid_card_or_tag_is_field_specific_and_preserves_observation() {
    let fixture = Fixture::new();
    let note = fixture.observation("Invalid_Structure", "Original note");
    let service = ObservationService::new(&fixture.repository);

    assert_eq!(
        service.set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![card("<script>", CardCertainty::Observed)],
        ),
        Err(RepoError::InvalidCard)
    );
    assert_eq!(
        service.set_tags(
            &observation_id(&note.id),
            revision(note.revision),
            vec!["<script>".to_owned()],
        ),
        Err(RepoError::InvalidTag)
    );

    let card_error = serde_json::to_value(set_card_observations_for(
        CallerIdentity::Main,
        &fixture.repository,
        SetCardObservationsRequest {
            observation_id: note.id.clone(),
            expected_revision: note.revision,
            cards: vec![card("", CardCertainty::Observed)],
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ))
    .expect("card error");
    let tag_error = serde_json::to_value(set_tendency_tags_for(
        CallerIdentity::Main,
        &fixture.repository,
        SetTendencyTagsRequest {
            observation_id: note.id.clone(),
            expected_revision: note.revision,
            tags: vec!["\0".to_owned()],
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ))
    .expect("tag error");
    assert_eq!(card_error["error"]["field"], "cards");
    assert_eq!(tag_error["error"]["field"], "tags");
    assert_eq!(
        service
            .get(&observation_id(&note.id))
            .expect("saved note")
            .text,
        "Original note"
    );

    let encounter = EntityId::parse(note.encounter_id.clone()).expect("encounter id");
    let before = fixture
        .repository
        .snapshot()
        .expect("before invalid save")
        .observation_count;
    let save_error = serde_json::to_value(save_observation_for(
        CallerIdentity::Main,
        &fixture.repository,
        SaveObservationRequest {
            encounter_id: encounter.to_string(),
            text: "Draft remains renderer-owned".to_owned(),
            cards: vec![card("", CardCertainty::Observed)],
            tags: Vec::new(),
            user_deck_label: None,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ))
    .expect("save error");
    assert_eq!(save_error["error"]["field"], "cards");
    assert_eq!(
        fixture
            .repository
            .snapshot()
            .expect("after invalid save")
            .observation_count,
        before
    );
}

#[test]
fn it_072_free_text_only_observation_has_empty_structure() {
    let fixture = Fixture::new();
    let note = fixture.observation("Free_Text", "Only the required field");
    assert!(note.cards.is_empty());
    assert!(note.tags.is_empty());
    assert!(note.user_deck_label.is_none());
}

#[test]
fn it_073_large_structured_note_retains_all_data_and_bounded_suggestions() {
    let fixture = Fixture::new();
    let note = fixture.observation("Large_Structure", "Many facts");
    let service = ObservationService::new(&fixture.repository);
    let cards = (0..200)
        .map(|index| card(format!("Card {index:03}"), CardCertainty::Observed))
        .collect();
    let with_cards = service
        .set_cards(&observation_id(&note.id), revision(note.revision), cards)
        .expect("cards");
    let tags = (0..200).map(|index| format!("Tag {index:03}")).collect();
    let structured = service
        .set_tags(
            &observation_id(&note.id),
            revision(with_cards.revision),
            tags,
        )
        .expect("tags");
    assert_eq!(structured.cards.len(), 200);
    assert_eq!(structured.tags.len(), 200);
    assert_eq!(
        service.tag_suggestions("tag", 500).expect("bounded").len(),
        50
    );
}

#[test]
fn it_074_offline_free_card_text_and_user_tag_save_without_reference_data() {
    let fixture = Fixture::new();
    let note = fixture.observation("Offline_Structure", "Local-only fact");
    let service = ObservationService::new(&fixture.repository);
    let with_card = service
        .set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![card("Unknown Local Card", CardCertainty::Suspected)],
        )
        .expect("free card");
    let with_tag = service
        .set_tags(
            &observation_id(&note.id),
            revision(with_card.revision),
            vec!["Custom local tendency".to_owned()],
        )
        .expect("tag");
    assert_eq!(with_tag.cards[0].oracle_id, "name:unknown local card");
    assert_eq!(with_tag.tags[0].display_label, "Custom local tendency");
}

#[test]
fn it_075_duplicate_normalized_cards_and_tags_consolidate_with_context() {
    let fixture = Fixture::new();
    let note = fixture.observation("Duplicate_Structure", "Duplicate facts");
    let service = ObservationService::new(&fixture.repository);
    let with_cards = service
        .set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![
                CardObservationInput {
                    context: Some("first".to_owned()),
                    ..card("Ｆａｔａｌ Push", CardCertainty::Observed)
                },
                CardObservationInput {
                    context: Some("second".to_owned()),
                    ..card("fatal push", CardCertainty::Observed)
                },
            ],
        )
        .expect("cards");
    let structured = service
        .set_tags(
            &observation_id(&note.id),
            revision(with_cards.revision),
            vec!["Fast".to_owned(), "ｆａｓｔ".to_owned()],
        )
        .expect("tags");
    assert_eq!(structured.cards.len(), 1);
    assert_eq!(structured.tags.len(), 1);
    let context = structured.cards[0].context.as_deref().expect("context");
    assert!(context.contains("first") && context.contains("second"));
}

#[test]
fn it_076_unsubmitted_structure_does_not_change_committed_free_text() {
    let fixture = Fixture::new();
    let note = fixture.observation("Unsaved_Structure", "Committed text");
    let stored = ObservationService::new(&fixture.repository)
        .get(&observation_id(&note.id))
        .expect("stored");
    assert_eq!(stored.text, "Committed text");
    assert!(stored.edited_at.is_none());
    assert!(stored.cards.is_empty());
}

#[test]
fn it_077_repeated_observed_card_is_one_normalized_fact() {
    let fixture = Fixture::new();
    let note = fixture.observation("Repeated_Card", "Saw removal");
    let stored = ObservationService::new(&fixture.repository)
        .set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![
                card("Lightning Bolt", CardCertainty::Observed),
                card("lightning bolt", CardCertainty::Observed),
            ],
        )
        .expect("cards");
    assert_eq!(stored.cards.len(), 1);
}

#[test]
fn it_078_suspected_to_observed_keeps_provenance_and_increments_revision() {
    let fixture = Fixture::new();
    let note = fixture.observation("Certainty_Change", "Possible card");
    let service = ObservationService::new(&fixture.repository);
    let suspected = service
        .set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![card("Counterspell", CardCertainty::Suspected)],
        )
        .expect("suspected");
    let observed = service
        .set_cards(
            &observation_id(&note.id),
            revision(suspected.revision),
            vec![card("Counterspell", CardCertainty::Observed)],
        )
        .expect("observed");
    assert_eq!(observed.encounter_started_at, note.encounter_started_at);
    assert!(observed.revision > suspected.revision);
    assert!(observed.edited_at.is_some());
}

#[test]
fn it_079_retired_tag_leaves_historical_display_text_but_not_suggestions() {
    let fixture = Fixture::new();
    let note = fixture.observation("Retired_Tag", "Historical tendency");
    let service = ObservationService::new(&fixture.repository);
    let tagged = service
        .set_tags(
            &observation_id(&note.id),
            revision(note.revision),
            vec!["Deliberate pace".to_owned()],
        )
        .expect("tag");
    let tag_id = EntityId::parse(tagged.tags[0].id.clone()).expect("tag id");
    service
        .retire_tag(&tag_id, UtcMillis::now())
        .expect("retire tag");
    let historical = service.get(&observation_id(&note.id)).expect("historical");
    assert_eq!(historical.tags[0].display_label, "Deliberate pace");
    assert!(
        service
            .tag_suggestions("deliberate", 50)
            .expect("suggestions")
            .is_empty()
    );
}

#[test]
fn it_080_ten_thousand_tags_return_only_a_bounded_page() {
    let fixture = Fixture::new();
    fixture
        .repository
        .transact_domain(|transaction| {
            let mut statement = transaction
                .prepare(
                    "INSERT INTO tendency_tags(
                        id, normalized_label, display_label, retired_at
                     ) VALUES (?1, ?2, ?3, NULL)",
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            for index in 0..10_000 {
                statement
                    .execute((
                        EntityId::new().as_str(),
                        format!("scale tag {index:05}"),
                        format!("Scale Tag {index:05}"),
                    ))
                    .map_err(|_| RepoError::NotebookInvalid)?;
            }
            Ok(())
        })
        .expect("seed tags");
    let page = ObservationService::new(&fixture.repository)
        .tag_suggestions("scale tag", 10_000)
        .expect("suggestions");
    assert_eq!(page.len(), 50);
}

#[test]
fn it_081_invalid_edit_preserves_prior_text_and_revision() {
    let fixture = Fixture::new();
    let note = fixture.observation("Invalid_Edit", "Saved version");
    assert_eq!(
        ObservationService::new(&fixture.repository).update_text(
            &observation_id(&note.id),
            revision(note.revision),
            " \n ",
        ),
        Err(RepoError::BlankObservation)
    );
    let stored = ObservationService::new(&fixture.repository)
        .get(&observation_id(&note.id))
        .expect("stored");
    assert_eq!(stored.text, "Saved version");
    assert_eq!(stored.revision, note.revision);
}

#[test]
fn it_082_finishing_note_free_encounter_creates_no_placeholder() {
    let fixture = Fixture::new();
    let encounter = fixture.encounter("No_Notes");
    fixture
        .repository
        .finish_encounter(&encounter, UtcMillis::now())
        .expect("finish");
    assert_eq!(
        fixture
            .repository
            .snapshot()
            .expect("snapshot")
            .observation_count,
        0
    );
}

#[test]
fn it_083_bulk_review_changes_selected_revisions_only() {
    let fixture = Fixture::new();
    let encounter = fixture.encounter("Bulk_Review");
    let service = ObservationService::new(&fixture.repository);
    let first = service.create(&encounter, "First").expect("first");
    let second = service.create(&encounter, "Second").expect("second");
    let third = service.create(&encounter, "Third").expect("third");
    let first_updated = service
        .update_text(
            &observation_id(&first.id),
            revision(first.revision),
            "First edited",
        )
        .expect("first edited");
    let third_updated = service
        .update_text(
            &observation_id(&third.id),
            revision(third.revision),
            "Third edited",
        )
        .expect("third edited");
    let second_stored = service.get(&observation_id(&second.id)).expect("second");
    assert!(first_updated.revision > first.revision);
    assert!(third_updated.revision > third.revision);
    assert_eq!(second_stored.revision, second.revision);
}

#[test]
fn it_084_injected_mutation_denial_leaves_prior_state_visible() {
    let fixture = Fixture::new();
    let note = fixture.observation("Denied_Edit", "Prior state");
    fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute_batch(
                    "CREATE TRIGGER deny_observation_update
                     BEFORE UPDATE ON observations
                     BEGIN
                         SELECT RAISE(ABORT, 'injected mutation denial');
                     END;",
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("failure trigger");
    let result = update_observation_for(
        CallerIdentity::Main,
        &fixture.repository,
        UpdateObservationRequest {
            observation_id: note.id.clone(),
            text: "Denied".to_owned(),
            expected_revision: note.revision,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    );
    assert!(!result.is_success());
    assert_eq!(
        ObservationService::new(&fixture.repository)
            .get(&observation_id(&note.id))
            .expect("stored")
            .text,
        "Prior state"
    );
}

#[test]
fn it_085_edit_delete_race_has_one_explicit_revision_winner() {
    let fixture = Fixture::new();
    let note = fixture.observation("Edit_Delete_Race", "Original");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    assert_eq!(
        ObservationService::new(&fixture.repository).update_text(
            &observation_id(&note.id),
            revision(note.revision),
            "Late edit",
        ),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_086_restart_recovers_pending_undo_and_never_resurrects_after_purge() {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = directory.path().join("notebook.db");
    let key = directory.path().join("notebook.key");
    let runtime = NotebookBootstrap::new(&database, &key, Protector)
        .initialize()
        .expect("notebook");
    let profile = ProfileService::new(&runtime.repository)
        .create("Restart_Undo")
        .expect("profile");
    let deletion = DeletionService::new(&runtime.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, profile.profile.id.as_str())
        .expect("preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    drop(runtime);

    let runtime = NotebookBootstrap::new(&database, &key, Protector)
        .initialize()
        .expect("reopen");
    DeletionService::new(&runtime.repository)
        .undo(
            DeletionEntityType::Profile,
            profile.profile.id.as_str(),
            &pending.undo_token,
            UtcMillis::new(pending.undo_deadline).expect("deadline"),
        )
        .expect("undo after restart");
    let preview = DeletionService::new(&runtime.repository)
        .preview(DeletionEntityType::Profile, profile.profile.id.as_str())
        .expect("second preview");
    let pending = DeletionService::new(&runtime.repository)
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("second delete");
    DeletionService::new(&runtime.repository)
        .purge_due(UtcMillis::new(pending.undo_deadline).expect("deadline"))
        .expect("purge");
    drop(runtime);

    let runtime = NotebookBootstrap::new(&database, &key, Protector)
        .initialize()
        .expect("reopen after purge");
    assert_eq!(
        ProfileService::new(&runtime.repository).get(&profile.profile.id),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_087_repeated_delete_key_returns_same_tombstone() {
    let fixture = Fixture::new();
    let note = fixture.observation("Repeated_Delete", "Delete me");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    let key = IdempotencyKey::new();
    let first = deletion
        .request(&preview, &preview.confirmation, &key)
        .expect("first");
    let second = deletion
        .request(&preview, &preview.confirmation, &key)
        .expect("second");
    assert_eq!(first, second);
}

#[test]
fn it_088_edit_after_merge_updates_original_encounter_in_canonical_view() {
    let fixture = Fixture::new();
    let primary = ProfileService::new(&fixture.repository)
        .create("Canonical_Edit")
        .expect("primary");
    let secondary = ProfileService::new(&fixture.repository)
        .create("Merged_Edit")
        .expect("secondary");
    let encounter = EntityId::new();
    fixture
        .repository
        .start_encounter(&encounter, &secondary.profile.id, UtcMillis::now(), 1)
        .expect("encounter");
    let note = ObservationService::new(&fixture.repository)
        .create(&encounter, "Before merge")
        .expect("note");
    let identity = super::identity::IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(
            &primary.profile.id,
            &secondary.profile.id,
            &primary.profile.id,
        )
        .expect("preview");
    identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    let edited = ObservationService::new(&fixture.repository)
        .update_text(
            &observation_id(&note.id),
            revision(note.revision),
            "After merge",
        )
        .expect("edit");
    assert_eq!(edited.encounter_id, encounter.to_string());
}

#[test]
fn it_089_edit_under_purged_encounter_returns_not_found() {
    let fixture = Fixture::new();
    let encounter = fixture.encounter("Purged_Encounter");
    let note = ObservationService::new(&fixture.repository)
        .create(&encounter, "Purged note")
        .expect("note");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Encounter, encounter.as_str())
        .expect("preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    deletion
        .purge_due(UtcMillis::new(pending.undo_deadline).expect("deadline"))
        .expect("purge");
    assert_eq!(
        ObservationService::new(&fixture.repository).update_text(
            &observation_id(&note.id),
            revision(note.revision),
            "Too late",
        ),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_090_edit_in_large_history_preserves_unrelated_order_and_revisions() {
    let fixture = Fixture::new();
    let encounter = fixture.encounter("Large_History_Edit");
    let service = ObservationService::new(&fixture.repository);
    let notes = (0..100)
        .map(|index| {
            service
                .create(&encounter, &format!("Note {index:03}"))
                .expect("note")
        })
        .collect::<Vec<_>>();
    let before = notes
        .iter()
        .map(|note| (note.id.clone(), note.revision))
        .collect::<Vec<_>>();
    let target = &notes[50];
    service
        .update_text(
            &observation_id(&target.id),
            revision(target.revision),
            "Only this note changed",
        )
        .expect("edit");
    for (id, expected_revision) in before {
        let stored = service.get(&observation_id(&id)).expect("stored");
        if id == target.id {
            assert!(stored.revision > expected_revision);
        } else {
            assert_eq!(stored.revision, expected_revision);
        }
    }
}

#[test]
fn e2e_008_free_text_structure_and_post_encounter_edit_keep_provenance() {
    let fixture = Fixture::new();
    let encounter = fixture.encounter("Structured_Journey");
    let service = ObservationService::new(&fixture.repository);
    let note = service
        .create(&encounter, "Free text first")
        .expect("free text");
    let with_cards = service
        .set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![
                CardObservationInput {
                    context: Some("cast in game one".to_owned()),
                    ..card("Fatal Push", CardCertainty::Observed)
                },
                card("Subtlety", CardCertainty::Suspected),
            ],
        )
        .expect("cards");
    let with_tags = service
        .set_tags(
            &observation_id(&note.id),
            revision(with_cards.revision),
            vec!["Patient".to_owned()],
        )
        .expect("tags");
    fixture
        .repository
        .finish_encounter(&encounter, UtcMillis::now())
        .expect("finish");
    let edited = service
        .update_text(
            &observation_id(&note.id),
            revision(with_tags.revision),
            "Reviewed after encounter",
        )
        .expect("edit");
    assert_eq!(edited.encounter_id, encounter.to_string());
    assert_eq!(edited.cards.len(), 2);
    assert_eq!(edited.tags.len(), 1);
    assert!(edited.edited_at.is_some());
}

#[test]
fn it_208_set_card_observations_command_atomically_replaces_normalized_entries() {
    let fixture = Fixture::new();
    let note = fixture.observation("Command_Cards", "Structured command");
    let seeded = ObservationService::new(&fixture.repository)
        .set_cards(
            &observation_id(&note.id),
            revision(note.revision),
            vec![card("Old card", CardCertainty::Observed)],
        )
        .expect("seed cards");
    let result = set_card_observations_for(
        CallerIdentity::Main,
        &fixture.repository,
        SetCardObservationsRequest {
            observation_id: note.id.clone(),
            expected_revision: seeded.revision,
            cards: vec![
                card("Fatal Push", CardCertainty::Observed),
                card("fatal push", CardCertainty::Observed),
                card("Subtlety", CardCertainty::Suspected),
            ],
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    );
    let value = serde_json::to_value(result).expect("command");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["cards"].as_array().expect("cards").len(), 2);
    assert!(
        value["data"]["cards"]
            .as_array()
            .expect("cards")
            .iter()
            .all(|card| card["displayName"] != "Old card")
    );
}

#[test]
fn it_209_set_tendency_tags_command_atomically_replaces_normalized_links() {
    let fixture = Fixture::new();
    let note = fixture.observation("Command_Tags", "Tag command");
    let seeded = ObservationService::new(&fixture.repository)
        .set_tags(
            &observation_id(&note.id),
            revision(note.revision),
            vec!["Old tag".to_owned()],
        )
        .expect("seed tags");
    let result = set_tendency_tags_for(
        CallerIdentity::Main,
        &fixture.repository,
        SetTendencyTagsRequest {
            observation_id: note.id,
            expected_revision: seeded.revision,
            tags: vec![
                "Patient".to_owned(),
                "ｐａｔｉｅｎｔ".to_owned(),
                "Fast mana".to_owned(),
            ],
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    );
    let value = serde_json::to_value(result).expect("command");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["tags"].as_array().expect("tags").len(), 2);
    assert!(
        value["data"]["tags"]
            .as_array()
            .expect("tags")
            .iter()
            .all(|tag| tag["displayLabel"] != "Old tag")
    );
}

#[test]
fn it_247_invalid_card_quantity_returns_invalid_card_with_field_path() {
    let fixture = Fixture::new();
    let note = fixture.observation("Invalid_Card_Command", "Keep this note");
    let result = set_card_observations_for(
        CallerIdentity::Main,
        &fixture.repository,
        SetCardObservationsRequest {
            observation_id: note.id.clone(),
            expected_revision: note.revision,
            cards: vec![CardObservationInput {
                oracle_id: None,
                display_name: "Counterspell".to_owned(),
                quantity: 0,
                certainty: CardCertainty::Observed,
                context: None,
            }],
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    );
    let value = serde_json::to_value(result).expect("command");
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "invalid_card");
    assert_eq!(value["error"]["field"], "cards");
    assert_eq!(
        ObservationService::new(&fixture.repository)
            .get(&observation_id(&note.id))
            .expect("preserved")
            .text,
        "Keep this note"
    );
}
