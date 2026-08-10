mod core;
mod models;
mod ui;
mod utils;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Start MCP Server in a background thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            crate::core::mcp::start_mcp_server().await;
        });
    });

    // Define window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("X-Term")
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };

    // Run the app
    eframe::run_native(
        "X-Term",
        options,
        Box::new(|cc| Ok(Box::new(ui::app::XTermApp::new(cc)))),
    )
}
