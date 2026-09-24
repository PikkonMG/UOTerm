//! The old menus of the shard (0x7C), as the reference client draws them. A menu
//! whose first entry has a picture is a menu of pictures: the
//! question over a dark strip of item pictures that scrolls sideways by a
//! slider or the arrows at its ends; a double click on a picture answers
//! with it, and a right click closes the menu with no answer. A menu with
//! no pictures is a gray menu: the question over one radio button for
//! each entry, with Cancel and Continue; a right click does not close it.

use super::canvas::{ButtonArt, Canvas, ItemLook, SliderStyle};
use super::message_box::place_once;
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpLocks, GumpRules};
use super::text::TextLook;
use crate::view::{WatchFrame, WatchOldMenu};
use crate::window::control::Act;
use eframe::egui::{Pos2, Rect, Vec2};

pub const OLD_MENU: GumpKind = GumpKind {
    id: well_known::OLD_MENU,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(OldMenu::default()),
};

const WORDS_FONT: u8 = 1;
const WORDS_HUE: u16 = 0x0386;
const NO_HUE: u16 = 0;

// The menu of pictures.
/// Where the reference client opens a menu of pictures.
const PICTURES_PLACE: Pos2 = Pos2::new(100.0, 100.0);
const PICTURES_BACK: u16 = 0x0910;
const QUESTION_AT: (i32, i32) = (39, 18);
const QUESTION_WIDTH: u32 = 200;
const STRIP_AT: (i32, i32) = (40, 42);
const STRIP_SIZE: (i32, i32) = (217, 49);
/// The hue of the dark strip under the pictures.
const STRIP_HUE: u16 = 1;
/// A picture lower than this is centered in it.
const PICTURE_ROW: i32 = 47;
/// The slider lies this far under the strip.
const SLIDER_GAP: i32 = 12;
/// The arrows at the ends of the strip: held down, they scroll it.
const LEFT_ARROW: (i32, i32, i32, i32) = (25, 60, 10, 15);
const RIGHT_ARROW: (i32, i32, i32, i32) = (260, 60, 10, 15);
/// How far a held arrow scrolls the strip each frame.
const ARROW_STEP: i32 = 1;
/// A strip that fits its pictures does not scroll.
const NO_SCROLL: i32 = 0;

// The gray menu.
const GRAY_FRAME: u16 = 0x13EC;
const GRAY_WIDTH: i32 = 400;
const GRAY_QUESTION_AT: (i32, i32) = (20, 16);
const GRAY_QUESTION_WIDTH: u32 = 370;
const RADIO: (u16, u16) = (0x138A, 0x138B);
const RADIO_X: i32 = 50;
const RADIO_WORDS_WIDTH: u32 = 330;
/// The first entry lies this far under the top of the question.
const FIRST_ENTRY_GAP: i32 = 35;
/// The gray frame is this much taller than the top of the first entry, and
/// grows by each entry.
const GRAY_FRAME_EXTRA: i32 = 70;
/// The least height of an entry. Entries overlap by one pixel.
const ENTRY_LEAST_HEIGHT: i32 = 21;
const ENTRY_OVERLAP: i32 = 1;
/// The buttons lie this far under the last entry.
const BUTTONS_GAP: i32 = 5;
const CANCEL: ButtonArt = ButtonArt::new(0x1450, 0x1451, 0x1450);
const CANCEL_X: i32 = 70;
const CONTINUE: ButtonArt = ButtonArt::new(0x13B2, 0x13B3, 0x13B2);
const CONTINUE_X: i32 = 200;
/// The reference client opens the gray menu this far left of the middle of the window,
/// and half of this base and of a row for each entry above it.
const GRAY_LEFT_OF_MIDDLE: f32 = 200.0;
const GRAY_HEIGHT_BASE: i32 = 121;
const GRAY_ROW: i32 = 21;
const HALF: i32 = 2;
/// Pictures show at the size of their art.
const NATURAL_SIZE: f32 = 1.0;

/// A menu shows pictures when its first entry has one, as the reference client picks.
fn has_pictures(menu: &WatchOldMenu) -> bool {
    menu.entries.first().is_some_and(|entry| entry.graphic != 0)
}

/// The number the shard knows an entry by: entries count from one.
fn entry_number(at: usize) -> u16 {
    u16::try_from(at + 1).unwrap_or(u16::MAX)
}

/// Where each picture lies in the strip, from its left end and its top, and
/// how wide all of them are. Pictures stand side by side, each centered in
/// the row unless it is taller. An entry whose picture the files lack takes
/// no room.
fn strip_places(sizes: &[Vec2]) -> (Vec<Option<(i32, i32)>>, i32) {
    let mut x = 0;
    let places = sizes
        .iter()
        .map(|size| {
            let (w, h) = (size.x as i32, size.y as i32);
            if w == 0 || h == 0 {
                return None;
            }
            let y = if h >= PICTURE_ROW {
                0
            } else {
                (PICTURE_ROW - h) / HALF
            };
            let place = (x, y);
            x += w;
            Some(place)
        })
        .collect();
    (places, x)
}

