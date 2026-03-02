use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::{info, warn};

use crate::PluginManifest;

/// A loaded plugin instance.
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    /// Directory this plugin was loaded from (used for removal).
    pub dir: PathBuf,
    /// Whether the plugin is enabled. Disabled plugins are still listed in the
    /// manager but their hooks are not fired. Persisted in Config.
    pub enabled: bool,
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
    /// `disabled` is the list of plugin names that should start as disabled.
    pub fn load_from_dir(&mut self, dir: &Path, disabled: &[String]) -> Result<()> {
        if !dir.exists() {
            info!("Plugin dir does not exist, skipping: {}", dir.display());
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let plugin_dir = entry.path();
            let manifest_path = plugin_dir.join("plugin.toml");
            if manifest_path.exists() {
                match self.load_plugin(&manifest_path, &plugin_dir, disabled) {
                    Ok(_) => {}
                    Err(e) => warn!("Failed to load plugin at {}: {e}", manifest_path.display()),
                }
            }
        }
        Ok(())
    }

    /// Clear all plugins and re-scan the directory.
    pub fn reload(&mut self, dir: &Path, disabled: &[String]) -> Result<()> {
        self.plugins.clear();
        self.load_from_dir(dir, disabled)
    }

    /// Delete a plugin's directory from the filesystem and remove it from the list.
    /// Returns `Ok(true)` if the plugin was found and removed.
    pub fn remove_plugin(&mut self, name: &str) -> Result<bool> {
        let pos = self.plugins.iter().position(|p| p.manifest.name == name);
        match pos {
            None => Ok(false),
            Some(idx) => {
                let plugin = self.plugins.remove(idx);
                if plugin.dir.exists() {
                    std::fs::remove_dir_all(&plugin.dir)?;
                    info!("Removed plugin directory: {}", plugin.dir.display());
                }
                Ok(true)
            }
        }
    }

    fn load_plugin(
        &mut self,
        manifest_path: &Path,
        plugin_dir: &Path,
        disabled: &[String],
    ) -> Result<()> {
        let raw = std::fs::read_to_string(manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&raw)?;
        let enabled = !disabled.iter().any(|d| d == &manifest.name);
        info!("Loaded plugin: {} v{} (enabled={enabled})", manifest.name, manifest.version);
        // TODO(Phase 2): instantiate wasmtime engine + store + module here
        self.plugins.push(LoadedPlugin {
            manifest,
            dir: plugin_dir.to_path_buf(),
            enabled,
        });
        Ok(())
    }

    /// Fire `on_note_open` for all enabled plugins.
    pub fn on_note_open(&self, path: &str) {
        for plugin in self.plugins.iter().filter(|p| p.enabled) {
            // TODO(Phase 2): call WASM export
            let _ = (plugin, path);
        }
    }

    /// Fire `on_note_save` for all enabled plugins. Returns (possibly modified) content.
    pub fn on_note_save(&self, path: &str, content: String) -> String {
        let result = content;
        for plugin in self.plugins.iter().filter(|p| p.enabled) {
            // TODO(Phase 2): call WASM export and replace result
            let _ = (plugin, path);
        }
        result
    }

    /// Return all commands registered by enabled plugins.
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
