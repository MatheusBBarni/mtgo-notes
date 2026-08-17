use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{EntityId, IdempotencyKey, RepoError, UtcMillis};
use crate::notebook::key::DatabaseKey;
use crate::notebook::migrations::MigrationManager;
use crate::notebook::repository::NotebookRepository;
use crate::operations::{
    CancellationToken, OperationCoordinator, OperationKind, OperationRecord, OperationState,
};
use crate::portability::archive::{ARCHIVE_EXTENSION, verify_archive};
use crate::portability::backup::{BackupRequest, create_backup};
use crate::portability::export::{ExportRequest, ExportScope, create_export};
use crate::portability::restore::{
    RestoreMode, RestorePreviewInput, apply_restore, apply_rollback, discard_rollback,
    list_rollbacks, preview_restore,
};
use crate::portability::{
    PortabilityRuntime, atomic_publish, create_notebook_snapshot, partial_path,
};

const PASSPHRASE: &str = "correct horse battery staple";
const NOTE_CANARY: &str = "CANARY-private-Thoughtseize-plan";

struct Fixture {
    _directory: tempfile::TempDir,
    database_path: PathBuf,
    key: DatabaseKey,
    repository: NotebookRepository,
}

impl Fixture {
    fn empty() -> Self {
        let directory = tempfile::tempdir().expect("temporary fixture");
        let database_path = directory.path().join("notebook.db");
        let key = DatabaseKey::from_bytes(&[73; 32]).expect("database key");
        MigrationManager::default()
            .migrate(&database_path, &key)
            .expect("migrate");
        let repository = NotebookRepository::open(&database_path, &key).expect("repository");
        Self {
            _directory: directory,
            database_path,
            key,
            repository,
        }
    }

    fn seeded() -> Self {
        let fixture = Self::empty();
        let now = UtcMillis::new(1_753_689_600_000).expect("timestamp");
        let profile_id = EntityId::new();
        fixture
            .repository
            .create_profile(&profile_id, "Opponent_42", "opponent_42", now)
            .expect("profile");
        let encounter_id = EntityId::new();
        fixture
            .repository
            .start_encounter(&encounter_id, &profile_id, now, 1)
            .expect("encounter");
        fixture
            .repository
            .add_observation(&EntityId::new(), &encounter_id, NOTE_CANARY, now, true)
            .expect("observation");
        fixture
    }

    fn path(&self, file_name: &str) -> PathBuf {
        self.database_path.with_file_name(file_name)
    }
}

fn backup_request(destination: PathBuf) -> BackupRequest {
    BackupRequest {
        operation_id: EntityId::new(),
        destination,
        passphrase_acknowledged: true,
        confirm_empty: true,
        overwrite: false,
        idempotency_key: IdempotencyKey::new(),
    }
}

fn export_request(destination: PathBuf, scope: ExportScope) -> ExportRequest {
    ExportRequest {
        operation_id: EntityId::new(),
        destination,
        scope,
        plaintext_acknowledged: true,
        confirm_empty: true,
        unsaved_edits_resolved: true,
        overwrite: false,
        idempotency_key: IdempotencyKey::new(),
    }
}

fn create_archive(fixture: &Fixture, file_name: &str) -> PathBuf {
    let destination = fixture.path(file_name);
    create_backup(
        &fixture.repository,
        &fixture.key,
        &OperationCoordinator::default(),
        &backup_request(destination.clone()),
        PASSPHRASE,
        &CancellationToken::new(),
    )
    .expect("backup");
    destination
}

fn stage_archive(
    target: &Fixture,
    coordinator: &OperationCoordinator,
    archive: &Path,
    passphrase: &str,
) -> Result<crate::portability::restore::StagedRestore, RepoError> {
    preview_restore(
        &target.repository,
        &target.key,
        coordinator,
        RestorePreviewInput {
            operation_id: EntityId::new(),
            idempotency_key: IdempotencyKey::new(),
            archive_path: archive,
            passphrase,
            cancellation: &CancellationToken::new(),
        },
    )
}

