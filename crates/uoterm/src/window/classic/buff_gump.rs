//! The buff gump of the classic client: the icon of
//! each buff and debuff on the character in a row or a column of one of
//! four backgrounds, which its button turns through and the profile keeps
//! for the character. Each icon has a tooltip with its title, its words
//! and the time it has left; with "Show buff duration" on, the time shows
//! over the icon too. An icon with less than ten seconds left fades in and
//! out.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::WatchBuff;
use crate::window::model::buffs::{gump_of, ENDING_SECONDS};
use eframe::egui::Vec2;
use std::collections::HashMap;
use uoterm_nav::TextAlign;

pub const BUFFS: GumpKind = GumpKind {
    id: well_known::BUFFS,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(BuffGump::default()),
};

const TURN_BUTTON: ButtonArt = ButtonArt::new(0x7585, 0x7589, 0x7589);
/// One icon takes this much room along the row or the column.
const ICON_STEP: i32 = 31;
/// The icons of a background at the right end start this far from it.
const FROM_FAR_END: i32 = 48;
const TIME_FONT: u8 = 2;
const TIME_HUE: u16 = 0xFFFF;
const TIME_LEFT: i32 = 3;
const TIME_UP: i32 = 3;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const MILLIS_PER_SECOND: i64 = 1000;
// The fade of an ending icon, as the reference client steps it each frame.
const ALPHA_FULL: i32 = 255;
const ALPHA_LEAST: i32 = 60;
const FADE_WINDOW_MS: i64 = (ENDING_SECONDS as i64) * MILLIS_PER_SECOND;
const FADE_STEP_MS: i64 = 600;
const HALF: i32 = 2;

/// The four looks of the gump: where its icons go and where its button is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    LeftVertical,
    LeftHorizontal,
    RightVertical,
    RightHorizontal,
}

/// The backgrounds in the order the button turns through them, from the
/// first the gump opens with.
const LOOKS: [(u16, Direction); 4] = [
    (0x7580, Direction::LeftHorizontal),
    (0x7581, Direction::RightVertical),
    (0x7582, Direction::RightHorizontal),
    (0x757F, Direction::LeftVertical),
];

impl Direction {
    fn button_at(self) -> (i32, i32) {
        match self {
            Direction::LeftHorizontal => (-2, 36),
            Direction::RightVertical => (34, 78),
            Direction::RightHorizontal => (76, 36),
            Direction::LeftVertical => (0, 0),
        }
    }

    /// Where the icon at `index` goes on a background of `size`.
    fn icon_at(self, index: usize, size: Vec2) -> (i32, i32) {
        let offset = index as i32 * ICON_STEP;
        match self {
            Direction::LeftVertical => (25, 26 + offset),
            Direction::LeftHorizontal => (26 + offset, 5),
            Direction::RightVertical => (5, size.y as i32 - FROM_FAR_END - offset),
            Direction::RightHorizontal => (size.x as i32 - FROM_FAR_END - offset, 5),
        }
    }
}

/// Seconds as hours, minutes and seconds.
fn clock(secs: u64) -> (u64, u64, u64) {
    (
        secs / SECONDS_PER_HOUR,
        secs % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        secs % SECONDS_PER_MINUTE,
    )
}

/// The time an icon shows over itself: "+2hr", "4:05" or "09s".
fn time_words(secs: u64) -> String {
    let (hours, minutes, seconds) = clock(secs);
    if hours > 0 {
        format!("+{hours}hr")
    } else if minutes > 0 {
        format!("{minutes}:{seconds:02}")
    } else {
        format!("{seconds:02}s")
    }
}

/// The tooltip of an icon: its title and words, its number and the time
/// it has left.
fn tooltip_words(buff: &WatchBuff) -> String {
    let mut words = buff.title.clone();
    if !buff.text.is_empty() {
        words.push('\n');
        words.push_str(&buff.text);
    }
    words.push_str(&format!("\nID: {}", buff.icon));
    if let Some(secs) = buff.remaining_secs.filter(|secs| *secs > 0) {
        let (hours, minutes, seconds) = clock(secs);
        words.push_str(&format!(
            "\nTime left: {hours:02}:{minutes:02}:{seconds:02}"
        ));
    }
    words
}

/// How much an ending icon moves its opacity in one frame, by the time it
/// has left.
fn fade_step(remaining_ms: i64) -> i32 {
    ((FADE_WINDOW_MS - remaining_ms) / FADE_STEP_MS) as i32
}

/// The opacity of an icon and whether it fades out now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fade {
    alpha: i32,
    falling: bool,
}

impl Default for Fade {
    fn default() -> Self {
        Self {
            alpha: ALPHA_FULL,
            falling: true,
        }
    }
}

impl Fade {
    /// One frame of the fade of an icon with `remaining_ms` left.
    fn step(&mut self, remaining_ms: i64) {
        let step = fade_step(remaining_ms);
        if self.falling {
            self.alpha -= step;
            if self.alpha <= ALPHA_LEAST {
                self.alpha = ALPHA_LEAST;
                self.falling = false;
            }
        } else {
            self.alpha += step;
            if self.alpha >= ALPHA_FULL {
                self.alpha = ALPHA_FULL;
                self.falling = true;
            }
        }
    }

