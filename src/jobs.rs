// SPDX-FileCopyrightText: 2026 Ultimate Game Backup Manager contributors
// SPDX-License-Identifier: GPL-3.0-only

//! What the tasks in the task manager do. A job is kept until its task leaves the history, so a
//! failed or cancelled task can be retried.

use crate::{
    config::{Config, ConfigContents},
    games::{
        ImportEntry,
        calc_sha1::calc_sha1,
        convert::convert_game,
        export::export_game,
        game::Game,
        import::{self, import_game},
        scrub::{self, scrub_game},
    },
    tasks::{Resource, TaskControl, WorkerEvent},
    util::drive_state::DriveState,
};
use serde::{Deserialize, Serialize};
use smol::stream::{Stream, StreamExt};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub enum Job {
    /// A Wii or GameCube game to the drive. The drive and its settings are those of the moment
    /// the game was added, so choosing another drive meanwhile doesn't change where it goes.
    Import {
        entry: ImportEntry,
        config: Config,
        drive: DriveState,
    },
    /// A disc image into another format, from one file to another (Toolbox).
    Convert {
        disc_path: PathBuf,
        out_path: PathBuf,
    },
    /// A game from the drive at `mount_point` to a file.
    Export {
        game: Game,
        out_path: PathBuf,
        mount_point: PathBuf,
    },
    /// Removes the update partition of a Wii game on the drive at `mount_point`.
    Scrub { game: Game, mount_point: PathBuf },
    /// Checks a game's checksum against the known good dumps; only reads.
    Hash { game: Game, mount_point: PathBuf },
}

/// The folder a file is in, standing for its drive: the task manager doesn't know mount points
/// of arbitrary paths, so two folders on one disk count as two resources.
fn folder_of(path: &Path) -> Resource {
    Resource::Drive(path.parent().unwrap_or(path).to_path_buf())
}

impl Job {
    pub fn title(&self) -> String {
        match self {
            Self::Import { entry, .. } => format!("Import {}", entry.display_title()),
            Self::Convert { out_path, .. } => format!(
                "Convert into {}",
                out_path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default()
            ),
            Self::Export { game, .. } => format!("Export {}", game.title()),
            Self::Scrub { game, .. } => format!("Scrub {}", game.title()),
            Self::Hash { game, .. } => format!("Check the hash of {}", game.title()),
        }
    }

    pub fn resources(&self) -> Vec<Resource> {
        match self {
            Self::Import { config, .. } => {
                vec![Resource::Drive(config.mount_point().to_path_buf())]
            }
            Self::Convert {
                disc_path,
                out_path,
            } => {
                let mut resources = vec![folder_of(disc_path), folder_of(out_path)];
                resources.dedup();
                resources
            }
            Self::Export {
                out_path,
                mount_point,
                ..
            } => vec![Resource::Drive(mount_point.clone()), folder_of(out_path)],
            Self::Scrub { mount_point, .. } | Self::Hash { mount_point, .. } => {
                vec![Resource::Drive(mount_point.clone())]
            }
        }
    }

    /// The drive whose list of games changes when the job is done, so it is reloaded if shown.
    pub fn changes_games_on(&self) -> Option<&Path> {
        match self {
            Self::Import { config, .. } => Some(config.mount_point()),
            Self::Scrub { mount_point, .. } => Some(mount_point),
            Self::Convert { .. } | Self::Export { .. } | Self::Hash { .. } => None,
        }
    }

    /// Whether this job scrubs the game in `game_dir`.
    pub fn scrubs(&self, game_dir: &Path) -> bool {
        matches!(self, Self::Scrub { game, .. } if game.path() == game_dir)
    }

    pub fn run(self, control: TaskControl) -> impl Stream<Item = WorkerEvent> {
        match self {
            Self::Import {
                entry,
                config,
                drive,
            } => import_game(entry, config, drive, control).boxed(),
            Self::Convert {
                disc_path,
                out_path,
            } => convert_game(disc_path, out_path, control).boxed(),
            Self::Export { game, out_path, .. } => export_game(game, out_path, control).boxed(),
            Self::Scrub { game, .. } => scrub_game(game, control).boxed(),
            Self::Hash { game, .. } => calc_sha1(game, control).boxed(),
        }
    }
}

