use mtgo_opponent_notes::commands::{bootstrap_for, get_settings_for, update_settings_for};
use mtgo_opponent_notes::ipc::{CallerIdentity, CommandResult};
use mtgo_opponent_notes::settings::{AppState, Settings, UpdateSettingsRequest};
use serde_json::json;

#[test]
fn it_191_main_bootstrap_returns_safe_typed_state_and_revision() {
    let state = AppState::default();
    let value = serde_json::to_value(bootstrap_for(CallerIdentity::Main, &state))
        .expect("serialize bootstrap");

    assert_eq!(value["ok"], true);
    assert_eq!(value["revision"], 1);
    assert_eq!(value["data"]["app"]["localOnly"], true);
    assert_eq!(value["data"]["caller"], "main");
    assert_eq!(value["data"]["encounter"], serde_json::Value::Null);
    assert!(value["data"].get("secret").is_none());
}

#[test]
fn it_192_get_settings_returns_typed_non_secret_preferences() {
    let state = AppState::default();
    let value = serde_json::to_value(get_settings_for(CallerIdentity::Main, &state))
        .expect("serialize settings");

    assert_eq!(
        value,
        json!({
            "ok": true,
            "data": {
                "schemaVersion": 1,
                "providerAccessEnabled": false,
                "overlayEnabled": true,
                "trayEnabled": true,
                "launchWithWindows": false,
                "updateChecksEnabled": false,
                "classifierUpdateChecksEnabled": false,
                "diagnosticsEnabled": false
            },
            "revision": 1
        })
    );
}

#[test]
fn it_193_update_settings_persists_preferences_at_current_revision() {
    let state = AppState::default();
    let request = UpdateSettingsRequest {
        idempotency_key: "018f6b8a-9a21-7c4d-8f11-0123456789ab".to_owned(),
        expected_revision: 1,
        settings: Settings {
            overlay_enabled: false,
            launch_with_windows: true,
            update_checks_enabled: true,
            ..Settings::default()
        },
    };

    let updated = serde_json::to_value(update_settings_for(CallerIdentity::Main, &state, request))
        .expect("serialize update");
    let persisted = serde_json::to_value(get_settings_for(CallerIdentity::Main, &state))
        .expect("serialize persisted settings");

    assert_eq!(updated["revision"], 2);
    assert_eq!(persisted["revision"], 2);
    assert_eq!(persisted["data"]["overlayEnabled"], false);
    assert_eq!(persisted["data"]["launchWithWindows"], true);
    assert_eq!(persisted["data"]["updateChecksEnabled"], true);
}

#[test]
fn ut_116_invalid_idempotency_key_returns_invalid_request() {
    let state = AppState::default();
    for request in [
        UpdateSettingsRequest {
            idempotency_key: "not-a-uuid".to_owned(),
            expected_revision: 1,
            settings: Settings::default(),
        },
        serde_json::from_value(json!({
            "expectedRevision": 1,
            "settings": {
                "overlayEnabled": true,
                "launchWithWindows": false,
                "updateChecksEnabled": false
            }
        }))
        .expect("missing idempotency key defaults to invalid input"),
    ] {
        let result: CommandResult<Settings> =
            update_settings_for(CallerIdentity::Main, &state, request);
        let value = serde_json::to_value(result).expect("serialize invalid request");

        assert_eq!(value["error"]["code"], "invalid_request");
        assert_eq!(value["error"]["field"], "idempotencyKey");
    }
}
