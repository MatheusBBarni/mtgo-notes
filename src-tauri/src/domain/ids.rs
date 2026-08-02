use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::{Uuid, Version};

use crate::domain::RepoError;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EntityId(String);

impl EntityId {
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, RepoError> {
        let value = value.into();
        let parsed = Uuid::parse_str(&value).map_err(|_| RepoError::InvalidRequest)?;
        if parsed.get_version() != Some(Version::SortRand) {
            return Err(RepoError::InvalidRequest);
        }
        Ok(Self(parsed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for EntityId {
    type Err = RepoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct IdempotencyKey(EntityId);

impl IdempotencyKey {
    pub fn new() -> Self {
        Self(EntityId::new())
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, RepoError> {
        EntityId::parse(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Default for IdempotencyKey {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct UtcMillis(i64);

impl UtcMillis {
    pub fn new(value: i64) -> Result<Self, RepoError> {
        if value < 0 {
            return Err(RepoError::InvalidRequest);
        }
        Ok(Self(value))
    }

    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_millis();
        Self(i64::try_from(millis).expect("UTC millisecond timestamp must fit i64"))
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    pub const INITIAL: Self = Self(1);

    pub fn new(value: u64) -> Result<Self, RepoError> {
        if value == 0 {
            return Err(RepoError::InvalidRequest);
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Result<Self, RepoError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(RepoError::RevisionConflict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_identifiers_are_uuid_v7_strings() {
        let id = EntityId::new();
        let parsed = Uuid::parse_str(id.as_str()).expect("UUID");
        assert_eq!(parsed.get_version(), Some(Version::SortRand));
        assert_eq!(id.as_str().len(), 36);
    }

    #[test]
    fn revisions_and_timestamps_reject_invalid_values() {
        assert_eq!(Revision::INITIAL.next().expect("next").get(), 2);
        assert!(Revision::new(0).is_err());
        assert!(UtcMillis::new(-1).is_err());
    }
}
