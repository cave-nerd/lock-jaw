use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Color32, RichText, Ui};
use lj_core::Vault;

use crate::theme::parse_color;

pub struct Sidebar {
    pub search_query: String,
    /// Shared text buffer for all new-note / new-folder name inputs.
    pub new_note_name: String,
    /// Whether the new-note input is visible.
    pub show_new_note_input: bool,
    /// `None` = create at vault root, `Some(rel_path)` = create inside that folder.
    new_note_folder: Option<PathBuf>,
    /// Whether the new-section (folder) input is visible.
    show_new_folder_input: bool,
    new_folder_name: String,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            new_note_name: String::new(),
            show_new_note_input: false,
            new_note_folder: None,
            show_new_folder_input: false,
            new_folder_name: String::new(),
        }
    }
}

pub enum SidebarAction {
    OpenNote(PathBuf),
    /// Create a note at the vault root.
    CreateNote(String),
    /// Create a note inside `folder` (relative to vault root).
    CreateNoteIn(PathBuf, String),
    /// Create a new section (sub-directory) at the vault root.
    CreateFolder(String),
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
        let mut action: Option<SidebarAction> = None;
        let accent = parse_color(accent_color);
        let muted  = parse_color(muted_color);

        // ── Header row ──────────────────────────────────────────────────
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("NOTES").small().color(muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⊕")
                    .on_hover_text("New section")
                    .clicked()
                {
                    self.show_new_folder_input = !self.show_new_folder_input;
                    self.show_new_note_input = false;
                }
                if ui.small_button("+")
                    .on_hover_text("New note")
                    .clicked()
                {
                    self.show_new_note_input = !self.show_new_note_input;
                    self.new_note_folder = None;
                    self.show_new_folder_input = false;
                    self.new_note_name.clear();
                }
            });
        });

        // ── New-folder input ────────────────────────────────────────────
        if self.show_new_folder_input {
            ui.add_space(4.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.new_folder_name)
                    .hint_text("Section name…")
                    .desired_width(f32::INFINITY),
            );
            resp.request_focus();
            if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let name = self.new_folder_name.trim().to_string();
                if !name.is_empty() {
                    action = Some(SidebarAction::CreateFolder(name));
                    self.new_folder_name.clear();
                    self.show_new_folder_input = false;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_new_folder_input = false;
                self.new_folder_name.clear();
            }
        }

        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Search notes…")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(6.0);

        // ── Build display data before entering closures ─────────────────
        let query = self.search_query.to_lowercase();
        let searching = !query.is_empty();

        let filtered: Vec<PathBuf> = vault.entries.iter()
            .filter(|p| {
                if searching {
                    p.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Group notes by folder (relative to vault root).
        // When searching, show flat (no grouping).
        let mut root_notes: Vec<PathBuf> = Vec::new();
        let mut folder_map: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();

        if searching {
            root_notes = filtered;
        } else {
            for path in &filtered {
                let rel = path.strip_prefix(&vault.root).unwrap_or(path);
                match rel.parent() {
                    None => root_notes.push(path.clone()),
                    Some(p) if p == Path::new("") => root_notes.push(path.clone()),
                    Some(parent) => {
                        folder_map
                            .entry(parent.to_path_buf())
                            .or_default()
                            .push(path.clone());
                    }
                }
            }
        }

        let folder_groups: Vec<(PathBuf, Vec<PathBuf>)> = folder_map.into_iter().collect();

        // ── Scroll area ─────────────────────────────────────────────────
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Root-level notes
            for path in &root_notes {
                note_entry(ui, path, active_path, accent, &mut action);
            }

            // Root new-note input (None folder)
            if self.show_new_note_input && self.new_note_folder.is_none() {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.new_note_name)
                        .hint_text("Note name…")
                        .desired_width(f32::INFINITY),
                );
                resp.request_focus();
                if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let name = self.new_note_name.trim().to_string();
                    if !name.is_empty() {
                        action = Some(SidebarAction::CreateNote(name));
                        self.new_note_name.clear();
                        self.show_new_note_input = false;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.show_new_note_input = false;
                    self.new_note_name.clear();
                }
            }

            if root_notes.is_empty() && folder_groups.is_empty() && !searching {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("No notes yet.\nClick + to create one.")
                        .color(Color32::GRAY)
                        .small(),
                );
            }

            // ── Folder sections ─────────────────────────────────────────
            for (folder_rel, folder_notes) in &folder_groups {
                let folder_name = folder_rel
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();

                ui.add_space(4.0);

                // The body closure captures only local data — no `self` needed.
                let cr = egui::CollapsingHeader::new(
                    RichText::new(&folder_name).strong(),
                )
                .default_open(true)
                .id_salt(folder_rel.display().to_string())
                .show(ui, |ui| -> (Option<SidebarAction>, bool) {
                    let mut inner: Option<SidebarAction> = None;
                    for path in folder_notes {
                        note_entry(ui, path, active_path, accent, &mut inner);
                    }
                    ui.add_space(2.0);
                    let plus = ui
                        .small_button("+ New note")
                        .on_hover_text("New note in this section")
                        .clicked();
                    (inner, plus)
                });

                // Process body return value — `self` is fine to use here (flat code).
                if let Some((inner_action, plus_clicked)) = cr.body_returned {
                    if inner_action.is_some() {
                        action = inner_action;
                    }
                    if plus_clicked {
                        self.show_new_note_input = true;
                        self.new_note_folder = Some(folder_rel.clone());
                        self.new_note_name.clear();
                        self.show_new_folder_input = false;
                    }
                }

                // Per-folder new-note input — rendered right after the body.
                if self.show_new_note_input
                    && self.new_note_folder.as_deref() == Some(folder_rel.as_path())
                {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.new_note_name)
                            .hint_text("Note name…")
                            .desired_width(f32::INFINITY),
                    );
                    resp.request_focus();
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let name = self.new_note_name.trim().to_string();
                        if !name.is_empty() {
                            action = Some(SidebarAction::CreateNoteIn(
                                folder_rel.clone(),
                                name,
                            ));
                            self.new_note_name.clear();
                            self.show_new_note_input = false;
                            self.new_note_folder = None;
                        }
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.show_new_note_input = false;
                        self.new_note_folder = None;
                        self.new_note_name.clear();
                    }
                }
            }
        });

        action
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Render a single note entry with click + right-click delete.
fn note_entry(
    ui: &mut Ui,
    path: &PathBuf,
    active_path: Option<&Path>,
    accent: egui::Color32,
    action: &mut Option<SidebarAction>,
) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("?");

    let is_active = active_path.map(|p| p == path.as_path()).unwrap_or(false);
    let label_text = if is_active {
        RichText::new(stem).color(accent).strong()
    } else {
        RichText::new(stem)
    };

    let resp = ui.selectable_label(is_active, label_text);
    resp.context_menu(|ui| {
        if ui.button("Delete").clicked() {
            *action = Some(SidebarAction::DeleteNote(path.clone()));
            ui.close_menu();
        }
    });
    if resp.clicked() {
        *action = Some(SidebarAction::OpenNote(path.clone()));
    }
}
