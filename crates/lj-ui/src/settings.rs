use std::path::PathBuf;

use egui::{Color32, RichText, Ui};
use lj_core::Config;

use crate::theme::{theme_accent, THEMES};

#[derive(PartialEq, Clone, Copy, Default)]
enum Tab {
    #[default]
    General,
    Editor,
    Appearance,
    Sync,
}

pub struct SettingsWindow {
    pub is_open: bool,
    draft: Config,
    /// Editable string for the vault path (PathBuf ↔ String bridge).
    vault_path_str: String,
    tab: Tab,
    webdav_pw_visible: bool,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self {
            is_open: false,
            draft: Config::default(),
            vault_path_str: String::new(),
            tab: Tab::default(),
            webdav_pw_visible: false,
        }
    }
}

impl SettingsWindow {
    /// Open the settings window, seeding the draft from the current config.
    pub fn open(&mut self, config: &Config) {
        self.draft = config.clone();
        self.vault_path_str = config.vault_path.to_string_lossy().into_owned();
        self.tab = Tab::General;
        self.webdav_pw_visible = false;
        self.is_open = true;
    }

    /// Render the settings window.
    /// Returns `Some(new_config)` when the user clicks **Save**, `None` otherwise.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Config> {
        if !self.is_open {
            return None;
        }

        let mut saved_config: Option<Config> = None;
        let mut close_requested = false;
        let mut window_open = self.is_open;

        egui::Window::new("Settings")
            .open(&mut window_open)
            .resizable(false)
            .collapsible(false)
            .default_size([520.0, 420.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // ── Tab bar ──────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::General,    "General");
                    ui.selectable_value(&mut self.tab, Tab::Editor,     "Editor");
                    ui.selectable_value(&mut self.tab, Tab::Appearance, "Appearance");
                    ui.selectable_value(&mut self.tab, Tab::Sync,       "Sync");
                });
                ui.separator();

                // ── Tab content ──────────────────────────────────────────
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| match self.tab {
                        Tab::General    => show_general(ui, &mut self.draft, &mut self.vault_path_str),
                        Tab::Editor     => show_editor(ui, &mut self.draft),
                        Tab::Appearance => show_appearance(ui, &mut self.draft),
                        Tab::Sync       => show_sync(ui, &mut self.draft, &mut self.webdav_pw_visible),
                    });

                ui.add_space(8.0);
                ui.separator();

                // ── Bottom buttons ───────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("  Save  ").clicked() {
                            self.draft.vault_path = PathBuf::from(&self.vault_path_str);
                            saved_config = Some(self.draft.clone());
                            close_requested = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        self.is_open = window_open && !close_requested;
        saved_config
    }
}

// ── Tab renderers (free functions keep things tidy) ───────────────────────────

fn show_general(ui: &mut Ui, _draft: &mut Config, vault_path_str: &mut String) {
    egui::Grid::new("general_grid")
        .num_columns(2)
        .spacing([16.0, 10.0])
        .min_col_width(120.0)
        .show(ui, |ui| {
            ui.label("Vault path:");
            ui.add(
                egui::TextEdit::singleline(vault_path_str)
                    .hint_text("/home/you/Documents/LockJaw")
                    .desired_width(300.0),
            );
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(
        RichText::new("Tip: Changes to vault path take effect after saving.")
            .small()
            .color(Color32::GRAY),
    );
}

fn show_editor(ui: &mut Ui, draft: &mut Config) {
    egui::Grid::new("editor_grid")
        .num_columns(2)
        .spacing([16.0, 12.0])
        .min_col_width(140.0)
        .show(ui, |ui| {
            // ── Font size ─────────────────────────────────────────────────
            ui.label("Font size:");
            ui.add(
                egui::Slider::new(&mut draft.font_size, 10.0..=28.0)
                    .suffix(" pt")
                    .step_by(0.5),
            );
            ui.end_row();

            // ── Heading scale ─────────────────────────────────────────────
            ui.label("Heading scale:");
            ui.add(
                egui::Slider::new(&mut draft.heading_scale, 1.2..=3.0)
                    .suffix("×")
                    .step_by(0.1),
            );
            ui.end_row();
        });

    ui.add_space(12.0);

    // Live preview of heading sizes
    let body = draft.font_size;
    let h1   = body * draft.heading_scale;
    let diff = h1 - body;

    ui.label(RichText::new("Preview (approximate):").small().color(Color32::GRAY));
    ui.add_space(4.0);

    egui::Frame::none()
        .inner_margin(egui::Margin::same(8.0))
        .fill(ui.visuals().faint_bg_color)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.label(RichText::new("# Heading 1").size(h1).strong());
            ui.label(RichText::new("## Heading 2").size(body + diff * 0.835).strong());
            ui.label(RichText::new("### Heading 3").size(body + diff * 0.668).strong());
            ui.label(RichText::new("#### Heading 4").size(body + diff * 0.501).strong());
            ui.label(RichText::new("Body text").size(body));
        });

    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Font size sets the base body size in both editor and preview.\n\
             Heading scale controls how large H1 is relative to body text;\n\
             sub-headings are interpolated between body and H1.",
        )
        .small()
        .color(Color32::GRAY),
    );
}

fn show_appearance(ui: &mut Ui, draft: &mut Config) {
    ui.label("Theme:");
    ui.add_space(6.0);

    for (key, display_name) in THEMES {
        let is_selected = draft.theme == *key;
        let accent = theme_accent(key);

        ui.horizontal(|ui| {
            // Color swatch dot
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 6.0, accent);

            let label = if is_selected {
                RichText::new(*display_name).strong()
            } else {
                RichText::new(*display_name)
            };

            if ui.selectable_label(is_selected, label).clicked() {
                draft.theme = key.to_string();
            }
        });
        ui.add_space(2.0);
    }
}

fn show_sync(ui: &mut Ui, draft: &mut Config, pw_visible: &mut bool) {
    ui.checkbox(&mut draft.webdav.enabled, "Enable WebDAV sync");
    ui.add_space(8.0);

    ui.add_enabled_ui(draft.webdav.enabled, |ui| {
        egui::Grid::new("sync_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .min_col_width(100.0)
            .show(ui, |ui| {
                ui.label("Server URL:");
                ui.add(
                    egui::TextEdit::singleline(&mut draft.webdav.url)
                        .hint_text("https://cloud.example.com/dav/notes/")
                        .desired_width(280.0),
                );
                ui.end_row();

                ui.label("Username:");
                ui.add(
                    egui::TextEdit::singleline(&mut draft.webdav.username)
                        .desired_width(200.0),
                );
                ui.end_row();

                ui.label("Password:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.webdav.password)
                            .password(!*pw_visible)
                            .desired_width(180.0),
                    );
                    let eye = if *pw_visible { "Hide" } else { "Show" };
                    if ui.small_button(eye).clicked() {
                        *pw_visible = !*pw_visible;
                    }
                });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Tip: point the URL at the directory that should contain your .md files.\n\
                 Supports Nextcloud, ownCloud, Apache WebDAV, and most WebDAV servers.",
            )
            .small()
            .color(Color32::GRAY),
        );
    });
}
