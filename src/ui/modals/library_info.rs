// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::util::size::size_str;
use crate::{
    library::LibraryItem,
    message::Message,
    state::AppState,
    ui::{
        components::{
            my_button::{MyButtonKind, my_button},
            my_card::my_card,
            my_link::my_link,
        },
        pages::library::{item_icon, item_id, kind_label, presence, release_target, rom_verdict},
    },
};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{button, column, image, row, rule, space, text},
};
use lucide_icons::{
    Icon,
    iced::{
        icon_copy, icon_file_question, icon_fingerprint, icon_gamepad, icon_hard_drive, icon_ruler,
        icon_search_check, icon_tag,
    },
};

/// The info of a library item, like the game info of a game on the drive
pub fn library_info<'a>(item: &'a LibraryItem, state: &'a AppState) -> Element<'a, Message> {
    let on = presence(item, state);

    let mut facts = column![
        row![icon_gamepad(), text!("System: {}", item.platform.label())].spacing(5),
        row![
            icon_file_question(),
            text!("Kind: {}", kind_label(item.kind))
        ]
        .spacing(5),
        row![icon_ruler(), text!("Size: {}", size_str(item.size))].spacing(5),
    ]
    .spacing(5);
    if let Some(id) = item_id(item) {
        facts = facts.push(row![icon_tag(), text!("ID: {id}")].spacing(5));
    }
    if let Some(rom) = &item.rom {
        facts = facts
            .push(row![icon_search_check(), text(rom_verdict(rom))].spacing(5))
            .push(row![icon_fingerprint(), text!("CRC32: {:08X}", rom.crc32)].spacing(5))
            .push(
                row![
                    icon_fingerprint(),
                    text!("SHA-1: {}", rom.sha1).size(12).width(Length::Fill)
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            );
    }
    if !item.copies.is_empty() {
        facts = facts.push(row![icon_copy(), text!("{} copies", item.copies.len() + 1)].spacing(5));
    }
    if !on.is_empty() {
        facts = facts.push(row![icon_hard_drive(), text!("On {}", on.join(", "))].spacing(5));
    }
    if item.in_cloud {
        facts = facts.push(text("Only in the cloud: not read yet"));
    }

    let cover: Element<'_, Message> = match &item.icon {
        Some(handle) => image(handle).height(200).into(),
        None => item_icon(item).widget().size(96).into(),
    };

    fn link_style(theme: &Theme, status: button::Status) -> button::Style {
        button::Style {
            text_color: theme.palette().primary,
            ..button::text(theme, status)
        }
    }

    let release = item
        .rom
        .as_ref()
        .and_then(release_target)
        .map(|(platform, release)| {
            button(
                row![Icon::Globe.widget(), text("libretro-database")]
                    .spacing(5)
                    .align_y(Alignment::Center),
            )
            .style(link_style)
            .padding(0)
            .on_press_with(move || Message::OpenRelease(platform, release.to_string()))
        });

    let copy = (on.is_empty() && item.is_drive_game()).then(|| {
        my_button()
            .label("Copy to drive")
            .icon(Icon::HardDriveUpload)
            .kind(MyButtonKind::Secondary)
            .on_press_with(|| Message::CopyToDrive(item.path.clone()))
    });

    my_card(
        column![
            column![
                text(item.title()).size(18),
                my_link(item.path.to_string_lossy(), item.path.clone()).icon(Icon::Folder)
            ]
            .spacing(10)
            .padding(20),
            space::vertical(),
            row![facts.width(Length::Fill), cover]
                .padding(20)
                .spacing(20)
                .align_y(Alignment::Center),
            space::vertical(),
            rule::horizontal(1),
            row![
                space(),
                release,
                space::horizontal(),
                copy,
                my_button()
                    .label("Close")
                    .kind(MyButtonKind::Primary)
                    .on_press(Message::CloseModal)
            ]
            .spacing(10)
            .padding(10)
            .align_y(Alignment::Center)
        ]
        .width(600)
        .height(400),
    )
    .padding(0)
    .into()
}
