// src/listener.rs

use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::thread;
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::prelude::*;
use crate::socket;

pub fn start(app_id: String, on_show: Box<dyn Fn() + Send>) -> mpsc::Receiver<()> {
    let listener = loop {
        match ListenerOptions::new().name(socket::socket_name(&app_id)).create_sync() {
            Ok(l) => break l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                #[cfg(unix)]
                socket::cleanup(&app_id);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => panic!("IPC 错误: {e}"),
        }
    };

    let (tx, rx) = mpsc::channel::<()>();

    thread::spawn(move || {
        for conn in listener.incoming().filter_map(|c| c.ok()) {
            let mut line = String::new();
            if BufReader::new(conn).read_line(&mut line).is_ok() {
                let _ = tx.send(());
                on_show();  // 通知调用方
            }
        }
    });

    rx
}