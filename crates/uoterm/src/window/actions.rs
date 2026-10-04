//! The actions of the play window. The registry, what each action does,
//! the macro runner and the other rules are `uoterm_view::actions`, which
//! the window and the web client share. The window keeps its controls of
//! each frame (`client`), the grab-bag file (`guard`), the Modern windows
//! (`modern`) and the screenshot files (`screenshot`).

pub mod client;
pub mod guard;
pub mod modern;
pub mod screenshot;

pub use uoterm_view::actions::*;
