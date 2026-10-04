//! The chat panel of the Modern style, apart from how it draws: its place,
//! its words, when it opens by itself, the channel the player picked, what
//! Join, Leave and Create do, the small box that asks for a channel name or
//! a password, what the line he types says, and the channel Jev picks for
//! plain words. The rules of the chat are `model::chat`'s.

use crate::act::{Act, Answer, Ask};
use crate::frame::{WatchChat, WatchFrame};
use crate::geom::{Area, Vector};
use crate::model::chat::{joining, Asking, ChatWatch, Joining};
use crate::ui::layout::{first_place, Spot};

pub const CHAT_ID: &str = "modern:chat";
const CHAT_SIZE: Vector = Vector::new(420.0, 600.0);
const CHAT_LEAST_SIZE: Vector = Vector::new(360.0, 460.0);
pub const CHANNEL_ROW: f32 = 24.0;
pub const CHANNEL_ROWS: usize = 4;
pub const LABEL_WIDTH: f32 = 80.0;

pub const WORDS_CHAT: &str = "Chat";
pub const WORDS_OKAY: &str = "Okay";
pub const WORDS_CANCEL: &str = "Cancel";
pub const WORDS_TURN_ON: &str = "Turn the chat on";
pub const WORDS_NAME: &str = "Name:";
pub const WORDS_PASSWORD: &str = "Password:";
const WORDS_LOCKED: &str = "needs a password";
pub const WORDS_LOCK_MARK: &str = "locked";
pub const HINT_CHANNEL_ROW: &str = "Click: pick.  Double-click: join.";
pub const HINT_SAY: &str = "Words for the channel. Press Enter.";
pub const HINT_CHANNEL: &str = "Say the channel in plain words, for example: the trade one";

/// A button under the channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatButton {
    /// Join the channel the player picked.
    Join,
    Leave,
    /// Ask the name of a new channel.
    Create,
}

impl ChatButton {
    pub fn words(self) -> &'static str {
        match self {
            Self::Join => "Join",
            Self::Leave => "Leave",
            Self::Create => "Create",
        }
    }
}

pub const CHAT_BUTTONS: [ChatButton; 3] = [ChatButton::Join, ChatButton::Leave, ChatButton::Create];

/// Where the chat first stands in a window, and the least size the human
/// sizes it to.
pub fn chat_first_place(window: Area) -> (Area, Vector) {
    (
        first_place(window, Spot::LeftColumn(0), CHAT_SIZE),
        CHAT_LEAST_SIZE,
    )
}

/// The title of the chat: the channel it is in, when it is in one.
pub fn chat_title(frame: &WatchFrame) -> String {
    match frame.chat.as_ref() {
        Some(chat) if !chat.in_channel.is_empty() => {
            format!("{WORDS_CHAT}: {}", chat.in_channel)
        }
        _ => WORDS_CHAT.to_string(),
    }
}

/// The words of the label of the small box.
pub fn asking_label(asking: &Asking) -> &'static str {
    match asking {
        Asking::NewChannel => WORDS_NAME,
        Asking::Password(_) => WORDS_PASSWORD,
    }
}

/// The act a line typed for the channel makes, when it makes one.
pub fn say_act(words: &str) -> Option<Act> {
    let words = words.trim();
    (!words.is_empty()).then(|| Act::ChatSay(words.to_string()))
}

/// The question to Jev for the channel plain words mean. None for no
/// words, or a chat with no channels.
pub fn channel_ask(chat: &WatchChat, wish: &str) -> Option<Ask> {
    let wish = wish.trim();
    if wish.is_empty() || chat.channels.is_empty() {
        return None;
    }
    Some(Ask::Channel {
        wish: wish.to_string(),
        options: chat
            .channels
            .iter()
            .map(|(name, locked)| {
                if *locked {
                    format!("{name} ({WORDS_LOCKED})")
                } else {
                    name.clone()
                }
            })
            .collect(),
    })
}

/// What the chat panel keeps between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatPanel {
    pub open: bool,
    /// The channel the player picked in the list.
    pub picked: Option<String>,
    /// The small box that asks for a channel name or a password, with the
    /// words typed in it.
    pub asking: Option<(Asking, String)>,
    watch: ChatWatch,
}

