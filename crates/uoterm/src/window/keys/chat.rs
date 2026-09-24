//! The chat line as the official client has it, without its drawing, so
//! each UI style draws its own: when it takes the keys, what Enter does,
//! the message history of Ctrl+Q and Ctrl+W, and the speech prefixes.
//!
//! A line that starts with `! ` is yelled, `; ` whispered and `: ` an
//! emote. `/` speaks to the party (`/2 ` to its second member), `\` to the
//! guild, `|` to the alliance and `,` on the global chat, as the
//! reference client's chat line reads them. Each goes out in the hue the Speech page gives
//! its channel. The party words `add`, `loot`, `quit`, `accept`, `decline`
//! and `rem` are orders; one that cannot be done now, and words to a party
//! the character is not in, give the line the reference client prints in
//! the journal.

use crate::view::WatchFrame;
use crate::window::control::{Act, Channel, Hand};
use crate::window::model::party::{
    leads, ACCEPT_COMMAND, DECLINE_COMMAND, INVITE_COMMAND, PARTY_PLACES,
};
use crate::window::settings::SpeechOptions;
use eframe::egui::{self, Id, Key};

/// The chat line keeps this many sent lines for Ctrl+Q and Ctrl+W.
const HISTORY_LINES: usize = 50;
const PREFIX_YELL: char = '!';
const PREFIX_WHISPER: char = ';';
const PREFIX_EMOTE: char = ':';
const PREFIX_PARTY: char = '/';
const PREFIX_GUILD: char = '\\';
const PREFIX_ALLIANCE: char = '|';
const PREFIX_GLOBAL_CHAT: char = ',';
/// The keys that open a closed chat line when the Speech page says so,
/// as in the reference client.
const OPENING_CHARACTERS: [char; 10] = ['!', ';', ':', '/', '\\', '|', ',', '.', '[', '-'];
/// The script command that has the shard ask, with a target cursor, whom
/// to take out of the party.
const REMOVE_COMMAND: &str = "partyremove";
/// The reference client's words when a party order cannot be done.
const NOT_IN_PARTY: &str = "You are not in a party.";
const NOT_PARTY_LEADER: &str = "You are not party leader.";
const NOT_INVITED: &str = "No one has invited you to be in a party.";
const NOTE_TO_SELF: &str = "Note to self: ";

/// A party word the client turns into an order, as the reference client reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyOrder {
    Add,
    Loot,
    Quit,
    Accept,
    Decline,
    Remove,
}

const PARTY_ORDERS: [(&str, PartyOrder); 6] = [
    ("add", PartyOrder::Add),
    ("loot", PartyOrder::Loot),
    ("quit", PartyOrder::Quit),
    ("accept", PartyOrder::Accept),
    ("decline", PartyOrder::Decline),
    ("rem", PartyOrder::Remove),
];

/// Who hears a line of the chat line, and the words without the prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spoken {
    Say(String),
    On(Channel, String),
    /// To a party member by his place, from 1.
    PartyPlace(usize, String),
    GlobalChat(String),
    PartyOrder(PartyOrder),
}

/// What a line of the chat line comes to: an act, or words the client
/// prints in the journal itself.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    Act(Act),
    Note(String),
}

/// Reads the prefix of a line.
pub fn parse_line(line: &str) -> Spoken {
    let mut letters = line.chars();
    let first = letters.next();
    let rest = letters.as_str();
    let after_space = rest.strip_prefix(' ');
    let words = |text: &str| text.trim_start().to_string();
    match (first, after_space) {
        (Some(PREFIX_YELL), Some(text)) => Spoken::On(Channel::Yell, words(text)),
        (Some(PREFIX_WHISPER), Some(text)) => Spoken::On(Channel::Whisper, words(text)),
        (Some(PREFIX_EMOTE), Some(text)) => Spoken::On(Channel::Emote, words(text)),
        (Some(PREFIX_PARTY), _) => party_line(rest),
        (Some(PREFIX_GUILD), _) => Spoken::On(Channel::Guild, words(rest)),
        (Some(PREFIX_ALLIANCE), _) => Spoken::On(Channel::Alliance, words(rest)),
        (Some(PREFIX_GLOBAL_CHAT), _) => Spoken::GlobalChat(words(rest)),
        _ => Spoken::Say(line.to_string()),
    }
}

