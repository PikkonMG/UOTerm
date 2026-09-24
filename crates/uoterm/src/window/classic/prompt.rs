//! The words the shard waits for (a prompt, 0x9A or 0xC2), in the look of
//! the text entry dialog: the shard asked its question in the journal, and
//! the player types the answer in the field. Okay or Enter answers; Cancel
//! or a right click tells the shard the player will not answer. It takes the
//! keys once as it opens; unlike the text entry dialog it is not modal and
//! the player may move it.

use super::canvas::Canvas;
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text_entry::{Entered, EntryDialog, DIALOG_PLACE};
use crate::view::WatchFrame;
use crate::window::model::asked;

pub const PROMPT: GumpKind = GumpKind {
    id: well_known::PROMPT,
    rules: GumpRules {
        kept: false,
        first_place: DIALOG_PLACE,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(Prompt::default()),
};

/// A prompt has no limit of its own and takes any words.
const PROMPT_MAX_CHARS: Option<usize> = None;
const PROMPT_NUMERIC: bool = false;
/// The field takes the keys once, so other fields may have them after.
const PROMPT_HOLDS_FOCUS: bool = false;

pub struct Prompt {
    dialog: EntryDialog,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            dialog: EntryDialog::new(PROMPT_NUMERIC, PROMPT_MAX_CHARS, PROMPT_HOLDS_FOCUS),
        }
    }
}

impl GumpBody for Prompt {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let act = match self
            .dialog
            .draw(g, asked::PROMPT_TITLE, asked::PROMPT_DESCRIPTION)
        {
            Some(Entered::Answer(words)) => asked::PROMPT.answer_act(&words),
            Some(Entered::Cancel) => asked::PROMPT.cancel_act(),
            None => return,
        };
        cx.act(act);
        cx.close(cx.me);
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(asked::PROMPT.cancel_act());
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::{draw_frames, focus_after_drawing};
    use crate::window::keys::Focus;
    use crate::window::settings::Profile;

    #[test]
    fn the_prompt_moves_and_lives_while_the_shard_waits() {
        let rules = PROMPT.rules;
        assert_eq!(
            (rules.movable, rules.modal, rules.kept),
            (true, false, false)
        );
        let body = Prompt::default();
        let waiting = WatchFrame {
            prompt: true,
            ..WatchFrame::default()
        };
        assert!(body.alive(&waiting));
        assert!(!body.alive(&WatchFrame::default()));
    }

    #[test]
    fn the_open_prompt_field_takes_the_keys_from_the_game() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        manager.open(GumpId::one(well_known::PROMPT), &mut profile);
        let waiting = WatchFrame {
            prompt: true,
            ..WatchFrame::default()
        };
        if let Some(focus) = focus_after_drawing(&mut manager, &mut profile, &waiting) {
            assert_eq!(focus, Focus::OtherField);
        }
    }

    #[test]
    fn the_prompt_draws_and_goes_when_the_shard_stops_waiting() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::one(well_known::PROMPT);
        manager.open(id, &mut profile);
        let waiting = WatchFrame {
            prompt: true,
            ..WatchFrame::default()
        };
        if !draw_frames(&mut manager, &mut profile, &waiting) {
            return;
        }
        assert!(manager.is_open(&id));
        draw_frames(&mut manager, &mut profile, &WatchFrame::default());
        assert!(!manager.is_open(&id));
    }
}
