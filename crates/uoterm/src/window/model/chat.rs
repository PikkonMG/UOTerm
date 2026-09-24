//! The chat of the shard, apart from how it draws: what opening the chat
//! does, when the window shows it by itself, what Join does for a channel
//! with or without a password, what the small box that asks for a channel
//! name or a password sends, the chat name the shard asks for, and how the
//! lines follow the newest one. The classic chat gumps and the Modern chat
//! panel both work through these.

use crate::view::{WatchChat, WatchFrame};
use crate::window::control::Act;

/// The chat names the shard takes are this long at most.
pub const CHAT_NAME_MAX_CHARS: usize = 30;
pub const WORDS_CHOOSE_NAME: &str = "You are about to choose your name for the Ultima Online chat \
system. This name will be PERMANENT and unique on this shard. It will apply to all characters \
using this account. Do not use your Ultima Online account name for this name as the name you \
choose will be public. Enter your chat name here:";

/// What opening the chat does now.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatDoor {
    /// The chat is on: its window shows.
    Chat,
    /// The shard asks for the chat name: the box for it shows.
    AskName,
    /// The chat is off: this act asks the shard to turn it on.
    TurnOn(Act),
}

pub fn chat_door(frame: &WatchFrame) -> ChatDoor {
    if frame.chat.is_some() {
        ChatDoor::Chat
    } else if frame.chat_asks_for_name {
        ChatDoor::AskName
    } else {
        ChatDoor::TurnOn(Act::ChatOpen(frame.name.clone()))
    }
}

/// Whether the shard showed its chat or asked for the chat name in the
/// last frame, so the window opens the chat once when the shard does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChatWatch {
    shown: bool,
}

impl ChatWatch {
    /// True in the frame the shard opened its chat or asked for the name.
    pub fn opened(&mut self, frame: &WatchFrame) -> bool {
        let shown = frame.chat.is_some() || frame.chat_asks_for_name;
        let opened = shown && !self.shown;
        self.shown = shown;
        opened
    }
}

/// What the Join button does for the picked channel.
#[derive(Clone, Debug, PartialEq)]
pub enum Joining {
    Send(Act),
    /// The channel has a password: the small box asks for it.
    AskPassword(String),
    /// No channel is picked, or the chat has no such channel now.
    Nothing,
}

pub fn joining(chat: &WatchChat, picked: Option<&str>) -> Joining {
    let found = picked.and_then(|picked| chat.channels.iter().find(|(name, _)| name == picked));
    match found {
        Some((name, true)) => Joining::AskPassword(name.clone()),
        Some((name, false)) => Joining::Send(Act::ChatJoin(name.clone())),
        None => Joining::Nothing,
    }
}

/// What the small box asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    /// The name of a new channel.
    NewChannel,
    /// The password of this channel.
    Password(String),
}

impl Asking {
    /// The field of the box hides its chars.
    pub fn hides_words(&self) -> bool {
        matches!(self, Self::Password(_))
    }

    /// The act the typed words of the box make, when they make one.
    pub fn act(&self, words: &str) -> Option<Act> {
        let words = words.trim();
        if words.is_empty() {
            return None;
        }
        Some(match self {
            Self::NewChannel => Act::ChatCreate(words.to_string()),
            Self::Password(channel) => Act::ChatJoinWithPassword {
                channel: channel.clone(),
                password: words.to_string(),
            },
        })
    }
}

/// The act a typed chat name makes, when it makes one.
pub fn chat_name_act(words: &str) -> Option<Act> {
    let name = words.trim();
    (!name.is_empty()).then(|| Act::ChatOpen(name.to_string()))
}

/// How many of the newest lines stay hidden under the view, after
/// `seen` lines became `now`: a view at the newest line follows the new
/// ones, and a view further back stays on its lines.
pub fn lines_back(back: usize, seen: usize, now: usize) -> usize {
    if back == 0 {
        return 0;
    }
    (back + now.saturating_sub(seen)).min(now.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat() -> WatchChat {
        WatchChat {
            name: "Mara".into(),
            channels: vec![("General".into(), false), ("Guild".into(), true)],
            in_channel: "General".into(),
            lines: Vec::new(),
        }
    }

    #[test]
    fn opening_the_chat_shows_it_asks_the_name_or_asks_the_shard() {
        let mut frame = WatchFrame {
            name: "Mara".into(),
            ..WatchFrame::default()
        };
        assert_eq!(
            chat_door(&frame),
            ChatDoor::TurnOn(Act::ChatOpen("Mara".into()))
        );
        frame.chat_asks_for_name = true;
        assert_eq!(chat_door(&frame), ChatDoor::AskName);
        frame.chat = Some(chat());
        assert_eq!(chat_door(&frame), ChatDoor::Chat);
    }

    #[test]
    fn the_chat_opens_once_when_the_shard_opens_it() {
        let mut watch = ChatWatch::default();
        let mut frame = WatchFrame::default();
        assert!(!watch.opened(&frame));
        frame.chat_asks_for_name = true;
        assert!(watch.opened(&frame));
        frame.chat_asks_for_name = false;
        frame.chat = Some(chat());
        assert!(!watch.opened(&frame), "the name box led to the chat");
        assert!(!watch.opened(&WatchFrame::default()));
        assert!(watch.opened(&frame), "a chat shown again opens again");
    }

    #[test]
    fn join_sends_an_open_channel_and_asks_the_password_of_a_locked_one() {
        let chat = chat();
        assert_eq!(
            joining(&chat, Some("General")),
            Joining::Send(Act::ChatJoin("General".into()))
        );
        assert_eq!(
            joining(&chat, Some("Guild")),
            Joining::AskPassword("Guild".into())
        );
        assert_eq!(joining(&chat, None), Joining::Nothing);
        assert_eq!(joining(&chat, Some("Gone")), Joining::Nothing);
    }

    #[test]
    fn the_small_box_makes_a_channel_or_joins_with_the_password() {
        assert!(!Asking::NewChannel.hides_words());
        assert_eq!(Asking::NewChannel.act("  "), None, "no name");
        assert_eq!(
            Asking::NewChannel.act(" Trade "),
            Some(Act::ChatCreate("Trade".into()))
        );
        let password = Asking::Password("Guild".into());
        assert!(password.hides_words(), "the password shows as stars");
        assert_eq!(
            password.act("pw"),
            Some(Act::ChatJoinWithPassword {
                channel: "Guild".into(),
                password: "pw".into(),
            })
        );
        assert_eq!(chat_name_act(" "), None);
        assert_eq!(chat_name_act(" Mara "), Some(Act::ChatOpen("Mara".into())));
    }

    #[test]
    fn the_lines_follow_the_newest_unless_the_player_looks_back() {
        assert_eq!(lines_back(0, 3, 5), 0, "at the newest line it follows");
        assert_eq!(lines_back(2, 3, 5), 4, "further back it stays on its lines");
        assert_eq!(lines_back(2, 3, 2), 1, "never past the oldest line");
        assert_eq!(lines_back(1, 0, 0), 0);
    }
}
