use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rusqlite::Connection;
use serde_json::json;

use super::*;
use crate::disclosure::DisclosurePolicy;
use crate::domain::{EntityId, IdempotencyKey, InternalPhase, RepoError, Revision, UtcMillis};
use crate::notebook::connection::EncryptedConnection;
use crate::notebook::key::{DatabaseKey, KeyCustody, KeyProtector};
use crate::notebook::migrations::{Migration, MigrationManager, current_version};
use crate::notebook::schema::{INITIAL_SCHEMA, SCHEMA_VERSION};

#[derive(Clone)]
struct TestProtector {
    scope: u8,
    fail_unprotect: bool,
}

impl KeyProtector for TestProtector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, RepoError> {
        let mut output = vec![self.scope];
        output.extend(plaintext.iter().map(|byte| byte ^ self.scope));
        Ok(output)
    }

    fn unprotect(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RepoError> {
        if self.fail_unprotect || ciphertext.first() != Some(&self.scope) {
            return Err(RepoError::KeyUnavailable);
        }
        Ok(ciphertext[1..]
            .iter()
            .map(|byte| byte ^ self.scope)
            .collect())
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    database_path: std::path::PathBuf,
    key_path: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("tempdir");
        let database_path = directory.path().join("notebook.db");
        let key_path = directory.path().join("notebook.key");
        Self {
            _directory: directory,
            database_path,
            key_path,
        }
    }

    fn protector(&self) -> TestProtector {
        TestProtector {
            scope: 42,
            fail_unprotect: false,
        }
    }

    fn key(&self) -> DatabaseKey {
        KeyCustody::new(&self.key_path, &self.database_path, self.protector())
            .load_or_create()
            .expect("key")
    }

    fn boot(&self) -> NotebookRuntime {
        NotebookBootstrap::new(&self.database_path, &self.key_path, self.protector())
            .initialize()
            .expect("bootstrap")
    }

    fn repository(&self) -> NotebookRepository {
        let key = self.key();
        NotebookRepository::open(&self.database_path, &key).expect("repository")
    }
}

fn seed_profile_and_encounter(repository: &NotebookRepository) -> (EntityId, EntityId, UtcMillis) {
    let now = UtcMillis::new(1_753_689_600_000).expect("timestamp");
    let profile_id = EntityId::new();
    repository
        .create_profile(&profile_id, "Opponent_42", "opponent_42", now)
        .expect("profile");
    let encounter_id = EntityId::new();
    repository
        .start_encounter(&encounter_id, &profile_id, now, 1)
        .expect("encounter");
    (profile_id, encounter_id, now)
}

#[test]
fn ut_031_first_launch_seals_key_and_opens_sqlcipher() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    assert_eq!(fs::read(&fixture.key_path).expect("sealed key").len(), 33);
    runtime
        .repository
        .with_connection(|connection| {
            assert!(connection.security().cipher_active);
            assert!(!connection.security().cipher_version.is_empty());
            assert!(connection.security().foreign_keys);
            assert!(connection.security().wal);
            assert!(connection.security().secure_delete);
            assert_eq!(connection.security().busy_timeout_ms, 5_000);
            Ok(())
        })
        .expect("security");
    let header = fs::read(&fixture.database_path).expect("database");
    assert_ne!(&header[..16], b"SQLite format 3\0");
}

#[test]
fn ut_033_plaintext_and_wrong_key_never_open_as_empty_notebook() {
    let plaintext_fixture = Fixture::new();
    let plaintext = Connection::open(&plaintext_fixture.database_path).expect("plaintext");
    plaintext
        .execute("CREATE TABLE private_notes(value TEXT)", [])
        .expect("table");
    drop(plaintext);
    let key = DatabaseKey::from_bytes(&[7_u8; 32]).expect("key");
    assert!(matches!(
        EncryptedConnection::open(&plaintext_fixture.database_path, &key),
        Err(RepoError::NotebookInvalid)
    ));

    let encrypted_fixture = Fixture::new();
    encrypted_fixture.boot();
    let wrong_key = DatabaseKey::from_bytes(&[9_u8; 32]).expect("key");
    assert!(matches!(
        EncryptedConnection::open(&encrypted_fixture.database_path, &wrong_key),
        Err(RepoError::NotebookInvalid)
    ));
}

