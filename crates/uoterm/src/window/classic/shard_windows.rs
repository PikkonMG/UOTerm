//! The windows the shard opens that are not gumps of the shard: books,
//! bulletin boards, map items, the old menus, the text entry dialog, the
//! prompt, the chat, tips and notices, and the questions the window asks
//! before an act. In the Classic style each one opens as a classic gump
//! when it comes. The player may close it; it opens again when the shard
//! opens it again.

use super::manager::GumpManager;
use super::registry::{well_known, GumpId};
use crate::view::WatchFrame;
use crate::window::control::Hand;
use crate::window::settings::Profile;
use std::collections::HashSet;

/// The classic gumps of what the shard shows now.
fn wanted(frame: &WatchFrame, question: bool) -> Vec<GumpId> {
    let mut ids = Vec::new();
    if let Some(book) = &frame.book {
        ids.push(GumpId::of(well_known::BOOK, book.serial));
    }
    if let Some(board) = &frame.board {
        ids.push(GumpId::of(well_known::BULLETIN_BOARD, board.serial));
    }
    ids.extend(
        frame
            .maps
            .iter()
            .map(|map| GumpId::of(well_known::MAP_ITEM, map.serial)),
    );
    let single = [
        (frame.old_menu.is_some(), well_known::OLD_MENU),
        (frame.text_entry.is_some(), well_known::TEXT_ENTRY),
        (frame.prompt, well_known::PROMPT),
        (frame.chat.is_some(), well_known::CHAT),
        (frame.chat_asks_for_name, well_known::CHAT_NAME),
        (question, well_known::CRIMINAL_QUESTION),
        (frame.race_change.is_some(), well_known::RACE_CHANGE),
    ];
    ids.extend(
        single
            .into_iter()
            .filter(|(shown, _)| *shown)
            .map(|(_, kind)| GumpId::one(kind)),
    );
    ids
}

/// What the shard showed in the last frame, so each window opens once.
#[derive(Default)]
pub struct ShardWindows {
    present: HashSet<GumpId>,
    /// The last notice of the shard that opened a tip gump.
    notice: Option<String>,
}

impl ShardWindows {
    /// Opens the gump of each window the shard opened since the last frame.
    pub fn sync(
        &mut self,
        manager: &mut GumpManager,
        frame: &WatchFrame,
        hand: &Hand,
        profile: &mut Profile,
    ) {
        let now: HashSet<GumpId> = wanted(frame, hand.question().is_some())
            .into_iter()
            .collect();
        for id in now.difference(&self.present) {
            if !manager.is_open(id) {
                manager.open(*id, profile);
            }
        }
        self.present = now;
        if frame.shard_notice != self.notice {
            if frame.shard_notice.is_some() {
                manager.open(GumpId::one(well_known::TIP_NOTICE), profile);
            }
            self.notice.clone_from(&frame.shard_notice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchBoard, WatchBook, WatchMap};

    #[test]
    fn each_window_of_the_shard_names_its_gump() {
        let frame = WatchFrame {
            book: Some(WatchBook {
                serial: 0x40,
                ..WatchBook::default()
            }),
            board: Some(WatchBoard {
                serial: 0x41,
                ..WatchBoard::default()
            }),
            maps: vec![WatchMap {
                serial: 0x42,
                ..WatchMap::default()
            }],
            prompt: true,
            chat_asks_for_name: true,
            ..WatchFrame::default()
        };
        let ids = wanted(&frame, true);
        assert_eq!(
            ids,
            vec![
                GumpId::of(well_known::BOOK, 0x40),
                GumpId::of(well_known::BULLETIN_BOARD, 0x41),
                GumpId::of(well_known::MAP_ITEM, 0x42),
                GumpId::one(well_known::PROMPT),
                GumpId::one(well_known::CHAT_NAME),
                GumpId::one(well_known::CRIMINAL_QUESTION),
            ]
        );
        assert!(wanted(&WatchFrame::default(), false).is_empty());
    }
}
