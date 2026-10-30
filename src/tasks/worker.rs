// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use smol::{channel::Sender, stream::Stream};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// What a running task reports back
#[derive(Debug, Clone)]
pub enum Event {
    /// What it is doing right now, e.g. "Opening Mario Kart Wii.wbfs"
    Step(String),
    /// Bytes done of bytes in total
    Progress {
        done: u64,
        total: u64,
    },
    Finished(Result<String, String>),
    Cancelled,
}

/// Shared between the task list and the thread doing the work
#[derive(Debug, Clone, Default)]
pub struct Control(Arc<Flags>);

#[derive(Debug, Default)]
struct Flags {
    cancel: AtomicBool,
    pause: AtomicBool,
}

impl Control {
    pub fn cancel(&self) {
        self.0.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.cancel.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.0.pause.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.0.pause.load(Ordering::Relaxed)
    }

    /// Waits while the task is paused; fails once it is cancelled
    pub fn checkpoint(&self) -> io::Result<()> {
        while self.is_paused() && !self.is_cancelled() {
            std::thread::sleep(Duration::from_millis(100));
        }

        if self.is_cancelled() {
            Err(io::Error::other("cancelled"))
        } else {
            Ok(())
        }
    }
}

/// Hands progress from the worker thread to the UI, a few times per second
pub struct Reporter {
    tx: Sender<Event>,
    control: Control,
    last_sent: Option<Instant>,
}

impl Reporter {
    const INTERVAL: Duration = Duration::from_millis(200);

    pub fn step(&self, label: impl Into<String>) {
        let _ = self.tx.send_blocking(Event::Step(label.into()));
    }

    /// Also the place where a task is paused or cancelled
    pub fn progress(&mut self, done: u64, total: u64) -> io::Result<()> {
        self.control.checkpoint()?;

        if done >= total || self.last_sent.is_none_or(|t| t.elapsed() >= Self::INTERVAL) {
            // a dropped update is fine, the next one follows shortly
            let _ = self.tx.try_send(Event::Progress { done, total });
            self.last_sent = Some(Instant::now());
        }

        Ok(())
    }
}

/// Runs blocking work on its own thread; `cleanup` runs when it fails or is cancelled
pub fn spawn(
    control: Control,
    work: impl FnOnce(&mut Reporter) -> Result<String> + Send + 'static,
    cleanup: impl FnOnce() + Send + 'static,
) -> impl Stream<Item = Event> {
    let (tx, rx) = smol::channel::bounded(16);

    std::thread::spawn(move || {
        let mut reporter = Reporter {
            tx: tx.clone(),
            control: control.clone(),
            last_sent: None,
        };

        let result = work(&mut reporter);
        if result.is_err() {
            cleanup();
        }

        let event = match result {
            Ok(msg) => Event::Finished(Ok(msg)),
            Err(_) if control.is_cancelled() => Event::Cancelled,
            Err(e) => Event::Finished(Err(format!("{e:#}"))),
        };

        let _ = tx.send_blocking(event);
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol::stream::StreamExt;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn cancel_stops_the_work_and_cleans_up() {
        let control = Control::default();
        let cleaned_up = Arc::new(AtomicBool::new(false));
        let reached = Arc::new(AtomicU64::new(0));

        let events = spawn(
            control.clone(),
            {
                let reached = reached.clone();
                move |reporter| {
                    for done in 0..1000 {
                        reporter.progress(done, 1000)?;
                        reached.store(done, Ordering::Relaxed);
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Ok("finished".into())
                }
            },
            {
                let cleaned_up = cleaned_up.clone();
                move || cleaned_up.store(true, Ordering::Relaxed)
            },
        );

        control.set_paused(true);
        std::thread::sleep(Duration::from_millis(100));
        let paused_at = reached.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(reached.load(Ordering::Relaxed), paused_at);

        control.cancel();
        let events = smol::block_on(events.collect::<Vec<_>>());

        assert!(matches!(events.last(), Some(Event::Cancelled)));
        assert!(cleaned_up.load(Ordering::Relaxed));
        assert!(reached.load(Ordering::Relaxed) < 999);
    }
}
