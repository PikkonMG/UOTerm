//! The info bar of the classic client: a dark
//! see-through strip with the items of the Info Bar page, each label in its
//! hue and each value in the hue that tells when it runs low, or in plain
//! words with a warning line over and under it, as the page's highlight
//! asks. It shows while the Info Bar page has it on; a right click does not
//! close it.

use super::canvas::Canvas;
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::window::model::info_bar::{self, HUE_FINE};
use crate::window::settings::{InfoBarData, InfoBarHighlight};

pub const INFO_BAR: GumpKind = GumpKind {
    id: well_known::INFO_BAR,
    rules: GumpRules {
        right_click_closes: false,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(InfoBar),
};

const HEIGHT: i32 = 20;
const OPACITY: f32 = 0.7;
const BLACK_HUE: u16 = 0;
const FIRST_X: i32 = 5;
const ITEM_GAP: i32 = 5;
const WORDS_FONT: u8 = 1;
/// The warning lines of the colored-bar highlight are this thick.
const WARNING_LINE: i32 = 2;

/// The hue of a value's words, and the hue of its warning lines when it
/// has them, by the highlight of the page. The name always shows its hue.
fn value_hues(data: InfoBarData, highlight: InfoBarHighlight, hue: u16) -> (u16, Option<u16>) {
    match highlight {
        InfoBarHighlight::TextColor => (hue, None),
        _ if data == InfoBarData::Name => (hue, None),
        InfoBarHighlight::ColoredBars => (HUE_FINE, (hue != HUE_FINE).then_some(hue)),
    }
}

/// One item of the bar, measured, where it stands.
struct Part {
    x: i32,
    label: String,
    label_look: TextLook,
    label_width: i32,
    words: String,
    value_look: TextLook,
    value_width: i32,
    warning: Option<u16>,
}

pub struct InfoBar;

impl GumpBody for InfoBar {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let options = &cx.profile.info_bar;
        let mut parts = Vec::with_capacity(options.items.len());
        let mut width = FIRST_X;
        for item in &options.items {
            let value = info_bar::value_of(item.data, cx.frame, &cx.profile.combat);
            let label_look = TextLook::unicode(WORDS_FONT, item.hue);
            let (hue, warning) = value_hues(item.data, options.highlight, value.hue);
            let value_look = TextLook::unicode(WORDS_FONT, hue);
            let part = Part {
                x: width,
                label_width: g.measure(&item.label, &label_look).x as i32,
                label: item.label.clone(),
                label_look,
                value_width: g.measure(&value.words, &value_look).x as i32,
                words: value.words,
                value_look,
                warning,
            };
            width += part.label_width + part.value_width + ITEM_GAP;
            parts.push(part);
        }
        g.shade(0, 0, width, HEIGHT, BLACK_HUE, OPACITY);
        for part in parts {
            g.label(part.x, 0, &part.label, &part.label_look);
            let value_x = part.x + part.label_width;
            g.label(value_x, 0, &part.words, &part.value_look);
            if let Some(hue) = part.warning {
                g.hue_box(value_x, 0, part.value_width, WARNING_LINE, hue);
                let foot = HEIGHT - WARNING_LINE;
                g.hue_box(value_x, foot, part.value_width, WARNING_LINE, hue);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const LOW: u16 = 0x0021;

    #[test]
    fn the_highlight_colors_the_words_or_draws_warning_lines() {
        assert_eq!(
            value_hues(InfoBarData::HitPoints, InfoBarHighlight::TextColor, LOW),
            (LOW, None)
        );
        assert_eq!(
            value_hues(InfoBarData::HitPoints, InfoBarHighlight::ColoredBars, LOW),
            (HUE_FINE, Some(LOW))
        );
        assert_eq!(
            value_hues(
                InfoBarData::HitPoints,
                InfoBarHighlight::ColoredBars,
                HUE_FINE
            ),
            (HUE_FINE, None)
        );
        assert_eq!(
            value_hues(InfoBarData::Name, InfoBarHighlight::ColoredBars, LOW),
            (LOW, None)
        );
    }

    #[test]
    fn the_bar_draws_with_the_client_files() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        manager.open(GumpId::one(well_known::INFO_BAR), &mut profile);
        let frame = WatchFrame {
            name: "Mara".into(),
            hits: 10,
            hits_max: 50,
            ..WatchFrame::default()
        };
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&GumpId::one(well_known::INFO_BAR)));
    }
}
