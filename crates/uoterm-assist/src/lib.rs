//! Game data and pure rules for the assistant.
//!
//! Nothing here reads the network or the world. Each table holds facts about
//! the game: spell numbers and names, potion items, weapon special moves.
//! The runtime and the script engine look names up here.

pub mod abilities;
pub mod buffs;
pub mod items;
pub mod mobiles;
mod name;
pub mod skills;
pub mod spells;

pub use name::name_key;
