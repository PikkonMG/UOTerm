//! The hotbar: ten slots in a row over the pack, each pressed by a click
//! or its number key, emptied by a right click, filled from the picker of
//! an empty slot or by an item, a skill or a spell dropped on it. What a
//! slot does is `uoterm_view::ui::deck`.

use super::{FrameSpec, Framed, TipKey, PANEL_HOTBAR};
use crate::out::OutCall;
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::art::{ArtRequest, ItemPaint};
use uoterm_view::frame::WatchFrame;
use uoterm_view::ui::deck::{
    hotbar_size, picking_after, slot_choices, slot_picture, KeptHotbars, Press, Slot, SlotPicture,
    HINT_EMPTY_SLOT, HINT_SLOT, HOTBAR_FILE, HOTBAR_ID, HOTBAR_KEYS, HOTBAR_SLOTS, WORDS_HOTBAR,
    WORDS_NO_MACROS, WORDS_PICK_FOR,
};
use uoterm_view::ui::hud::PACK_WIDTH;
use uoterm_view::ui::layout::{first_place, Spot};

/// What the hotbar keeps between frames.
#[derive(Default)]
pub(crate) struct DeckState {
    /// The empty slot whose picker is open.
    pub picking: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HotbarData {
    pub slots: Vec<HotbarSlot>,
    /// The empty slot whose picker is open.
    pub picking: Option<usize>,
}

/// One slot of the hotbar.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HotbarSlot {
    /// The first letters of its words, for a slot with no picture. Empty
    /// for an empty slot.
    pub words: String,
    /// The key that presses it.
    pub key: &'static str,
    /// The key of its picture in the page's cache of pictures.
    pub picture: Option<String>,
    /// All its words, for its tip.
    pub tip: String,
    /// What the tooltip of the slot shows.
    pub hover: TipKey,
}

/// The choices of an empty slot: the macros of the profile and the
/// abilities.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PickerData {
    pub title: String,
    pub choices: Vec<String>,
    /// The words that say there is no macro yet.
    pub no_macros: Option<&'static str>,
}

/// What a button of the hotbar asks: `{"click": slot}` (a slot that holds
/// something does it; a click on an empty one opens or shuts its picker),
/// `{"clear": slot}` (a right click), `{"choose": choice}` of the picker,
/// or `{"close_picker": true}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotbarAction {
    Click(usize),
    Clear(usize),
    Choose(usize),
    ClosePicker(bool),
}

impl WebView {
    pub(super) fn hotbar_spec(&self) -> FrameSpec {
        let default = first_place(self.panel_room(), Spot::Hotbar, hotbar_size(PACK_WIDTH));
        FrameSpec::fixed(HOTBAR_ID, WORDS_HOTBAR, default)
    }

    pub(super) fn hotbar_data(&mut self, frame: &WatchFrame) -> Framed<HotbarData> {
        let slots = (0..HOTBAR_SLOTS)
            .map(|slot| self.slot_data(frame, slot))
            .collect();
        let body = HotbarData {
            slots,
            picking: self.panels.deck.picking,
        };
        self.framed(PANEL_HOTBAR, &self.hotbar_spec(), body)
    }

    fn slot_data(&mut self, frame: &WatchFrame, slot: usize) -> HotbarSlot {
        let key = HOTBAR_KEYS[slot];
        let Some(what) = self.hotbars.slot(&frame.name, slot).cloned() else {
            return HotbarSlot {
                words: String::new(),
                key,
                picture: None,
                tip: String::new(),
                hover: TipKey::label(key, HINT_EMPTY_SLOT),
            };
        };
        let picture = self.slot_picture_key(&what, frame);
        let hover = match &what {
            Slot::Item { serial, name, .. } => TipKey::thing(*serial, name, HINT_SLOT),
            other => TipKey::label(&other.words(frame), HINT_SLOT),
        };
        HotbarSlot {
            words: what.face_words(frame),
            key,
            picture,
            tip: what.words(frame),
            hover,
        }
    }

