//! What the panels show, apart from how they draw it: the data logic of the
//! Modern extras (grid containers, loot, durability, cooldowns, casting,
//! counters, buffs, info bar, journal, world map, agents, damage meter).
//! No module here draws. The Modern panels and the Classic gumps both read
//! from these, so a rule lives in one place.

pub mod abilities;
pub mod agents;
pub mod asked;
pub mod buffs;
pub mod casting;
pub mod chat;
pub mod clicks;
pub mod compare;
pub mod cooldowns;
pub mod counters;
pub mod creation;
pub mod deals;
pub mod dolls;
pub mod dps;
pub mod durability;
pub mod fonts;
pub mod grid;
pub mod health_bars;
pub mod highlight;
pub mod house_design;
pub mod hue_grid;
pub mod info_bar;
pub mod journal;
pub mod key_macros;
pub mod loot;
pub mod map_item;
pub mod options_draft;
pub mod pages;
pub mod party;
pub mod places;
pub mod properties;
pub mod race_change;
pub mod reads;
pub mod skills;
pub mod spell_data;
pub mod stats;
pub mod status;
pub mod system_chat;
pub mod world_map;
