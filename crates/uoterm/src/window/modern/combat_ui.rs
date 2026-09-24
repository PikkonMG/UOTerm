//! The cooldown bars and the spell cast indicator. The bars are the rules
//! of the Combat & Spells page, started by journal lines. The indicator
//! shows the spell being cast, how far its cast is, and how many stand in
//! the range of the range circle.

use super::super::boxes_ui::Tools;
use super::super::model::casting;
use super::super::model::casting::{spell_hue, CastWatch};
use super::super::model::cooldowns::Cooldowns;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Vec2};
use uoterm_assist::spells::SpellFlag;

pub const COOLDOWNS_ID: &str = "modern:cooldowns";
pub const CAST_ID: &str = "modern:cast";
const WIDTH: f32 = 260.0;
const BAR_ROW: f32 = 26.0;
const BAR_HEIGHT: f32 = 8.0;
const CAST_HEIGHT: f32 = 64.0;
const WORDS_COOLDOWNS: &str = "Cooldowns";
const WORDS_CAST: &str = "Casting";
const WORDS_AIM: &str = "choose a target";
const WORDS_IN_RANGE: &str = "in range";

#[derive(Default)]
pub struct CombatUi {
    cooldowns: Cooldowns,
    casts: CastWatch,
}

/// The color of a spell by what it does, in the hues of the Combat page.
fn spell_color(flag: SpellFlag, profile: &Profile, tools: &Tools<'_>) -> Color32 {
    tools.scene.words_color(spell_hue(flag, &profile.combat))
}

impl CombatUi {
    /// Reads the journal and the cursor. Call it once in each frame.
    pub fn observe(&mut self, frame: &WatchFrame, profile: &Profile, time: f64) {
        self.cooldowns
            .observe(&profile.combat.cooldowns, &frame.speech, frame.serial, time);
        self.casts.observe(frame, time);
    }

    /// Draws the cooldown bars that run. Gives their place, when any runs.
    pub fn cooldown_bars(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let bars = self.cooldowns.bars(tools.time);
        if bars.is_empty() {
            return None;
        }
        let height = frame::TITLE_ROW + bars.len() as f32 * BAR_ROW + theme::PANEL_PAD * 2.0;
        let spec = PanelSpec {
            id: COOLDOWNS_ID,
            title: WORDS_COOLDOWNS,
            default: layout::first_place(rect, Spot::MiddleTop(1), Vec2::new(WIDTH, height)),
            min_size: None,
            closable: false,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_COOLDOWNS);
        let painter = ui.painter();
        for (at, bar) in bars.iter().enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, at as f32 * BAR_ROW),
                Vec2::new(body.width(), BAR_ROW - theme::ROW_GAP),
            );
            let color = tools.scene.words_color(bar.hue);
            painter.text(
                row.left_top(),
                Align2::LEFT_TOP,
                &bar.label,
                text_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
            painter.text(
                row.right_top(),
                Align2::RIGHT_TOP,
                format!("{:.1}s", bar.seconds_left(tools.time)),
                number_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            );
            let track =
                Rect::from_min_max(Pos2::new(row.left(), row.bottom() - BAR_HEIGHT), row.max);
            theme::bar(painter, track, bar.share_left(tools.time), color);
        }
        frame::controls(ui, panel, &spec, profile, tools);
        ui.ctx().request_repaint();
        Some(panel)
    }

    /// Draws the cast indicator while a cast runs. Gives its place.
    pub fn cast_indicator(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let cast = self.casts.cast()?;
        let spec = PanelSpec {
            id: CAST_ID,
            title: WORDS_CAST,
            default: layout::first_place(
                rect,
                Spot::MiddleBottom(1),
                Vec2::new(WIDTH, CAST_HEIGHT + frame::TITLE_ROW),
            ),
            min_size: None,
            closable: false,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_CAST);
        let painter = ui.painter();
        let color = spell_color(cast.flag, profile, tools);
        painter.text(
            body.left_top(),
            Align2::LEFT_TOP,
            &cast.name,
            text_font(theme::SIZE_BODY),
            color,
        );
        let range = profile.combat.range_circle_tiles;
        let reach = casting::in_range(frame, range).len();
        let right_words = if cast.aiming {
            WORDS_AIM.to_string()
        } else {
            format!("{reach} {WORDS_IN_RANGE} ({range})")
        };
        painter.text(
            body.right_top(),
            Align2::RIGHT_TOP,
            right_words,
            text_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let track =
            Rect::from_min_max(Pos2::new(body.left(), body.bottom() - BAR_HEIGHT), body.max);
        theme::bar(painter, track, cast.share_done(tools.time), color);
        frame::controls(ui, panel, &spec, profile, tools);
        ui.ctx().request_repaint();
        Some(panel)
    }
}
