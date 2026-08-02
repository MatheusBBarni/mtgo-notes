use tempfile::TempDir;

use super::deletion::{DeletionEntityType, DeletionService};
use super::history::{HistoryFilters, HistoryQuery, HistoryService};
use super::identity::IdentityService;
use super::observations::ObservationService;
use super::profiles::ProfileService;
use crate::commands::history::{
    EntityRequest, get_encounter_for, get_profile_for, search_history_for,
};
use crate::commands::identity::{
    ApplyMergeRequest, ApplyUnmergeRequest, PreviewMergeRequest, PreviewUnmergeRequest,
    apply_merge_for, apply_unmerge_for, preview_merge_for, preview_unmerge_for,
};
use crate::commands::notes::{UpdateObservationRequest, update_observation_for};
use crate::disclosure::DisclosurePolicy;
use crate::domain::{EntityId, IdempotencyKey, InternalPhase, RepoError, Revision, UtcMillis};
use crate::ipc::CallerIdentity;
use crate::notebook::NotebookBootstrap;
use crate::notebook::key::KeyProtector;
use crate::notebook::repository::NotebookRepository;

#[derive(Clone)]
struct Protector(u8);

impl KeyProtector for Protector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Ok(plaintext.iter().map(|byte| byte ^ self.0).collect())
    }

    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        Ok(ciphertext.iter().map(|byte| byte ^ self.0).collect())
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
            Protector(0x3c),
        )
        .initialize()
        .expect("notebook");
        Self {
            _directory: directory,
            repository: runtime.repository,
        }
    }

    fn profile(&self, handle: &str) -> EntityId {
        ProfileService::new(&self.repository)
            .create(handle)
            .expect("profile")
            .profile
            .id
    }

    fn active_encounter(&self, profile_id: &EntityId, generation: u64) -> EntityId {
        let encounter = EntityId::new();
        self.repository
            .start_encounter(&encounter, profile_id, UtcMillis::now(), generation)
            .expect("encounter");
        encounter
    }

    fn finished_encounter(&self, profile_id: &EntityId, generation: u64) -> EntityId {
        let encounter = self.active_encounter(profile_id, generation);
        self.repository
            .finish_encounter(&encounter, UtcMillis::now())
            .expect("finish");
        encounter
    }

    fn profile_with_note(&self, handle: &str, text: &str) -> (EntityId, EntityId, String) {
        let profile = self.profile(handle);
        let encounter = self.active_encounter(&profile, 1);
        let note = ObservationService::new(&self.repository)
            .create(&encounter, text)
            .expect("note");
        (profile, encounter, note.id)
    }
}

fn history_query(text: &str, cursor: Option<String>, page_size: usize) -> HistoryQuery {
    HistoryQuery {
        text: text.to_owned(),
        filters: HistoryFilters::default(),
        cursor,
        page_size,
    }
}

fn success_json<T: serde::Serialize>(result: T) -> serde_json::Value {
    serde_json::to_value(result).expect("serialize command")
}

#[test]
fn it_111_hostile_fts_text_is_bound_as_data() {
    let fixture = Fixture::new();
    fixture.profile_with_note("Safe_Query", "ordinary notebook text");
    let result = HistoryService::new(&fixture.repository, &DisclosurePolicy).search(
        InternalPhase::Finished,
        history_query("\" OR 1=1 -- <script>", None, 50),
    );
    assert!(result.is_ok());
    assert_eq!(
        fixture
            .repository
            .snapshot()
            .expect("snapshot")
            .profile_count,
        1
    );
}

#[test]
fn it_112_zero_result_query_returns_empty_replacement_page() {
    let fixture = Fixture::new();
    fixture.profile("Existing_Profile");
    let page = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .search(
            InternalPhase::Finished,
            history_query("definitely-no-match", None, 50),
        )
        .expect("search");
    assert!(page.items.is_empty());
    assert!(page.replacement);
}

