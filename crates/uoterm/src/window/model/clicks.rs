//! What a click on an item in a panel or a gump does, apart from how it
//! draws: a single click targets the item while the shard waits for a
//! target, and else asks its name once the double click time is over; a
//! double click in that time cancels the name. A dragged pile asks how many
//! to move by the "Hold Shift to split stacks" option. The Modern panels and
//! the classic gumps click items through these.

use crate::view::WatchFrame;
use crate::window::control::Act;

/// A single click waits this long for a second one, in seconds.
pub const DOUBLE_CLICK_SECONDS: f64 = 0.35;

/// A single click that waits for the double click time before it asks for
/// the name of a thing.
#[derive(Default)]
pub struct ClickDelay {
    waiting: Option<(u32, f64)>,
}

impl ClickDelay {
    /// A click on a thing: it asks for the name later, unless a double
    /// click comes first.
    pub fn clicked(&mut self, serial: u32, time: f64) {
        self.waiting = Some((serial, time));
    }

    /// A double click came: the click waits no more.
    pub fn double_clicked(&mut self) {
        self.waiting = None;
    }

    /// The thing whose click waited long enough, once.
    pub fn due(&mut self, time: f64) -> Option<u32> {
        match self.waiting {
            Some((serial, at)) if time - at >= DOUBLE_CLICK_SECONDS => {
                self.waiting = None;
                Some(serial)
            }
            _ => None,
        }
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting.is_some()
    }

    /// A single click on an item: the act to send now. It targets the item
    /// while the shard waits for a target; else the click waits to ask the
    /// name, and nothing goes now.
    pub fn single_click(&mut self, frame: &WatchFrame, serial: u32, time: f64) -> Option<Act> {
        if frame.target_cursor {
            return Some(Act::Target(serial));
        }
        self.clicked(serial, time);
        None
    }

    /// The act that asks the name of the thing whose click waited long
    /// enough.
    pub fn due_look(&mut self, time: f64) -> Option<Act> {
        self.due(time).map(Act::Look)
    }
}

/// True when a dragged pile asks how many to move: a pile of a stackable
/// item, with Shift held when the "Hold Shift to split stacks" option is on,
/// or without it when the option is off.
pub fn asks_amount(amount: u16, stackable: bool, shift_to_split: bool, shift: bool) -> bool {
    amount > 1 && stackable && shift_to_split == shift
}

#[cfg(test)]
mod tests {
    use super::*;

    const COIN: u32 = 7;

    #[test]
    fn a_click_waits_for_the_double_click_time_and_a_double_click_cancels_it() {
        let mut clicks = ClickDelay::default();
        clicks.clicked(COIN, 1.0);
        assert_eq!(clicks.due(1.1), None);
        assert_eq!(clicks.due(1.0 + DOUBLE_CLICK_SECONDS), Some(COIN));
        assert_eq!(clicks.due(5.0), None);
        clicks.clicked(8, 2.0);
        clicks.double_clicked();
        assert!(!clicks.is_waiting());
        assert_eq!(clicks.due(9.0), None);
    }

    #[test]
    fn a_single_click_targets_under_a_target_cursor_and_else_names_later() {
        let mut clicks = ClickDelay::default();
        let aiming = WatchFrame {
            target_cursor: true,
            ..WatchFrame::default()
        };
        assert_eq!(
            clicks.single_click(&aiming, COIN, 1.0),
            Some(Act::Target(COIN))
        );
        assert!(!clicks.is_waiting(), "a target asks no name");
        assert_eq!(clicks.single_click(&WatchFrame::default(), COIN, 1.0), None);
        assert_eq!(clicks.due_look(1.1), None);
        assert_eq!(
            clicks.due_look(1.0 + DOUBLE_CLICK_SECONDS),
            Some(Act::Look(COIN))
        );
    }

    #[test]
    fn a_pile_asks_its_amount_by_the_split_option() {
        assert!(
            asks_amount(5, true, false, false),
            "option off: a plain drag"
        );
        assert!(
            !asks_amount(5, true, false, true),
            "option off: Shift moves all"
        );
        assert!(asks_amount(5, true, true, true), "option on: Shift asks");
        assert!(!asks_amount(5, true, true, false));
        assert!(!asks_amount(1, true, false, false), "one item is no pile");
        assert!(!asks_amount(5, false, false, false), "not stackable");
    }
}
