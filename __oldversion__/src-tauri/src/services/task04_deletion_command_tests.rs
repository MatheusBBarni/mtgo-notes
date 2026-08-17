use tempfile::TempDir;

use super::deletion::{DeletionCounts, DeletionEntityType, DeletionService};
use super::history::{HistoryFilters, HistoryQuery, HistoryService};
use super::identity::IdentityService;
use super::observations::ObservationService;
use super::profiles::ProfileService;
use crate::commands::privacy::{
    RequestDeletionRequest, UndoDeletionRequest, request_deletion_coordinated_for,
    request_deletion_for, undo_deletion_for,
};
use crate::disclosure::DisclosurePolicy;
use crate::domain::{EntityId, IdempotencyKey, InternalPhase, RepoError, Revision, UtcMillis};
use crate::ipc::CallerIdentity;
use crate::notebook::NotebookBootstrap;
use crate::notebook::key::KeyProtector;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{OperationCoordinator, OperationKind, OperationState};

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

    fn profile(&self, handle: &str) -> EntityId {
        ProfileService::new(&self.repository)
            .create(handle)
            .expect("profile")
            .profile
            .id
    }

    fn encounter(&self, profile_id: &EntityId, generation: u64) -> EntityId {
        let encounter = EntityId::new();
        self.repository
            .start_encounter(&encounter, profile_id, UtcMillis::now(), generation)
            .expect("encounter");
        encounter
    }

    fn observation(
        &self,
        handle: &str,
        text: &str,
    ) -> (EntityId, EntityId, super::observations::ObservationDetail) {
        let profile = self.profile(handle);
        let encounter = self.encounter(&profile, 1);
        let note = ObservationService::new(&self.repository)
            .create(&encounter, text)
            .expect("note");
        (profile, encounter, note)
    }
}

fn request(preview: super::deletion::DeletionPreview) -> RequestDeletionRequest {
    RequestDeletionRequest {
        confirmation: preview.confirmation.clone(),
        preview,
        idempotency_key: IdempotencyKey::new().as_str().to_owned(),
    }
}

fn search(repository: &NotebookRepository, text: &str) -> usize {
    HistoryService::new(repository, &DisclosurePolicy)
        .search(
            InternalPhase::Finished,
            HistoryQuery {
                text: text.to_owned(),
                filters: HistoryFilters::default(),
                cursor: None,
                page_size: 50,
            },
        )
        .expect("search")
        .items
        .len()
}

#[test]
fn it_161_invalid_deletion_id_returns_not_found_without_writes() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Deletion_Sentinel");
    let before = fixture.repository.snapshot().expect("before");
    assert_eq!(
        DeletionService::new(&fixture.repository)
            .preview(DeletionEntityType::Profile, EntityId::new().as_str(),),
        Err(RepoError::NotFound)
    );
    assert_eq!(fixture.repository.snapshot().expect("after"), before);
    assert!(
        ProfileService::new(&fixture.repository)
            .get(&profile)
            .is_ok()
    );
}

