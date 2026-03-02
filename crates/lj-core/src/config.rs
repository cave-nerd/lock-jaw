use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::webdav::WebDavConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the notes vault directory.
    pub vault_path: PathBuf,
    /// Name of the active theme file (without extension), e.g. "dark".
    pub theme: String,
    /// Font size for the editor.
    pub font_size: f32,
    /// WebDAV sync settings.
    #[serde(default)]
    pub webdav: WebDavConfig,
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
            webdav: WebDavConfig::default(),
        }
    }
}

impl Config {
    /// Returns the XDG config dir for Lock Jaw.
    pub fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "lockjaw", "LockJaw")
            .map(|dirs| dirs.config_dir().to_path_buf())
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

    /// Load config from disk, or return defaults if not found.
    pub fn load() -> Self {
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
