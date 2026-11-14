// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    link::serve::DownloadState,
    message::Message,
    state::AppState,
    ui::components::{my_button::my_button, my_card::my_card},
};
use iced::{
    Alignment, Element, Length,
    widget::{Column, column, container, image, row, rule, space, text},
};
use size::Size;

/// The QR code for FBI's "Remote Install → Scan QR Code", with what the console has downloaded
pub fn qr_install(state: &AppState) -> Element<'_, Message> {
    let Some(qr) = &state.qr_install else {
        return space().into();
    };
    let server = &qr.server;

    let code: Element<'_, Message> = match &qr.qr {
        Some(handle) => image(handle)
            .width(260)
            .height(260)
            .filter_method(image::FilterMethod::Nearest)
            .into(),
        None => text("The address is too long for a QR code.").into(),
    };

    let warning = (!server.reachable()).then(|| {
        text(
            "This computer doesn't seem to be in a network: the 3DS can't reach it. Connect it \
             to the same Wi-Fi as the 3DS and open this again.",
        )
        .size(12)
    });

    let downloads = server
        .downloads()
        .into_iter()
        .map(|d| {
            let name = server.file_name(d.file).unwrap_or_default();
            let what = match d.state {
                DownloadState::Running if d.total > 0 => format!(
                    "{} is downloading {name}: {} of {} ({}%)",
                    d.client,
                    Size::from_bytes(d.sent),
                    Size::from_bytes(d.total),
                    d.sent * 100 / d.total
                ),
                DownloadState::Running => format!("{} is downloading {name}", d.client),
                DownloadState::Done => format!(
                    "{} has downloaded {name}; FBI finishes the install on the 3DS",
                    d.client
                ),
                DownloadState::Aborted => {
                    format!("{} stopped downloading {name} halfway", d.client)
                }
            };
            text(what).size(12).into()
        })
        .collect::<Column<'_, _>>()
        .spacing(4);

    let urls = server
        .urls()
        .into_iter()
        .map(|url| text(url).size(12).into())
        .collect::<Column<'_, _>>();

    my_card(
        column![
            container(
                column![
                    text!("Install {} with FBI", qr.name).size(18),
                    row![
                        container(code).padding(10),
                        column![
                            text(
                                "On the 3DS, in the same Wi-Fi as this computer: open FBI, \
                                 choose \"Remote Install\", then \"Scan QR Code\", and point the \
                                 camera at this code. FBI downloads the CIA from this computer \
                                 and installs it."
                            ),
                            text("Without the camera, FBI can take the address by hand:").size(12),
                            urls,
                            warning,
                            downloads,
                        ]
                        .spacing(10)
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Start),
                ]
                .spacing(10)
            )
            .padding(10),
            rule::horizontal(1),
            row![
                text("The CIA is served until this window is closed.").size(12),
                space::horizontal(),
                my_button().label("Close").on_press(Message::CloseModal),
            ]
            .spacing(10)
            .padding(10)
            .align_y(Alignment::Center),
        ]
        .width(720)
        .height(Length::Shrink),
    )
    .padding(0)
    .into()
}
