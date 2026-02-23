// src/listener.rs

use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::thread;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::prelude::*;
use crate::socket;

/// Binds a local socket for `app_id` and spawns a background thread to accept incoming connections.
///
/// If the socket address is already in use (e.g. from a previous unclean shutdown),
/// the stale socket file is removed and binding is retried automatically.
///
/// For each accepted connection, one line is read. Upon receipt, `on_show` is invoked
/// and a unit value is sent to the returned [`mpsc::Receiver`], allowing the caller
/// to detect the event via either the callback or polling.
///
/// # Panics
///
/// Panics if the socket cannot be created for any reason other than `AddrInUse`.
pub fn start(app_id: String, on_show: Box<dyn Fn() + Send>) -> mpsc::Receiver<()> {
    let listener = loop {
        match ListenerOptions::new().name(socket::socket_name(&app_id)).create_sync() {
            Ok(l) => break l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                #[cfg(unix)]
                socket::cleanup(&app_id);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => panic!("IPC error: {e}"),
        }
    };

    let (tx, rx) = mpsc::channel::<()>();

    thread::spawn(move || {
        for conn in listener.incoming().filter_map(|c| c.ok()) {
            let mut line = String::new();
            if BufReader::new(conn).read_line(&mut line).is_ok() {
                let _ = tx.send(());
                on_show();  // notify the caller
            }
        }
    });

    rx
}