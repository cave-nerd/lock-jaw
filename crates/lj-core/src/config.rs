use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::webdav::WebDavConfig;

fn default_heading_scale() -> f32 {
    2.0
}
fn default_icon_style() -> String {
    "emoji".to_string()
}
fn default_view_mode() -> String {
    "both".to_string()
}
fn default_autosave() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the notes vault directory.
    pub vault_path: PathBuf,
    /// Name of the active theme file (without extension), e.g. "dark".
    pub theme: String,
    /// Font size for the editor and preview body text.
    pub font_size: f32,
    /// Heading scale: H1 size = font_size × heading_scale.
    /// Controls how much bigger H1 is vs body; other heading levels scale proportionally.
    #[serde(default = "default_heading_scale")]
    pub heading_scale: f32,
    /// Custom text color override (hex `#rrggbb`). Empty string = use theme default.
    #[serde(default)]
    pub text_color: String,
    /// Sidebar icon style. Options: "unicode", "emoji", "ascii", "none".
    #[serde(default = "default_icon_style")]
    pub icon_style: String,
    /// Editor/preview view mode. Options: "both", "editor", "preview".
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
    /// WebDAV sync settings.
    #[serde(default)]
    pub webdav: WebDavConfig,
    /// Plugin names that are explicitly disabled in the addon manager.
    #[serde(default)]
    pub disabled_plugins: Vec<String>,
    /// Save the current note automatically ~1 second after the user stops typing.
    #[serde(default = "default_autosave")]
    pub autosave_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        let vault_path = directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("LockJaw");
        Self {
            vault_path,
            theme: "dark".to_string(),
            font_size: 14.0,
            heading_scale: default_heading_scale(),
            text_color: String::new(),
            icon_style: default_icon_style(),
            view_mode: default_view_mode(),
            webdav: WebDavConfig::default(),
            disabled_plugins: Vec::new(),
            autosave_enabled: default_autosave(),
        }
    }
}

impl Config {
    /// Returns the XDG config dir for Lock Jaw.
    pub fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "lockjaw", "LockJaw").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Returns the path to the config file.
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("config.toml"))
    }

    /// Returns the path where themes are stored.
    pub fn themes_dir() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("themes"))
    }

    /// Returns the path where plugins are stored.
    pub fn plugins_dir() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("plugins"))
    }

    /// Ensure all required application directories exist, creating them if needed.
    pub fn ensure_dirs() {
        if let Some(dir) = Self::config_dir() {
            std::fs::create_dir_all(&dir).ok();
        }
        if let Some(dir) = Self::plugins_dir() {
            std::fs::create_dir_all(&dir).ok();
        }
        if let Some(dir) = Self::themes_dir() {
            std::fs::create_dir_all(&dir).ok();
        }
        // Also create the default vault directory
        if let Some(vault) =
            directories::UserDirs::new().and_then(|u| u.document_dir().map(|d| d.join("LockJaw")))
        {
            std::fs::create_dir_all(&vault).ok();
        }
    }

    /// Load config from disk, or return defaults if not found.
    pub fn load() -> Self {
        Self::ensure_dirs();
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::config_path() else {
            anyhow::bail!("Could not determine config directory");
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
}
