// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    games::game::Game,
    tasks::{self, Reporter, TaskControl, WorkerEvent},
    util::sha1_list,
};
use anyhow::{Context, Result};
use nod::{
    common::PartitionKind,
    read::{DiscOptions, DiscReader},
    write::{DiscWriter, FormatOptions, ProcessOptions},
};
use smol::stream::Stream;

/// What the hash says about a dump.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// The SHA-1 is in the list of known good dumps (Redump).
    Known,
    /// A Wii dump without its update partition (WBFS tools often drop it): it can't match the
    /// disc any more, whether or not the rest is intact.
    UpdateRemoved,
    /// Not in the list: damaged, another version, or changed in a way the hash can't tell.
    Unknown,
}

/// Whether a Wii disc's partitions lack the update partition; GameCube discs have none at all.
fn lacks_update_partition(kinds: &[PartitionKind]) -> bool {
    !kinds.is_empty() && !kinds.contains(&PartitionKind::Update)
}

fn perform_blocking(
    game: &Game,
    control: &TaskControl,
    reporter: &mut Reporter,
) -> Result<Verdict> {
    let disc_path = game.get_disc_path_blocking().context("disc not found")?;
    let message = || format!("Hashing {}", game.title());
    reporter.progress(0, None, message);

    let disc = DiscReader::new(&disc_path, &DiscOptions::default())?;
    let kinds = disc.partitions().iter().map(|p| p.kind).collect::<Vec<_>>();

    let process_opts = ProcessOptions {
        digest_sha1: true,
        ..Default::default()
    };

    let writer = DiscWriter::new(disc, &FormatOptions::default())?;

    let finalization = writer.process(
        |_, progress, total| {
            control.checkpoint().map_err(std::io::Error::other)?;
            reporter.progress(progress, Some(total), message);
            Ok(())
        },
        &process_opts,
    );

    // nod wraps the error; the task manager knows it was cancelled anyway
    control.checkpoint()?;
    let sha1 = finalization?.sha1.context("Failed to calculate SHA1")?;

    Ok(if sha1_list::is_known(&sha1) {
        Verdict::Known
    } else if lacks_update_partition(&kinds) {
        Verdict::UpdateRemoved
    } else {
        Verdict::Unknown
    })
}

/// Checks the game's SHA-1 against the list of known good dumps as a task.
pub fn calc_sha1(game: Game, control: TaskControl) -> impl Stream<Item = WorkerEvent> {
    tasks::spawn_blocking(
        move |reporter| match perform_blocking(&game, &control, reporter) {
            Ok(Verdict::Known) => Ok(format!(
                "Hash match for {}!  -  SHA1 is well known, your dump is perfect",
                game.title()
            )),
            // the task's title already names the game
            Ok(Verdict::UpdateRemoved) => Err("Hash mismatch: the dump has no update partition \
                 (often removed when making a WBFS), so it can't match the disc. That alone \
                 doesn't stop the game from working; whether the rest is intact, the hash can't \
                 tell"
                .to_string()),
            Ok(Verdict::Unknown) => Err("Hash mismatch: not a known good dump. It may be \
                 damaged, another version or changed otherwise; the hash can't tell which"
                .to_string()),
            Err(e) => Err(e.to_string()),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_wii_disc_without_update_partition_counts_as_shortened() {
        assert!(lacks_update_partition(&[PartitionKind::Data]));
        assert!(!lacks_update_partition(&[
            PartitionKind::Update,
            PartitionKind::Data
        ]));
        // GameCube: no partitions at all
        assert!(!lacks_update_partition(&[]));
    }
}
