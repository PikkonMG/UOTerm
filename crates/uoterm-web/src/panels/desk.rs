//! The item on the mouse, and the box that asks how many of a pile to
//! move. A panel starts a drag of an item or of a slot for the hotbar; the
//! page tells where the button came up and the zone of the panel it came
//! up over; `uoterm_view::desk` decides what the drop does, as in the Rust
//! window.

use super::{FrameSpec, Framed, PANEL_SPLIT};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_nav::TileFlagSet;
use uoterm_view::act::Act;
use uoterm_view::art::WorldArt;
use uoterm_view::desk::{
    split_first_place, split_title, Desk, DeskMouse, Landing, MapUnder, Split, Zone, CARRY_ALPHA,
    SPLIT_ID, WORDS_MOVE,
};
use uoterm_view::frame::{WatchFrame, WatchPackItem};
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::scene::SceneState;
use uoterm_view::ui::deck::Slot;

/// A drop lands in a zone this small round the point of the drop.
const DROP_SIDE: f32 = 1.0;

#[derive(Default)]
pub(crate) struct DeskState {
    pub desk: Desk,
    /// A slot of the sheet the player drags toward the hotbar.
    pub dragging: Option<Slot>,
    /// The pile that waits for its amount.
    pub split: Option<Split>,
}

/// What the player carries: the picture of the item or of the slot, or
/// its words when it has none.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CarriedData {
    pub picture: Option<String>,
    pub words: String,
    /// How opaque the carried picture shows.
    pub alpha: f32,
}

/// The box that asks how many of a pile to move.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SplitData {
    pub amount: u16,
    pub most: u16,
    pub go_words: &'static str,
}

/// The zone of a panel a drop lands on, as the panel data names it:
/// `{"into": serial}`, `"wear"`, or `{"slot": index}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropZone {
    Into(u32),
    Wear,
    Slot(usize),
}

impl From<DropZone> for Zone {
    fn from(zone: DropZone) -> Self {
        match zone {
            DropZone::Into(serial) => Zone::Into(serial),
            DropZone::Wear => Zone::Wear,
            DropZone::Slot(slot) => Zone::Slot(slot),
        }
    }
}

impl From<Zone> for DropZone {
    fn from(zone: Zone) -> Self {
        match zone {
            Zone::Into(serial) => DropZone::Into(serial),
            Zone::Wear => DropZone::Wear,
            Zone::Slot(slot) => DropZone::Slot(slot),
        }
    }
}

/// Where the button came up, in points of the view: over a zone of a
/// panel, over a panel that is no zone, or over the map.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Drop {
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub zone: Option<DropZone>,
    #[serde(default)]
    pub on_panel: bool,
    #[serde(default)]
    pub shift: bool,
}

/// `{"drop": Drop}` lets the carried thing go; `{"cancel": true}` drops
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DeskAction {
    Drop(Drop),
    Cancel(bool),
}

/// `{"amount": n}` sets how many; `{"go": true}` moves them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SplitAction {
    Amount(u16),
    Go(bool),
}

/// The map of the view under a drop.
struct ViewUnder<'a, A: WorldArt> {
    scene: &'a mut SceneState,
    art: &'a mut A,
    view: Area,
    frame: &'a WatchFrame,
}

impl<A: WorldArt> MapUnder for ViewUnder<'_, A> {
    fn thing_at(&self, at: Point) -> Option<u32> {
        self.scene.thing_at(at).map(|thing| thing.serial)
    }

    fn tile_at(&mut self, at: Point) -> (u16, u16, i8) {
        self.scene.tile_at(self.art, self.view, self.frame, at)
    }

    /// With no tile data the amount of the item tells.
    fn stacks(&self, graphic: u16) -> bool {
        self.art
            .item_tile(graphic)
            .is_none_or(|tile| tile.flags.contains(TileFlagSet::STACKABLE))
    }
}

impl WebView {
    /// Picks up an item of a panel or of the map onto the mouse.
    pub(crate) fn pick_up(&mut self, item: &WatchPackItem) {
        self.panels.desk.desk.pick_up(item);
    }

    /// Starts a drag of a slot of the sheet toward the hotbar.
    pub(crate) fn drag_slot(&mut self, slot: Slot) {
        self.panels.desk.dragging = Some(slot);
    }

    /// True while the player carries something.
    pub(crate) fn carries(&self) -> bool {
        self.panels.desk.desk.carries() || self.panels.desk.dragging.is_some()
    }

