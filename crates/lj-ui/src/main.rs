mod addons;
mod app;
mod editor;
mod settings;
mod sidebar;
mod theme;

use app::LockJawApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("lj=info".parse().unwrap()),
        )
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lock Jaw")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([700.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Lock Jaw",
        native_options,
        Box::new(|cc| Ok(Box::new(LockJawApp::new(cc)))),
    )
}
