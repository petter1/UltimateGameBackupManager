// SPDX-FileCopyrightText: 2026 Ultimate Game Backup Manager contributors
// SPDX-License-Identifier: GPL-3.0-only

//! The system's notifications (M1a+), for what matters while the app is in the background:
//! macOS Notification Center, Windows toasts, the freedesktop notification spec on Linux.

use notify_rust::Notification;

/// Shows a notification from the system; failures are only printed, the toast in the window is
/// still there.
pub fn show(summary: String, body: String) {
    smol::unblock(move || {
        #[cfg(target_os = "macos")]
        {
            // shows our name and icon when run from the .app; `cargo run` keeps the default.
            // Can only be set once: later calls fail with "already set".
            static SET_APPLICATION: std::sync::Once = std::sync::Once::new();
            SET_APPLICATION.call_once(|| {
                if let Err(e) =
                    notify_rust::set_application("io.github.petter1.UltimateGameBackupManager")
                {
                    eprintln!("Can't send system notifications as the app: {e}");
                }
            });
        }

        // on macOS, show() only builds the notification: it is sent when the handle is dropped,
        // and a failure there isn't reported
        if let Err(e) = Notification::new()
            .appname("Ultimate Game Backup Manager")
            .summary(&summary)
            .body(&body)
            .show()
        {
            eprintln!("Can't show a system notification: {e}");
        }
    })
    .detach();
}
