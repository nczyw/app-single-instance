// src/lib.rs

mod socket;
mod listener;

use std::io::Write;
use interprocess::local_socket::prelude::*;

/// A handle representing the primary (first) instance of the application.
///
/// Holds the IPC listener alive for the lifetime of the application.
/// When dropped, cleans up the socket file on Unix platforms.
pub struct PrimaryHandle {
    #[cfg(unix)]
    app_id: String,
    rx: std::sync::mpsc::Receiver<()>,
}

impl PrimaryHandle {
    /// Returns `true` if another instance has sent a wake-up signal since the last call.
    ///
    /// This is non-blocking and is intended to be polled from the application's main loop,
    /// for example to bring the existing window to the foreground.
    pub fn check_show(&self) -> bool {
        self.rx.try_recv().is_ok()
    }
}

impl Drop for PrimaryHandle {
    fn drop(&mut self) {
        #[cfg(unix)]
        socket::cleanup(&self.app_id);
    }
}

/// Checks whether another instance of the application is already running.
///
/// If a running instance is found, sends a wake-up signal to it and returns `true`.
/// The caller should exit immediately in this case.
///
/// Returns `false` if no existing instance was detected, meaning the caller
/// may proceed to launch as the primary instance.
///
/// # Arguments
///
/// * `app_id` - A unique identifier for the application, used to name the IPC socket.
pub fn notify_if_running(app_id: &str) -> bool {
    if let Ok(mut s) = LocalSocketStream::connect(socket::socket_name(app_id)) {
        let _ = s.write_all(b"show\n");
        return true;
    }
    false
}

/// Registers the current process as the primary instance and begins listening
/// for signals from any subsequently launched instances.
///
/// Spawns a background thread that listens on a local socket. When a signal
/// is received, `on_show` is called on the background thread and a message is
/// also sent to [`PrimaryHandle::check_show`] for poll-based detection.
///
/// This function must be called only after [`notify_if_running`] has returned `false`.
///
/// # Arguments
///
/// * `app_id` - A unique identifier for the application, must match the one passed to [`notify_if_running`].
/// * `on_show` - Callback invoked on the listener thread each time a wake-up signal is received.
///
/// # Returns
///
/// A [`PrimaryHandle`] that keeps the listener alive. Dropping it will shut down
/// the listener and clean up resources.
pub fn start_primary(app_id: &str, on_show: impl Fn() + Send + 'static) -> PrimaryHandle {
    let rx = listener::start(app_id.to_string(), Box::new(on_show));
    PrimaryHandle {
        #[cfg(unix)]
        app_id: app_id.to_string(),
        rx,
    }
}