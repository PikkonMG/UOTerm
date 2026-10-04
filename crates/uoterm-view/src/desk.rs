//! What the panels and the map share while the human plays: the item on
//! the mouse, the places that take a dropped item, and what a drop does,
//! with the box that asks how much of a pile to move by the "Hold Shift
//! to split stacks" option. Each client draws the carried item and the
//! box its own way.

use crate::act::{Act, DropTo};
use crate::frame::WatchPackItem;
use crate::geom::{Area, Point, Vector};
use crate::model::clicks::asks_amount;

/// What a dropped item lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// A container, a pile to join, or a mobile to give to.
    Into(u32),
    /// The body of the character: the item is put on.
    Wear,
    /// A slot of the hotbar. The item stays where it is.
    Slot(usize),
}

/// A pile that waits for its amount.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Split {
    pub item: WatchPackItem,
    pub to: DropTo,
    pub amount: u16,
}

/// What lies under the mouse on the map, which only the map knows.
pub trait MapUnder {
    /// The thing drawn under the mouse, by its serial.
    fn thing_at(&self, at: Point) -> Option<u32>;
    /// The tile under the mouse, and the height of its floor.
    fn tile_at(&mut self, at: Point) -> (u16, u16, i8);
    /// True when items of this graphic stack into one pile.
    fn stacks(&self, graphic: u16) -> bool;
}

/// What happens to the carried item in one frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Landing {
    Nothing,
    /// The item stays on the mouse: its picture goes here.
    Carried(Point),
    /// The pile asks how many to move. The caller gives the split back
    /// to the desk with `ask_amount`, where the box that asks takes it with
    /// `take_split`. A drop on a hotbar slot is no landing: the desk keeps
    /// it in `slotted`.
    AskAmount(Split),
    Act(Act),
}

/// The mouse as the desk reads it in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeskMouse {
    /// None when the mouse left the window.
    pub at: Option<Point>,
    /// A mouse button came up.
    pub released: bool,
    pub shift: bool,
    /// The mouse is on a panel that is no zone, where a drop does nothing.
    pub on_panel: bool,
}

#[derive(Default)]
pub struct Desk {
    carry: Option<WatchPackItem>,
    /// Where the middle of the carried picture is from the mouse, in window
    /// points.
    grab: Vector,
    /// The item was picked up in this frame, so a mouse button that comes up
    /// in it, as the click of an Okay button, does not drop it.
    picked_now: bool,
    zones: Vec<(Area, Zone)>,
    split: Option<Split>,
    /// The slot an item was dropped on this frame, for the hotbar to take.
    pub slotted: Option<(usize, WatchPackItem)>,
    /// The container whose gump is under the mouse in this frame, and in
    /// the last one, whose item the other gumps mark.
    hovered_container: Option<u32>,
    marked_container: Option<u32>,
}

impl Desk {
    /// Call this first in each frame. The panels then name their zones again.
    pub fn begin(&mut self) {
        self.zones.clear();
        self.slotted = None;
        self.marked_container = self.hovered_container.take();
        self.picked_now = false;
    }

    /// A container gump is under the mouse: the other gumps mark its item.
    pub fn hover_container(&mut self, container: u32) {
        self.hovered_container = Some(container);
    }

    /// The container whose gump was under the mouse in the last frame.
    pub fn marked_container(&self) -> Option<u32> {
        self.marked_container
    }

    pub fn zone(&mut self, area: Area, zone: Zone) {
        self.zones.push((area, zone));
    }

    pub fn carries(&self) -> bool {
        self.carry.is_some()
    }

    /// A panel calls this when the human starts to drag one of its items.
    pub fn pick_up(&mut self, item: &WatchPackItem) {
        self.pick_up_at(item, Vector::ZERO);
    }

    /// Picks up an item that stays `grab` away from the mouse, as a gump
    /// with relative drag and drop holds it.
    pub fn pick_up_at(&mut self, item: &WatchPackItem, grab: Vector) {
        self.carry = Some(item.clone());
        self.grab = grab;
        self.picked_now = true;
    }

    /// The item on the mouse.
    pub fn carried(&self) -> Option<&WatchPackItem> {
        self.carry.as_ref()
    }

    /// For a gump that lands a dropped item itself, as a container puts it
    /// at the place of the mouse: the carried item and its grab, when a
    /// mouse button came up in this frame. The desk then carries nothing.
    pub fn land(&mut self, released: bool) -> Option<(WatchPackItem, Vector)> {
        if self.picked_now || !released {
            return None;
        }
        self.carry.take().map(|item| (item, self.grab))
    }

    /// The pile that waits for its amount, taken for the box that asks.
    pub fn take_split(&mut self) -> Option<Split> {
        self.split.take()
    }

    /// A pile waits for its amount until the box takes it.
    pub fn ask_amount(&mut self, split: Split) {
        self.split = Some(split);
    }

    /// The last zone named is on top, as it was drawn last.
    pub fn zone_at(&self, at: Point) -> Option<Zone> {
        self.zones
            .iter()
            .rev()
            .find(|(area, _)| area.contains(at))
            .map(|(_, zone)| *zone)
    }

