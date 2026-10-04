//! The Modern windows where the shard asks the player something, as both
//! windows show them: the dialog for words the shard waits for, the race
//! change, and the tip of the day or the notice. Where each first stands,
//! its words, and what the player keeps of it between frames. The rules of
//! the answers are `model::asked` and `model::race_change`.

use super::layout::{first_place, Spot};
use super::places::{FOOT_ROW, TITLE_ROW};
use super::theme::{PANEL_PAD, ROW_GAP};
use crate::frame::WatchFrame;
use crate::geom::{Area, Vector};
use crate::model::asked::{kept_words, AskedDialog};
use crate::model::race_change::{Paint, RacePicks, StylePart};
use uoterm_world::RaceChange;

pub const ENTRY_ID: &str = "modern:entry";
pub const ENTRY_WIDTH: f32 = 400.0;
pub const FIELD_ROW: f32 = 30.0;
const WORDS_ASKS: &str = "The shard asks";
pub const WORDS_OKAY: &str = "Okay";
pub const WORDS_CANCEL: &str = "Cancel";
pub const WORDS_TAKE_CONTROL: &str = "Take control to answer.";
const HINT_WORDS: &str = "Type the answer and press Enter.";
const HINT_DIGITS: &str = "Digits only. Press Enter.";

pub const RACE_ID: &str = "modern:race_change";
const RACE_SIZE: Vector = Vector::new(600.0, 420.0);
pub const WORDS_CHANGE: &str = "Change";
pub const WORDS_KEEP: &str = "Keep my looks";
pub const HINT_COLOR: &str = "Click: pick from the palette.";

pub const TIP_ID: &str = "modern:tip";
const TIP_SIZE: Vector = Vector::new(360.0, 320.0);
pub const TIP_LEAST_SIZE: Vector = Vector::new(240.0, 180.0);
const WORDS_TIP: &str = "Tip of the day";
const WORDS_NOTICE: &str = "Notice";
pub const WORDS_PREVIOUS: &str = "Previous";
pub const WORDS_NEXT: &str = "Next";

/// The title of the dialog: the shard's, or plain words with none.
pub fn entry_title(dialog: &AskedDialog) -> &str {
    if dialog.title.is_empty() {
        WORDS_ASKS
    } else {
        &dialog.title
    }
}

/// The hint of the field: digits only, or any words.
pub fn entry_hint(dialog: &AskedDialog) -> &'static str {
    if dialog.numeric {
        HINT_DIGITS
    } else {
        HINT_WORDS
    }
}

/// Where the dialog first stands in `window`, with its description
/// `description_height` tall.
pub fn entry_first_place(window: Area, description_height: f32) -> Area {
    let height = PANEL_PAD * 2.0 + TITLE_ROW + description_height + ROW_GAP + FIELD_ROW + FOOT_ROW;
    first_place(window, Spot::Middle(0), Vector::new(ENTRY_WIDTH, height))
}

/// The room of a line of the description of the dialog.
pub fn entry_text_room() -> f32 {
    ENTRY_WIDTH - PANEL_PAD * 2.0
}

/// The words typed in the dialog for the question it shows. A new
/// question gets an empty field, which takes the keys once.
#[derive(Default)]
pub struct AskedField {
    shown: Option<AskedDialog>,
    pub words: String,
    /// The field has had the keys once.
    pub focused: bool,
}

impl AskedField {
    /// Follows the question of the shard. True when a new one came.
    pub fn follow(&mut self, dialog: Option<&AskedDialog>) -> bool {
        if self.shown.as_ref() == dialog {
            return false;
        }
        self.shown = dialog.cloned();
        self.words.clear();
        self.focused = false;
        dialog.is_some()
    }

    /// Takes the words the player typed, by the rules of the shard.
    pub fn typed(&mut self, words: &str, dialog: &AskedDialog) {
        self.words = kept_words(words, dialog.numeric, dialog.max_chars);
    }
}

/// Where the race change first stands in `window`.
pub fn race_first_place(window: Area) -> Area {
    first_place(window, Spot::Middle(0), RACE_SIZE)
}

/// The picks of the race change and the palette that is open.
#[derive(Default)]
pub struct RacePanel {
    pub picks: RacePicks,
    pub picking: Option<Paint>,
}

impl RacePanel {
    /// Follows the request of the shard: another one starts over.
    pub fn follow(&mut self, change: RaceChange) {
        if self.picks.change != Some(change) {
            self.picking = None;
        }
        self.picks.follow(change);
    }

    /// The request is gone: no palette stays open.
    pub fn stop(&mut self) {
        self.picking = None;
    }

