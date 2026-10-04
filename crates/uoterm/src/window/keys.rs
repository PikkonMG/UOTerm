//! The keys of the play window. Their rules are `uoterm_view::keys`; here
//! the window reads the keys egui reports, finds which field has them, and
//! takes the keys that fired something from the input.

pub mod chat;

pub use uoterm_view::keys::*;

use super::bridge;
use eframe::egui::{self, Event, Id};
use uoterm_view::input::{KeyName, KeyPress, Mods};

/// The mark of a text field that is not an egui text edit, such as a text
/// box of a classic gump, so the keys know it takes words.
#[derive(Clone, Copy, Debug, Default)]
struct WordField;

/// Marks the widget `id` as a field that takes words. A field drawn by hand
/// calls it each frame it draws.
pub fn mark_word_field(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|data| data.insert_temp(id, WordField));
}

fn is_word_field(ctx: &egui::Context, id: Id) -> bool {
    egui::text_edit::TextEditState::load(ctx, id).is_some()
        || ctx.data(|data| data.get_temp::<WordField>(id)).is_some()
}

/// The focus of the window: the chat line is the field with `chat_id`.
pub fn focus(ctx: &egui::Context, chat_id: Id, chat_empty: bool) -> Focus {
    let focused = ctx.memory(|memory| memory.focused());
    Focus::of(
        focused == Some(chat_id),
        chat_empty,
        focused.is_some_and(|id| id != chat_id && is_word_field(ctx, id)),
    )
}

/// The key presses of this frame.
pub fn presses(ctx: &egui::Context) -> Vec<KeyPress> {
    ctx.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } => Some(KeyPress {
                    key: bridge::key_name(*key),
                    mods: bridge::mods(*modifiers),
                    pressed: *pressed,
                    repeat: *repeat,
                }),
                _ => None,
            })
            .collect()
    })
}

/// Takes the used keys from the input, so no field and no other part of
/// the window acts on them, and drops the letters they would type.
pub fn take_used(ctx: &egui::Context, used: &[(Mods, KeyName)]) {
    if used.is_empty() {
        return;
    }
    ctx.input_mut(|input| {
        for (mods, name) in used {
            if let Some(key) = bridge::egui_key(name) {
                input.consume_key(bridge::modifiers(*mods), key);
            }
        }
        input
            .events
            .retain(|event| !matches!(event, Event::Text(_)));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_focused_hand_drawn_field_takes_the_keys_and_a_button_does_not() {
        let ctx = egui::Context::default();
        let chat = Id::new("chat");
        let field = Id::new("field");
        let button = Id::new("button");
        let focus_on = |id| {
            ctx.memory_mut(|memory| memory.request_focus(id));
            focus(&ctx, chat, true)
        };
        assert_eq!(focus_on(field), Focus::Free, "not marked yet");
        mark_word_field(&ctx, field);
        assert_eq!(focus_on(field), Focus::OtherField);
        assert_eq!(focus_on(button), Focus::Free);
        assert_eq!(focus_on(chat), Focus::ChatEmpty);
    }

    #[test]
    fn a_used_key_is_taken_from_the_input() {
        let ctx = egui::Context::default();
        let tab = egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let input = egui::RawInput {
            events: vec![tab, egui::Event::Text("\t".into())],
            ..egui::RawInput::default()
        };
        let mut left = Vec::new();
        let _ = ctx.run(input, |ctx| {
            let pressed = presses(ctx);
            take_used(ctx, &[(pressed[0].mods, pressed[0].key.clone())]);
            left = ctx.input(|i| i.events.clone());
        });
        assert!(left.is_empty(), "{left:?}");
    }
}
