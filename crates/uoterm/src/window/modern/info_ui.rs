//! The info bar: the items of the Info Bar page in one row, each label in
//! its hue and each value in the hue that tells when it runs low, or with
//! a colored bar under it.

use super::super::boxes_ui::Tools;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Pos2, Rect, Vec2};
use uoterm_view::ui::info_bar::{
    info_first_place, info_parts, INFO_BAR_HEIGHT as BAR_HEIGHT, INFO_BAR_ID,
    INFO_ITEM_GAP as ITEM_GAP, INFO_LABEL_GAP as LABEL_GAP, WORDS_INFO as WORDS_TITLE,
};

/// Draws the bar. Gives its place.
pub fn draw(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> Rect {
    let painter = ui.painter();
    let font = text_font(theme::SIZE_BODY);
    let parts = info_parts(frame, profile);
    let width = |words: &str| {
        painter
            .layout_no_wrap(words.to_string(), font.clone(), theme::TEXT)
            .size()
            .x
    };
    let spec = PanelSpec {
        id: INFO_BAR_ID,
        title: WORDS_TITLE,
        default: bridge::rect(info_first_place(bridge::area(rect), &parts, width)),
        min_size: None,
        closable: false,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(painter, panel, WORDS_TITLE);
    let mut x = body.left();
    for part in parts {
        let label = painter.layout_no_wrap(
            part.label,
            font.clone(),
            tools.scene.words_color(part.label_hue),
        );
        let words = painter.layout_no_wrap(
            part.words,
            font.clone(),
            tools.scene.words_color(part.words_hue),
        );
        let label_width = label.size().x;
        painter.galley(Pos2::new(x, body.top()), label, theme::TEXT);
        x += label_width + LABEL_GAP;
        let words_width = words.size().x;
        let words_height = words.size().y;
        painter.galley(Pos2::new(x, body.top()), words, theme::TEXT);
        if let Some(fill) = part.fill {
            let track = Rect::from_min_size(
                Pos2::new(x, body.top() + words_height),
                Vec2::new(words_width, BAR_HEIGHT),
            );
            theme::bar(painter, track, fill, tools.scene.words_color(part.bar_hue));
        }
        x += words_width + ITEM_GAP;
    }
    frame::controls(ui, panel, &spec, profile, tools);
    panel
}