#[test]
fn it_113_overflow_uses_stable_cursor_without_loss_or_duplication() {
    let fixture = Fixture::new();
    for index in 0..25 {
        fixture.profile(&format!("Paged Player {index:03}"));
    }
    let service = HistoryService::new(&fixture.repository, &DisclosurePolicy);
    let first = service
        .search(InternalPhase::Finished, history_query("Paged", None, 10))
        .expect("first");
    let second = service
        .search(
            InternalPhase::Finished,
            history_query("Paged", first.next_cursor.clone(), 10),
        )
        .expect("second");
    let first_ids = first
        .items
        .iter()
        .map(|item| &item.entity_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(first.items.len(), 10);
    assert_eq!(second.items.len(), 10);
    assert!(
        second
            .items
            .iter()
            .all(|item| !first_ids.contains(&item.entity_id))
    );
}

#[test]
fn it_114_restricted_search_is_host_denied_without_payload() {
    let fixture = Fixture::new();
    fixture.profile("Restricted_History");
    let value = success_json(search_history_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::InGameRestricted,
        history_query("Restricted", None, 50),
    ));
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "disclosure_restricted");
    assert!(value.get("data").is_none());
}

#[test]
fn it_115_concurrent_profile_edit_exposes_newer_revision() {
    let fixture = Fixture::new();
    let profile_id = fixture.profile("Revision_Profile");
    let old = ProfileService::new(&fixture.repository)
        .get(&profile_id)
        .expect("old");
    ProfileService::new(&fixture.repository)
        .update_primary_handle(
            &profile_id,
            old.profile.revision,
            "Revision_Profile_Updated",
        )
        .expect("update");
    let current = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_profile(InternalPhase::Finished, &profile_id)
        .expect("current");
    assert!(current.profile.profile.revision > old.profile.revision);
}

#[test]
fn it_116_closing_filtered_review_performs_zero_mutations() {
    let fixture = Fixture::new();
    fixture.profile_with_note("Filtered_Close", "filterable note");
    let before = fixture.repository.snapshot().expect("before");
    HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .search(
            InternalPhase::Finished,
            history_query("filterable", None, 10),
        )
        .expect("search");
    assert_eq!(fixture.repository.snapshot().expect("after"), before);
}

#[test]
fn it_117_repeated_filter_returns_stable_ordered_ids() {
    let fixture = Fixture::new();
    for index in 0..10 {
        fixture.profile(&format!("Stable Filter {index:02}"));
    }
    let service = HistoryService::new(&fixture.repository, &DisclosurePolicy);
    let first = service
        .search(InternalPhase::Finished, history_query("Stable", None, 50))
        .expect("first");
    let second = service
        .search(InternalPhase::Finished, history_query("Stable", None, 50))
        .expect("second");
    assert_eq!(first.items, second.items);
}

#[test]
fn it_118_deep_link_to_purged_history_is_not_found() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Purged_Deep_Link");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, profile.as_str())
        .expect("preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    deletion
        .purge_due(UtcMillis::new(pending.undo_deadline).expect("deadline"))
        .expect("purge");
    assert_eq!(
        HistoryService::new(&fixture.repository, &DisclosurePolicy)
            .get_profile(InternalPhase::Finished, &profile),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_119_open_merged_profile_redirects_to_canonical_identity() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Canonical_Profile");
    let secondary = fixture.profile("Redirected_Profile");
    let service = IdentityService::new(&fixture.repository);
    let preview = service
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    service
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    let detail = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_profile(InternalPhase::Finished, &secondary)
        .expect("redirect");
    assert_eq!(detail.profile.profile.id, primary);
    assert_eq!(detail.canonical_profile_id, Some(primary.to_string()));
}

#[test]
fn it_120_scaled_history_is_bounded_and_foreign_key_protector_cannot_open() {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = directory.path().join("notebook.db");
    let key = directory.path().join("notebook.key");
    let runtime = NotebookBootstrap::new(&database, &key, Protector(0x3c))
        .initialize()
        .expect("notebook");
    for index in 0..300 {
        ProfileService::new(&runtime.repository)
            .create(&format!("Scale History {index:03}"))
            .expect("profile");
    }
    let page = HistoryService::new(&runtime.repository, &DisclosurePolicy)
        .search(InternalPhase::Finished, history_query("Scale", None, 50))
        .expect("search");
    assert_eq!(page.items.len(), 50);
    drop(runtime);
    assert!(
        NotebookBootstrap::new(&database, &key, Protector(0x7f))
            .initialize()
            .is_err()
    );
}

#[test]
fn it_121_invalid_identical_or_tombstoned_merge_ids_change_nothing() {
    let fixture = Fixture::new();
    let left = fixture.profile("Merge_Left");
    let right = fixture.profile("Merge_Right");
    let identity = IdentityService::new(&fixture.repository);
    assert_eq!(
        identity.preview_merge(&left, &left, &left),
        Err(RepoError::MergeConflict)
    );
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, right.as_str())
        .expect("delete preview");
    deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    assert_eq!(
        identity.preview_merge(&left, &right, &left),
        Err(RepoError::MergeConflict)
    );
    assert!(ProfileService::new(&fixture.repository).get(&left).is_ok());
}