#[test]
fn ut_034_forward_migration_commits_version_and_checksum() {
    let fixture = Fixture::new();
    let key = fixture.key();
    let report = MigrationManager::default()
        .migrate(&fixture.database_path, &key)
        .expect("migrate");
    assert_eq!(report.current_version, SCHEMA_VERSION);
    let connection = EncryptedConnection::open(&fixture.database_path, &key).expect("open");
    let (version, checksum): (i64, String) = connection
        .connection
        .query_row(
            "SELECT version, checksum FROM schema_migrations
             ORDER BY version DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("ledger");
    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(checksum.len(), 64);
    let retired_at_exists = connection
        .connection
        .prepare("SELECT retired_at FROM tendency_tags LIMIT 0")
        .is_ok();
    assert!(retired_at_exists);
    connection.integrity_check().expect("integrity");
}

#[test]
fn ut_035_failed_migration_restores_encrypted_rollback() {
    let fixture = Fixture::new();
    let key = fixture.key();
    MigrationManager::new(vec![Migration::new(1, INITIAL_SCHEMA)])
        .migrate(&fixture.database_path, &key)
        .expect("initial migration");
    let connection = EncryptedConnection::open(&fixture.database_path, &key).expect("open");
    connection
        .connection
        .execute(
            "INSERT INTO settings(key, value_json, schema_version, revision)
             VALUES ('theme', '\"dark\"', 1, 1)",
            [],
        )
        .expect("sentinel");
    drop(connection);

    let failing = MigrationManager::new(vec![
        Migration::new(1, INITIAL_SCHEMA),
        Migration::new(
            2,
            "CREATE TABLE partial_write(id INTEGER); SELECT no_such_function();",
        ),
    ]);
    assert!(matches!(
        failing.migrate(&fixture.database_path, &key),
        Err(RepoError::MigrationFailed)
    ));

    let restored = EncryptedConnection::open(&fixture.database_path, &key).expect("restored");
    assert_eq!(current_version(&restored).expect("version"), 1);
    let value: String = restored
        .connection
        .query_row(
            "SELECT value_json FROM settings WHERE key = 'theme'",
            [],
            |row| row.get(0),
        )
        .expect("sentinel");
    assert_eq!(value, "\"dark\"");
    let partial_exists: i64 = restored
        .connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'partial_write'
             )",
            [],
            |row| row.get(0),
        )
        .expect("partial check");
    assert_eq!(partial_exists, 0);
}

#[test]
fn ut_036_partial_unique_index_rejects_second_active_encounter() {
    let fixture = Fixture::new();
    fixture.boot();
    let first_repository = fixture.repository();
    let second_repository = fixture.repository();
    let now = UtcMillis::new(1_753_689_600_000).expect("timestamp");
    let first_profile = EntityId::new();
    let second_profile = EntityId::new();
    first_repository
        .create_profile(&first_profile, "First", "first", now)
        .expect("profile");
    first_repository
        .create_profile(&second_profile, "Second", "second", now)
        .expect("profile");
    first_repository
        .start_encounter(&EntityId::new(), &first_profile, now, 1)
        .expect("first encounter");
    assert!(
        second_repository
            .start_encounter(&EntityId::new(), &second_profile, now, 2)
            .is_err()
    );
    let active_count: i64 = first_repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT count(*) FROM encounters WHERE status = 'active'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("count");
    assert_eq!(active_count, 1);
}