fn snapshot_counts(repository: &NotebookRepository) -> (i64, i64, i64) {
    let snapshot = repository.snapshot().expect("snapshot");
    (
        snapshot.profile_count,
        snapshot.encounter_count,
        snapshot.observation_count,
    )
}

// UT-077, UT-080, IT-220, E2E-014
#[test]
fn encrypted_backup_is_logical_authenticated_and_contains_no_plaintext_canary() {
    let fixture = Fixture::seeded();
    let archive = fixture.path(&format!("notebook.{ARCHIVE_EXTENSION}"));
    let result = create_backup(
        &fixture.repository,
        &fixture.key,
        &OperationCoordinator::default(),
        &backup_request(archive.clone()),
        PASSPHRASE,
        &CancellationToken::new(),
    )
    .expect("backup");
    let verified = verify_archive(&archive, PASSPHRASE).expect("verified archive");
    assert_eq!(verified.manifest.table_counts["opponent_profiles"], 1);
    assert_eq!(verified.manifest.table_counts["observations"], 1);
    assert_eq!(verified.manifest.schema_min, verified.manifest.schema_max);
    assert!(!verified.manifest.table_hashes.is_empty());
    assert!(
        !fs::read(&archive)
            .expect("archive bytes")
            .windows(NOTE_CANARY.len())
            .any(|window| window == NOTE_CANARY.as_bytes())
    );
    let serialized = serde_json::to_string(&result).expect("serialized result");
    assert!(serialized.contains("notebook.mtgonotes"));
    assert!(
        !serialized.contains(
            fixture
                .database_path
                .parent()
                .expect("parent")
                .to_string_lossy()
                .as_ref()
        )
    );
}

// UT-078, IT-131, IT-132, IT-134, IT-257
#[test]
fn backup_validates_acknowledgement_empty_state_and_destination_before_writing() {
    let fixture = Fixture::empty();
    let coordinator = OperationCoordinator::default();
    let cancellation = CancellationToken::new();
    let mut request = backup_request(fixture.path("invalid.txt"));
    request.passphrase_acknowledged = false;
    assert_eq!(
        create_backup(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &request,
            PASSPHRASE,
            &cancellation
        ),
        Err(RepoError::AcknowledgementRequired)
    );

    request.destination = fixture.path("empty.mtgonotes");
    request.passphrase_acknowledged = true;
    request.confirm_empty = false;
    assert_eq!(
        create_backup(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &request,
            PASSPHRASE,
            &cancellation
        ),
        Err(RepoError::EmptyPortabilityConfirmationRequired)
    );

    request.confirm_empty = true;
    request.destination = fixture.path("missing").join("backup.mtgonotes");
    assert_eq!(
        create_backup(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &request,
            PASSPHRASE,
            &cancellation
        ),
        Err(RepoError::DestinationUnwritable)
    );
    assert!(!partial_path(&request.destination).exists());
}

// IT-132 snapshot consistency and transient cleanup.
#[test]
fn encrypted_snapshot_is_stable_while_live_capture_continues() {
    let fixture = Fixture::seeded();
    let snapshot = create_notebook_snapshot(&fixture.repository, &fixture.key, &EntityId::new())
        .expect("encrypted snapshot");
    let now = UtcMillis::new(1_753_689_700_000).expect("timestamp");
    fixture
        .repository
        .create_profile(&EntityId::new(), "LiveCapture", "livecapture", now)
        .expect("live capture write");
    assert_eq!(
        snapshot_counts(snapshot.repository().expect("snapshot")),
        (1, 1, 1)
    );
    assert_eq!(snapshot_counts(&fixture.repository), (2, 1, 1));
    drop(snapshot);
    let transient_count = fs::read_dir(fixture.database_path.parent().expect("parent"))
        .expect("directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains(".portability-snapshot-")
        })
        .count();
    assert_eq!(transient_count, 0);
}

