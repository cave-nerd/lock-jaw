use std::path::PathBuf;

use egui::{FontId, RichText, Ui};
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
    /// Render the title bar + editor/preview pane(s).
    ///
    /// Returns `(content_modified, rename_request)`:
    /// - `content_modified` — note body text was edited this frame
    /// - `rename_request`   — `Some(new_name)` when the user commits a rename
    pub fn show(
        &mut self,
        ui: &mut Ui,
        note: &mut Note,
        font_size: f32,
        view_mode: &str,
    ) -> (bool, Option<String>) {
        let mut modified = false;
        let mut rename_request: Option<String> = None;

        let note_id = note.path.to_string_lossy().into_owned();
        let stem = note
            .path
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
        let pending = self.rename_text.trim() != stem && !self.rename_text.trim().is_empty();
        let title_color = if pending {
            ui.visuals().widgets.hovered.fg_stroke.color
        } else {
            ui.visuals().widgets.noninteractive.fg_stroke.color
        };

        ui.add_space(8.0);
        let title_resp = ui.add(
            egui::TextEdit::singleline(&mut self.rename_text)
                .font(FontId::proportional(font_size * 1.8))
                .desired_width(f32::INFINITY)
                .text_color(title_color)
                .frame(false)
                .hint_text("Note title…  (press Enter to rename)"),
        );

        let enter_pressed = title_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let escape_pressed =
            title_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
        // Commit on focus-lost too — matches expected "click away = confirm" UX.
        let commit = enter_pressed || (title_resp.lost_focus() && !escape_pressed);

        if escape_pressed {
            self.rename_text = stem.clone();
        } else if commit && !self.rename_text.trim().is_empty() && self.rename_text.trim() != stem {
            rename_request = Some(self.rename_text.trim().to_string());
        }

        // Subtle separator line under the title bar.
        ui.add_space(4.0);
        let sep_color = if pending {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke.color
        };
        let rect = ui.cursor();
        let line = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.top()),
            egui::vec2(ui.available_width(), 1.0),
        );
        ui.painter().rect_filled(line, 0.0, sep_color);
        ui.add_space(8.0);

        // Header pill label function.
        let pill_label = |u: &mut egui::Ui, text: &str| {
            let bg = u.visuals().widgets.noninteractive.bg_fill;
            egui::Frame::none()
                .fill(bg)
                .rounding(16.0)
                .inner_margin(egui::vec2(8.0, 4.0))
                .show(u, |u| {
                    u.label(RichText::new(text).small().strong());
                });
        };

        // ── Content area ────────────────────────────────────────────────
        match view_mode {
            "editor" => {
                pill_label(ui, "MARKDOWN");
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .id_salt(format!("editor_{note_id}"))
                    .show(ui, |ui| {
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
            }
            "preview" => {
                pill_label(ui, "PREVIEW");
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .id_salt(format!("preview_{note_id}"))
                    .show(ui, |ui| {
                        self.markdown_cache.render(ui, &note_id, &note.body);
                    });
            }
            _ => {
                // "both" — split view
                // SidePanel defaults to Left
                egui::SidePanel::left(format!("editor_split_{note_id}"))
                    .resizable(true)
                    .default_width(ui.available_width() * 0.5)
                    .frame(egui::Frame::none().inner_margin(egui::Margin {
                        left: 0.0,
                        right: 8.0,
                        top: 0.0,
                        bottom: 0.0,
                    }))
                    .show_inside(ui, |ui| {
                        pill_label(ui, "MARKDOWN");
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt(format!("editor_{note_id}"))
                            .show(ui, |ui| {
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
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().inner_margin(egui::Margin {
                        left: 8.0,
                        right: 0.0,
                        top: 0.0,
                        bottom: 0.0,
                    }))
                    .show_inside(ui, |ui| {
                        pill_label(ui, "PREVIEW");
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt(format!("preview_{note_id}"))
                            .show(ui, |ui| {
                                self.markdown_cache.render(ui, &note_id, &note.body);
                            });
                    });
            }
        }

        (modified, rename_request)
    }
}
