//! One message of a bulletin board, as the reference client
//! draws it on a scroll of paper the player makes longer: the author, the
//! date and the title over a rule, and the words under it. A message of
//! someone else has a Reply button, which opens a new message that answers
//! it; the player's own message has a Remove button. A new message has a
//! title box, a box for its words and a Post button.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::{WatchFrame, WatchPost};
use crate::window::control::Act;
use crate::window::model::pages::fitted;
use eframe::egui::Pos2;

pub const BULLETIN_POST: GumpKind = GumpKind {
    id: well_known::BULLETIN_POST,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| {
        Box::new(match serial.unwrap_or(NEW_POST) {
            NEW_POST => Post::new_post(),
            message => Post::reading(message),
        })
    },
};

/// The serial the gump of a new message opens under.
pub const NEW_POST: u32 = 0;

const ARTICLE_SCROLL: u16 = 0x0820;
const ARTICLE_HEIGHT: i32 = 408;
const NO_HUE: u16 = 0;
// The labels over the rule.
const LABEL_X: i32 = 30;
const AUTHOR_Y: i32 = 40;
const DATE_Y: i32 = 58;
/// The date stands two pixels further from its label than the others.
const DATE_GAP: i32 = 2;
const TITLE_Y: i32 = 77;
const SUBJECT_WIDTH: i32 = 150;
const RULE: u16 = 0x0835;
const RULE_AT: (i32, i32) = (30, 106);
const RULE_SIZE: (i32, i32) = (235, 4);
// The words of the message, in a box that scrolls.
const BODY_Y: i32 = 120;
const BODY_WIDTH: i32 = 272;
/// The box of the words is this much shorter than the scroll.
const BODY_ROOM: i32 = 184;
const BODY_X: i32 = 40;
const BODY_TEXT_WIDTH: i32 = 220;
const BODY_PAD: i32 = 5;
const BODY_LEAST_HEIGHT: i32 = 20;
// The buttons, this far over the foot of the scroll.
const WRITE_HEADER: u16 = 0x0883;
const WRITE_HEADER_AT: (i32, i32) = (97, 12);
const POST_BUTTON: u16 = 0x0886;
const REPLY_BUTTON: u16 = 0x0884;
const REMOVE_BUTTON: u16 = 0x0885;
const BUTTON_X: i32 = 37;
const REMOVE_X: i32 = 235;
const BUTTON_OVER_FOOT: i32 = 50;
// The words.
const FONT: u8 = 1;
const TEXT_HUE: u16 = 0;
/// The title of a new message is written in this hue.
const NEW_SUBJECT_HUE: u16 = 0x0008;
const WORDS_AUTHOR: &str = "Author:";
const WORDS_DATE: &str = "Date:";
const WORDS_TITLE: &str = "Title:";
const WORDS_DATE_TIME: &str = "Date/Time";
const REPLY_PREFIX: &str = "RE: ";
const NEW_LINE: &str = "\n";
// Where each kind first opens.
const READ_PLACE: Pos2 = Pos2::new(40.0, 40.0);
const WRITE_PLACE: Pos2 = Pos2::new(400.0, 335.0);

/// The message the gump shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    /// A message of the board, by its serial.
    Read(u32),
    /// A new message, which may answer another one.
    Write { reply_to: Option<u32>, time: String },
}

/// What the gump lets the player do, as the reference client's variants of the gump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Write,
    /// Someone else's message.
    Reply,
    /// The player's own message.
    Remove,
}

fn variant(mode: &Mode, poster: &str, me: &str) -> Variant {
    match mode {
        Mode::Write { .. } => Variant::Write,
        Mode::Read(_) if poster == me => Variant::Remove,
        Mode::Read(_) => Variant::Reply,
    }
}

pub struct Post {
    mode: Mode,
    subject: TextField,
    text: TextField,
    /// The board the gump showed first, so it closes with that board.
    board: Option<u32>,
}

impl Post {
    fn reading(message: u32) -> Self {
        Self {
            mode: Mode::Read(message),
            subject: TextField::default(),
            text: TextField::default(),
            board: None,
        }
    }

    /// A new message with no title, as the blank paper of a board starts
    /// one.
    pub fn new_post() -> Self {
        Self::writing(None, String::new(), WORDS_DATE_TIME.to_string())
    }

    /// A new message with a title, that answers `reply_to` when it is
    /// some, dated with `time`.
    fn writing(reply_to: Option<u32>, subject: String, time: String) -> Self {
        let mut text = TextField::default();
        text.multiline = true;
        Self {
            mode: Mode::Write { reply_to, time },
            subject: TextField::new(&subject),
            text,
            board: None,
        }
    }

