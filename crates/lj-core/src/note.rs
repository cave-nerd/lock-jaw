use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Frontmatter parsed from YAML between `---` delimiters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// An in-memory representation of a single markdown note.
#[derive(Debug, Clone)]
pub struct Note {
    /// Absolute path to the `.md` file on disk.
    pub path: PathBuf,
    /// Raw markdown content (including frontmatter if present).
    pub raw: String,
    /// Parsed frontmatter (empty if none).
    pub frontmatter: Frontmatter,
    /// Markdown body (frontmatter stripped).
    pub body: String,
    /// Whether there are unsaved local edits.
    pub dirty: bool,
}

impl Note {
    /// Load a note from disk, parsing frontmatter if present.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let raw = std::fs::read_to_string(&path)?;
        let (frontmatter, body) = parse_frontmatter(&raw);
        Ok(Self {
            path,
            raw,
            frontmatter,
            body,
            dirty: false,
        })
    }

    /// Save the current body (and frontmatter) back to disk.
    pub fn save(&mut self) -> Result<()> {
        std::fs::write(&self.path, &self.raw)?;
        self.dirty = false;
        Ok(())
    }

    /// Update the raw content from editor and re-parse frontmatter.
    pub fn update_raw(&mut self, new_raw: String) {
        let (fm, body) = parse_frontmatter(&new_raw);
        self.raw = new_raw;
        self.frontmatter = fm;
        self.body = body;
        self.dirty = true;
    }

    /// Display name: frontmatter title, or stem of filename.
    pub fn display_name(&self) -> &str {
        if let Some(title) = self.frontmatter.title.as_deref() {
            return title;
        }
        self.path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
    }
}

/// Split raw markdown into (frontmatter, body).
/// Frontmatter is YAML between leading `---` delimiters.
fn parse_frontmatter(raw: &str) -> (Frontmatter, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return (Frontmatter::default(), raw.to_string());
    }
    // Find closing ---
    let after_open = &trimmed[3..];
    if let Some(end) = after_open.find("\n---") {
        let yaml = &after_open[..end];
        let body_start = end + 4; // skip \n---
        let body = after_open[body_start..].trim_start().to_string();
        let frontmatter = serde_yaml::from_str(yaml).unwrap_or_default();
        (frontmatter, body)
    } else {
        (Frontmatter::default(), raw.to_string())
    }
}