#[test]
fn it_122_empty_profile_becomes_alias_without_changing_other_encounters() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Merge_With_History");
    let encounter = fixture.finished_encounter(&primary, 1);
    let empty = fixture.profile("Empty_Profile");
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &empty, &primary)
        .expect("preview");
    identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    let canonical = ProfileService::new(&fixture.repository)
        .get(&primary)
        .expect("canonical");
    assert!(
        canonical
            .aliases
            .iter()
            .any(|alias| alias.display_handle == "Empty_Profile")
    );
    let detail = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_encounter(InternalPhase::Finished, &encounter)
        .expect("encounter");
    assert_eq!(detail.profile_id, primary.to_string());
}

#[test]
fn it_123_extensive_merge_preview_bounds_conflict_details_and_exact_counts() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Conflict_Primary");
    let secondary = fixture.profile("Conflict_Secondary");
    fixture
        .repository
        .transact_domain(|transaction| {
            for index in 0..100 {
                let key = format!("duplicate {index:03}");
                transaction
                    .execute(
                        "INSERT INTO opponent_aliases(
                            id, profile_id, display_handle, normalized_handle,
                            provenance, created_at
                         ) VALUES (?1, ?2, ?3, ?4, 'fixture', ?5)",
                        (
                            EntityId::new().as_str(),
                            primary.as_str(),
                            format!("Duplicate {index:03}"),
                            &key,
                            index,
                        ),
                    )
                    .map_err(|_| RepoError::NotebookInvalid)?;
                transaction
                    .execute(
                        "INSERT INTO opponent_aliases(
                            id, profile_id, display_handle, normalized_handle,
                            provenance, created_at
                         ) VALUES (?1, ?2, ?3, ?4, 'fixture', ?5)",
                        (
                            EntityId::new().as_str(),
                            secondary.as_str(),
                            format!("Duplicate {index:03}"),
                            &key,
                            index,
                        ),
                    )
                    .map_err(|_| RepoError::NotebookInvalid)?;
            }
            Ok(())
        })
        .expect("aliases");
    let preview = IdentityService::new(&fixture.repository)
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    assert_eq!(preview.conflict_count, 100);
    assert_eq!(preview.conflicts.len(), 50);
    assert!(preview.conflict_details_bounded);
    assert_eq!(preview.affected.aliases, 201);
}

#[test]
fn it_124_denied_merge_write_leaves_both_profiles_active() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Denied_Primary");
    let secondary = fixture.profile("Denied_Secondary");
    let preview = IdentityService::new(&fixture.repository)
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute_batch(
                    "CREATE TRIGGER deny_profile_update
                     BEFORE UPDATE ON opponent_profiles
                     BEGIN
                         SELECT RAISE(ABORT, 'injected merge denial');
                     END;",
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("failure trigger");
    let result = apply_merge_for(
        CallerIdentity::Main,
        &fixture.repository,
        ApplyMergeRequest {
            preview,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    );
    let value = success_json(result);
    assert_eq!(value["error"]["code"], "save_failed");
    assert!(
        ProfileService::new(&fixture.repository)
            .get(&primary)
            .is_ok()
    );
    assert!(
        ProfileService::new(&fixture.repository)
            .get(&secondary)
            .is_ok()
    );
}

#[test]
fn it_125_overlapping_merge_loser_gets_revision_conflict() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Overlap_Primary");
    let second = fixture.profile("Overlap_Second");
    let third = fixture.profile("Overlap_Third");
    let identity = IdentityService::new(&fixture.repository);
    let first = identity
        .preview_merge(&primary, &second, &primary)
        .expect("first");
    let overlapping = identity
        .preview_merge(&primary, &third, &primary)
        .expect("overlap");
    identity
        .apply_merge(&first, &IdempotencyKey::new())
        .expect("winner");
    assert_eq!(
        identity.apply_merge(&overlapping, &IdempotencyKey::new()),
        Err(RepoError::RevisionConflict)
    );
}