/// A party line: a member's place, a party word, or words to all.
fn party_line(rest: &str) -> Spoken {
    let (head, tail) = rest.split_once(' ').unwrap_or((rest, ""));
    if let Ok(place) = head.parse::<usize>() {
        if (1..=PARTY_PLACES).contains(&place) {
            return Spoken::PartyPlace(place, tail.trim_start().to_string());
        }
    }
    let order = PARTY_ORDERS
        .into_iter()
        .find(|(word, _)| rest.trim().eq_ignore_ascii_case(word));
    match order {
        Some((_, order)) => Spoken::PartyOrder(order),
        None => Spoken::On(Channel::Party, rest.trim_start().to_string()),
    }
}

/// The hue the Speech page gives words on a channel, as the reference
/// client sends them. None is plain speech.
pub fn channel_hue(speech: &SpeechOptions, channel: Option<Channel>) -> u16 {
    match channel {
        None => speech.speech_hue,
        Some(Channel::Yell) => speech.yell_hue,
        Some(Channel::Whisper) => speech.whisper_hue,
        Some(Channel::Emote) => speech.emote_hue,
        Some(Channel::Party | Channel::PartyMember(_)) => speech.party_hue,
        Some(Channel::Guild) => speech.guild_hue,
        Some(Channel::Alliance) => speech.alliance_hue,
    }
}

/// The hue of a line as it is typed: the reference client colours its
/// chat line by the channel the prefix picks, and the global chat by the chat hue.
pub fn typed_hue(line: &str, speech: &SpeechOptions) -> u16 {
    match parse_line(line) {
        Spoken::Say(_) => channel_hue(speech, None),
        Spoken::On(channel, _) => channel_hue(speech, Some(channel)),
        Spoken::PartyPlace(..) | Spoken::PartyOrder(_) => channel_hue(speech, Some(Channel::Party)),
        Spoken::GlobalChat(_) => speech.chat_hue,
    }
}

impl Spoken {
    /// What the line comes to. None for a line with no words.
    pub fn said(self, frame: &WatchFrame, speech: &SpeechOptions) -> Option<Said> {
        let in_party = !frame.party_members.is_empty();
        let note = |words: String| Some(Said::Note(words));
        let speak = |channel, text: String| {
            let hue = channel_hue(speech, Some(channel));
            (!text.is_empty()).then_some(Said::Act(Act::Speak { channel, text, hue }))
        };
        match self {
            Spoken::Say(text) => (!text.trim().is_empty()).then(|| {
                let hue = channel_hue(speech, None);
                Said::Act(Act::Say { text, hue })
            }),
            Spoken::On(Channel::Party, text) if !in_party && !text.is_empty() => {
                note(format!("{NOTE_TO_SELF}{text}"))
            }
            Spoken::PartyPlace(place, text) if !in_party => {
                note(format!("{NOTE_TO_SELF}{place} {text}"))
            }
            Spoken::On(channel, text) => speak(channel, text),
            Spoken::PartyPlace(place, text) => {
                let channel = frame
                    .party_members
                    .get(place - 1)
                    .map_or(Channel::Party, |member| Channel::PartyMember(member.serial));
                speak(channel, text)
            }
            Spoken::GlobalChat(text) => (!text.is_empty()).then_some(Said::Act(Act::ChatSay(text))),
            Spoken::PartyOrder(order) => Some(party_order(order, frame, in_party)),
        }
    }
}

/// A party order, or the words the reference client prints when it
/// cannot be done.
fn party_order(order: PartyOrder, frame: &WatchFrame, in_party: bool) -> Said {
    let invited = !in_party && frame.party_invite.is_some();
    let command = |line: &str| Said::Act(Act::Command(line.to_string()));
    let note = |words: &str| Said::Note(words.to_string());
    match order {
        PartyOrder::Add if leads(frame) => command(INVITE_COMMAND),
        PartyOrder::Remove if in_party && leads(frame) => command(REMOVE_COMMAND),
        PartyOrder::Add | PartyOrder::Remove => note(NOT_PARTY_LEADER),
        PartyOrder::Loot if in_party => Said::Act(Act::PartyLoot(!frame.party_can_loot)),
        PartyOrder::Quit if in_party => Said::Act(Act::PartyLeave),
        PartyOrder::Loot | PartyOrder::Quit => note(NOT_IN_PARTY),
        PartyOrder::Accept if invited => command(ACCEPT_COMMAND),
        PartyOrder::Decline if invited => command(DECLINE_COMMAND),
        PartyOrder::Accept | PartyOrder::Decline => note(NOT_INVITED),
    }
}

