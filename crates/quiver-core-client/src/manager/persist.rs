//! Saved connections and the device id, kept in the host's settings store.

use crate::stores::SettingsStore;
use crate::types::ConnectionConfig;
use crate::Result;

const CONNECTIONS_STORE: &str = "connections.json";
const STORE_KEY: &str = "connections";
const DEVICE_STORE: &str = "device.json";
const DEVICE_ID_KEY: &str = "device_id";

pub(super) fn load_remote_configs(settings: &dyn SettingsStore) -> Vec<ConnectionConfig> {
    let Some(value) = settings.get(CONNECTIONS_STORE, STORE_KEY) else {
        return vec![];
    };
    serde_json::from_value(value).unwrap_or_default()
}

pub(super) fn save_remote_configs(
    settings: &dyn SettingsStore,
    configs: &[ConnectionConfig],
) -> Result<()> {
    let value = serde_json::to_value(configs).map_err(|e| crate::Error::Settings(e.to_string()))?;
    settings.set(CONNECTIONS_STORE, STORE_KEY, value)
}

/// This installation's stable device-pairing id -- minted once, on first
/// use, and persisted so every pairing this app ever performs (across every
/// remote it is ever pointed at) is attributed to the same device, letting
/// an operator find and revoke it as one thing via `quiver auth devices
/// list` rather than once per connection.
pub(super) fn device_id(settings: &dyn SettingsStore) -> Result<String> {
    if let Some(id) = settings
        .get(DEVICE_STORE, DEVICE_ID_KEY)
        .and_then(|v| v.as_str().map(str::to_string))
    {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    settings.set(
        DEVICE_STORE,
        DEVICE_ID_KEY,
        serde_json::Value::String(id.clone()),
    )?;
    Ok(id)
}
