use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};

use crate::domain::RepoError;
use crate::notebook::key::DatabaseKey;

const BUSY_TIMEOUT_MS: u64 = 5_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionSecurity {
    pub cipher_active: bool,
    pub cipher_version: String,
    pub foreign_keys: bool,
    pub wal: bool,
    pub secure_delete: bool,
    pub busy_timeout_ms: u64,
}

pub struct EncryptedConnection {
    pub(crate) connection: Connection,
    path: PathBuf,
    security: ConnectionSecurity,
}

impl EncryptedConnection {
    pub fn open(path: impl AsRef<Path>, key: &DatabaseKey) -> Result<Self, RepoError> {
        let path = path.as_ref();
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| RepoError::NotebookInvalid)?;

        apply_raw_key(&connection, key)?;
        let cipher_version = connection
            .query_row("PRAGMA cipher_version", [], |row| row.get::<_, String>(0))
            .map_err(|_| RepoError::NotebookInvalid)?;
        if cipher_version.trim().is_empty() {
            return Err(RepoError::NotebookInvalid);
        }

        connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|_| RepoError::NotebookInvalid)?;

        connection
            .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))
            .map_err(|_| RepoError::NotebookInvalid)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 PRAGMA secure_delete = ON;",
            )
            .map_err(|_| RepoError::NotebookInvalid)?;

        let foreign_keys = pragma_i64(&connection, "PRAGMA foreign_keys")? == 1;
        let journal_mode = pragma_string(&connection, "PRAGMA journal_mode")?;
        let secure_delete = pragma_i64(&connection, "PRAGMA secure_delete")? == 1;
        if !foreign_keys || !journal_mode.eq_ignore_ascii_case("wal") || !secure_delete {
            return Err(RepoError::NotebookInvalid);
        }

        Ok(Self {
            connection,
            path: path.to_owned(),
            security: ConnectionSecurity {
                cipher_active: true,
                cipher_version,
                foreign_keys,
                wal: true,
                secure_delete,
                busy_timeout_ms: BUSY_TIMEOUT_MS,
            },
        })
    }

    pub fn security(&self) -> &ConnectionSecurity {
        &self.security
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn integrity_check(&self) -> Result<(), RepoError> {
        let result = pragma_string(&self.connection, "PRAGMA integrity_check")?;
        if result == "ok" {
            Ok(())
        } else {
            Err(RepoError::NotebookInvalid)
        }
    }

    pub fn encrypted_backup_to(
        &self,
        destination: impl AsRef<Path>,
        key: &DatabaseKey,
    ) -> Result<(), RepoError> {
        let destination = destination.as_ref();
        if destination.exists() {
            std::fs::remove_file(destination).map_err(|_| RepoError::MigrationFailed)?;
        }
        let mut target = Self::open(destination, key)?;
        {
            let backup = Backup::new(&self.connection, &mut target.connection)
                .map_err(|_| RepoError::MigrationFailed)?;
            backup
                .run_to_completion(64, Duration::from_millis(1), None)
                .map_err(|_| RepoError::MigrationFailed)?;
        }
        target.integrity_check()?;
        Ok(())
    }
}

fn apply_raw_key(connection: &Connection, key: &DatabaseKey) -> Result<(), RepoError> {
    let key_length = i32::try_from(key.expose().len()).map_err(|_| RepoError::NotebookInvalid)?;
    // SAFETY: SQLCipher requires sqlite3_key immediately after sqlite3_open. The
    // connection and key buffer remain valid for the duration of this call.
    let result = unsafe {
        rusqlite::ffi::sqlite3_key(
            connection.handle(),
            key.expose().as_ptr().cast(),
            key_length,
        )
    };
    if result == rusqlite::ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(RepoError::NotebookInvalid)
    }
}

fn pragma_i64(connection: &Connection, pragma: &str) -> Result<i64, RepoError> {
    connection
        .query_row(pragma, [], |row| row.get(0))
        .map_err(|_| RepoError::NotebookInvalid)
}

fn pragma_string(connection: &Connection, pragma: &str) -> Result<String, RepoError> {
    connection
        .query_row(pragma, [], |row| row.get(0))
        .map_err(|_| RepoError::NotebookInvalid)
}