    /// The key of the picture of a slot in the page's cache, asked for.
    pub(super) fn slot_picture_key(&mut self, what: &Slot, frame: &WatchFrame) -> Option<String> {
        slot_picture(what, frame).map(|picture| match picture {
            SlotPicture::Item { graphic, hue } => {
                let request = self.item_picture_request(graphic, hue);
                self.picture_key(&request)
            }
            SlotPicture::Gump { gump, hue } => self.gump_picture(gump, hue),
        })
    }

    /// The request of the picture of an item of `graphic` in `hue`, as a
    /// panel shows it.
    pub(super) fn item_picture_request(&self, graphic: u16, hue: u16) -> ArtRequest {
        self.scene.item_request(
            &self.art,
            graphic,
            ItemPaint {
                hue,
                ..ItemPaint::default()
            },
            true,
        )
    }

    pub(super) fn picker_data(&self, frame: &WatchFrame) -> Option<PickerData> {
        let slot = self.panels.deck.picking.filter(|_| frame.human_control)?;
        let choices = slot_choices(frame, &self.profile);
        let has_macros = choices
            .iter()
            .any(|choice| matches!(choice, Slot::Macro { .. }));
        Some(PickerData {
            title: format!("{WORDS_PICK_FOR} {}", HOTBAR_KEYS[slot]),
            choices: choices.iter().map(|choice| choice.words(frame)).collect(),
            no_macros: (!has_macros).then_some(WORDS_NO_MACROS),
        })
    }

