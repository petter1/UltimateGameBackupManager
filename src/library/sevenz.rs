// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! 7z archives, read in the app with `sevenz-rust2` (pure Rust, only LZMA/LZMA2), as ps5upload
//! does (https://github.com/phantomptr/ps5upload, `ps5upload-core/src/archive_extract.rs`); an
//! archive with another codec or a password fails here, and 7-Zip on the computer is asked
//! instead ([`super::archives`]).

use super::archives::Entry;
use anyhow::Result;
use sevenz_rust2::{Archive, ArchiveReader, Password};
use std::{fs::File, io::BufReader, path::Path};

/// What is in a 7z archive, from its header (nothing is unpacked)
pub fn entries(archive: &Path) -> Result<Vec<Entry>> {
    let header = Archive::read(
        &mut BufReader::new(File::open(archive)?),
        &Password::empty(),
    )?;
    Ok(header
        .files
        .iter()
        .map(|file| Entry {
            path: file.name().to_string(),
            size: file.size(),
            is_dir: file.is_directory(),
        })
        .collect())
}

/// The data of the files `members` (paths as [`entries`] gave them) one after the other; None
/// when it is longer than `max_len`
pub fn read(archive: &Path, members: &[&str], max_len: u64) -> Result<Option<Vec<u8>>> {
    let mut reader = ArchiveReader::new(BufReader::new(File::open(archive)?), Password::empty())?;
    let size: u64 = members
        .iter()
        .map(|name| {
            reader
                .archive()
                .files
                .iter()
                .find(|file| file.name() == *name)
                .map_or(0, |file| file.size())
        })
        .sum();
    if size > max_len {
        return Ok(None);
    }

    let mut data = Vec::new();
    for name in members {
        data.extend(reader.read_file(name)?);
    }
    Ok((data.len() as u64 <= max_len).then_some(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sevenz_rust2::ArchiveWriter;

    #[test]
    fn a_7z_is_listed_and_read_without_7_zip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.7z");
        let mut writer = ArchiveWriter::create(&path).unwrap();
        for (name, data) in [
            ("Game/meta/meta.xml", &b"<menu/>"[..]),
            ("Game/rom.gba", b"ROM"),
        ] {
            writer
                .push_archive_entry(
                    sevenz_rust2::ArchiveEntry::new_file(name),
                    Some(std::io::Cursor::new(data)),
                )
                .unwrap();
        }
        writer.finish().unwrap();

        let listed = entries(&path).unwrap();
        let names: Vec<&str> = listed.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(names, ["Game/meta/meta.xml", "Game/rom.gba"]);
        assert_eq!(listed[1].size, 3);

        assert_eq!(
            read(&path, &["Game/rom.gba", "Game/meta/meta.xml"], 100).unwrap(),
            Some(b"ROM<menu/>".to_vec())
        );
        assert_eq!(read(&path, &["Game/rom.gba"], 2).unwrap(), None);
    }
}
