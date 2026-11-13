// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! The list of games as the drive shows it: a grid of cards or a table of rows. The drive's
//! games and the library both use it.

use crate::{
    message::Message,
    ui::components::{my_button::my_button, my_card::my_card},
};
use iced::{
    Alignment, Element, Length, padding,
    widget::{
        Column, Row, column, container, image, row, rule, space, text, text::IntoFragment, tooltip,
    },
};
use iced_palace::widget::ellipsized_text;
use itertools::Itertools;
use lucide_icons::{Icon, iced::icon_tag};
use std::borrow::Cow;

/// The cards side by side, wrapping
pub fn grid<'a>(cards: impl Iterator<Item = Element<'a, Message>>) -> Element<'a, Message> {
    cards.collect::<Row<'_, _>>().spacing(10).wrap().into()
}

/// The rows in one card, a line between two
pub fn table<'a>(rows: impl Iterator<Item = Element<'a, Message>>) -> Element<'a, Message> {
    my_card(
        rows.intersperse_with(|| rule::horizontal(1).into())
            .collect::<Column<'_, _>>(),
    )
    .padding(0)
    .into()
}

/// The list's padding inside its scrollable
pub fn list_padding() -> iced::Padding {
    padding::all(10).right(20).top(0)
}

/// A small button with a hint, for a card
pub fn card_button<'a>(
    icon: Icon,
    hint: &'static str,
    on_press: Option<impl Fn() -> Message + 'a>,
) -> Element<'a, Message> {
    let mut btn = my_button().icon(icon);
    if let Some(on_press) = on_press {
        btn = btn.on_press_with(on_press);
    }
    tooltip(btn, my_card(hint), tooltip::Position::Bottom).into()
}

/// What a card or row shows
pub struct Entry<'a> {
    /// game ID, title ID, serial
    pub id: Cow<'a, str>,
    pub size: Cow<'a, str>,
    pub title: &'a str,
    pub cover: Option<&'a image::Handle>,
    /// shown in place of a missing cover; nothing when `None`
    pub placeholder: Option<Icon>,
}

/// A card: ID and size, the cover, the title, "Info" and the other buttons
pub fn card<'a>(
    entry: Entry<'a>,
    on_info: impl Fn() -> Message + 'a,
    buttons: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let cover: Element<'_, Message> = match (entry.cover, entry.placeholder) {
        (Some(cover), _) => image(cover).height(96).into(),
        (None, Some(icon)) => container(icon.widget().size(48)).center_y(96).into(),
        (None, None) => space().height(96).into(),
    };

    let info = my_button()
        .label("Info")
        .icon(Icon::Info)
        .on_press_with(on_info)
        .expand_width();

    my_card(
        column![
            row![
                icon_tag(),
                text(entry.id),
                space::horizontal(),
                text(entry.size)
            ]
            .spacing(5),
            cover,
            ellipsized_text(entry.title).wrapping(text::Wrapping::None),
            Row::with_children(std::iter::once(info.into()).chain(buttons)).spacing(5),
        ]
        .align_x(Alignment::Center)
        .padding(5)
        .spacing(10),
    )
    .width(169.5)
    .into()
}

/// A table row: the platform's icon, title and ID, what else is worth saying, size, buttons
pub fn table_row<'a>(
    icon: Icon,
    platform: impl IntoFragment<'a>,
    entry: Entry<'a>,
    note: Option<String>,
    buttons: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let title = if entry.id.is_empty() {
        entry.title.to_string()
    } else {
        format!("{} [{}]", entry.title, entry.id)
    };

    Row::with_children(
        [
            tooltip(
                icon.widget(),
                my_card(text(platform)),
                tooltip::Position::Top,
            )
            .into(),
            text(title).into(),
            space::horizontal().into(),
        ]
        .into_iter()
        .chain(note.map(|note| text(note).size(12).into()))
        .chain([text(entry.size).into(), rule::vertical(1).into()])
        .chain(buttons),
    )
    .spacing(5)
    .height(Length::Shrink)
    .align_y(Alignment::Center)
    .padding(padding::all(2).left(5))
    .into()
}
