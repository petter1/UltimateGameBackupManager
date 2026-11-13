// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    games::game::Game,
    message::Message,
    ui::components::{game_card::entry, game_list::table_row, row_button::row_button},
};
use iced::Element;
use lucide_icons::Icon;

pub fn game_row(game: &Game) -> Element<'_, Message> {
    table_row(
        if game.is_wii() {
            Icon::Pointer
        } else {
            Icon::Box
        },
        if game.is_wii() { "Wii" } else { "GameCube" },
        entry(game),
        None,
        [
            row_button(
                Icon::Trash,
                "Delete game",
                Some(|| Message::AskDeleteDir(game.path().to_path_buf())),
            ),
            row_button(
                Icon::HardDriveDownload,
                "Export game",
                Some(|| Message::PickExportDest(game.clone())),
            ),
            row_button(
                Icon::Info,
                "Game info",
                Some(|| Message::OpenGameInfo(game.clone())),
            ),
        ],
    )
}
