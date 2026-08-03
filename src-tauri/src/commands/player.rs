//! Main-window Player command admission helpers.
//!
//! The concrete service commands are added around the same façade by later
//! Player slices.  Keeping the caller/phase/payload gate here ensures every
//! extension uses one fail-closed policy rather than accepting renderer fields.

use crate::domain::InternalPhase;
use crate::ipc::CallerIdentity;
use crate::player::runtime::{
    PlayerCommandKind, PlayerError, PlayerPublicResultsRuntime, authorize_command,
};

#[allow(dead_code)]
pub fn admit_player_command(
    caller: CallerIdentity,
    phase: InternalPhase,
    command: PlayerCommandKind,
    payload_bytes: usize,
) -> Result<(), PlayerError> {
    authorize_command(caller, phase, command, payload_bytes)
}

#[allow(dead_code)]
pub fn provider_status(
    caller: CallerIdentity,
    runtime: &PlayerPublicResultsRuntime,
) -> Result<crate::player::runtime::PlayerProviderStatus, PlayerError> {
    admit_player_command(caller, InternalPhase::Idle, PlayerCommandKind::Status, 0)?;
    runtime.consent_status(crate::player::models::PlayerSourceRoute::CensusMocs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_only_admission_is_shared_by_future_handlers() {
        assert!(
            admit_player_command(
                CallerIdentity::Main,
                InternalPhase::Idle,
                PlayerCommandKind::Status,
                0
            )
            .is_ok()
        );
        assert!(
            admit_player_command(
                CallerIdentity::Overlay,
                InternalPhase::Idle,
                PlayerCommandKind::Status,
                0
            )
            .is_err()
        );
    }
}
