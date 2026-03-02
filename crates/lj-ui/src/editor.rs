use egui::{Color32, FontId, RichText, Ui};
use lj_core::Note;
use lj_md::MarkdownCache;

use crate::theme::parse_color;

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
    pub fn show(
        &mut self,
        ui: &mut Ui,
        note: &mut Note,
        font_size: f32,
        code_bg: &str,
    ) -> bool {
        let mut modified = false;
        let available = ui.available_size();
        let half_width = (available.x - 8.0) / 2.0;

        ui.horizontal(|ui| {
            // --- Editor (left) ---
            ui.allocate_ui(egui::vec2(half_width, available.y), |ui| {
                egui::Frame::none()
                    .fill(parse_color(code_bg))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("MARKDOWN").small().color(Color32::GRAY),
                        );
                        ui.add_space(4.0);
                        egui::ScrollArea::both()
                            .id_salt("editor_scroll")
                            .show(ui, |ui| {
                                let resp = ui.add(
                                    egui::TextEdit::multiline(&mut note.raw)
                                        .font(FontId::monospace(font_size))
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(40)
                                        .frame(false),
                                );
                                if resp.changed() {
                                    // Re-parse frontmatter
                                    let new_raw = note.raw.clone();
                                    note.update_raw(new_raw);
                                    modified = true;
                                }
                            });
                    });
            });

            ui.add_space(8.0);

            // --- Preview (right) — show_scrollable handles its own scroll area ---
            ui.allocate_ui(egui::vec2(half_width, available.y), |ui| {
                ui.add_space(2.0);
                ui.label(
                    RichText::new("PREVIEW").small().color(Color32::GRAY),
                );
                ui.add_space(4.0);
                let note_id = note.path.to_string_lossy().into_owned();
                self.markdown_cache.render(ui, &note_id, &note.body);
            });
        });

        modified
    }
}
