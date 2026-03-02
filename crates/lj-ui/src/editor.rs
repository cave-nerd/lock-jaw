use egui::{Color32, FontId, RichText, Ui};
use lj_core::Note;
use lj_md::MarkdownCache;

pub struct EditorPane {
    pub markdown_cache: MarkdownCache,
}

impl Default for EditorPane {
    fn default() -> Self {
        Self {
            markdown_cache: MarkdownCache::new(),
        }
    }
}

impl EditorPane {
    /// Render the split editor + preview pane. Returns true if the note was modified.
    pub fn show(&mut self, ui: &mut Ui, note: &mut Note, font_size: f32) -> bool {
        let mut modified = false;
        let note_id = note.path.to_string_lossy().into_owned();

        ui.columns(2, |cols| {
            // ── Left: raw markdown editor ──────────────────────────────
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

            // ── Right: rendered preview ─────────────────────────────────
            cols[1].label(RichText::new("PREVIEW").small().color(Color32::GRAY));
            self.markdown_cache.render(&mut cols[1], &note_id, &note.body);
        });

        modified
    }
}
