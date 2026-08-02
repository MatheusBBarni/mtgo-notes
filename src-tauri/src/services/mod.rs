pub mod decks;
pub mod deletion;
pub mod history;
pub mod identity;
pub mod observations;
pub mod profiles;

#[cfg(test)]
mod task04_deletion_command_tests;
#[cfg(test)]
mod task04_history_identity_tests;
#[cfg(test)]
mod task04_observation_tests;
#[cfg(test)]
mod tests;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::domain::RepoError;

pub(crate) fn database_error(_error: rusqlite::Error) -> RepoError {
    RepoError::NotebookInvalid
}

pub(crate) fn contract_token<T: Serialize>(domain: &[u8], value: &T) -> Result<String, RepoError> {
    let encoded = serde_json::to_vec(value).map_err(|_| RepoError::NotebookInvalid)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(encoded);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
