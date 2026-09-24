//! The info bar: the items of the Info Bar page in one row, each label in
//! its hue and each value in the hue that tells when it runs low, or with
//! a colored bar under it.

use super::super::boxes_ui::Tools;
use super::super::model::info_bar::{self, HUE_FINE};
use super::super::settings::InfoBarHighlight;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Pos2, Rect, Vec2};

pub const INFO_BAR_ID: &str = "modern:info_bar";
const ITEM_GAP: f32 = 14.0;
const LABEL_GAP: f32 = 4.0;
const ROW: f32 = 20.0;
const BAR_HEIGHT: f32 = 3.0;
const WORDS_TITLE: &str = "Info";

/// Draws the bar. Gives its place.
pub fn draw(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> Rect {
    let options = profile.info_bar.clone();
    let painter = ui.painter();
    let font = text_font(theme::SIZE_BODY);
    let parts: Vec<_> = options
        .items
        .iter()
        .map(|item| {
            let value = info_bar::value_of(item.data, frame, &profile.combat);
            let label = painter.layout_no_wrap(
                item.label.clone(),
                font.clone(),
                tools.scene.words_color(item.hue),
            );
            let value_color = match options.highlight {
                InfoBarHighlight::TextColor => tools.scene.words_color(value.hue),
                InfoBarHighlight::ColoredBars => tools.scene.words_color(HUE_FINE),
            };
            let words = painter.layout_no_wrap(value.words.clone(), font.clone(), value_color);
            (label, words, value)
        })
        .collect();
    let width: f32 = parts
        .iter()
        .map(|(label, words, _)| label.size().x + LABEL_GAP + words.size().x + ITEM_GAP)
        .sum::<f32>()
        .max(ITEM_GAP);
    let size = frame::with_title_room(
        Vec2::new(width - ITEM_GAP, ROW + frame::TITLE_ROW) + Vec2::splat(theme::PANEL_PAD * 2.0),
    );
    let spec = PanelSpec {
        id: INFO_BAR_ID,
        title: WORDS_TITLE,
        default: layout::first_place(rect, Spot::MiddleBottom(0), size),
        min_size: None,
        closable: false,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(painter, panel, WORDS_TITLE);
    let mut x = body.left();
    for (label, words, value) in parts {
        let label_width = label.size().x;
        painter.galley(Pos2::new(x, body.top()), label, theme::TEXT);
        x += label_width + LABEL_GAP;
        let words_width = words.size().x;
        let words_height = words.size().y;
        painter.galley(Pos2::new(x, body.top()), words, theme::TEXT);
        if let (InfoBarHighlight::ColoredBars, Some(fill)) = (options.highlight, value.fill) {
            let track = Rect::from_min_size(
                Pos2::new(x, body.top() + words_height),
                Vec2::new(words_width, BAR_HEIGHT),
            );
            theme::bar(painter, track, fill, tools.scene.words_color(value.hue));
        }
        x += words_width + ITEM_GAP;
    }
    frame::controls(ui, panel, &spec, profile, tools);
    panel
}
