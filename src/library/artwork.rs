// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Box art of the ROMs the game databases know, from libretro's thumbnail server
//! (`thumbnails.libretro.com/<list>/Named_Boxarts/<release>.png`), found by the release's name.
//! Each picture is downloaded once, made smaller and kept as a JPEG in `covers/libretro/` in the
//! data folder; a release the server has no picture of is asked for again after a month. The
//! library shows them turned as a box seen at an angle, like the Wii and GameCube covers.

use super::{LibraryItem, Platform, box3d};
use crate::{
    romdb::{self, Match},
    util::http,
};
use anyhow::{Result, bail};
use iced::widget;
use image::{ImageFormat, codecs::jpeg::JpegEncoder, imageops::FilterType};
use smol::stream::{self, Stream, StreamExt};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const THUMBNAILS: &str = "https://thumbnails.libretro.com/";

/// The kind of picture: libretro also has `Named_Titles` (title screens), `Named_Snaps` and
/// `Named_Logos`
const BOXARTS: &str = "Named_Boxarts";

/// The longer side of a kept picture, in pixels: the server's are up to about 1000, the library
/// shows them at 48
const KEPT_SIZE: u32 = 256;

/// The quality a kept picture is saved with
const JPEG_QUALITY: u8 = 85;

/// A download larger than this isn't a box art
const MAX_DOWNLOAD: usize = 16 * 1024 * 1024;

/// How long a missing picture isn't asked for again
const ASK_AGAIN_AFTER: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The release's name as the thumbnail server has it: the DAT's escaped quotes undone, and
/// `&*/:`<>?\|"` replaced by `_`, as libretro's frontends do
fn file_name(release: &str) -> String {
    release
        .replace("\\\"", "\"")
        .chars()
        .map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c })
        .collect()
}

/// `text` for a URL path: everything but letters, digits and `-._~` percent-encoded
fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// The box art of a release on the thumbnail server; `platform` is its list's
fn boxart_url(platform: Platform, release: &str) -> Option<String> {
    let list = romdb::list_name(platform)?;
    Some(format!(
        "{THUMBNAILS}{}/{BOXARTS}/{}.png",
        encode(list),
        encode(&file_name(release))
    ))
}

/// Where the box art of a release is kept in the data folder
pub fn boxart_path(data_dir: &Path, platform: Platform, release: &str) -> Option<PathBuf> {
    let list = romdb::list_name(platform)?;
    Some(
        data_dir
            .join("covers/libretro")
            .join(list)
            .join(BOXARTS)
            .join(format!("{}.jpg", file_name(release))),
    )
}

/// The file that says the server had no box art of a release, next to where it would be kept
fn missing_marker(path: &Path) -> PathBuf {
    path.with_added_extension("missing")
}

/// What became of asking for a box art
#[derive(Debug, PartialEq, Eq)]
pub enum Fetched {
    /// kept at this path (also when it was there already)
    Kept(PathBuf),
    /// the server has none (or had none less than a month ago)
    Missing,
}

/// The box art of a release: the one kept, else downloaded from the thumbnail server and kept;
/// blocking. An error when the server doesn't answer (no network): nothing is noted then.
///
/// The server's names lag behind the lists' ("Metroid - Zero Mission (USA)" for today's "(USA,
/// Australia)"), so when it has no picture by the release's name, the one of the same game in
/// its folder's listing is taken ([`closest`]).
pub fn fetch_boxart(data_dir: &Path, platform: Platform, release: &str) -> Result<Fetched> {
    let (Some(path), Some(url), Some(list)) = (
        boxart_path(data_dir, platform, release),
        boxart_url(platform, release),
        romdb::list_name(platform),
    ) else {
        return Ok(Fetched::Missing);
    };
    let other = || {
        let names = listing(data_dir, list)?;
        Ok(closest(&file_name(release), &names).map(|name| {
            format!(
                "{THUMBNAILS}{}/{BOXARTS}/{}.png",
                encode(list),
                encode(name)
            )
        }))
    };
    fetch(&path, &url, other, |url| http::get_bytes(url, MAX_DOWNLOAD))
}

