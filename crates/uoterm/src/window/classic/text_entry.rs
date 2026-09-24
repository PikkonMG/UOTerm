//! The text entry dialog of the shard (0xAB), as the reference client
//! draws it: a scroll with the title and the
//! description of the shard over one text field, with Okay and Cancel. It is
//! modal and does not move. A field of the numeric style takes digits only,
//! and the field takes no more chars than the shard allows. A right click
//! cancels it when the shard lets the player cancel.
//!
//! [`EntryDialog`] draws the scroll, the field and the buttons; the prompt
//! gump draws with it too.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpLocks, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::{WatchFrame, WatchTextEntry};
use crate::window::model::asked::{self, field_limit};
use eframe::egui::Pos2;

/// Where the reference client opens the dialog.
pub const DIALOG_PLACE: Pos2 = Pos2::new(143.0, 172.0);

pub const TEXT_ENTRY: GumpKind = GumpKind {
    id: well_known::TEXT_ENTRY,
    rules: GumpRules {
        movable: false,
        modal: true,
        kept: false,
        first_place: DIALOG_PLACE,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(TextEntry::default()),
};

const BACKGROUND: u16 = 0x0474;
const FIELD_ART: u16 = 0x0477;
const OKAY: ButtonArt = ButtonArt::new(0x047B, 0x047C, 0x047D);
const CANCEL: ButtonArt = ButtonArt::new(0x0478, 0x0478, 0x047A);
const TITLE_AT: (i32, i32) = (60, 50);
const DESCRIPTION_AT: (i32, i32) = (60, 108);
/// The words wrap this much narrower than the scroll.
const WORDS_ROOM: i32 = 110;
const FIELD_AT: (i32, i32) = (60, 130);
const TEXT_AT: (i32, i32) = (71, 137);
const TEXT_WIDTH: i32 = 250;
const OKAY_AT: (i32, i32) = (117, 190);
const CANCEL_AT: (i32, i32) = (204, 190);
const WORDS_FONT: u8 = 2;
const TEXT_FONT: u8 = 1;
const HUE: u16 = 0x0386;
const FIELD_KEY: &str = "field";
/// A field with no art has no room.
const NO_ROOM: i32 = 0;
/// Words wrap to at least this width.
const LEAST_WRAP: i32 = 1;

/// What the player did in an entry dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entered {
    /// Okay or Enter, with the typed words.
    Answer(String),
    Cancel,
}

/// What the player sees and types in: the scroll with its words, one text
/// field and the Okay and Cancel buttons.
pub struct EntryDialog {
    field: TextField,
    /// The field has had the keys once.
    focused: bool,
    /// The field takes the keys back each frame, as the only field of a
    /// modal dialog.
    hold_focus: bool,
}

impl EntryDialog {
    pub fn new(numeric: bool, max_chars: Option<usize>, hold_focus: bool) -> Self {
        let mut field = TextField::new("").with_max_chars(max_chars);
        field.numeric = numeric;
        Self {
            field,
            focused: false,
            hold_focus,
        }
    }

    /// The dialog of a text entry of the shard.
    fn for_entry(entry: &WatchTextEntry) -> Self {
        Self::new(entry.numeric(), field_limit(entry.max_length), true)
    }

    /// Draws the dialog with a title and a description. Gives what the
    /// player did this frame.
    pub fn draw(&mut self, g: &mut Canvas<'_>, title: &str, description: &str) -> Option<Entered> {
        let width = g.pic(0, 0, BACKGROUND, 0).x as i32;
        let words =
            TextLook::ascii(WORDS_FONT, HUE).wrap((width - WORDS_ROOM).max(LEAST_WRAP) as u32);
        g.label(TITLE_AT.0, TITLE_AT.1, title, &words);
        g.label(DESCRIPTION_AT.0, DESCRIPTION_AT.1, description, &words);
        let field_height = g.pic(FIELD_AT.0, FIELD_AT.1, FIELD_ART, 0).y as i32;
        let typed = g.text_box(
            FIELD_KEY,
            TEXT_AT.0,
            TEXT_AT.1,
            TEXT_WIDTH,
            (field_height - (TEXT_AT.1 - FIELD_AT.1)).max(NO_ROOM),
            &mut self.field,
            &TextLook::ascii(TEXT_FONT, HUE),
        );
        // The keys are asked for after the field draws: a field drawn
        // unseen, as on the first frame of a new layer, gives them up.
        if self.hold_focus || !self.focused {
            g.focus(FIELD_KEY);
            self.focused = true;
        }
        let okay = g.button("okay", OKAY_AT.0, OKAY_AT.1, OKAY) || typed.submitted;
        let cancel = g.button("cancel", CANCEL_AT.0, CANCEL_AT.1, CANCEL);
        if okay {
            Some(Entered::Answer(self.field.text().to_string()))
        } else if cancel {
            Some(Entered::Cancel)
        } else {
            None
        }
    }
}

