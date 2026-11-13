// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    config::{ViewAs, ui::view_as::view_as},
    library::{ItemId, ItemKind, LibraryItem, Platform, RomInfo},
    message::Message,
    romdb::{self, Match},
    state::AppState,
    ui::components::{
        my_button::{MyButtonKind, my_button},
        my_card::my_card,
        my_group::my_group,
        row_button::row_button,
        search_bar::search_bar,
    },
};
use iced::{
    Alignment, Element, Length, padding,
    widget::{Column, Row, column, container, image, row, rule, scrollable, space, text, tooltip},
};
use iced_palace::widget::ellipsized_text;
use itertools::Itertools;
use lucide_icons::{Icon, iced::icon_tag};
use size::Size;

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
            ViewAs::Grid => library
                .visible()
                .map(|item| item_card(item, state))
                .collect::<Row<'_, _>>()
                .spacing(10)
                .wrap()
                .into(),
            ViewAs::Table => my_card(
                library
                    .visible()
                    .map(|item| item_row(item, state))
                    .intersperse_with(|| rule::horizontal(1).into())
                    .collect::<Column<'_, _>>(),
            )
            .padding(0)
            .into(),
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

/// What the item is, for the tooltip: platform, kind, copies, the game database's verdict, and
/// a ROM's file and checksums
fn details(item: &LibraryItem) -> String {
    let mut details = vec![item.platform.label().to_string()];
    details.push(kind_label(item.kind).to_string());
    details.push(Size::from_bytes(item.size).to_string());
    if item.in_cloud {
        details.push("in the cloud".to_string());
    }
    if !item.copies.is_empty() {
        details.push(format!("{} copies", item.copies.len() + 1));
    }

    let mut lines = vec![details.join(" · ")];
    if let Some(rom) = &item.rom {
        lines.push(rom_verdict(rom));
        lines.push(rom_checksums(item, rom));
    } else {
        lines.push(item.path.display().to_string());
    }
    lines.join("\n")
}

fn details_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    item: &LibraryItem,
) -> Element<'a, Message> {
    tooltip(
        content,
        my_card(text(details(item)).size(12)),
        tooltip::Position::Bottom,
    )
    .into()
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

/// A card like the ones of the games on the drive: ID and size, the cover, the title, buttons
fn item_card<'a>(item: &'a LibraryItem, state: &'a AppState) -> Element<'a, Message> {
    let cover: Element<'_, Message> = match &item.icon {
        Some(handle) => image(handle).height(96).into(),
        None => container(item_icon(item).widget().size(48))
            .center_x(Length::Shrink)
            .center_y(96)
            .into(),
    };

    let on = presence(item, state);
    let copy: Element<'_, Message> = if !on.is_empty() {
        tooltip(
            my_button::<Message>().icon(Icon::CircleCheck),
            my_card(text!("On {}", on.join(", "))),
            tooltip::Position::Bottom,
        )
        .into()
    } else if item.is_drive_game() {
        tooltip(
            my_button()
                .icon(Icon::HardDriveUpload)
                .on_press_with(|| Message::CopyToDrive(item.path.clone())),
            my_card("Copy to drive"),
            tooltip::Position::Bottom,
        )
        .into()
    } else {
        tooltip(
            my_button()
                .icon(Icon::FolderOpen)
                .on_press_with(|| show_in_folder(item)),
            my_card("Show in its folder"),
            tooltip::Position::Bottom,
        )
        .into()
    };

    let release = item
        .rom
        .as_ref()
        .and_then(release_target)
        .map(|(platform, release)| {
            tooltip(
                my_button()
                    .icon(Icon::Globe)
                    .on_press_with(move || Message::OpenRelease(platform, release.to_string())),
                my_card("The release in libretro-database"),
                tooltip::Position::Bottom,
            )
        });

    my_card(
        column![
            row![
                icon_tag(),
                text(item_id(item).unwrap_or_else(|| item.platform.label().to_string())),
                space::horizontal(),
                text(Size::from_bytes(item.size).to_string())
            ]
            .spacing(5),
            cover,
            ellipsized_text(item.title()).wrapping(text::Wrapping::None),
            row![
                my_button()
                    .label("Info")
                    .icon(Icon::Info)
                    .on_press_with(|| Message::OpenLibraryInfo(item.path.clone()))
                    .expand_width(),
                release,
                copy,
            ]
            .spacing(5)
        ]
        .align_x(Alignment::Center)
        .padding(5)
        .spacing(10),
    )
    .width(169.5)
    .into()
}

/// A table row like the ones of the games on the drive
fn item_row<'a>(item: &'a LibraryItem, state: &'a AppState) -> Element<'a, Message> {
    let title = match item_id(item) {
        Some(id) => format!("{} [{id}]", item.title()),
        None => item.title().to_string(),
    };

    let on = presence(item, state);
    let copy = (on.is_empty() && item.is_drive_game()).then(|| {
        row_button(
            Icon::HardDriveUpload,
            "Copy to drive",
            Some(|| Message::CopyToDrive(item.path.clone())),
        )
    });
    let release = item
        .rom
        .as_ref()
        .and_then(release_target)
        .map(|(platform, release)| {
            row_button(
                Icon::Globe,
                "The release in libretro-database",
                Some(move || Message::OpenRelease(platform, release.to_string())),
            )
        });

    row![
        tooltip(
            item_icon(item).widget(),
            my_card(item.platform.label()),
            tooltip::Position::Top
        ),
        details_tooltip(text(title), item),
        space::horizontal(),
        (!on.is_empty()).then(|| text!("✓ {}", on.join(", ")).size(12)),
        text(Size::from_bytes(item.size).to_string()),
        rule::vertical(1),
        copy,
        release,
        row_button(
            Icon::FolderOpen,
            "Show in its folder",
            Some(|| show_in_folder(item))
        ),
        row_button(
            Icon::Info,
            "Info",
            Some(|| Message::OpenLibraryInfo(item.path.clone()))
        ),
    ]
    .spacing(5)
    .height(Length::Shrink)
    .align_y(Alignment::Center)
    .padding(padding::all(2).left(5))
    .into()
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

/// The file, checksums and serial of a ROM, for the tooltip
fn rom_checksums(item: &LibraryItem, rom: &RomInfo) -> String {
    let file = item
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut lines = vec![
        file,
        format!("CRC32 {:08X}", rom.crc32),
        format!("SHA-1 {}", rom.sha1),
    ];
    if let Some(serial) = &rom.serial {
        lines.push(format!("Serial {serial}"));
    }
    lines.join("\n")
}
