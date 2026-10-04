//! The rules of the play window that do not depend on how a window draws:
//! the watch frame, acts, settings, keys, the scene and the panels. The
//! Rust window and the web client both use them, so a rule lives in one
//! place. Nothing here opens a file, starts a thread or reads a clock; the
//! caller gives the time in seconds.

pub mod act;
pub mod actions;
pub mod art;
pub mod atlas;
pub mod audio;
pub mod clicks;
pub mod cursor;
pub mod desk;
pub mod filters;
pub mod floats;
pub mod frame;
pub mod geom;
pub mod guard;
pub mod input;
pub mod keys;
pub mod lights;
pub mod look;
pub mod map_lay;
pub mod model;
pub mod pad;
pub mod predict;
pub mod scene;
pub mod settings;
pub mod sky;
pub mod steer;
pub mod tips;
pub mod ui;
pub mod video;
