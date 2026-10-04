//! The durability window: each worn item with a durability, the most worn
//! first, with a bar that turns to the alarm under the warning of the
//! Interface page.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::durability::{self, Wear};
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, Pos2, Rect, Vec2};
use uoterm_view::ui::launch::DURABILITY_ID;
use uoterm_view::ui::lists;
use uoterm_view::ui::meters::{
    durability_first_place, wear_words, DURABILITY_ART as ART_SIDE,
    DURABILITY_MAX_ROWS as MAX_ROWS, DURABILITY_ROW as ROW, WORDS_DURABILITY as WORDS_TITLE,
    WORDS_NO_WEAR as WORDS_NONE,
};

/// The color of a durability bar: the alarm under the warning.
pub fn wear_color(wear: &Wear, warning: u8) -> Color32 {
    bridge::color(lists::wear_color(wear, warning))
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
    let spec = PanelSpec {
        id: DURABILITY_ID,
        title: WORDS_TITLE,
        default: bridge::rect(durability_first_place(bridge::area(rect), wears.len())),
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
        if let Some((texture, sprite)) = tools.scene.item_picture(wear.graphic, wear.hue) {
            painter.image(
                texture,
                theme::fit(art, sprite.width, sprite.height),
                bridge::rect(sprite.uv),
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
            wear_words(wear),
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
