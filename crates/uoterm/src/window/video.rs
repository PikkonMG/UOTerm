//! The Video page applied to the window: how it sits on the screen, its
//! size, the frame rate while it is in front and while it is not, and the
//! scale of the whole interface. VSync is chosen when the window opens.
//! The frame rate and the scale rules are `uoterm_view::video`.

pub use uoterm_view::video::frame_interval;

use super::settings::{VideoOptions, WindowMode};
use eframe::egui::{self, Vec2, ViewportBuilder, ViewportCommand};
use uoterm_view::video::ui_scale;

/// The part of the Video page the window has put into effect.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Applied {
    mode: WindowMode,
    size: Vec2,
    ui_scale: f32,
}

impl Applied {
    fn of(video: &VideoOptions) -> Self {
        Self {
            mode: video.window_mode,
            size: Vec2::new(video.window_width, video.window_height),
            ui_scale: ui_scale(video),
        }
    }
}

/// Keeps the window as the Video page says.
pub struct Video {
    applied: Applied,
}

impl Video {
    /// The window opens as `video` says, so that is in effect at first.
    pub fn opened_as(video: &VideoOptions) -> Self {
        Self {
            applied: Applied::of(video),
        }
    }

    /// Puts each change of the Video page into effect.
    pub fn apply(&mut self, ctx: &egui::Context, video: &VideoOptions) {
        let wanted = Applied::of(video);
        if wanted.mode != self.applied.mode || wanted.size != self.applied.size {
            for command in mode_commands(wanted.mode, wanted.size) {
                ctx.send_viewport_cmd(command);
            }
        }
        if ctx.zoom_factor() != wanted.ui_scale {
            ctx.set_zoom_factor(wanted.ui_scale);
        }
        self.applied = wanted;
    }
}

/// The commands that put the window in a mode, at a size when it has a
/// frame of its own.
fn mode_commands(mode: WindowMode, size: Vec2) -> Vec<ViewportCommand> {
    match mode {
        WindowMode::Windowed => vec![
            ViewportCommand::Fullscreen(false),
            ViewportCommand::Decorations(true),
            ViewportCommand::Maximized(false),
            ViewportCommand::InnerSize(size),
        ],
        WindowMode::Borderless => vec![
            ViewportCommand::Fullscreen(false),
            ViewportCommand::Decorations(false),
            ViewportCommand::Maximized(true),
        ],
        WindowMode::Fullscreen => vec![ViewportCommand::Fullscreen(true)],
    }
}

/// The window as it opens, in the mode and at the size of the Video page.
pub fn opening_viewport(builder: ViewportBuilder, video: &VideoOptions) -> ViewportBuilder {
    let builder = builder.with_inner_size([video.window_width, video.window_height]);
    match video.window_mode {
        WindowMode::Windowed => builder,
        WindowMode::Borderless => builder.with_decorations(false).with_maximized(true),
        WindowMode::Fullscreen => builder.with_fullscreen(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mode_has_its_own_commands() {
        let size = Vec2::new(800.0, 600.0);
        assert!(
            mode_commands(WindowMode::Windowed, size).contains(&ViewportCommand::InnerSize(size))
        );
        assert!(mode_commands(WindowMode::Borderless, size)
            .contains(&ViewportCommand::Decorations(false)));
        assert_eq!(
            mode_commands(WindowMode::Fullscreen, size),
            vec![ViewportCommand::Fullscreen(true)]
        );
    }
}