/// The names of the box arts in a list's folder on the server (without `.png`), from its listing
/// (an HTML page of about 2 MB), kept for a month next to the list's box arts
fn listing(data_dir: &Path, list: &str) -> Result<Vec<String>> {
    let kept = data_dir
        .join("covers/libretro")
        .join(list)
        .join(format!("{BOXARTS}.txt"));
    let fresh = fs::metadata(&kept)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < ASK_AGAIN_AFTER);
    if fresh && let Ok(text) = fs::read_to_string(&kept) {
        return Ok(text.lines().map(str::to_string).collect());
    }

    let url = format!("{THUMBNAILS}{}/{BOXARTS}/", encode(list));
    let Some(page) = http::get_bytes(&url, MAX_DOWNLOAD)? else {
        return Ok(Vec::new());
    };
    let names = names_in_listing(&String::from_utf8_lossy(&page));
    if let Some(folder) = kept.parent() {
        fs::create_dir_all(folder)?;
        fs::write(&kept, names.join("\n"))?;
    }
    Ok(names)
}

/// The `.png` files a server's listing links to, by name
fn names_in_listing(page: &str) -> Vec<String> {
    page.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"'))
        .filter_map(|(link, _)| link.strip_suffix(".png"))
        .filter(|link| !link.contains('/'))
        .map(decode)
        .collect()
}

/// `%XX` in a URL undone
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A game's title without its tags, only letters and digits, lower case: "Mega Man - Battle
/// Network 4 - Red Sun (USA)" and "Mega Man Battle Network 4 - Red Sun (USA)" are the same
fn title_key(name: &str) -> String {
    let title = name.split(" (").next().unwrap_or(name);
    title
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The tags of a name, in brackets: "(USA, Australia) (Rev 1)" gives "USA", "Australia", "Rev 1"
fn tags(name: &str) -> Vec<&str> {
    name.split(['(', '['])
        .skip(1)
        .filter_map(|part| part.split_once([')', ']']).map(|(tag, _)| tag))
        .flat_map(|tag| tag.split(", "))
        .collect()
}

/// Of `names`, the box art of the same game as `wanted` (a release's file name) that fits best:
/// the same title, then the most tags in common (region, languages), not a re-release or
/// unfinished one, then the shortest name
fn closest<'a>(wanted: &str, names: &'a [String]) -> Option<&'a str> {
    const UNLIKE: [&str; 6] = [
        "Virtual Console",
        "Beta",
        "Proto",
        "Demo",
        "Sample",
        "Switch Online",
    ];
    let key = title_key(wanted);
    let wanted_tags = tags(wanted);
    names
        .iter()
        .filter(|name| title_key(name) == key && name.as_str() != wanted)
        .min_by_key(|name| {
            let tags = tags(name);
            let unlike = tags
                .iter()
                .filter(|t| UNLIKE.iter().any(|u| t.starts_with(u)))
                .count();
            let common = tags.iter().filter(|t| wanted_tags.contains(t)).count();
            (unlike, std::cmp::Reverse(common), name.len())
        })
        .map(String::as_str)
}

fn fetch(
    path: &Path,
    url: &str,
    other: impl FnOnce() -> Result<Option<String>>,
    get: impl Fn(&str) -> Result<Option<Vec<u8>>>,
) -> Result<Fetched> {
    if path.is_file() {
        return Ok(Fetched::Kept(path.to_path_buf()));
    }
    let marker = missing_marker(path);
    let asked_lately = fs::metadata(&marker)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < ASK_AGAIN_AFTER);
    if asked_lately {
        return Ok(Fetched::Missing);
    }

    let folder = path.parent().unwrap_or(path);
    fs::create_dir_all(folder)?;
    let data = match get(url)? {
        Some(data) => data,
        None => match other()?.map(|url| get(&url)).transpose()?.flatten() {
            Some(data) => data,
            None => {
                fs::write(&marker, b"")?;
                return Ok(Fetched::Missing);
            }
        },
    };
    let Ok(picture) = image::load_from_memory_with_format(&data, ImageFormat::Png) else {
        bail!("{url} isn't a PNG");
    };
    let picture = if picture.width().max(picture.height()) > KEPT_SIZE {
        picture.resize(KEPT_SIZE, KEPT_SIZE, FilterType::Triangle)
    } else {
        picture
    };
    // kept as a JPEG: a box art is a photo, which a PNG of 256 pixels keeps in about 120 KB, a
    // JPEG in about 20
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY).encode_image(&picture.to_rgb8())?;

    let mut file = tempfile::NamedTempFile::new_in(folder)?;
    file.write_all(&jpeg)?;
    file.persist(path)?;
    let _ = fs::remove_file(&marker);
    Ok(Fetched::Kept(path.to_path_buf()))
}

