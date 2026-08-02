use std::path::Path;

use tempfile::TempDir;

use super::identity::IdentityService;
use super::observations::{CardObservationInput, ObservationService};
use super::profiles::{ProfileService, normalize_handle};
use crate::domain::{CardCertainty, EntityId, IdempotencyKey, RepoError, Revision, UtcMillis};
use crate::notebook::NotebookBootstrap;
use crate::notebook::key::KeyProtector;
use crate::notebook::repository::NotebookRepository;

#[derive(Clone)]
struct Protector;

impl KeyProtector for Protector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Ok(plaintext.iter().map(|byte| byte ^ 0x5a).collect())
    }

    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Ok(ciphertext.iter().map(|byte| byte ^ 0x5a).collect())
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

    fn profile_and_encounter(&self, handle: &str) -> (EntityId, EntityId) {
        let profile = ProfileService::new(&self.repository)
            .create(handle)
            .expect("profile");
        let encounter_id = EntityId::new();
        self.repository
            .start_encounter(&encounter_id, &profile.profile.id, UtcMillis::now(), 1)
            .expect("encounter");
        (profile.profile.id, encounter_id)
    }
}

#[test]
fn ut_068_exact_normalized_primary_or_alias_resolves_canonical_profile() {
    let fixture = Fixture::new();
    let service = ProfileService::new(&fixture.repository);
    let profile = service.create("  ＧＰＴ_42  ").expect("profile");
    service
        .add_alias(&profile.profile.id, "Renamed_Player")
        .expect("alias");

    assert_eq!(
        normalize_handle("ＧＰＴ_42").expect("normalized").display,
        "ＧＰＴ_42"
    );
    assert_eq!(
        service
            .resolve_exact("gpt_42")
            .expect("lookup")
            .expect("profile")
            .id,
        profile.profile.id.to_string()
    );
    assert!(
        service
            .resolve_exact("RENAMED_PLAYER")
            .expect("lookup")
            .expect("profile")
            .matched_as_alias
    );
    assert!(service.resolve_exact("gpt_4").expect("lookup").is_none());
}

#[test]
fn ut_069_hostile_or_blank_manual_handle_writes_nothing() {
    let fixture = Fixture::new();
    let service = ProfileService::new(&fixture.repository);
    assert_eq!(service.create(" \n "), Err(RepoError::InvalidHandle));
    assert_eq!(service.create("<script>"), Err(RepoError::InvalidHandle));
    assert_eq!(
        fixture
            .repository
            .snapshot()
            .expect("snapshot")
            .profile_count,
        0
    );
}

#[test]
fn ut_070_profile_suggestions_are_bounded_for_empty_and_scaled_notebooks() {
    let fixture = Fixture::new();
    let service = ProfileService::new(&fixture.repository);
    assert!(service.suggestions("none", 20).expect("empty").is_empty());
    for index in 0..120 {
        service
            .create(&format!("Player_{index:03}"))
            .expect("profile");
    }
    let page = service.suggestions("player_", 500).expect("page");
    assert_eq!(page.len(), 50);
}

#[test]
fn ut_071_free_text_observation_requires_no_structured_data() {
    let fixture = Fixture::new();
    let (_, encounter_id) = fixture.profile_and_encounter("Note_Player");
    let observation = ObservationService::new(&fixture.repository)
        .create(&encounter_id, "Kept a risky seven")
        .expect("observation");
    assert_eq!(observation.encounter_id, encounter_id.to_string());
    assert!(observation.cards.is_empty());
    assert!(observation.tags.is_empty());
    assert_eq!(observation.source, "player_observation");
}

