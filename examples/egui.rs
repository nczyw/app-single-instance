//! # Single Instance Example (egui)
//!
//! This example demonstrates how to use `app-single-instance` with an [`eframe`] / [`egui`] application.
//!
//! ## Behavior
//!
//! - If the application is already running, the new instance sends a wake-up signal
//!   to the existing window and exits immediately.
//! - The primary instance listens for wake-up signals in the background. When one is
//!   received, [`bring_to_front`] is called to restore and focus the window.
//!
//! ## How to run
//!
//! ```sh
//! cargo run --example egui
//! ```
//!
//! Launch a second instance while the first is running to see the single-instance behavior.

use eframe::egui;
use app_single_instance::{notify_if_running, start_primary};

const APPID: &str = env!("CARGO_PKG_NAME");

fn main() -> eframe::Result<()> {
    // If another instance is already running, wake it up and exit.
    if notify_if_running(APPID) {
        return Ok(());
    }

    eframe::run_native(
        "Single Instance Demo",
        eframe::NativeOptions {
            run_and_return: false,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([300.0, 150.0])
                .with_title("Single Instance Demo"),
            ..Default::default()
        },
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            // Register as the primary instance.
            // Request a repaint so check_show() is evaluated on the next frame.
            let handle = start_primary(APPID, move || {
                ctx.request_repaint();
            });
            Ok(Box::new(MyApp { handle }))
        }),
    )
}

struct MyApp {
    handle: app_single_instance::PrimaryHandle,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for wake-up signals from other instances.
        if self.handle.check_show() {
            bring_to_front(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                if ui.add_sized([120.0, 40.0], egui::Button::new("❌ Close")).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

/// Restores and focuses the application window.
///
/// Uses platform-specific mechanisms where egui's viewport commands alone are insufficient:
/// - **macOS**: Activates the app via `NSApplication` and deminiaturizes any minimized windows.
/// - **Linux/X11**: Uses `wmctrl` to bring the window to the front.
/// - **Linux/Wayland**: Forceful window activation is not supported; logs a warning instead.
/// - **Windows**: egui's [`egui::ViewportCommand::Focus`] is sufficient.
fn bring_to_front(ctx: &egui::Context) {
    println!("Wake-up signal received, bringing window to front.");
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
        egui::UserAttentionType::Informational,
    ));

    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSApplication;

        let mtm = MainThreadMarker::new().expect("must be called on the main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.activate();

        let windows = app.windows();
        for window in windows {
            if window.isMiniaturized() {
                window.deminiaturize(Some(&window));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            eprintln!("Wayland: forceful window activation is not supported.");
        } else {
            let _ = std::process::Command::new("wmctrl")
                .args(["-a", "Single Instance Demo"])
                .spawn();
        }
    }
}