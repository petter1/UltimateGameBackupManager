// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    games::game::Game,
    tasks::worker::{self, Control, Event, Reporter},
    util::misc::OPTIMAL_THREADS,
};
use anyhow::{Context, Result};
use nod::{
    common::Format,
    read::{DiscOptions, DiscReader},
    write::{DiscWriter, FormatOptions, ProcessOptions, ScrubLevel},
};
use smol::stream::Stream;
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Seek, Write},
    path::PathBuf,
};

pub fn ext_to_format(ext: &str) -> Option<Format> {
    match ext {
        "iso" | "ISO" => Some(Format::Iso),
        "ciso" | "CISO" => Some(Format::Ciso),
        "gcz" | "CGZ" => Some(Format::Gcz),
        "rvz" | "RVZ" => Some(Format::Rvz),
        "wbfs" | "WBFS" => Some(Format::Wbfs),
        "wia" | "WIA" => Some(Format::Wia),
        "tgc" | "TGC" => Some(Format::Tgc),
        _ => None,
    }
}

fn perform_blocking(game: &Game, out_path: PathBuf, reporter: &mut Reporter) -> Result<()> {
    let disc_path = game.get_disc_path_blocking().context("disc not found")?;

    let ext = out_path
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ext_to_format)
        .context("invalid extension")?;

    let format_opts = FormatOptions::new(ext);

    let disc_reader = DiscReader::new(
        &disc_path,
        &DiscOptions {
            preloader_threads: OPTIMAL_THREADS.preloader,
            ..Default::default()
        },
    )?;

    let disc_writer = DiscWriter::new(disc_reader, &format_opts)?;
    reporter.step(format!("Writing {}", out_path.display()));
    let mut out_writer = {
        let file = File::create(out_path)?;
        BufWriter::new(file)
    };

    let finalization = disc_writer.process(
        |data, progress, total| {
            out_writer.write_all(&data)?;
            reporter.progress(progress, total)
        },
        &ProcessOptions {
            processor_threads: OPTIMAL_THREADS.processor,
            scrub: ScrubLevel::None,
            digest_crc32: true,
            digest_md5: false,
            digest_sha1: true,
            digest_xxh64: true,
        },
    )?;

    if !finalization.header.is_empty() {
        out_writer.rewind()?;
        out_writer.write_all(&finalization.header)?;
    }

    out_writer.flush()?;
    Ok(())
}

pub fn export_game(game: Game, out_path: PathBuf, control: Control) -> impl Stream<Item = Event> {
    let partial = out_path.clone();

    worker::spawn(
        control,
        move |reporter| {
            perform_blocking(&game, out_path, reporter)?;
            Ok(format!("Exported {}", game.title()))
        },
        move || {
            let _ = std::fs::remove_file(partial);
        },
    )
}
