use serde::{Deserialize, Serialize};

use crate::ipc::{CallerIdentity, CommandResult, panic_boundary};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayInteractionRequest {
    pub expanded: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayInteractionView {
    pub expanded: bool,
    pub click_through: bool,
}

#[tauri::command]
pub fn set_overlay_interaction(
    window: tauri::WebviewWindow,
    request: OverlayInteractionRequest,
) -> CommandResult<OverlayInteractionView> {
    panic_boundary("set-overlay-interaction-command", || {
        let caller = match CallerIdentity::from_window_label(window.label()) {
            Ok(caller) => caller,
            Err(error) => return CommandResult::failure(error),
        };
        if let Err(error) = caller.require(&[CallerIdentity::Overlay]) {
            return CommandResult::failure(error);
        }
        match crate::shell::set_overlay_expanded(&window, request.expanded) {
            Ok(()) => CommandResult::success(
                OverlayInteractionView {
                    expanded: request.expanded,
                    click_through: !request.expanded,
                },
                1,
            ),
            Err(error) => CommandResult::failure(error),
        }
    })
}