#[test]
fn it_126_failed_merge_transaction_leaves_two_complete_originals() {
    for (index, trigger) in [
        "CREATE TRIGGER fail_secondary_tombstone
         BEFORE UPDATE ON opponent_profiles
         WHEN NEW.deleted_at IS NOT NULL
         BEGIN
             SELECT RAISE(ABORT, 'fail after reassignment');
         END;",
        "CREATE TRIGGER fail_merge_record
         BEFORE INSERT ON profile_merges
         BEGIN
             SELECT RAISE(ABORT, 'fail before merge record');
         END;",
    ]
    .into_iter()
    .enumerate()
    {
        let fixture = Fixture::new();
        let primary = fixture.profile(&format!("Atomic_Primary_{index}"));
        let secondary = fixture.profile(&format!("Atomic_Secondary_{index}"));
        let encounter = fixture.finished_encounter(&secondary, 1);
        let identity = IdentityService::new(&fixture.repository);
        let preview = identity
            .preview_merge(&primary, &secondary, &primary)
            .expect("preview");
        fixture
            .repository
            .with_connection(|connection| {
                connection
                    .connection
                    .execute_batch(trigger)
                    .map_err(|_| RepoError::NotebookInvalid)
            })
            .expect("failure trigger");
        assert_eq!(
            identity.apply_merge(&preview, &IdempotencyKey::new()),
            Err(RepoError::SaveFailed)
        );
        assert!(
            ProfileService::new(&fixture.repository)
                .get(&primary)
                .is_ok()
        );
        assert!(
            ProfileService::new(&fixture.repository)
                .get(&secondary)
                .is_ok()
        );
        assert_eq!(
            HistoryService::new(&fixture.repository, &DisclosurePolicy)
                .get_encounter(InternalPhase::Finished, &encounter)
                .expect("encounter")
                .profile_id,
            secondary.to_string()
        );
    }
}

#[test]
fn it_127_repeated_merge_key_creates_no_duplicate_aliases_or_encounters() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Idempotent_Primary");
    let secondary = fixture.profile("Idempotent_Secondary");
    fixture.finished_encounter(&secondary, 1);
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let key = IdempotencyKey::new();
    let first = identity.apply_merge(&preview, &key).expect("first");
    let second = identity.apply_merge(&preview, &key).expect("second");
    assert_eq!(first, second);
    let canonical = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_profile(InternalPhase::Finished, &primary)
        .expect("canonical");
    assert_eq!(canonical.encounters.len(), 1);
    assert_eq!(
        canonical
            .profile
            .aliases
            .iter()
            .filter(|alias| alias.display_handle == "Idempotent_Secondary")
            .count(),
        1
    );
}

#[test]
fn it_128_unmerge_previews_post_merge_records_before_assignment() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Unmerge_Primary");
    let secondary = fixture.profile("Unmerge_Secondary");
    let moved = fixture.finished_encounter(&secondary, 1);
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let merged = identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    let post_merge = fixture.finished_encounter(&primary, 2);
    let unmerge = identity
        .preview_unmerge(&EntityId::parse(merged.merge_id).expect("merge id"))
        .expect("unmerge preview");
    assert_eq!(unmerge.restored_encounters, 1);
    assert_eq!(unmerge.post_merge_encounters, 1);
    assert_eq!(
        unmerge.proposed_post_merge_assignment,
        "retain_with_primary"
    );
    identity
        .apply_unmerge(&unmerge, &IdempotencyKey::new())
        .expect("unmerge");
    let moved_detail = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_encounter(InternalPhase::Finished, &moved)
        .expect("moved");
    let post_detail = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_encounter(InternalPhase::Finished, &post_merge)
        .expect("post");
    assert_eq!(moved_detail.profile_id, secondary.to_string());
    assert_eq!(post_detail.profile_id, primary.to_string());
}

#[test]
fn it_129_deleted_target_invalidates_merge_preview() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Stale_Primary");
    let secondary = fixture.profile("Stale_Secondary");
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let deletion = DeletionService::new(&fixture.repository);
    let delete_preview = deletion
        .preview(DeletionEntityType::Profile, secondary.as_str())
        .expect("delete preview");
    deletion
        .request(
            &delete_preview,
            &delete_preview.confirmation,
            &IdempotencyKey::new(),
        )
        .expect("delete");
    assert_eq!(
        identity.apply_merge(&preview, &IdempotencyKey::new()),
        Err(RepoError::RevisionConflict)
    );
}

