use egui::Color32;
use serde::{Deserialize, Serialize};

/// A Lock Jaw color theme, loaded from a TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub meta: ThemeMeta,
    pub colors: ThemeColors,
    #[serde(default)]
    pub editor: ThemeEditor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub name: String,
    #[serde(default = "default_dark")]
    pub dark: bool,
}

fn default_dark() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub bg_primary: String,
    pub bg_surface: String,
    pub bg_code: String,
    pub fg_primary: String,
    pub fg_muted: String,
    pub accent: String,
    pub accent_muted: String,
    pub success: String,
    pub warning: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEditor {
    pub font_size: f32,
}

impl Default for ThemeEditor {
    fn default() -> Self {
        Self { font_size: 14.0 }
    }
}

impl Theme {
    /// Built-in dark theme (used if no theme file is found).
    pub fn dark() -> Self {
        toml::from_str(include_str!("../../../themes/dark.toml"))
            .expect("bundled dark theme is valid TOML")
    }

    /// Built-in light theme.
    pub fn light() -> Self {
        toml::from_str(include_str!("../../../themes/light.toml"))
            .expect("bundled light theme is valid TOML")
    }

    /// Apply this theme to an egui Context.
    pub fn apply(&self, ctx: &egui::Context) {
        let c = &self.colors;
        let bg = parse_color(&c.bg_primary);
        let surface = parse_color(&c.bg_surface);
        let fg = parse_color(&c.fg_primary);
        let accent = parse_color(&c.accent);
        let muted = parse_color(&c.fg_muted);

        let mut visuals = if self.meta.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.window_fill = bg;
        visuals.panel_fill = bg;
        visuals.faint_bg_color = surface;
        visuals.extreme_bg_color = parse_color(&c.bg_code);

        visuals.widgets.noninteractive.bg_fill = surface;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, fg);

        visuals.widgets.inactive.bg_fill = surface;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, muted);

        visuals.widgets.hovered.bg_fill = lerp_color(surface, accent, 0.15);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg);

        visuals.widgets.active.bg_fill = lerp_color(surface, accent, 0.3);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, fg);

        visuals.selection.bg_fill = accent.linear_multiply(0.35);
        visuals.selection.stroke = egui::Stroke::new(1.0, accent);

        visuals.hyperlink_color = accent;

        ctx.set_visuals(visuals);
    }
}

/// Parse a `#rrggbb` hex color string into `Color32`.
pub fn parse_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return Color32::WHITE;
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    Color32::from_rgb(r, g, b)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        lerp_u8(a.r(), b.r(), t),
        lerp_u8(a.g(), b.g(), t),
        lerp_u8(a.b(), b.b(), t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}
