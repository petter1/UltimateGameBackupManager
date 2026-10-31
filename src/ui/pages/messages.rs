// SPDX-FileCopyrightText: 2026 Ultimate Game Backup Manager contributors
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    message::Message,
    notifications::notification::Notification,
    state::AppState,
    ui::components::{
        my_button::{MyButtonKind, my_button},
        my_card::my_card,
        notifications::level_icon,
    },
};
use iced::{
    Alignment, Element, Length, padding,
    widget::{Column, checkbox, column, container, row, rule, scrollable, space, text, text_input},
};
use itertools::Itertools;
use lucide_icons::Icon;

fn message_row(notification: &Notification) -> Element<'_, Message> {
    row![
        level_icon(notification.level()),
        text(notification.time().unwrap_or_default()).size(12),
        text(notification.label()).width(Length::Fill),
        (notification.count() > 1).then(|| text!("{}×", notification.count()).size(12)),
    ]
    .spacing(10)
    .padding(10)
    .align_y(Alignment::Center)
    .into()
}

fn titlebar(state: &AppState) -> Element<'_, Message> {
    let mut clear_btn = my_button()
        .label("Clear")
        .icon(Icon::BrushCleaning)
        .kind(MyButtonKind::Secondary);

    if !state.notifications.history().is_empty() {
        clear_btn = clear_btn.on_press(Message::ClearMessages);
    }

    let log_btn = my_button()
        .label("Open log file")
        .icon(Icon::FileText)
        .kind(MyButtonKind::Secondary)
        .on_press_with(|| Message::Open(state.data_dir.join("log.txt").into_os_string()));

    row![
        "  Messages",
        text_input("Search", &state.messages_search)
            .on_input(Message::SearchMessages)
            .width(200),
        space::horizontal(),
        checkbox(state.messages_problems_only)
            .label("Only warnings and errors")
            .on_toggle(Message::SetMessagesProblemsOnly),
        clear_btn,
        log_btn,
    ]
    .spacing(10)
    .padding(10)
    .align_y(Alignment::Center)
    .into()
}

fn matches_search(notification: &Notification, search: &str) -> bool {
    search.is_empty()
        || notification
            .label()
            .to_lowercase()
            .contains(&search.trim().to_lowercase())
}

/// Every message since the start, the newest first; the log file has all of them.
pub fn messages(state: &AppState) -> Element<'_, Message> {
    let shown = state
        .notifications
        .history()
        .iter()
        .rev()
        .filter(|n| !state.messages_problems_only || n.is_problem())
        .filter(|n| matches_search(n, &state.messages_search))
        .collect::<Vec<_>>();

    let content: Element<'_, Message> = if shown.is_empty() {
        text("No messages yet.").into()
    } else {
        my_card(
            shown
                .into_iter()
                .map(message_row)
                .intersperse_with(|| rule::horizontal(1).into())
                .collect::<Column<'_, _>>(),
        )
        .padding(0)
        .into()
    };

    column![
        titlebar(state),
        scrollable(container(content).padding(padding::all(10).right(20).top(0)))
            .width(Length::Fill),
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ignores_case_and_surrounding_spaces() {
        let n = Notification::error("Read-only file system");
        assert!(matches_search(&n, ""));
        assert!(matches_search(&n, " read-ONLY "));
        assert!(!matches_search(&n, "disk full"));
    }
}
