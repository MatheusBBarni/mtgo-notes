use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::ipc::{AppError, ErrorCode};

pub const MAX_LOG_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_LOG_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const MAX_LOG_FILE_BYTES: u64 = 1024 * 1024;

const ALLOWED_FIELDS: &[&str] = &[
    "timestamp",
    "level",
    "component",
    "eventCode",
    "appVersion",
    "schemaVersion",
    "classifierVersion",
    "durationBucket",
    "errorCode",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeLogEvent {
    pub timestamp: i64,
    pub level: LogLevel,
    pub component: String,
    pub event_code: String,
    pub app_version: String,
    pub schema_version: u32,
    pub classifier_version: Option<String>,
    pub duration_bucket: Option<String>,
    pub error_code: Option<String>,
}

impl SafeLogEvent {
    pub fn validate(&self) -> Result<(), AppError> {
        for value in [
            self.component.as_str(),
            self.event_code.as_str(),
            self.app_version.as_str(),
        ]
        .into_iter()
        .chain(self.classifier_version.as_deref())
        .chain(self.duration_bucket.as_deref())
        .chain(self.error_code.as_deref())
        {
            if !is_safe_token(value) {
                return Err(redaction_failed());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticArtifactPreview {
    pub file_name: String,
    pub field_classes: Vec<String>,
    pub event_count: usize,
    pub redaction_count: usize,
    pub omitted: bool,
    pub omission_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsPreview {
    pub preview_token: String,
    pub artifacts: Vec<DiagnosticArtifactPreview>,
    pub total_events: usize,
    pub total_redactions: usize,
    pub summarized: bool,
    pub expires_at: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsBundleResult {
    pub file_name: String,
    pub artifact_count: usize,
    pub event_count: usize,
    pub network_requests: u64,
}

#[derive(Clone, Debug)]
struct DiagnosticSnapshot {
    preview: DiagnosticsPreview,
    artifacts: BTreeMap<String, Vec<Value>>,
    cancelled: bool,
}

#[derive(Clone, Default)]
pub struct DiagnosticsService {
    previews: Arc<Mutex<HashMap<String, DiagnosticSnapshot>>>,
    destinations: Arc<Mutex<BTreeSet<PathBuf>>>,
}

impl DiagnosticsService {
    pub fn append(&self, directory: &Path, event: &SafeLogEvent) -> Result<(), AppError> {
        event.validate()?;
        fs::create_dir_all(directory).map_err(|_| diagnostics_io_error())?;
        let path = directory.join("events-current.jsonl");
        if fs::metadata(&path).is_ok_and(|metadata| metadata.len() >= MAX_LOG_FILE_BYTES) {
            let sequence = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            fs::rename(&path, directory.join(format!("events-{sequence}.jsonl")))
                .map_err(|_| diagnostics_io_error())?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|_| diagnostics_io_error())?;
        let encoded = serde_json::to_vec(event).map_err(|_| redaction_failed())?;
        file.write_all(&encoded)
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|_| diagnostics_io_error())?;
        self.cleanup(directory, SystemTime::now())?;
        Ok(())
    }

    pub fn cleanup(&self, directory: &Path, now: SystemTime) -> Result<Vec<PathBuf>, AppError> {
        let mut files = Vec::new();
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(diagnostics_io_error()),
        };
        for entry in entries {
            let entry = entry.map_err(|_| diagnostics_io_error())?;
            let metadata = entry.metadata().map_err(|_| diagnostics_io_error())?;
            if metadata.is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|value| value == "jsonl")
            {
                files.push(RetentionFile {
                    path: entry.path(),
                    modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    bytes: metadata.len(),
                });
            }
        }
        let removed = retention_plan(&files, now);
        for path in &removed {
            fs::remove_file(path).map_err(|_| diagnostics_io_error())?;
        }
        Ok(removed)
    }

    pub fn preview(
        &self,
        directory: &Path,
        environment: SafeLogEvent,
        now_ms: i64,
    ) -> Result<DiagnosticsPreview, AppError> {
        environment.validate()?;
        let mut artifacts = BTreeMap::<String, Vec<Value>>::new();
        let mut previews = Vec::new();
        let mut source_bytes = 0_u64;

        let environment_value =
            serde_json::to_value(environment).map_err(|_| redaction_failed())?;
        let (environment_value, environment_redactions) = redact_value(&environment_value)?;
        artifacts.insert("environment.json".into(), vec![environment_value]);
        previews.push(DiagnosticArtifactPreview {
            file_name: "environment.json".into(),
            field_classes: ALLOWED_FIELDS
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            event_count: 1,
            redaction_count: environment_redactions,
            omitted: false,
            omission_code: None,
        });

        match fs::read_dir(directory) {
            Ok(entries) => {
                let mut paths = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().is_some_and(|value| value == "jsonl"))
                    .collect::<Vec<_>>();
                paths.sort();
                for path in paths {
                    source_bytes = source_bytes.saturating_add(
                        fs::metadata(&path)
                            .map(|metadata| metadata.len())
                            .unwrap_or(0),
                    );
                    let file_name = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("omitted.jsonl")
                        .to_owned();
                    match sanitize_log_file(&path) {
                        Ok((values, redactions)) => {
                            previews.push(DiagnosticArtifactPreview {
                                file_name: file_name.clone(),
                                field_classes: ALLOWED_FIELDS
                                    .iter()
                                    .map(|value| (*value).to_owned())
                                    .collect(),
                                event_count: values.len(),
                                redaction_count: redactions,
                                omitted: false,
                                omission_code: None,
                            });
                            artifacts.insert(file_name, values);
                        }
                        Err(error) if error.code == ErrorCode::DestinationUnwritable => {
                            previews.push(DiagnosticArtifactPreview {
                                file_name,
                                field_classes: Vec::new(),
                                event_count: 0,
                                redaction_count: 0,
                                omitted: true,
                                omission_code: Some("source_unavailable".into()),
                            });
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                previews.push(DiagnosticArtifactPreview {
                    file_name: "local-logs".into(),
                    field_classes: Vec::new(),
                    event_count: 0,
                    redaction_count: 0,
                    omitted: true,
                    omission_code: Some("no_diagnostic_events".into()),
                });
            }
            Err(_) => return Err(diagnostics_io_error()),
        }

        let total_events = previews.iter().map(|item| item.event_count).sum();
        let total_redactions = previews.iter().map(|item| item.redaction_count).sum();
        let token = Uuid::now_v7().to_string();
        let preview = DiagnosticsPreview {
            preview_token: token.clone(),
            artifacts: previews,
            total_events,
            total_redactions,
            summarized: total_events > 1_000 || source_bytes >= MAX_LOG_BYTES,
            expires_at: now_ms.saturating_add(10 * 60 * 1_000),
        };
        let snapshot = DiagnosticSnapshot {
            preview: preview.clone(),
            artifacts,
            cancelled: false,
        };
        self.previews
            .lock()
            .map_err(|_| diagnostics_io_error())?
            .insert(token, snapshot);
        Ok(preview)
    }

    pub fn cancel(&self, preview_token: &str) -> Result<(), AppError> {
        let mut previews = self.previews.lock().map_err(|_| diagnostics_io_error())?;
        let snapshot = previews
            .get_mut(preview_token)
            .ok_or_else(invalid_preview)?;
        snapshot.cancelled = true;
        Ok(())
    }

    pub fn create_bundle(
        &self,
        preview_token: &str,
        destination: &Path,
        now_ms: i64,
    ) -> Result<DiagnosticsBundleResult, AppError> {
        let snapshot = self
            .previews
            .lock()
            .map_err(|_| diagnostics_io_error())?
            .get(preview_token)
            .cloned()
            .ok_or_else(invalid_preview)?;
        if snapshot.cancelled || snapshot.preview.expires_at < now_ms {
            return Err(invalid_preview());
        }

        let mut destinations = self
            .destinations
            .lock()
            .map_err(|_| diagnostics_io_error())?;
        if !destinations.insert(destination.to_owned()) {
            return Err(AppError::new(
                ErrorCode::OperationBusy,
                "A diagnostic bundle is already using this destination.",
                true,
            ));
        }
        let result = write_bundle(destination, &snapshot);
        destinations.remove(destination);
        result
    }
}

#[derive(Clone, Debug)]
pub struct RetentionFile {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub bytes: u64,
}

pub fn retention_plan(files: &[RetentionFile], now: SystemTime) -> Vec<PathBuf> {
    let mut ordered = files.to_vec();
    ordered.sort_by_key(|file| file.modified);
    let mut retained_bytes = ordered.iter().map(|file| file.bytes).sum::<u64>();
    let mut removed = Vec::new();
    for file in ordered {
        let expired = now
            .duration_since(file.modified)
            .is_ok_and(|age| age > MAX_LOG_AGE);
        if expired || retained_bytes > MAX_LOG_BYTES {
            retained_bytes = retained_bytes.saturating_sub(file.bytes);
            removed.push(file.path);
        }
    }
    removed
}

pub fn redact_value(value: &Value) -> Result<(Value, usize), AppError> {
    let object = value.as_object().ok_or_else(redaction_failed)?;
    let mut safe = Map::new();
    let mut removed = 0;
    for (key, value) in object {
        if !ALLOWED_FIELDS.contains(&key.as_str()) {
            removed += 1;
            continue;
        }
        if contains_private_canary(value) || !safe_field_value(value) {
            return Err(redaction_failed());
        }
        safe.insert(key.clone(), value.clone());
    }
    Ok((Value::Object(safe), removed))
}

fn sanitize_log_file(path: &Path) -> Result<(Vec<Value>, usize), AppError> {
    let contents = fs::read_to_string(path).map_err(|_| diagnostics_io_error())?;
    let mut sanitized = Vec::new();
    let mut redactions = 0;
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).map_err(|_| redaction_failed())?;
        let (value, count) = redact_value(&value)?;
        if contains_private_canary(&value) {
            return Err(redaction_failed());
        }
        redactions += count;
        sanitized.push(value);
    }
    Ok((sanitized, redactions))
}

fn write_bundle(
    destination: &Path,
    snapshot: &DiagnosticSnapshot,
) -> Result<DiagnosticsBundleResult, AppError> {
    let parent = destination.parent().ok_or_else(diagnostics_io_error)?;
    fs::create_dir_all(parent).map_err(|_| diagnostics_io_error())?;
    let partial = destination.with_extension("partial");
    let document = serde_json::json!({
        "schemaVersion": 1,
        "preview": snapshot.preview,
        "artifacts": snapshot.artifacts,
        "upload": null,
    });
    if contains_private_canary(&document) {
        return Err(redaction_failed());
    }
    let encoded = serde_json::to_vec_pretty(&document).map_err(|_| redaction_failed())?;
    let write_result = fs::File::create(&partial)
        .and_then(|mut file| {
            file.write_all(&encoded)?;
            file.sync_all()
        })
        .and_then(|_| fs::rename(&partial, destination));
    if write_result.is_err() {
        let _ = fs::remove_file(&partial);
        return Err(diagnostics_io_error());
    }
    Ok(DiagnosticsBundleResult {
        file_name: destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("diagnostics.mtgodiag")
            .to_owned(),
        artifact_count: snapshot.artifacts.len(),
        event_count: snapshot.preview.total_events,
        network_requests: 0,
    })
}

fn is_safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._:-".contains(character))
        && !value.contains("CANARY_PRIVATE_")
}

fn safe_field_value(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => true,
        Value::String(value) => is_safe_token(value),
        Value::Array(_) | Value::Object(_) => false,
    }
}

fn contains_private_canary(value: &Value) -> bool {
    match value {
        Value::String(value) => value.contains("CANARY_PRIVATE_"),
        Value::Array(values) => values.iter().any(contains_private_canary),
        Value::Object(values) => values.values().any(contains_private_canary),
        _ => false,
    }
}

fn redaction_failed() -> AppError {
    AppError::new(
        ErrorCode::RedactionFailed,
        "Diagnostics could not be proven free of private notebook content.",
        false,
    )
}

fn diagnostics_io_error() -> AppError {
    AppError::new(
        ErrorCode::DestinationUnwritable,
        "The diagnostics source or destination is unavailable.",
        false,
    )
}

fn invalid_preview() -> AppError {
    AppError::new(
        ErrorCode::InvalidRequest,
        "Preview diagnostics before creating a support bundle.",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_event() -> SafeLogEvent {
        SafeLogEvent {
            timestamp: 1,
            level: LogLevel::Info,
            component: "detector".into(),
            event_code: "detector.available".into(),
            app_version: "0.1.0".into(),
            schema_version: 1,
            classifier_version: Some("2026.07.1".into()),
            duration_bucket: Some("under_100ms".into()),
            error_code: None,
        }
    }

    #[test]
    fn ut_089_allowlist_removes_every_prohibited_field_class() {
        let input = serde_json::json!({
            "timestamp": 1,
            "eventCode": "detector.available",
            "handle": "opponent",
            "note": "private note",
            "sourceUrl": "https://example.invalid",
            "ocrText": "visible text",
            "destination": "C:\\private",
        });
        let (safe, removed) = redact_value(&input).expect("redacted");
        assert_eq!(removed, 5);
        assert_eq!(safe.as_object().expect("object").len(), 2);
        assert_eq!(safe["eventCode"], "detector.available");
    }

    #[test]
    fn ut_090_canary_in_any_allowed_source_fails_closed() {
        let input = serde_json::json!({
            "eventCode": "CANARY_PRIVATE_HANDLE",
            "timestamp": 1,
        });
        assert_eq!(
            redact_value(&input).expect_err("must fail").code,
            ErrorCode::RedactionFailed
        );
    }

    #[test]
    fn ut_091_retention_uses_seven_days_or_twenty_mib_whichever_is_smaller() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * 24 * 60 * 60);
        let files = vec![
            RetentionFile {
                path: "expired.jsonl".into(),
                modified: SystemTime::UNIX_EPOCH,
                bytes: 1,
            },
            RetentionFile {
                path: "old.jsonl".into(),
                modified: now - Duration::from_secs(60),
                bytes: 12 * 1024 * 1024,
            },
            RetentionFile {
                path: "new.jsonl".into(),
                modified: now,
                bytes: 12 * 1024 * 1024,
            },
        ];
        let removed = retention_plan(&files, now);
        assert_eq!(
            removed,
            vec![PathBuf::from("expired.jsonl"), PathBuf::from("old.jsonl")]
        );
    }

    #[test]
    fn it_171_hostile_content_is_removed_before_preview() {
        let input = serde_json::json!({
            "eventCode": "safe.event",
            "note": "<script>CANARY_PRIVATE_NOTE</script>",
        });
        let (safe, removed) = redact_value(&input).expect("unknown field removed");
        assert_eq!(removed, 1);
        assert_eq!(safe["eventCode"], "safe.event");
    }

    #[test]
    fn safe_log_event_is_allowlisted_by_construction() {
        safe_event().validate().expect("safe event");
    }

    #[test]
    fn it_172_empty_logs_preview_environment_and_explicit_omission() {
        let directory = tempfile::tempdir().expect("temp");
        let missing = directory.path().join("missing");
        let preview = DiagnosticsService::default()
            .preview(&missing, safe_event(), 1)
            .expect("preview");
        assert_eq!(preview.total_events, 1);
        assert!(
            preview
                .artifacts
                .iter()
                .any(|artifact| artifact.omission_code.as_deref() == Some("no_diagnostic_events"))
        );
    }

    #[test]
    fn it_173_twenty_mib_preview_is_summarized_and_cancelable() {
        let directory = tempfile::tempdir().expect("temp");
        let logs = directory.path().join("logs");
        fs::create_dir_all(&logs).expect("logs");
        let padding = "x".repeat(MAX_LOG_BYTES as usize);
        fs::write(
            logs.join("large.jsonl"),
            serde_json::json!({
                "eventCode": "diagnostics.large",
                "timestamp": 1,
                "unknown": padding,
            })
            .to_string(),
        )
        .expect("large log");
        let service = DiagnosticsService::default();
        let preview = service.preview(&logs, safe_event(), 1).expect("preview");
        assert!(preview.summarized);
        service.cancel(&preview.preview_token).expect("cancel");
    }

    #[test]
    fn it_174_unwritable_destination_writes_nothing_and_has_no_network_path() {
        let directory = tempfile::tempdir().expect("temp");
        let service = DiagnosticsService::default();
        let preview = service
            .preview(directory.path(), safe_event(), 1)
            .expect("preview");
        let parent_file = directory.path().join("not-a-directory");
        fs::write(&parent_file, b"file").expect("parent file");
        let destination = parent_file.join("support.mtgodiag");
        assert_eq!(
            service
                .create_bundle(&preview.preview_token, &destination, 2)
                .expect_err("unwritable")
                .code,
            ErrorCode::DestinationUnwritable
        );
        assert!(!destination.exists());
    }

    #[test]
    fn it_175_concurrent_bundle_snapshots_never_mix_destinations() {
        let directory = tempfile::tempdir().expect("temp");
        let service = DiagnosticsService::default();
        let first = service
            .preview(directory.path(), safe_event(), 1)
            .expect("first");
        let mut second_event = safe_event();
        second_event.event_code = "diagnostics.second".into();
        let second = service
            .preview(directory.path(), second_event, 1)
            .expect("second");
        let first_path = directory.path().join("first.mtgodiag");
        let second_path = directory.path().join("second.mtgodiag");
        service
            .create_bundle(&first.preview_token, &first_path, 2)
            .expect("first bundle");
        service
            .create_bundle(&second.preview_token, &second_path, 2)
            .expect("second bundle");
        let first_bytes = fs::read_to_string(first_path).expect("first bytes");
        let second_bytes = fs::read_to_string(second_path).expect("second bytes");
        assert!(!first_bytes.contains("diagnostics.second"));
        assert!(second_bytes.contains("diagnostics.second"));
    }

    #[test]
    fn it_176_cancelled_bundle_never_publishes_partial_artifact() {
        let directory = tempfile::tempdir().expect("temp");
        let service = DiagnosticsService::default();
        let preview = service
            .preview(directory.path(), safe_event(), 1)
            .expect("preview");
        service.cancel(&preview.preview_token).expect("cancel");
        let destination = directory.path().join("cancelled.mtgodiag");
        assert!(
            service
                .create_bundle(&preview.preview_token, &destination, 2)
                .is_err()
        );
        assert!(!destination.exists());
        assert!(!destination.with_extension("partial").exists());
    }

    #[test]
    fn it_177_repeated_bundles_are_local_and_report_zero_network_requests() {
        let directory = tempfile::tempdir().expect("temp");
        let service = DiagnosticsService::default();
        let preview = service
            .preview(directory.path(), safe_event(), 1)
            .expect("preview");
        for index in 0..2 {
            let result = service
                .create_bundle(
                    &preview.preview_token,
                    &directory.path().join(format!("support-{index}.mtgodiag")),
                    2,
                )
                .expect("bundle");
            assert_eq!(result.network_requests, 0);
        }
    }

    #[test]
    fn it_178_creation_before_preview_is_rejected() {
        let directory = tempfile::tempdir().expect("temp");
        assert_eq!(
            DiagnosticsService::default()
                .create_bundle("missing", &directory.path().join("support.mtgodiag"), 1)
                .expect_err("missing preview")
                .code,
            ErrorCode::InvalidRequest
        );
    }

    #[test]
    fn it_179_missing_source_uses_omission_code_without_notebook_fallback() {
        let directory = tempfile::tempdir().expect("temp");
        let preview = DiagnosticsService::default()
            .preview(&directory.path().join("missing"), safe_event(), 1)
            .expect("preview");
        let json = serde_json::to_string(&preview).expect("json");
        assert!(json.contains("no_diagnostic_events"));
        assert!(!json.contains("opponent"));
        assert!(!json.contains("observation"));
    }

    #[test]
    fn it_180_full_retained_corpus_fails_if_any_canary_survives() {
        let directory = tempfile::tempdir().expect("temp");
        fs::write(
            directory.path().join("events.jsonl"),
            "{\"timestamp\":1,\"eventCode\":\"CANARY_PRIVATE_URL\"}\n",
        )
        .expect("log");
        assert_eq!(
            DiagnosticsService::default()
                .preview(directory.path(), safe_event(), 1)
                .expect_err("canary")
                .code,
            ErrorCode::RedactionFailed
        );
    }
}
