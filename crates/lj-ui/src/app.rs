use std::path::PathBuf;

use lj_core::{Config, Note, Vault};
use lj_plugin::PluginHost;
use tracing::{error, info};

use crate::{
    editor::EditorPane,
    sidebar::{Sidebar, SidebarAction},
    theme::Theme,
};

pub struct LockJawApp {
    config: Config,
    theme: Theme,
    vault: Option<Vault>,
    open_note: Option<Note>,
    sidebar: Sidebar,
    editor: EditorPane,
    plugin_host: PluginHost,
    status_message: Option<String>,
}

impl LockJawApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let theme = load_theme(&config);
        theme.apply(&cc.egui_ctx);

        // Set a slightly larger default font
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(config.font_size),
        );
        cc.egui_ctx.set_style(style);

        let vault = open_vault(&config);

        let mut plugin_host = PluginHost::new();
        if let Some(plugins_dir) = Config::plugins_dir() {
            if let Err(e) = plugin_host.load_from_dir(&plugins_dir) {
                error!("Plugin load error: {e}");
            }
        }

        Self {
            config,
            theme,
            vault,
            open_note: None,
            sidebar: Sidebar::default(),
            editor: EditorPane::default(),
            plugin_host,
            status_message: None,
        }
    }

    fn save_current_note(&mut self) {
        if let Some(note) = self.open_note.as_mut() {
            // Let plugins transform content before save
            let transformed = self
                .plugin_host
                .on_note_save(note.path.to_str().unwrap_or(""), note.raw.clone());
            note.raw = transformed;

            match note.save() {
                Ok(_) => {
                    self.status_message = Some(format!("Saved: {}", note.display_name()));
                    info!("Saved note: {}", note.path.display());
                }
                Err(e) => {
                    self.status_message = Some(format!("Error saving: {e}"));
                    error!("Save error: {e}");
                }
            }
        }
    }

    fn open_note(&mut self, path: PathBuf) {
        // Save current note first
        if self.open_note.as_ref().map(|n| n.dirty).unwrap_or(false) {
            self.save_current_note();
        }
        match Note::load(&path) {
            Ok(note) => {
                self.plugin_host
                    .on_note_open(path.to_str().unwrap_or(""));
                self.open_note = Some(note);
            }
            Err(e) => {
                self.status_message = Some(format!("Error opening note: {e}"));
            }
        }
    }
}

impl eframe::App for LockJawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Global keyboard shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_current_note();
        }

        let accent = self.theme.colors.accent.clone();
        let muted = self.theme.colors.fg_muted.clone();
        let code_bg = self.theme.colors.bg_code.clone();
        let font_size = self.theme.editor.font_size;

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Note").clicked() {
                        self.sidebar.show_new_note_input = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Save  Ctrl+S").clicked() {
                        self.save_current_note();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Dark Theme").clicked() {
                        self.theme = Theme::dark();
                        self.theme.apply(ctx);
                        ui.close_menu();
                    }
                    if ui.button("Light Theme").clicked() {
                        self.theme = Theme::light();
                        self.theme.apply(ctx);
                        ui.close_menu();
                    }
                });

                // Plugin commands
                let commands = self.plugin_host.commands();
                if !commands.is_empty() {
                    ui.menu_button("Plugins", |ui| {
                        for cmd in &commands {
                            if ui.button(&cmd.name).clicked() {
                                // TODO(Phase 2): dispatch command
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(note) = &self.open_note {
                    let word_count = lj_md::word_count(&note.body);
                    ui.label(format!("{}", note.path.display()));
                    ui.separator();
                    ui.label(format!("{word_count} words"));
                    if note.dirty {
                        ui.separator();
                        ui.label(
                            egui::RichText::new("unsaved")
                                .color(crate::theme::parse_color(&self.theme.colors.warning)),
                        );
                    }
                } else if let Some(ref vault) = self.vault {
                    ui.label(format!("Vault: {}", vault.root.display()));
                } else {
                    ui.label("No vault open");
                }

                if let Some(msg) = &self.status_message {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(msg);
                    });
                }

                // Plugin count
                if !self.plugin_host.plugins.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "{} plugin(s)",
                            self.plugin_host.plugins.len()
                        ));
                    });
                }
            });
        });

        // Sidebar
        let sidebar_action = egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                if let Some(vault) = &self.vault {
                    let active_path = self.open_note.as_ref().map(|n| n.path.as_path());
                    self.sidebar.show(ui, vault, active_path, &accent, &muted)
                } else {
                    // No vault — show open vault prompt
                    ui.centered_and_justified(|ui| {
                        ui.label("No vault open.\nEdit config.toml to set vault_path.");
                    });
                    None
                }
            })
            .inner;

        // Handle sidebar actions
        match sidebar_action {
            Some(SidebarAction::OpenNote(path)) => {
                self.open_note(path);
                // clear status after navigating
                self.status_message = None;
            }
            Some(SidebarAction::CreateNote(name)) => {
                if let Some(vault) = self.vault.as_mut() {
                    match vault.create_note(&name) {
                        Ok(note) => {
                            let path = note.path.clone();
                            self.open_note = Some(note);
                            self.plugin_host.on_note_open(path.to_str().unwrap_or(""));
                            self.status_message = Some(format!("Created: {name}"));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error creating note: {e}"));
                        }
                    }
                }
            }
            Some(SidebarAction::DeleteNote(path)) => {
                if let Some(vault) = self.vault.as_mut() {
                    if let Err(e) = vault.delete_note(&path) {
                        self.status_message = Some(format!("Error deleting: {e}"));
                    } else {
                        if self.open_note.as_ref().map(|n| &n.path == &path).unwrap_or(false) {
                            self.open_note = None;
                        }
                        self.status_message = Some("Note deleted.".to_string());
                    }
                }
            }
            None => {}
        }

        // Central editor panel
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(note) = self.open_note.as_mut() {
                self.editor.show(ui, note, font_size, &code_bg);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Lock Jaw\n\nSelect or create a note to get started.")
                            .size(18.0)
                            .color(crate::theme::parse_color(&muted)),
                    );
                });
            }
        });
    }
}

fn load_theme(config: &Config) -> Theme {
    // Try user theme dir first
    if let Some(themes_dir) = Config::themes_dir() {
        let path = themes_dir.join(format!("{}.toml", config.theme));
        if path.exists() {
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(t) = toml::from_str::<Theme>(&s) {
                    return t;
                }
            }
        }
    }
    // Fall back to bundled themes
    if config.theme == "light" {
        Theme::light()
    } else {
        Theme::dark()
    }
}

fn open_vault(config: &Config) -> Option<Vault> {
    let path = &config.vault_path;
    if !path.exists() {
        if let Err(e) = std::fs::create_dir_all(path) {
            error!("Could not create vault dir: {e}");
            return None;
        }
        info!("Created vault directory: {}", path.display());
    }
    match Vault::open(path) {
        Ok(v) => {
            info!("Opened vault: {}", path.display());
            Some(v)
        }
        Err(e) => {
            error!("Failed to open vault: {e}");
            None
        }
    }
}