#[test]
fn it_130_ten_thousand_aliases_return_bounded_canonical_suggestions() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Alias_Canonical");
    fixture
        .repository
        .transact_domain(|transaction| {
            let mut statement = transaction
                .prepare(
                    "INSERT INTO opponent_aliases(
                        id, profile_id, display_handle, normalized_handle,
                        provenance, created_at
                     ) VALUES (?1, ?2, ?3, ?4, 'fixture', ?5)",
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            for index in 0..10_000 {
                statement
                    .execute((
                        EntityId::new().as_str(),
                        profile.as_str(),
                        format!("Alias Scale {index:05}"),
                        format!("alias scale {index:05}"),
                        index,
                    ))
                    .map_err(|_| RepoError::NotebookInvalid)?;
            }
            Ok(())
        })
        .expect("aliases");
    let suggestions = ProfileService::new(&fixture.repository)
        .suggestions("alias scale", 10_000)
        .expect("suggestions");
    assert_eq!(suggestions.len(), 50);
    assert!(suggestions.iter().all(|item| item.id == profile.to_string()
        && item.primary_handle == "Alias_Canonical"
        && item.matched_as_alias));
}

#[test]
fn it_207_update_observation_command_preserves_encounter_time_and_marks_edit() {
    let fixture = Fixture::new();
    let (_, _, note_id) = fixture.profile_with_note("Command_Update", "Before");
    let before = ObservationService::new(&fixture.repository)
        .get(&EntityId::parse(note_id.clone()).expect("note id"))
        .expect("before");
    let result = success_json(update_observation_for(
        CallerIdentity::Main,
        &fixture.repository,
        UpdateObservationRequest {
            observation_id: note_id,
            text: "After".to_owned(),
            expected_revision: before.revision,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ));
    assert_eq!(result["ok"], true);
    assert_eq!(
        result["data"]["encounterStartedAt"],
        before.encounter_started_at
    );
    assert!(result["data"]["editedAt"].is_number());
}

#[test]
fn it_212_search_history_command_returns_stable_cursor_page() {
    let fixture = Fixture::new();
    for index in 0..12 {
        fixture.profile(&format!("Command Search {index:02}"));
    }
    let value = success_json(search_history_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::Finished,
        history_query("Command", None, 5),
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["items"].as_array().expect("items").len(), 5);
    assert!(value["data"]["nextCursor"].is_string());
}

#[test]
fn it_213_get_profile_command_returns_chronological_policy_safe_detail() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Profile_Command");
    fixture.finished_encounter(&profile, 1);
    fixture.finished_encounter(&profile, 2);
    let value = success_json(get_profile_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::Finished,
        EntityRequest {
            id: profile.to_string(),
        },
    ));
    let encounters = value["data"]["encounters"].as_array().expect("encounters");
    assert_eq!(encounters.len(), 2);
    assert!(encounters[0]["startedAt"].as_i64() >= encounters[1]["startedAt"].as_i64());
}

#[test]
fn it_214_get_encounter_command_returns_source_edit_and_incomplete_provenance() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Encounter_Command");
    let encounter = fixture.active_encounter(&profile, 1);
    let note = ObservationService::new(&fixture.repository)
        .create(&encounter, "Original")
        .expect("note");
    ObservationService::new(&fixture.repository)
        .update_text(
            &EntityId::parse(note.id).expect("note id"),
            Revision::new(note.revision).expect("revision"),
            "Edited",
        )
        .expect("edit");
    fixture
        .repository
        .mark_active_encounter_incomplete("window_lost", UtcMillis::now())
        .expect("incomplete");
    let value = success_json(get_encounter_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::Finished,
        EntityRequest {
            id: encounter.to_string(),
        },
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["summary"]["status"], "incomplete");
    assert_eq!(value["data"]["summary"]["incompleteReason"], "window_lost");
    assert_eq!(
        value["data"]["observations"][0]["source"],
        "player_observation"
    );
    assert!(value["data"]["observations"][0]["editedAt"].is_number());
}

