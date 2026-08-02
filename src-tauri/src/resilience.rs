use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

const LOCAL_WORKFLOWS: &[&str] = &[
    "detection",
    "encounters",
    "notes",
    "history",
    "search",
    "backup",
    "restore",
    "export",
    "classification",
    "diagnostics",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Connectivity {
    Online,
    Offline,
    Degraded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineStatus {
    pub connectivity: Connectivity,
    pub manual_fallback: bool,
    pub retry_paused: bool,
    pub retry_attempts: u8,
    pub local_workflows: Vec<String>,
    pub queued_external_requests: usize,
}

#[derive(Clone, Debug)]
pub struct OfflineCoordinator {
    connectivity: Connectivity,
    retry_attempts: u8,
    retry_paused: bool,
    failed_source_tokens: HashSet<String>,
    manual_identity: Option<String>,
    completed_encounters: BTreeSet<String>,
    provider_available: bool,
    attributed_snapshots: usize,
}

impl Default for OfflineCoordinator {
    fn default() -> Self {
        Self {
            connectivity: Connectivity::Online,
            retry_attempts: 0,
            retry_paused: false,
            failed_source_tokens: HashSet::new(),
            manual_identity: None,
            completed_encounters: BTreeSet::new(),
            provider_available: true,
            attributed_snapshots: 0,
        }
    }
}

impl OfflineCoordinator {
    pub fn set_connectivity(&mut self, connectivity: Connectivity) {
        self.connectivity = connectivity;
    }

    pub fn record_provider_failure(&mut self, source_token: &str) {
        if self.failed_source_tokens.insert(source_token.to_owned()) {
            self.retry_attempts = self.retry_attempts.saturating_add(1).min(3);
            self.retry_paused = self.retry_attempts >= 3;
        }
        self.connectivity = Connectivity::Degraded;
    }

    pub fn explicit_retry(&mut self) {
        self.retry_attempts = 0;
        self.retry_paused = false;
    }

    pub fn confirm_manual_identity(&mut self, identity: impl Into<String>) {
        self.manual_identity = Some(identity.into());
    }

    pub fn accept_automatic_identity(&mut self, identity: &str) -> bool {
        if self.manual_identity.is_some() {
            return false;
        }
        self.manual_identity = Some(identity.to_owned());
        true
    }

    pub fn complete_encounter(&mut self, encounter_id: impl Into<String>) {
        self.completed_encounters.insert(encounter_id.into());
    }

    pub fn may_enrich(&self, encounter_id: &str) -> bool {
        self.connectivity == Connectivity::Online
            && self.provider_available
            && !self.completed_encounters.contains(encounter_id)
    }

    pub fn remove_provider(&mut self) {
        self.provider_available = false;
    }

    pub fn record_attributed_snapshot(&mut self) {
        self.attributed_snapshots = self.attributed_snapshots.saturating_add(1);
    }

    pub fn attributed_snapshots(&self) -> usize {
        self.attributed_snapshots
    }

    pub fn status(&self) -> OfflineStatus {
        OfflineStatus {
            connectivity: self.connectivity,
            manual_fallback: self.connectivity != Connectivity::Online || !self.provider_available,
            retry_paused: self.retry_paused,
            retry_attempts: self.retry_attempts,
            local_workflows: LOCAL_WORKFLOWS
                .iter()
                .map(|workflow| (*workflow).to_owned())
                .collect(),
            queued_external_requests: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_101_false_online_degrades_once_and_keeps_local_commands() {
        let mut state = OfflineCoordinator::default();
        state.record_provider_failure("lookup-1");
        assert_eq!(state.status().connectivity, Connectivity::Degraded);
        assert!(state.status().manual_fallback);
        assert!(state.status().local_workflows.contains(&"notes".into()));
    }

    #[test]
    fn it_102_offline_empty_cache_exposes_absence_without_stale_opponent_data() {
        let mut state = OfflineCoordinator::default();
        state.set_connectivity(Connectivity::Offline);
        let serialized = serde_json::to_string(&state.status()).expect("status");
        assert!(!serialized.contains("opponent"));
        assert_eq!(state.attributed_snapshots(), 0);
    }

    #[test]
    fn it_103_three_failures_pause_retry_and_explicit_retry_resumes() {
        let mut state = OfflineCoordinator::default();
        for token in ["one", "two", "three"] {
            state.record_provider_failure(token);
        }
        assert!(state.status().retry_paused);
        state.explicit_retry();
        assert!(!state.status().retry_paused);
    }

    #[test]
    fn it_104_expired_offline_consent_queues_nothing_for_reconnect() {
        let mut state = OfflineCoordinator::default();
        state.set_connectivity(Connectivity::Offline);
        state.remove_provider();
        state.set_connectivity(Connectivity::Online);
        assert_eq!(state.status().queued_external_requests, 0);
    }

    #[test]
    fn it_105_reconnect_cannot_replace_confirmed_manual_identity() {
        let mut state = OfflineCoordinator::default();
        state.confirm_manual_identity("ManualOpponent");
        assert!(!state.accept_automatic_identity("LateAutomaticOpponent"));
    }

    #[test]
    fn it_106_offline_restart_exposes_every_local_workflow_immediately() {
        let mut state = OfflineCoordinator::default();
        state.set_connectivity(Connectivity::Offline);
        let workflows = state.status().local_workflows;
        for required in LOCAL_WORKFLOWS {
            assert!(workflows.iter().any(|workflow| workflow == required));
        }
    }

    #[test]
    fn it_107_repeated_failed_source_token_is_idempotent() {
        let mut state = OfflineCoordinator::default();
        state.record_provider_failure("same");
        state.record_provider_failure("same");
        assert_eq!(state.status().retry_attempts, 1);
    }

    #[test]
    fn it_108_reconnect_does_not_enrich_completed_history() {
        let mut state = OfflineCoordinator::default();
        state.complete_encounter("finished");
        state.set_connectivity(Connectivity::Online);
        assert!(!state.may_enrich("finished"));
    }

    #[test]
    fn it_109_removed_provider_keeps_attributed_snapshots_readable() {
        let mut state = OfflineCoordinator::default();
        state.record_attributed_snapshot();
        state.remove_provider();
        assert_eq!(state.attributed_snapshots(), 1);
        assert!(state.status().manual_fallback);
    }

    #[test]
    fn it_110_many_offline_encounters_schedule_no_bulk_lookup() {
        let mut state = OfflineCoordinator::default();
        state.set_connectivity(Connectivity::Offline);
        for index in 0..10_000 {
            state.complete_encounter(format!("encounter-{index}"));
        }
        assert_eq!(state.status().queued_external_requests, 0);
        assert!(
            state
                .status()
                .local_workflows
                .contains(&"encounters".into())
        );
    }

    #[test]
    fn e2e_011_offline_first_use_stays_local_and_reconnects_without_retroactive_work() {
        let mut state = OfflineCoordinator::default();
        state.set_connectivity(Connectivity::Offline);
        state.confirm_manual_identity("ManualOpponent");
        state.complete_encounter("offline-finished");

        let offline = state.status();
        assert!(offline.manual_fallback);
        assert_eq!(offline.queued_external_requests, 0);
        for required in LOCAL_WORKFLOWS {
            assert!(
                offline
                    .local_workflows
                    .iter()
                    .any(|workflow| workflow == required)
            );
        }

        state.set_connectivity(Connectivity::Online);
        assert!(!state.may_enrich("offline-finished"));
        assert!(!state.accept_automatic_identity("LateAutomaticOpponent"));
        assert!(state.may_enrich("future-encounter"));
    }
}
