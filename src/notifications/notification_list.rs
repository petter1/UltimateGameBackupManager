// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::notifications::notification::Notification;
use smol::{fs::File, io::AsyncWriteExt, lock::Mutex};
use std::{path::Path, sync::Arc};

/// How many messages the history keeps; the log file has all.
const HISTORY_LEN: usize = 1000;

#[derive(Debug)]
pub struct NotificationList {
    /// The toasts shown in the window, until closed.
    inner: Vec<Notification>,
    /// Every message since the start, closed or not, for the Messages page.
    history: Vec<Notification>,
    log: Option<Arc<Mutex<File>>>,
}

impl NotificationList {
    pub fn new(data_dir: &Path) -> Self {
        let log = {
            let path = data_dir.join("log.txt");

            std::fs::File::create(path)
                .map(|f| Arc::new(Mutex::new(File::from(f))))
                .ok()
        };

        NotificationList {
            inner: Vec::new(),
            history: Vec::new(),
            log,
        }
    }

    pub fn push(&mut self, notification: Notification) {
        println!("{notification}");

        if let Some(log) = &self.log {
            let log = log.clone();
            let s = format!("{notification}\n");

            smol::spawn(async move {
                let _ = log.lock().await.write_all(s.as_bytes()).await;
            })
            .detach();
        }

        // in the history, the same message right after itself is one entry
        match self.history.last_mut() {
            Some(last) if last.is_same(&notification) => last.repeat(notification.clone()),
            _ => {
                if self.history.len() == HISTORY_LEN {
                    self.history.remove(0);
                }
                self.history.push(notification.clone());
            }
        }

        // the same message again (e.g. one error per file) is counted, not shown again
        match self.inner.iter_mut().find(|n| n.is_same(&notification)) {
            Some(existing) => existing.repeat(notification),
            None => self.inner.push(notification),
        }
    }

    pub fn close(&mut self, index: usize) {
        self.inner.remove(index);
    }

    pub fn has_notifications(&self) -> bool {
        !self.inner.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.inner.iter()
    }

    /// Every message since the start, the oldest first.
    pub fn history(&self) -> &[Notification] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> NotificationList {
        NotificationList {
            inner: Vec::new(),
            history: Vec::new(),
            log: None,
        }
    }

    #[test]
    fn the_same_message_is_counted_once() {
        let mut list = list();
        for _ in 0..3 {
            list.push(Notification::error("Read-only file system"));
        }
        list.push(Notification::warning("Read-only file system"));
        list.push(Notification::error("Disk full"));

        let shown = list
            .iter()
            .map(|n| (n.label(), n.count()))
            .collect::<Vec<_>>();
        assert_eq!(
            shown,
            [
                ("Read-only file system", 3),
                ("Read-only file system", 1),
                ("Disk full", 1)
            ]
        );
    }

    #[test]
    fn the_history_keeps_closed_messages_and_groups_runs() {
        let mut list = list();
        list.push(Notification::error("Read-only file system"));
        list.push(Notification::error("Read-only file system"));
        list.push(Notification::info("Imported"));
        list.push(Notification::error("Read-only file system"));
        list.close(0);
        list.close(0);

        let history = list
            .history()
            .iter()
            .map(|n| (n.label(), n.count()))
            .collect::<Vec<_>>();
        assert_eq!(
            history,
            [
                ("Read-only file system", 2),
                ("Imported", 1),
                ("Read-only file system", 1)
            ]
        );
        assert!(!list.has_notifications());
    }

    #[test]
    fn a_closed_message_starts_counting_again() {
        let mut list = list();
        list.push(Notification::error("Read-only file system"));
        list.push(Notification::error("Read-only file system"));
        list.close(0);
        list.push(Notification::error("Read-only file system"));

        assert_eq!(
            list.iter().map(Notification::count).collect::<Vec<_>>(),
            [1]
        );
    }
}
