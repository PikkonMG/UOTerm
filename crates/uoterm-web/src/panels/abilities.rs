//! The ability panels of the Modern style and the box of a party invite.
//! The combat panel shows the primary and the secondary ability of the
//! weapon in hand: a click arms one or lets it go, and each pins or drags
//! onto the hotbar; under them every weapon ability with the weapons that
//! have it. The racial panel shows the abilities of the character's race.
//! The box of an invite shows while the party tab is closed. Every rule is
//! `uoterm_view::ui::abilities`' and `model::party`'s; the acts need
//! control.

use super::{Colored, FrameSpec, Framed, TipKey, PANEL_ABILITIES, PANEL_INVITE, PANEL_RACIAL};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::actions::windows::Tab;
use uoterm_view::art::WorldArt;
use uoterm_view::frame::WatchFrame;
use uoterm_view::model::abilities::{ability_of, armed, icon_of, race_of, slot_hue, AbilitySlot};
use uoterm_view::model::party::{invite_words, inviter_name, ACCEPT_COMMAND, DECLINE_COMMAND};
use uoterm_view::model::places;
use uoterm_view::ui::abilities::{
    abilities_first_place, ability_rows, arm_words, racial_act, racial_first_place, racial_slot,
    slot_act, slot_of, ABILITIES_ID, ABILITIES_MIN, ABILITY_ICON, HINT_FLIGHT, HINT_SLOT,
    RACIAL_ID, WORDS_ABILITIES, WORDS_ALL, WORDS_ARMED, WORDS_NO_RACE, WORDS_PASSIVE, WORDS_RACIAL,
    WORDS_RACIAL_USE, WORDS_WEAPONS,
};
use uoterm_view::ui::lists::{ability_slot_words, ability_weapon_names};
use uoterm_view::ui::sheet::{
    invite_first_place, INVITE_ID, WORDS_ACCEPT, WORDS_DECLINE, WORDS_INVITE_TITLE, WORDS_PIN,
};
use uoterm_view::ui::theme::{css_color, ALARM, GOAL, TEXT};

/// The combat panel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AbilitiesData {
    pub icon: f32,
    pub slots: Vec<SlotCard>,
    pub all_title: &'static str,
    pub rows: Vec<AbilityRowData>,
}

/// The primary or the secondary ability.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlotCard {
    pub picture: String,
    pub words: String,
    /// "Armed" in the alarm color while armed.
    pub armed: Option<Colored>,
    /// Arm or Let go, and Pin, while the human has control.
    pub buttons: Vec<Colored>,
    pub hover: TipKey,
}

/// One ability of the list of every weapon ability.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AbilityRowData {
    pub picture: String,
    pub name: Colored,
    /// The slot of the weapon in hand that has it.
    pub slot: Option<&'static str>,
    pub hover: TipKey,
}

/// The racial panel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RacialData {
    pub icon: f32,
    pub rows: Vec<RacialRow>,
    /// The words for a race the shard does not name.
    pub none: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RacialRow {
    pub picture: String,
    pub name: &'static str,
    pub passive: Option<&'static str>,
    /// Use and Pin, for the ability that acts, while the human has control.
    pub buttons: Vec<Colored>,
    pub hover: Option<TipKey>,
}

/// The box of a party invite.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InviteData {
    pub words: String,
    /// Accept and Decline, while the human has control.
    pub accept: Option<Colored>,
    pub decline: Option<Colored>,
}

/// A slot or a racial ability by its place: `{"use": i}` (a click on its
/// icon or its first button), `{"pin": i}`, `{"drag": i}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AbilityAction {
    Use(usize),
    Pin(usize),
    Drag(usize),
}

/// `{"accept": true}` or `{"decline": true}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum InviteAction {
    Accept(bool),
    Decline(bool),
}

/// The two buttons beside an ability: the first in the goal color.
fn button_pair(first: &str, second: &str) -> Vec<Colored> {
    [(first, GOAL), (second, TEXT)]
        .into_iter()
        .map(|(words, color)| Colored {
            words: words.to_string(),
            color: css_color(color),
        })
        .collect()
}

impl WebView {
    pub(super) fn abilities_spec(&self) -> Option<FrameSpec> {
        places::is_open(&self.profile, ABILITIES_ID).then(|| {
            let default = abilities_first_place(self.panel_room());
            FrameSpec::fixed(ABILITIES_ID, WORDS_ABILITIES, default)
                .closable()
                .sized(ABILITIES_MIN)
        })
    }

