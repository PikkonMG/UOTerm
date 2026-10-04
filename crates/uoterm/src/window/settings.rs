//! The settings of the play window. The profile and its option table are
//! `uoterm_view::settings`, which the window and the web client share; the
//! files that keep the profiles (`store`) stay with the window.

pub use uoterm_view::settings::*;

mod store;

pub use store::{shard_address, CharacterKey, ProfileHome, ProfileStore};
