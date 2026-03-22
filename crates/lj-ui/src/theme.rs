use egui::Color32;
use serde::{Deserialize, Serialize};

/// All themes available in the UI, in display order.
/// `("config_key", "Display Name")`
pub const THEMES: &[(&str, &str)] = &[
    ("system", "System (follow OS)"),
    ("dark", "Dark"),
    ("light", "Light"),
    ("nord", "Nord"),
    ("dracula", "Dracula"),
    ("solarized-dark", "Solarized Dark"),
    ("solarized-light", "Solarized Light"),
    ("gruvbox-dark", "Gruvbox Dark"),
    ("rose-pine", "Rosé Pine"),
];

/// A Lock Jaw color theme loaded from TOML.
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

// ── Bundled theme constructors ────────────────────────────────────────────────

impl Theme {
    pub fn dark() -> Self {
        parse_bundled(include_str!("../../../themes/dark.toml"))
    }
    pub fn light() -> Self {
        parse_bundled(include_str!("../../../themes/light.toml"))
    }
    pub fn nord() -> Self {
        parse_bundled(include_str!("../../../themes/nord.toml"))
    }
    pub fn dracula() -> Self {
        parse_bundled(include_str!("../../../themes/dracula.toml"))
    }
    pub fn solarized_dark() -> Self {
        parse_bundled(include_str!("../../../themes/solarized-dark.toml"))
    }
    pub fn solarized_light() -> Self {
        parse_bundled(include_str!("../../../themes/solarized-light.toml"))
    }
    pub fn gruvbox_dark() -> Self {
        parse_bundled(include_str!("../../../themes/gruvbox-dark.toml"))
    }
    pub fn rose_pine() -> Self {
        parse_bundled(include_str!("../../../themes/rose-pine.toml"))
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

        // Modern visual polish
        visuals.window_rounding = egui::Rounding::same(8.0);
        visuals.menu_rounding = egui::Rounding::same(6.0);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0.0, 8.0].into(),
            blur: 32.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(96),
        };
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0.0, 4.0].into(),
            blur: 16.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(96),
        };

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(8.0, 6.0);
        style.spacing.menu_margin = egui::Margin::same(6.0);

        style.visuals = visuals;
        ctx.set_style(style);
    }
}

fn parse_bundled(src: &str) -> Theme {
    toml::from_str(src).expect("bundled theme must be valid TOML")
}

// ── Public helpers ────────────────────────────────────────────────────────────

/// Load a theme by its config key.
/// Checks user themes dir first, then falls back to bundled themes.
pub fn load_theme_by_name(name: &str) -> Theme {
    if let Some(dir) = lj_core::Config::themes_dir() {
        let path = dir.join(format!("{name}.toml"));
        if path.exists() {
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(t) = toml::from_str::<Theme>(&s) {
                    return t;
                }
            }
        }
    }
    match name {
        "light" => Theme::light(),
        "nord" => Theme::nord(),
        "dracula" => Theme::dracula(),
        "solarized-dark" => Theme::solarized_dark(),
        "solarized-light" => Theme::solarized_light(),
        "gruvbox-dark" => Theme::gruvbox_dark(),
        "rose-pine" => Theme::rose_pine(),
        "system" => resolve_system_theme(),
        _ => Theme::dark(),
    }
}

/// Detect the OS dark/light preference and return the matching bundled theme.
pub fn resolve_system_theme() -> Theme {
    match dark_light::detect() {
        dark_light::Mode::Light => Theme::light(),
        _ => Theme::dark(),
    }
}

/// Representative accent swatch color for each theme key — used in the settings UI.
pub fn theme_accent(key: &str) -> Color32 {
    match key {
        "light" => parse_color("#c0392b"),
        "nord" => parse_color("#88c0d0"),
        "dracula" => parse_color("#ff79c6"),
        "solarized-dark" | "solarized-light" => parse_color("#268bd2"),
        "gruvbox-dark" => parse_color("#b8bb26"),
        "rose-pine" => parse_color("#eb6f92"),
        "system" => Color32::GRAY,
        _ => parse_color("#e94560"),
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
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}