    pub(super) fn hotbar_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<HotbarAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        match action {
            HotbarAction::Click(slot) if self.hotbars.slot(&frame.name, slot).is_some() => {
                self.press_slot(&frame, slot);
            }
            HotbarAction::Click(slot) if slot < HOTBAR_SLOTS => {
                let deck = &mut self.panels.deck;
                deck.picking = picking_after(deck.picking, slot);
            }
            HotbarAction::Choose(choice) => {
                let Some(slot) = self.panels.deck.picking.take() else {
                    return;
                };
                if let Some(what) = slot_choices(&frame, &self.profile).into_iter().nth(choice) {
                    self.set_slot(&frame.name, slot, Some(what));
                }
            }
            HotbarAction::Clear(slot) => self.set_slot(&frame.name, slot, None),
            HotbarAction::ClosePicker(_) => self.panels.deck.picking = None,
            HotbarAction::Click(_) => {}
        }
    }

    /// Does what a slot of the hotbar does.
    pub(crate) fn press_slot(&mut self, frame: &WatchFrame, slot: usize) {
        let Some(what) = self.hotbars.slot(&frame.name, slot) else {
            return;
        };
        match what.press(frame, &self.profile) {
            Some(Press::Act(act)) => self.hand.act(act),
            Some(Press::Macro(steps)) => self.controls.run_macro(steps),
            Some(Press::Report(words)) => self.hand.report(words),
            None => {}
        }
    }

    /// Puts `what` on a slot of the character, or empties it, and keeps
    /// the hotbars.
    pub(crate) fn set_slot(&mut self, character: &str, slot: usize, what: Option<Slot>) {
        self.hotbars.set(character, slot, what);
        self.save_hotbars();
    }

    /// Puts `what` on the first free slot, and keeps the hotbars. False
    /// when the bar is full.
    pub(crate) fn pin(&mut self, character: &str, what: Slot) -> bool {
        let pinned = self.hotbars.pin(character, what).is_some();
        if pinned {
            self.save_hotbars();
        }
        pinned
    }

    /// Asks the page to keep the hotbars.
    fn save_hotbars(&mut self) {
        if let Ok(data) = serde_json::to_value(&self.hotbars) {
            self.hand.push(OutCall::SaveKept {
                name: HOTBAR_FILE.to_string(),
                data,
            });
        }
    }

    /// Puts a script line on the first free slot. False when the bar is
    /// full.
    pub(crate) fn pin_command(&mut self, character: &str, text: &str) -> bool {
        self.pin(
            character,
            Slot::Command {
                text: text.to_string(),
            },
        )
    }

    /// Takes the hotbars the page read from the session.
    pub(crate) fn keep_hotbars(&mut self, hotbars: KeptHotbars) {
        self.hotbars = hotbars;
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, HATCHET, MARA};
    use serde_json::json;
    use uoterm_view::settings::{KeyBinding, MacroStep, Profile};

    /// A view of Mara with the hatchet of her backpack in slot 0.
    fn view_with_hatchet() -> (WebView, Slot) {
        let mut view = WebView::new(&serde_json::to_string(&Profile::default()).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        let hatchet = Slot::Item {
            serial: HATCHET,
            graphic: 0x0F43,
            hue: 0,
            name: "hatchet".into(),
        };
        let mut hotbars = KeptHotbars::default();
        hotbars.set(MARA, 0, Some(hatchet.clone()));
        view.data_arrived_native(
            &format!("{}{HOTBAR_FILE}", crate::KEPT_PREFIX),
            &json!(hotbars),
        );
        (view, hatchet)
    }

    #[test]
    fn a_hotbar_press_makes_the_same_act_as_the_window() {
        let (mut view, slot) = view_with_hatchet();
        let out = press(&mut view, PANEL_HOTBAR, json!({"click": 0}));
        let frame = view.frame_ref().unwrap().clone();
        let Some(Press::Act(act)) = slot.press(&frame, &Profile::default()) else {
            panic!("the hatchet slot uses the hatchet");
        };
        assert_eq!(out_acts(&out), vec![act.for_page()]);
    }

    #[test]
    fn a_number_key_presses_its_slot_and_a_right_click_empties_it() {
        let (mut view, _) = view_with_hatchet();
        view.tick_native(0.0, crate::tests::VIEW, None);
        view.take_out_native();
        view.input_native(
            &json!({"kind": "Key", "key": "1", "pressed": true}).to_string(),
            0.0,
        );
        view.tick_native(0.0, crate::tests::VIEW, None);
        assert_eq!(out_acts(&view.take_out_native()).len(), 1);
        let out = press(&mut view, PANEL_HOTBAR, json!({"clear": 0}));
        assert!(matches!(out.as_slice(), [OutCall::SaveKept { name, .. }] if name == HOTBAR_FILE));
        let hotbar = view.panel_data(0.0).hotbar.unwrap().body;
        assert_eq!(hotbar.slots.len(), HOTBAR_SLOTS);
        assert_eq!(hotbar.slots[0].words, "", "empty now");
        assert_eq!(hotbar.slots[0].key, "1");
    }

    #[test]
    fn a_slot_with_an_item_shows_its_picture_once_it_is_asked_for() {
        let (mut view, _) = view_with_hatchet();
        let data = view.panel_data(0.0);
        let slot = &data.hotbar.unwrap().body.slots[0];
        assert_eq!(slot.words, "hatche");
        assert_eq!(slot.tip, "hatchet");
        assert_eq!(slot.hover.serial, Some(HATCHET));
        let key = slot.picture.clone().unwrap();
        let wanted = view.art.take_wanted();
        assert!(wanted.iter().any(|wanted| wanted.key == key));
    }

    #[test]
    fn an_empty_slot_takes_a_macro_from_its_picker() {
        let (mut view, _) = view_with_hatchet();
        let mut profile = Profile::default();
        profile.macros.key_bindings.push(KeyBinding {
            name: "bow".into(),
            chord: None,
            pad: None,
            steps: vec![MacroStep::new("say", "bow")],
        });
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        press(&mut view, PANEL_HOTBAR, json!({"click": 3}));
        let picker = view.panel_data(0.0).picker.unwrap();
        assert_eq!(picker.title, "Put on slot 4");
        assert_eq!(picker.choices[0], "bow");
        assert!(picker.no_macros.is_none());
        press(&mut view, PANEL_HOTBAR, json!({"choose": 0}));
        let data = view.panel_data(0.0);
        assert!(data.picker.is_none());
        assert_eq!(data.hotbar.unwrap().body.slots[3].tip, "bow");
    }
}
