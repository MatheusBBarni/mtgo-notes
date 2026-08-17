use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

static NEXT_EVENT_REVISION: AtomicU64 = AtomicU64::new(1);

pub fn next_event_revision() -> u64 {
    NEXT_EVENT_REVISION.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventVersion {
    pub major: u16,
}

impl EventVersion {
    pub const V1: Self = Self { major: 1 };
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EventName {
    #[serde(rename = "encounter://state-v1")]
    EncounterState,
    #[serde(rename = "overlay://view-v1")]
    OverlayView,
    #[serde(rename = "capture://draft-v1")]
    CaptureDraft,
    #[serde(rename = "notebook://view-v1")]
    NotebookView,
    #[serde(rename = "operation://progress-v1")]
    OperationProgress,
    #[serde(rename = "provider://status-v1")]
    ProviderStatus,
    #[serde(rename = "classifier://progress-v1")]
    ClassifierProgress,
    #[serde(rename = "update://status-v1")]
    UpdateStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementEvent<T> {
    pub name: EventName,
    pub version: EventVersion,
    pub revision: u64,
    pub payload: T,
}

impl<T> ReplacementEvent<T> {
    pub fn v1(name: EventName, revision: u64, payload: T) -> Self {
        Self {
            name,
            version: EventVersion::V1,
            revision,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn ut_117_events_include_name_and_supported_major() {
        let names = [
            EventName::EncounterState,
            EventName::OverlayView,
            EventName::CaptureDraft,
            EventName::NotebookView,
            EventName::OperationProgress,
            EventName::ProviderStatus,
            EventName::ClassifierProgress,
            EventName::UpdateStatus,
        ];

        for name in names {
            let event = ReplacementEvent::v1(name, 1, json!({ "replacement": true }));
            let serialized = serde_json::to_value(event).expect("serialize replacement event");
            assert!(serialized["name"].as_str().is_some());
            assert_eq!(serialized["version"]["major"], 1);
        }
    }

    #[test]
    fn live_event_revisions_are_strictly_increasing() {
        let first = next_event_revision();
        let second = next_event_revision();
        assert!(second > first);
    }
}
