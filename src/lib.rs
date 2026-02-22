// src/lib.rs

mod socket;
mod listener;

use std::io::Write;
use interprocess::local_socket::prelude::*;

pub struct PrimaryHandle {
    #[cfg(unix)]
    app_id: String,
    rx: std::sync::mpsc::Receiver<()>,
}

impl PrimaryHandle {
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

/// 如果已有实例在运行，发送唤起信号并返回 true
/// 返回 true 时调用方应该直接退出
pub fn notify_if_running(app_id: &str) -> bool {
    if let Ok(mut s) = LocalSocketStream::connect(socket::socket_name(app_id)) {
        let _ = s.write_all(b"show\n");
        return true;
    }
    false
}

/// 注册为主实例，开始监听其他实例的信号
/// 必须在 notify_if_running 返回 false 之后调用
pub fn start_primary(app_id: &str, on_show: impl Fn() + Send + 'static) -> PrimaryHandle {
    let rx = listener::start(app_id.to_string(), Box::new(on_show));
    PrimaryHandle {
        #[cfg(unix)]
        app_id: app_id.to_string(),
        rx,
    }
}