#[test]
fn it_162_empty_notebook_deletion_is_no_op_without_destructive_record() {
    let fixture = Fixture::new();
    let service = DeletionService::new(&fixture.repository);
    let preview = service
        .preview(DeletionEntityType::Notebook, "notebook")
        .expect("preview");
    assert_eq!(preview.counts, DeletionCounts::default());
    let result = service
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("request");
    assert_eq!(result.tombstone_state, "no_op");
    let records = fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row("SELECT count(*) FROM operation_records", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("records");
    assert_eq!(records, 0);
}

#[test]
fn it_163_profile_deletion_preview_reports_exact_large_dependency_counts() {
    let fixture = Fixture::new();
    let profile = fixture.profile("Large_Delete");
    let profiles = ProfileService::new(&fixture.repository);
    for index in 0..25 {
        profiles
            .add_alias(&profile, &format!("Large Delete Alias {index:02}"))
            .expect("alias");
    }
    for generation in 1..=10 {
        let encounter = fixture.encounter(&profile, generation);
        for index in 0..10 {
            ObservationService::new(&fixture.repository)
                .create(&encounter, &format!("Delete fact {generation}-{index}"))
                .expect("note");
        }
        fixture
            .repository
            .finish_encounter(&encounter, UtcMillis::now())
            .expect("finish");
    }
    let preview = DeletionService::new(&fixture.repository)
        .preview(DeletionEntityType::Profile, profile.as_str())
        .expect("preview");
    assert_eq!(preview.counts.profiles, 1);
    assert_eq!(preview.counts.aliases, 25);
    assert_eq!(preview.counts.encounters, 10);
    assert_eq!(preview.counts.observations, 100);
}

#[test]
fn it_164_denied_deletion_write_leaves_visible_and_searchable_data_intact() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Denied_Delete", "searchable deletion sentinel");
    let preview = DeletionService::new(&fixture.repository)
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute_batch(
                    "CREATE TRIGGER deny_observation_delete
                     BEFORE UPDATE ON observations
                     BEGIN
                         SELECT RAISE(ABORT, 'injected deletion denial');
                     END;",
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("failure trigger");
    let result = request_deletion_for(CallerIdentity::Main, &fixture.repository, request(preview));
    assert!(!result.is_success());
    assert_eq!(
        search(&fixture.repository, "searchable deletion sentinel"),
        1
    );
    assert!(
        ObservationService::new(&fixture.repository)
            .get(&EntityId::parse(note.id).expect("note id"))
            .is_ok()
    );
}

#[test]
fn it_165_deletion_conflicts_with_snapshot_coordinator_without_partial_tombstone() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Busy_Delete", "busy sentinel");
    let preview = DeletionService::new(&fixture.repository)
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    let coordinator = OperationCoordinator::default();
    let snapshot = coordinator
        .begin(OperationKind::BackupSnapshot, None)
        .expect("snapshot lease");
    let value = serde_json::to_value(request_deletion_coordinated_for(
        CallerIdentity::Main,
        &fixture.repository,
        &coordinator,
        request(preview),
    ))
    .expect("command");
    assert_eq!(value["error"]["code"], "operation_busy");
    assert!(
        !DeletionService::new(&fixture.repository)
            .is_tombstoned(DeletionEntityType::Observation, &note.id)
            .expect("tombstone")
    );
    drop(snapshot);
}

