//! The menu bar of the classic client: a stone
//! strip of buttons that open the map, the paperdoll, the backpack, the
//! journal, chat, help, the world map, the info bar, the debug gump, the
//! network statistics, the chat again as the reference client's "Global Chat", the buff
//! window and the options. The
//! arrow at its left folds it to a small stub, and the profile keeps it
//! folded. A right click puts it back in the top left corner; it never
//! closes. A button for a gump whose kind
//! is not registered does not show.

use super::canvas::{ButtonArt, Canvas};
use super::chat::chat_button;
use super::journal::journal_kind;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::windows::PAPERDOLL_COMMAND;
use crate::window::control::Act;
use eframe::egui::Pos2;

pub const TOP_BAR: GumpKind = GumpKind {
    id: well_known::TOP_BAR,
    rules: GumpRules {
        right_click_closes: false,
        first_place: Pos2::ZERO,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(TopBar),
};

const STRIP: u16 = 0x13BE;
pub(super) const STRIP_HEIGHT: i32 = 27;
const STUB_WIDTH: i32 = 30;
const FOLD: u16 = 0x15A4;
const UNFOLD: u16 = 0x15A1;
const ARROW_AT: (i32, i32) = (5, 3);
const SMALL_BUTTON: u16 = 0x098B;
const LARGE_BUTTON: u16 = 0x098D;
const FIRST_BUTTON_X: i32 = 30;
const BUTTON_Y: i32 = 1;
const BUTTON_GAP: i32 = 1;
const CAPTION_FONT: u8 = 1;
const CAPTION_HUE: u16 = 0;
const CAPTION_HOVER_HUE: u16 = 0x0036;
const OPTIONS_WORDS: &str = "Options";
const BUFFS_WORDS: &str = "Buffs";

/// What a button of the bar does.
#[derive(Clone, Copy)]
enum Press {
    Toggle(&'static str),
    /// The journal the Journal page asks for.
    Journal,
    /// The info bar, which the Info Bar page turns on and off.
    InfoBar,
    Paperdoll,
    Backpack,
    Chat,
    Help,
}

/// One button: large or small art, its text number and English words, and
/// what it does.
struct Entry {
    large: bool,
    number: Option<u32>,
    words: &'static str,
    press: Press,
}

const ENTRIES: [Entry; 13] = [
    Entry {
        large: false,
        number: Some(3_000_430),
        words: "Map",
        press: Press::Toggle(well_known::MINIMAP),
    },
    Entry {
        large: true,
        number: Some(3_000_133),
        words: "Paperdoll",
        press: Press::Paperdoll,
    },
    Entry {
        large: true,
        number: Some(3_000_431),
        words: "Inventory",
        press: Press::Backpack,
    },
    Entry {
        large: true,
        number: Some(3_000_129),
        words: "Journal",
        press: Press::Journal,
    },
    Entry {
        large: false,
        number: Some(3_000_131),
        words: "Chat",
        press: Press::Chat,
    },
    Entry {
        large: false,
        number: Some(3_000_134),
        words: "Help",
        press: Press::Help,
    },
    Entry {
        large: true,
        number: Some(1_015_233),
        words: "World Map",
        press: Press::Toggle(well_known::WORLD_MAP),
    },
    Entry {
        large: false,
        number: Some(1_079_449),
        words: "Info",
        press: Press::InfoBar,
    },
    Entry {
        large: false,
        number: Some(1_042_237),
        words: "Debug",
        press: Press::Toggle(well_known::DEBUG),
    },
    Entry {
        large: true,
        number: Some(3_000_169),
        words: "NetStats",
        press: Press::Toggle(well_known::NET_STATS),
    },
    Entry {
        large: true,
        number: Some(1_158_390),
        words: "Global Chat",
        press: Press::Chat,
    },
    Entry {
        large: false,
        number: None,
        words: BUFFS_WORDS,
        press: Press::Toggle(well_known::BUFFS),
    },
    Entry {
        large: true,
        number: None,
        words: OPTIONS_WORDS,
        press: Press::Toggle(well_known::OPTIONS),
    },
];

/// Every word starts with a capital, as the classic bar writes "World Map".
pub(super) fn capitalized(words: &str) -> String {
    words
        .split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct TopBar;

impl GumpBody for TopBar {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if g.right_click() && g.at(0, 0) != Pos2::ZERO {
            g.move_to(Pos2::ZERO);
        }
        if cx.folded() {
            g.frame(0, 0, STUB_WIDTH, STRIP_HEIGHT, STRIP);
            let arrow = ButtonArt::new(UNFOLD, UNFOLD, UNFOLD);
            if g.button("unfold", ARROW_AT.0, ARROW_AT.1, arrow) {
                cx.set_folded(false);
            }
            return;
        }
        let shown: Vec<&Entry> = ENTRIES
            .iter()
            .filter(|entry| match entry.press {
                Press::Toggle(kind) => cx.has_kind(kind),
                _ => true,
            })
            .collect();
        let widths: Vec<i32> = shown
            .iter()
            .map(|entry| {
                let art = if entry.large {
                    LARGE_BUTTON
                } else {
                    SMALL_BUTTON
                };
                g.gump_size(art).map_or(0, |size| size.x as i32)
            })
            .collect();
        let width =
            FIRST_BUTTON_X + widths.iter().map(|w| w + BUTTON_GAP).sum::<i32>() + BUTTON_GAP;
        g.frame(0, 0, width, STRIP_HEIGHT, STRIP);
        let arrow = ButtonArt::new(FOLD, FOLD, FOLD);
        if g.button("fold", ARROW_AT.0, ARROW_AT.1, arrow) {
            cx.set_folded(true);
        }
        let look = TextLook::unicode(CAPTION_FONT, CAPTION_HUE);
        let mut x = FIRST_BUTTON_X;
        for (entry, width) in shown.into_iter().zip(widths) {
            let art = if entry.large {
                LARGE_BUTTON
            } else {
                SMALL_BUTTON
            };
            let words = match entry.number {
                Some(number) => capitalized(&g.words(number, entry.words)),
                None => entry.words.to_string(),
            };
            let art = ButtonArt::new(art, art, art);
            if g.caption_button(
                entry.words,
                x,
                BUTTON_Y,
                art,
                &words,
                &look,
                CAPTION_HOVER_HUE,
            ) {
                press(entry.press, cx);
            }
            x += width + BUTTON_GAP;
        }
    }
}

fn press(press: Press, cx: &mut GumpContext<'_>) {
    match press {
        Press::Toggle(kind) => cx.toggle(GumpId::one(kind)),
        Press::Journal => cx.toggle(GumpId::one(journal_kind(cx.profile))),
        Press::InfoBar => {
            cx.profile.info_bar.enabled = !cx.profile.info_bar.enabled;
            cx.profile_changed();
        }
        Press::Paperdoll => cx.act(Act::Command(PAPERDOLL_COMMAND.into())),
        Press::Backpack => {
            if let Some(backpack) = cx.frame.backpack() {
                cx.act(Act::Use(backpack));
            }
        }
        Press::Chat => match chat_button(cx.frame) {
            Ok(id) => cx.open(id),
            Err(ask) => cx.act(ask),
        },
        Press::Help => cx.act(Act::Help),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_words_start_with_capitals() {
        assert_eq!(capitalized("world map"), "World Map");
        assert_eq!(capitalized("JOURNAL"), "Journal");
        assert_eq!(capitalized(""), "");
    }
}
