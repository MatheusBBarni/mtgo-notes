pub mod connection;
pub mod fts;
pub mod key;
pub mod migrations;
pub mod repository;
pub mod schema;

use std::path::{Path, PathBuf};

use crate::domain::RepoError;
use crate::notebook::key::{DatabaseKey, KeyCustody, KeyProtector};
use crate::notebook::migrations::{MigrationManager, MigrationReport};
use crate::notebook::repository::{IntegrityOutcome, NotebookRepository};

pub struct NotebookBootstrap<P> {
    database_path: PathBuf,
    key_path: PathBuf,
    protector: P,
    migrations: MigrationManager,
}

pub struct NotebookRuntime {
    pub repository: NotebookRepository,
    pub migration: MigrationReport,
    pub integrity: IntegrityOutcome,
    pub(crate) key: DatabaseKey,
}

#[cfg(test)]
mod tests;

impl<P: KeyProtector> NotebookBootstrap<P> {
    pub fn new(
        database_path: impl Into<PathBuf>,
        key_path: impl Into<PathBuf>,
        protector: P,
    ) -> Self {
        Self {
            database_path: database_path.into(),
            key_path: key_path.into(),
            protector,
            migrations: MigrationManager::default(),
        }
    }

    pub fn with_migrations(mut self, migrations: MigrationManager) -> Self {
        self.migrations = migrations;
        self
    }

    pub fn initialize(self) -> Result<NotebookRuntime, RepoError> {
        let custody = KeyCustody::new(&self.key_path, &self.database_path, self.protector);
        let key = custody.load_or_create()?;
        let migration = self.migrations.migrate(&self.database_path, &key)?;
        let repository = NotebookRepository::open(&self.database_path, &key)?;
        let integrity = repository.begin_runtime()?;
        repository.recover_interrupted_operations()?;
        Ok(NotebookRuntime {
            repository,
            migration,
            integrity,
            key,
        })
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }
}