/// The places of the gray menu, in its own pixels.
#[derive(Debug, PartialEq, Eq)]
struct GrayLayout {
    /// The top of each entry.
    rows: Vec<i32>,
    buttons_y: i32,
    height: i32,
}

/// Lays the gray menu out under a question of `question_height`, with
/// entries of these heights, as the reference client does.
fn gray_layout(question_height: i32, entry_heights: &[i32]) -> GrayLayout {
    let mut y = FIRST_ENTRY_GAP + question_height;
    let mut height = GRAY_FRAME_EXTRA + y;
    let mut rows = Vec::with_capacity(entry_heights.len());
    for entry in entry_heights {
        rows.push(y);
        let entry = (*entry).max(ENTRY_LEAST_HEIGHT);
        y += entry - ENTRY_OVERLAP;
        height += entry;
    }
    GrayLayout {
        rows,
        buttons_y: y + BUTTONS_GAP,
        height,
    }
}

/// Where the reference client opens a gray menu of `count` entries.
fn gray_place(screen: Rect, count: usize) -> Pos2 {
    let rows = i32::try_from(count).unwrap_or(i32::MAX);
    let half_height = GRAY_HEIGHT_BASE.saturating_add(rows.saturating_mul(GRAY_ROW)) / HALF;
    Pos2::new(
        screen.center().x - GRAY_LEFT_OF_MIDDLE,
        screen.center().y - half_height as f32,
    )
}

/// The old menu of the shard, with pictures or gray.
#[derive(Default)]
pub struct OldMenu {
    /// The menu the gump shows. A new menu starts afresh.
    shown: Option<WatchOldMenu>,
    /// How far the strip of pictures is scrolled, in pixels.
    scroll: i32,
    /// The entry the radio buttons of a gray menu picked.
    picked: Option<usize>,
    placed: bool,
}

impl OldMenu {
    /// The menu of pictures. Gives the entry the player double-clicked.
    fn pictures(&mut self, g: &mut Canvas<'_>, menu: &WatchOldMenu) -> Option<usize> {
        place_once(g, &mut self.placed, Vec2::ZERO, |_, _| PICTURES_PLACE);
        g.pic(0, 0, PICTURES_BACK, NO_HUE);
        let (strip_x, strip_y) = STRIP_AT;
        let (strip_w, strip_h) = STRIP_SIZE;
        g.hue_box(strip_x, strip_y, strip_w, strip_h, STRIP_HUE);
        let look = TextLook::ascii(WORDS_FONT, WORDS_HUE).cropped(QUESTION_WIDTH);
        g.label(QUESTION_AT.0, QUESTION_AT.1, &menu.question, &look);
        let sizes: Vec<Vec2> = menu
            .entries
            .iter()
            .map(|entry| g.item_size(entry.graphic))
            .collect();
        let (places, width) = strip_places(&sizes);
        let most = (width - strip_w).max(NO_SCROLL);
        for (key, (x, y, w, h), step) in [
            ("left", LEFT_ARROW, -ARROW_STEP),
            ("right", RIGHT_ARROW, ARROW_STEP),
        ] {
            if g.click_area(key, x, y, w, h).is_pointer_button_down_on() {
                self.scroll += step;
                g.ctx().request_repaint();
            }
        }
        self.scroll = self.scroll.clamp(0, most);
        g.slider(
            "scroll",
            strip_x,
            strip_y + strip_h + SLIDER_GAP,
            strip_w,
            (0, most),
            &mut self.scroll,
            SliderStyle::Recessed,
        );
        let scroll = self.scroll;
        g.clipped(strip_x, strip_y, strip_w, strip_h, |g| {
            let mut picked = None;
            for (at, (entry, place)) in menu.entries.iter().zip(&places).enumerate() {
                let Some((x, y)) = *place else {
                    continue;
                };
                let seen = x + sizes[at].x as i32 > scroll && x - scroll < strip_w;
                if !seen {
                    continue;
                }
                let look = ItemLook {
                    graphic: entry.graphic,
                    hue: entry.hue,
                    whole_hue: false,
                    scale: NATURAL_SIZE,
                    pile_offset: None,
                };
                let at_x = strip_x + x - scroll;
                if let Some(response) = g.item_button(("entry", at), at_x, strip_y + y, look) {
                    g.tooltip(&entry.name);
                    if response.double_clicked() {
                        picked = Some(at);
                    }
                }
            }
            picked
        })
    }