#[test]
fn it_216_preview_merge_command_reports_counts_conflicts_and_plan_without_mutation() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Preview_Primary");
    let secondary = fixture.profile("Preview_Secondary");
    let value = success_json(preview_merge_for(
        CallerIdentity::Main,
        &fixture.repository,
        PreviewMergeRequest {
            left_profile_id: primary.to_string(),
            right_profile_id: secondary.to_string(),
            primary_profile_id: primary.to_string(),
        },
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["affected"]["profiles"], 2);
    assert!(
        ProfileService::new(&fixture.repository)
            .get(&secondary)
            .is_ok()
    );
}

#[test]
fn it_217_apply_merge_command_preserves_associations_and_aliases() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Apply_Primary");
    let secondary = fixture.profile("Apply_Secondary");
    fixture.finished_encounter(&secondary, 1);
    let preview = IdentityService::new(&fixture.repository)
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let value = success_json(apply_merge_for(
        CallerIdentity::Main,
        &fixture.repository,
        ApplyMergeRequest {
            preview,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["canonicalProfileId"], primary.as_str());
    let detail = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .get_profile(InternalPhase::Finished, &primary)
        .expect("detail");
    assert_eq!(detail.encounters.len(), 1);
}

#[test]
fn it_218_preview_unmerge_command_includes_post_merge_assignments() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Preview_Unmerge_Primary");
    let secondary = fixture.profile("Preview_Unmerge_Secondary");
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let merged = identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    fixture.finished_encounter(&primary, 1);
    let value = success_json(preview_unmerge_for(
        CallerIdentity::Main,
        &fixture.repository,
        PreviewUnmergeRequest {
            merge_id: merged.merge_id,
        },
    ));
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["postMergeEncounters"], 1);
    assert_eq!(
        value["data"]["proposedPostMergeAssignment"],
        "retain_with_primary"
    );
}

#[test]
fn it_219_apply_unmerge_command_restores_confirmed_preview_assignments() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Apply_Unmerge_Primary");
    let secondary = fixture.profile("Apply_Unmerge_Secondary");
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let merged = identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    let unmerge = identity
        .preview_unmerge(&EntityId::parse(merged.merge_id).expect("merge id"))
        .expect("unmerge");
    let value = success_json(apply_unmerge_for(
        CallerIdentity::Main,
        &fixture.repository,
        ApplyUnmergeRequest {
            preview: unmerge,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ));
    assert_eq!(value["ok"], true);
    assert!(
        ProfileService::new(&fixture.repository)
            .get(&secondary)
            .is_ok()
    );
}

#[test]
fn it_235_stale_mutable_command_returns_revision_conflict_without_partial_write() {
    let fixture = Fixture::new();
    let (_, _, note_id) = fixture.profile_with_note("Stale_Command", "Original");
    let note = ObservationService::new(&fixture.repository)
        .get(&EntityId::parse(note_id.clone()).expect("note id"))
        .expect("note");
    ObservationService::new(&fixture.repository)
        .update_text(
            &EntityId::parse(note_id.clone()).expect("note id"),
            Revision::new(note.revision).expect("revision"),
            "Winner",
        )
        .expect("winner");
    let value = success_json(update_observation_for(
        CallerIdentity::Main,
        &fixture.repository,
        UpdateObservationRequest {
            observation_id: note_id.clone(),
            text: "Loser".to_owned(),
            expected_revision: note.revision,
            idempotency_key: IdempotencyKey::new().as_str().to_owned(),
        },
    ));
    assert_eq!(value["error"]["code"], "revision_conflict");
    assert_eq!(
        ObservationService::new(&fixture.repository)
            .get(&EntityId::parse(note_id).expect("note id"))
            .expect("stored")
            .text,
        "Winner"
    );
}

#[test]
fn it_250_all_history_commands_are_restricted_during_gameplay() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Restricted_All");
    let encounter = fixture.active_encounter(&profile, 1);
    let search = success_json(search_history_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::InGameRestricted,
        history_query("Restricted", None, 50),
    ));
    let profile_result = success_json(get_profile_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::InGameRestricted,
        EntityRequest {
            id: profile.to_string(),
        },
    ));
    let encounter_result = success_json(get_encounter_for(
        CallerIdentity::Main,
        &fixture.repository,
        InternalPhase::InGameRestricted,
        EntityRequest {
            id: encounter.to_string(),
        },
    ));
    for value in [search, profile_result, encounter_result] {
        assert_eq!(value["error"]["code"], "disclosure_restricted");
        assert!(value.get("data").is_none());
    }
}