impl ChatPanel {
    /// The chat as the window starts.
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            ..Self::default()
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Opens the chat in the frame the shard opens its chat or asks for
    /// the chat name.
    pub fn follow(&mut self, frame: &WatchFrame) {
        if self.watch.opened(frame) {
            self.open = true;
        }
    }

    /// Joins a channel: at once, or through the box that asks for its
    /// password.
    pub fn join(&mut self, chat: &WatchChat, channel: String) -> Option<Act> {
        let act = match joining(chat, Some(&channel)) {
            Joining::Send(act) => Some(act),
            Joining::AskPassword(channel) => {
                self.asking = Some((Asking::Password(channel), String::new()));
                None
            }
            Joining::Nothing => None,
        };
        self.picked = Some(channel);
        act
    }

    /// A press on a button under the channels.
    pub fn press(&mut self, chat: &WatchChat, button: ChatButton) -> Option<Act> {
        match button {
            ChatButton::Join => {
                let channel = self.picked.clone()?;
                self.join(chat, channel)
            }
            ChatButton::Leave => Some(Act::ChatLeave),
            ChatButton::Create => {
                self.asking = Some((Asking::NewChannel, String::new()));
                None
            }
        }
    }

    /// Okay, or Cancel, on the small box: Okay sends what the typed words
    /// make. Either shuts the box.
    pub fn answer_box(&mut self, okay: bool) -> Option<Act> {
        let (asking, words) = self.asking.take()?;
        okay.then(|| asking.act(&words)).flatten()
    }

    /// Takes the answer of Jev about a channel: the channel he picked is
    /// joined. The words of a failure as an error.
    pub fn take_answer(
        &mut self,
        chat: Option<&WatchChat>,
        answer: Answer,
    ) -> Result<Option<Act>, String> {
        match (answer, chat) {
            (Answer::Picked(Ok(place)), Some(chat)) => match chat.channels.get(place) {
                Some((name, _)) => Ok(self.join(chat, name.clone())),
                None => Ok(None),
            },
            (Answer::Picked(Err(words)), _) => Err(words),
            // The chat asks only for a pick, and the chat is gone.
            _ => Ok(None),
        }
    }
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
    fn a_locked_channel_asks_its_password_and_an_open_one_joins() {
        let chat = chat();
        let mut panel = ChatPanel::default();
        assert_eq!(panel.press(&chat, ChatButton::Join), None, "none picked");
        assert_eq!(panel.join(&chat, "Guild".into()), None);
        assert_eq!(
            panel.asking,
            Some((Asking::Password("Guild".into()), String::new()))
        );
        panel.asking.as_mut().unwrap().1 = "pw".into();
        assert_eq!(
            panel.answer_box(true),
            Some(Act::ChatJoinWithPassword {
                channel: "Guild".into(),
                password: "pw".into(),
            })
        );
        assert!(panel.asking.is_none());
        assert_eq!(
            panel.join(&chat, "General".into()),
            Some(Act::ChatJoin("General".into()))
        );
        assert_eq!(
            panel.press(&chat, ChatButton::Join),
            Some(Act::ChatJoin("General".into()))
        );
        assert_eq!(panel.press(&chat, ChatButton::Leave), Some(Act::ChatLeave));
        panel.press(&chat, ChatButton::Create);
        assert_eq!(panel.answer_box(false), None, "Cancel sends nothing");
    }

    #[test]
    fn the_chat_opens_itself_once_when_the_shard_opens_it() {
        let mut panel = ChatPanel::default();
        let frame = WatchFrame {
            chat: Some(chat()),
            ..WatchFrame::default()
        };
        panel.follow(&frame);
        assert!(panel.open);
        panel.toggle();
        panel.follow(&frame);
        assert!(!panel.open, "the player closed it");
        assert_eq!(chat_title(&frame), "Chat: General");
    }

    #[test]
    fn jev_joins_the_channel_he_picks() {
        let chat = chat();
        let mut panel = ChatPanel::default();
        assert!(matches!(
            channel_ask(&chat, "the guild"),
            Some(Ask::Channel { options, .. }) if options[1] == "Guild (needs a password)"
        ));
        assert_eq!(
            panel.take_answer(Some(&chat), Answer::Picked(Ok(0))),
            Ok(Some(Act::ChatJoin("General".into())))
        );
        assert_eq!(
            panel.take_answer(Some(&chat), Answer::Picked(Err("no".into()))),
            Err("no".into())
        );
        assert_eq!(say_act("  "), None);
    }
}
