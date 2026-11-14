// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::util::size::size_str;
use crate::{
    config::{ViewAs, ui::view_as::view_as},
    library::{ItemId, ItemKind, LibraryItem, Platform, RomInfo},
    message::Message,
    romdb::{self, Match},
    state::AppState,
    ui::components::{
        game_list::{Entry, card, card_button, grid, table, table_row},
        my_button::{MyButtonKind, my_button},
        my_card::my_card,
        my_group::my_group,
        row_button::row_button,
        search_bar::search_bar,
    },
};
use iced::{
    Alignment, Element, Length, padding,
    widget::{Column, Row, column, row, scrollable, space, text, tooltip},
};
use lucide_icons::Icon;

pub fn library(state: &AppState) -> Element<'_, Message> {
    let library = &state.library;

    let titlebar = row![
        search_bar(&library.search, Message::SearchLibrary, "Search by title"),
        space::horizontal(),
        library.scanning.then(|| text("Scanning…")),
        library
            .shown_drive_games()
            .any(|item| !item.in_cloud && presence(item, state).is_empty())
            .then(|| tooltip(
                my_button::<Message>()
                    .icon(Icon::HardDriveUpload)
                    .label("Copy all to drive")
                    .on_press(Message::CopyShownToDrive),
                my_card("Copy the Wii and GameCube games shown here that aren't on the drive yet"),
                tooltip::Position::Bottom
            )),
        view_as(&state.config).map(Message::Config),
        space().width(5),
        tooltip(
            my_button::<Message>()
                .icon(Icon::RefreshCw)
                .kind(MyButtonKind::Toolbar)
                .on_press(Message::ScanLibrary),
            my_card("Scan the sources again"),
            tooltip::Position::Bottom
        ),
        tooltip(
            my_button::<Message>()
                .icon(Icon::FolderOpen)
                .kind(MyButtonKind::Toolbar)
                .on_press(Message::ToggleRomSources),
            my_card("Sources: the folders with your games"),
            tooltip::Position::Bottom
        ),
    ]
    .spacing(5)
    .padding(10)
    .align_y(Alignment::Center);

    let chips = Row::with_children(
        std::iter::once(chip("All", library.platform.is_none(), None)).chain(
            library
                .platforms()
                .into_iter()
                .map(|p| chip(p.label(), library.platform == Some(p), Some(p))),
        ),
    )
    .spacing(5)
    .padding(padding::horizontal(10))
    .wrap();

    let sources =
        (library.show_sources || state.config.rom_sources().is_empty()).then(|| sources(state));

    let body: Element<'_, Message> = if state.config.rom_sources().is_empty() {
        space().into()
    } else if library.items.is_empty() {
        text(if library.scanning {
            "Scanning the sources…"
        } else {
            "No games found in the sources."
        })
        .into()
    } else {
        match state.config.view_as() {
            ViewAs::Grid => grid(library.visible().map(|item| item_card(item, state))),
            ViewAs::Table => table(library.visible().map(|item| item_row(item, state))),
        }
    };

    column![
        titlebar,
        chips,
        scrollable(
            column![sources, body]
                .spacing(10)
                .padding(padding::all(10).right(20))
                .width(Length::Fill)
        )
    ]
    .spacing(5)
    .into()
}

fn chip(
    label: &'static str,
    active: bool,
    platform: Option<Platform>,
) -> Element<'static, Message> {
    my_button::<Message>()
        .label(label)
        .kind(if active {
            MyButtonKind::Primary
        } else {
            MyButtonKind::Secondary
        })
        .on_press(Message::FilterLibrary(platform))
        .into()
}

fn sources(state: &AppState) -> Element<'_, Message> {
    let list = state
        .config
        .rom_sources()
        .iter()
        .map(|source| {
            row![
                text(source.display().to_string()).width(Length::Fill),
                my_button::<Message>()
                    .icon(Icon::Trash)
                    .kind(MyButtonKind::Danger)
                    .on_press(Message::RemoveRomSource(source.clone())),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
        })
        .collect::<Column<'_, _>>()
        .spacing(5);

    let hint = state.config.rom_sources().is_empty().then(|| {
        text(
            "Add the folders where your games, ROMs, CIAs and WADs are. They are scanned \
             by file name; files that are only in the cloud are not downloaded.",
        )
        .size(12)
    });

    my_group(
        "Sources",
        column![
            hint,
            list,
            my_button::<Message>()
                .icon(Icon::FolderPlus)
                .label("Add folder")
                .on_press(Message::PickRomSource)
        ]
        .spacing(10),
    )
    .icon(Icon::FolderOpen)
    .into()
}

pub fn placeholder_icon(platform: Platform) -> Icon {
    match platform {
        Platform::N3ds => Icon::Gamepad2,
        Platform::Wii | Platform::GameCube | Platform::WiiU => Icon::Disc3,
        _ => Icon::Joystick,
    }
}