/// Sends a line of the chat line, or prints what the client answers
/// itself.
pub fn say_line(line: &str, frame: &WatchFrame, speech: &SpeechOptions, hand: &Hand) {
    match parse_line(line).said(frame, speech) {
        Some(Said::Act(act)) => hand.act(act),
        Some(Said::Note(words)) => hand.note(&words),
        None => {}
    }
}

/// The chat line: the words in it, whether it takes the keys, and the
/// lines sent before.
#[derive(Default)]
pub struct ChatLine {
    pub text: String,
    /// The line is open for typing. With the Speech page's "press Enter to
    /// chat" off, it is always open.
    open: bool,
    history: Vec<String>,
    /// Where Ctrl+Q and Ctrl+W are in the history. The end is a new line.
    history_at: usize,
    /// The player hid the line with a key.
    hidden: bool,
    /// A key asked to paste into the line.
    paste_wanted: bool,
}

impl ChatLine {
    /// The line takes the keys now.
    pub fn is_open(&self, speech: &SpeechOptions) -> bool {
        !speech.chat_on_enter || self.open
    }

    /// The player hid the line with a key.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Hides the line, or shows it again.
    pub fn toggle_hidden(&mut self) {
        self.hidden = !self.hidden;
    }

    /// Shows the line and pastes the clipboard into it in the next frame.
    pub fn paste(&mut self) {
        self.hidden = false;
        self.paste_wanted = true;
    }

