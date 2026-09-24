//! A bulletin board, as the reference client draws it: the board
//! with its name, the list of its messages (author, title and date), each
//! answer under the message it answers, and the blank paper at its left
//! that starts a new message. A double click on a message asks the shard
//! for its words and opens it (`bulletin_post`). Closing the board closes
//! its messages too.

use super::bulletin_post::{Post, NEW_POST};
use super::canvas::Canvas;
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::{WatchFrame, WatchPost};
use crate::window::control::Act;
use crate::window::model::pages::{shown_depth, threaded};
use uoterm_nav::TextAlign;

pub const BULLETIN_BOARD: GumpKind = GumpKind {
    id: well_known::BULLETIN_BOARD,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(Board::new(serial.unwrap_or_default())),
};

const BACKGROUND: u16 = 0x087A;
const NO_HUE: u16 = 0;
const NAME_X: i32 = 159;
const NAME_Y: i32 = 36;
const NAME_WIDTH: u32 = 170;
const NAME_HUE: u16 = 1;
/// The blank paper that starts a new message.
const NEW_POST_AREA: (i32, i32, i32, i32) = (15, 170, 80, 80);
// The list of the messages.
const LIST_X: i32 = 127;
const LIST_Y: i32 = 159;
const LIST_WIDTH: i32 = 241;
const LIST_ROWS: i32 = 9;
const ROW_HEIGHT: i32 = 18;
const ROW_WIDTH: i32 = 230;
const ROW_ICON: u16 = 0x1523;
const ROW_TEXT_X: i32 = 23;
const ROW_TEXT_Y: i32 = 1;
const ROW_HUE: u16 = 0;
/// An answer stands this much further right for each step of its thread.
const THREAD_INDENT: i32 = 10;
const FONT: u8 = 1;
const SUMMARY_PARTS_GAP: &str = " - ";

/// The line of a message in the list, as the reference client writes it.
fn summary(post: &WatchPost) -> String {
    [
        post.poster.as_str(),
        post.subject.as_str(),
        post.time.as_str(),
    ]
    .join(SUMMARY_PARTS_GAP)
}

/// How far right a row of a thread starts.
fn row_indent(depth: usize) -> i32 {
    THREAD_INDENT * i32::try_from(shown_depth(depth)).unwrap_or_default()
}

/// The message gumps of a board, new or read, that close with it.
fn message_gumps(frame: &WatchFrame) -> Vec<GumpId> {
    let posts = frame.board.iter().flat_map(|board| &board.posts);
    std::iter::once(NEW_POST)
        .chain(posts.map(|post| post.serial))
        .map(|serial| GumpId::of(well_known::BULLETIN_POST, serial))
        .collect()
}

pub struct Board {
    serial: u32,
}

impl Board {
    fn new(serial: u32) -> Self {
        Self { serial }
    }
}

impl GumpBody for Board {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let frame = cx.frame;
        let Some(board) = frame.board.as_ref().filter(|b| b.serial == self.serial) else {
            return;
        };
        g.pic(0, 0, BACKGROUND, NO_HUE);
        let name = TextLook::unicode(FONT, NAME_HUE)
            .wrap(NAME_WIDTH)
            .aligned(TextAlign::Center);
        g.label(NAME_X, NAME_Y, &board.name, &name);
        let (x, y, w, h) = NEW_POST_AREA;
        if g.click_area("new", x, y, w, h).clicked() {
            let new_post = GumpId::of(well_known::BULLETIN_POST, NEW_POST);
            cx.close(new_post);
            cx.open_with(new_post, Box::new(Post::new_post()));
        }
        let rows = threaded(&board.posts);
        let mut read = None;
        g.scroll_area(
            "posts",
            LIST_X,
            LIST_Y,
            LIST_WIDTH,
            ROW_HEIGHT * LIST_ROWS,
            |g| {
                let mut y = 0;
                for (post, depth) in &rows {
                    let x = row_indent(*depth);
                    let width = ROW_WIDTH - x;
                    g.pic(x, y, ROW_ICON, NO_HUE);
                    let look = TextLook::unicode(FONT, ROW_HUE)
                        .cropped(u32::try_from(width - ROW_TEXT_X).unwrap_or_default());
                    g.label(x + ROW_TEXT_X, y + ROW_TEXT_Y, &summary(post), &look);
                    let row = g.click_area(("post", post.serial), x, y, width, ROW_HEIGHT);
                    if row.double_clicked() {
                        read = Some(post.serial);
                    }
                    y += ROW_HEIGHT;
                }
                y
            },
        );
        if let Some(message) = read {
            cx.act(Act::BoardRead(message));
            cx.open(GumpId::of(well_known::BULLETIN_POST, message));
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        for id in message_gumps(cx.frame) {
            cx.close(id);
        }
        cx.act(Act::BoardClose);
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame
            .board
            .as_ref()
            .is_some_and(|board| board.serial == self.serial)
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

    fn post(serial: u32, parent: Option<u32>) -> WatchPost {
        WatchPost {
            serial,
            parent,
            poster: "Ann".into(),
            subject: "Ore".into(),
            time: "Day 1".into(),
            lines: None,
        }
    }

    fn frame() -> WatchFrame {
        WatchFrame {
            board: Some(WatchBoard {
                serial: BOARD,
                name: "town board".into(),
                reading: None,
                posts: vec![post(1, None), post(2, Some(1)), post(3, None)],
            }),
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_row_names_the_author_the_title_and_the_date_and_answers_stand_right() {
        assert_eq!(summary(&post(1, None)), "Ann - Ore - Day 1");
        assert_eq!(row_indent(0), 0);
        assert_eq!(row_indent(2), 2 * THREAD_INDENT);
        assert_eq!(row_indent(99), row_indent(shown_depth(99)));
    }

    #[test]
    fn closing_the_board_closes_its_messages() {
        let post = |serial| GumpId::of(well_known::BULLETIN_POST, serial);
        assert_eq!(
            message_gumps(&frame()),
            vec![post(NEW_POST), post(1), post(2), post(3)]
        );
        assert_eq!(message_gumps(&WatchFrame::default()), vec![post(NEW_POST)]);
    }

    #[test]
    fn the_board_lives_while_the_shard_shows_it() {
        let board = Board::new(BOARD);
        assert!(board.alive(&frame()));
        assert!(!board.alive(&WatchFrame::default()));
        assert!(!Board::new(BOARD + 1).alive(&frame()), "another board");
    }

    #[test]
    fn the_board_draws_with_the_client_files() {
        let mut profile = Profile::default();
        let mut manager = GumpManager::default();
        let id = GumpId::of(well_known::BULLETIN_BOARD, BOARD);
        manager.open(id, &mut profile);
        if !draw_frames(&mut manager, &mut profile, &frame()) {
            return;
        }
        assert!(manager.is_open(&id));
        draw_frames(&mut manager, &mut profile, &WatchFrame::default());
        assert!(!manager.is_open(&id), "the board closed");
    }
}
