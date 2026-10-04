//! The Modern panels as the page draws them: the data of each panel, built
//! by the shared rules, and the small actions its buttons send, which turn
//! into the same acts the panels of the Rust window make. Each panel has
//! one field of [`PanelData`] and one action type here.

use crate::{TooltipData, WebView};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Report;
use uoterm_view::art::{ArtRequest, ItemPaint};
use uoterm_view::clicks::report_shows;
use uoterm_view::frame::WatchFrame;
use uoterm_view::ui::deck::{
    slot_picture, KeptHotbars, Press, SlotPicture, HOTBAR_FILE, HOTBAR_KEY_WORDS, HOTBAR_SLOTS,
};

/// The names the page gives its panels in a `Panel` event.
pub const PANEL_HOTBAR: &str = "hotbar";
pub const PANEL_QUESTION: &str = "question";

/// What the page draws of the panels this frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PanelData {
    /// The hotbar, while the human has control.
    pub hotbar: Option<HotbarData>,
    /// The words of the last act, while they show.
    pub report: Option<Report>,
    /// The question that waits for Yes or No.
    pub question: Option<&'static str>,
    pub chat: ChatData,
    /// The tooltip of the thing under the mouse on the map.
    pub tooltip: Option<TooltipData>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HotbarData {
    pub slots: Vec<HotbarSlot>,
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
}

/// The chat line: its words, and whether it takes the keys or hides.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ChatData {
    pub text: String,
    pub open: bool,
    pub hidden: bool,
}

/// What a button of the hotbar asks: `{"press": slot}` or `{"clear": slot}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotbarAction {
    Press(usize),
    /// A right click empties the slot.
    Clear(usize),
}

/// The answer to the question: `{"answer": true}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionAction {
    Answer(bool),
}

impl WebView {
    /// The data of every panel at `time`.
    pub fn panel_data(&mut self, time: f64) -> PanelData {
        let report = self
            .hand
            .newest_report()
            .filter(|(_, since)| report_shows(*since, time))
            .map(|(report, _)| report.clone());
        let hotbar = self
            .frame
            .as_ref()
            .filter(|frame| frame.human_control)
            .cloned()
            .map(|frame| self.hotbar_data(&frame));
        PanelData {
            hotbar,
            report,
            question: self.hand.question(),
            chat: ChatData {
                text: self.chat.text.clone(),
                open: self.chat.is_open(&self.profile.speech),
                hidden: self.chat.is_hidden(),
            },
            tooltip: self.tooltip.clone(),
        }
    }

    fn hotbar_data(&mut self, frame: &WatchFrame) -> HotbarData {
        let slots = (0..HOTBAR_SLOTS)
            .map(|slot| {
                let key = HOTBAR_KEY_WORDS[slot];
                let Some(what) = self.hotbars.slot(&frame.name, slot).cloned() else {
                    return HotbarSlot {
                        words: String::new(),
                        key,
                        picture: None,
                        tip: String::new(),
                    };
                };
                let picture = slot_picture(&what, frame).map(|picture| {
                    let request = match picture {
                        SlotPicture::Item { graphic, hue } => self.scene.item_request(
                            &self.art,
                            graphic,
                            ItemPaint {
                                hue,
                                ..ItemPaint::default()
                            },
                            true,
                        ),
                        SlotPicture::Gump { gump, hue } => ArtRequest::Gump {
                            gump,
                            hue,
                            partial: false,
                        },
                    };
                    self.picture_key(&request)
                });
                HotbarSlot {
                    words: what.face_words(frame),
                    key,
                    picture,
                    tip: what.words(frame),
                }
            })
            .collect();
        HotbarData { slots }
    }

    /// Takes the action of a panel. An action of a panel this view does not
    /// know, or one that does not read, does nothing.
    pub(crate) fn panel_action(&mut self, panel: &str, action: Value) {
        match panel {
            PANEL_HOTBAR => {
                if let Ok(action) = serde_json::from_value(action) {
                    self.hotbar_action(action);
                }
            }
            PANEL_QUESTION => {
                if let Ok(QuestionAction::Answer(yes)) = serde_json::from_value(action) {
                    self.hand.answer_question(yes);
                }
            }
            _ => {}
        }
    }

    fn hotbar_action(&mut self, action: HotbarAction) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        match action {
            HotbarAction::Press(slot) => self.press_slot(&frame, slot),
            HotbarAction::Clear(slot) => {
                self.hotbars.set(&frame.name, slot, None);
                self.save_hotbars();
            }
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

    fn save_hotbars(&mut self) {
        if let Ok(data) = serde_json::to_value(&self.hotbars) {
            self.hand.push(crate::out::OutCall::SaveKept {
                name: HOTBAR_FILE.to_string(),
                data,
            });
        }
    }

    /// Takes the hotbars the page read from the session.
    pub(crate) fn keep_hotbars(&mut self, hotbars: KeptHotbars) {
        self.hotbars = hotbars;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::out::OutCall;
    use crate::tests::{fixture_watch_with_backpack, HATCHET, MARA};
    use serde_json::json;
    use uoterm_view::act::PageAct;
    use uoterm_view::settings::Profile;
    use uoterm_view::ui::deck::Slot;

    /// The acts among the calls for the page.
    fn out_acts(out: &[OutCall]) -> Vec<PageAct> {
        out.iter()
            .filter_map(|call| match call {
                OutCall::Act { act, .. } => Some(act.clone()),
                _ => None,
            })
            .collect()
    }

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
        let out = view.input_native(
            &json!({"kind": "Panel", "panel": "hotbar", "action": {"press": 0}}).to_string(),
            0.0,
        );
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
            &json!({"kind": "Key", "key": "Num1", "pressed": true}).to_string(),
            0.0,
        );
        view.tick_native(0.0, crate::tests::VIEW, None);
        assert_eq!(out_acts(&view.take_out_native()).len(), 1);
        let out = view.input_native(
            &json!({"kind": "Panel", "panel": "hotbar", "action": {"clear": 0}}).to_string(),
            0.0,
        );
        assert!(matches!(out.as_slice(), [OutCall::SaveKept { name, .. }] if name == HOTBAR_FILE));
        let data = view.panel_data(0.0);
        let hotbar = data.hotbar.unwrap();
        assert_eq!(hotbar.slots.len(), HOTBAR_SLOTS);
        assert_eq!(hotbar.slots[0].words, "", "empty now");
        assert_eq!(hotbar.slots[0].key, "1");
    }

    #[test]
    fn a_slot_with_an_item_shows_its_picture_once_it_is_asked_for() {
        let (mut view, _) = view_with_hatchet();
        let data = view.panel_data(0.0);
        let slot = &data.hotbar.unwrap().slots[0];
        assert_eq!(slot.words, "hatche");
        assert_eq!(slot.tip, "hatchet");
        let key = slot.picture.clone().unwrap();
        let wanted = view.art.take_wanted();
        assert!(wanted.iter().any(|wanted| wanted.key == key));
    }
}
