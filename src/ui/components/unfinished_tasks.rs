// SPDX-FileCopyrightText: 2026 Ultimate Game Backup Manager contributors
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    jobs::SavedTask,
    message::Message,
    ui::components::{
        my_button::{MyButtonKind, my_button},
        my_card::my_card,
        row_button::row_button,
    },
};
use iced::{
    Alignment, Element, Length, padding,
    widget::{Column, column, row, rule, space, text},
};
use itertools::Itertools;
use lucide_icons::Icon;

/// The tasks that weren't finished when the app was closed, to resume or drop.
pub fn unfinished_tasks(saved: &[SavedTask]) -> Element<'_, Message> {
    let header = row![
        text("Not finished when the app was closed"),
        space::horizontal(),
        my_button()
            .label("Drop all")
            .icon(Icon::Trash2)
            .kind(MyButtonKind::Secondary)
            .on_press(Message::DropUnfinished(None)),
        my_button()
            .label("Resume all")
            .icon(Icon::Play)
            .on_press(Message::ResumeUnfinished(None)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let rows = saved
        .iter()
        .enumerate()
        .map(|(index, task)| {
            row![
                text(task.title.as_str()).width(Length::Fill),
                space::horizontal().width(10),
                row![
                    row_button(
                        Icon::Play,
                        "Resume",
                        Some(move || Message::ResumeUnfinished(Some(index)))
                    ),
                    row_button(
                        Icon::Trash2,
                        "Drop",
                        Some(move || Message::DropUnfinished(Some(index)))
                    ),
                ]
                .spacing(5),
            ]
            .align_y(Alignment::Center)
            .padding(padding::all(6).left(10).right(10))
            .into()
        })
        .intersperse_with(|| rule::horizontal(1).into())
        .collect::<Column<'_, _>>();

    column![header, my_card(rows).padding(0)].spacing(10).into()
}
