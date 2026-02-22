// src/socket.rs

#[cfg(unix)]
pub fn socket_name(app_id: &str) -> interprocess::local_socket::Name<'static> {
    use interprocess::local_socket::{prelude::*, GenericFilePath};
    let path = Box::leak(format!("/tmp/{app_id}.sock").into_boxed_str());
    path.to_fs_name::<GenericFilePath>().unwrap()
}

#[cfg(not(unix))]
pub fn socket_name(app_id: &str) -> interprocess::local_socket::Name<'static> {
    use interprocess::local_socket::{prelude::*, GenericNamespaced};
    let name = Box::leak(app_id.to_string().into_boxed_str());
    name.to_ns_name::<GenericNamespaced>().unwrap()
}

#[cfg(unix)]
pub fn cleanup(app_id: &str) {
    
    let _ = std::fs::remove_file(format!("/tmp/{app_id}.sock"));
}