/// The release whose box art an item shows: a ROM the databases know by its checksum (by its
/// serial only the game is known, not the release)
fn release_of(item: &LibraryItem) -> Option<(Platform, &str)> {
    let found = item.rom.as_ref()?.identified.as_ref()?;
    (found.how != Match::Serial).then_some((found.platform, found.release.as_str()))
}

/// Gives the items of a newer scan the box art the older items show, so it isn't loaded again
pub fn carry_over(old: &[LibraryItem], new: &mut [LibraryItem]) {
    let shown: HashMap<(Platform, &str), &widget::image::Handle> = old
        .iter()
        .filter_map(|item| Some((release_of(item)?, item.icon.as_ref()?)))
        .collect();
    for item in new.iter_mut().filter(|item| item.icon.is_none()) {
        if let Some(handle) = release_of(item).and_then(|release| shown.get(&release)) {
            item.icon = Some((*handle).clone());
        }
    }
}

/// Shows a release's box art on every item of that release
pub fn show(
    items: &mut [LibraryItem],
    platform: Platform,
    release: &str,
    handle: &widget::image::Handle,
) {
    for item in items.iter_mut() {
        if item.icon.is_none() && release_of(item) == Some((platform, release)) {
            item.icon = Some(handle.clone());
        }
    }
}

/// The releases of the items that show no box art yet, each once; those kept in the data folder
/// first, so that they are shown also when the server can't be reached
pub fn wanted(data_dir: &Path, items: &[LibraryItem]) -> Vec<(Platform, String)> {
    let releases: BTreeSet<(Platform, String)> = items
        .iter()
        .filter(|item| item.icon.is_none())
        .filter_map(release_of)
        .map(|(platform, release)| (platform, release.to_string()))
        .collect();
    let (kept, other): (Vec<_>, Vec<_>) = releases.into_iter().partition(|(platform, release)| {
        boxart_path(data_dir, *platform, release).is_some_and(|path| path.is_file())
    });
    kept.into_iter().chain(other).collect()
}

/// A kept box art turned into a box seen at an angle ([`box3d`]), ready to be shown
fn turned(path: &Path) -> Result<widget::image::Handle> {
    let turned = box3d::render(&image::open(path)?.to_rgb8());
    Ok(widget::image::Handle::from_rgba(
        turned.width(),
        turned.height(),
        turned.into_raw(),
    ))
}