/// The text entry dialog of the shard.
pub struct TextEntry {
    /// The dialog of the shard the field was made for. A new dialog gets a
    /// new, empty field.
    shown: Option<WatchTextEntry>,
    dialog: EntryDialog,
}

impl Default for TextEntry {
    fn default() -> Self {
        Self {
            shown: None,
            dialog: EntryDialog::for_entry(&WatchTextEntry::default()),
        }
    }
}

impl GumpBody for TextEntry {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(entry) = cx.frame.text_entry.as_ref() else {
            return;
        };
        if self.shown.as_ref() != Some(entry) {
            self.dialog = EntryDialog::for_entry(entry);
            self.shown = Some(entry.clone());
        }
        let act = match self.dialog.draw(g, &entry.title, &entry.description) {
            Some(Entered::Answer(words)) => asked::TEXT_ENTRY.answer_act(&words),
            Some(Entered::Cancel) => asked::TEXT_ENTRY.cancel_act(),
            None => return,
        };
        cx.act(act);
        cx.close(cx.me);
    }

    fn locks(&self, frame: &WatchFrame) -> GumpLocks {
        GumpLocks {
            no_move: true,
            no_close: !frame
                .text_entry
                .as_ref()
                .is_some_and(|entry| entry.can_cancel),
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(asked::TEXT_ENTRY.cancel_act());
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.text_entry.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::TEXT_ENTRY_STYLE_NUMERIC;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const MOST_CHARS: u32 = 3;

    fn numeric_entry(can_cancel: bool) -> WatchTextEntry {
        WatchTextEntry {
            title: "How many?".into(),
            description: "Up to 999".into(),
            can_cancel,
            style: TEXT_ENTRY_STYLE_NUMERIC,
            max_length: MOST_CHARS,
        }
    }

    #[test]
    fn a_numeric_field_takes_only_as_many_digits_as_the_shard_allows() {
        assert_eq!(field_limit(0), None);
        assert_eq!(field_limit(MOST_CHARS), Some(3));
        let mut dialog = EntryDialog::for_entry(&numeric_entry(true));
        dialog.field.set_text("12a345");
        assert_eq!(dialog.field.text(), "123");
        let words = EntryDialog::for_entry(&WatchTextEntry::default());
        assert!(!words.field.numeric && words.field.max_chars.is_none());
    }

    #[test]
    fn a_right_click_cancels_only_a_dialog_that_may_be_cancelled() {
        let body = TextEntry::default();
        let mut frame = WatchFrame {
            text_entry: Some(numeric_entry(false)),
            ..WatchFrame::default()
        };
        assert!(body.locks(&frame).no_close && body.locks(&frame).no_move);
        frame.text_entry = Some(numeric_entry(true));
        assert!(!body.locks(&frame).no_close);
        assert!(body.alive(&frame));
        assert!(!body.alive(&WatchFrame::default()));
        let rules = TEXT_ENTRY.rules;
        assert_eq!((rules.modal, rules.movable), (true, false));
        assert_eq!(TEXT_ENTRY.rules.first_place, DIALOG_PLACE);
    }

    #[test]
    fn the_dialog_stays_while_the_shard_waits_and_goes_with_it() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::one(well_known::TEXT_ENTRY);
        manager.open(id, &mut profile);
        let frame = WatchFrame {
            text_entry: Some(numeric_entry(true)),
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
