use crate::{
    games::{
        disc_reader::{SharedMultiFileReader, retrieve_multi_disc_files},
        game::Game,
        import::{SPLIT_SIZE, get_filename},
    },
    tasks::worker::{self, Control, Event, Reporter},
    util::misc::OPTIMAL_THREADS,
};
use anyhow::{Context, Result, anyhow};
use nod::{
    common::Format,
    read::{DiscOptions, DiscReader},
    write::{DiscWriter, FormatOptions, ProcessOptions, ScrubLevel},
};
use smol::stream::Stream;
use split_write::SplitWriter;
use std::{
    ffi::OsStr,
    io::{BufWriter, Write},
    path::PathBuf,
};

fn tmp_game_dir(game: &Game) -> Result<PathBuf> {
    let game_dir_name = game
        .path()
        .file_name()
        .and_then(OsStr::to_str)
        .context("invalid path")?;

    Ok(game.path().with_file_name(format!("{game_dir_name} SCRUB")))
}

fn perform_blocking(game: &Game, reporter: &mut Reporter) -> Result<()> {
    let disc_path = game.get_disc_path_blocking().context("disc not found")?;

    let tmp_game_dir = tmp_game_dir(game)?;
    std::fs::create_dir(&tmp_game_dir)?;

    let format_opts = FormatOptions::new(Format::Wbfs);

    let process_opts = ProcessOptions {
        processor_threads: OPTIMAL_THREADS.processor,
        scrub: ScrubLevel::UpdatePartition,
        digest_crc32: true,
        digest_md5: false,
        digest_sha1: true,
        digest_xxh64: true,
    };

    {
        let files = retrieve_multi_disc_files(&disc_path)?;

        let must_split = files.len() > 1;
        let split_size = must_split.then_some(SPLIT_SIZE);

        let disc_reader = DiscReader::new_stream(
            Box::new(SharedMultiFileReader::new(files)?),
            &DiscOptions {
                preloader_threads: OPTIMAL_THREADS.preloader,
                ..Default::default()
            },
        )?;

        let disc_writer = DiscWriter::new(disc_reader, &format_opts)?;

        let out_writer = SplitWriter::create(
            &tmp_game_dir,
            |part| {
                get_filename(
                    game.id(),
                    true,
                    part,
                    0,
                    Format::Wbfs,
                    Format::Iso,
                    must_split,
                )
            },
            split_size,
        )?;
        let mut out_writer = BufWriter::new(out_writer);

        let finalization = disc_writer.process(
            |data, progress, total| {
                out_writer.write_all(&data)?;
                reporter.progress(progress, total)
            },
            &process_opts,
        )?;

        let mut out_writer = out_writer
            .into_inner()
            .map_err(|_| anyhow!("Failed to get inner writer"))?;

        if !finalization.header.is_empty() {
            out_writer.write_header(&finalization.header)?;
        }

        out_writer.flush()?;
    }

    std::fs::remove_dir_all(game.path())?;
    std::fs::rename(tmp_game_dir, game.path())?;

    Ok(())
}

pub fn scrub_game(game: Game, control: Control) -> impl Stream<Item = Event> {
    // the game itself is only replaced once the scrubbed copy is complete
    let partial = tmp_game_dir(&game).ok();

    worker::spawn(
        control,
        move |reporter| {
            perform_blocking(&game, reporter)?;
            Ok(format!("Scrubbed {}", game.title()))
        },
        move || {
            if let Some(partial) = partial {
                let _ = std::fs::remove_dir_all(partial);
            }
        },
    )
}
