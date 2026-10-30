// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    message::Message,
    state::AppState,
    tasks::{Progress, Status},
    ui::{
        components::my_button::{MyButtonKind, my_button},
        pages::Page,
    },
};
use iced::{
    Alignment, Element, Length,
    widget::{progress_bar, row, space, text},
};
use lucide_icons::Icon;
use size::Size;
use std::time::Duration;

fn format_time_left(left: Duration) -> String {
    let minutes = left.as_secs().div_ceil(60);
    match minutes {
        0..=1 => "about a minute left".to_string(),
        2..=59 => format!("{minutes} min left"),
        _ => format!("{} h {} min left", minutes / 60, minutes % 60),
    }
}

/// "1.2 GB of 4.3 GB · 35 MB/s · 2 min left"
pub fn progress_details(progress: &Progress) -> String {
    let mut details = format!(
        "{} of {}",
        Size::from_bytes(progress.done),
        Size::from_bytes(progress.total)
    );

    if let Some(speed) = progress.speed {
        details.push_str(&format!(" · {}/s", Size::from_bytes(speed as u64)));
    }

    if let Some(left) = progress.time_left() {
        details.push_str(" · ");
        details.push_str(&format_time_left(left));
    }

    details
}

/// One line at the bottom of every page while there are tasks
pub fn task_bar(state: &AppState) -> Option<Element<'_, Message>> {
    let tasks = &state.tasks;
    if tasks.is_empty() || state.current_page == Page::Tasks {
        return None;
    }

    let running = tasks.count(&Status::Running);
    let waiting = tasks.count(&Status::Waiting);
    let failed = tasks.failed_count();

    let summary = if running == 1 {
        tasks
            .iter()
            .find(|entry| entry.status == Status::Running)
            .map(|entry| entry.job.label())
            .unwrap_or_default()
    } else if running > 1 {
        format!("{running} tasks running")
    } else {
        "All tasks finished".to_string()
    };

    let mut counts = Vec::new();
    if waiting > 0 {
        counts.push(format!("{waiting} waiting"));
    }
    if failed > 0 {
        counts.push(format!("{failed} failed"));
    }

    let overall = tasks.overall_progress();

    Some(
        row![
            Icon::ListChecks.widget(),
            text(summary),
            (!counts.is_empty()).then(|| text(counts.join(" · ")).size(12)),
            match &overall {
                Some(progress) => Element::from(
                    progress_bar(0.0..=1.0, progress.fraction())
                        .length(Length::Fill)
                        .girth(6),
                ),
                None => space::horizontal().into(),
            },
            overall.map(|progress| text(progress_details(&progress)).size(12)),
            my_button()
                .label("Tasks")
                .kind(MyButtonKind::Secondary)
                .on_press(Message::NavigateTo(Page::Tasks)),
        ]
        .spacing(10)
        .padding([5, 10])
        .align_y(Alignment::Center)
        .into(),
    )
}
