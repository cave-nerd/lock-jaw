use std::path::{Path, PathBuf};

use egui::{Color32, RichText, Ui};
use lj_core::Vault;

use crate::theme::parse_color;

pub struct Sidebar {
    pub search_query: String,
    pub new_note_name: String,
    pub show_new_note_input: bool,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            new_note_name: String::new(),
            show_new_note_input: false,
        }
    }
}

pub enum SidebarAction {
    OpenNote(PathBuf),
    CreateNote(String),
    DeleteNote(PathBuf),
}

impl Sidebar {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        vault: &Vault,
        active_path: Option<&Path>,
        accent_color: &str,
        muted_color: &str,
    ) -> Option<SidebarAction> {
        let mut action = None;
        let accent = parse_color(accent_color);
        let muted = parse_color(muted_color);

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("NOTES").small().color(muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+").on_hover_text("New note").clicked() {
                    self.show_new_note_input = !self.show_new_note_input;
                }
            });
        });

        if self.show_new_note_input {
            ui.add_space(4.0);
            let response = ui.text_edit_singleline(&mut self.new_note_name);
            response.request_focus();
            // Check Enter while focused (lost_focus + key_pressed don't fire in the same frame)
            let pressed_enter = response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if pressed_enter {
                if !self.new_note_name.trim().is_empty() {
                    action = Some(SidebarAction::CreateNote(self.new_note_name.trim().to_string()));
                    self.new_note_name.clear();
                    self.show_new_note_input = false;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_new_note_input = false;
                self.new_note_name.clear();
            }
        }

        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Search notes...")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(6.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let query = self.search_query.to_lowercase();
            for path in &vault.entries {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?");

                if !query.is_empty() && !stem.to_lowercase().contains(&query) {
                    continue;
                }

                let is_active = active_path.map(|p| p == path).unwrap_or(false);
                let label_text = if is_active {
                    RichText::new(stem).color(accent).strong()
                } else {
                    RichText::new(stem)
                };

                let resp = ui.selectable_label(is_active, label_text);
                resp.context_menu(|ui| {
                    if ui.button("Delete").clicked() {
                        action = Some(SidebarAction::DeleteNote(path.clone()));
                        ui.close_menu();
                    }
                });
                if resp.clicked() {
                    action = Some(SidebarAction::OpenNote(path.clone()));
                }
            }

            if vault.entries.is_empty() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("No notes yet.\nClick + to create one.")
                        .color(Color32::GRAY)
                        .small(),
                );
            }
        });

        action
    }
}
