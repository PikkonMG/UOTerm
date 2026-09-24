//! The questions and messages of the classic client, as one body with two
//! looks, as the reference client has them: the question (a small stone box of
//! words with Cancel and Okay) and the message box (a framed box of words
//! with Okay, or Okay and Cancel; Enter is Okay). Both are modal and open in the
//! middle of the window. A gump opens one with `cx.open_with`, giving the
//! words, the look and what the answer does. A right click answers no.
//!
//! Two kinds ask with a question: the criminal action question of the hand,
//! and the question before the game quits. The kind [`QUESTION`] is for the
//! questions of other gumps, which open it with `cx.open_with`.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::window::actions::guard::QUESTION_CRIMINAL;
use eframe::egui::{self, Key, Pos2, Rect, Vec2};

/// What the answer does: true for Okay, false for Cancel or a right click.
/// It gets the window too, so an answer can close it.
pub type OnAnswer = Box<dyn FnMut(bool, &mut GumpContext<'_>, &egui::Context)>;

/// The rules of a question and a message box: modal and kept nowhere. The
/// body takes the right click, which answers no.
pub const ASKING_RULES: GumpRules = GumpRules {
    right_click_closes: false,
    movable: false,
    modal: true,
    kept: false,
    ..GumpRules::DEFAULT
};

/// The question an attack or a spell asks when it may flag the character
/// criminal. Its answer goes to the hand, which sends the act on a yes.
pub const CRIMINAL_QUESTION: GumpKind = GumpKind {
    id: well_known::CRIMINAL_QUESTION,
    rules: ASKING_RULES,
    open: |_| {
        Box::new(MessageBox::new(
            QUESTION_CRIMINAL,
            BoxLook::Question,
            Box::new(answer_criminal),
        ))
    },
};

/// The question before the game quits, as the reference client asks it.
pub const QUIT_QUESTION: GumpKind = GumpKind {
    id: well_known::QUIT_QUESTION,
    rules: ASKING_RULES,
    open: |_| {
        Box::new(MessageBox::new(
            QUIT_WORDS,
            BoxLook::Question,
            Box::new(quit_on_yes),
        ))
    },
};

/// A question of another gump. It opens only with `cx.open_with`, with the
/// words and the answer of the gump that asks; opened by its id alone, its
/// answer does nothing.
pub const QUESTION: GumpKind = GumpKind {
    id: well_known::QUESTION,
    rules: ASKING_RULES,
    open: |_| {
        Box::new(MessageBox::new(
            String::new(),
            BoxLook::Question,
            Box::new(no_answer),
        ))
    },
};

/// The reference client's words for the question before the game quits.
pub const QUIT_WORDS: &str = "Quit\nUltima Online?";

// The question.
const QUESTION_BACK: u16 = 0x0816;
const QUESTION_CANCEL: ButtonArt = ButtonArt::new(0x0817, 0x0818, 0x0819);
const QUESTION_OKAY: ButtonArt = ButtonArt::new(0x081A, 0x081B, 0x081C);
const QUESTION_WORDS_AT: (i32, i32) = (33, 30);
const QUESTION_WORDS_WIDTH: u32 = 165;
const QUESTION_CANCEL_AT: (i32, i32) = (37, 75);
const QUESTION_OKAY_AT: (i32, i32) = (100, 75);

// The framed message box.
const BOX_FRAME: u16 = 0x0A28;
const BOX_BACKGROUND: u16 = 0x0BB8;
const BOX_OKAY: ButtonArt = ButtonArt::new(0x0481, 0x0483, 0x0482);
const BOX_CANCEL: ButtonArt = ButtonArt::new(0x047E, 0x047F, 0x0480);
const BOX_WORDS_AT: (i32, i32) = (40, 45);
/// The words wrap this much narrower than the box.
const BOX_WORDS_ROOM: i32 = 90;
const BOX_BACKGROUND_AT: (i32, i32) = (30, 40);
/// The background is this much narrower and lower than the box.
const BOX_BACKGROUND_ROOM: (i32, i32) = (60, 100);
/// The buttons stand this far over the bottom of the box.
const BOX_BUTTONS_UP: i32 = 45;
const BOX_BUTTON_GAP: i32 = 5;

const WORDS_FONT: u8 = 1;
const WORDS_HUE: u16 = 0x0386;
const NO_HUE: u16 = 0;
const HALF: i32 = 2;
const HALF_SIZE: f32 = 2.0;
/// Words wrap to at least this width.
const LEAST_WRAP: i32 = 1;

/// An answer that does nothing.
fn no_answer(_yes: bool, _cx: &mut GumpContext<'_>, _window: &egui::Context) {}

/// The criminal action question answers the hand.
fn answer_criminal(yes: bool, cx: &mut GumpContext<'_>, _window: &egui::Context) {
    cx.hand.answer(yes);
}

/// A yes leaves the world and closes the window.
fn quit_on_yes(yes: bool, cx: &mut GumpContext<'_>, window: &egui::Context) {
    if yes {
        cx.hand.quit(window);
    }
}

/// The top left corner that puts a box of `size` in the middle of the
/// window, as the reference client centers its questions.
pub fn middle(screen: Rect, size: Vec2) -> Pos2 {
    screen.center() - size / HALF_SIZE
}

/// Moves a gump the first time it draws to the place `place` finds for it
/// from the window and the gump's size in window points.
pub fn place_once(
    g: &mut Canvas<'_>,
    placed: &mut bool,
    size: Vec2,
    place: impl FnOnce(Rect, Vec2) -> Pos2,
) {
    if !*placed {
        *placed = true;
        let shown = g.area(0, 0, size).size();
        let screen = g.ctx().screen_rect();
        g.move_to(place(screen, shown));
    }
}

/// How a message box looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxLook {
    /// The small stone question with Cancel and Okay.
    Question,
    /// A frame of `size` gump pixels with Okay, and Cancel when asked; a
    /// paper background behind the words when asked.
    Framed {
        size: (i32, i32),
        cancel: bool,
        background: bool,
    },
}

/// The answer the buttons gave this frame: Okay wins.
fn answer_of(okay: bool, cancel: bool) -> Option<bool> {
    match (okay, cancel) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

/// Where the Okay button and the Cancel button (when there is one) stand
/// in a framed box of `size`, by the widths of their pictures.
fn framed_buttons(
    size: (i32, i32),
    cancel: bool,
    okay_width: i32,
    cancel_width: i32,
) -> ((i32, i32), Option<(i32, i32)>) {
    let (w, h) = size;
    let y = h - BOX_BUTTONS_UP;
    if cancel {
        let okay_x = w / HALF - cancel_width;
        let cancel_x = okay_x + okay_width + BOX_BUTTON_GAP;
        ((okay_x, y), Some((cancel_x, y)))
    } else {
        (((w - okay_width) / HALF, y), None)
    }
}

/// A question or a message of the classic client.
pub struct MessageBox {
    words: String,
    look: BoxLook,
    on_answer: OnAnswer,
    placed: bool,
}

impl MessageBox {
    pub fn new(words: impl Into<String>, look: BoxLook, on_answer: OnAnswer) -> Self {
        Self {
            words: words.into(),
            look,
            on_answer,
            placed: false,
        }
    }

    /// A message in a frame of `size` with Okay alone, whose answer does
    /// nothing, as the reference client's message box tells the player a thing.
    pub fn told(words: impl Into<String>, size: (i32, i32)) -> Self {
        let look = BoxLook::Framed {
            size,
            cancel: false,
            background: false,
        };
        Self::new(words, look, Box::new(no_answer))
    }

    /// The question. Gives the answer of this frame.
    fn question(&mut self, g: &mut Canvas<'_>) -> Option<bool> {
        let size = g.gump_size(QUESTION_BACK).unwrap_or(Vec2::ZERO);
        place_once(g, &mut self.placed, size, middle);
        g.pic(0, 0, QUESTION_BACK, NO_HUE);
        let look = TextLook::ascii(WORDS_FONT, WORDS_HUE).wrap(QUESTION_WORDS_WIDTH);
        g.label(QUESTION_WORDS_AT.0, QUESTION_WORDS_AT.1, &self.words, &look);
        let (cancel_x, cancel_y) = QUESTION_CANCEL_AT;
        let cancel = g.button("cancel", cancel_x, cancel_y, QUESTION_CANCEL);
        let (okay_x, okay_y) = QUESTION_OKAY_AT;
        let okay = g.button("okay", okay_x, okay_y, QUESTION_OKAY);
        answer_of(okay, cancel)
    }

    /// The framed box. Gives the answer of this frame.
    fn framed(
        &mut self,
        g: &mut Canvas<'_>,
        size: (i32, i32),
        cancel: bool,
        background: bool,
    ) -> Option<bool> {
        let (w, h) = size;
        place_once(g, &mut self.placed, Vec2::new(w as f32, h as f32), middle);
        g.frame(0, 0, w, h, BOX_FRAME);
        if background {
            let (x, y) = BOX_BACKGROUND_AT;
            let (less_w, less_h) = BOX_BACKGROUND_ROOM;
            g.frame(x, y, w - less_w, h - less_h, BOX_BACKGROUND);
        }
        let wrap = (w - BOX_WORDS_ROOM).max(LEAST_WRAP) as u32;
        let look = TextLook::ascii(WORDS_FONT, WORDS_HUE).wrap(wrap);
        g.label(BOX_WORDS_AT.0, BOX_WORDS_AT.1, &self.words, &look);
        let width = |g: &mut Canvas<'_>, art: ButtonArt| {
            g.gump_size(art.normal).map_or(0, |size| size.x as i32)
        };
        let (okay_width, cancel_width) = (width(g, BOX_OKAY), width(g, BOX_CANCEL));
        let (okay_at, cancel_at) = framed_buttons(size, cancel, okay_width, cancel_width);
        let entered = g.ctx().input(|i| i.key_pressed(Key::Enter));
        let okay = g.button("okay", okay_at.0, okay_at.1, BOX_OKAY) || entered;
        let cancel = cancel_at.is_some_and(|(x, y)| g.button("cancel", x, y, BOX_CANCEL));
        answer_of(okay, cancel)
    }
}

impl GumpBody for MessageBox {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let answer = match self.look {
            BoxLook::Question => self.question(g),
            BoxLook::Framed {
                size,
                cancel,
                background,
            } => self.framed(g, size, cancel, background),
        };
        let answer = answer.or_else(|| g.right_click().then_some(false));
        if let Some(yes) = answer {
            (self.on_answer)(yes, cx, g.ctx());
            cx.close(cx.me);
        }
    }

    /// A framed box takes Enter.
    fn wants_keys(&self) -> bool {
        matches!(self.look, BoxLook::Framed { .. })
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
    use std::cell::Cell;
    use std::rc::Rc;

    const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));

    #[test]
    fn a_box_opens_in_the_middle_and_okay_wins() {
        assert_eq!(
            middle(SCREEN, Vec2::new(200.0, 100.0)),
            Pos2::new(300.0, 250.0)
        );
        assert_eq!(answer_of(true, true), Some(true));
        assert_eq!(answer_of(false, true), Some(false));
        assert_eq!(answer_of(false, false), None);
    }

    #[test]
    fn a_told_message_is_framed_with_okay_alone_and_takes_enter() {
        let told = MessageBox::told("Cannot delete this group.", (200, 125));
        assert_eq!(
            told.look,
            BoxLook::Framed {
                size: (200, 125),
                cancel: false,
                background: false,
            }
        );
        assert!(told.wants_keys());
    }

    #[test]
    fn a_framed_box_centers_one_button_or_sets_two_side_by_side() {
        const OKAY_WIDTH: i32 = 40;
        const CANCEL_WIDTH: i32 = 60;
        let size = (300, 200);
        assert_eq!(
            framed_buttons(size, false, OKAY_WIDTH, CANCEL_WIDTH),
            ((130, 155), None)
        );
        assert_eq!(
            framed_buttons(size, true, OKAY_WIDTH, CANCEL_WIDTH),
            ((90, 155), Some((135, 155)))
        );
    }

    #[test]
    fn the_questions_are_modal_and_answer_no_on_a_right_click() {
        for kind in [CRIMINAL_QUESTION, QUIT_QUESTION, QUESTION] {
            assert!(kind.rules.modal && !kind.rules.movable && !kind.rules.kept);
            assert!(!kind.rules.right_click_closes, "the body answers it");
        }
        assert_eq!(QUIT_QUESTION.id, well_known::QUIT_QUESTION);
        assert_eq!(CRIMINAL_QUESTION.id, well_known::CRIMINAL_QUESTION);
        let framed = BoxLook::Framed {
            size: (250, 150),
            cancel: true,
            background: true,
        };
        assert!(MessageBox::new("Saved.", framed, Box::new(no_answer)).wants_keys());
    }

    #[test]
    fn both_looks_draw_and_wait_for_their_answer() {
        let answered = Rc::new(Cell::new(None));
        let seen = answered.clone();
        let on_answer: OnAnswer = Box::new(move |yes, _, _| seen.set(Some(yes)));
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let question = GumpId::one(well_known::QUIT_QUESTION);
        let asking = MessageBox::new(QUIT_WORDS, BoxLook::Question, on_answer);
        manager.open_body(question, Box::new(asking), &mut profile);
        let told = GumpId::one(well_known::CRIMINAL_QUESTION);
        let framed = BoxLook::Framed {
            size: (250, 150),
            cancel: false,
            background: true,
        };
        let message = MessageBox::new("Saved.", framed, Box::new(no_answer));
        manager.open_body(told, Box::new(message), &mut profile);
        if !draw_frames(&mut manager, &mut profile, &WatchFrame::default()) {
            return;
        }
        assert!(manager.is_open(&question) && manager.is_open(&told));
        assert_eq!(answered.get(), None, "no button was pressed");
    }
}