#[test]
fn it_166_restart_before_deadline_can_undo_but_after_purge_stays_absent() {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = directory.path().join("notebook.db");
    let key = directory.path().join("notebook.key");
    let runtime = NotebookBootstrap::new(&database, &key, Protector)
        .initialize()
        .expect("notebook");
    let profile = ProfileService::new(&runtime.repository)
        .create("Restart_Delete")
        .expect("profile")
        .profile
        .id;
    let deletion = DeletionService::new(&runtime.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, profile.as_str())
        .expect("preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    drop(runtime);

    let runtime = NotebookBootstrap::new(&database, &key, Protector)
        .initialize()
        .expect("reopen pending");
    DeletionService::new(&runtime.repository)
        .undo(
            DeletionEntityType::Profile,
            profile.as_str(),
            &pending.undo_token,
            UtcMillis::now(),
        )
        .expect("undo after restart");
    let deletion = DeletionService::new(&runtime.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, profile.as_str())
        .expect("second preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("second delete");
    deletion
        .purge_due(UtcMillis::new(pending.undo_deadline).expect("deadline"))
        .expect("purge");
    drop(runtime);

    let runtime = NotebookBootstrap::new(&database, &key, Protector)
        .initialize()
        .expect("reopen purged");
    assert_eq!(
        ProfileService::new(&runtime.repository).get(&profile),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_167_repeated_deletion_key_returns_same_tombstone_and_deadline() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Repeated_Deletion", "repeat deletion");
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
fn it_168_profile_deletion_with_pending_merge_requires_resolution() {
    let fixture = Fixture::new();
    let primary = fixture.profile("Merge_Delete_Primary");
    let secondary = fixture.profile("Merge_Delete_Secondary");
    let identity = IdentityService::new(&fixture.repository);
    let merge = identity
        .preview_merge(&primary, &secondary, &primary)
        .expect("merge preview");
    identity
        .apply_merge(&merge, &IdempotencyKey::new())
        .expect("merge");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, primary.as_str())
        .expect("delete preview");
    assert_eq!(preview.dependencies, vec!["active_profile_merge"]);
    assert_eq!(
        deletion.request(&preview, &preview.confirmation, &IdempotencyKey::new(),),
        Err(RepoError::OperationBusy)
    );
}

#[test]
fn it_169_purged_identity_cannot_resurface_and_recreated_handle_gets_new_id() {
    let fixture = Fixture::new();
    let old_profile = fixture.profile("No_Resurrection");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Profile, old_profile.as_str())
        .expect("preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    deletion
        .purge_due(UtcMillis::new(pending.undo_deadline).expect("deadline"))
        .expect("purge");
    let replacement = fixture.profile("No_Resurrection");
    assert_ne!(replacement, old_profile);
    assert_eq!(
        ProfileService::new(&fixture.repository).get(&old_profile),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_170_hundred_record_erase_leaves_zero_active_search_and_records_purge() {
    let fixture = Fixture::new();
    for index in 0..100 {
        fixture.profile(&format!("Erase Scale {index:03}"));
    }
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Notebook, "notebook")
        .expect("preview");
    assert_eq!(preview.counts.profiles, 100);
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    let coordinator = OperationCoordinator::default();
    let operation = deletion
        .purge_due_operation(
            &coordinator,
            UtcMillis::new(pending.undo_deadline).expect("deadline"),
        )
        .expect("purge");
    assert_eq!(operation.state, OperationState::Completed);
    assert_eq!(operation.completed, 100);
    assert_eq!(operation.total, 100);
    assert_eq!(
        fixture
            .repository
            .snapshot()
            .expect("snapshot")
            .profile_count,
        0
    );
    assert_eq!(search(&fixture.repository, "Erase Scale"), 0);
    let purge_records = fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT count(*) FROM operation_records
                     WHERE kind = 'purge' AND state = 'completed'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("purge records");
    assert_eq!(purge_records, 1);
}

#[test]
fn it_225_request_deletion_command_returns_scope_deadline_and_tombstone() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Delete_Command", "delete command");
    let preview = DeletionService::new(&fixture.repository)
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    let value = serde_json::to_value(request_deletion_for(
        CallerIdentity::Main,
        &fixture.repository,
        request(preview),
    ))
    .expect("command");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["entityType"], "observation");
    assert_eq!(value["data"]["entityId"], note.id);
    assert_eq!(value["data"]["tombstoneState"], "pending");
    assert!(
        value["data"]["undoDeadline"].as_i64().expect("deadline")
            > value["data"]["requestedAt"].as_i64().expect("requested")
    );
}

#[test]
fn it_226_undo_deletion_command_restores_active_row_and_fts_entry() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Undo_Command", "restore searchable sentinel");
    assert_eq!(
        search(&fixture.repository, "restore searchable sentinel"),
        1
    );
    let service = DeletionService::new(&fixture.repository);
    let preview = service
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    let pending = service
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    assert_eq!(
        search(&fixture.repository, "restore searchable sentinel"),
        0
    );
    let value = serde_json::to_value(undo_deletion_for(
        CallerIdentity::Main,
        &fixture.repository,
        UndoDeletionRequest {
            entity_type: DeletionEntityType::Observation,
            entity_id: note.id,
            undo_token: pending.undo_token,
        },
        UtcMillis::now(),
    ))
    .expect("undo command");
    assert_eq!(value["data"]["restored"], true);
    assert_eq!(
        search(&fixture.repository, "restore searchable sentinel"),
        1
    );
}

#[test]
fn it_251_read_and_mutation_of_purged_entity_return_not_found() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Purged_Command", "purge me");
    let note_id = EntityId::parse(note.id.clone()).expect("note id");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    let pending = deletion
        .request(&preview, &preview.confirmation, &IdempotencyKey::new())
        .expect("delete");
    deletion
        .purge_due(UtcMillis::new(pending.undo_deadline).expect("deadline"))
        .expect("purge");
    let observations = ObservationService::new(&fixture.repository);
    assert_eq!(observations.get(&note_id), Err(RepoError::NotFound));
    assert_eq!(
        observations.update_text(
            &note_id,
            Revision::new(note.revision).expect("revision"),
            "too late",
        ),
        Err(RepoError::NotFound)
    );
}

#[test]
fn it_259_changed_deletion_graph_rejects_stale_scope_token() {
    let fixture = Fixture::new();
    let (_, _, note) = fixture.observation("Stale_Delete", "before preview");
    let deletion = DeletionService::new(&fixture.repository);
    let preview = deletion
        .preview(DeletionEntityType::Observation, &note.id)
        .expect("preview");
    ObservationService::new(&fixture.repository)
        .update_text(
            &EntityId::parse(note.id.clone()).expect("note id"),
            Revision::new(note.revision).expect("revision"),
            "after preview",
        )
        .expect("edit");
    assert_eq!(
        deletion.request(&preview, &preview.confirmation, &IdempotencyKey::new(),),
        Err(RepoError::ScopeMismatch)
    );
}