    /// The gray menu. Gives the answer when the player pressed a button:
    /// the picked entry, or none for Cancel.
    fn gray(&mut self, g: &mut Canvas<'_>, menu: &WatchOldMenu) -> Option<Option<usize>> {
        let count = menu.entries.len();
        place_once(g, &mut self.placed, Vec2::ZERO, |screen, _| {
            gray_place(screen, count)
        });
        let question = TextLook::ascii(WORDS_FONT, WORDS_HUE).wrap(GRAY_QUESTION_WIDTH);
        let words = TextLook::ascii(WORDS_FONT, WORDS_HUE).wrap(RADIO_WORDS_WIDTH);
        let radio_height = g.gump_size(RADIO.0).map_or(0, |size| size.y as i32);
        let heights: Vec<i32> = menu
            .entries
            .iter()
            .map(|entry| radio_height.max(g.measure(&entry.name, &words).y as i32))
            .collect();
        let question_height = g.measure(&menu.question, &question).y as i32;
        let layout = gray_layout(question_height, &heights);
        g.frame(0, 0, GRAY_WIDTH, layout.height, GRAY_FRAME);
        g.label(
            GRAY_QUESTION_AT.0,
            GRAY_QUESTION_AT.1,
            &menu.question,
            &question,
        );
        for (at, (entry, y)) in menu.entries.iter().zip(&layout.rows).enumerate() {
            let picked = self.picked == Some(at);
            if g.radio(
                ("entry", at),
                RADIO_X,
                *y,
                RADIO,
                picked,
                Some((&entry.name, &words)),
            ) {
                self.picked = Some(at);
            }
        }
        if g.button("cancel", CANCEL_X, layout.buttons_y, CANCEL) {
            return Some(None);
        }
        let go_on = g.button("continue", CONTINUE_X, layout.buttons_y, CONTINUE);
        // Continue does nothing until an entry is picked.
        match (go_on, self.picked) {
            (true, Some(at)) => Some(Some(at)),
            _ => None,
        }
    }
}

impl GumpBody for OldMenu {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(menu) = cx.frame.old_menu.as_ref() else {
            return;
        };
        if self.shown.as_ref() != Some(menu) {
            *self = Self {
                shown: Some(menu.clone()),
                ..Self::default()
            };
        }
        let answer = if has_pictures(menu) {
            self.pictures(g, menu).map(Some)
        } else {
            self.gray(g, menu)
        };
        if let Some(picked) = answer {
            cx.act(Act::OldMenuPick(picked.map(entry_number)));
            cx.close(cx.me);
        }
    }

    fn locks(&self, frame: &WatchFrame) -> GumpLocks {
        GumpLocks {
            no_move: false,
            // The gray menu closes only by its buttons.
            no_close: frame
                .old_menu
                .as_ref()
                .is_some_and(|menu| !has_pictures(menu)),
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::OldMenuPick(None));
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.old_menu.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPackItem;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const DAGGER: u16 = 0x0F52;
    const KRYSS: u16 = 0x1401;

    fn menu(graphics: &[u16]) -> WatchOldMenu {
        WatchOldMenu {
            question: "What do you make?".into(),
            entries: graphics
                .iter()
                .map(|graphic| WatchPackItem {
                    graphic: *graphic,
                    name: format!("item {graphic}"),
                    ..WatchPackItem::default()
                })
                .collect(),
        }
    }

    #[test]
    fn the_first_entry_picks_pictures_or_gray_lines() {
        assert!(has_pictures(&menu(&[DAGGER, 0])));
        assert!(!has_pictures(&menu(&[0, DAGGER])));
        assert!(!has_pictures(&menu(&[])));
        assert_eq!(entry_number(0), 1);
    }

    #[test]
    fn pictures_stand_side_by_side_centered_in_their_row() {
        let sizes = [
            Vec2::new(20.0, 27.0),
            Vec2::ZERO,
            Vec2::new(30.0, 60.0),
            Vec2::new(10.0, 46.0),
        ];
        let (places, width) = strip_places(&sizes);
        assert_eq!(
            places,
            vec![Some((0, 10)), None, Some((20, 0)), Some((50, 0))]
        );
        assert_eq!(width, 60);
    }

    #[test]
    fn the_gray_menu_grows_by_each_entry_as_classicuo_lays_it() {
        let layout = gray_layout(12, &[15, 30]);
        assert_eq!(layout.rows, vec![47, 67]);
        assert_eq!(layout.buttons_y, 101);
        assert_eq!(layout.height, 70 + 47 + 21 + 30);
        let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
        assert_eq!(gray_place(screen, 3), Pos2::new(200.0, 208.0));
    }

    #[test]
    fn a_right_click_closes_only_the_menu_of_pictures() {
        let body = OldMenu::default();
        let mut frame = WatchFrame {
            old_menu: Some(menu(&[DAGGER, KRYSS])),
            ..WatchFrame::default()
        };
        assert!(!body.locks(&frame).no_close);
        frame.old_menu = Some(menu(&[0, 0]));
        assert!(body.locks(&frame).no_close);
        assert!(body.alive(&frame));
        assert!(!body.alive(&WatchFrame::default()));
    }

    #[test]
    fn both_menus_draw_and_go_when_the_shard_takes_the_menu_away() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::one(well_known::OLD_MENU);
        for graphics in [&[DAGGER, KRYSS][..], &[0, 0][..]] {
            manager.open(id, &mut profile);
            let frame = WatchFrame {
                old_menu: Some(menu(graphics)),
                ..WatchFrame::default()
            };
            if !draw_frames(&mut manager, &mut profile, &frame) {
                return;
            }
            assert!(manager.is_open(&id));
            draw_frames(&mut manager, &mut profile, &WatchFrame::default());
            assert!(!manager.is_open(&id));
        }
    }
}