    pub(super) fn abilities_data(&mut self, frame: &WatchFrame) -> Option<Framed<AbilitiesData>> {
        let spec = self.abilities_spec()?;
        let live = frame.human_control;
        let slots = AbilitySlot::BOTH
            .into_iter()
            .map(|slot| {
                let is_armed = armed(frame, slot);
                let words = ability_slot_words(frame, slot);
                SlotCard {
                    picture: self
                        .gump_picture(icon_of(ability_of(frame, slot)), slot_hue(frame, slot)),
                    armed: is_armed.then(|| Colored {
                        words: WORDS_ARMED.to_string(),
                        color: css_color(ALARM),
                    }),
                    buttons: if live {
                        button_pair(arm_words(is_armed), WORDS_PIN)
                    } else {
                        Vec::new()
                    },
                    hover: TipKey::label(&words, if live { HINT_SLOT } else { "" }),
                    words,
                }
            })
            .collect();
        let rows = ability_rows(frame)
            .into_iter()
            .map(|row| {
                let weapons = ability_weapon_names(row.ability, |graphic| {
                    self.art.item_tile(graphic).map(|tile| tile.name.clone())
                });
                AbilityRowData {
                    picture: self.gump_picture(row.icon, 0),
                    name: Colored {
                        words: row.name.to_string(),
                        color: css_color(if row.in_hand.is_some() { GOAL } else { TEXT }),
                    },
                    slot: row.in_hand.map(AbilitySlot::title),
                    hover: TipKey::label(row.name, &format!("{WORDS_WEAPONS}{weapons}")),
                }
            })
            .collect();
        let body = AbilitiesData {
            icon: ABILITY_ICON,
            slots,
            all_title: WORDS_ALL,
            rows,
        };
        Some(self.framed(PANEL_ABILITIES, &spec, body))
    }

