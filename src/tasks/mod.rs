// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Every longer action is a task in one list. Tasks run side by side, but each
//! resource – a disk, later a console's connection – works off one task at a time.

pub mod worker;

use crate::{
    config::Config,
    games::{
        ImportEntry, calc_sha1::calc_sha1, convert::convert_game, export::export_game, game::Game,
        import::import_game, scrub::scrub_game,
    },
    util::{drive_state::DriveState, fs::volume_of},
};
use lucide_icons::Icon;
use smol::stream::Stream;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    pin::Pin,
    time::{Duration, Instant},
};
use worker::{Control, Event};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

/// What a task needs for itself while it runs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Disk(PathBuf),
}

#[derive(Debug, Clone)]
pub enum Job {
    Import {
        entry: ImportEntry,
        config: Config,
        drive: DriveState,
    },
    Convert {
        disc: PathBuf,
        out: PathBuf,
    },
    Export {
        game: Game,
        out: PathBuf,
    },
    Scrub(Game),
    Hash(Game),
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .map_or_else(|| path.display().to_string(), str::to_string)
}

impl Job {
    pub fn label(&self) -> String {
        match self {
            Job::Import { entry, config, .. } => format!(
                "Copy {} to {}",
                entry.meta().game_title(),
                file_name(config.mount_point())
            ),
            Job::Convert { disc, out } => {
                format!("Convert {} to {}", file_name(disc), file_name(out))
            }
            Job::Export { game, out } => format!("Export {} to {}", game.title(), file_name(out)),
            Job::Scrub(game) => format!("Scrub {}", game.title()),
            Job::Hash(game) => format!("Verify {}", game.title()),
        }
    }

    pub fn icon(&self) -> Icon {
        match self {
            Job::Import { .. } => Icon::HardDriveDownload,
            Job::Convert { .. } => Icon::FileCog,
            Job::Export { .. } => Icon::HardDriveUpload,
            Job::Scrub(_) => Icon::Shredder,
            Job::Hash(_) => Icon::ShieldCheck,
        }
    }

    /// Whether the games on the drive change when it is done
    pub fn changes_drive(&self) -> bool {
        matches!(self, Job::Import { .. } | Job::Scrub(_))
    }

    fn resources(&self) -> Vec<Resource> {
        let paths: Vec<&Path> = match self {
            Job::Import { entry, config, .. } => vec![entry.path(), config.mount_point()],
            Job::Convert { disc, out } => vec![disc, out],
            Job::Export { game, out } => vec![game.path(), out],
            Job::Scrub(game) | Job::Hash(game) => vec![game.path()],
        };

        let mut resources = Vec::new();
        for path in paths {
            let resource = Resource::Disk(volume_of(path));
            if !resources.contains(&resource) {
                resources.push(resource);
            }
        }

        resources
    }

    fn run(self, control: Control) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        match self {
            Job::Import {
                entry,
                config,
                drive,
            } => Box::pin(import_game(entry, config, drive, control)),
            Job::Convert { disc, out } => Box::pin(convert_game(disc, out, control)),
            Job::Export { game, out } => Box::pin(export_game(game, out, control)),
            Job::Scrub(game) => Box::pin(scrub_game(game, control)),
            Job::Hash(game) => Box::pin(calc_sha1(game, control)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Running,
    Done(String),
    Failed(String),
    Cancelled,
}

impl Status {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            Status::Done(_) | Status::Failed(_) | Status::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
    /// Bytes per second, smoothed
    pub speed: Option<f64>,
}

impl Progress {
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.done as f64 / self.total as f64) as f32
        }
    }

    pub fn time_left(&self) -> Option<Duration> {
        let speed = self.speed.filter(|&speed| speed > 0.0)?;
        Some(Duration::from_secs_f64(
            self.total.saturating_sub(self.done) as f64 / speed,
        ))
    }
}

#[derive(Debug)]
pub struct TaskEntry {
    pub id: TaskId,
    pub job: Job,
    pub status: Status,
    pub step: Option<String>,
    pub progress: Option<Progress>,
    control: Control,
    resources: Vec<Resource>,
    last_sample: Option<(Instant, u64)>,
}

impl TaskEntry {
    pub fn is_paused(&self) -> bool {
        self.status == Status::Running && self.control.is_paused()
    }

    pub fn is_cancelling(&self) -> bool {
        self.status == Status::Running && self.control.is_cancelled()
    }