#[test]
fn confirmed_opponent_replacement_is_atomic_and_records_one_undo_group() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    let (_first_profile, first_encounter, now) = seed_profile_and_encounter(&repository);
    let second_profile = EntityId::new();
    repository
        .create_profile(&second_profile, "Second", "second", now)
        .expect("second profile");
    let second_encounter = EntityId::new();
    let undo_group = EntityId::new();

    let replaced = repository
        .replace_active_encounter(&second_encounter, &second_profile, now, 2, &undo_group)
        .expect("atomic encounter replacement");
    assert_eq!(replaced.as_ref(), Some(&first_encounter));

    repository
        .with_connection(|connection| {
            let active: String = connection
                .connection
                .query_row(
                    "SELECT id FROM encounters WHERE status = 'active'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            assert_eq!(active, second_encounter.as_str());
            let finished: String = connection
                .connection
                .query_row(
                    "SELECT status FROM encounters WHERE id = ?1",
                    [first_encounter.as_str()],
                    |row| row.get(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            assert_eq!(finished, "finished");
            let grouped: i64 = connection
                .connection
                .query_row(
                    "SELECT count(*) FROM encounter_transitions
                     WHERE undo_group_id = ?1",
                    [undo_group.as_str()],
                    |row| row.get(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            assert_eq!(grouped, 2);
            Ok(())
        })
        .expect("replacement state");

    let missing_profile = EntityId::new();
    let failed_encounter = EntityId::new();
    assert!(
        repository
            .replace_active_encounter(
                &failed_encounter,
                &missing_profile,
                now,
                3,
                &EntityId::new(),
            )
            .is_err()
    );
    repository
        .with_connection(|connection| {
            let still_active: String = connection
                .connection
                .query_row(
                    "SELECT id FROM encounters WHERE status = 'active'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| RepoError::NotebookInvalid)?;
            assert_eq!(still_active, second_encounter.as_str());
            Ok(())
        })
        .expect("failed replacement rolled back");
}

#[test]
fn ut_037_fts_indexes_every_notebook_search_class() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    let (profile_id, encounter_id, now) = seed_profile_and_encounter(&repository);
    let alias_id = EntityId::new();
    repository
        .add_alias(&alias_id, &profile_id, "Alias_42", "alias_42", now)
        .expect("alias");
    let observation_id = EntityId::new();
    repository
        .add_observation(
            &observation_id,
            &encounter_id,
            "patient mulligan",
            now,
            true,
        )
        .expect("observation");
    let deck_id = EntityId::new();
    repository
        .transact(|transaction| {
            transaction.execute(
                "INSERT INTO card_observations(
                    observation_id, oracle_id, display_name, quantity, certainty
                 ) VALUES (?1, 'oracle-bolt', 'Lightning Bolt', 1, 'observed')",
                [observation_id.as_str()],
            )?;
            let tag_id = EntityId::new();
            transaction.execute(
                "INSERT INTO tendency_tags(id, normalized_label, display_label)
                 VALUES (?1, 'careful', 'Careful Player')",
                [tag_id.as_str()],
            )?;
            transaction.execute(
                "INSERT INTO observation_tags(observation_id, tag_id) VALUES (?1, ?2)",
                [observation_id.as_str(), tag_id.as_str()],
            )?;
            transaction.execute(
                "INSERT INTO deck_records(
                    id, profile_id, source_class, format, completeness,
                    user_label, current_revision, revision, created_at
                 ) VALUES (?1, ?2, 'user', 'Modern', 'complete',
                    'Izzet Murktide', 1, 1, ?3)",
                rusqlite::params![deck_id.as_str(), profile_id.as_str(), now.get()],
            )?;
            Ok(())
        })
        .expect("search fixtures");

    let policy = DisclosurePolicy;
    for (query, entity_type) in [
        ("Opponent_42", "profile"),
        ("Alias_42", "alias"),
        ("mulligan", "observation"),
        ("Murktide", "deck"),
        ("Lightning", "card"),
        ("Careful", "tag"),
    ] {
        let page = repository
            .search_history(&policy, InternalPhase::PreMatch, query, None, 20)
            .expect("search");
        assert!(
            page.items.iter().any(|hit| hit.entity_type == entity_type),
            "missing {entity_type} for {query}: {:?}",
            page.items
        );
    }
}

#[test]
fn ut_038_restricted_and_deleted_rows_leave_fts_transactionally() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    let (_, encounter_id, now) = seed_profile_and_encounter(&repository);
    let visible_id = EntityId::new();
    repository
        .add_observation(&visible_id, &encounter_id, "visible-canary", now, true)
        .expect("visible");
    let restricted_id = EntityId::new();
    repository
        .add_observation(
            &restricted_id,
            &encounter_id,
            "restricted-canary",
            now,
            false,
        )
        .expect("restricted");
    let policy = DisclosurePolicy;
    assert!(
        repository
            .search_history(
                &policy,
                InternalPhase::PreMatch,
                "restricted-canary",
                None,
                10
            )
            .expect("search")
            .items
            .is_empty()
    );
    repository
        .set_observation_searchable(&visible_id, false, Some(now))
        .expect("delete");
    assert!(
        repository
            .search_history(&policy, InternalPhase::PreMatch, "visible-canary", None, 10)
            .expect("search")
            .items
            .is_empty()
    );
}

#[test]
fn ut_039_equal_timestamp_cursor_pages_without_duplicates() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    repository
        .transact(|transaction| {
            for _ in 0..5 {
                transaction.execute(
                    "INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
                     VALUES ('fixture', ?1, 1000, 'same-boundary')",
                    [EntityId::new().to_string()],
                )?;
            }
            Ok(())
        })
        .expect("fixtures");
    let policy = DisclosurePolicy;
    let first = repository
        .search_history(&policy, InternalPhase::PreMatch, "same-boundary", None, 2)
        .expect("first");
    let second = repository
        .search_history(
            &policy,
            InternalPhase::PreMatch,
            "same-boundary",
            first.next_cursor.as_deref(),
            2,
        )
        .expect("second");
    let third = repository
        .search_history(
            &policy,
            InternalPhase::PreMatch,
            "same-boundary",
            second.next_cursor.as_deref(),
            2,
        )
        .expect("third");
    let ids = first
        .items
        .into_iter()
        .chain(second.items)
        .chain(third.items)
        .map(|hit| hit.entity_id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 5);
}