    /// What the carried item does in this frame: it stays on the mouse, or
    /// lands where the button came up. A pile asks how many to move by the
    /// "Hold Shift to split stacks" option, `shift_to_split`.
    pub fn landing(
        &mut self,
        mouse: DeskMouse,
        shift_to_split: bool,
        map: &mut impl MapUnder,
    ) -> Landing {
        let Some(item) = self.carry.clone() else {
            return Landing::Nothing;
        };
        let Some(at) = mouse.at else {
            self.carry = None;
            return Landing::Nothing;
        };
        if !mouse.released || self.picked_now {
            return Landing::Carried(at + self.grab);
        }
        self.carry = None;
        let zone = self.zone_at(at);
        if let Some(Zone::Slot(slot)) = zone {
            self.slotted = Some((slot, item));
            return Landing::Nothing;
        }
        let to = match zone {
            Some(Zone::Wear) => return Landing::Act(Act::Wear(item.serial)),
            Some(Zone::Into(dest)) if dest != item.serial => DropTo::Into(dest),
            Some(_) => return Landing::Nothing,
            None if mouse.on_panel => return Landing::Nothing,
            None => match map.thing_at(at) {
                Some(serial) if serial != item.serial => DropTo::Into(serial),
                Some(_) => return Landing::Nothing,
                None => {
                    let (x, y, z) = map.tile_at(at);
                    DropTo::Ground { x, y, z }
                }
            },
        };
        if asks_amount(
            item.amount,
            map.stacks(item.graphic),
            shift_to_split,
            mouse.shift,
        ) {
            return Landing::AskAmount(Split {
                amount: item.amount,
                item,
                to,
            });
        }
        Landing::Act(Act::Move {
            item: item.serial,
            amount: item.amount.max(1),
            to,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAG: u32 = 0x4000_0002;
    const COINS: u32 = 0x4000_0003;
    const ORC: u32 = 9;
    const TILE: (u16, u16, i8) = (10, 20, 5);

    /// A map with one thing at the left and the ground at the right.
    struct Map;

    impl MapUnder for Map {
        fn thing_at(&self, at: Point) -> Option<u32> {
            (at.x < 0.0).then_some(ORC)
        }

        fn tile_at(&mut self, _at: Point) -> (u16, u16, i8) {
            TILE
        }

        fn stacks(&self, _graphic: u16) -> bool {
            true
        }
    }

    fn coins(amount: u16) -> WatchPackItem {
        WatchPackItem {
            serial: COINS,
            amount,
            ..WatchPackItem::default()
        }
    }

    fn released_at(x: f32) -> DeskMouse {
        DeskMouse {
            at: Some(Point::new(x, 0.0)),
            released: true,
            ..DeskMouse::default()
        }
    }

    #[test]
    fn the_zone_drawn_last_is_on_top() {
        let mut desk = Desk::default();
        let area = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(100.0, 100.0));
        desk.zone(area, Zone::Into(BAG));
        desk.zone(area.expand(-20.0), Zone::Wear);
        assert_eq!(desk.zone_at(Point::new(50.0, 50.0)), Some(Zone::Wear));
        assert_eq!(desk.zone_at(Point::new(5.0, 5.0)), Some(Zone::Into(BAG)));
        assert_eq!(desk.zone_at(Point::new(500.0, 5.0)), None);
        desk.begin();
        assert_eq!(desk.zone_at(Point::new(50.0, 50.0)), None);
    }

    #[test]
    fn an_item_stays_on_the_mouse_in_the_frame_it_was_picked_up() {
        let mut desk = Desk::default();
        desk.pick_up_at(&coins(1), Vector::new(3.0, 4.0));
        assert_eq!(
            desk.landing(released_at(10.0), false, &mut Map),
            Landing::Carried(Point::new(13.0, 4.0))
        );
        desk.begin();
        assert_eq!(
            desk.landing(released_at(10.0), false, &mut Map),
            Landing::Act(Act::Move {
                item: COINS,
                amount: 1,
                to: DropTo::Ground {
                    x: TILE.0,
                    y: TILE.1,
                    z: TILE.2
                },
            })
        );
        assert!(!desk.carries());
    }

    #[test]
    fn a_drop_on_a_thing_gives_it_and_a_pile_asks_its_amount() {
        let mut desk = Desk::default();
        desk.pick_up(&coins(5));
        desk.begin();
        let landing = desk.landing(released_at(-10.0), false, &mut Map);
        assert_eq!(
            landing,
            Landing::AskAmount(Split {
                item: coins(5),
                to: DropTo::Into(ORC),
                amount: 5,
            })
        );
    }

    #[test]
    fn a_drop_on_a_slot_leaves_the_item_for_the_hotbar() {
        const SLOT: usize = 2;
        let mut desk = Desk::default();
        desk.pick_up(&coins(1));
        desk.begin();
        desk.zone(
            Area::from_min_size(Point::new(0.0, -5.0), Vector::new(20.0, 10.0)),
            Zone::Slot(SLOT),
        );
        assert_eq!(
            desk.landing(released_at(10.0), false, &mut Map),
            Landing::Nothing
        );
        assert_eq!(desk.slotted, Some((SLOT, coins(1))));
    }
}
