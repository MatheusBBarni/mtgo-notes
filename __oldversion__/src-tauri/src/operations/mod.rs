use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, IdempotencyKey, RepoError, Revision, UtcMillis};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    BackupSnapshot,
    ExportSnapshot,
    Migration,
    RestoreMerge,
    RestoreReplace,
    Purge,
    RollbackApply,
}

impl OperationKind {
    fn is_snapshot(self) -> bool {
        matches!(self, Self::BackupSnapshot | Self::ExportSnapshot)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Requested,
    Running,
    AwaitingConfirmation,
    Committing,
    Completed,
    Failed,
    Cancelled,
    Recoverable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: EntityId,
    pub kind: OperationKind,
    pub idempotency_key: IdempotencyKey,
    pub state: OperationState,
    pub requested_at: UtcMillis,
    pub completed_at: Option<UtcMillis>,
    pub completed: u64,
    pub total: u64,
    pub rollback_location: Option<String>,
    pub revision: Revision,
}

impl OperationRecord {
    pub fn requested(kind: OperationKind, idempotency_key: IdempotencyKey) -> Self {
        Self::requested_with_id(EntityId::new(), kind, idempotency_key)
    }

    pub fn requested_with_id(
        id: EntityId,
        kind: OperationKind,
        idempotency_key: IdempotencyKey,
    ) -> Self {
        Self {
            id,
            kind,
            idempotency_key,
            state: OperationState::Requested,
            requested_at: UtcMillis::now(),
            completed_at: None,
            completed: 0,
            total: 0,
            rollback_location: None,
            revision: Revision::INITIAL,
        }
    }

    pub fn update_progress(&mut self, completed: u64, total: u64) -> Result<(), RepoError> {
        if total < self.total || completed < self.completed || completed > total {
            return Err(RepoError::InvalidRequest);
        }
        self.completed = completed;
        self.total = total;
        self.revision = self.revision.next()?;
        Ok(())
    }

    pub fn transition(&mut self, state: OperationState) -> Result<(), RepoError> {
        let allowed = matches!(
            (self.state, state),
            (OperationState::Requested, OperationState::Running)
                | (
                    OperationState::Running,
                    OperationState::AwaitingConfirmation
                )
                | (OperationState::Running, OperationState::Committing)
                | (
                    OperationState::AwaitingConfirmation,
                    OperationState::Committing
                )
                | (
                    OperationState::AwaitingConfirmation,
                    OperationState::Cancelled
                )
                | (OperationState::AwaitingConfirmation, OperationState::Failed)
                | (OperationState::Committing, OperationState::Completed)
                | (OperationState::Committing, OperationState::Recoverable)
                | (OperationState::Committing, OperationState::Failed)
                | (OperationState::Running, OperationState::Completed)
                | (OperationState::Running, OperationState::Failed)
                | (OperationState::Running, OperationState::Cancelled)
        );
        if !allowed {
            return Err(RepoError::InvalidTransition);
        }
        self.state = state;
        if matches!(
            state,
            OperationState::Completed
                | OperationState::Failed
                | OperationState::Cancelled
                | OperationState::Recoverable
        ) {
            self.completed_at = Some(UtcMillis::now());
        }
        self.revision = self.revision.next()?;
        Ok(())
    }
}

#[derive(Default)]
struct CoordinatorState {
    snapshots: usize,
    exclusive: bool,
    claimed_paths: BTreeSet<String>,
    operations: BTreeMap<String, OperationRecord>,
}

#[derive(Clone, Default)]
pub struct OperationCoordinator {
    state: Arc<Mutex<CoordinatorState>>,
}

impl OperationCoordinator {
    pub fn begin(
        &self,
        kind: OperationKind,
        claimed_path: Option<&str>,
    ) -> Result<OperationLease, RepoError> {
        self.begin_with_cancellation(kind, claimed_path, CancellationToken::new())
    }

    pub fn begin_with_cancellation(
        &self,
        kind: OperationKind,
        claimed_path: Option<&str>,
        cancellation: CancellationToken,
    ) -> Result<OperationLease, RepoError> {
        let mut state = self.state.lock().map_err(|_| RepoError::OperationBusy)?;
        if claimed_path.is_some_and(|path| state.claimed_paths.contains(path)) {
            return Err(RepoError::OperationBusy);
        }
        if kind.is_snapshot() {
            if state.exclusive {
                return Err(RepoError::OperationBusy);
            }
            state.snapshots += 1;
        } else {
            if state.exclusive || state.snapshots > 0 {
                return Err(RepoError::OperationBusy);
            }
            state.exclusive = true;
        }
        if let Some(path) = claimed_path {
            state.claimed_paths.insert(path.to_owned());
        }
        drop(state);
        Ok(OperationLease {
            kind,
            claimed_path: claimed_path.map(str::to_owned),
            state: Arc::clone(&self.state),
            cancellation: Arc::clone(&cancellation.cancelled),
            safe_to_cancel: Arc::clone(&cancellation.safe_to_cancel),
        })
    }

