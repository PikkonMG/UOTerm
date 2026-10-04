//! The guard the hand checks each act with is `uoterm_view::guard`, which
//! the window and the web client share. The window keeps the grab bags in
//! a file of the config folder.

pub use uoterm_view::guard::*;

use crate::window::kept;

/// The grab bags kept from the last run.
pub fn load_grab_bags() -> KeptGrabBags {
    kept::load(GRAB_BAGS_FILE)
}

pub fn save_grab_bags(grab_bags: &KeptGrabBags) {
    kept::save(GRAB_BAGS_FILE, grab_bags);
}
