use serde::{Deserialize, Serialize};

/// Metadata declared in a plugin's `plugin.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    /// The WASM file name relative to the manifest.
    pub wasm: String,
}