/// Where an item is already: the online consoles and the selected drive
pub fn presence(item: &LibraryItem, state: &AppState) -> Vec<String> {
    match &item.id {
        Some(ItemId::Title(id)) => state
            .consoles
            .having_title(*id)
            .map(|entry| entry.label.clone())
            .collect(),
        Some(ItemId::Game(id)) => match &state.games {
            crate::games::games_state::GamesState::Loaded(games)
                if games.get_all_game_ids().any(|g| g.as_str() == id) =>
            {
                let drive = state.config.mount_point();
                vec![
                    drive
                        .file_name()
                        .unwrap_or(drive.as_os_str())
                        .to_string_lossy()
                        .into_owned(),
                ]
            }
            _ => Vec::new(),
        },
        None => Vec::new(),
    }
}

/// The game ID, title ID or cartridge serial, shown like a disc's game ID; of a 3DS title ID
/// only the unique part, the high half is the same for all games
pub fn item_id(item: &LibraryItem) -> Option<String> {
    match &item.id {
        Some(ItemId::Game(id)) => Some(id.clone()),
        Some(ItemId::Title(id)) => Some(format!("{:08X}", id & 0xFFFF_FFFF)),
        None => item.rom.as_ref().and_then(|rom| rom.serial.clone()),
    }
}

pub fn item_icon(item: &LibraryItem) -> Icon {
    if item.kind == ItemKind::SdBackup {
        Icon::MemoryStick
    } else {
        placeholder_icon(item.platform)
    }
}

pub fn kind_label(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Cia => "CIA",
        ItemKind::Cart => "game card",
        ItemKind::Disc => "disc",
        ItemKind::Wad => "WAD",
        ItemKind::WiiUTitle => "title",
        ItemKind::Rom => "ROM",
        ItemKind::SdBackup => "SD card backup",
        ItemKind::Archive => "archive",
    }
}

/// Opens the folder the item is in
fn show_in_folder(item: &LibraryItem) -> Message {
    Message::Open(
        item.path
            .parent()
            .unwrap_or(&item.path)
            .as_os_str()
            .to_os_string(),
    )
}

fn entry(item: &LibraryItem) -> Entry<'_> {
    Entry {
        id: item_id(item).unwrap_or_default().into(),
        size: size_str(item.size).to_string().into(),
        title: item.title(),
        cover: item.icon.as_ref(),
        placeholder: Some(item_icon(item)),
    }
}

type OnPress<'a> = Option<Box<dyn Fn() -> Message + 'a>>;

/// A card's or a row's button
type ButtonFn<'a> = fn(Icon, &'static str, OnPress<'a>) -> Element<'a, Message>;

/// The buttons after "Info": the release in libretro-database, then copying to the drive (a
/// check mark where the game is already) or showing the file in its folder
fn buttons<'a>(
    item: &'a LibraryItem,
    state: &AppState,
    button: ButtonFn<'a>,
) -> Vec<Element<'a, Message>> {
    let mut buttons = Vec::new();
    if let Some((platform, release)) = item.rom.as_ref().and_then(release_target) {
        buttons.push(button(
            Icon::Globe,
            "The release in libretro-database",
            Some(Box::new(move || {
                Message::OpenRelease(platform, release.to_string())
            })),
        ));
    }
    buttons.push(if !presence(item, state).is_empty() {
        button(Icon::CircleCheck, "Already there", None)
    } else if item.is_drive_game() {
        button(
            Icon::HardDriveUpload,
            "Copy to drive",
            Some(Box::new(|| Message::CopyToDrive(item.path.clone()))),
        )
    } else {
        button(
            Icon::FolderOpen,
            "Show in its folder",
            Some(Box::new(|| show_in_folder(item))),
        )
    });
    buttons
}

/// A card of the drive's games
fn item_card<'a>(item: &'a LibraryItem, state: &AppState) -> Element<'a, Message> {
    card(
        entry(item),
        || Message::OpenLibraryInfo(item.path.clone()),
        buttons(item, state, |icon, hint, on_press| {
            card_button(icon, hint, on_press)
        }),
    )
}

/// A table row of the drive's games
fn item_row<'a>(item: &'a LibraryItem, state: &AppState) -> Element<'a, Message> {
    let on = presence(item, state);
    let mut buttons = buttons(item, state, |icon, hint, on_press| {
        row_button(icon, hint, on_press)
    });
    buttons.push(row_button(
        Icon::Info,
        "Info",
        Some(Box::new(|| Message::OpenLibraryInfo(item.path.clone()))),
    ));
    table_row(
        item_icon(item),
        item.platform.label(),
        entry(item),
        (!on.is_empty()).then(|| format!("✓ {}", on.join(", "))),
        buttons,
    )
}

/// What the game database says about a ROM
pub fn rom_verdict(rom: &RomInfo) -> String {
    match &rom.identified {
        Some(found) => match found.how {
            Match::Verified => "✓ good dump (No-Intro)".to_string(),
            Match::Crc32 => "CRC32 in No-Intro, SHA-1 not".to_string(),
            Match::Serial => format!(
                "probably {} (other dump, hack or translation)",
                found.release
            ),
        },
        None => "not in No-Intro".to_string(),
    }
}

/// The release in libretro-database's DAT on GitHub, for a ROM that is a known dump
pub fn release_target(rom: &RomInfo) -> Option<(Platform, &str)> {
    let found = rom.identified.as_ref()?;
    (found.how != Match::Serial && romdb::has_release_page(found.platform))
        .then_some((found.platform, found.release.as_str()))
}
