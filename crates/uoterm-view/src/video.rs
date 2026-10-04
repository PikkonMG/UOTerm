//! The rules of the Video page that every client puts into effect the same
//! way: the time between two frames and the scale of the interface.

use crate::settings::VideoOptions;
use std::time::Duration;

/// The frame rates the classic client allows.
pub const FPS_MIN: u16 = 12;
pub const FPS_MAX: u16 = 250;
/// The interface scales no smaller or larger than this.
pub const UI_SCALE_MIN: f32 = 0.5;
pub const UI_SCALE_MAX: f32 = 3.0;

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

/// The scale of the whole interface, inside the range it allows.
pub fn ui_scale(video: &VideoOptions) -> f32 {
    video.ui_scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX)
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
    fn the_ui_scale_stays_in_its_range() {
        let video = VideoOptions {
            ui_scale: 10.0,
            ..VideoOptions::default()
        };
        assert_eq!(ui_scale(&video), UI_SCALE_MAX);
    }
}