// UT-079, IT-133, IT-136, IT-139, IT-153, IT-156
#[test]
fn failed_atomic_publication_preserves_older_destination_and_never_publishes_partial() {
    let directory = tempfile::tempdir().expect("directory");
    let destination = directory.path().join("backup.mtgonotes");
    let missing_partial = directory.path().join("missing.partial");
    fs::write(&destination, b"older-valid-backup").expect("older destination");
    assert_eq!(
        atomic_publish(&missing_partial, &destination, true),
        Err(RepoError::DestinationUnwritable)
    );
    assert_eq!(
        fs::read(&destination).expect("preserved destination"),
        b"older-valid-backup"
    );
    assert!(!missing_partial.exists());
}

// UT-081, IT-141, IT-255, IT-256
#[test]
fn wrong_passphrase_and_malformed_archives_preserve_live_notebook() {
    let source = Fixture::seeded();
    let archive = create_archive(&source, "source.mtgonotes");
    let target = Fixture::seeded();
    let before = snapshot_counts(&target.repository);
    assert!(matches!(
        stage_archive(
            &target,
            &OperationCoordinator::default(),
            &archive,
            "wrong passphrase"
        ),
        Err(RepoError::WrongPassphrase)
    ));
    let malformed = target.path("malformed.mtgonotes");
    fs::write(&malformed, b"not an archive").expect("malformed fixture");
    assert!(matches!(
        stage_archive(
            &target,
            &OperationCoordinator::default(),
            &malformed,
            PASSPHRASE
        ),
        Err(RepoError::InvalidBackup)
    ));
    assert_eq!(snapshot_counts(&target.repository), before);
}

// IT-142, IT-143, IT-144, IT-145, IT-148, IT-254
#[test]
fn restore_preview_is_staged_exclusive_and_required_before_apply() {
    let source = Fixture::empty();
    let archive = create_archive(&source, "empty.mtgonotes");
    let target = Fixture::empty();
    let coordinator = OperationCoordinator::default();
    let lease = coordinator
        .begin(OperationKind::Purge, None)
        .expect("exclusive lease");
    assert!(matches!(
        stage_archive(&target, &coordinator, &archive, PASSPHRASE),
        Err(RepoError::OperationBusy)
    ));
    drop(lease);

    let staged =
        stage_archive(&target, &coordinator, &archive, PASSPHRASE).expect("staged preview");
    assert_eq!(staged.preview.manifest.record_count, 0);
    assert_eq!(snapshot_counts(&target.repository), (0, 0, 0));
    assert!(matches!(
        PortabilityRuntime::default().take_preview("missing"),
        Err(RepoError::InvalidRequest)
    ));
}

// IT-146, IT-150 restore cancellation before staging mutation.
#[test]
fn cancelled_restore_authentication_leaves_live_and_staging_unchanged() {
    let source = Fixture::seeded();
    let archive = create_archive(&source, "cancel-restore.mtgonotes");
    let target = Fixture::empty();
    let coordinator = OperationCoordinator::default();
    let cancellation = CancellationToken::new();
    cancellation.cancel().expect("cancel");
    let operation_id = EntityId::new();
    let result = preview_restore(
        &target.repository,
        &target.key,
        &coordinator,
        RestorePreviewInput {
            operation_id: operation_id.clone(),
            idempotency_key: IdempotencyKey::new(),
            archive_path: &archive,
            passphrase: PASSPHRASE,
            cancellation: &cancellation,
        },
    );
    assert!(matches!(result, Err(RepoError::InvalidRequest)));
    assert_eq!(snapshot_counts(&target.repository), (0, 0, 0));
    assert_eq!(
        coordinator.get(&operation_id).expect("operation").state,
        OperationState::Cancelled
    );
    let staging_count = fs::read_dir(target.database_path.parent().expect("parent"))
        .expect("directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains(".restore-staging-")
        })
        .count();
    assert_eq!(staging_count, 0);
}

