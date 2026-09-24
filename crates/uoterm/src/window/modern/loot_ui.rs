//! The nearby-loot window: the corpses round the character, nearest first.
//! One click opens a corpse, or loots it by the loot list of the session.
//! Corpses open by themselves by the corpse options of the General page.

use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::model::agents::AUTOLOOT;
use super::super::model::loot::{self, NEARBY_LOOT_TILES};
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Id, Pos2, Rect, Vec2};
use std::collections::HashSet;

pub const LOOT_ID: &str = "modern:loot";
const WIDTH: f32 = 300.0;
const ROW: f32 = 26.0;
const MAX_ROWS: usize = 8;
const BUTTON_WIDTH: f32 = 54.0;
const BUTTON_GAP: f32 = 6.0;

const WORDS_TITLE: &str = "Nearby loot";
const WORDS_OPEN: &str = "Open";
const WORDS_LOOT: &str = "Loot";
const WORDS_LOOT_ALL: &str = "Loot all in reach";
const WORDS_NONE: &str = "No corpse near.";
const WORDS_EMPTY: &str = "empty";
const WORDS_SHUT: &str = "shut";

#[derive(Default)]
pub struct LootUi {
    /// The corpses the window opened, so each opens once.
    opened: HashSet<u32>,
}

impl LootUi {
    /// Opens the corpses the corpse options ask for. It runs whether the
    /// window shows or not.
    pub fn open_corpses(&mut self, frame: &WatchFrame, tools: &Tools<'_>, profile: &Profile) {
        self.opened
            .retain(|serial| frame.items.iter().any(|item| item.serial == *serial));
        if !frame.human_control {
            return;
        }
        for corpse in loot::corpses_to_open(&profile.general, frame, &self.opened) {
            self.opened.insert(corpse);
            tools.hand.act(Act::Use(corpse));
        }
    }

    /// Draws the window. Gives its place, and true when the player closed
    /// it.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> (Rect, bool) {
        let corpses = loot::nearby_corpses(frame, NEARBY_LOOT_TILES);
        let rows = corpses.len().clamp(1, MAX_ROWS) + 1;
        let height = frame::TITLE_ROW + rows as f32 * ROW + theme::PANEL_PAD * 2.0;
        let spec = PanelSpec {
            id: LOOT_ID,
            title: WORDS_TITLE,
            default: layout::first_place(rect, Spot::RightColumn(0), Vec2::new(WIDTH, height)),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
        let live = frame.human_control;
        if corpses.is_empty() {
            ui.painter().text(
                body.left_top(),
                Align2::LEFT_TOP,
                WORDS_NONE,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
        }
        for (at, corpse) in corpses.iter().take(MAX_ROWS).enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, at as f32 * ROW),
                Vec2::new(body.width(), ROW - BUTTON_GAP / 2.0),
            );
            let state = match (corpse.open, corpse.items) {
                (false, _) => WORDS_SHUT.to_string(),
                (true, 0) => WORDS_EMPTY.to_string(),
                (true, items) => items.to_string(),
            };
            ui.painter().text(
                row.left_center(),
                Align2::LEFT_CENTER,
                &corpse.name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            let button = |from_right: f32| {
                Rect::from_min_size(
                    Pos2::new(row.right() - from_right, row.top()),
                    Vec2::new(BUTTON_WIDTH, row.height()),
                )
            };
            let (loot_area, open_area) = (
                button(BUTTON_WIDTH),
                button(BUTTON_WIDTH * 2.0 + BUTTON_GAP),
            );
            ui.painter().text(
                Pos2::new(open_area.left() - BUTTON_GAP, row.center().y),
                Align2::RIGHT_CENTER,
                format!("{state}  {}", corpse.distance),
                number_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            );
            if !live {
                continue;
            }
            if theme::segment_keyed(
                ui,
                open_area,
                Id::new(("loot-open", corpse.serial)),
                WORDS_OPEN,
                theme::TEXT,
            ) {
                self.opened.insert(corpse.serial);
                tools.hand.act(Act::Use(corpse.serial));
            }
            if theme::segment_keyed(
                ui,
                loot_area,
                Id::new(("loot-one", corpse.serial)),
                WORDS_LOOT,
                theme::GOAL,
            ) {
                tools.hand.act(Act::Loot(corpse.serial));
            }
        }
        if live && !corpses.is_empty() {
            let all = Rect::from_min_size(
                Pos2::new(body.left(), body.bottom() - ROW + BUTTON_GAP / 2.0),
                Vec2::new(body.width(), ROW - BUTTON_GAP / 2.0),
            );
            if theme::segment(ui, all, WORDS_LOOT_ALL, theme::GOAL) {
                tools.hand.act(Act::AgentRun {
                    agent: AUTOLOOT.to_string(),
                    list: None,
                });
            }
        }
        let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        (panel, closed)
    }
}
