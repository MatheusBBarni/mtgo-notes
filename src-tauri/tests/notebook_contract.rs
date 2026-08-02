use std::fs;
use std::path::Path;

use mtgo_opponent_notes::commands::bootstrap_for;
use mtgo_opponent_notes::domain::RepoError;
use mtgo_opponent_notes::ipc::{CallerIdentity, CommandResult};
use mtgo_opponent_notes::notebook::NotebookBootstrap;
use mtgo_opponent_notes::notebook::connection::EncryptedConnection;
use mtgo_opponent_notes::notebook::key::{KeyCustody, KeyProtector};
use mtgo_opponent_notes::notebook::migrations::{Migration, MigrationManager, current_version};
use mtgo_opponent_notes::notebook::schema::{INITIAL_SCHEMA, SCHEMA_VERSION};
use mtgo_opponent_notes::settings::AppState;

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

fn paths(directory: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        directory.join("notebook.db"),
        directory.join("notebook.key"),
    )
}

#[test]
fn it_233_bootstrap_with_unsealable_key_is_typed_and_fail_closed() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (database_path, key_path) = paths(directory.path());
    fs::write(&database_path, b"existing encrypted notebook").expect("database");
    fs::write(&key_path, b"unsealable key").expect("key");
    let result = NotebookBootstrap::new(
        &database_path,
        &key_path,
        TestProtector {
            scope: 7,
            fail_unprotect: true,
        },
    )
    .initialize();
    assert!(matches!(result, Err(RepoError::KeyUnavailable)));

    let state = AppState::with_notebook_error(RepoError::KeyUnavailable.to_app_error());
    let response = bootstrap_for(CallerIdentity::Main, &state);
    let serialized = serde_json::to_value(response).expect("serialize");
    assert_eq!(serialized["ok"], false);
    assert_eq!(serialized["error"]["code"], "key_unavailable");
    assert_eq!(serialized["error"]["retryable"], false);
    assert!(serialized.get("data").is_none());
    assert_eq!(
        fs::read(&database_path).expect("database"),
        b"existing encrypted notebook"
    );
    assert_eq!(fs::read(&key_path).expect("key"), b"unsealable key");
}

#[test]
fn it_234_unrecoverable_migration_returns_status_without_database_details() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (database_path, key_path) = paths(directory.path());
    let protector = TestProtector {
        scope: 7,
        fail_unprotect: false,
    };
    let runtime = NotebookBootstrap::new(&database_path, &key_path, protector.clone())
        .initialize()
        .expect("initial bootstrap");
    runtime
        .repository
        .mark_clean_shutdown()
        .expect("clean shutdown");
    drop(runtime);

    let failing_migrations = MigrationManager::new(vec![
        Migration::new(1, INITIAL_SCHEMA),
        Migration::new(
            2,
            "CREATE TABLE partial(id INTEGER); SELECT no_such_function();",
        ),
    ]);
    let result = NotebookBootstrap::new(&database_path, &key_path, protector)
        .with_migrations(failing_migrations)
        .initialize();
    assert!(matches!(result, Err(RepoError::MigrationFailed)));

    let state = AppState::with_notebook_error(RepoError::MigrationFailed.to_app_error());
    let response: CommandResult<mtgo_opponent_notes::commands::BootstrapState> =
        bootstrap_for(CallerIdentity::Main, &state);
    let serialized = serde_json::to_value(response).expect("serialize");
    assert_eq!(serialized["error"]["code"], "migration_failed");
    assert_eq!(serialized["error"]["retryable"], false);
    assert!(
        serialized["error"]["message"]
            .as_str()
            .expect("message")
            .contains("Rollback status")
    );
    let rendered = serialized.to_string();
    assert!(!rendered.contains(database_path.to_string_lossy().as_ref()));
    assert!(!rendered.contains("no_such_function"));
}

#[test]
fn it_278_user_scoped_key_fixture_rejects_another_user() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (database_path, key_path) = paths(directory.path());
    let owner = KeyCustody::new(
        &key_path,
        &database_path,
        TestProtector {
            scope: 7,
            fail_unprotect: false,
        },
    );
    let expected = owner.load_or_create().expect("owner key");
    assert_eq!(
        owner.load_or_create().expect("owner reopen").expose(),
        expected.expose()
    );
    let foreign = KeyCustody::new(
        &key_path,
        &database_path,
        TestProtector {
            scope: 9,
            fail_unprotect: false,
        },
    );
    assert!(matches!(
        foreign.load_or_create(),
        Err(RepoError::KeyUnavailable)
    ));
}

#[test]
fn it_279_bundled_sqlcipher_opens_migrates_and_backs_up_encrypted() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (database_path, key_path) = paths(directory.path());
    let protector = TestProtector {
        scope: 7,
        fail_unprotect: false,
    };
    let runtime = NotebookBootstrap::new(&database_path, &key_path, protector.clone())
        .initialize()
        .expect("bootstrap");
    assert_eq!(
        runtime.repository.schema_version().expect("version"),
        SCHEMA_VERSION
    );
    runtime
        .repository
        .mark_clean_shutdown()
        .expect("clean shutdown");
    drop(runtime);

    let key = KeyCustody::new(&key_path, &database_path, protector)
        .load_or_create()
        .expect("key");
    let source = EncryptedConnection::open(&database_path, &key).expect("source");
    assert!(source.security().cipher_active);
    let backup_path = directory.path().join("notebook-backup.db");
    source
        .encrypted_backup_to(&backup_path, &key)
        .expect("backup");
    let backup = EncryptedConnection::open(&backup_path, &key).expect("backup open");
    assert_eq!(
        current_version(&backup).expect("backup version"),
        SCHEMA_VERSION
    );
    backup.integrity_check().expect("backup integrity");

    for path in [&database_path, &backup_path] {
        let header = fs::read(path).expect("database bytes");
        assert_ne!(&header[..16], b"SQLite format 3\0");
    }
}
