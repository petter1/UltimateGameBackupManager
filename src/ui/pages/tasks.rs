// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    message::Message,
    state::AppState,
    tasks::{Status, TaskAction, TaskEntry},
    ui::components::{
        my_button::{MyButtonKind, my_button},
        my_card::my_card,
        row_button::row_button,
        task_bar::progress_details,
    },
};
use iced::{
    Alignment, Element, Length, Theme, padding,
    widget::{Column, column, container, progress_bar, row, rule, scrollable, space, text},
};
use itertools::Itertools;
use lucide_icons::Icon;

fn action(
    entry: &TaskEntry,
    icon: Icon,
    hint: &'static str,
    action: TaskAction,
) -> Element<'static, Message> {
    let id = entry.id;
    row_button(icon, hint, Some(move || Message::TaskAction(id, action)))
}

fn task_row(entry: &TaskEntry) -> Element<'_, Message> {
    let buttons: Vec<Element<'_, Message>> = match &entry.status {
        Status::Running if entry.is_cancelling() => vec![],
        Status::Running if entry.is_paused() => vec![
            action(entry, Icon::Play, "Resume", TaskAction::Resume),
            action(entry, Icon::X, "Cancel", TaskAction::Cancel),
        ],
        Status::Running => vec![
            action(entry, Icon::Pause, "Pause", TaskAction::Pause),
            action(entry, Icon::X, "Cancel", TaskAction::Cancel),
        ],
        Status::Waiting => vec![
            action(entry, Icon::ArrowUp, "Earlier", TaskAction::MoveUp),
            action(entry, Icon::ArrowDown, "Later", TaskAction::MoveDown),
            action(entry, Icon::X, "Cancel", TaskAction::Cancel),
        ],
        Status::Failed(_) | Status::Cancelled => {
            vec![action(
                entry,
                Icon::RotateCcw,
                "Try again",
                TaskAction::Retry,
            )]
        }
        Status::Done(_) => vec![],
    };

    let state_text = match &entry.status {
        Status::Running if entry.is_cancelling() => Some("Cancelling…".to_string()),
        Status::Running if entry.is_paused() => Some("Paused".to_string()),
        Status::Running => entry.step.clone(),
        Status::Waiting => Some("Waiting".to_string()),
        Status::Done(msg) => Some(msg.clone()),
        Status::Failed(e) => Some(e.clone()),
        Status::Cancelled => Some("Cancelled".to_string()),
    };

    let is_failed = matches!(entry.status, Status::Failed(_));

    let progress = (entry.status == Status::Running)
        .then_some(entry.progress)
        .flatten()
        .map(|progress| {
            row![
                progress_bar(0.0..=1.0, progress.fraction())
                    .length(Length::Fill)
                    .girth(6),
                text!("{:.0}%", progress.fraction() * 100.0).size(12),
                text(progress_details(&progress)).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        });

    column![
        row![
            entry.job.icon().widget(),
            text(entry.job.label()),
            space::horizontal(),
            row(buttons).spacing(5),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        progress,
        state_text.map(|state_text| {
            text(state_text)
                .size(12)
                .style(move |theme: &Theme| text::Style {
                    color: is_failed.then_some(theme.palette().danger),
                })
        }),
    ]
    .spacing(5)
    .padding(padding::all(5).left(10).right(10))
    .into()
}

fn section<'a>(
    title: &'static str,
    entries: impl Iterator<Item = &'a TaskEntry>,
) -> Option<Element<'a, Message>> {
    let mut entries = entries.peekable();
    entries.peek()?;

    Some(
        column![
            text(title).size(16),
            my_card(
                entries
                    .map(task_row)
                    .intersperse_with(|| rule::horizontal(1).into())
                    .collect::<Column<'_, _>>()
            )
            .padding(0)
        ]
        .spacing(5)
        .into(),
    )
}

pub fn tasks(state: &AppState) -> Element<'_, Message> {
    let tasks = &state.tasks;

    let mut clear_btn = my_button()
        .label("Clear finished")
        .icon(Icon::BrushCleaning)
        .kind(MyButtonKind::Secondary);

    if tasks.iter().any(|entry| entry.status.is_finished()) {
        clear_btn = clear_btn.on_press(Message::ClearFinishedTasks);
    }

    let titlebar = row![text("Tasks").size(18), space::horizontal(), clear_btn]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center);

    let content: Element<'_, Message> = if tasks.is_empty() {
        text("Nothing to do. Copies, imports, conversions and the like show up here.").into()
    } else {
        column![
            section(
                "Running",
                tasks.iter().filter(|entry| entry.status == Status::Running)
            ),
            section(
                "Waiting",
                tasks.iter().filter(|entry| entry.status == Status::Waiting)
            ),
            section(
                "Finished",
                tasks
                    .iter()
                    .filter(|entry| entry.status.is_finished())
                    .rev()
            ),
        ]
        .spacing(15)
        .into()
    };

    column![
        titlebar,
        scrollable(container(content).padding(padding::all(10).right(20).top(0)))
            .width(Length::Fill),
    ]
    .into()
}
