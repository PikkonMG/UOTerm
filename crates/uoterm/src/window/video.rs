//! The Video page applied to the window: how it sits on the screen, its
//! size, the frame rate while it is in front and while it is not, and the
//! scale of the whole interface. VSync is chosen when the window opens.

use super::settings::{VideoOptions, WindowMode};
use eframe::egui::{self, Vec2, ViewportBuilder, ViewportCommand};
use std::time::Duration;

/// The frame rates the classic client allows.
const FPS_MIN: u16 = 12;
const FPS_MAX: u16 = 250;
/// The interface scales no smaller or larger than this.
const UI_SCALE_MIN: f32 = 0.5;
const UI_SCALE_MAX: f32 = 3.0;

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
            ui_scale: video.ui_scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX),
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

/// The time between two frames of the window, from the frame rate of the
/// Video page, or its lower rate while the window is not in front.
pub fn frame_interval(video: &VideoOptions, focused: bool) -> Duration {
    let fps = if !focused && video.reduce_fps_when_inactive {
        video.inactive_fps
    } else {
        video.fps
    };
    Duration::from_secs_f64(1.0 / f64::from(fps.clamp(FPS_MIN, FPS_MAX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_slows_down_when_it_is_not_in_front() {
        let mut video = VideoOptions {
            fps: 60,
            inactive_fps: 15,
            ..VideoOptions::default()
        };
        assert_eq!(
            frame_interval(&video, true),
            Duration::from_secs_f64(1.0 / 60.0)
        );
        assert_eq!(
            frame_interval(&video, false),
            Duration::from_secs_f64(1.0 / 15.0)
        );
        video.reduce_fps_when_inactive = false;
        assert_eq!(
            frame_interval(&video, false),
            Duration::from_secs_f64(1.0 / 60.0)
        );
        video.fps = 1;
        assert_eq!(
            frame_interval(&video, true),
            Duration::from_secs_f64(1.0 / f64::from(FPS_MIN))
        );
    }

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

    #[test]
    fn the_ui_scale_stays_in_its_range() {
        let video = VideoOptions {
            ui_scale: 10.0,
            ..VideoOptions::default()
        };
        assert_eq!(Applied::of(&video).ui_scale, UI_SCALE_MAX);
    }
}
