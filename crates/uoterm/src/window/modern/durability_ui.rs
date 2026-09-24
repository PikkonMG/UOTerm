//! The durability window: each worn item with a durability, the most worn
//! first, with a bar that turns to the alarm under the warning of the
//! Interface page.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::durability::{self, Wear};
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Color32, CornerRadius, Pos2, Rect, Vec2};

pub const DURABILITY_ID: &str = "modern:durability";
const WIDTH: f32 = 300.0;
const ROW: f32 = 34.0;
const ART_SIDE: f32 = 30.0;
const MAX_ROWS: usize = 12;

const WORDS_TITLE: &str = "Durability";
const WORDS_NONE: &str = "Nothing worn has a durability, or the shard has not said yet.";

/// The color of a durability bar: the alarm under the warning.
pub fn wear_color(wear: &Wear, warning: u8) -> Color32 {
    if wear.warns(warning) {
        theme::ALARM
    } else {
        theme::GOAL
    }
}

/// Draws the window. Gives its place, and true when the player closed it.
pub fn draw(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> (Rect, bool) {
    let wears = durability::worn_wear(frame, tools.readings);
    let rows = wears.len().clamp(1, MAX_ROWS);
    let height = frame::TITLE_ROW + rows as f32 * ROW + theme::PANEL_PAD * 2.0;
    let spec = PanelSpec {
        id: DURABILITY_ID,
        title: WORDS_TITLE,
        default: layout::first_place(rect, Spot::Middle(0), Vec2::new(WIDTH, height)),
        min_size: None,
        closable: true,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
    let painter = ui.painter();
    if wears.is_empty() {
        painter.text(
            body.left_top(),
            Align2::LEFT_TOP,
            WORDS_NONE,
            text_font(theme::SIZE_SMALL),
            theme::TEXT_FAINT,
        );
    }
    let warning = profile.interface.durability_warning;
    for (at, wear) in wears.iter().take(MAX_ROWS).enumerate() {
        let row = Rect::from_min_size(
            body.left_top() + Vec2::new(0.0, at as f32 * ROW),
            Vec2::new(body.width(), ROW - theme::ROW_GAP),
        );
        let art = Rect::from_min_size(row.min, Vec2::splat(ART_SIDE.min(row.height())));
        painter.rect_filled(art, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, wear.graphic, wear.hue)
        {
            painter.image(
                texture,
                theme::fit(art, sprite.width, sprite.height),
                sprite.uv,
                Color32::WHITE,
            );
        }
        let text_left = art.right() + theme::ROW_GAP;
        painter.text(
            Pos2::new(text_left, row.top()),
            Align2::LEFT_TOP,
            &wear.name,
            text_font(theme::SIZE_SMALL),
            theme::TEXT,
        );
        painter.text(
            Pos2::new(row.right(), row.top()),
            Align2::RIGHT_TOP,
            format!("{} / {}", wear.now, wear.max),
            number_font(theme::SIZE_SMALL),
            wear_color(wear, warning),
        );
        let track = Rect::from_min_max(
            Pos2::new(text_left, row.bottom() - theme::PIP_HEIGHT * 2.0),
            row.right_bottom(),
        );
        theme::bar(painter, track, wear.share(), wear_color(wear, warning));
    }
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    (panel, closed)
}
