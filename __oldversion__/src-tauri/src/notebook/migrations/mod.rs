use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};

use crate::domain::{RepoError, UtcMillis};
use crate::notebook::connection::EncryptedConnection;
use crate::notebook::key::DatabaseKey;
use crate::notebook::schema::{INITIAL_SCHEMA, RETIRED_TAGS_MIGRATION, SCHEMA_VERSION};

#[derive(Clone, Debug)]
pub struct Migration {
    pub version: i64,
    pub sql: String,
}

impl Migration {
    pub fn new(version: i64, sql: impl Into<String>) -> Self {
        Self {
            version,
            sql: sql.into(),
        }
    }

    pub fn checksum(&self) -> String {
        Sha256::digest(self.sql.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackStatus {
    NotRequired,
    Created,
    Restored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReport {
    pub previous_version: i64,
    pub current_version: i64,
    pub rollback_status: RollbackStatus,
}

pub struct MigrationManager {
    migrations: Vec<Migration>,
    supported_version: i64,
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new(vec![
            Migration::new(1, INITIAL_SCHEMA),
            Migration::new(SCHEMA_VERSION, RETIRED_TAGS_MIGRATION),
        ])
    }
}

impl MigrationManager {
    pub fn new(mut migrations: Vec<Migration>) -> Self {
        migrations.sort_by_key(|migration| migration.version);
        let supported_version = migrations.last().map_or(0, |migration| migration.version);
        Self {
            migrations,
            supported_version,
        }
    }

    pub fn migrate(
        &self,
        database_path: impl AsRef<Path>,
        key: &DatabaseKey,
    ) -> Result<MigrationReport, RepoError> {
        let database_path = database_path.as_ref();
        let mut encrypted = EncryptedConnection::open(database_path, key)?;
        let previous_version = current_version(&encrypted)?;
        if previous_version > self.supported_version {
            return Err(RepoError::MigrationFailed);
        }
        verify_applied_checksums(&encrypted, &self.migrations)?;

        let pending = self
            .migrations
            .iter()
            .filter(|migration| migration.version > previous_version)
            .collect::<Vec<_>>();
        if pending.is_empty() {
            return Ok(MigrationReport {
                previous_version,
                current_version: previous_version,
                rollback_status: RollbackStatus::NotRequired,
            });
        }

        let rollback_path = rollback_path(database_path);
        encrypted.encrypted_backup_to(&rollback_path, key)?;

        let migration_result = (|| {
            for migration in pending {
                let transaction = encrypted
                    .connection
                    .transaction()
                    .map_err(|_| RepoError::MigrationFailed)?;
                transaction
                    .execute_batch(&migration.sql)
                    .map_err(|_| RepoError::MigrationFailed)?;
                transaction
                    .execute(
                        "INSERT INTO schema_migrations(version, checksum, applied_at)
                         VALUES (?1, ?2, ?3)",
                        (
                            migration.version,
                            migration.checksum(),
                            UtcMillis::now().get(),
                        ),
                    )
                    .map_err(|_| RepoError::MigrationFailed)?;
                let foreign_key_violation = transaction
                    .query_row("PRAGMA foreign_key_check", [], |row| {
                        row.get::<_, String>(0)
                    })
                    .optional()
                    .map_err(|_| RepoError::MigrationFailed)?;
                if foreign_key_violation.is_some() {
                    return Err(RepoError::MigrationFailed);
                }
                transaction
                    .commit()
                    .map_err(|_| RepoError::MigrationFailed)?;
            }
            encrypted.integrity_check()?;
            Ok(())
        })();

        match migration_result {
            Ok(()) => {
                drop(encrypted);
                remove_database_family(&rollback_path);
                Ok(MigrationReport {
                    previous_version,
                    current_version: self.supported_version,
                    rollback_status: RollbackStatus::Created,
                })
            }
            Err(_) => {
                drop(encrypted);
                restore_rollback(database_path, &rollback_path)?;
                Err(RepoError::MigrationFailed)
            }
        }
    }
}

pub fn current_version(connection: &EncryptedConnection) -> Result<i64, RepoError> {
    let table_exists = connection
        .connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'schema_migrations'
             )",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| RepoError::MigrationFailed)?
        == 1;
    if !table_exists {
        return Ok(0);
    }
    connection
        .connection
        .query_row(
            "SELECT coalesce(max(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|_| RepoError::MigrationFailed)
}

fn verify_applied_checksums(
    connection: &EncryptedConnection,
    migrations: &[Migration],
) -> Result<(), RepoError> {
    if current_version(connection)? == 0 {
        return Ok(());
    }
    let mut statement = connection
        .connection
        .prepare("SELECT version, checksum FROM schema_migrations ORDER BY version")
        .map_err(|_| RepoError::MigrationFailed)?;
    let applied = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| RepoError::MigrationFailed)?;
    for row in applied {
        let (version, checksum) = row.map_err(|_| RepoError::MigrationFailed)?;
        let expected = migrations
            .iter()
            .find(|migration| migration.version == version)
            .ok_or(RepoError::MigrationFailed)?
            .checksum();
        if checksum != expected {
            return Err(RepoError::MigrationFailed);
        }
    }
    Ok(())
}

pub fn rollback_path(database_path: &Path) -> PathBuf {
    database_path.with_extension("rollback")
}

fn restore_rollback(database_path: &Path, rollback_path: &Path) -> Result<(), RepoError> {
    if !rollback_path.exists() {
        return Err(RepoError::MigrationFailed);
    }
    let failed_path = database_path.with_extension("failed");
    remove_database_family(&failed_path);
    fs::rename(database_path, &failed_path).map_err(|_| RepoError::MigrationFailed)?;
    fs::rename(rollback_path, database_path).map_err(|_| RepoError::MigrationFailed)?;
    remove_database_family(&failed_path);
    remove_sidecars(database_path);
    Ok(())
}

fn remove_database_family(path: &Path) {
    let _ = fs::remove_file(path);
    remove_sidecars(path);
}

fn remove_sidecars(path: &Path) {
    let display = path.as_os_str().to_string_lossy();
    let _ = fs::remove_file(format!("{display}-wal"));
    let _ = fs::remove_file(format!("{display}-shm"));
}