    pub(super) fn abilities_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<AbilityAction>(action) else {
            return;
        };
        let slot_at = |at: usize| AbilitySlot::BOTH.get(at).copied();
        match action {
            AbilityAction::Use(at) => {
                if let Some(slot) = slot_at(at) {
                    self.hand.act(slot_act(&frame, slot));
                }
            }
            AbilityAction::Pin(at) => {
                if let Some(slot) = slot_at(at) {
                    self.pin_slot(&frame, slot_of(slot));
                }
            }
            AbilityAction::Drag(at) => {
                if let Some(slot) = slot_at(at) {
                    self.drag_slot(slot_of(slot));
                }
            }
        }
    }

    pub(super) fn racial_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        places::is_open(&self.profile, RACIAL_ID).then(|| {
            let default = racial_first_place(self.panel_room(), race_of(frame));
            FrameSpec::fixed(RACIAL_ID, WORDS_RACIAL, default).closable()
        })
    }

    pub(super) fn racial_data(&mut self, frame: &WatchFrame) -> Option<Framed<RacialData>> {
        let spec = self.racial_spec(frame)?;
        let race = race_of(frame);
        let rows = race.map_or_else(Vec::new, |race| {
            race.names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let acts = racial_act(frame, race, index).is_some();
                    RacialRow {
                        picture: self.gump_picture(race.icon(index), 0),
                        name,
                        passive: race.passive(index).then_some(WORDS_PASSIVE),
                        buttons: if acts {
                            button_pair(WORDS_RACIAL_USE, WORDS_PIN)
                        } else {
                            Vec::new()
                        },
                        hover: acts.then(|| TipKey::label(name, HINT_FLIGHT)),
                    }
                })
                .collect()
        });
        let body = RacialData {
            icon: ABILITY_ICON,
            rows,
            none: race.is_none().then_some(WORDS_NO_RACE),
        };
        Some(self.framed(PANEL_RACIAL, &spec, body))
    }

    pub(super) fn racial_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let (Some(race), Ok(action)) = (
            race_of(&frame),
            serde_json::from_value::<AbilityAction>(action),
        ) else {
            return;
        };
        match action {
            AbilityAction::Use(index) => {
                if let Some(act) = racial_act(&frame, race, index) {
                    self.hand.act(act);
                }
            }
            AbilityAction::Pin(index) => {
                if let Some(slot) = racial_slot(&frame, race, index) {
                    self.pin_slot(&frame, slot);
                }
            }
            AbilityAction::Drag(index) => {
                if let Some(slot) = racial_slot(&frame, race, index) {
                    self.drag_slot(slot);
                }
            }
        }
    }

    /// The box of an invite shows while the party tab of the sheet does
    /// not.
    pub(super) fn invite_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        frame.party_invite?;
        let sheet = &self.panels.sheet;
        if sheet.open && sheet.tab == Tab::Party {
            return None;
        }
        let default = invite_first_place(self.panel_room());
        Some(FrameSpec::fixed(INVITE_ID, WORDS_INVITE_TITLE, default))
    }

    pub(super) fn invite_data(&self, frame: &WatchFrame) -> Option<Framed<InviteData>> {
        let spec = self.invite_spec(frame)?;
        let leader = frame.party_invite?;
        let live = frame.human_control;
        let choice = |words: &str, color| Colored {
            words: words.to_string(),
            color: css_color(color),
        };
        let body = InviteData {
            words: invite_words(&inviter_name(frame, leader)),
            accept: live.then(|| choice(WORDS_ACCEPT, GOAL)),
            decline: live.then(|| choice(WORDS_DECLINE, ALARM)),
        };
        Some(self.framed(PANEL_INVITE, &spec, body))
    }

    pub(super) fn invite_action(&mut self, action: Value) {
        let live = self
            .frame
            .as_ref()
            .is_some_and(|frame| frame.human_control && frame.party_invite.is_some());
        if !live {
            return;
        }
        let command = match serde_json::from_value::<InviteAction>(action) {
            Ok(InviteAction::Accept(_)) => ACCEPT_COMMAND,
            Ok(InviteAction::Decline(_)) => DECLINE_COMMAND,
            Err(_) => return,
        };
        self.hand.act(Act::Command(command.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, settled, VIEW};
    use serde_json::json;
    use uoterm_view::model::abilities::toggle_command;

    const RACE_GARGOYLE: u8 = 3;
    const LEADER: u32 = 2;

    /// A gargoyle with an invite, the ability panels open.
    fn gargoyle(control: bool) -> WebView {
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["self_state"]["status"] = json!({ "race": RACE_GARGOYLE });
        watch["party_invite"] = json!(LEADER);
        watch["human_control"] = json!(control);
        let mut view = settled();
        view.frame(&watch.to_string(), 0.1);
        view.tick_native(0.1, VIEW, None);
        view.take_out_native();
        places::set_open(&mut view.profile, ABILITIES_ID, true);
        places::set_open(&mut view.profile, RACIAL_ID, true);
        view
    }

    #[test]
    fn a_slot_arms_and_the_flight_flies_as_the_window_does() {
        let mut view = gargoyle(true);
        let frame = view.frame.clone().unwrap();
        let acts = out_acts(&press(&mut view, PANEL_ABILITIES, json!({ "use": 1 })));
        let toggle = Act::Command(toggle_command(&frame, AbilitySlot::Secondary));
        assert_eq!(acts, vec![toggle.for_page()]);
        let race = race_of(&frame).unwrap();
        let fly = racial_act(&frame, race, 0).unwrap();
        let acts = out_acts(&press(&mut view, PANEL_RACIAL, json!({ "use": 0 })));
        assert_eq!(acts, vec![fly.for_page()]);
        assert!(out_acts(&press(&mut view, PANEL_RACIAL, json!({ "use": 1 }))).is_empty());
        press(&mut view, PANEL_RACIAL, json!({ "drag": 0 }));
        assert!(view.carries(), "the flight goes toward the hotbar");
    }

    #[test]
    fn nothing_of_the_ability_panels_and_the_invite_acts_without_control() {
        let mut view = gargoyle(false);
        for (panel, action) in [
            (PANEL_ABILITIES, json!({ "use": 0 })),
            (PANEL_RACIAL, json!({ "use": 0 })),
            (PANEL_INVITE, json!({ "accept": true })),
        ] {
            assert!(
                out_acts(&press(&mut view, panel, action)).is_empty(),
                "{panel}"
            );
        }
        let data = view.panel_data(0.0);
        assert!(data.abilities.unwrap().body.slots[0].buttons.is_empty());
        assert!(data.invite.unwrap().body.accept.is_none());
    }

    #[test]
    fn the_invite_answers_with_the_commands_of_the_window() {
        let mut view = gargoyle(true);
        let acts = out_acts(&press(&mut view, PANEL_INVITE, json!({ "decline": true })));
        assert_eq!(acts, vec![Act::Command(DECLINE_COMMAND.into()).for_page()]);
        view.panels.sheet.open = true;
        view.panels.sheet.tab = Tab::Party;
        assert!(
            view.panel_data(0.0).invite.is_none(),
            "the party tab shows the invite"
        );
    }
}
