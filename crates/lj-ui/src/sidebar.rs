use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use egui::{Color32, RichText, Ui};
use lj_core::Vault;

use crate::theme::parse_color;

pub struct Sidebar {
    pub search_query: String,
    /// Shared text buffer for all new-note name inputs.
    pub new_note_name: String,
    /// Whether the new-note input is visible.
    pub show_new_note_input: bool,
    /// `None` = create at vault root, `Some(rel_path)` = create inside that folder.
    new_note_folder: Option<PathBuf>,
    /// Whether the new-section (folder) input is visible.
    show_new_folder_input: bool,
    new_folder_name: String,
    /// Which folder (relative path) is currently being renamed, if any.
    renaming_folder: Option<PathBuf>,
    rename_folder_buf: String,
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
            renaming_folder: None,
            rename_folder_buf: String::new(),
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
    /// Move a note to a folder (relative to vault root), or `None` = vault root.
    MoveNote(PathBuf, Option<PathBuf>),
    /// Rename a section (folder). `old_rel` is relative to vault root.
    RenameFolder(PathBuf, String),
    /// Delete a section and all notes inside it.
    DeleteFolder(PathBuf),
}

impl Sidebar {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        vault: &Vault,
        active_path: Option<&Path>,
        accent_color: &str,
        muted_color: &str,
        icon_style: &str,
    ) -> Option<SidebarAction> {
        let mut action: Option<SidebarAction> = None;
        let accent = parse_color(accent_color);
        let muted = parse_color(muted_color);
        let note_pfx = note_prefix(icon_style);
        let folder_pfx = folder_prefix(icon_style);

        // Snapshot folder list for use inside closures (shared ref, no cost).
        let vault_folders: &[PathBuf] = &vault.folders;

        // ── Header row ──────────────────────────────────────────────────
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("NOTES").small().color(muted));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⊕").on_hover_text("New section").clicked() {
                    self.show_new_folder_input = !self.show_new_folder_input;
                    self.show_new_note_input = false;
                }
                if ui.small_button("+").on_hover_text("New note").clicked() {
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
        let search_frame = egui::Frame::none()
            .fill(ui.visuals().widgets.noninteractive.bg_fill)
            .rounding(6.0)
            .inner_margin(egui::vec2(8.0, 6.0));

        search_frame.show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍  Search notes…")
                    .frame(false)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.add_space(8.0);

        // ── Build display data before entering closures ─────────────────
        let query = self.search_query.to_lowercase();
        let searching = !query.is_empty();

        let filtered: Vec<PathBuf> = vault
            .entries
            .iter()
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

        let mut root_notes: Vec<PathBuf> = Vec::new();
        let mut folder_map: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();

        if searching {
            root_notes = filtered;
        } else {
            for path in &filtered {
                let rel = path.strip_prefix(&vault.root).unwrap_or(path);
                let parent = rel.parent();
                if parent.is_none() || parent == Some(Path::new("")) {
                    root_notes.push(path.clone());
                } else if let Some(p) = parent {
                    folder_map
                        .entry(p.to_path_buf())
                        .or_default()
                        .push(path.clone());
                }
            }
        }

        // Merge vault.folders (includes empty dirs) with folder_map keys.
        let all_folder_keys: BTreeSet<PathBuf> = if searching {
            BTreeSet::new()
        } else {
            vault_folders
                .iter()
                .cloned()
                .chain(folder_map.keys().cloned())
                .collect()
        };

        // Convert to an owned Vec so the scroll closure doesn't need to borrow
        // folder_map and all_folder_keys simultaneously.
        let folder_sections: Vec<(PathBuf, Vec<PathBuf>)> = all_folder_keys
            .into_iter()
            .map(|f| {
                let notes = folder_map.remove(&f).unwrap_or_default();
                (f, notes)
            })
            .collect();

        // ── Scroll area ─────────────────────────────────────────────────
        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Root notes drop zone ─────────────────────────────────────
            let (_, root_drop) = ui.dnd_drop_zone::<PathBuf, _>(egui::Frame::none(), |ui| {
                for path in &root_notes {
                    note_entry(
                        ui,
                        path,
                        active_path,
                        accent,
                        note_pfx,
                        muted,
                        vault_folders,
                        &mut action,
                    );
                }

                // Root new-note input (no folder targeted).
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
            });
            if let Some(dropped) = root_drop {
                action = Some(SidebarAction::MoveNote((*dropped).clone(), None));
            }

            if root_notes.is_empty() && folder_sections.is_empty() && !searching {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("No notes yet.\nClick + to create one.")
                        .color(Color32::GRAY)
                        .small(),
                );
            }

            // ── Folder sections (each is a DnD drop zone) ─────────────
            for (folder_rel, folder_notes) in &folder_sections {
                let folder_name = folder_rel
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();

                ui.add_space(8.0);

                // ── Inline rename input ──────────────────────────────────
                if self.renaming_folder.as_deref() == Some(folder_rel.as_path()) {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_folder_buf)
                            .desired_width(f32::INFINITY),
                    );
                    resp.request_focus();
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let new_name = self.rename_folder_buf.trim().to_string();
                        if !new_name.is_empty() {
                            action = Some(SidebarAction::RenameFolder(
                                folder_rel.clone(),
                                new_name,
                            ));
                        }
                        self.renaming_folder = None;
                        self.rename_folder_buf.clear();
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.renaming_folder = None;
                        self.rename_folder_buf.clear();
                    }
                    continue;
                }

                // ── Normal folder header ─────────────────────────────────
                let header_label = if folder_pfx.is_empty() {
                    RichText::new(&folder_name).strong()
                } else {
                    RichText::new(format!("{folder_pfx}{folder_name}")).strong()
                };

                let (dnd_inner, folder_drop) =
                    ui.dnd_drop_zone::<PathBuf, _>(egui::Frame::none(), |ui| {
                        egui::CollapsingHeader::new(header_label)
                            .default_open(true)
                            .id_salt(folder_rel.display().to_string())
                            .show(ui, |ui| -> (Option<SidebarAction>, bool) {
                                let mut inner: Option<SidebarAction> = None;

                                if folder_notes.is_empty() {
                                    ui.label(
                                        RichText::new("No notes yet — click + to add one.")
                                            .small()
                                            .color(Color32::GRAY),
                                    );
                                } else {
                                    for path in folder_notes {
                                        note_entry(
                                            ui,
                                            path,
                                            active_path,
                                            accent,
                                            note_pfx,
                                            muted,
                                            vault_folders,
                                            &mut inner,
                                        );
                                    }
                                }

                                ui.add_space(2.0);
                                let plus = ui
                                    .small_button("+ New note")
                                    .on_hover_text("New note in this section")
                                    .clicked();
                                (inner, plus)
                            })
                    });

                // Right-click context menu on the section header.
                dnd_inner.inner.header_response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        self.renaming_folder = Some(folder_rel.clone());
                        self.rename_folder_buf = folder_name.clone();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(RichText::new("Delete").color(Color32::from_rgb(220, 38, 38))).clicked() {
                        action = Some(SidebarAction::DeleteFolder(folder_rel.clone()));
                        ui.close_menu();
                    }
                });

                // Process CollapsingHeader body return.
                if let Some(body_returned) = dnd_inner.inner.body_returned {
                    let (inner_action, plus_clicked) = body_returned;
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

                // Handle drag-and-drop onto this folder.
                if let Some(dropped) = folder_drop {
                    action = Some(SidebarAction::MoveNote(
                        (*dropped).clone(),
                        Some(folder_rel.clone()),
                    ));
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
                            action = Some(SidebarAction::CreateNoteIn(folder_rel.clone(), name));
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

// ── Icon helpers ──────────────────────────────────────────────────────────────

fn note_prefix(style: &str) -> &'static str {
    match style {
        "emoji" => "📝 ",
        "ascii" => "- ",
        "bullets" => "• ",
        "arrows" => "→ ",
        "none" => "",
        _ => "◦ ", // "unicode" (default)
    }
}

fn folder_prefix(style: &str) -> &'static str {
    match style {
        "emoji" => "📁 ",
        _ => "", // CollapsingHeader's ▸/▾ triangle is enough for other styles
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Render a single note entry with drag support, click-to-open, and a
/// right-click context menu with Delete and "Move to…" options.
fn note_entry(
    ui: &mut Ui,
    path: &PathBuf,
    active_path: Option<&Path>,
    accent: egui::Color32,
    icon_prefix: &str,
    muted: egui::Color32,
    folders: &[PathBuf],
    action: &mut Option<SidebarAction>,
) {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");

    let is_active = active_path.map(|p| p == path.as_path()).unwrap_or(false);

    let display = if icon_prefix.is_empty() {
        stem.to_string()
    } else {
        format!("{icon_prefix}{stem}")
    };

    // Single widget with Sense::click_and_drag so both click-to-open and DnD work.
    // selectable_label only has Sense::click, which is why the old dual-response
    // approach got stuck: the drag overlay won pointer-down events and swallowed clicks.
    let padding = ui.spacing().button_padding;
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let row_h = ui.text_style_height(&egui::TextStyle::Button) + padding.y * 2.0;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_h),
        egui::Sense::click_and_drag(),
    );

    if ui.is_rect_visible(rect) {
        let bg = if is_active {
            ui.visuals().selection.bg_fill
        } else if resp.hovered() {
            ui.visuals().widgets.hovered.weak_bg_fill
        } else {
            Color32::TRANSPARENT
        };
        if bg != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, 6.0, bg);
        }
        let text_color = if is_active {
            accent
        } else if resp.hovered() {
            ui.visuals().text_color()
        } else {
            muted
        };
        let galley = ui.fonts(|f| f.layout_no_wrap(display, font_id, text_color));
        let text_pos = egui::pos2(
            rect.left() + padding.x,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter_at(rect).galley(text_pos, galley, text_color);
    }

    resp.dnd_set_drag_payload(path.clone());

    // Context menu: Move to… and Delete.
    let path_clone = path.clone();
    resp.context_menu(|ui| {
        // Move to… sub-menu (only shown when there are destinations).
        if !folders.is_empty() {
            ui.menu_button("Move to…", |ui| {
                if ui.button("(Root)").clicked() {
                    *action = Some(SidebarAction::MoveNote(path_clone.clone(), None));
                    ui.close_menu();
                }
                for folder in folders {
                    let name = folder.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                    if ui.button(name).clicked() {
                        *action = Some(SidebarAction::MoveNote(
                            path_clone.clone(),
                            Some(folder.clone()),
                        ));
                        ui.close_menu();
                    }
                }
            });
            ui.separator();
        }
        if ui.button("Delete").clicked() {
            *action = Some(SidebarAction::DeleteNote(path_clone));
            ui.close_menu();
        }
    });

    if resp.clicked() {
        *action = Some(SidebarAction::OpenNote(path.clone()));
    }
}
