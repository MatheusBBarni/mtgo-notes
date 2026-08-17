use serde::{Deserialize, Serialize};

use super::{AppError, ErrorCode};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallerIdentity {
    Main,
    Overlay,
    Capture,
}

impl CallerIdentity {
    pub fn from_window_label(label: &str) -> Result<Self, AppError> {
        match label {
            "main" => Ok(Self::Main),
            "overlay" => Ok(Self::Overlay),
            "capture" => Ok(Self::Capture),
            _ => Err(AppError::new(
                ErrorCode::UnauthorizedCaller,
                "This window is not authorized to call application commands.",
                false,
            )),
        }
    }

    pub fn require(self, allowed: &[Self]) -> Result<(), AppError> {
        if allowed.contains(&self) {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::UnauthorizedCaller,
                "This command is not available to the calling window.",
                false,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ut_104_unknown_window_label_receives_no_caller_identity() {
        let error =
            CallerIdentity::from_window_label("unexpected-window").expect_err("unauthorized");
        assert_eq!(error.code, ErrorCode::UnauthorizedCaller);
    }
}
