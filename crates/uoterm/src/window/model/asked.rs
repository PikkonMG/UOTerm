//! What the shard asks the player to type: a prompt (0x9A, 0xC2) or a
//! dialog with a text field (0xAB), the words and the rules of its field,
//! and the script commands that answer and cancel each. The chat line and
//! the entry dialog of the Modern style and the classic prompt and text
//! entry gumps answer through these.

use crate::view::{WatchFrame, WatchTextEntry};
use crate::window::control::{quoted, Act};

/// The script command that answers a prompt of the shard.
pub const COMMAND_PROMPT_ANSWER: &str = "promptmsg";
pub const COMMAND_PROMPT_CANCEL: &str = "cancelprompt";
/// The script command that answers the text entry dialog of the shard.
pub const COMMAND_ENTRY_ANSWER: &str = "textentrymsg";
pub const COMMAND_ENTRY_CANCEL: &str = "canceltextentry";

/// The two script commands that answer and cancel one kind of question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AskedCommands {
    pub answer: &'static str,
    pub cancel: &'static str,
}

/// The commands of a prompt.
pub const PROMPT: AskedCommands = AskedCommands {
    answer: COMMAND_PROMPT_ANSWER,
    cancel: COMMAND_PROMPT_CANCEL,
};

/// The commands of a text entry dialog.
pub const TEXT_ENTRY: AskedCommands = AskedCommands {
    answer: COMMAND_ENTRY_ANSWER,
    cancel: COMMAND_ENTRY_CANCEL,
};

impl AskedCommands {
    /// The act that sends typed words as the answer.
    pub fn answer_act(self, words: &str) -> Act {
        Act::Command(typed_answer(self.answer, words))
    }

    /// The act that tells the shard the player will not answer.
    pub fn cancel_act(self) -> Act {
        Act::Command(self.cancel.into())
    }
}

/// An answer as one line of the script language. The quote that would end
/// the text early is taken out.
pub fn typed_answer(command: &str, words: &str) -> String {
    format!("{command} {}", quoted(words))
}

/// The title and the description of a prompt: the shard asked its
/// question in the journal.
pub const PROMPT_TITLE: &str = "The shard waits for your words.";
pub const PROMPT_DESCRIPTION: &str = "Its question is in the journal. Type the answer:";

/// The most chars a field takes: none for a limit of zero.
pub fn field_limit(max_length: u32) -> Option<usize> {
    (max_length > 0).then(|| usize::try_from(max_length).unwrap_or(usize::MAX))
}

/// Typed words as a field keeps them: digits only in a numeric field, and
/// no more chars than its limit.
pub fn kept_words(words: &str, numeric: bool, max_chars: Option<usize>) -> String {
    words
        .chars()
        .filter(|c| !numeric || c.is_ascii_digit())
        .take(max_chars.unwrap_or(usize::MAX))
        .collect()
}

/// What the shard asks now, as a dialog shows it: its words, the rules of
/// its field, whether the player may say no, and its commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedDialog {
    pub title: String,
    pub description: String,
    pub numeric: bool,
    pub max_chars: Option<usize>,
    pub can_cancel: bool,
    pub commands: AskedCommands,
}

impl AskedDialog {
    fn of_entry(entry: &WatchTextEntry) -> Self {
        Self {
            title: entry.title.clone(),
            description: entry.description.clone(),
            numeric: entry.numeric(),
            max_chars: field_limit(entry.max_length),
            can_cancel: entry.can_cancel,
            commands: TEXT_ENTRY,
        }
    }

    fn of_prompt() -> Self {
        Self {
            title: PROMPT_TITLE.into(),
            description: PROMPT_DESCRIPTION.into(),
            numeric: false,
            max_chars: None,
            can_cancel: true,
            commands: PROMPT,
        }
    }
}

/// The dialog of what the shard asks for now: a dialog with a text field
/// first, then a prompt. None when it asks for nothing.
pub fn asked_dialog(frame: &WatchFrame) -> Option<AskedDialog> {
    match (&frame.text_entry, frame.prompt) {
        (Some(entry), _) => Some(AskedDialog::of_entry(entry)),
        (None, true) => Some(AskedDialog::of_prompt()),
        (None, false) => None,
    }
}

/// The commands that answer and cancel what the shard asks for now: a
/// dialog with a text field first, then a prompt. None when it asks for
/// nothing.
pub fn asked_commands(frame: &WatchFrame) -> Option<AskedCommands> {
    if frame.text_entry.is_some() {
        Some(TEXT_ENTRY)
    } else if frame.prompt {
        Some(PROMPT)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::TEXT_ENTRY_STYLE_NUMERIC;

    #[test]
    fn a_text_entry_comes_before_a_prompt_and_answers_are_one_command() {
        let mut frame = WatchFrame::default();
        assert_eq!(asked_commands(&frame), None);
        frame.prompt = true;
        assert_eq!(asked_commands(&frame), Some(PROMPT));
        assert_eq!(
            typed_answer(COMMAND_PROMPT_ANSWER, "Bob's shop"),
            "promptmsg \"Bob's shop\""
        );
        frame.text_entry = Some(WatchTextEntry::default());
        assert_eq!(asked_commands(&frame), Some(TEXT_ENTRY));
        assert_eq!(
            TEXT_ENTRY.answer_act("Rex"),
            Act::Command("textentrymsg 'Rex'".into())
        );
        assert_eq!(
            PROMPT.cancel_act(),
            Act::Command(COMMAND_PROMPT_CANCEL.into())
        );
    }

    #[test]
    fn a_dialog_carries_the_rules_of_its_field() {
        assert_eq!(asked_dialog(&WatchFrame::default()), None);
        let prompt = WatchFrame {
            prompt: true,
            ..WatchFrame::default()
        };
        let dialog = asked_dialog(&prompt).expect("a prompt");
        assert!(dialog.can_cancel && !dialog.numeric && dialog.max_chars.is_none());
        assert_eq!(dialog.commands, PROMPT);
        let entry = WatchFrame {
            text_entry: Some(WatchTextEntry {
                title: "How many?".into(),
                style: TEXT_ENTRY_STYLE_NUMERIC,
                max_length: 3,
                ..WatchTextEntry::default()
            }),
            ..prompt
        };
        let dialog = asked_dialog(&entry).expect("a text entry");
        assert_eq!(dialog.commands, TEXT_ENTRY);
        assert!(dialog.numeric && !dialog.can_cancel);
        assert_eq!(dialog.max_chars, Some(3));
        assert_eq!(
            kept_words("12a345", dialog.numeric, dialog.max_chars),
            "123"
        );
        assert_eq!(kept_words("Rex the dog", false, None), "Rex the dog");
        assert_eq!(field_limit(0), None);
    }
}