    /// A click on the color of a part opens its palette, or shuts it.
    pub fn click_paint(&mut self, paint: Paint) {
        self.picking = (self.picking != Some(paint)).then_some(paint);
    }

    /// A click on a hue of the open palette picks it and shuts the palette.
    pub fn pick_hue(&mut self, paint: Paint, at: usize) {
        *self.picks.hue_place(paint) = at;
        self.picking = None;
    }

    /// A pick of a style list.
    pub fn pick_style(&mut self, part: StylePart, at: usize) {
        *self.picks.style_place(part) = at;
    }
}

/// Where the tip or the notice first stands in `window`.
pub fn tip_first_place(window: Area) -> Area {
    first_place(window, Spot::Middle(0), TIP_SIZE)
}

/// The words of the shard the player closed, so they stay hidden until
/// the shard sends others.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoticePanel {
    closed: Option<String>,
}

/// The tip or the notice that shows: its title, its words, and whether it
/// is a tip with the buttons that ask for another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownNotice<'a> {
    pub title: &'static str,
    pub words: &'a str,
    pub tip: bool,
}

impl NoticePanel {
    /// What shows now. No words of the shard forget what was closed.
    pub fn shown<'a>(&mut self, frame: &'a WatchFrame) -> Option<ShownNotice<'a>> {
        let Some(words) = frame.shard_notice.as_deref() else {
            self.closed = None;
            return None;
        };
        if self.closed.as_deref() == Some(words) {
            return None;
        }
        let tip = frame.shard_tip.is_some();
        Some(ShownNotice {
            title: if tip { WORDS_TIP } else { WORDS_NOTICE },
            words,
            tip,
        })
    }

    pub fn close(&mut self, words: &str) {
        self.closed = Some(words.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchTextEntry, TEXT_ENTRY_STYLE_NUMERIC};
    use crate::model::asked::asked_dialog;
    use uoterm_world::Race;

    const HUMAN_MAN: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };

    #[test]
    fn the_field_keeps_the_rules_of_the_shard_and_a_new_question_empties_it() {
        let asking = WatchFrame {
            text_entry: Some(WatchTextEntry {
                title: "How many?".into(),
                style: TEXT_ENTRY_STYLE_NUMERIC,
                max_length: 3,
                ..WatchTextEntry::default()
            }),
            ..WatchFrame::default()
        };
        let dialog = asked_dialog(&asking).unwrap();
        let mut field = AskedField::default();
        assert!(field.follow(Some(&dialog)));
        field.typed("12a345", &dialog);
        assert_eq!(field.words, "123", "digits only, three at most");
        assert!(!field.follow(Some(&dialog)), "the same question");
        assert_eq!(field.words, "123");
        let prompt = asked_dialog(&WatchFrame {
            prompt: true,
            ..WatchFrame::default()
        })
        .unwrap();
        assert!(field.follow(Some(&prompt)));
        assert!(field.words.is_empty());
        assert_eq!(entry_title(&dialog), "How many?");
        assert_eq!(entry_hint(&dialog), HINT_DIGITS);
    }

    #[test]
    fn a_palette_opens_by_its_color_and_shuts_on_a_pick() {
        let mut race = RacePanel::default();
        race.follow(HUMAN_MAN);
        race.click_paint(Paint::Hair);
        assert_eq!(race.picking, Some(Paint::Hair));
        race.pick_hue(Paint::Hair, 3);
        assert_eq!(
            (race.picking, race.picks.hues[Paint::Hair as usize]),
            (None, 3)
        );
        race.click_paint(Paint::Skin);
        race.click_paint(Paint::Skin);
        assert_eq!(race.picking, None, "a second click shuts it");
        race.pick_style(StylePart::Hair, 2);
        race.follow(HUMAN_MAN);
        assert_eq!(race.picks.hair, 2, "the same request keeps the picks");
    }

    #[test]
    fn the_words_show_until_closed_and_new_words_show_again() {
        let notice = |words: &str| WatchFrame {
            shard_notice: Some(words.into()),
            ..WatchFrame::default()
        };
        let mut panel = NoticePanel::default();
        let hail = notice("Hail");
        assert_eq!(panel.shown(&hail).unwrap().title, WORDS_NOTICE);
        panel.close("Hail");
        assert!(panel.shown(&hail).is_none(), "closed");
        assert!(panel.shown(&notice("Welcome")).is_some());
        assert!(panel.shown(&WatchFrame::default()).is_none());
        assert_eq!(panel, NoticePanel::default(), "no words, nothing closed");
    }
}