#[test]
fn ut_040_tampered_cursor_is_rejected() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    repository
        .transact(|transaction| {
            for _ in 0..2 {
                transaction.execute(
                    "INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
                     VALUES ('fixture', ?1, 1000, 'cursor-fixture')",
                    [EntityId::new().to_string()],
                )?;
            }
            Ok(())
        })
        .expect("fixtures");
    let policy = DisclosurePolicy;
    let page = repository
        .search_history(&policy, InternalPhase::PreMatch, "cursor-fixture", None, 1)
        .expect("page");
    let mut cursor = page.next_cursor.expect("cursor").into_bytes();
    let last = cursor.last_mut().expect("last");
    *last = if *last == b'A' { b'B' } else { b'A' };
    let cursor = String::from_utf8(cursor).expect("UTF-8");
    assert!(matches!(
        repository.search_history(
            &policy,
            InternalPhase::PreMatch,
            "cursor-fixture",
            Some(&cursor),
            1
        ),
        Err(RepoError::InvalidCursor)
    ));
}

#[test]
fn ut_041_stale_revision_preserves_winning_observation() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    let (_, encounter_id, now) = seed_profile_and_encounter(&repository);
    let observation_id = EntityId::new();
    repository
        .add_observation(&observation_id, &encounter_id, "original", now, true)
        .expect("observation");
    let revision = repository
        .update_observation(&observation_id, Revision::INITIAL, "winner", now)
        .expect("winner");
    assert_eq!(revision.get(), 2);
    assert!(matches!(
        repository.update_observation(&observation_id, Revision::INITIAL, "stale", now),
        Err(RepoError::RevisionConflict)
    ));
    let stored = repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row(
                    "SELECT text, revision FROM observations WHERE id = ?1",
                    [observation_id.as_str()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("stored");
    assert_eq!(stored, ("winner".into(), 2));
}

#[test]
fn ut_042_repeated_source_token_inserts_one_snapshot() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    let (profile_id, encounter_id, now) = seed_profile_and_encounter(&repository);
    let deck_id = EntityId::new();
    let revision_id = EntityId::new();
    repository
        .transact(|transaction| {
            transaction.execute(
                "INSERT INTO deck_records(
                    id, profile_id, source_class, format, completeness,
                    current_revision, revision, created_at
                 ) VALUES (?1, ?2, 'public', 'Modern', 'complete', 1, 1, ?3)",
                rusqlite::params![deck_id.as_str(), profile_id.as_str(), now.get()],
            )?;
            transaction.execute(
                "INSERT INTO deck_revisions(
                    id, deck_id, revision_number, canonical_digest, complete, created_at
                 ) VALUES (?1, ?2, 1, 'digest', 1, ?3)",
                rusqlite::params![revision_id.as_str(), deck_id.as_str(), now.get()],
            )?;
            Ok(())
        })
        .expect("deck");
    let first = repository
        .save_public_snapshot_once(
            &EntityId::new(),
            &encounter_id,
            &revision_id,
            "source-token",
            now,
        )
        .expect("first");
    let second = repository
        .save_public_snapshot_once(
            &EntityId::new(),
            &encounter_id,
            &revision_id,
            "source-token",
            now,
        )
        .expect("second");
    assert_eq!(first, second);
    let count: i64 = repository
        .with_connection(|connection| {
            connection
                .connection
                .query_row("SELECT count(*) FROM public_snapshots", [], |row| {
                    row.get(0)
                })
                .map_err(|_| RepoError::NotebookInvalid)
        })
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn ut_043_repeated_operation_key_returns_recorded_result() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    let key = IdempotencyKey::new();
    let calls = AtomicUsize::new(0);
    let first = repository
        .run_idempotent_operation(&EntityId::new(), "fixture", &key, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(json!({ "value": 42 }))
        })
        .expect("first");
    let second = repository
        .run_idempotent_operation(&EntityId::new(), "fixture", &key, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(json!({ "value": 99 }))
        })
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn ut_044_unclean_shutdown_runs_integrity_check() {
    let fixture = Fixture::new();
    let runtime = fixture.boot();
    assert_eq!(runtime.integrity, IntegrityOutcome::Clean);
    drop(runtime);

    let repository = fixture.repository();
    assert_eq!(
        repository.begin_runtime().expect("recovery"),
        IntegrityOutcome::ValidAfterUncleanShutdown
    );
}

