// SPDX-FileCopyrightText: 2026 Ultimate Game Backup Manager contributors
// SPDX-License-Identifier: GPL-3.0-only

//! Keeps the computer from going to sleep from idleness while data moves (M1a). The screen may
//! still turn off.
//!
//! - macOS: `caffeinate -i`, like an IOKit "prevent idle sleep" power assertion; `-w` ends it if
//!   the app quits without letting go.
//! - Linux: `systemd-inhibit --what=idle:sleep`; it holds the lock while `cat` reads from a pipe
//!   to the app, so it ends with the app too.
//! - Windows: `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)` on the UI thread.

#[derive(Debug, Default)]
pub struct KeepAwake {
    guard: Option<platform::Guard>,
    /// Holding failed once (e.g. no systemd); not tried again, so the user is told only once.
    failed: bool,
}

impl KeepAwake {
    /// Holds or lets go, as `busy` says. Returns why holding failed, the first time it does.
    pub fn set(&mut self, busy: bool) -> Option<String> {
        if !busy {
            self.guard = None;
            return None;
        }

        if self.guard.is_some() || self.failed {
            return None;
        }

        match platform::Guard::acquire() {
            Ok(guard) => {
                self.guard = Some(guard);
                None
            }
            Err(e) => {
                self.failed = true;
                Some(format!(
                    "Can't keep the computer awake while tasks run: {e}"
                ))
            }
        }
    }

    #[cfg(test)]
    fn is_held(&self) -> bool {
        self.guard.is_some()
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod platform {
    use std::{
        io,
        process::{Child, Command, Stdio},
    };

    /// The helper process holding the lock; killed to let go.
    #[derive(Debug)]
    pub struct Guard(Child);

    impl Guard {
        #[cfg(target_os = "macos")]
        pub fn acquire() -> io::Result<Self> {
            Command::new("caffeinate")
                .arg("-i")
                .arg("-w")
                .arg(std::process::id().to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map(Self)
        }

        #[cfg(target_os = "linux")]
        pub fn acquire() -> io::Result<Self> {
            let mut child = Command::new("systemd-inhibit")
                .args([
                    "--what=idle:sleep",
                    "--who=Ultimate Game Backup Manager",
                    "--why=Tasks are moving data",
                    "--mode=block",
                    "cat",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()?;

            // systemd-inhibit fails at once without logind (e.g. in a container)
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(status) = child.try_wait()? {
                let mut reason = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let _ = io::Read::read_to_string(&mut stderr, &mut reason);
                }
                let reason = reason.trim();
                return Err(io::Error::other(if reason.is_empty() {
                    format!("systemd-inhibit ended ({status})")
                } else {
                    reason.to_string()
                }));
            }

            Ok(Self(child))
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

// Unsafe like the one in main.rs: one Win32 call with plain flags, which no dependency offers
// (winsafe doesn't wrap it); a crate for it would pull in the `windows` crate.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
mod platform {
    use std::io;

    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }

    /// Holds while it lives. Created and dropped on the UI thread, as the state belongs to the
    /// thread that set it.
    #[derive(Debug)]
    pub struct Guard;

    impl Guard {
        pub fn acquire() -> io::Result<Self> {
            // SAFETY: takes flags only; 0 means it failed
            if unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) } == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(Self)
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            // SAFETY: as above
            unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_while_busy_and_reports_a_failure_once() {
        let mut keep_awake = KeepAwake::default();

        let first = keep_awake.set(true);
        // without logind (CI containers) holding fails: then it says so once, and only once
        assert_eq!(keep_awake.is_held(), first.is_none());
        assert_eq!(keep_awake.set(true), None);

        assert_eq!(keep_awake.set(false), None);
        assert!(!keep_awake.is_held());
    }
}