/// A job as it is saved in the data folder, so unfinished tasks can be resumed after a restart.
/// Only paths and settings: the games and discs are read again when it is resumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SavedJob {
    Import {
        source: PathBuf,
        config: ConfigContents,
    },
    Convert {
        disc_path: PathBuf,
        out_path: PathBuf,
    },
    Export {
        game_path: PathBuf,
        is_wii: bool,
        out_path: PathBuf,
        mount_point: PathBuf,
    },
    Scrub {
        game_path: PathBuf,
        is_wii: bool,
        mount_point: PathBuf,
    },
    Hash {
        game_path: PathBuf,
        is_wii: bool,
        mount_point: PathBuf,
    },
}

/// What a job that was cut off by quitting may have half-written, removed before it is resumed
/// or dropped. Cancelling cleans up by itself; quitting doesn't give the worker the chance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Leftovers {
    Nothing,
    File(PathBuf),
    /// The scrubbed copy of a game: removed, unless quitting came after the game's folder was
    /// removed and before the copy took its place; then the copy is the game and is moved back.
    ScrubCopy {
        game_dir: PathBuf,
        copy: PathBuf,
    },
    /// The files in `dir` whose names begin with `prefix`, and `dir` itself once it is empty (an
    /// import: the folder may already hold the other disc of the game).
    Files {
        dir: PathBuf,
        prefix: String,
    },
}

impl Leftovers {
    pub fn remove_blocking(&self) {
        match self {
            Self::Nothing => {}
            Self::File(path) => {
                let _ = std::fs::remove_file(path);
            }
            Self::ScrubCopy { game_dir, copy } => {
                if game_dir.exists() {
                    let _ = std::fs::remove_dir_all(copy);
                } else {
                    let _ = std::fs::rename(copy, game_dir);
                }
            }
            Self::Files { dir, prefix } => {
                let Ok(entries) = std::fs::read_dir(dir) else {
                    return;
                };
                for entry in entries.flatten() {
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(prefix.as_str())
                    {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
                // fails while the folder holds anything else, which is what should stay
                let _ = std::fs::remove_dir(dir);
            }
        }
    }
}

/// An unfinished task in `tasks.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTask {
    pub title: String,
    pub job: SavedJob,
    /// Set once the task has run, so it may have left something half-written.
    pub leftovers: Option<Leftovers>,
}

impl SavedTask {
    /// What to remove before the task is resumed or dropped. Nothing for an import whose source
    /// is gone: it was removed after a complete import ("remove the source"), and the app was
    /// closed before the task was marked done, so the files on the drive are the only copy.
    pub fn leftovers_to_remove(&self) -> Option<&Leftovers> {
        if let SavedJob::Import { source, .. } = &self.job
            && !source.exists()
        {
            return None;
        }

        self.leftovers.as_ref()
    }
}

const TASKS_FILENAME: &str = "tasks.json";

/// The tasks that weren't finished when the app was closed last. A task that can't be read (a
/// kind this version doesn't know, or a damaged file) is left out with a warning, not the others.
pub async fn load_unfinished(data_dir: &Path) -> Vec<SavedTask> {
    let Ok(contents) = smol::fs::read_to_string(data_dir.join(TASKS_FILENAME)).await else {
        return Vec::new();
    };

    parse_unfinished(&contents)
}

fn parse_unfinished(contents: &str) -> Vec<SavedTask> {
    let tasks: Vec<serde_json::Value> = match serde_json::from_str(contents) {
        Ok(tasks) => tasks,
        Err(e) => {
            eprintln!("WARN: Could not read the unfinished tasks in {TASKS_FILENAME}: {e}");
            return Vec::new();
        }
    };

    tasks
        .into_iter()
        .filter_map(|task| match serde_json::from_value(task) {
            Ok(task) => Some(task),
            Err(e) => {
                eprintln!("WARN: Left out an unfinished task in {TASKS_FILENAME}: {e}");
                None
            }
        })
        .collect()
}

/// Writes the unfinished tasks. Blocking, but small and only when they change; writing in the
/// background could let an older list overtake a newer one.
pub fn save_unfinished_blocking(data_dir: &Path, json: &str) -> std::io::Result<()> {
    let path = data_dir.join(TASKS_FILENAME);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)
}