    /// Gives the line, the field with id `key`, the keys when the Speech
    /// page says it has them: always, or after Enter or a prefix key. Esc
    /// lets them go. True when Enter opened the line this frame: that
    /// Enter sends nothing.
    pub fn take_keys(&mut self, ctx: &egui::Context, key: Id, speech: &SpeechOptions) -> bool {
        let focused = ctx.memory(|m| m.focused());
        if focused == Some(key) {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.open = false;
                ctx.memory_mut(|m| m.surrender_focus(key));
            }
            return false;
        }
        if focused.is_some_and(|id| super::is_word_field(ctx, id)) {
            return false;
        }
        let typed = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Text(text) => Some(text.clone()),
                _ => None,
            })
        });
        if typed.is_some_and(|text| self.opens_on(&text, speech)) {
            self.open = true;
        }
        let opened = speech.chat_on_enter
            && !self.is_open(speech)
            && ctx.input(|i| i.key_pressed(Key::Enter));
        if opened {
            self.enter(false, speech);
        }
        let paste = std::mem::take(&mut self.paste_wanted);
        if paste {
            self.open = true;
        }
        if self.is_open(speech) {
            ctx.memory_mut(|m| m.request_focus(key));
        }
        if paste {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
        opened
    }

    /// Typed words open a closed line when the first one is a prefix key
    /// and the Speech page allows it.
    fn opens_on(&self, typed: &str, speech: &SpeechOptions) -> bool {
        speech.chat_on_enter
            && speech.chat_prefix_keys
            && !self.open
            && typed
                .chars()
                .next()
                .is_some_and(|c| OPENING_CHARACTERS.contains(&c))
    }

    /// Enter was pressed. Gives the line to send, if it has words. The line
    /// closes after it, unless the line is always open or Shift was held
    /// and the Speech page keeps it open for Shift+Enter.
    pub fn enter(&mut self, shift: bool, speech: &SpeechOptions) -> Option<String> {
        if speech.chat_on_enter && !self.open {
            self.open = true;
            return None;
        }
        let line = self.text.trim().to_string();
        self.text.clear();
        if !(shift && speech.shift_enter_sends) {
            self.open = false;
        }
        if line.is_empty() {
            return None;
        }
        self.history.push(line.clone());
        if self.history.len() > HISTORY_LINES {
            self.history.remove(0);
        }
        self.history_at = self.history.len();
        Some(line)
    }

    /// Ctrl+Q: the line sent before the one shown.
    pub fn older(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.history_at = self.history_at.saturating_sub(1);
        self.text = self.history[self.history_at].clone();
    }

    /// Ctrl+W: the line sent after the one shown, or an empty line after
    /// the newest.
    pub fn newer(&mut self) {
        if self.history_at + 1 < self.history.len() {
            self.history_at += 1;
            self.text = self.history[self.history_at].clone();
        } else {
            self.history_at = self.history.len();
            self.text.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPartyMember;

    #[test]
    fn a_prefix_picks_who_hears_the_line() {
        assert_eq!(
            parse_line("! guards"),
            Spoken::On(Channel::Yell, "guards".into())
        );
        assert_eq!(
            parse_line("; psst"),
            Spoken::On(Channel::Whisper, "psst".into())
        );
        assert_eq!(
            parse_line(": waves"),
            Spoken::On(Channel::Emote, "waves".into())
        );
        assert_eq!(
            parse_line("/heal me"),
            Spoken::On(Channel::Party, "heal me".into())
        );
        assert_eq!(
            parse_line("/2 heal me"),
            Spoken::PartyPlace(2, "heal me".into())
        );
        assert_eq!(
            parse_line("/accept"),
            Spoken::PartyOrder(PartyOrder::Accept)
        );
        assert_eq!(parse_line("/REM"), Spoken::PartyOrder(PartyOrder::Remove));
        assert_eq!(
            parse_line("\\ hi guild"),
            Spoken::On(Channel::Guild, "hi guild".into())
        );
        assert_eq!(
            parse_line("|hi all"),
            Spoken::On(Channel::Alliance, "hi all".into())
        );
        assert_eq!(parse_line(",hello"), Spoken::GlobalChat("hello".into()));
        // With no space after it, the mark is part of the words.
        assert_eq!(parse_line("!help"), Spoken::Say("!help".into()));
        assert_eq!(parse_line(":)"), Spoken::Say(":)".into()));
        assert_eq!(parse_line("bank"), Spoken::Say("bank".into()));
    }

    #[test]
    fn a_line_to_a_party_place_goes_to_that_member() {
        let frame = WatchFrame {
            party_members: vec![
                WatchPartyMember {
                    serial: 7,
                    ..WatchPartyMember::default()
                },
                WatchPartyMember {
                    serial: 8,
                    ..WatchPartyMember::default()
                },
            ],
            ..WatchFrame::default()
        };
        let speech = SpeechOptions::default();
        let party_hue = speech.party_hue;
        assert_eq!(
            parse_line("/2 heal me").said(&frame, &speech),
            Some(Said::Act(Act::Speak {
                channel: Channel::PartyMember(8),
                text: "heal me".into(),
                hue: party_hue,
            }))
        );
        assert_eq!(
            parse_line("/9 heal me").said(&frame, &speech),
            Some(Said::Act(Act::Speak {
                channel: Channel::Party,
                text: "heal me".into(),
                hue: party_hue,
            }))
        );
        assert_eq!(parse_line("! ").said(&frame, &speech), None);
        assert_eq!(
            parse_line("/quit").said(&frame, &speech),
            Some(Said::Act(Act::PartyLeave))
        );
        assert_eq!(
            parse_line("/loot").said(&frame, &speech),
            Some(Said::Act(Act::PartyLoot(true)))
        );
    }

    #[test]
    fn each_line_goes_out_in_the_hue_of_its_channel() {
        let speech = SpeechOptions {
            speech_hue: 1,
            yell_hue: 2,
            whisper_hue: 3,
            emote_hue: 4,
            guild_hue: 5,
            chat_hue: 6,
            ..SpeechOptions::default()
        };
        let frame = WatchFrame::default();
        assert_eq!(
            parse_line("hail").said(&frame, &speech),
            Some(Said::Act(Act::Say {
                text: "hail".into(),
                hue: 1
            }))
        );
        assert_eq!(
            parse_line(": waves").said(&frame, &speech),
            Some(Said::Act(Act::Speak {
                channel: Channel::Emote,
                text: "waves".into(),
                hue: 4
            }))
        );
        let hues: Vec<u16> = ["hi", "! hi", "; hi", ": hi", "\\hi", ",hi", "/hi"]
            .into_iter()
            .map(|line| typed_hue(line, &speech))
            .collect();
        assert_eq!(hues, [1, 2, 3, 4, 5, 6, speech.party_hue]);
    }

    /// The words the reference client prints when a party order cannot be
    /// done, and
    /// for words to a party the character is not in.
    #[test]
    fn a_party_order_that_cannot_be_done_prints_classic_words() {
        let speech = SpeechOptions::default();
        let alone = WatchFrame {
            serial: 1,
            ..WatchFrame::default()
        };
        let said = |line: &str, frame: &WatchFrame| parse_line(line).said(frame, &speech);
        let note = |words: &str| Some(Said::Note(words.to_string()));
        assert_eq!(said("/quit", &alone), note(NOT_IN_PARTY));
        assert_eq!(said("/loot", &alone), note(NOT_IN_PARTY));
        assert_eq!(said("/accept", &alone), note(NOT_INVITED));
        assert_eq!(said("/decline", &alone), note(NOT_INVITED));
        assert_eq!(said("/rem", &alone), note(NOT_PARTY_LEADER));
        assert_eq!(said("/heal me", &alone), note("Note to self: heal me"));
        assert_eq!(said("/2 heal me", &alone), note("Note to self: 2 heal me"));
        assert_eq!(
            said("/add", &alone),
            Some(Said::Act(Act::Command(INVITE_COMMAND.into())))
        );
        let invited = WatchFrame {
            party_invite: Some(9),
            ..alone.clone()
        };
        assert_eq!(
            said("/accept", &invited),
            Some(Said::Act(Act::Command(ACCEPT_COMMAND.into())))
        );
        let member = WatchFrame {
            party_members: vec![
                WatchPartyMember {
                    serial: 9,
                    ..WatchPartyMember::default()
                },
                WatchPartyMember {
                    serial: 1,
                    ..WatchPartyMember::default()
                },
            ],
            ..alone.clone()
        };
        assert_eq!(said("/add", &member), note(NOT_PARTY_LEADER));
        assert_eq!(said("/rem", &member), note(NOT_PARTY_LEADER));
        let leader = WatchFrame {
            party_members: member.party_members.iter().rev().cloned().collect(),
            ..alone
        };
        assert_eq!(
            said("/rem", &leader),
            Some(Said::Act(Act::Command(REMOVE_COMMAND.into())))
        );
    }

    #[test]
    fn ctrl_q_and_ctrl_w_walk_the_sent_lines() {
        let speech = SpeechOptions::default();
        let mut line = ChatLine::default();
        for words in ["one", "two", "three"] {
            line.text = words.into();
            assert_eq!(line.enter(false, &speech), Some(words.to_string()));
        }
        line.older();
        assert_eq!(line.text, "three");
        line.older();
        line.older();
        line.older();
        assert_eq!(line.text, "one");
        line.newer();
        assert_eq!(line.text, "two");
        line.newer();
        line.newer();
        assert_eq!(line.text, "");
    }

    #[test]
    fn enter_opens_and_closes_the_line_as_the_speech_page_says() {
        let always = SpeechOptions::default();
        let mut line = ChatLine::default();
        assert!(line.is_open(&always));
        assert_eq!(line.enter(false, &always), None);
        assert!(line.is_open(&always));
        let on_enter = SpeechOptions {
            chat_on_enter: true,
            ..SpeechOptions::default()
        };
        assert!(!line.is_open(&on_enter));
        assert!(line.opens_on("!", &on_enter) && !line.opens_on("a", &on_enter));
        assert_eq!(line.enter(false, &on_enter), None);
        assert!(line.is_open(&on_enter));
        line.text = "hail".into();
        assert_eq!(line.enter(true, &on_enter), Some("hail".into()));
        assert!(line.is_open(&on_enter), "Shift+Enter keeps it open");
        line.text = "bye".into();
        assert_eq!(line.enter(false, &on_enter), Some("bye".into()));
        assert!(!line.is_open(&on_enter));
    }
}
