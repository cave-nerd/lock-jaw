use std::path::Path;

use anyhow::Result;
use tracing::{info, warn};

use crate::PluginManifest;

/// A loaded plugin instance.
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    // Future: wasmtime Instance lives here
}

/// Hosts all active plugins. Plugins are loaded from a plugins directory.
///
/// # Plugin ABI (WASM exports expected)
/// ```text
/// on_load()
/// on_note_open(json_ptr: i32, len: i32)
/// on_note_save(json_ptr: i32, len: i32) -> i32
/// get_commands() -> i32
/// ```
///
/// # Host imports provided to plugins
/// ```text
/// host_log(ptr: i32, len: i32)
/// host_read_note(path_ptr: i32, path_len: i32) -> i32
/// ```
pub struct PluginHost {
    pub plugins: Vec<LoadedPlugin>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    /// Scan a directory for `plugin.toml` manifests and load each plugin.
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            info!("Plugin dir does not exist, skipping: {}", dir.display());
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let manifest_path = entry.path().join("plugin.toml");
            if manifest_path.exists() {
                match self.load_plugin(&manifest_path) {
                    Ok(_) => {}
                    Err(e) => warn!("Failed to load plugin at {}: {e}", manifest_path.display()),
                }
            }
        }
        Ok(())
    }

    fn load_plugin(&mut self, manifest_path: &Path) -> Result<()> {
        let raw = std::fs::read_to_string(manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&raw)?;
        info!("Loaded plugin: {} v{}", manifest.name, manifest.version);
        // TODO(Phase 2): instantiate wasmtime engine + store + module here
        self.plugins.push(LoadedPlugin { manifest });
        Ok(())
    }

    /// Fire `on_note_open` for all plugins.
    pub fn on_note_open(&self, path: &str) {
        for plugin in &self.plugins {
            // TODO(Phase 2): call WASM export
            let _ = (plugin, path);
        }
    }

    /// Fire `on_note_save` for all plugins. Returns (possibly modified) content.
    pub fn on_note_save(&self, path: &str, content: String) -> String {
        let result = content;
        for plugin in &self.plugins {
            // TODO(Phase 2): call WASM export and replace result
            let _ = (plugin, path);
        }
        result
    }

    /// Return all commands registered by plugins.
    pub fn commands(&self) -> Vec<PluginCommand> {
        // TODO(Phase 2): collect from WASM exports
        Vec::new()
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
}
