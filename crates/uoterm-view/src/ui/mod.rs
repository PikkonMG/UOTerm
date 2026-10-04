//! The rules of the Modern panels that do not depend on how a window
//! draws: where each panel first stands, its frame, the launcher, the
//! ring, the hotbar, the gumps, the grids, the vitals, the health bars,
//! the lists of the sheet and the other panels, the text field and the
//! colors and sizes of the theme. The Rust window draws them with egui;
//! the browser draws them with its own page.

pub mod bars;
pub mod build;
pub mod chat_panel;
pub mod control_bar;
pub mod deals;
pub mod deck;
pub mod doll;
pub mod grid_clicks;
pub mod grids;
pub mod gump_frame;
pub mod gumps;
pub mod html;
pub mod hud;
pub mod hues;
pub mod launch;
pub mod layout;
pub mod lists;
pub mod map_item;
pub mod map_panel;
pub mod markers;
pub mod pages;
pub mod places;
pub mod quest_arrow;
pub mod question;
pub mod ring;
pub mod shard_asks;
pub mod shard_gump;
pub mod sheet;
pub mod text_field;
pub mod theme;
