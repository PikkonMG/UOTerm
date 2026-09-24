//! The DPS meter: the damage meter of the session, its buttons, and the
//! damage each second over the time it ran.

use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::model::dps::DamageReport;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Id, Pos2, Rect, Vec2};
use serde_json::json;
use uoterm_runtime::tools::TOOL_DAMAGE_METER;

pub const DPS_ID: &str = "modern:dps";
const WIDTH: f32 = 300.0;
const ROW: f32 = 22.0;
const BUTTON_ROW: f32 = 28.0;
const MAX_ROWS: usize = 8;
/// The meter is read this often while the window shows, in seconds.
const REPORT_MAX_AGE: f64 = 1.0;
const METER_START: &str = "start";
const METER_PAUSE: &str = "pause";
const METER_RESUME: &str = "resume";
const METER_STOP: &str = "stop";

const WORDS_TITLE: &str = "Damage";
const WORDS_START: &str = "Start";
const WORDS_PAUSE: &str = "Pause";
const WORDS_RESUME: &str = "Resume";
const WORDS_STOP: &str = "Stop";
const WORDS_PER_SECOND: &str = "per second";
const WORDS_NONE: &str = "No damage counted. Press Start.";

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
        .want(TOOL_DAMAGE_METER, json!({}), REPORT_MAX_AGE)
        .map(DamageReport::read)
        .unwrap_or_default();
    let rows = report.mobiles.len().clamp(1, MAX_ROWS) + 1;
    let height = frame::TITLE_ROW + BUTTON_ROW + rows as f32 * ROW + theme::PANEL_PAD * 2.0;
    let spec = PanelSpec {
        id: DPS_ID,
        title: WORDS_TITLE,
        default: layout::first_place(rect, Spot::Middle(1), Vec2::new(WIDTH, height)),
        min_size: None,
        closable: true,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
    if frame.human_control {
        let pause = if report.running {
            (WORDS_PAUSE, METER_PAUSE)
        } else {
            (WORDS_RESUME, METER_RESUME)
        };
        let buttons = [(WORDS_START, METER_START), pause, (WORDS_STOP, METER_STOP)];
        let width = (body.width() - theme::ROW_GAP * 2.0) / buttons.len() as f32;
        for (at, (words, action)) in buttons.into_iter().enumerate() {
            let area = Rect::from_min_size(
                body.left_top() + Vec2::new(at as f32 * (width + theme::ROW_GAP), 0.0),
                Vec2::new(width, BUTTON_ROW - theme::ROW_GAP),
            );
            if theme::segment_keyed(ui, area, Id::new(("dps", at)), words, theme::TEXT) {
                tools.hand.act(Act::DamageMeter(action));
                tools.readings.refresh(TOOL_DAMAGE_METER);
            }
        }
    }
    let painter = ui.painter();
    let top = body.top() + BUTTON_ROW;
    painter.text(
        Pos2::new(body.left(), top),
        Align2::LEFT_TOP,
        format!("{:.1} {WORDS_PER_SECOND}", report.per_second()),
        number_font(theme::SIZE_BODY),
        if report.running {
            theme::GOAL
        } else {
            theme::TEXT_DIM
        },
    );
    painter.text(
        Pos2::new(body.right(), top),
        Align2::RIGHT_TOP,
        format!("{}  {:.0}s", report.total(), report.seconds),
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
            format!("{}  {:.1}/s", dealt.damage, dealt.per_second),
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
    }
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    (panel, closed)
}
