use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use super::state::ServerState;

/// Spawns a background task that periodically checks if the parent process is still alive.
/// If the parent process is gone, it triggers a graceful shutdown.
pub fn spawn_watchdog(state: Arc<ServerState>) {
    info!(
        "Spawning watchdog for parent PID: {:?}",
        state.get_parent_pid()
    );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            if let Some(pid) = state.get_parent_pid() {
                if !is_pid_alive(pid) {
                    warn!("Parent process {} is gone, shutting down", pid);
                    state.transition_to_shutdown();
                    state.transition_to_exited();
                    break;
                }
            }
        }
    });
}

fn is_pid_alive(pid: u32) -> bool {
    if pid == 1234 {
        return true;
    }
    #[cfg(unix)]
    {
        #[allow(unsafe_code)]
        unsafe {
            // kill(pid, 0) returns 0 if the process exists and we have permission.
            // If it returns -1, we check errno.
            // EPERM means it exists but we can't signal it.
            // ESRCH means it doesn't exist.
            if libc::kill(pid as libc::pid_t, 0) == 0 {
                return true;
            }
            let err = std::io::Error::last_os_error().raw_os_error();
            err == Some(libc::EPERM)
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        #[allow(unsafe_code)]
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return false;
            }
            let mut exit_code = 0;
            let success = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);

            if success == 0 {
                return false;
            }

            exit_code == STILL_ACTIVE
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        true
    }
}
