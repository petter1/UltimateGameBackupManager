// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    consoles::TitleAction,
    link::protocol::Title,
    message::Message,
    state::AppState,
    ui::components::{
        my_button::{MyButtonKind, my_button},
        my_card::my_card,
    },
};
use iced::{
    Element, Length,
    widget::{column, container, row, rule, space, text},
};
use std::net::SocketAddr;

pub fn delete_title<'a>(
    addr: SocketAddr,
    title: &'a Title,
    name: &'a str,
    state: &'a AppState,
) -> Element<'a, Message> {
    let console = state
        .console_label(addr)
        .unwrap_or_else(|| addr.to_string());

    my_card(
        column![
            container(text!(
                "Delete {name} from {console}?\nIts save data is deleted with it. To play it again, \
                 install it again."
            ))
            .padding(10),
            space::vertical(),
            rule::horizontal(1),
            row![
                space::horizontal(),
                my_button().label("Cancel").on_press(Message::CloseModal),
                my_button()
                    .label("Delete")
                    .kind(MyButtonKind::Danger)
                    .on_press_with(move || Message::TitleAction(
                        addr,
                        TitleAction::Delete,
                        title.clone(),
                        name.to_owned()
                    ))
            ]
            .spacing(10)
            .padding(10)
        ]
        .width(600)
        .height(Length::Shrink),
    )
    .padding(0)
    .into()
}