    pub(super) fn carried_data(&mut self) -> Option<CarriedData> {
        let frame = self.frame.clone()?;
        if let Some(slot) = self.panels.desk.dragging.clone() {
            return Some(CarriedData {
                picture: self.slot_picture_key(&slot, &frame),
                words: slot.words(&frame),
                alpha: CARRY_ALPHA,
            });
        }
        let item = self.panels.desk.desk.carried()?.clone();
        let request = self.item_picture_request(item.graphic, item.hue);
        Some(CarriedData {
            picture: Some(self.picture_key(&request)),
            words: item.name,
            alpha: CARRY_ALPHA,
        })
    }

    pub(super) fn desk_action(&mut self, action: Value) {
        match serde_json::from_value::<DeskAction>(action) {
            Ok(DeskAction::Drop(drop)) => self.drop_carried(drop),
            Ok(DeskAction::Cancel(_)) => {
                self.panels.desk.dragging = None;
                self.panels.desk.desk = Desk::default();
            }
            Err(_) => {}
        }
    }

    /// Lets the carried thing go where the button came up.
    fn drop_carried(&mut self, drop: Drop) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        // Nothing lands while the agent has the character: the item leaves
        // the mouse.
        if !frame.human_control {
            self.panels.desk.dragging = None;
            self.panels.desk.desk = Desk::default();
            return;
        }
        let at = Point::new(drop.x, drop.y);
        let zone = drop.zone.map(Zone::from);
        if let Some(slot) = self.panels.desk.dragging.take() {
            if let Some(Zone::Slot(at)) = zone {
                self.set_slot(&frame.name, at, Some(slot));
            }
            return;
        }
        let desk = &mut self.panels.desk.desk;
        desk.begin();
        if let Some(zone) = zone {
            desk.zone(
                Area::from_center_size(at, Vector::new(DROP_SIDE, DROP_SIDE)),
                zone,
            );
        }
        let mouse = DeskMouse {
            at: Some(at),
            released: true,
            shift: drop.shift,
            on_panel: drop.on_panel,
        };
        let mut map = ViewUnder {
            scene: &mut self.scene,
            art: &mut self.art,
            view: self.view,
            frame: &frame,
        };
        let shift_to_split = self.profile.general.shift_to_split_stacks;
        let landing = desk.landing(mouse, shift_to_split, &mut map);
        if let Some((slot, item)) = desk.slotted.take() {
            let what = Slot::Item {
                serial: item.serial,
                graphic: item.graphic,
                hue: item.hue,
                name: item.name,
            };
            self.set_slot(&frame.name, slot, Some(what));
        }
        match landing {
            Landing::AskAmount(split) => self.panels.desk.split = Some(split),
            Landing::Act(act) => self.hand.act(act),
            Landing::Nothing | Landing::Carried(_) => {}
        }
    }

    pub(super) fn split_spec(&self) -> Option<FrameSpec> {
        let split = self.panels.desk.split.as_ref()?;
        let default = split_first_place(self.panel_room());
        Some(FrameSpec::fixed(SPLIT_ID, &split_title(&split.item.name), default).closable())
    }

    pub(super) fn split_data(&self) -> Option<Framed<SplitData>> {
        let split = self.panels.desk.split.as_ref()?;
        let spec = self.split_spec()?;
        let body = SplitData {
            amount: split.amount,
            most: split.item.amount,
            go_words: WORDS_MOVE,
        };
        Some(self.framed(PANEL_SPLIT, &spec, body))
    }

    pub(super) fn split_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<SplitAction>(action) else {
            return;
        };
        match action {
            SplitAction::Amount(amount) => {
                if let Some(split) = self.panels.desk.split.as_mut() {
                    split.amount = amount.clamp(1, split.item.amount.max(1));
                }
            }
            SplitAction::Go(_) => {
                let live = self.frame.as_ref().is_some_and(|frame| frame.human_control);
                if let Some(split) = self.panels.desk.split.take().filter(|_| live) {
                    self.hand.act(Act::Move {
                        item: split.item.serial,
                        amount: split.amount,
                        to: split.to,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::super::{PANEL_DESK, PANEL_SPLIT};
    use super::*;
    use crate::tests::{settled, HATCHET, MARA};
    use serde_json::json;
    use uoterm_view::act::DropTo;

    const BAG: u32 = 0x4000_0099;
    const LOGS: u32 = 0x4000_0020;

    fn logs(amount: u16) -> WatchPackItem {
        WatchPackItem {
            serial: LOGS,
            graphic: 0x1BDD,
            amount,
            name: "logs".into(),
            ..WatchPackItem::default()
        }
    }

    #[test]
    fn an_item_dropped_on_a_zone_moves_into_it_as_in_the_window() {
        let mut view = settled();
        view.pick_up(&logs(1));
        assert!(view.panel_data(0.0).carried.is_some());
        let out = press(
            &mut view,
            PANEL_DESK,
            json!({"drop": {"x": 5, "y": 5, "zone": {"into": BAG}, "on_panel": true}}),
        );
        let moved = Act::Move {
            item: LOGS,
            amount: 1,
            to: DropTo::Into(BAG),
        };
        assert_eq!(out_acts(&out), vec![moved.for_page()]);
        assert!(view.panel_data(0.0).carried.is_none());
    }

    #[test]
    fn a_pile_asks_how_many_then_moves_them() {
        let mut view = settled();
        view.pick_up(&logs(10));
        let drop = json!({"drop": {"x": 5, "y": 5, "zone": "wear", "on_panel": true}});
        let out = press(&mut view, PANEL_DESK, drop);
        assert_eq!(out_acts(&out), vec![Act::Wear(LOGS).for_page()]);
        view.pick_up(&logs(10));
        // "Hold Shift to split stacks" is off: a pile asks with no Shift.
        let drop = json!({"drop": {"x": 5, "y": 5, "zone": {"into": BAG}, "on_panel": true}});
        assert!(out_acts(&press(&mut view, PANEL_DESK, drop)).is_empty());
        let split = view.panel_data(0.0).split.unwrap();
        assert_eq!((split.body.amount, split.body.most), (10, 10));
        press(&mut view, PANEL_SPLIT, json!({"amount": 4}));
        let out = press(&mut view, PANEL_SPLIT, json!({"go": true}));
        let moved = Act::Move {
            item: LOGS,
            amount: 4,
            to: DropTo::Into(BAG),
        };
        assert_eq!(out_acts(&out), vec![moved.for_page()]);
    }

    #[test]
    fn a_drop_on_a_slot_puts_the_item_on_the_hotbar() {
        let mut view = settled();
        let hatchet = WatchPackItem {
            serial: HATCHET,
            graphic: 0x0F43,
            name: "hatchet".into(),
            amount: 1,
            ..WatchPackItem::default()
        };
        view.pick_up(&hatchet);
        press(
            &mut view,
            PANEL_DESK,
            json!({"drop": {"x": 5, "y": 5, "zone": {"slot": 2}, "on_panel": true}}),
        );
        assert_eq!(
            view.hotbars
                .slot(MARA, 2)
                .unwrap()
                .words(view.frame_ref().unwrap()),
            "hatchet"
        );
    }

    #[test]
    fn a_skill_dragged_from_the_sheet_lands_on_a_slot_only() {
        let mut view = settled();
        view.drag_slot(Slot::Skill {
            id: 21,
            name: "Hiding".into(),
        });
        assert_eq!(view.panel_data(0.0).carried.unwrap().words, "Hiding");
        press(
            &mut view,
            PANEL_DESK,
            json!({"drop": {"x": 5, "y": 5, "zone": {"slot": 0}, "on_panel": true}}),
        );
        assert!(view.hotbars.slot(MARA, 0).is_some());
        assert!(!view.carries());
    }

    #[test]
    fn nothing_carried_lands_or_moves_without_control() {
        let mut view = settled();
        view.pick_up(&logs(10));
        let drop = json!({"drop": {"x": 5, "y": 5, "zone": {"into": BAG}, "on_panel": true}});
        press(&mut view, PANEL_DESK, drop.clone());
        let mut watch: serde_json::Value =
            serde_json::from_str(&crate::tests::fixture_watch_with_backpack()).unwrap();
        watch["human_control"] = json!(false);
        view.frame(&watch.to_string(), 0.1);
        assert!(out_acts(&press(&mut view, PANEL_SPLIT, json!({"go": true}))).is_empty());
        view.pick_up(&logs(1));
        assert!(out_acts(&press(&mut view, PANEL_DESK, drop)).is_empty());
        assert!(!view.carries(), "the item leaves the mouse");
    }
}