impl Job {
    pub fn to_saved(&self) -> SavedJob {
        match self {
            Self::Import { entry, config, .. } => SavedJob::Import {
                source: entry.path().to_path_buf(),
                config: config.contents().clone(),
            },
            Self::Convert {
                disc_path,
                out_path,
            } => SavedJob::Convert {
                disc_path: disc_path.clone(),
                out_path: out_path.clone(),
            },
            Self::Export {
                game,
                out_path,
                mount_point,
            } => SavedJob::Export {
                game_path: game.path().to_path_buf(),
                is_wii: game.is_wii(),
                out_path: out_path.clone(),
                mount_point: mount_point.clone(),
            },
            Self::Scrub { game, mount_point } => SavedJob::Scrub {
                game_path: game.path().to_path_buf(),
                is_wii: game.is_wii(),
                mount_point: mount_point.clone(),
            },
            Self::Hash { game, mount_point } => SavedJob::Hash {
                game_path: game.path().to_path_buf(),
                is_wii: game.is_wii(),
                mount_point: mount_point.clone(),
            },
        }
    }

    /// What this job writes before it is done; see [`Leftovers`].
    pub fn leftovers(&self) -> Leftovers {
        match self {
            Self::Import { entry, config, .. } => Leftovers::Files {
                dir: import::game_dir(entry, config.mount_point()),
                prefix: import::file_prefix(entry),
            },
            Self::Convert { out_path, .. } | Self::Export { out_path, .. } => {
                Leftovers::File(out_path.clone())
            }
            Self::Scrub { game, .. } => Leftovers::ScrubCopy {
                game_dir: game.path().to_path_buf(),
                copy: scrub::tmp_game_dir(game),
            },
            Self::Hash { .. } => Leftovers::Nothing,
        }
    }
}

impl SavedJob {
    /// Reads the game or disc again; fails when it is gone or the drive isn't connected.
    pub async fn restore(self) -> Result<Job, String> {
        let job = match self {
            Self::Import { source, config } => {
                let config = Config::from_contents(config);
                let mount_point = config.mount_point().to_path_buf();
                if !mount_point.is_dir() {
                    return Err(format!("{} is not connected", mount_point.display()));
                }
                let entry = ImportEntry::new(source).await.map_err(|e| e.to_string())?;
                let drive = DriveState::load(mount_point).await;
                Job::Import {
                    entry,
                    config,
                    drive,
                }
            }
            Self::Convert {
                disc_path,
                out_path,
            } => Job::Convert {
                disc_path,
                out_path,
            },
            Self::Export {
                game_path,
                is_wii,
                out_path,
                mount_point,
            } => Job::Export {
                game: Game::try_from_path(game_path, is_wii)
                    .await
                    .map_err(|e| e.to_string())?,
                out_path,
                mount_point,
            },
            Self::Scrub {
                game_path,
                is_wii,
                mount_point,
            } => Job::Scrub {
                game: Game::try_from_path(game_path, is_wii)
                    .await
                    .map_err(|e| e.to_string())?,
                mount_point,
            },
            Self::Hash {
                game_path,
                is_wii,
                mount_point,
            } => Job::Hash {
                game: Game::try_from_path(game_path, is_wii)
                    .await
                    .map_err(|e| e.to_string())?,
                mount_point,
            },
        };

        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn an_import_leaves_the_other_disc() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("games/Game [GXXE01]");
        fs::create_dir_all(&game_dir).unwrap();
        fs::write(game_dir.join("game.iso"), "disc 1").unwrap();
        fs::write(game_dir.join("disc2.iso"), "half").unwrap();

        let leftovers = Leftovers::Files {
            dir: game_dir.clone(),
            prefix: "disc2.".into(),
        };
        leftovers.remove_blocking();
        assert!(game_dir.join("game.iso").exists());
        assert!(!game_dir.join("disc2.iso").exists());

        // the first disc alone: the folder goes too
        Leftovers::Files {
            dir: game_dir.clone(),
            prefix: "game.".into(),
        }
        .remove_blocking();
        assert!(!game_dir.exists());
    }