    fn share(self) -> f32 {
        self.alpha as f32 / ALPHA_FULL as f32
    }
}

/// The look the button turned to is kept in the profile, by its place in
/// [`LOOKS`].
#[derive(Default)]
pub struct BuffGump {
    /// How far the icons were moved right and down so none is left of or
    /// above the gump's corner.
    shift: (i32, i32),
    fades: HashMap<u16, Fade>,
}

impl GumpBody for BuffGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let look = cx.look() % LOOKS.len();
        let (background, direction) = LOOKS[look];
        let size = g.gump_size(background).unwrap_or(Vec2::ZERO);
        let buffs: Vec<&WatchBuff> = cx
            .frame
            .buff_icons
            .iter()
            .filter(|buff| gump_of(buff.icon).is_some())
            .collect();
        let button = direction.button_at();
        let icons: Vec<(i32, i32)> = (0..buffs.len())
            .map(|index| direction.icon_at(index, size))
            .collect();
        let min_x = icons
            .iter()
            .map(|at| at.0)
            .chain([0, button.0])
            .min()
            .unwrap_or(0);
        let min_y = icons
            .iter()
            .map(|at| at.1)
            .chain([0, button.1])
            .min()
            .unwrap_or(0);
        let shift = (-min_x.min(0), -min_y.min(0));
        if shift != self.shift {
            let moved = Vec2::new(
                (shift.0 - self.shift.0) as f32,
                (shift.1 - self.shift.1) as f32,
            );
            let scale = g.area(0, 0, Vec2::splat(1.0)).width();
            g.move_to(g.at(0, 0) - moved * scale);
            self.shift = shift;
        }
        let (dx, dy) = shift;
        g.pic(dx, dy, background, 0);
        if g.button("turn", button.0 + dx, button.1 + dy, TURN_BUTTON) {
            cx.set_look((look + 1) % LOOKS.len());
        }
        self.fades
            .retain(|icon, _| buffs.iter().any(|buff| buff.icon == *icon));
        let show_time = cx.profile.combat.buff_duration;
        for (buff, (x, y)) in buffs.into_iter().zip(icons) {
            let Some(gump) = gump_of(buff.icon) else {
                continue;
            };
            let fade = self.fades.entry(buff.icon).or_default();
            match buff.remaining_secs {
                Some(secs) if secs < ENDING_SECONDS => {
                    fade.step(secs as i64 * MILLIS_PER_SECOND);
                    g.ctx().request_repaint();
                }
                _ => *fade = Fade::default(),
            }
            let (x, y) = (x + dx, y + dy);
            let icon = g.faded(fade.share(), |g| g.pic(x, y, gump, 0));
            g.tooltip(&tooltip_words(buff));
            if let Some(secs) = buff.remaining_secs.filter(|_| show_time) {
                let look = TextLook::unicode(TIME_FONT, TIME_HUE)
                    .bordered()
                    .aligned(TextAlign::Center)
                    .wrap(icon.x as u32);
                let words = time_words(secs);
                g.faded(fade.share(), |g| {
                    g.label(
                        x - TIME_LEFT,
                        y + icon.y as i32 / HALF - TIME_UP,
                        &words,
                        &look,
                    )
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    #[test]
    fn the_time_reads_as_the_classic_client_writes_it() {
        assert_eq!(time_words(7300), "+2hr");
        assert_eq!(time_words(245), "4:05");
        assert_eq!(time_words(9), "09s");
        let buff = WatchBuff {
            icon: 1010,
            title: "Bless".into(),
            text: "+10 Str".into(),
            remaining_secs: Some(65),
            ..WatchBuff::default()
        };
        assert_eq!(
            tooltip_words(&buff),
            "Bless\n+10 Str\nID: 1010\nTime left: 00:01:05"
        );
    }

    #[test]
    fn the_looks_put_icons_along_their_side() {
        let size = Vec2::new(200.0, 100.0);
        assert_eq!(Direction::LeftHorizontal.icon_at(1, size), (57, 5));
        assert_eq!(Direction::LeftVertical.icon_at(2, size), (25, 88));
        assert_eq!(Direction::RightVertical.icon_at(0, size), (5, 52));
        assert_eq!(Direction::RightHorizontal.icon_at(1, size), (121, 5));
    }

    #[test]
    fn an_ending_icon_fades_down_and_back_up() {
        let mut fade = Fade::default();
        for _ in 0..40 {
            fade.step(1000);
        }
        assert!(fade.alpha >= ALPHA_LEAST && fade.alpha <= ALPHA_FULL);
        let mut fast = Fade::default();
        fast.step(0);
        assert_eq!(fast.alpha, ALPHA_FULL - fade_step(0));
    }

    #[test]
    fn the_buff_gump_draws_its_icons() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        profile.combat.buff_duration = true;
        let id = GumpId::one(well_known::BUFFS);
        assert!(manager.open(id, &mut profile));
        let frame = WatchFrame {
            buff_icons: vec![WatchBuff {
                icon: 1010,
                remaining_secs: Some(5),
                ..WatchBuff::default()
            }],
            ..WatchFrame::default()
        };
        if draw_frames(&mut manager, &mut profile, &frame) {
            assert!(manager
                .drawn_of(well_known::BUFFS)
                .iter()
                .any(|(open, _)| *open == id));
        }
    }
}
