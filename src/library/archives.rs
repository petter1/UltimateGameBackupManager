// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! RAR and 7z archives: listed and read by the app itself where it can, as ps5upload does
//! (https://github.com/phantomptr/ps5upload) – RAR through the separate `ugbm-unrar` program
//! ([`super::rar`]), 7z with `sevenz-rust2` ([`super::sevenz`]) – and with the 7-Zip on the
//! computer ([`super::sevenzip`]) when that fails (a codec `sevenz-rust2` doesn't have, a
//! missing `ugbm-unrar`).

use super::{rar, sevenz, sevenzip};
use std::path::Path;

/// A file or folder in an archive
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// the path as the archive names it, which is what it takes back to read the file
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

impl Entry {
    /// The path with `/` between folders on every system
    pub fn slash_path(&self) -> String {
        self.path.replace('\\', "/")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Rar,
    SevenZ,
}

fn kind(archive: &Path) -> Option<Kind> {
    let ext = archive.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rar" => Some(Kind::Rar),
        "7z" => Some(Kind::SevenZ),
        _ => None,
    }
}

/// What is in an archive, from its headers (nothing is unpacked). None when neither the app nor
/// 7-Zip can read it (damaged, its names encrypted, or RAR without `ugbm-unrar` and 7-Zip)
pub fn list(archive: &Path) -> Option<Vec<Entry>> {
    let own = match kind(archive) {
        Some(Kind::Rar) => rar::entries(archive).map_err(|e| e.to_string()),
        Some(Kind::SevenZ) => sevenz::entries(archive).map_err(|e| e.to_string()),
        None => Err("not a RAR or 7z".to_string()),
    };
    match own {
        Ok(entries) => Some(entries),
        Err(e) => {
            eprintln!("{}: {e}; asking 7-Zip", archive.display());
            sevenzip::list(archive)
        }
    }
}

/// The data of the files `members` (paths as [`list`] gave them) one after the other in the
/// archive's order; None when it can't be read or is longer than `max_len`
pub fn read(archive: &Path, members: &[&str], max_len: u64) -> Option<Vec<u8>> {
    if members.is_empty() {
        return Some(Vec::new());
    }
    let own = match kind(archive) {
        Some(Kind::Rar) => rar::read(archive, members, max_len).map_err(|e| e.to_string()),
        Some(Kind::SevenZ) => sevenz::read(archive, members, max_len).map_err(|e| e.to_string()),
        None => Err("not a RAR or 7z".to_string()),
    };
    match own {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{}: {e}; asking 7-Zip", archive.display());
            sevenzip::read(archive, members, max_len)
        }
    }
}