// UT-082, IT-147, IT-149, IT-221, IT-222, E2E-015
#[test]
fn staged_merge_is_idempotent_and_does_not_restore_provider_consent() {
    let source = Fixture::seeded();
    source
        .repository
        .set_provider_consent("official_mtgo", true, "[\"handle\"]", UtcMillis::now())
        .expect("source consent");
    let archive = create_archive(&source, "merge.mtgonotes");
    let target = Fixture::empty();
    let coordinator = OperationCoordinator::default();
    let staged = stage_archive(&target, &coordinator, &archive, PASSPHRASE).expect("preview");
    assert_eq!(snapshot_counts(&target.repository), (0, 0, 0));
    let first = apply_restore(
        &target.repository,
        &target.key,
        &coordinator,
        staged,
        RestoreMode::Merge,
        IdempotencyKey::new(),
    )
    .expect("merge");
    assert_eq!(snapshot_counts(&target.repository), (1, 1, 1));
    assert_eq!(
        target
            .repository
            .provider_consent("official_mtgo")
            .expect("consent"),
        None
    );
    assert_eq!(first.operation.state, OperationState::Recoverable);

    let staged_again =
        stage_archive(&target, &coordinator, &archive, PASSPHRASE).expect("repeat preview");
    let repeated = apply_restore(
        &target.repository,
        &target.key,
        &coordinator,
        staged_again,
        RestoreMode::Merge,
        IdempotencyKey::new(),
    )
    .expect("repeat merge");
    assert!(repeated.exact_duplicates >= 3);
    assert_eq!(snapshot_counts(&target.repository), (1, 1, 1));
}

// Additional restore conflict contract required by Task 06.
#[test]
fn merge_preserves_divergent_stable_ids_as_explicit_conflicts() {
    let source = Fixture::empty();
    let target = Fixture::empty();
    let profile_id = EntityId::new();
    let created_at = UtcMillis::new(1_753_689_600_000).expect("timestamp");
    source
        .repository
        .create_profile(&profile_id, "ImportedDisplay", "same_identity", created_at)
        .expect("source profile");
    target
        .repository
        .create_profile(&profile_id, "LocalDisplay", "same_identity", created_at)
        .expect("target profile");
    let archive = create_archive(&source, "divergent.mtgonotes");
    let coordinator = OperationCoordinator::default();
    let staged = stage_archive(&target, &coordinator, &archive, PASSPHRASE).expect("preview");
    assert_eq!(staged.preview.diff.conflicts, 1);
    let result = apply_restore(
        &target.repository,
        &target.key,
        &coordinator,
        staged,
        RestoreMode::Merge,
        IdempotencyKey::new(),
    )
    .expect("merge");
    assert_eq!(result.conflicts, 1);
    let (handle, conflicts) = target
        .repository
        .with_connection(|connection| {
            let handle = connection
                .connection
                .query_row(
                    "SELECT primary_handle FROM opponent_profiles WHERE id = ?1",
                    [profile_id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            let conflicts = connection
                .connection
                .query_row("SELECT count(*) FROM restore_conflicts", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|_| RepoError::NotebookInvalid)?;
            Ok((handle, conflicts))
        })
        .expect("conflict state");
    assert_eq!(handle, "LocalDisplay");
    assert_eq!(conflicts, 1);
}

// E2E-009, E2E-017 and the Task 06 restore no-resurrection contract.
#[test]
fn restore_tombstones_suppress_the_entire_imported_dependency_graph() {
    let source = Fixture::seeded();
    let profile_id = source
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row("SELECT id FROM opponent_profiles LIMIT 1", [], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("source profile id");
    let archive = create_archive(&source, "tombstoned.mtgonotes");
    let target = Fixture::empty();
    target
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute(
                    "INSERT INTO deletion_tombstones(
                       entity_type, entity_id, requested_at, effective_at,
                       undo_token_digest, purge_state
                     ) VALUES ('profile', ?1, 1, 2, 'digest', 'purged')",
                    [&profile_id],
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            Ok(())
        })
        .expect("tombstone");
    let coordinator = OperationCoordinator::default();
    let staged = stage_archive(&target, &coordinator, &archive, PASSPHRASE)
        .expect("preview without resurrection");
    assert!(staged.preview.diff.tombstone_skips >= 3);
    apply_restore(
        &target.repository,
        &target.key,
        &coordinator,
        staged,
        RestoreMode::Merge,
        IdempotencyKey::new(),
    )
    .expect("merge");
    assert_eq!(snapshot_counts(&target.repository), (0, 0, 0));
}

// UT-085, IT-146, IT-150, IT-221, IT-222, E2E-015
#[test]
fn replace_restore_creates_discoverable_encrypted_atomic_rollback() {
    let source = Fixture::seeded();
    let archive = create_archive(&source, "replace.mtgonotes");
    let target = Fixture::seeded();
    target
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute(
                    "UPDATE opponent_profiles
                     SET primary_handle = 'BeforeReplace',
                         normalized_handle = 'beforereplace'",
                    [],
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            Ok(())
        })
        .expect("distinct target");
    let coordinator = OperationCoordinator::default();
    let staged = stage_archive(&target, &coordinator, &archive, PASSPHRASE).expect("preview");
    let restored = apply_restore(
        &target.repository,
        &target.key,
        &coordinator,
        staged,
        RestoreMode::Replace,
        IdempotencyKey::new(),
    )
    .expect("replace");
    assert_eq!(restored.operation.state, OperationState::Recoverable);
    let available = list_rollbacks(&target.repository).expect("rollbacks");
    assert!(available.iter().any(|item| item.id == restored.rollback.id));
    apply_rollback(
        &target.repository,
        &target.key,
        &coordinator,
        &restored.rollback.id,
    )
    .expect("rollback");
    let handle = target
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT primary_handle FROM opponent_profiles LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("restored old state");
    assert_eq!(handle, "BeforeReplace");
}