    #[test]
    fn a_complete_scrub_copy_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("Game [RXXE01]");
        let copy = dir.path().join("Game [RXXE01] SCRUB");
        let leftovers = Leftovers::ScrubCopy {
            game_dir: game_dir.clone(),
            copy: copy.clone(),
        };

        // quit while writing the copy: the game stays, the copy goes
        fs::create_dir(&game_dir).unwrap();
        fs::create_dir(&copy).unwrap();
        leftovers.remove_blocking();
        assert!(game_dir.exists());
        assert!(!copy.exists());

        // quit after the game was removed: the copy becomes the game
        fs::remove_dir(&game_dir).unwrap();
        fs::create_dir(&copy).unwrap();
        fs::write(copy.join("RXXE01.wbfs"), "scrubbed").unwrap();
        leftovers.remove_blocking();
        assert!(game_dir.join("RXXE01.wbfs").exists());
        assert!(!copy.exists());
    }

    #[test]
    fn saved_tasks_are_loaded_and_restored() {
        let dir = tempfile::tempdir().unwrap();
        let saved = vec![SavedTask {
            title: "Convert into game.rvz".into(),
            job: SavedJob::Convert {
                disc_path: "/a/game.iso".into(),
                out_path: "/b/game.rvz".into(),
            },
            leftovers: Some(Leftovers::File("/b/game.rvz".into())),
        }];
        let json = serde_json::to_string(&saved).unwrap();
        save_unfinished_blocking(dir.path(), &json).unwrap();

        let loaded = smol::block_on(load_unfinished(dir.path()));
        assert_eq!(serde_json::to_string(&loaded).unwrap(), json);

        let job = smol::block_on(loaded[0].job.clone().restore()).unwrap();
        assert_eq!(job.title(), "Convert into game.rvz");
        assert_eq!(
            serde_json::to_string(&job.to_saved()).unwrap(),
            serde_json::to_string(&saved[0].job).unwrap()
        );
    }

    #[test]
    fn an_unknown_task_leaves_the_others() {
        let json = r#"[
            {"title": "Install", "job": {"Install": {"cia": "/a.cia"}}, "leftovers": null},
            {"title": "Convert", "job": {"Convert": {"disc_path": "/a.iso", "out_path": "/b.rvz"}},
             "leftovers": null}
        ]"#;
        let loaded = parse_unfinished(json);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "Convert");

        assert!(parse_unfinished("not json").is_empty());
    }

    #[test]
    fn an_import_without_its_source_keeps_its_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("game.iso");
        let config: ConfigContents = serde_json::from_str("{}").unwrap();
        let task = SavedTask {
            title: "Import".into(),
            job: SavedJob::Import {
                source: source.clone(),
                config,
            },
            leftovers: Some(Leftovers::File(dir.path().join("out"))),
        };
        assert!(task.leftovers_to_remove().is_none());

        fs::write(&source, "disc").unwrap();
        assert!(task.leftovers_to_remove().is_some());
    }

    #[test]
    fn an_import_from_a_missing_drive_is_not_restored() {
        let config: ConfigContents =
            serde_json::from_str(r#"{"mount_point": "/Volumes/Not connected"}"#).unwrap();
        let job = SavedJob::Import {
            source: "/games/game.iso".into(),
            config,
        };
        let error = smol::block_on(job.restore()).unwrap_err();
        assert!(error.contains("is not connected"), "{error}");
    }
}
