// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    games::game::Game,
    message::Message,
    ui::components::game_list::{Entry, card, card_button},
};
use iced::Element;
use lucide_icons::Icon;

pub fn entry(game: &Game) -> Entry<'_> {
    Entry {
        id: game.id_str().into(),
        size: game.size_str().into(),
        title: game.title(),
        cover: game.cover(),
        placeholder: None,
    }
}

pub fn game_card(game: &Game) -> Element<'_, Message> {
    card(
        entry(game),
        || Message::OpenGameInfo(game.clone()),
        [
            card_button(
                Icon::HardDriveDownload,
                "Export game",
                Some(|| Message::PickExportDest(game.clone())),
            ),
            card_button(
                Icon::Trash,
                "Delete game",
                Some(|| Message::AskDeleteDir(game.path().to_path_buf())),
            ),
        ],
    )
}