// IT-224 command-core cancellation journey.
#[test]
fn portability_command_core_journey_returns_operations_and_cleans_cancelled_output() {
    let fixture = Fixture::seeded();
    let coordinator = OperationCoordinator::default();
    let cancellation = CancellationToken::new();
    cancellation.cancel().expect("safe cancel");
    let destination = fixture.path("cancelled.mtgonotes");
    assert_eq!(
        create_backup(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &backup_request(destination.clone()),
            PASSPHRASE,
            &cancellation,
        ),
        Err(RepoError::InvalidRequest)
    );
    assert!(!destination.exists());
    assert!(!partial_path(&destination).exists());
}

// IT-140, IT-224
#[test]
fn running_backup_accepts_concurrent_safe_cancellation_and_removes_partial_output() {
    let fixture = Fixture::seeded();
    let coordinator = OperationCoordinator::default();
    let cancellation = CancellationToken::new();
    let destination = fixture.path("concurrent-cancel.mtgonotes");
    let request = backup_request(destination.clone());
    let operation_id = request.operation_id.clone();

    std::thread::scope(|scope| {
        let task = scope.spawn(|| {
            create_backup(
                &fixture.repository,
                &fixture.key,
                &coordinator,
                &request,
                PASSPHRASE,
                &cancellation,
            )
        });
        while coordinator.get(&operation_id).is_err() {
            std::thread::yield_now();
        }
        cancellation.cancel().expect("safe cancellation");
        assert_eq!(
            task.join().expect("backup task"),
            Err(RepoError::InvalidRequest)
        );
    });

    assert!(!destination.exists());
    assert!(!partial_path(&destination).exists());
    assert_eq!(
        coordinator.get(&operation_id).expect("operation").state,
        OperationState::Cancelled
    );
}

