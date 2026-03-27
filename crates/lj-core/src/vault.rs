use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::Note;

/// A vault is a directory containing markdown notes.
pub struct Vault {
    pub root: PathBuf,
    /// All discovered `.md` file paths (absolute), sorted.
    pub entries: Vec<PathBuf>,
    /// Relative names of direct sub-directories of root, sorted.
    /// Tracked separately so empty sections still appear in the UI.
    pub folders: Vec<PathBuf>,
}

impl Vault {
    /// Open a vault at the given directory, discovering all `.md` files.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        anyhow::ensure!(
            root.is_dir(),
            "Vault path is not a directory: {}",
            root.display()
        );
        let mut vault = Self {
            root,
            entries: Vec::new(),
            folders: Vec::new(),
        };
        vault.refresh()?;
        Ok(vault)
    }

    /// Re-scan the vault directory for `.md` files and direct sub-directories.
    pub fn refresh(&mut self) -> Result<()> {
        self.entries = collect_md_files(&self.root)?;
        self.entries.sort();
        self.folders = collect_direct_subdirs(&self.root)?;
        self.folders.sort();
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

    /// Create a new note inside a sub-directory (folder relative to vault root).
    pub fn create_note_in(&mut self, folder: &Path, name: &str) -> Result<Note> {
        let filename = sanitize_filename(name);
        let abs_folder = self.root.join(folder);
        std::fs::create_dir_all(&abs_folder)?;
        let path = abs_folder.join(format!("{filename}.md"));
        let initial = format!("# {name}\n\n");
        std::fs::write(&path, &initial)?;
        self.entries.push(path.clone());
        self.entries.sort();
        // Ensure the folder is tracked
        let rel = folder
            .components()
            .next()
            .map(|c| PathBuf::from(c.as_os_str()))
            .unwrap_or(folder.to_path_buf());
        if !self.folders.contains(&rel) {
            self.folders.push(rel);
            self.folders.sort();
        }
        Note::load(&path)
    }

    /// Create a new section (sub-directory) inside the vault root.
    pub fn create_folder(&mut self, name: &str) -> Result<PathBuf> {
        let dir_name = sanitize_filename(name);
        let path = self.root.join(&dir_name);
        std::fs::create_dir_all(&path)?;
        let rel = PathBuf::from(&dir_name);
        if !self.folders.contains(&rel) {
            self.folders.push(rel);
            self.folders.sort();
        }
        Ok(path)
    }

    /// Rename a note file. Returns the new absolute path.
    pub fn rename_note(&mut self, old_path: &Path, new_name: &str) -> Result<PathBuf> {
        let filename = sanitize_filename(new_name);
        let parent = old_path.parent().unwrap_or(&self.root);
        let new_path = parent.join(format!("{filename}.md"));
        if new_path == old_path {
            return Ok(old_path.to_path_buf());
        }
        std::fs::rename(old_path, &new_path)?;
        self.entries.retain(|p| p != old_path);
        self.entries.push(new_path.clone());
        self.entries.sort();
        Ok(new_path)
    }

    /// Move a note to a different folder (or to the vault root when `target_folder` is `None`).
    /// Returns the new absolute path.
    pub fn move_note(&mut self, path: &Path, target_folder: Option<&Path>) -> Result<PathBuf> {
        let filename = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Note path has no filename"))?;
        let dest_dir = match target_folder {
            Some(folder) => {
                let abs = self.root.join(folder);
                std::fs::create_dir_all(&abs)?;
                abs
            }
            None => self.root.clone(),
        };
        let new_path = dest_dir.join(filename);
        if new_path == path {
            return Ok(path.to_path_buf());
        }
        std::fs::rename(path, &new_path)?;
        self.entries.retain(|p| p != path);
        self.entries.push(new_path.clone());
        self.entries.sort();
        Ok(new_path)
    }

    /// Rename a section (sub-directory). Moves the directory on disk and updates
    /// all in-memory entry paths that lived inside it. Returns the new absolute path.
    pub fn rename_folder(&mut self, old_rel: &Path, new_name: &str) -> Result<PathBuf> {
        let new_dir_name = sanitize_filename(new_name);
        let old_abs = self.root.join(old_rel);
        let new_abs = self.root.join(&new_dir_name);
        if new_abs == old_abs {
            return Ok(old_abs);
        }
        std::fs::rename(&old_abs, &new_abs)?;
        // Re-root every entry that was inside the old folder.
        for entry in &mut self.entries {
            if entry.starts_with(&old_abs) {
                let tail = entry.strip_prefix(&old_abs).unwrap().to_path_buf();
                *entry = new_abs.join(tail);
            }
        }
        // Update the folders list.
        let old_rel_pb = old_rel.to_path_buf();
        self.folders.retain(|f| f != &old_rel_pb);
        let new_rel = PathBuf::from(&new_dir_name);
        if !self.folders.contains(&new_rel) {
            self.folders.push(new_rel);
            self.folders.sort();
        }
        Ok(new_abs)
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

/// Returns the names of direct sub-directories of `dir` as relative `PathBuf`s.
fn collect_direct_subdirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_dir() && !entry.file_name().to_string_lossy().starts_with('.') {
            results.push(PathBuf::from(entry.file_name()));
        }
    }
    Ok(results)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}
