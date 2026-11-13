// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    config::ViewAs,
    games::games_state::GamesState,
    message::Message,
    state::AppState,
    ui::{
        components::{
            game_card::game_card,
            game_list::{grid, list_padding, table},
            game_row::game_row,
            games_titlebar::games_titlebar,
        },
        pages::{errored::errored, loading::loading, no_drive::no_drive},
    },
};
use iced::{
    Element, Length,
    widget::{column, container, scrollable},
};

pub fn games(state: &AppState) -> Element<'_, Message> {
    if state.config.mount_point().as_os_str().is_empty() {
        return no_drive();
    }

    match &state.games {
        GamesState::NotLoaded | GamesState::Loading => loading(),
        GamesState::Errored(e) => errored(e),
        GamesState::Loaded(games) => {
            let content: Element<'_, Message> = match state.config.view_as() {
                ViewAs::Grid => grid(games.iter_by(state.config.sort_by()).map(game_card)),
                ViewAs::Table => table(games.iter_by(state.config.sort_by()).map(game_row)),
            };

            column![
                games_titlebar(games, state),
                scrollable(container(content).padding(list_padding())).width(Length::Fill),
            ]
            .into()
        }
    }
}
