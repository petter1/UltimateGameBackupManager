// SPDX-FileCopyrightText: 2026 Ultimate Game Backup Manager contributors
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    message::Message,
    state::AppState,
    ui::components::{
        my_card::my_card, task_row::task_row, tasks_titlebar::tasks_titlebar,
        unfinished_tasks::unfinished_tasks,
    },
};
use iced::{
    Element, Length, padding,
    widget::{Column, column, container, rule, scrollable, text},
};
use itertools::Itertools;

/// The task manager: unfinished tasks from before a restart, running and waiting tasks, then the
/// history.
pub fn tasks(state: &AppState) -> Element<'_, Message> {
    let (finished, open): (Vec<_>, Vec<_>) =
        state.tasks.iter().partition(|t| t.state().is_finished());
    let unfinished = state.unfinished.as_deref().unwrap_or_default();

    let mut content = Column::new().spacing(10);

    if !unfinished.is_empty() {
        content = content.push(unfinished_tasks(unfinished));
    }

    if !finished.is_empty() || !open.is_empty() {
        content = content.push(
            my_card(
                open.into_iter()
                    .chain(finished.into_iter().rev())
                    .map(task_row)
                    .intersperse_with(|| rule::horizontal(1).into())
                    .collect::<Column<'_, _>>(),
            )
            .padding(0),
        );
    } else if unfinished.is_empty() {
        content = content.push(text(
            "No tasks yet: importing, exporting, converting, scrubbing and hashing games show up here.",
        ));
    }

    column![
        tasks_titlebar(state),
        scrollable(container(content).padding(padding::all(10).right(20).top(0)))
            .width(Length::Fill),
    ]
    .into()
}
