// SPDX-FileCopyrightText: 2026 Manuel Quarneti <mq1@ik.me>
// SPDX-License-Identifier: GPL-3.0-only

use crate::link::protocol::Title;
use crate::{games::game::Game, homebrew::homebrew_app::HomebrewApp, osc::osc_app::OscApp};
use std::{net::SocketAddr, path::PathBuf};

pub mod confirm_scub;
pub mod delete_dir;
pub mod delete_title;
pub mod game_info;
pub mod homebrew_app_info;
pub mod osc_app_info;

#[derive(Debug, Clone)]
pub enum Modal {
    GameInfo((Game, Option<wii_disc_info::Meta>, bool)),
    HomebrewAppInfo(HomebrewApp),
    OscAppInfo(OscApp),
    DeleteDir(PathBuf),
    ConfirmScrub(Game),
    /// A title on a console, with its name
    DeleteTitle(SocketAddr, Title, String),
}
