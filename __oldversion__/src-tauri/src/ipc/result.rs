use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::Serialize;

use super::AppError;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum CommandResult<T> {
    Success { ok: bool, data: T, revision: u64 },
    Failure { ok: bool, error: AppError },
}

impl<T> CommandResult<T> {
    pub fn success(data: T, revision: u64) -> Self {
        Self::Success {
            ok: true,
            data,
            revision,
        }
    }

    pub fn failure(error: AppError) -> Self {
        Self::Failure { ok: false, error }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    pub fn revision(&self) -> Option<u64> {
        match self {
            Self::Success { revision, .. } => Some(*revision),
            Self::Failure { .. } => None,
        }
    }
}

pub fn panic_boundary<T>(
    correlation_code: &str,
    operation: impl FnOnce() -> CommandResult<T>,
) -> CommandResult<T> {
    catch_unwind(AssertUnwindSafe(operation))
        .unwrap_or_else(|_| CommandResult::failure(AppError::internal(correlation_code)))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::ipc::{AppError, ErrorCode};

    #[test]
    fn ut_113_success_envelope_is_plain_json() {
        let value = serde_json::to_value(CommandResult::success(json!({ "ready": true }), 7))
            .expect("serialize command success");

        assert_eq!(
            value,
            json!({ "ok": true, "data": { "ready": true }, "revision": 7 })
        );
    }

    #[test]
    fn ut_114_expected_error_has_stable_shape() {
        let error = AppError::new(ErrorCode::InvalidRequest, "Invalid input.", false)
            .with_field("idempotencyKey");
        let value =
            serde_json::to_value(CommandResult::<()>::failure(error)).expect("serialize error");

        assert_eq!(
            value,
            json!({
                "ok": false,
                "error": {
                    "code": "invalid_request",
                    "message": "Invalid input.",
                    "retryable": false,
                    "field": "idempotencyKey"
                }
            })
        );
    }

    #[test]
    fn ut_115_panic_boundary_redacts_sensitive_debug_text() {
        let result: CommandResult<()> =
            panic_boundary("corr-safe-115", || panic!("secret database path"));
        let value = serde_json::to_value(result).expect("serialize panic result");
        let rendered = value.to_string();

        assert_eq!(value["error"]["code"], "internal_error");
        assert!(rendered.contains("corr-safe-115"));
        assert!(!rendered.contains("secret database path"));
    }
}