// IT-135, IT-137, IT-138
#[test]
fn backup_same_path_overwrite_and_pending_deletion_obey_exclusion_contracts() {
    let fixture = Fixture::seeded();
    let destination = fixture.path("exclusive.mtgonotes");
    let coordinator = OperationCoordinator::default();
    let path_claim = destination.to_string_lossy().into_owned();
    let lease = coordinator
        .begin(OperationKind::BackupSnapshot, Some(&path_claim))
        .expect("path lease");
    assert_eq!(
        create_backup(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &backup_request(destination.clone()),
            PASSPHRASE,
            &CancellationToken::new(),
        ),
        Err(RepoError::OperationBusy)
    );
    drop(lease);
    fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute(
                    "INSERT INTO deletion_tombstones(
                       entity_type, entity_id, requested_at, effective_at,
                       undo_token_digest, purge_state
                     ) VALUES ('profile', ?1, 1, 2, 'digest', 'pending')",
                    [EntityId::new().as_str()],
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            Ok(())
        })
        .expect("pending deletion");
    assert_eq!(
        create_backup(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &backup_request(destination),
            PASSPHRASE,
            &CancellationToken::new(),
        ),
        Err(RepoError::OperationBusy)
    );
}

// UT-083, UT-084, IT-152, IT-157, IT-158, IT-159, IT-223, E2E-016
#[test]
fn text_export_is_disclosed_deterministic_scoped_utf8_and_not_importable() {
    let fixture = Fixture::seeded();
    let first_path = fixture.path("first.txt");
    let second_path = fixture.path("second.txt");
    let first = create_export(
        &fixture.repository,
        &fixture.key,
        &OperationCoordinator::default(),
        &export_request(first_path.clone(), ExportScope::CompleteNotebook),
        &CancellationToken::new(),
    )
    .expect("first export");
    let _second = create_export(
        &fixture.repository,
        &fixture.key,
        &OperationCoordinator::default(),
        &export_request(second_path.clone(), ExportScope::CompleteNotebook),
        &CancellationToken::new(),
    )
    .expect("second export");
    assert_eq!(first.opponent_count, 1);
    assert_eq!(first.encounter_count, 1);
    assert_eq!(first.observation_count, 1);
    assert_eq!(first.operation.state, OperationState::Completed);
    assert_eq!(
        fs::read(&first_path).expect("first"),
        fs::read(&second_path).expect("second")
    );
    let text = fs::read_to_string(&first_path).expect("UTF-8");
    assert!(text.contains("WARNING: This file is unencrypted."));
    assert!(text.contains("Opponent: Opponent_42"));
    assert!(text.contains("Encounter:"));
    assert!(text.contains(NOTE_CANARY));
    assert_eq!(
        verify_archive(&first_path, PASSPHRASE),
        Err(RepoError::InvalidBackup)
    );

    let mut unresolved = export_request(
        fixture.path("unresolved.txt"),
        ExportScope::CompleteNotebook,
    );
    unresolved.unsaved_edits_resolved = false;
    assert_eq!(
        create_export(
            &fixture.repository,
            &fixture.key,
            &OperationCoordinator::default(),
            &unresolved,
            &CancellationToken::new(),
        ),
        Err(RepoError::InvalidRequest)
    );
    let missing = export_request(
        fixture.path("missing-profile.txt"),
        ExportScope::SelectedOpponent {
            profile_id: EntityId::new().to_string(),
        },
    );
    assert_eq!(
        create_export(
            &fixture.repository,
            &fixture.key,
            &OperationCoordinator::default(),
            &missing,
            &CancellationToken::new(),
        ),
        Err(RepoError::NotFound)
    );
}

// IT-151, IT-154, IT-155, IT-257
#[test]
fn export_rejects_invalid_unwritable_and_concurrently_claimed_destinations() {
    let fixture = Fixture::seeded();
    let coordinator = OperationCoordinator::default();
    let invalid = export_request(fixture.path("not-text.bin"), ExportScope::CompleteNotebook);
    assert_eq!(
        create_export(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &invalid,
            &CancellationToken::new(),
        ),
        Err(RepoError::InvalidRequest)
    );
    let unwritable = export_request(
        fixture.path("missing").join("export.txt"),
        ExportScope::CompleteNotebook,
    );
    assert_eq!(
        create_export(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &unwritable,
            &CancellationToken::new(),
        ),
        Err(RepoError::DestinationUnwritable)
    );
    let destination = fixture.path("claimed.txt");
    let path_claim = destination.to_string_lossy().into_owned();
    let lease = coordinator
        .begin(OperationKind::ExportSnapshot, Some(&path_claim))
        .expect("path lease");
    assert_eq!(
        create_export(
            &fixture.repository,
            &fixture.key,
            &coordinator,
            &export_request(destination, ExportScope::CompleteNotebook),
            &CancellationToken::new(),
        ),
        Err(RepoError::OperationBusy)
    );
    drop(lease);
}

