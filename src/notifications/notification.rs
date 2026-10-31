// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use strum_macros::Display;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct Notification {
    label: Box<str>,
    level: NotificationLevel,
    created: Option<OffsetDateTime>,
    /// How often the same message came, this one included.
    count: u32,
}

impl Notification {
    pub fn info(label: impl ToString) -> Self {
        Self {
            label: label.to_string().into_boxed_str(),
            level: NotificationLevel::Info,
            created: OffsetDateTime::now_local().ok(),
            count: 1,
        }
    }

    pub fn warning(label: impl ToString) -> Self {
        Self {
            label: label.to_string().into_boxed_str(),
            level: NotificationLevel::Warning,
            created: OffsetDateTime::now_local().ok(),
            count: 1,
        }
    }

    pub fn error(label: impl ToString) -> Self {
        Self {
            label: label.to_string().into_boxed_str(),
            level: NotificationLevel::Error,
            created: OffsetDateTime::now_local().ok(),
            count: 1,
        }
    }

    pub fn success(label: impl ToString) -> Self {
        Self {
            label: label.to_string().into_boxed_str(),
            level: NotificationLevel::Success,
            created: OffsetDateTime::now_local().ok(),
            count: 1,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn level(&self) -> NotificationLevel {
        self.level
    }

    /// When it came (the latest, if it came again), as "14:03:27".
    pub fn time(&self) -> Option<String> {
        self.created.map(|created| {
            format!(
                "{:02}:{:02}:{:02}",
                created.hour(),
                created.minute(),
                created.second()
            )
        })
    }

    pub fn is_problem(&self) -> bool {
        matches!(
            self.level,
            NotificationLevel::Warning | NotificationLevel::Error
        )
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    /// Whether `other` says the same, so it is shown as this one again.
    pub fn is_same(&self, other: &Self) -> bool {
        self.level == other.level && self.label == other.label
    }

    /// Counts one more of the same message; it takes the time of the latest.
    pub fn repeat(&mut self, other: Self) {
        self.count = self.count.saturating_add(1);
        self.created = other.created;
    }
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(created) = self.created {
            write!(
                f,
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02} > ",
                created.year(),
                u8::from(created.month()),
                created.day(),
                created.hour(),
                created.minute(),
                created.second()
            )?;
        }

        write!(f, "[{}] {}", self.level, self.label)
    }
}

impl<S: ToString, E: ToString> From<std::result::Result<S, E>> for Notification {
    fn from(result: anyhow::Result<S, E>) -> Self {
        match result {
            Ok(label) => Notification::success(label),
            Err(e) => Notification::error(e),
        }
    }
}

impl From<anyhow::Error> for Notification {
    fn from(e: anyhow::Error) -> Self {
        Notification::error(e)
    }
}