    fn record_progress(&mut self, done: u64, total: u64) {
        let now = Instant::now();
        let mut speed = self.progress.and_then(|p| p.speed);

        match self.last_sample {
            Some((at, before)) if now.duration_since(at) >= Duration::from_millis(500) => {
                let current =
                    done.saturating_sub(before) as f64 / now.duration_since(at).as_secs_f64();
                speed = Some(speed.map_or(current, |speed| 0.7 * speed + 0.3 * current));
                self.last_sample = Some((now, done));
            }
            Some(_) => {}
            None => self.last_sample = Some((now, done)),
        }

        self.progress = Some(Progress { done, total, speed });
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TaskAction {
    Cancel,
    Pause,
    Resume,
    Retry,
    MoveUp,
    MoveDown,
}

/// A task that may start now, with what it needs to run
pub type Ready = (TaskId, Pin<Box<dyn Stream<Item = Event> + Send>>);

#[derive(Debug, Default)]
pub struct Tasks {
    entries: Vec<TaskEntry>,
    next_id: u64,
}

impl Tasks {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &TaskEntry> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn get_mut(&mut self, id: TaskId) -> Option<&mut TaskEntry> {
        self.entries.iter_mut().find(|entry| entry.id == id)
    }

    pub fn count(&self, status: &Status) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == *status)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.status, Status::Failed(_)))
            .count()
    }

    /// Bytes done and in total of the running tasks
    pub fn overall_progress(&self) -> Option<Progress> {
        let mut overall = Progress {
            done: 0,
            total: 0,
            speed: None,
        };

        for progress in self
            .entries
            .iter()
            .filter(|entry| entry.status == Status::Running)
            .filter_map(|entry| entry.progress)
        {
            overall.done += progress.done;
            overall.total += progress.total;
            if let Some(speed) = progress.speed {
                overall.speed = Some(overall.speed.unwrap_or(0.0) + speed);
            }
        }

        (overall.total > 0).then_some(overall)
    }

    /// Queues a job, unless the same one is already waiting or running
    pub fn add(&mut self, job: Job) -> Option<TaskId> {
        let label = job.label();
        if self
            .entries
            .iter()
            .any(|entry| !entry.status.is_finished() && entry.job.label() == label)
        {
            return None;
        }

        let id = TaskId(self.next_id);
        self.next_id += 1;

        self.entries.push(TaskEntry {
            id,
            resources: job.resources(),
            job,
            status: Status::Waiting,
            step: None,
            progress: None,
            control: Control::default(),
            last_sample: None,
        });

        Some(id)
    }

    /// Starts every waiting task whose resources are free, in the order of the list
    pub fn start_ready(&mut self) -> Vec<Ready> {
        let mut busy = self
            .entries
            .iter()
            .filter(|entry| entry.status == Status::Running)
            .flat_map(|entry| entry.resources.clone())
            .collect::<Vec<_>>();

        let mut ready = Vec::new();

        for entry in &mut self.entries {
            if entry.status != Status::Waiting {
                continue;
            }

            let free = !entry.resources.iter().any(|r| busy.contains(r));

            // a waiting task keeps its place in the queue of each of its resources
            busy.extend(entry.resources.iter().cloned());

            if free {
                entry.status = Status::Running;
                entry.control = Control::default();
                ready.push((entry.id, entry.job.clone().run(entry.control.clone())));
            }
        }

        ready
    }

    /// Takes an event of a running task; returns the task once it has ended
    pub fn handle(&mut self, id: TaskId, event: Event) -> Option<&TaskEntry> {
        let entry = self.get_mut(id)?;

        match event {
            Event::Step(step) => {
                entry.step = Some(step);
                None
            }
            Event::Progress { done, total } => {
                entry.record_progress(done, total);
                None
            }
            Event::Finished(result) => {
                entry.status = match result {
                    Ok(msg) => Status::Done(msg),
                    Err(e) => Status::Failed(e),
                };
                Some(entry)
            }
            Event::Cancelled => {
                entry.status = Status::Cancelled;
                Some(entry)
            }
        }
    }

    pub fn apply(&mut self, id: TaskId, action: TaskAction) {
        let Some(i) = self.entries.iter().position(|entry| entry.id == id) else {
            return;
        };

        let entry = &mut self.entries[i];

        match action {
            TaskAction::Cancel => match entry.status {
                Status::Waiting => entry.status = Status::Cancelled,
                // the worker stops at its next checkpoint and cleans up
                Status::Running => entry.control.cancel(),
                _ => {}
            },
            TaskAction::Pause => entry.control.set_paused(true),
            TaskAction::Resume => {
                entry.control.set_paused(false);
                // the time paused doesn't count for the speed
                entry.last_sample = None;
            }
            TaskAction::Retry => {
                if matches!(entry.status, Status::Failed(_) | Status::Cancelled) {
                    entry.status = Status::Waiting;
                    entry.step = None;
                    entry.progress = None;
                    entry.last_sample = None;

                    // to the end of the queue
                    let entry = self.entries.remove(i);
                    self.entries.push(entry);
                }
            }
            TaskAction::MoveUp => {
                if let Some(j) = self.entries[..i]
                    .iter()
                    .rposition(|entry| entry.status == Status::Waiting)
                {
                    self.entries.swap(i, j);
                }
            }
            TaskAction::MoveDown => {
                if let Some(j) = self.entries[i + 1..]
                    .iter()
                    .position(|entry| entry.status == Status::Waiting)
                {
                    self.entries.swap(i, i + 1 + j);
                }
            }
        }
    }

    pub fn clear_finished(&mut self) {
        self.entries.retain(|entry| !entry.status.is_finished());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conversions on one temporary disk; their files don't exist, so they fail when run
    fn convert(dir: &Path, name: &str) -> Job {
        Job::Convert {
            disc: dir.join(format!("{name}.iso")),
            out: dir.join(format!("{name}.rvz")),
        }
    }

    fn statuses(tasks: &Tasks) -> Vec<Status> {
        tasks.iter().map(|entry| entry.status.clone()).collect()
    }

    #[test]
    fn one_task_at_a_time_per_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut tasks = Tasks::default();
        let a = tasks.add(convert(dir.path(), "a")).unwrap();
        tasks.add(convert(dir.path(), "b")).unwrap();

        assert_eq!(tasks.start_ready().len(), 1);
        assert_eq!(statuses(&tasks), [Status::Running, Status::Waiting]);

        tasks.handle(a, Event::Finished(Ok("done".into())));
        assert_eq!(tasks.start_ready().len(), 1);
        assert_eq!(
            statuses(&tasks),
            [Status::Done("done".into()), Status::Running]
        );
    }

    #[test]
    fn the_same_job_is_queued_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut tasks = Tasks::default();
        assert!(tasks.add(convert(dir.path(), "a")).is_some());
        assert!(tasks.add(convert(dir.path(), "a")).is_none());
    }

    #[test]
    fn cancel_retry_and_reorder() {
        let dir = tempfile::tempdir().unwrap();
        let mut tasks = Tasks::default();
        let a = tasks.add(convert(dir.path(), "a")).unwrap();
        let b = tasks.add(convert(dir.path(), "b")).unwrap();
        let c = tasks.add(convert(dir.path(), "c")).unwrap();

        // waiting tasks are cancelled at once
        tasks.apply(a, TaskAction::Cancel);
        assert_eq!(tasks.iter().next().unwrap().status, Status::Cancelled);

        tasks.apply(c, TaskAction::MoveUp);
        let order = tasks.iter().map(|entry| entry.id).collect::<Vec<_>>();
        assert_eq!(order, [a, c, b]);

        // a retried task goes to the end of the queue
        tasks.apply(a, TaskAction::Retry);
        let order = tasks.iter().map(|entry| entry.id).collect::<Vec<_>>();
        assert_eq!(order, [c, b, a]);
        assert_eq!(tasks.count(&Status::Waiting), 3);

        tasks.apply(a, TaskAction::Cancel);
        tasks.clear_finished();
        assert_eq!(tasks.iter().count(), 2);
    }

    #[test]
    fn a_failed_job_reports_its_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut tasks = Tasks::default();
        tasks.add(convert(dir.path(), "a")).unwrap();

        let (id, events) = tasks.start_ready().pop().unwrap();
        let events = smol::block_on(smol::stream::StreamExt::collect::<Vec<_>>(events));
        for event in events {
            tasks.handle(id, event);
        }

        assert!(matches!(
            tasks.iter().next().unwrap().status,
            Status::Failed(_)
        ));
    }
}