    /// The new message that answers a message.
    fn reply(post: &WatchPost) -> Self {
        Self::writing(
            Some(post.serial),
            format!("{REPLY_PREFIX}{}", post.subject),
            post.time.clone(),
        )
    }

    /// The act of the Post button. None while the title is empty, as the
    /// shard takes no message without one.
    fn post_act(&self) -> Option<Act> {
        let Mode::Write { reply_to, .. } = &self.mode else {
            return None;
        };
        let subject = self.subject.text().trim();
        (!subject.is_empty()).then(|| Act::BoardPost {
            subject: subject.to_string(),
            text: self.text.text().to_string(),
            reply_to: *reply_to,
        })
    }

    /// The words of the message: a box to write them in, or the lines
    /// that came. Gives their height.
    fn body(&mut self, g: &mut Canvas<'_>, post: Option<&WatchPost>, look: &TextLook) -> i32 {
        if !matches!(self.mode, Mode::Write { .. }) {
            let words = post
                .and_then(|post| post.lines.as_ref())
                .map(|lines| lines.join(NEW_LINE))
                .unwrap_or_default();
            let wrapped = look.wrap(BODY_TEXT_WIDTH as u32);
            return g.label(BODY_X, 0, words.trim_start(), &wrapped).y as i32;
        }
        let line_height = g.text.fonts.line_height(look) as i32;
        let lines = self.text.text().split(NEW_LINE).count() as i32;
        let height = (lines * line_height + BODY_PAD).max(BODY_LEAST_HEIGHT);
        let outcome = g.text_box(
            "text",
            BODY_X,
            0,
            BODY_TEXT_WIDTH,
            height,
            &mut self.text,
            look,
        );
        if outcome.changed {
            let fonts = &g.text.fonts;
            let fits = |line: &str| fonts.width(look.font, line) <= BODY_TEXT_WIDTH as u32;
            if let Some((words, caret)) =
                fitted(self.text.text(), self.text.caret(), fits, usize::MAX)
            {
                if words != self.text.text() {
                    self.text.set_text(&words);
                    self.text.place_caret(caret, false);
                }
            }
        }
        height
    }
}

