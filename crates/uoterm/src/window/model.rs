//! What the panels show, apart from how they draw it. The rules are
//! `uoterm_view::model`, which the window and the web client share; the
//! parts that read files, start threads, read the clock of the computer or
//! make egui textures (`host`) stay with the window.

pub use uoterm_view::model::*;

pub mod host;