#[test]
fn it_252_tampered_history_cursor_returns_invalid_cursor() {
    let fixture = Fixture::new();
    for index in 0..3 {
        fixture.profile(&format!("Cursor Test {index}"));
    }
    let first = HistoryService::new(&fixture.repository, &DisclosurePolicy)
        .search(InternalPhase::Finished, history_query("Cursor", None, 1))
        .expect("first");
    let mut cursor = first.next_cursor.expect("cursor");
    cursor.push('x');
    assert_eq!(
        HistoryService::new(&fixture.repository, &DisclosurePolicy).search(
            InternalPhase::Finished,
            history_query("Cursor", Some(cursor), 1),
        ),
        Err(RepoError::InvalidCursor)
    );
}

#[test]
fn it_253_invalid_merge_graph_or_preview_returns_merge_conflict() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Invalid_Graph_A");
    let secondary = fixture.profile("Invalid_Graph_B");
    let third = fixture.profile("Invalid_Graph_C");
    let identity = IdentityService::new(&fixture.repository);
    assert_eq!(
        identity.preview_merge(&primary, &secondary, &third),
        Err(RepoError::MergeConflict)
    );
    let mut preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    preview.plan_token.clear();
    assert_eq!(
        identity.apply_merge(&preview, &IdempotencyKey::new()),
        Err(RepoError::MergeConflict)
    );
}

#[test]
fn e2e_012_search_pages_and_chronology_are_host_denied_after_gameplay_begins() {
    let fixture = Fixture::new();
    let profile = fixture.profile("History_Journey");
    let encounter = fixture.active_encounter(&profile, 1);
    let note = ObservationService::new(&fixture.repository)
        .create(&encounter, "Observed Fatal Push with patient pacing")
        .expect("note");
    ObservationService::new(&fixture.repository)
        .set_tags(
            &EntityId::parse(note.id).expect("note id"),
            Revision::new(note.revision).expect("revision"),
            vec!["Patient".to_owned()],
        )
        .expect("tag");
    fixture
        .repository
        .finish_encounter(&encounter, UtcMillis::now())
        .expect("finish");
    let service = HistoryService::new(&fixture.repository, &DisclosurePolicy);
    assert!(
        !service
            .search(InternalPhase::Finished, history_query("Patient", None, 1),)
            .expect("outside gameplay")
            .items
            .is_empty()
    );
    assert!(
        service
            .get_profile(InternalPhase::Finished, &profile)
            .is_ok()
    );
    assert_eq!(
        service.search(
            InternalPhase::InGameRestricted,
            history_query("Patient", None, 1),
        ),
        Err(RepoError::DisclosureRestricted)
    );
    assert_eq!(
        service.get_profile(InternalPhase::InGameRestricted, &profile),
        Err(RepoError::DisclosureRestricted)
    );
}

#[test]
fn e2e_013_merge_alias_post_merge_data_and_unmerge_preserve_provenance() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Journey_Primary");
    let secondary = fixture.profile("Journey_Secondary");
    let original = fixture.finished_encounter(&secondary, 1);
    let identity = IdentityService::new(&fixture.repository);
    let preview = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("preview");
    let merged = identity
        .apply_merge(&preview, &IdempotencyKey::new())
        .expect("merge");
    assert_eq!(
        ProfileService::new(&fixture.repository)
            .resolve_exact("Journey_Secondary")
            .expect("lookup")
            .expect("canonical")
            .id,
        primary.to_string()
    );
    let post_merge = fixture.finished_encounter(&primary, 2);
    let unmerge = identity
        .preview_unmerge(&EntityId::parse(merged.merge_id).expect("merge id"))
        .expect("preview unmerge");
    identity
        .apply_unmerge(&unmerge, &IdempotencyKey::new())
        .expect("unmerge");
    let history = HistoryService::new(&fixture.repository, &DisclosurePolicy);
    assert_eq!(
        history
            .get_encounter(InternalPhase::Finished, &original)
            .expect("original")
            .profile_id,
        secondary.to_string()
    );
    assert_eq!(
        history
            .get_encounter(InternalPhase::Finished, &post_merge)
            .expect("post merge")
            .profile_id,
        primary.to_string()
    );
}
