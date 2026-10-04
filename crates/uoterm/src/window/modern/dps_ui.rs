//! The DPS meter: the damage meter of the session, its buttons, and the
//! damage each second over the time it ran.

use super::super::boxes_ui::Tools;
use super::super::model::dps::DamageReport;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Id, Pos2, Rect, Vec2};
use uoterm_runtime::tools::TOOL_DAMAGE_METER;
use uoterm_view::ui::launch::DPS_ID;
use uoterm_view::ui::meters::{
    dealt_words, dps_buttons, dps_first_place, meter_key, per_second_color, per_second_words,
    total_words, DPS_BUTTON_ROW as BUTTON_ROW, DPS_MAX_AGE, DPS_MAX_ROWS as MAX_ROWS,
    DPS_ROW as ROW, WORDS_DAMAGE as WORDS_TITLE, WORDS_NO_DAMAGE as WORDS_NONE,
};

/// Draws the window. Gives its place, and true when the player closed it.
pub fn draw(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> (Rect, bool) {
    let report = tools
        .readings
        .want(meter_key(), DPS_MAX_AGE)
        .map(DamageReport::read)
        .unwrap_or_default();
    let spec = PanelSpec {
        id: DPS_ID,
        title: WORDS_TITLE,
        default: bridge::rect(dps_first_place(bridge::area(rect), &report)),
        min_size: None,
        closable: true,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
    if frame.human_control {
        let buttons = dps_buttons(&report);
        let width = (body.width() - theme::ROW_GAP * 2.0) / buttons.len() as f32;
        for (at, (words, act)) in buttons.into_iter().enumerate() {
            let area = Rect::from_min_size(
                body.left_top() + Vec2::new(at as f32 * (width + theme::ROW_GAP), 0.0),
                Vec2::new(width, BUTTON_ROW - theme::ROW_GAP),
            );
            if theme::segment_keyed(ui, area, Id::new(("dps", at)), words, theme::TEXT) {
                tools.hand.act(act);
                tools.readings.refresh(TOOL_DAMAGE_METER);
            }
        }
    }
    let painter = ui.painter();
    let top = body.top() + BUTTON_ROW;
    painter.text(
        Pos2::new(body.left(), top),
        Align2::LEFT_TOP,
        per_second_words(&report),
        number_font(theme::SIZE_BODY),
        bridge::color(per_second_color(&report)),
    );
    painter.text(
        Pos2::new(body.right(), top),
        Align2::RIGHT_TOP,
        total_words(&report),
        number_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    if report.mobiles.is_empty() {
        painter.text(
            Pos2::new(body.left(), top + ROW),
            Align2::LEFT_TOP,
            WORDS_NONE,
            text_font(theme::SIZE_SMALL),
            theme::TEXT_FAINT,
        );
    }
    for (at, dealt) in report.mobiles.iter().take(MAX_ROWS).enumerate() {
        let y = top + (at + 1) as f32 * ROW;
        painter.text(
            Pos2::new(body.left(), y),
            Align2::LEFT_TOP,
            &dealt.name,
            text_font(theme::SIZE_SMALL),
            theme::TEXT,
        );
        painter.text(
            Pos2::new(body.right(), y),
            Align2::RIGHT_TOP,
            dealt_words(dealt),
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
    }
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    (panel, closed)
}