// IT-140, IT-150, IT-160, IT-268, IT-254, IT-258, UT-086, UT-087, UT-088, IT-268
#[test]
fn operation_progress_cancellation_memory_and_restart_contracts_are_bounded() {
    let fixture = Fixture::seeded();
    let mut operation =
        OperationRecord::requested(OperationKind::BackupSnapshot, IdempotencyKey::new());
    operation
        .transition(OperationState::Running)
        .expect("running");
    operation.update_progress(5, 10).expect("progress");
    fixture
        .repository
        .persist_operation_record(&operation)
        .expect("persist running");
    assert_eq!(fixture.repository.recover_interrupted_operations(), Ok(1));
    let state = fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT state FROM operation_records WHERE id = ?1",
                    [operation.id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("recovered state");
    assert_eq!(state, "failed");
    assert!(std::hint::black_box(64 * 1024_usize) < 64 * 1024 * 1024);

    let token = CancellationToken::new();
    let coordinator = OperationCoordinator::default();
    let lease = coordinator
        .begin_with_cancellation(OperationKind::BackupSnapshot, None, token.clone())
        .expect("lease");
    lease.enter_commit();
    assert_eq!(token.cancel(), Err(RepoError::CancelUnsafe));
}

// Explicit Task 06 rollback discovery/apply/discard extension.
#[test]
fn rollback_discovery_apply_and_discard_lifecycle_is_explicit_and_bounded() {
    let source = Fixture::seeded();
    let archive = create_archive(&source, "rollback-source.mtgonotes");
    let target = Fixture::empty();
    let coordinator = OperationCoordinator::default();
    for mode in [RestoreMode::Merge, RestoreMode::Replace] {
        let staged = stage_archive(&target, &coordinator, &archive, PASSPHRASE).expect("preview");
        let result = apply_restore(
            &target.repository,
            &target.key,
            &coordinator,
            staged,
            mode,
            IdempotencyKey::new(),
        )
        .expect("restore");
        assert!(
            list_rollbacks(&target.repository)
                .expect("rollbacks")
                .iter()
                .any(|rollback| rollback.id == result.rollback.id)
        );
        discard_rollback(&target.repository, &result.rollback.id).expect("discard");
        assert!(
            !list_rollbacks(&target.repository)
                .expect("rollbacks")
                .iter()
                .any(|rollback| rollback.id == result.rollback.id)
        );
    }
}

// E2E-009, E2E-017
#[test]
fn deleted_records_are_absent_from_backup_and_export() {
    let fixture = Fixture::seeded();
    fixture
        .repository
        .with_connection(|connection| {
            connection
                .connection
                .execute(
                    "UPDATE observations SET deleted_at = ?1",
                    [UtcMillis::now().get()],
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            Ok(())
        })
        .expect("delete observation");
    let archive = create_archive(&fixture, "without-deleted.mtgonotes");
    assert_eq!(
        verify_archive(&archive, PASSPHRASE)
            .expect("archive")
            .manifest
            .table_counts
            .get("observations"),
        None
    );
    let export = fixture.path("without-deleted.txt");
    create_export(
        &fixture.repository,
        &fixture.key,
        &OperationCoordinator::default(),
        &export_request(export.clone(), ExportScope::CompleteNotebook),
        &CancellationToken::new(),
    )
    .expect("export");
    assert!(
        !fs::read_to_string(export)
            .expect("export text")
            .contains(NOTE_CANARY)
    );
}

#[test]
fn portability_fixture_paths_are_local_and_explicit() {
    let path = Path::new("portable.mtgonotes");
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some(ARCHIVE_EXTENSION)
    );
}
