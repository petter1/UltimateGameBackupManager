// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use smol::{
    fs,
    stream::{self, Stream, StreamExt},
};
use std::{
    convert::identity,
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub async fn get_dir_size(path: &Path) -> u64 {
    stream::unfold(vec![path.to_path_buf()], |mut stack| async move {
        let current = stack.pop()?;

        match fs::symlink_metadata(&current).await {
            Ok(meta) if meta.is_file() => Some((meta.len(), stack)),
            Ok(meta) if meta.is_dir() => match fs::read_dir(&current).await {
                Ok(entries) => {
                    let mut new = entries
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .collect::<Vec<_>>()
                        .await;

                    stack.append(&mut new);

                    Some((0, stack))
                }

                Err(_) => Some((0, stack)),
            },
            _ => Some((0, stack)),
        }
    })
    .fold(0, u64::saturating_add)
    .await
}

pub fn recursive_file_scan<'a>(
    path: impl Into<PathBuf>,
    exts: &'a [&'a str],
) -> impl Stream<Item = PathBuf> + 'a {
    stream::unfold(vec![path.into()], move |mut stack| async move {
        let current = stack.pop()?;

        // Skip ignored names (hidden files, split ISOs, non-utf8)
        let is_ignored = current
            .file_name()
            .and_then(OsStr::to_str)
            .is_none_or(|name| name.starts_with('.') || name.ends_with(".part1.iso"));

        if is_ignored {
            return Some((None, stack));
        }

        match fs::symlink_metadata(&current).await {
            Ok(meta) if meta.is_dir() => match fs::read_dir(&current).await {
                Ok(entries) => {
                    let mut new = entries
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .collect::<Vec<_>>()
                        .await;

                    stack.append(&mut new);

                    Some((None, stack))
                }
                Err(_) => Some((None, stack)),
            },

            Ok(meta) if meta.is_file() => {
                let matches_ext = current
                    .extension()
                    .and_then(OsStr::to_str)
                    .is_some_and(|ext| exts.contains(&ext));

                Some((matches_ext.then_some(current), stack))
            }

            _ => Some((None, stack)),
        }
    })
    .filter_map(identity)
}

/// The root of the volume a path is on, so that tasks on one disk can wait for each other
#[cfg(unix)]
pub fn volume_of(path: &Path) -> PathBuf {
    use std::os::unix::fs::MetadataExt;

    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());

    // the path itself may not exist yet, e.g. a file still to be written
    let Some((mut root, dev)) = path
        .ancestors()
        .find_map(|p| std::fs::metadata(p).ok().map(|meta| (p, meta.dev())))
    else {
        return path;
    };

    while let Some(parent) = root.parent()
        && std::fs::metadata(parent).is_ok_and(|meta| meta.dev() == dev)
    {
        root = parent;
    }

    root.to_path_buf()
}

/// The root of the volume a path is on, so that tasks on one disk can wait for each other
#[cfg(windows)]
pub fn volume_of(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    path.ancestors()
        .last()
        .map_or(path.clone(), Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::volume_of;
    use std::path::Path;

    #[test]
    fn volume_of_paths_that_dont_exist_yet() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not yet").join("game.wbfs");
        assert_eq!(volume_of(&file), volume_of(dir.path()));
    }

    #[cfg(unix)]
    #[test]
    fn volume_of_the_root() {
        assert_eq!(volume_of(Path::new("/")), Path::new("/"));
    }
}