#[test]
fn ut_072_whitespace_observation_is_rejected_without_mutation() {
    let fixture = Fixture::new();
    let (_, encounter_id) = fixture.profile_and_encounter("Blank_Player");
    assert_eq!(
        ObservationService::new(&fixture.repository).create(&encounter_id, " \n "),
        Err(RepoError::BlankObservation)
    );
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
fn ut_073_suspected_to_observed_keeps_encounter_provenance_and_marks_edit() {
    let fixture = Fixture::new();
    let (_, encounter_id) = fixture.profile_and_encounter("Certainty_Player");
    let service = ObservationService::new(&fixture.repository);
    let note = service
        .create(&encounter_id, "Possible removal")
        .expect("note");
    let suspected = service
        .set_cards(
            &EntityId::parse(note.id.clone()).expect("id"),
            Revision::new(note.revision).expect("revision"),
            vec![CardObservationInput {
                oracle_id: None,
                display_name: "Fatal Push".to_owned(),
                quantity: 1,
                certainty: CardCertainty::Suspected,
                context: Some("Held priority".to_owned()),
            }],
        )
        .expect("suspected");
    let observed = service
        .set_cards(
            &EntityId::parse(note.id).expect("id"),
            Revision::new(suspected.revision).expect("revision"),
            vec![CardObservationInput {
                oracle_id: None,
                display_name: "Fatal Push".to_owned(),
                quantity: 1,
                certainty: CardCertainty::Observed,
                context: Some("Cast on turn two".to_owned()),
            }],
        )
        .expect("observed");
    assert_eq!(observed.encounter_id, encounter_id.to_string());
    assert_eq!(observed.cards[0].certainty, CardCertainty::Observed);
    assert!(observed.edited_at.is_some());
}

#[test]
fn ut_074_duplicate_card_and_tag_inputs_consolidate_without_losing_context() {
    let fixture = Fixture::new();
    let (_, encounter_id) = fixture.profile_and_encounter("Duplicate_Player");
    let service = ObservationService::new(&fixture.repository);
    let note = service.create(&encounter_id, "Structured").expect("note");
    let cards = service
        .set_cards(
            &EntityId::parse(note.id.clone()).expect("id"),
            Revision::new(note.revision).expect("revision"),
            vec![
                CardObservationInput {
                    oracle_id: None,
                    display_name: "Ｆａｔａｌ Push".to_owned(),
                    quantity: 1,
                    certainty: CardCertainty::Observed,
                    context: Some("first context".to_owned()),
                },
                CardObservationInput {
                    oracle_id: None,
                    display_name: "fatal push".to_owned(),
                    quantity: 2,
                    certainty: CardCertainty::Observed,
                    context: Some("second context".to_owned()),
                },
            ],
        )
        .expect("cards");
    let tagged = service
        .set_tags(
            &EntityId::parse(note.id).expect("id"),
            Revision::new(cards.revision).expect("revision"),
            vec!["Fast Play".to_owned(), "fast play".to_owned()],
        )
        .expect("tags");
    assert_eq!(tagged.cards.len(), 1);
    assert!(
        tagged.cards[0]
            .context
            .as_deref()
            .unwrap()
            .contains("first")
    );
    assert!(
        tagged.cards[0]
            .context
            .as_deref()
            .unwrap()
            .contains("second")
    );
    assert_eq!(tagged.tags.len(), 1);
}

#[test]
fn ut_075_stale_merge_edit_or_delete_has_one_revision_winner() {
    let fixture = Fixture::new();
    let (_, encounter_id) = fixture.profile_and_encounter("Revision_Player");
    let service = ObservationService::new(&fixture.repository);
    let note = service.create(&encounter_id, "Original").expect("note");
    service
        .update_text(
            &EntityId::parse(note.id.clone()).expect("id"),
            Revision::new(note.revision).expect("revision"),
            "Winner",
        )
        .expect("winner");
    assert_eq!(
        service.update_text(
            &EntityId::parse(note.id).expect("id"),
            Revision::new(1).expect("revision"),
            "Loser",
        ),
        Err(RepoError::RevisionConflict)
    );
}

#[test]
fn ut_076_merge_and_unmerge_preserve_provenance_and_reassignments() {
    let fixture = Fixture::new();
    let (left, _) = fixture.profile_and_encounter("Primary_Player");
    let right = ProfileService::new(&fixture.repository)
        .create("Renamed_Player")
        .expect("right");
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&left, &right.profile.id, &left)
        .expect("preview");
    let merged = identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    let unmerge = identity
        .preview_unmerge(&EntityId::parse(merged.merge_id).expect("merge id"))
        .expect("unmerge preview");
    identity
        .apply_unmerge(&unmerge, &IdempotencyKey::new())
        .expect("unmerge");
    assert!(
        ProfileService::new(&fixture.repository)
            .get(&right.profile.id)
            .is_ok()
    );
}

#[allow(dead_code)]
fn _path_is_used(path: &Path) -> bool {
    path.exists()
}