impl GumpBody for Post {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let frame = cx.frame;
        let Some(board) = frame.board.as_ref() else {
            return;
        };
        self.board.get_or_insert(board.serial);
        let post = match self.mode {
            Mode::Read(message) => board.posts.iter().find(|post| post.serial == message),
            Mode::Write { .. } => None,
        };
        let (poster, time, subject) = match (&self.mode, post) {
            (Mode::Write { time, .. }, _) => (frame.name.as_str(), time.as_str(), ""),
            (_, Some(post)) => (
                post.poster.as_str(),
                post.time.as_str(),
                post.subject.as_str(),
            ),
            (_, None) => ("", "", ""),
        };
        let variant = variant(&self.mode, poster, &frame.name);
        let height = g.expandable_scroll(0, 0, ARTICLE_SCROLL, ARTICLE_HEIGHT).y as i32;
        let look = TextLook::unicode(FONT, TEXT_HUE);
        let width = g.label(LABEL_X, AUTHOR_Y, WORDS_AUTHOR, &look).x as i32;
        g.label(LABEL_X + width, AUTHOR_Y, poster, &look);
        let width = g.label(LABEL_X, DATE_Y, WORDS_DATE, &look).x as i32;
        g.label(LABEL_X + DATE_GAP + width, DATE_Y, time, &look);
        let width = g.label(LABEL_X, TITLE_Y, WORDS_TITLE, &look).x as i32;
        if variant == Variant::Write {
            let subject_look = TextLook::unicode(FONT, NEW_SUBJECT_HUE);
            let line_height = g.text.fonts.line_height(&subject_look) as i32;
            g.text_box(
                "subject",
                LABEL_X + width,
                TITLE_Y,
                SUBJECT_WIDTH,
                line_height,
                &mut self.subject,
                &subject_look,
            );
            g.pic(WRITE_HEADER_AT.0, WRITE_HEADER_AT.1, WRITE_HEADER, NO_HUE);
        } else {
            let cropped = look.cropped(SUBJECT_WIDTH as u32);
            g.label(LABEL_X + width, TITLE_Y, subject, &cropped);
        }
        g.pic_tiled(RULE_AT.0, RULE_AT.1, RULE_SIZE.0, RULE_SIZE.1, RULE, NO_HUE);
        g.scroll_area("body", 0, BODY_Y, BODY_WIDTH, height - BODY_ROOM, |g| {
            self.body(g, post, &look)
        });
        let button_y = height - BUTTON_OVER_FOOT;
        let art = |gump| ButtonArt::new(gump, gump, gump);
        match variant {
            Variant::Write => {
                if g.button("post", BUTTON_X, button_y, art(POST_BUTTON)) {
                    if let Some(act) = self.post_act() {
                        cx.act(act);
                        cx.close(cx.me);
                    }
                }
            }
            Variant::Reply => {
                if g.button("reply", BUTTON_X, button_y, art(REPLY_BUTTON)) {
                    if let Some(post) = post {
                        let new_post = GumpId::of(well_known::BULLETIN_POST, NEW_POST);
                        cx.close(new_post);
                        cx.open_with(new_post, Box::new(Post::reply(post)));
                        cx.close(cx.me);
                    }
                }
            }
            Variant::Remove => {
                if g.button("remove", REMOVE_X, button_y, art(REMOVE_BUTTON)) {
                    if let Mode::Read(message) = self.mode {
                        cx.act(Act::BoardRemove(message));
                        cx.close(cx.me);
                    }
                }
            }
        }
    }

    fn first_place(&self, _frame: &WatchFrame) -> Option<Pos2> {
        Some(match self.mode {
            Mode::Read(_) => READ_PLACE,
            Mode::Write { .. } => WRITE_PLACE,
        })
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        let Some(board) = frame.board.as_ref() else {
            return false;
        };
        let same_board = self.board.is_none_or(|serial| serial == board.serial);
        let there = match self.mode {
            Mode::Read(message) => board.posts.iter().any(|post| post.serial == message),
            Mode::Write { .. } => true,
        };
        same_board && there
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchBoard;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const BOARD: u32 = 0x4000_0A00;
    const MESSAGE: u32 = 0x4000_0A01;
    const ME: &str = "Ann";

    fn message(poster: &str) -> WatchPost {
        WatchPost {
            serial: MESSAGE,
            parent: None,
            poster: poster.into(),
            subject: "Ore".into(),
            time: "Day 1".into(),
            lines: Some(vec!["I buy ore.".into(), "Bring it.".into()]),
        }
    }

    fn frame(posts: Vec<WatchPost>) -> WatchFrame {
        WatchFrame {
            name: ME.into(),
            human_control: true,
            board: Some(WatchBoard {
                serial: BOARD,
                name: "town board".into(),
                reading: None,
                posts,
            }),
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_own_message_is_removed_and_another_answered() {
        let read = Mode::Read(MESSAGE);
        assert_eq!(variant(&read, ME, ME), Variant::Remove);
        assert_eq!(variant(&read, "Bob", ME), Variant::Reply);
        let write = Mode::Write {
            reply_to: None,
            time: String::new(),
        };
        assert_eq!(variant(&write, ME, ME), Variant::Write);
    }

    #[test]
    fn a_reply_answers_its_message_and_posts_only_with_a_title() {
        let mut reply = Post::reply(&message("Bob"));
        assert_eq!(reply.subject.text(), "RE: Ore");
        assert_eq!(
            reply.mode,
            Mode::Write {
                reply_to: Some(MESSAGE),
                time: "Day 1".into()
            }
        );
        reply.text.set_text("How much?");
        assert_eq!(
            reply.post_act(),
            Some(Act::BoardPost {
                subject: "RE: Ore".into(),
                text: "How much?".into(),
                reply_to: Some(MESSAGE),
            })
        );
        let mut new = Post::new_post();
        assert_eq!(new.post_act(), None, "no title");
        new.subject.set_text("  Sale ");
        assert_eq!(
            new.post_act(),
            Some(Act::BoardPost {
                subject: "Sale".into(),
                text: String::new(),
                reply_to: None,
            })
        );
        assert_eq!(Post::reading(MESSAGE).post_act(), None);
    }

    #[test]
    fn a_message_lives_while_its_board_holds_it() {
        let read = Post::reading(MESSAGE);
        assert!(!read.alive(&WatchFrame::default()));
        assert!(!read.alive(&frame(Vec::new())), "the message is gone");
        assert!(read.alive(&frame(vec![message("Bob")])));
        let mut new = Post::writing(None, String::new(), String::new());
        assert!(new.alive(&frame(Vec::new())));
        new.board = Some(BOARD + 1);
        assert!(!new.alive(&frame(Vec::new())), "another board opened");
    }

    #[test]
    fn each_kind_of_message_draws_with_the_client_files() {
        for (poster, serial) in [("Bob", MESSAGE), (ME, MESSAGE), (ME, NEW_POST)] {
            let frame = frame(vec![message(poster)]);
            let mut profile = Profile::default();
            let mut manager = GumpManager::default();
            let id = GumpId::of(well_known::BULLETIN_POST, serial);
            manager.open(id, &mut profile);
            if !draw_frames(&mut manager, &mut profile, &frame) {
                return;
            }
            assert!(manager.is_open(&id));
        }
    }
}
