//! Library entry point. Split out from `main.rs` so the same crate can be
//! embedded in integration tests.

pub mod analysis;
pub mod app;
pub mod domain;
pub mod ipc;

use std::sync::Arc;

use tauri::Manager;

/// Shared state container. Wrapping `GraphService` in `Arc` makes it cheap
/// to clone into multiple Tauri commands.
pub struct AppState {
    pub graph: Arc<crate::app::GraphService>,
}

/// Boot the Tauri application. Called from `main.rs` and from tests.
pub fn run() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        graph: Arc::new(crate::app::GraphService::new()),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            crate::ipc::analyze_solution,
            crate::ipc::get_graph,
            crate::ipc::get_method,
            crate::ipc::get_callers,
            crate::ipc::get_callees,
        ])
        .setup(|app| {
            // Ensure the main window exists. Tauri 2 emits a warning if
            // `tauri.conf.json` declares no windows and we don't create one
            // programmatically, so we attach a fallback here.
            if app.get_webview_window("main").is_none() {
                let _ = tauri::WebviewWindowBuilder::new(
                    app,
                    "main",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("CodeGraph")
                .inner_size(1280.0, 800.0)
                .build();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
