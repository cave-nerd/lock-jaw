use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use lj_core::{webdav, Config, Note, Vault};
use lj_plugin::PluginHost;
use tracing::{error, info};

use crate::{
    editor::EditorPane,
    settings::SettingsWindow,
    sidebar::{Sidebar, SidebarAction},
    theme::{load_theme_by_name, resolve_system_theme, Theme},
};

pub struct LockJawApp {
    config: Config,
    theme: Theme,
    vault: Option<Vault>,
    open_note: Option<Note>,
    sidebar: Sidebar,
    editor: EditorPane,
    plugin_host: PluginHost,
    settings: SettingsWindow,
    status_message: Option<String>,
    // WebDAV sync state
    sync_rx: Option<Receiver<anyhow::Result<webdav::SyncStats>>>,
    is_syncing: bool,
    // System theme polling (check every ~300 frames ≈ 5 s at 60 fps)
    system_theme_counter: u32,
}

impl LockJawApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let theme = load_theme_by_name(&config.theme);
        theme.apply(&cc.egui_ctx);
        apply_font_size(&cc.egui_ctx, config.font_size);

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
            settings: SettingsWindow::default(),
            status_message: None,
            sync_rx: None,
            is_syncing: false,
            system_theme_counter: 0,
        }
    }

    fn save_current_note(&mut self) {
        if let Some(note) = self.open_note.as_mut() {
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
        if self.open_note.as_ref().map(|n| n.dirty).unwrap_or(false) {
            self.save_current_note();
        }
        match Note::load(&path) {
            Ok(note) => {
                self.plugin_host.on_note_open(path.to_str().unwrap_or(""));
                self.open_note = Some(note);
            }
            Err(e) => {
                self.status_message = Some(format!("Error opening note: {e}"));
            }
        }
    }

    fn start_sync(&mut self) {
        if self.is_syncing {
            return;
        }
        if !self.config.webdav.enabled {
            self.status_message = Some(
                "WebDAV disabled. Open Settings → Sync to configure.".to_string(),
            );
            return;
        }
        if self.config.webdav.url.is_empty() {
            self.status_message = Some("WebDAV URL not set. Open Settings → Sync.".to_string());
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.sync_rx = Some(rx);
        self.is_syncing = true;

        let webdav_config = self.config.webdav.clone();
        let vault_root = self.vault.as_ref().map(|v| v.root.clone());

        std::thread::spawn(move || {
            let result = webdav::WebDavClient::new(webdav_config).and_then(|client| {
                vault_root
                    .map(|root| client.sync(&root))
                    .unwrap_or_else(|| anyhow::bail!("No vault open"))
            });
            tx.send(result).ok();
        });
    }

    fn poll_sync(&mut self) {
        let done = self.sync_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(result) = done {
            self.is_syncing = false;
            self.sync_rx = None;
            match result {
                Ok(stats) => {
                    if let Some(vault) = self.vault.as_mut() {
                        vault.refresh().ok();
                    }
                    let errs = if stats.errors.is_empty() {
                        String::new()
                    } else {
                        format!(", {} error(s)", stats.errors.len())
                    };
                    self.status_message = Some(format!(
                        "Sync done: ↑{} ↓{}{errs}",
                        stats.uploaded, stats.downloaded
                    ));
                }
                Err(e) => {
                    self.status_message = Some(format!("Sync failed: {e}"));
                }
            }
        }
    }

    /// Poll OS dark/light preference and switch if it changed (system theme only).
    fn poll_system_theme(&mut self, ctx: &egui::Context) {
        if self.config.theme != "system" {
            return;
        }
        self.system_theme_counter = self.system_theme_counter.wrapping_add(1);
        if self.system_theme_counter % 300 != 0 {
            return;
        }
        let new = resolve_system_theme();
        if new.meta.dark != self.theme.meta.dark {
            self.theme = new;
            self.theme.apply(ctx);
        }
    }

    /// Apply a saved config: reload theme, font, vault if needed, then persist.
    fn apply_config(&mut self, new_config: Config, ctx: &egui::Context) {
        // Theme
        if new_config.theme != self.config.theme {
            self.theme = load_theme_by_name(&new_config.theme);
            self.theme.apply(ctx);
            self.system_theme_counter = 0;
        }
        // Font size
        if (new_config.font_size - self.config.font_size).abs() > 0.1 {
            apply_font_size(ctx, new_config.font_size);
        }
        // Vault path
        if new_config.vault_path != self.config.vault_path {
            if self.open_note.as_ref().map(|n| n.dirty).unwrap_or(false) {
                self.save_current_note();
            }
            self.open_note = None;
            self.vault = open_vault(&new_config);
        }

        self.config = new_config;
        if let Err(e) = self.config.save() {
            self.status_message = Some(format!("Could not save config: {e}"));
        }
    }
}

impl eframe::App for LockJawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_sync();
        self.poll_system_theme(ctx);

        // Settings window (floating, before panels so it renders on top)
        if let Some(new_config) = self.settings.show(ctx) {
            self.apply_config(new_config, ctx);
            self.status_message = Some("Settings saved.".to_string());
        }

        // Ctrl+S
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_current_note();
        }

        let accent = self.theme.colors.accent.clone();
        let muted  = self.theme.colors.fg_muted.clone();
        let font_size = self.theme.editor.font_size;

        // ── Menu bar ────────────────────────────────────────────────────
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

                if ui.button("Settings").clicked() {
                    self.settings.open(&self.config);
                }

                // Sync button
                let sync_label = if self.is_syncing { "⟳ Syncing…" } else { "⟳ Sync" };
                if ui
                    .add_enabled(!self.is_syncing, egui::Button::new(sync_label))
                    .on_hover_text("Sync notes with WebDAV server")
                    .clicked()
                {
                    self.start_sync();
                }
                if self.is_syncing {
                    ctx.request_repaint();
                }

                let commands = self.plugin_host.commands();
                if !commands.is_empty() {
                    ui.menu_button("Plugins", |ui| {
                        for cmd in &commands {
                            if ui.button(&cmd.name).clicked() {
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
        });

        // ── Status bar ──────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(note) = &self.open_note {
                    let wc = lj_md::word_count(&note.body);
                    ui.label(format!("{}", note.path.display()));
                    ui.separator();
                    ui.label(format!("{wc} words"));
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.plugin_host.plugins.is_empty() {
                        ui.label(format!("{} plugin(s)", self.plugin_host.plugins.len()));
                        ui.separator();
                    }
                    // Theme indicator
                    let theme_name = crate::theme::THEMES
                        .iter()
                        .find(|(k, _)| *k == self.config.theme)
                        .map(|(_, n)| *n)
                        .unwrap_or(&self.config.theme);
                    ui.label(
                        egui::RichText::new(theme_name)
                            .small()
                            .color(crate::theme::parse_color(&muted)),
                    );
                    ui.separator();
                    if let Some(msg) = &self.status_message {
                        ui.label(msg);
                    }
                });
            });
        });

        // ── Sidebar ─────────────────────────────────────────────────────
        let sidebar_action = egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                if let Some(vault) = &self.vault {
                    let active_path = self.open_note.as_ref().map(|n| n.path.as_path());
                    self.sidebar.show(ui, vault, active_path, &accent, &muted)
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            "No vault open.\nOpen Settings → General to set a vault path.",
                        );
                    });
                    None
                }
            })
            .inner;

        match sidebar_action {
            Some(SidebarAction::OpenNote(path)) => {
                self.open_note(path);
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
                        if self.open_note.as_ref().map(|n| n.path == path).unwrap_or(false) {
                            self.open_note = None;
                        }
                        self.status_message = Some("Note deleted.".to_string());
                    }
                }
            }
            None => {}
        }

        // ── Central editor panel ─────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(note) = self.open_note.as_mut() {
                self.editor.show(ui, note, font_size);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "Lock Jaw\n\nSelect or create a note to get started.",
                        )
                        .size(18.0)
                        .color(crate::theme::parse_color(&muted)),
                    );
                });
            }
        });
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

fn apply_font_size(ctx: &egui::Context, size: f32) {
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(size),
    );
    ctx.set_style(style);
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
