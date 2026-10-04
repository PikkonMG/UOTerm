//! The parts of the panel models that reach the host: the files of the
//! config folder and of the client, the worker thread of the session
//! reads, the clock of the computer and the egui textures. Each calls the
//! rules of `uoterm_view::model`.

pub mod compare;
pub mod fonts;
pub mod journal;
pub mod map_item;
pub mod reads;
pub mod world_map;
