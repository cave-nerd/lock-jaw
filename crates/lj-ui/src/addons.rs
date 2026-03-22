use egui::{Color32, RichText, Ui};
use lj_core::Config;
use lj_plugin::{LoadedPlugin, PluginHost};

/// Action requested by the Addons window that the app must handle.
pub enum AddonsAction {
    /// Re-scan the plugins directory.
    Reload,
    /// Delete a plugin and remove it from the host.
    Remove(String),
    /// Toggle a plugin's enabled state and save config.
    Toggle(String),
}

/// A persistent window for browsing and managing installed addons.
pub struct AddonsWindow {
    pub open: bool,
    /// Plugin name waiting for remove confirmation. `None` = no pending confirm.
    confirm_remove: Option<String>,
}

impl Default for AddonsWindow {
    fn default() -> Self {
        Self {
            open: false,
            confirm_remove: None,
        }
    }
}

impl AddonsWindow {
    /// Render the addons window. Returns an action if the user triggered one.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        plugin_host: &PluginHost,
        config: &Config,
    ) -> Option<AddonsAction> {
        if !self.open {
            return None;
        }

        let mut action: Option<AddonsAction> = None;
        let plugins_dir = Config::plugins_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "(unknown)".to_string());

        egui::Window::new("Addons")
            .open(&mut self.open)
            .resizable(true)
            .min_width(420.0)
            .min_height(240.0)
            .default_width(520.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                // ── Top toolbar ─────────────────────────────────────────────
                ui.horizontal(|ui| {
                    let count = plugin_host.plugins.len();
                    ui.label(
                        RichText::new(format!(
                            "{count} addon{} installed",
                            if count == 1 { "" } else { "s" }
                        ))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📂 Open folder").clicked() {
                            open_plugins_folder(&plugins_dir);
                        }
                        if ui.button("🔄 Reload").clicked() {
                            action = Some(AddonsAction::Reload);
                        }
                    });
                });

                ui.separator();

                // ── Plugin list ──────────────────────────────────────────────
                if plugin_host.plugins.is_empty() {
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No addons installed.").weak());
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!("Drop a plugin folder into:\n{plugins_dir}"))
                                .small()
                                .weak(),
                        );
                        ui.add_space(6.0);
                        if ui.button("📂 Open folder").clicked() {
                            open_plugins_folder(&plugins_dir);
                        }
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for plugin in &plugin_host.plugins {
                            let name = &plugin.manifest.name;
                            let is_disabled = config.disabled_plugins.contains(name);

                            egui::Frame::none()
                                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                                .inner_margin(egui::Margin::same(10.0))
                                .outer_margin(egui::Margin::symmetric(0.0, 4.0))
                                .rounding(egui::Rounding::same(6.0))
                                .show(ui, |ui| {
                                    plugin_row(
                                        ui,
                                        plugin,
                                        is_disabled,
                                        &mut self.confirm_remove,
                                        &mut action,
                                    );
                                });
                        }
                    });
                }

                // ── Footer ───────────────────────────────────────────────────
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Plugin directory:  {plugins_dir}"))
                            .small()
                            .weak(),
                    );
                });
            });

        action
    }
}

// ── Plugin row ───────────────────────────────────────────────────────────────

fn plugin_row(
    ui: &mut Ui,
    plugin: &LoadedPlugin,
    is_disabled: bool,
    confirm_remove: &mut Option<String>,
    action: &mut Option<AddonsAction>,
) {
    let name = &plugin.manifest.name;

    ui.horizontal(|ui| {
        // Enable / disable toggle
        let mut enabled = !is_disabled;
        if ui.checkbox(&mut enabled, "").changed() {
            *action = Some(AddonsAction::Toggle(name.clone()));
        }

        // Name + version
        let name_text = if is_disabled {
            RichText::new(name).weak()
        } else {
            RichText::new(name).strong()
        };
        ui.label(name_text);
        ui.label(
            RichText::new(format!("v{}", plugin.manifest.version))
                .small()
                .weak(),
        );

        if is_disabled {
            ui.label(
                RichText::new("disabled")
                    .small()
                    .color(Color32::from_rgb(160, 120, 60)),
            );
        }

        // Remove button / confirmation (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if confirm_remove.as_deref() == Some(name.as_str()) {
                // Confirmation row
                if ui.button(RichText::new("Cancel").small()).clicked() {
                    *confirm_remove = None;
                }
                if ui
                    .button(
                        RichText::new("⚠ Confirm remove")
                            .small()
                            .color(Color32::RED),
                    )
                    .clicked()
                {
                    *action = Some(AddonsAction::Remove(name.clone()));
                    *confirm_remove = None;
                }
            } else if ui.button(RichText::new("Remove").small()).clicked() {
                *confirm_remove = Some(name.clone());
            }
        });
    });

    // Description + author (second line)
    let has_desc = plugin
        .manifest
        .description
        .as_deref()
        .map(|d| !d.is_empty())
        .unwrap_or(false);
    let has_author = plugin
        .manifest
        .author
        .as_deref()
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    if has_desc || has_author {
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(22.0); // indent under checkbox
            if let Some(desc) = &plugin.manifest.description {
                if !desc.is_empty() {
                    ui.label(RichText::new(desc).small());
                }
            }
            if has_desc && has_author {
                ui.label(RichText::new("·").small().weak());
            }
            if let Some(author) = &plugin.manifest.author {
                if !author.is_empty() {
                    ui.label(RichText::new(format!("by {author}")).small().weak());
                }
            }
        });
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn open_plugins_folder(path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
