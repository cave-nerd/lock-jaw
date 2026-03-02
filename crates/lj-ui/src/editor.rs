use std::path::PathBuf;

use egui::{Color32, FontId, RichText, Ui};
use lj_core::Note;
use lj_md::MarkdownCache;

pub struct EditorPane {
    pub markdown_cache: MarkdownCache,
    /// Draft title shown in the rename bar; kept in sync when the open note changes.
    rename_text: String,
    /// Tracks which note is open so we can reset `rename_text` on switch.
    last_note_path: PathBuf,
}

impl Default for EditorPane {
    fn default() -> Self {
        Self {
            markdown_cache: MarkdownCache::new(),
            rename_text: String::new(),
            last_note_path: PathBuf::new(),
        }
    }
}

impl EditorPane {
    /// Render the title bar + split editor / preview pane.
    ///
    /// Returns `(content_modified, rename_request)`:
    /// - `content_modified` — note body text was edited this frame
    /// - `rename_request`   — `Some(new_name)` when the user commits a rename (Enter key)
    pub fn show(&mut self, ui: &mut Ui, note: &mut Note, font_size: f32) -> (bool, Option<String>) {
        let mut modified = false;
        let mut rename_request: Option<String> = None;

        let note_id = note.path.to_string_lossy().into_owned();
        let stem = note.path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Reset draft title when a different note is opened.
        if self.last_note_path != note.path {
            self.rename_text = stem.clone();
            self.last_note_path = note.path.clone();
        }

        // ── Rename / title bar ──────────────────────────────────────────
        let title_resp = ui.add(
            egui::TextEdit::singleline(&mut self.rename_text)
                .font(FontId::proportional(font_size * 1.4))
                .desired_width(f32::INFINITY)
                .frame(false)
                .hint_text("Note title…"),
        );

        let enter_pressed  = title_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let escape_pressed = title_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed {
            self.rename_text = stem.clone();
        } else if enter_pressed
            && !self.rename_text.trim().is_empty()
            && self.rename_text.trim() != stem
        {
            rename_request = Some(self.rename_text.trim().to_string());
        }

        ui.separator();

        // ── Split pane: editor left, preview right ───────────────────────
        ui.columns(2, |cols| {
            // Left: raw markdown editor
            cols[0].label(RichText::new("MARKDOWN").small().color(Color32::GRAY));
            egui::ScrollArea::vertical()
                .id_salt(format!("editor_{note_id}"))
                .show(&mut cols[0], |ui| {
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut note.raw)
                            .font(FontId::monospace(font_size))
                            .desired_width(f32::INFINITY)
                            .desired_rows(50)
                            .frame(false)
                            .lock_focus(true),
                    );
                    if resp.changed() {
                        let new_raw = note.raw.clone();
                        note.update_raw(new_raw);
                        modified = true;
                    }
                });

            // Right: rendered preview
            cols[1].label(RichText::new("PREVIEW").small().color(Color32::GRAY));
            self.markdown_cache.render(&mut cols[1], &note_id, &note.body);
        });

        (modified, rename_request)
    }
}
