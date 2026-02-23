# app-single-instance

A lightweight, cross-platform single-instance library for Rust desktop applications.

When a second instance of your application is launched, this library sends a wake-up signal to the already-running instance and lets the second instance exit gracefully. The primary instance can then respond by restoring and focusing its window.

## Platform Support

| Platform | Socket Type | Force Window to Front |
|---|---|---|
| Windows | Named pipe (namespaced) | ✅ Supported via egui viewport commands |
| macOS | Unix domain socket (`/tmp/`) | ✅ Supported via `NSApplication` |
| Linux (X11) | Unix domain socket (`/tmp/`) | ✅ Supported via `wmctrl` |
| Linux (Wayland) | Unix domain socket (`/tmp/`) | ⚠️ Not supported (see [Wayland Limitation](#wayland-limitation)) |

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
app-single-instance = "0.1"
```

## Usage

The typical usage pattern involves two steps:

1. **Before launching your app**, call [`notify_if_running`] to check whether another instance is already running. If it is, the signal is sent and you should exit immediately.
2. **After launching your app**, call [`start_primary`] to register the current process as the primary instance and begin listening for signals.

```rust
use app_single_instance::{notify_if_running, start_primary};

const APP_ID: &str = env!("CARGO_PKG_NAME");

fn main() {
    // Step 1: if another instance is running, wake it up and exit.
    if notify_if_running(APP_ID) {
        return;
    }

    // Step 2: register as the primary instance.
    // The callback is called on a background thread when a wake-up signal arrives.
    let handle = start_primary(APP_ID, || {
        println!("Another instance was launched. Bring window to front here.");
    });

    // Keep `handle` alive for the duration of your application.
    // Dropping it will shut down the listener and clean up the socket.
    run_app(handle);
}
```

> **Important**: `notify_if_running` must always be called before `start_primary`. The `handle` returned by `start_primary` must be kept alive for the duration of your application — dropping it will stop the listener.

## API

### `notify_if_running(app_id: &str) -> bool`

Checks whether a primary instance is already running.

- Returns `true` if a running instance was found and the wake-up signal was sent. The caller should exit immediately.
- Returns `false` if no existing instance was detected. The caller may proceed to start normally.

### `start_primary(app_id: &str, on_show: impl Fn() + Send + 'static) -> PrimaryHandle`

Registers the current process as the primary instance and spawns a background listener thread.

- `app_id`: A string that uniquely identifies your application. It is used to name the IPC socket, so it must be consistent across all calls and unique enough to avoid conflicts with other applications. Using `env!("CARGO_PKG_NAME")` is a safe default.
- `on_show`: A callback invoked on the listener thread each time a wake-up signal is received. Use this to trigger a window restore, repaint request, or similar action.

### `PrimaryHandle`

The handle returned by `start_primary`. It serves two purposes:

- **Keeps the listener alive**: the background thread runs as long as the handle is not dropped.
- **Poll-based detection**: call `handle.check_show()` from your main loop as an alternative to the callback.

#### `PrimaryHandle::check_show() -> bool`

Returns `true` if a wake-up signal has been received since the last call. This is non-blocking and suitable for polling inside a UI update loop.

## Example: egui / eframe

A complete example using [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) is provided in [`examples/egui.rs`](examples/egui.rs).

```rust
use eframe::egui;
use app_single_instance::{notify_if_running, start_primary};

const APP_ID: &str = env!("CARGO_PKG_NAME");

fn main() -> eframe::Result<()> {
    if notify_if_running(APP_ID) {
        return Ok(());
    }

    eframe::run_native(
        "My App",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let handle = start_primary(APP_ID, move || {
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
        if self.handle.check_show() {
            // Restore and focus the window.
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello!");
        });
    }
}
```

Run the example with:

```sh
cargo run --example egui
```

Launch a second terminal and run the same command again to see the single-instance behavior in action.

## Wayland Limitation

On Linux under a Wayland compositor, **it is not possible for an application to force itself to the foreground**. This is a deliberate design decision of the Wayland protocol: compositors control window stacking and focus, and applications are not permitted to request it unconditionally.

As a result, when a wake-up signal is received on Wayland:

- `on_show` is still called and `check_show()` still returns `true` — your application logic runs normally.
- Viewport commands such as `ViewportCommand::Focus` are sent but may have no visible effect depending on the compositor.
- The window cannot be reliably un-minimized or raised to the front programmatically.

The recommended approach on Wayland is to use **taskbar attention requests** to notify the user that the application wants focus, and let the user act on it:

```rust
ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
    egui::UserAttentionType::Informational,
));
```

This will typically cause the application's taskbar icon to flash or highlight, which is the standard Wayland-compliant way to signal the user.

On X11 (still common on Ubuntu with the classic session), `wmctrl` can be used as a workaround:

```rust
#[cfg(target_os = "linux")]
{
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        // Wayland: notify the user via taskbar attention only.
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
    } else {
        // X11: use wmctrl to raise the window.
        let _ = std::process::Command::new("wmctrl")
            .args(["-a", "My App"])
            .spawn();
    }
}
```

> `wmctrl` must be installed on the user's system (`sudo apt install wmctrl`). The `-a` argument must match the window title exactly.

## Unclean Shutdown and Stale Sockets

On Unix platforms, local sockets are backed by files under `/tmp/`. If the primary instance exits uncleanly (e.g. via `SIGKILL` or a crash), the socket file may be left behind. On the next launch, this would normally cause an `AddrInUse` error.

This library handles this automatically: if binding fails with `AddrInUse`, the stale socket file is removed and the bind is retried. No user intervention is required.

## License

MIT