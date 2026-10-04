//! The chat line of the window. Its rules are `uoterm_view::keys::chat`;
//! here the keys egui reports go to it, and its lines go to the hand.

pub use uoterm_view::keys::chat::*;

use crate::view::WatchFrame;
use crate::window::control::Hand;
use crate::window::settings::SpeechOptions;
use eframe::egui::{self, Id, Key};
use uoterm_view::keys::Focus;

/// Sends a line of the chat line, or prints what the client answers
/// itself.
pub fn say_line(line: &str, frame: &WatchFrame, speech: &SpeechOptions, hand: &Hand) {
    tell(parse_line(line).said(frame, speech), hand);
}

/// Sends what a line comes to, or prints the words the client answers.
pub fn tell(said: Option<Said>, hand: &Hand) {
    match said {
        Some(Said::Act(act)) => hand.act(act),
        Some(Said::Note(words)) => hand.note(&words),
        None => {}
    }
}

/// Gives the line, the field with id `key`, the keys when the Speech page
/// says it has them: always, or after Enter or a prefix key. Esc lets them
/// go. True when Enter opened the line this frame: that Enter sends
/// nothing. The field types the words itself.
pub fn take_keys(
    line: &mut ChatLine,
    ctx: &egui::Context,
    key: Id,
    speech: &SpeechOptions,
) -> bool {
    let focused = ctx.memory(|m| m.focused());
    let focus = if focused == Some(key) {
        Focus::ChatTyping
    } else if focused.is_some_and(|id| super::is_word_field(ctx, id)) {
        Focus::OtherField
    } else {
        Focus::Free
    };
    // The field types the words itself, and its Enter sends when it lets
    // the keys go; here it takes Escape.
    let keys = ctx.input(|i| {
        let mut keys = Vec::new();
        if focus != Focus::Free {
            if i.key_pressed(Key::Escape) {
                keys.push(LineKey::Escape);
            }
            return keys;
        }
        let typed = i.events.iter().find_map(|event| match event {
            egui::Event::Text(text) => Some(text.clone()),
            _ => None,
        });
        keys.extend(typed.map(LineKey::Typed));
        if i.key_pressed(Key::Enter) {
            keys.push(LineKey::Enter { shift: false });
        }
        keys
    });
    let out = line.take_keys(focus, &keys, speech);
    if out.closed {
        ctx.memory_mut(|m| m.surrender_focus(key));
    }
    if focus != Focus::Free {
        return false;
    }
    let paste = line.take_paste();
    if line.is_open(speech) {
        ctx.memory_mut(|m| m.request_focus(key));
    }
    if paste {
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }
    out.opened
}
