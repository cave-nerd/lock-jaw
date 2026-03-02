use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::Note;

/// A vault is a directory containing markdown notes.
pub struct Vault {
    pub root: PathBuf,
    /// All discovered `.md` file paths, sorted.
    pub entries: Vec<PathBuf>,
}

impl Vault {
    /// Open a vault at the given directory, discovering all `.md` files.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        anyhow::ensure!(root.is_dir(), "Vault path is not a directory: {}", root.display());
        let mut vault = Self { root, entries: Vec::new() };
        vault.refresh()?;
        Ok(vault)
    }

    /// Re-scan the vault directory for `.md` files.
    pub fn refresh(&mut self) -> Result<()> {
        self.entries = collect_md_files(&self.root)?;
        self.entries.sort();
        Ok(())
    }

    /// Load a note by path.
    pub fn load_note(&self, path: &Path) -> Result<Note> {
        Note::load(path)
    }

    /// Create a new note with the given name in the vault root.
    pub fn create_note(&mut self, name: &str) -> Result<Note> {
        let filename = sanitize_filename(name);
        let path = self.root.join(format!("{filename}.md"));
        let initial = format!("# {name}\n\n");
        std::fs::write(&path, &initial)?;
        self.entries.push(path.clone());
        self.entries.sort();
        Note::load(&path)
    }

    /// Delete a note from disk and remove from entries.
    pub fn delete_note(&mut self, path: &Path) -> Result<()> {
        std::fs::remove_file(path)?;
        self.entries.retain(|p| p != path);
        Ok(())
    }

    /// Path relative to vault root, for display.
    pub fn relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
    }
}

fn collect_md_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Recurse, skipping hidden directories
            if !entry.file_name().to_string_lossy().starts_with('.') {
                results.extend(collect_md_files(&path)?);
            }
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            results.push(path);
        }
    }
    Ok(results)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}
