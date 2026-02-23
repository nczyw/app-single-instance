// src/socket.rs

/// Returns a local socket [`Name`] derived from `app_id`.
///
/// On Unix, the socket is created as a file at `/tmp/<app_id>.sock`.
/// On other platforms (e.g. Windows), a namespaced socket name is used instead.
///
/// The returned [`Name`] has a `'static` lifetime, achieved by leaking the
/// formatted string. This is acceptable because the socket name is created
/// once per process and lives for the duration of the application.
///
/// # Arguments
///
/// * `app_id` - A unique identifier for the application.
#[cfg(unix)]
pub fn socket_name(app_id: &str) -> interprocess::local_socket::Name<'static> {
    use interprocess::local_socket::{prelude::*, GenericFilePath};
    let path = Box::leak(format!("/tmp/{app_id}.sock").into_boxed_str());
    path.to_fs_name::<GenericFilePath>().unwrap()
}

/// Returns a local socket [`Name`] derived from `app_id`.
///
/// On Unix, the socket is created as a file at `/tmp/<app_id>.sock`.
/// On other platforms (e.g. Windows), a namespaced socket name is used instead.
///
/// The returned [`Name`] has a `'static` lifetime, achieved by leaking the
/// formatted string. This is acceptable because the socket name is created
/// once per process and lives for the duration of the application.
///
/// # Arguments
///
/// * `app_id` - A unique identifier for the application.
#[cfg(not(unix))]
pub fn socket_name(app_id: &str) -> interprocess::local_socket::Name<'static> {
    use interprocess::local_socket::{prelude::*, GenericNamespaced};
    let name = Box::leak(app_id.to_string().into_boxed_str());
    name.to_ns_name::<GenericNamespaced>().unwrap()
}

/// Removes the socket file left behind by a previous run.
///
/// On Unix, local sockets are represented as files under `/tmp/`.
/// If the application exits uncleanly (e.g. via a crash or SIGKILL),
/// the socket file may persist and cause `AddrInUse` errors on the next launch.
/// This function removes that stale file so the socket can be rebound.
///
/// Errors are silently ignored, as the file may not exist in normal circumstances.
///
/// # Arguments
///
/// * `app_id` - A unique identifier for the application, used to locate the socket file.
#[cfg(unix)]
pub fn cleanup(app_id: &str) {
    let _ = std::fs::remove_file(format!("/tmp/{app_id}.sock"));
}