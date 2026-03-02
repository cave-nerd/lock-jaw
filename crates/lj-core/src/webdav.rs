use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::Method;
use serde::{Deserialize, Serialize};

/// WebDAV connection settings, stored in config.toml under `[webdav]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {
    /// Enable or disable syncing.
    pub enabled: bool,
    /// Full URL to the remote notes directory, e.g. `https://cloud.example.com/dav/notes/`
    pub url: String,
    pub username: String,
    pub password: String,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            username: String::new(),
            password: String::new(),
        }
    }
}

/// Results from a sync operation.
#[derive(Debug, Default)]
pub struct SyncStats {
    pub uploaded: usize,
    pub downloaded: usize,
    pub errors: Vec<String>,
}

pub struct WebDavClient {
    config: WebDavConfig,
    client: Client,
    /// Base URL, guaranteed to end with `/`
    base_url: String,
}

impl WebDavClient {
    pub fn new(config: WebDavConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let base_url = if config.url.ends_with('/') {
            config.url.clone()
        } else {
            format!("{}/", config.url)
        };
        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    /// PROPFIND depth=1 to list all `.md` filenames in the remote directory.
    pub fn list_remote(&self) -> Result<HashSet<String>> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop><D:resourcetype/></D:prop>
</D:propfind>"#;

        let resp = self
            .client
            .request(Method::from_bytes(b"PROPFIND")?, &self.base_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()?;

        let text = resp.text()?;
        Ok(parse_md_hrefs(&text))
    }

    /// PUT a local note's content to the remote server.
    pub fn upload(&self, filename: &str, content: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, percent_encode(filename));
        self.client
            .put(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "text/markdown; charset=utf-8")
            .body(content.to_string())
            .send()?
            .error_for_status()?;
        Ok(())
    }

    /// GET a file from the remote server.
    pub fn download(&self, filename: &str) -> Result<String> {
        let url = format!("{}{}", self.base_url, percent_encode(filename));
        let text = self
            .client
            .get(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()?
            .error_for_status()?
            .text()?;
        Ok(text)
    }

    /// MKCOL to create the remote directory (idempotent — 405 means already exists).
    pub fn ensure_remote_dir(&self) -> Result<()> {
        let resp = self
            .client
            .request(Method::from_bytes(b"MKCOL")?, &self.base_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()?;
        match resp.status().as_u16() {
            201 | 301 | 405 | 409 => Ok(()), // created / redirect / already exists / conflict
            s => anyhow::bail!("MKCOL returned unexpected status {s}"),
        }
    }

    /// Sync local vault with the remote WebDAV directory.
    ///
    /// Strategy: upload every local note, then download any remote notes
    /// that don't exist locally. Local always wins on conflict.
    pub fn sync(&self, vault_root: &Path) -> Result<SyncStats> {
        let mut stats = SyncStats::default();

        // Ensure remote directory exists
        self.ensure_remote_dir().ok();

        // List remote .md files
        let remote_names = self.list_remote().unwrap_or_default();

        // Collect local .md files
        let local_files = collect_md_files(vault_root)?;
        let mut local_names: HashSet<String> = HashSet::new();

        // Upload all local notes
        for path in &local_files {
            let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            local_names.insert(filename.to_string());

            match std::fs::read_to_string(path) {
                Ok(content) => match self.upload(filename, &content) {
                    Ok(_) => stats.uploaded += 1,
                    Err(e) => stats.errors.push(format!("Upload {filename}: {e}")),
                },
                Err(e) => stats.errors.push(format!("Read {filename}: {e}")),
            }
        }

        // Download remote notes that don't exist locally
        for remote_name in &remote_names {
            if local_names.contains(remote_name) {
                continue;
            }
            match self.download(remote_name) {
                Ok(content) => {
                    let dest = vault_root.join(remote_name);
                    match std::fs::write(&dest, &content) {
                        Ok(_) => stats.downloaded += 1,
                        Err(e) => stats.errors.push(format!("Write {remote_name}: {e}")),
                    }
                }
                Err(e) => stats.errors.push(format!("Download {remote_name}: {e}")),
            }
        }

        Ok(stats)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract `.md` filenames from a PROPFIND XML response.
/// Handles both `<D:href>` and `<href>` namespace variants.
fn parse_md_hrefs(xml: &str) -> HashSet<String> {
    let mut results = HashSet::new();
    let mut remaining = xml;

    while !remaining.is_empty() {
        // Find opening href tag (with or without namespace prefix)
        let tag_start = remaining
            .find("<D:href>")
            .map(|i| (i, 8usize))
            .or_else(|| remaining.find("<href>").map(|i| (i, 6)));

        let Some((start, tag_len)) = tag_start else {
            break;
        };
        remaining = &remaining[start + tag_len..];

        let close = if remaining.contains("</D:href>") {
            "</D:href>"
        } else {
            "</href>"
        };
        let Some(end) = remaining.find(close) else {
            break;
        };

        let href = remaining[..end].trim();
        remaining = &remaining[end + close.len()..];

        if href.ends_with(".md") {
            // Take only the final path segment as the filename
            if let Some(name) = href.split('/').filter(|s| !s.is_empty()).last() {
                if let Ok(decoded) = percent_decode(name) {
                    results.insert(decoded);
                }
            }
        }
    }

    results
}

/// Minimal percent-encode for filenames: encodes spaces and non-ASCII.
fn percent_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c]
            } else if c == ' ' {
                "%20".chars().collect()
            } else {
                format!("%{:02X}", c as u32).chars().collect()
            }
        })
        .collect()
}

/// Decode `%XX` sequences in a URL-encoded filename.
fn percent_decode(s: &str) -> Result<String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().ok_or_else(|| anyhow::anyhow!("Truncated %"))?;
            let h2 = chars.next().ok_or_else(|| anyhow::anyhow!("Truncated %"))?;
            let byte = u8::from_str_radix(&format!("{h1}{h2}"), 16)?;
            out.push(byte as char);
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn collect_md_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    if !dir.exists() {
        return Ok(results);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            results.push(path);
        }
    }
    Ok(results)
}