    pub fn register(&self, record: OperationRecord) -> Result<(), RepoError> {
        let mut state = self.state.lock().map_err(|_| RepoError::OperationBusy)?;
        state.operations.insert(record.id.to_string(), record);
        Ok(())
    }

    pub fn update(
        &self,
        operation_id: &EntityId,
        update: impl FnOnce(&mut OperationRecord) -> Result<(), RepoError>,
    ) -> Result<OperationRecord, RepoError> {
        let mut state = self.state.lock().map_err(|_| RepoError::OperationBusy)?;
        let record = state
            .operations
            .get_mut(operation_id.as_str())
            .ok_or(RepoError::NotFound)?;
        update(record)?;
        Ok(record.clone())
    }

    pub fn get(&self, operation_id: &EntityId) -> Result<OperationRecord, RepoError> {
        self.state
            .lock()
            .map_err(|_| RepoError::OperationBusy)?
            .operations
            .get(operation_id.as_str())
            .cloned()
            .ok_or(RepoError::NotFound)
    }
}

pub struct OperationLease {
    kind: OperationKind,
    claimed_path: Option<String>,
    state: Arc<Mutex<CoordinatorState>>,
    cancellation: Arc<AtomicBool>,
    safe_to_cancel: Arc<AtomicBool>,
}

impl OperationLease {
    pub fn cancellation_token(&self) -> CancellationToken {
        CancellationToken {
            cancelled: Arc::clone(&self.cancellation),
            safe_to_cancel: Arc::clone(&self.safe_to_cancel),
        }
    }

    pub fn enter_commit(&self) {
        self.safe_to_cancel.store(false, Ordering::Release);
    }
}

impl Drop for OperationLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            if self.kind.is_snapshot() {
                state.snapshots = state.snapshots.saturating_sub(1);
            } else {
                state.exclusive = false;
            }
            if let Some(path) = &self.claimed_path {
                state.claimed_paths.remove(path);
            }
        }
    }
}

#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    safe_to_cancel: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            safe_to_cancel: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn cancel(&self) -> Result<(), RepoError> {
        if !self.safe_to_cancel.load(Ordering::Acquire) {
            return Err(RepoError::CancelUnsafe);
        }
        self.cancelled.store(true, Ordering::Release);
        Ok(())
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_086_coordinator_allows_snapshots_together_and_excludes_mutation() {
        let coordinator = OperationCoordinator::default();
        let backup = coordinator
            .begin(OperationKind::BackupSnapshot, Some("backup.mtgonotes"))
            .expect("backup");
        let export = coordinator
            .begin(OperationKind::ExportSnapshot, Some("export.txt"))
            .expect("export");
        assert!(matches!(
            coordinator.begin(OperationKind::RestoreMerge, None),
            Err(RepoError::OperationBusy)
        ));
        drop((backup, export));
        assert!(coordinator.begin(OperationKind::RestoreMerge, None).is_ok());
    }

    #[test]
    fn same_destination_is_exclusive_even_for_snapshot_operations() {
        let coordinator = OperationCoordinator::default();
        let _backup = coordinator
            .begin(OperationKind::BackupSnapshot, Some("portable.file"))
            .expect("backup");
        assert!(matches!(
            coordinator.begin(OperationKind::ExportSnapshot, Some("portable.file")),
            Err(RepoError::OperationBusy)
        ));
    }

    #[test]
    fn ut_087_cancellation_is_safe_only_before_commit() {
        let coordinator = OperationCoordinator::default();
        let lease = coordinator
            .begin(OperationKind::BackupSnapshot, None)
            .expect("lease");
        lease
            .cancellation_token()
            .cancel()
            .expect("cancel before commit");
        drop(lease);

        let lease = coordinator
            .begin(OperationKind::ExportSnapshot, None)
            .expect("lease");
        let token = lease.cancellation_token();
        lease.enter_commit();
        assert_eq!(token.cancel(), Err(RepoError::CancelUnsafe));
    }

    #[test]
    fn it_268_progress_is_monotonic_through_terminal_state() {
        let coordinator = OperationCoordinator::default();
        let mut record =
            OperationRecord::requested(OperationKind::BackupSnapshot, IdempotencyKey::new());
        let id = record.id.clone();
        record.transition(OperationState::Running).expect("running");
        coordinator.register(record).expect("register");
        coordinator
            .update(&id, |record| record.update_progress(10, 100))
            .expect("progress");
        coordinator
            .update(&id, |record| record.update_progress(100, 100))
            .expect("progress");
        let terminal = coordinator
            .update(&id, |record| record.transition(OperationState::Completed))
            .expect("completed");
        assert_eq!((terminal.completed, terminal.total), (100, 100));
        assert!(terminal.completed_at.is_some());
    }
}
