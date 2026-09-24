//! The chat line of the Classic style, as the reference client draws it at the foot of the game window: the words the player types in
//! the hue of the channel their prefix picks, on a half-dark band that the
//! Speech page's "hide chat gradient" takes away, and over it the shard's
//! own words and the party, guild and alliance lines for a few seconds.
//!
//! It is the one chat line of both styles (`keys::chat`): the same
//! prefixes, the same Ctrl+Q and Ctrl+W history and the same Speech page
//! rules for when it takes the keys. Only its drawing is here.

use super::canvas::Canvas;
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::WatchFrame;
use crate::window::control::Hand;
use crate::window::control_ui::chat_id;
use crate::window::keys::chat::{say_line, typed_hue, ChatLine};
use crate::window::model::system_chat::SystemChat;
use crate::window::settings::Profile;

/// The reference client's height of one line of the chat line, and the room round it.
const LINE_HEIGHT: i32 = 15;
const EDGE: i32 = 3;
/// The band behind the line is black at this opacity.
const BAND_OPACITY: f32 = 0.5;
const BAND_HUE: u16 = 0;
/// The lines over the chat line end this far over it, stand this far in
/// from the left and wrap this wide.
const LINES_GAP: i32 = 20;
const LINES_X: i32 = 2;
const LINES_WIDTH: u32 = 320;
/// The reference client's chat line takes at most this many chars.
const MOST_CHARS: usize = 500;

pub struct ClassicChat {
    field: TextField,
    lines: SystemChat,
}

impl Default for ClassicChat {
    fn default() -> Self {
        Self {
            field: TextField::new("").with_max_chars(Some(MOST_CHARS)),
            lines: SystemChat::default(),
        }
    }
}

/// What the chat line needs of the window.
pub struct ChatInputs<'a> {
    pub frame: &'a WatchFrame,
    pub profile: &'a Profile,
    pub hand: &'a Hand,
    pub time: f64,
}

impl ClassicChat {
    /// Draws the chat line at the foot of a game window of `size` gump
    /// pixels. True while lines show over it, so the window draws again
    /// to let them go.
    pub fn draw(
        &mut self,
        g: &mut Canvas<'_>,
        size: (i32, i32),
        line: &mut ChatLine,
        inputs: ChatInputs<'_>,
    ) -> bool {
        let speech = &inputs.profile.speech;
        self.lines.take(inputs.frame, speech, inputs.time);
        let (width, height) = size;
        let line_y = height - LINE_HEIGHT - EDGE;
        // The human talks only while he has the character.
        if inputs.frame.human_control && !line.is_hidden() {
            let opened = line.take_keys(g.ctx(), chat_id(), speech);
            if line.is_open(speech) {
                self.typed_line(g, (width, line_y), line, opened, &inputs);
            }
        }
        let font = inputs.profile.fonts.speech_font;
        let mut bottom = line_y - LINES_GAP;
        for shown in self.lines.lines().rev() {
            let look = TextLook::unicode(font, shown.hue)
                .bordered()
                .wrap(LINES_WIDTH);
            let top = bottom - g.measure(&shown.text, &look).y as i32;
            if top < 0 {
                break;
            }
            g.label(LINES_X, top, &shown.text, &look);
            bottom = top;
        }
        self.lines.showing()
    }

    /// The band and the words typed in it, at `y` across `width`. Enter
    /// sends the line, unless it was the Enter that opened it.
    fn typed_line(
        &mut self,
        g: &mut Canvas<'_>,
        (width, y): (i32, i32),
        line: &mut ChatLine,
        opened: bool,
        inputs: &ChatInputs<'_>,
    ) {
        let speech = &inputs.profile.speech;
        let band_height = LINE_HEIGHT + EDGE;
        if !speech.hide_chat_gradient {
            g.shade(0, y, width, band_height, BAND_HUE, BAND_OPACITY);
        }
        // Ctrl+Q and Ctrl+W change the line's words between frames.
        if self.field.text() != line.text {
            self.field.set_text(&line.text);
        }
        let look = TextLook::unicode(
            inputs.profile.fonts.speech_font,
            typed_hue(&line.text, speech),
        )
        .bordered();
        let typed = g.text_box_as(
            chat_id(),
            EDGE,
            y,
            width - EDGE,
            band_height,
            &mut self.field,
            &look,
        );
        line.text = self.field.text().to_string();
        if !typed.submitted || opened {
            return;
        }
        let shift = g.ui().input(|i| i.modifiers.shift);
        if let Some(words) = line.enter(shift, speech) {
            say_line(&words, inputs.frame, speech, inputs.hand);
        }
        self.field.set_text(&line.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::testing::{draw_canvas, idle_hand};
    use crate::window::keys::Focus;
    use crate::window::settings::SpeechOptions;
    use eframe::egui::{Event, Key, Modifiers};

    const SIZE: (i32, i32) = (640, 480);

    fn typed(words: &str) -> Vec<Event> {
        vec![Event::Text(words.into())]
    }

    fn enter() -> Vec<Event> {
        vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]
    }

    /// The line takes the keys by itself, keeps the typed words in the
    /// shared chat line, and Enter sends them and keeps them for Ctrl+Q.
    #[test]
    fn the_classic_line_types_into_the_shared_chat_line_and_sends_on_enter() {
        let hand = idle_hand();
        let frame = WatchFrame {
            human_control: true,
            ..WatchFrame::default()
        };
        let profile = Profile::default();
        let mut chat = ClassicChat::default();
        let mut line = ChatLine::default();
        let frames = [Vec::new(), typed("hail"), Vec::new(), enter(), Vec::new()];
        let mut focus = Vec::new();
        let drawn = draw_canvas(&frames, |g, ctx| {
            let inputs = ChatInputs {
                frame: &frame,
                profile: &profile,
                hand: &hand,
                time: 0.0,
            };
            chat.draw(g, SIZE, &mut line, inputs);
            focus.push((
                Focus::of(ctx, chat_id(), line.text.is_empty()),
                line.text.clone(),
            ));
        });
        if !drawn {
            return;
        }
        assert_eq!(focus[1], (Focus::ChatTyping, "hail".to_string()));
        assert_eq!(
            focus[4],
            (Focus::ChatEmpty, String::new()),
            "sent and emptied"
        );
        line.older();
        assert_eq!(line.text, "hail", "the history has the line");
        let speech = SpeechOptions::default();
        assert!(line.is_open(&speech));
    }

    #[test]
    fn a_closed_line_opens_on_enter_and_that_enter_sends_nothing() {
        let hand = idle_hand();
        let frame = WatchFrame {
            human_control: true,
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        profile.speech.chat_on_enter = true;
        let mut chat = ClassicChat::default();
        let mut line = ChatLine::default();
        let frames = [Vec::new(), enter(), Vec::new()];
        let drawn = draw_canvas(&frames, |g, _| {
            let inputs = ChatInputs {
                frame: &frame,
                profile: &profile,
                hand: &hand,
                time: 0.0,
            };
            chat.draw(g, SIZE, &mut line, inputs);
        });
        if !drawn {
            return;
        }
        assert!(line.is_open(&profile.speech));
        line.older();
        assert_eq!(line.text, "", "nothing was sent");
    }
}