#[test]
fn ut_045_search_budget_with_one_hundred_thousand_rows() {
    let fixture = Fixture::new();
    fixture.boot();
    let repository = fixture.repository();
    repository
        .transact(|transaction| {
            transaction.execute_batch(
                "WITH RECURSIVE
                    a(x) AS (VALUES(0) UNION ALL SELECT x + 1 FROM a WHERE x < 999),
                    b(y) AS (VALUES(0) UNION ALL SELECT y + 1 FROM b WHERE y < 99)
                 INSERT INTO history_fts(entity_type, entity_id, sort_ms, content)
                 SELECT
                    'observation',
                    printf('fixture-%06d', x * 100 + y),
                    x * 100 + y,
                    CASE WHEN x * 100 + y >= 99950 THEN 'needle' ELSE 'bulk' END
                 FROM a CROSS JOIN b;",
            )?;
            Ok(())
        })
        .expect("100k fixture");
    let started = Instant::now();
    let page = repository
        .search_history(
            &DisclosurePolicy,
            InternalPhase::PreMatch,
            "needle",
            None,
            50,
        )
        .expect("search");
    let elapsed = started.elapsed();
    assert_eq!(page.items.len(), 50);
    assert!(
        elapsed <= Duration::from_millis(200),
        "search took {elapsed:?}"
    );
}

#[cfg(windows)]
#[test]
fn it_279_packaged_sqlcipher_opens_migrates_and_backs_up_encrypted() {
    use crate::notebook::key::CurrentUserDpapi;

    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("notebook.db");
    let key_path = directory.path().join("notebook.key");
    let backup_path = directory.path().join("notebook-backup.db");

    let runtime = NotebookBootstrap::new(&database_path, &key_path, CurrentUserDpapi)
        .initialize()
        .expect("current-user DPAPI and packaged SQLCipher bootstrap");
    assert_eq!(runtime.migration.current_version, SCHEMA_VERSION);
    runtime
        .repository
        .with_connection(|connection| {
            assert!(connection.security().cipher_active);
            assert!(!connection.security().cipher_version.trim().is_empty());
            Ok(())
        })
        .expect("live SQLCipher security");

    runtime
        .repository
        .encrypted_backup_to(&backup_path, &runtime.key)
        .expect("encrypted SQLCipher backup");
    let backup = EncryptedConnection::open(&backup_path, &runtime.key)
        .expect("open encrypted SQLCipher backup");
    assert!(backup.security().cipher_active);
    assert_eq!(
        current_version(&backup).expect("backup schema version"),
        SCHEMA_VERSION
    );
    backup.integrity_check().expect("backup integrity");

    let header = fs::read(&backup_path).expect("backup bytes");
    assert_ne!(&header[..16], b"SQLite format 3\0");
}
