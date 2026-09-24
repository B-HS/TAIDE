use serde_json::Value;
use taide_model::settings::{Settings, SettingsPatch};

use crate::types::REMOTE_OWNER_LABEL;

/// Replaces every owner field in untrusted remote request arguments with the remote label.
pub fn enforce_remote_owner_label(mut args: Value) -> Value {
    match &mut args {
        Value::Object(fields) => {
            if fields.contains_key("owner") {
                fields.insert(
                    "owner".to_string(),
                    Value::String(REMOTE_OWNER_LABEL.to_string()),
                );
            }
            for field in fields.values_mut() {
                *field = enforce_remote_owner_label(std::mem::take(field));
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                *item = enforce_remote_owner_label(std::mem::take(item));
            }
        }
        _ => {}
    }
    args
}

/// Removes settings fields that a remote session must not change persistently.
pub fn strip_remote_gated_settings_patch(mut patch: SettingsPatch) -> SettingsPatch {
    patch.remote_password_only_login = None;
    patch.remote_allowed_hosts = None;
    patch.shell_override = None;
    patch.ai_omlx_base_url = None;
    patch
}

/// Restores protected fields from the current settings before a remote full-file write.
pub fn strip_remote_gated_settings(mut next: Settings, current: &Settings) -> Settings {
    next.remote_password_only_login = current.remote_password_only_login;
    next.remote_allowed_hosts = current.remote_allowed_hosts.clone();
    next.shell_override = current.shell_override.clone();
    next.ai_omlx_base_url = current.ai_omlx_base_url.clone();
    next
}
