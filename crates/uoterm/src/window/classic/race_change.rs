//! The race change window of the shard (`0xBF` `0x2A`), as the reference client
//! draws it: the hair and beard styles in drop-down lists
//! at the left, the skin, hair and beard colors at the right, the paperdoll
//! of the new looks in the middle, and the arrow that sends them. A color
//! box opens its palette over the right side; a click on a hue picks it,
//! and a click beside the palette shuts it. A right click on the window
//! says no to the shard.

use super::canvas::{ButtonArt, Canvas};
use super::doll_order::{body_gump, equipment_gump};
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::WatchFrame;
use crate::window::control::Act;
use crate::window::model::race_change::{
    doll_body, paints, palette, palette_columns, style_lists, Paint, RacePicks,
};
use eframe::egui::{Color32, Pos2};
use std::cell::RefCell;
use uoterm_nav::{TileFlagSet, HUE_ID_MASK, HUE_PARTIAL_BIT};
use uoterm_world::{Race, RaceChange, PALETTE_ROWS};

/// Where the window stands on the screen. It does not move.
const SCREEN_AT: (i32, i32) = (50, 50);

pub const RACE_CHANGE: GumpKind = GumpKind {
    id: well_known::RACE_CHANGE,
    rules: GumpRules {
        movable: false,
        kept: false,
        first_place: Pos2::new(SCREEN_AT.0 as f32, SCREEN_AT.1 as f32),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(RaceChangeGump::default()),
};

const NO_HUE: u16 = 0;
// The stone backgrounds: the window, and the panels at each side.
const BACKGROUND: u16 = 0x0E10;
const WINDOW_SIZE: (i32, i32) = (595, 400);
const LEFT_PANEL: (i32, i32) = (25, 45);
const RIGHT_PANEL: (i32, i32) = (419, 45);
const PANEL_SIZE: (i32, i32) = (151, 310);
// The words under the panels: the race, ending at the right of the left
// panel, and the sex.
const RACE_WORDS_RIGHT: i32 = 176;
const WORDS_Y: i32 = 360;
const HUMAN_WORDS: (u16, i32) = (0x0702, 79);
const ELF_WORDS: (u16, i32) = (0x0705, 79);
const GARGOYLE_WORDS: (u16, i32) = (0x07D4, 99);
const FEMALE_WORDS: u16 = 0x070D;
const MALE_WORDS: u16 = 0x0710;
const SEX_WORDS_X: i32 = 419;
/// The arrow that sends the looks.
const CONFIRM: ButtonArt = ButtonArt::new(0x15A4, 0x15A6, 0x15A5);
const CONFIRM_AT: (i32, i32) = (560, 360);
// The paperdoll and the title over it.
const DOLL_BACKGROUND: u16 = 0x0708;
const DOLL_BACKGROUND_AT: (i32, i32) = (185, 25);
const DOLL_AT: (i32, i32) = (210, 75);
const TITLE: u16 = 0x0769;
const TITLE_AT: (i32, i32) = (211, 15);
// The styles at the left, and the colors at the right.
const STYLES_AT: (i32, i32) = (40, 60);
const COLORS_AT: (i32, i32) = (434, 60);
const LABEL_FONT: u8 = 9;
const LABEL_HUE: u16 = 0;
/// A label stands one pixel right of its control.
const LABEL_INDENT: i32 = 1;
const LABEL_HEIGHT: i32 = 15;
const COMBO_WIDTH: i32 = 120;
/// From a style label to the next one.
const STYLE_STEP: i32 = LABEL_HEIGHT + 30;
/// From a color label to the next one.
const COLOR_STEP: i32 = 42;
const COLOR_BOX_AT: (i32, i32) = (1, LABEL_HEIGHT);
const COLOR_BOX_SIZE: (i32, i32) = (121, 23);
// The palette of a color, over the right panel: where it opens on the
// screen, and the room its cells share.
const PALETTE_SCREEN_AT: (i32, i32) = (485, 109);
const PALETTE_AT: (i32, i32) = (
    PALETTE_SCREEN_AT.0 - SCREEN_AT.0,
    PALETTE_SCREEN_AT.1 - SCREEN_AT.1,
);
const PALETTE_ROOM: (i32, i32) = (125, 280);
const MARK_SIDE: i32 = 2;
const HALF: i32 = 2;
/// The picture of the race words, and its width.
fn race_words(race: Race) -> (u16, i32) {
    match race {
        Race::Human => HUMAN_WORDS,
        Race::Elf => ELF_WORDS,
        Race::Gargoyle => GARGOYLE_WORDS,
    }
}

/// The columns of a palette grid and the size of one cell.
fn grid(hue_count: usize) -> (usize, i32, i32) {
    let columns = palette_columns(hue_count);
    (
        columns,
        PALETTE_ROOM.0 / columns as i32,
        PALETTE_ROOM.1 / PALETTE_ROWS as i32,
    )
}

/// The top left corner of one cell of a palette, from the palette's own.
fn cell_at(index: usize, columns: usize, cell: (i32, i32)) -> (i32, i32) {
    let (row, column) = (index / columns, index % columns);
    (column as i32 * cell.0, row as i32 * cell.1)
}

/// The race change window: the picks of the player, and the palette that
/// is open.
#[derive(Debug, Default)]
pub struct RaceChangeGump {
    picks: RacePicks,
    picking: Option<Paint>,
}

impl RaceChangeGump {
    /// The drop-down lists of the hair and the beard styles.
    fn styles(&mut self, g: &mut Canvas<'_>, change: RaceChange) {
        let look = TextLook::ascii(LABEL_FONT, LABEL_HUE);
        let (x, mut y) = STYLES_AT;
        for (part, (number, fallback), styles) in style_lists(change) {
            let place = self.picks.style_place(part);
            let key = ("style", part as u8);
            g.label(x + LABEL_INDENT, y, &g.words(number, fallback), &look);
            let names: Vec<String> = styles
                .iter()
                .map(|style| g.words(style.name, style.words))
                .collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            g.combobox(key, x, y + LABEL_HEIGHT, COMBO_WIDTH, &names, place);
            y += STYLE_STEP;
        }
    }

    /// The color boxes. A click on one opens its palette.
    fn colors(&mut self, g: &mut Canvas<'_>, change: RaceChange) {
        let look = TextLook::ascii(LABEL_FONT, LABEL_HUE);
        let (x, mut y) = COLORS_AT;
        for (paint, (number, fallback)) in paints(change) {
            g.label(x, y, &g.words(number, fallback), &look);
            let (box_x, box_y) = (x + COLOR_BOX_AT.0, y + COLOR_BOX_AT.1);
            let (w, h) = COLOR_BOX_SIZE;
            g.hue_box(box_x, box_y, w, h, self.picks.hue(change, paint));
            if g.click_area(("color", paint as u8), box_x, box_y, w, h)
                .clicked()
            {
                self.picking = Some(paint);
            }
            y += COLOR_STEP;
        }
    }

    /// The paperdoll of the new looks: the body in the skin hue, then the
    /// hair and the beard. A style of none shows nothing.
    fn doll(&self, g: &mut Canvas<'_>, change: RaceChange) {
        let body = doll_body(change);
        let (x, y) = DOLL_AT;
        let shown = body_gump(body, change.female);
        let skin = shown.hue.unwrap_or(self.picks.hue(change, Paint::Skin));
        g.pic(x, y, shown.gump, skin | HUE_PARTIAL_BIT);
        let looks = self.picks.looks(change);
        for (graphic, hue) in [(looks.hair, looks.hair_hue), (looks.beard, looks.beard_hue)] {
            if let Some((gump, partial)) = worn_gump(g, body, change.female, graphic) {
                let partial_bit = if partial { HUE_PARTIAL_BIT } else { 0 };
                g.pic(x, y, gump, (hue & HUE_ID_MASK) | partial_bit);
            }
        }
    }

    /// The open palette. It takes every click on the window: a hue picks
    /// that hue, and a click beside the palette shuts it.
    fn draw_palette(&mut self, g: &mut Canvas<'_>, change: RaceChange, paint: Paint) {
        let (w, h) = WINDOW_SIZE;
        let beside = g.click_area("beside palette", 0, 0, w, h).clicked();
        let hues = palette(change, paint);
        let (columns, cell_w, cell_h) = grid(hues.len());
        let (left, top) = PALETTE_AT;
        let mut picked = None;
        for (index, hue) in hues.iter().enumerate() {
            let (dx, dy) = cell_at(index, columns, (cell_w, cell_h));
            let (x, y) = (left + dx, top + dy);
            g.hue_box(x, y, cell_w, cell_h, *hue);
            if g.click_area(("cell", index), x, y, cell_w, cell_h)
                .clicked()
            {
                picked = Some(index);
            }
        }
        let place = self.picks.hue_place(paint);
        let (dx, dy) = cell_at(*place, columns, (cell_w, cell_h));
        let mark_x = left + dx + cell_w / HALF - MARK_SIDE / HALF;
        let mark_y = top + dy + cell_h / HALF - MARK_SIDE / HALF;
        g.fill(mark_x, mark_y, MARK_SIDE, MARK_SIDE, Color32::WHITE);
        if let Some(index) = picked {
            *place = index;
        }
        if picked.is_some() || beside {
            self.picking = None;
        }
    }
}

/// The paperdoll gump of a worn hair or beard graphic, and whether its hue
/// colors only its gray pixels. None for no style, or when the files lack
/// it. Hair and beards have no conversion on the bodies this window shows.
fn worn_gump(g: &mut Canvas<'_>, body: u16, female: bool, graphic: u16) -> Option<(u16, bool)> {
    if graphic == 0 {
        return None;
    }
    let tile = g.scene.item_tile(graphic)?;
    let (anim, partial) = (tile.anim_id, tile.flags.contains(TileFlagSet::PARTIAL_HUE));
    let canvas = RefCell::new(g);
    let exists = |gump: u16| canvas.borrow_mut().gump_size(gump).is_some();
    equipment_gump(body, anim, female, None, exists).map(|gump| (gump, partial))
}

impl GumpBody for RaceChangeGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(change) = cx.frame.race_change else {
            return;
        };
        if self.picks.change != Some(change) {
            self.picking = None;
        }
        self.picks.follow(change);
        let (w, h) = WINDOW_SIZE;
        g.frame(0, 0, w, h, BACKGROUND);
        let (panel_w, panel_h) = PANEL_SIZE;
        for (x, y) in [LEFT_PANEL, RIGHT_PANEL] {
            g.frame(x, y, panel_w, panel_h, BACKGROUND);
        }
        let (words, width) = race_words(change.race);
        g.pic(RACE_WORDS_RIGHT - width, WORDS_Y, words, NO_HUE);
        let sex = if change.female {
            FEMALE_WORDS
        } else {
            MALE_WORDS
        };
        g.pic(SEX_WORDS_X, WORDS_Y, sex, NO_HUE);
        if g.button("confirm", CONFIRM_AT.0, CONFIRM_AT.1, CONFIRM) {
            cx.act(Act::RaceChange(Some(self.picks.looks(change))));
            cx.close(cx.me);
        }
        self.styles(g, change);
        self.colors(g, change);
        g.pic(
            DOLL_BACKGROUND_AT.0,
            DOLL_BACKGROUND_AT.1,
            DOLL_BACKGROUND,
            NO_HUE,
        );
        self.doll(g, change);
        g.pic(TITLE_AT.0, TITLE_AT.1, TITLE, NO_HUE);
        if let Some(paint) = self.picking {
            self.draw_palette(g, change, paint);
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::RaceChange(None));
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.race_change.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const HUMAN_MAN: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };
    const ELF_WOMAN: RaceChange = RaceChange {
        race: Race::Elf,
        female: true,
    };
    const GARGOYLE_MAN: RaceChange = RaceChange {
        race: Race::Gargoyle,
        female: false,
    };

    /// The gargoyle words are wider.
    #[test]
    fn the_race_words_follow_the_race() {
        assert_eq!(race_words(Race::Gargoyle), GARGOYLE_WORDS);
        assert_eq!(RACE_WORDS_RIGHT - race_words(Race::Elf).1, 97);
    }

    /// The palette of a human's skin is eight by eight cells of fifteen by
    /// thirty-five pixels over the right panel, as the reference client
    /// lays it.
    #[test]
    fn a_palette_lays_its_hues_in_eight_rows() {
        assert_eq!(PALETTE_AT, (435, 59));
        let (columns, cell_w, cell_h) = grid(HUMAN_MAN.skin_hues().len());
        assert_eq!((columns, cell_w, cell_h), (8, 15, 35));
        assert_eq!(cell_at(9, columns, (cell_w, cell_h)), (15, 35));
        let (columns, cell_w, _) = grid(GARGOYLE_MAN.hair_hues().len());
        assert_eq!((columns, cell_w), (2, 62));
    }

    #[test]
    fn the_window_shows_while_the_shard_waits_and_goes_with_the_request() {
        for change in [HUMAN_MAN, ELF_WOMAN, GARGOYLE_MAN] {
            let frame = WatchFrame {
                race_change: Some(change),
                ..WatchFrame::default()
            };
            let mut profile = Profile::default();
            let mut manager = GumpManager::default();
            let id = GumpId::one(well_known::RACE_CHANGE);
            manager.open(id, &mut profile);
            if !draw_frames(&mut manager, &mut profile, &frame) {
                return;
            }
            assert!(manager.is_open(&id));
            draw_frames(&mut manager, &mut profile, &WatchFrame::default());
            assert!(!manager.is_open(&id), "the window closed");
        }
    }
}
