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
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Id, Pos2, Rect, Vec2};
use std::collections::HashSet;
use uoterm_view::ui::lists::{
    self, corpse_state_words, loot_first_place, LOOT_ID, LOOT_MAX_ROWS, LOOT_ROW,
    WORDS_LOOT_ALL_NEAR, WORDS_LOOT_ONE, WORDS_LOOT_OPEN, WORDS_LOOT_TITLE, WORDS_NO_CORPSE,
};

const BUTTON_WIDTH: f32 = 54.0;
const BUTTON_GAP: f32 = 6.0;

#[derive(Default)]
pub struct LootUi {
    /// The corpses the window opened, so each opens once.
    opened: HashSet<u32>,
}

impl LootUi {
    /// Opens the corpses the corpse options ask for. It runs whether the
    /// window shows or not.
    pub fn open_corpses(&mut self, frame: &WatchFrame, tools: &Tools<'_>, profile: &Profile) {
        for corpse in lists::open_corpses(&mut self.opened, frame, profile) {
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
        let spec = PanelSpec {
            id: LOOT_ID,
            title: WORDS_LOOT_TITLE,
            default: bridge::rect(loot_first_place(bridge::area(rect), corpses.len())),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_LOOT_TITLE);
        let live = frame.human_control;
        if corpses.is_empty() {
            ui.painter().text(
                body.left_top(),
                Align2::LEFT_TOP,
                WORDS_NO_CORPSE,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
        }
        for (at, corpse) in corpses.iter().take(LOOT_MAX_ROWS).enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, at as f32 * LOOT_ROW),
                Vec2::new(body.width(), LOOT_ROW - BUTTON_GAP / 2.0),
            );
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
                corpse_state_words(corpse),
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
                WORDS_LOOT_OPEN,
                theme::TEXT,
            ) {
                self.opened.insert(corpse.serial);
                tools.hand.act(Act::Use(corpse.serial));
            }
            if theme::segment_keyed(
                ui,
                loot_area,
                Id::new(("loot-one", corpse.serial)),
                WORDS_LOOT_ONE,
                theme::GOAL,
            ) {
                tools.hand.act(Act::Loot(corpse.serial));
            }
        }
        if live && !corpses.is_empty() {
            let all = Rect::from_min_size(
                Pos2::new(body.left(), body.bottom() - LOOT_ROW + BUTTON_GAP / 2.0),
                Vec2::new(body.width(), LOOT_ROW - BUTTON_GAP / 2.0),
            );
            if theme::segment(ui, all, WORDS_LOOT_ALL_NEAR, theme::GOAL) {
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