/// Loads the box art of `releases` one after the other, downloading what isn't kept yet, and
/// gives each one turned as a box; stops at the first time the server doesn't answer (no
/// network), so the next scan tries again
pub fn load(
    data_dir: &'static Path,
    releases: Vec<(Platform, String)>,
) -> impl Stream<Item = (Platform, String, widget::image::Handle)> {
    stream::iter(releases)
        .then(move |(platform, release)| async move {
            let shown = smol::unblock({
                let release = release.clone();
                move || match fetch_boxart(data_dir, platform, &release)? {
                    Fetched::Kept(path) => turned(&path).map(Some),
                    Fetched::Missing => Ok(None),
                }
            })
            .await;
            (platform, release, shown)
        })
        .take_while(|(_, _, shown)| {
            if let Err(e) = shown {
                eprintln!("WARNING: box art: {e:#}");
            }
            shown.is_ok()
        })
        .filter_map(|(platform, release, shown)| match shown {
            Ok(Some(handle)) => Some((platform, release, handle)),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::cell::Cell;

    #[test]
    fn a_release_s_box_art_is_found_by_its_name() {
        assert_eq!(
            boxart_url(Platform::GameBoyAdvance, "Metroid Fusion (USA)").as_deref(),
            Some(
                "https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy%20Advance/\
                 Named_Boxarts/Metroid%20Fusion%20%28USA%29.png"
            )
        );
        // characters a file name can't have become "_", the DAT's escaped quotes too
        assert_eq!(
            file_name("Zeta: \\\"Quest\\\" & More? (USA)"),
            "Zeta_ _Quest_ _ More_ (USA)"
        );
        assert_eq!(boxart_url(Platform::WiiU, "x"), None);
    }

    fn no_other() -> Result<Option<String>> {
        Ok(None)
    }

    #[test]
    fn a_box_art_of_the_same_game_is_found_under_another_name() {
        let names: Vec<String> = [
            "Metroid - Zero Mission (Europe) (En,Fr,De,Es,It)",
            "Metroid - Zero Mission (USA)",
            "Metroid - Zero Mission (USA) (Virtual Console)",
            "Metroid Fusion (USA)",
            "Mega Man Battle Network 4 - Red Sun (USA)",
            "Mega Man Battle Network 4 - Blue Moon (USA)",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            closest("Metroid - Zero Mission (USA, Australia)", &names),
            Some("Metroid - Zero Mission (USA)")
        );
        assert_eq!(
            closest("Mega Man - Battle Network 4 - Red Sun (USA)", &names),
            Some("Mega Man Battle Network 4 - Red Sun (USA)")
        );
        assert_eq!(closest("Metroid - Other M (USA)", &names), None);

        let page = r#"<a href="../">..</a><a href="Mega%20Man%20(USA,%20Europe).png">x</a>
                      <a href="sub/">sub</a><a href="Zeta%20%26%20Co%20(Japan).png">y</a>"#;
        assert_eq!(
            names_in_listing(page),
            ["Mega Man (USA, Europe)", "Zeta & Co (Japan)"]
        );
    }

    #[test]
    fn another_name_is_asked_for_when_the_release_s_isn_t_there() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Named_Boxarts/Game (USA, Australia).jpg");
        let get = |url: &str| Ok((url == "other").then(|| png(4, 4)));
        let other = || Ok(Some("other".to_string()));
        assert_eq!(
            fetch(&path, "exact", other, get).unwrap(),
            Fetched::Kept(path.clone())
        );

        // neither there: noted as missing
        let path = dir.path().join("Named_Boxarts/Nothing (USA).jpg");
        let other = || Ok(Some("gone".to_string()));
        assert_eq!(fetch(&path, "exact", other, get).unwrap(), Fetched::Missing);
        assert!(missing_marker(&path).exists());
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let picture = RgbaImage::from_pixel(width, height, Rgba([200, 30, 30, 255]));
        let mut data = Vec::new();
        picture
            .write_to(&mut std::io::Cursor::new(&mut data), ImageFormat::Png)
            .unwrap();
        data
    }

    #[test]
    fn a_box_art_is_downloaded_once_and_kept_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/Named_Boxarts/Game (USA).jpg");
        let asked = Cell::new(0);
        let get = |_: &str| {
            asked.set(asked.get() + 1);
            Ok(Some(png(512, 1024)))
        };

        assert_eq!(
            fetch(&path, "u", no_other, get).unwrap(),
            Fetched::Kept(path.clone())
        );
        let kept = image::open(&path).unwrap();
        assert_eq!((kept.width(), kept.height()), (128, 256));

        // kept: not asked again
        assert_eq!(
            fetch(&path, "u", no_other, get).unwrap(),
            Fetched::Kept(path.clone())
        );
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn a_missing_box_art_is_noted_and_no_network_notes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Named_Boxarts/Game (USA).jpg");

        // no network: an error, nothing noted, asked again next time
        assert!(fetch(&path, "u", no_other, |_| bail!("offline")).is_err());
        assert!(!missing_marker(&path).exists());

        // the server has none: noted, and not asked again for a month
        assert_eq!(
            fetch(&path, "u", no_other, |_| Ok(None)).unwrap(),
            Fetched::Missing
        );
        assert!(missing_marker(&path).exists());
        let asked = Cell::new(false);
        let get = |_: &str| {
            asked.set(true);
            Ok(Some(png(8, 8)))
        };
        assert_eq!(fetch(&path, "u", no_other, get).unwrap(), Fetched::Missing);
        assert!(!asked.get());

        // something that isn't a PNG (a server's error page) isn't kept
        let other = dir.path().join("Named_Boxarts/Other (USA).jpg");
        assert!(fetch(&other, "u", no_other, |_| Ok(Some(b"<html>".to_vec()))).is_err());
        assert!(!other.exists() && !missing_marker(&other).exists());
    }
}
