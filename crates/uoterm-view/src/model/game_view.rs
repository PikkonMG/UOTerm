//! The size of the game view the window tells the shard, as the reference
//! client tells it when its game window is resized: once a size has held
//! for a moment, not on each picture of a drag. Each new login hears it
//! again, and the session tells the shard only a size it has not heard.

use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::Area;
use crate::settings::Profile;

/// A size holds this long, in seconds, before the shard hears it.
pub const GAME_VIEW_SETTLE_SECONDS: f64 = 0.5;
/// The serial of the character while none is in the world.
const NO_CHARACTER: u32 = 0;

/// The width and the height of the game view, in pixels.
pub type ViewSize = [u32; 2];

#[derive(Default)]
pub struct GameViewReport {
    /// The character and the size the session was last told.
    told: Option<(u32, ViewSize)>,
    /// The size the view has now, and since when.
    seen: Option<(ViewSize, f64)>,
}

impl GameViewReport {
    /// The act that tells the session the size of the view, once it is due.
    /// `serial` is the character in the world.
    pub fn due(&mut self, serial: u32, size: ViewSize, time: f64) -> Option<Act> {
        if serial == NO_CHARACTER {
            self.told = None;
            return None;
        }
        match self.seen {
            Some((held, since)) if held == size => {
                if time - since < GAME_VIEW_SETTLE_SECONDS {
                    return None;
                }
            }
            _ => {
                self.seen = Some((size, time));
                return None;
            }
        }
        if self.told == Some((serial, size)) {
            return None;
        }
        self.told = Some((serial, size));
        let [width, height] = size;
        Some(Act::GameView { width, height })
    }
}

/// What the window tells the shard by itself, with no click: the names of
/// the mobiles and corpses that come into view, as the classic client asks
/// for them, whether to show what stands in public houses (when a human
/// plays, and each time the General page changes it), and the size of the
/// game view.
#[derive(Default)]
pub struct ShardReports {
    game_view: GameViewReport,
    /// The last choice about public house content the shard was told.
    house_content_sent: Option<bool>,
}

impl ShardReports {
    /// The acts of one frame. `arrivals` came into view this frame, and
    /// `view` is where the world is drawn.
    pub fn acts(
        &mut self,
        frame: &WatchFrame,
        profile: &Profile,
        arrivals: Vec<u32>,
        view: Area,
        time: f64,
    ) -> Vec<Act> {
        let mut acts = Vec::new();
        if frame.human_control {
            acts.extend(arrivals.into_iter().map(Act::Look));
        }
        let house_content = profile.general.show_house_content;
        if frame.human_control && self.house_content_sent != Some(house_content) {
            acts.push(Act::HouseContent(house_content));
            self.house_content_sent = Some(house_content);
        }
        let size = [view.width().round() as u32, view.height().round() as u32];
        acts.extend(self.game_view.due(frame.serial, size, time));
        acts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Point, Vector};

    const ME: u32 = 0x0000_00AA;
    const SIZE: ViewSize = [800, 600];
    const WIDER: ViewSize = [900, 600];

    fn told(act: Option<Act>) -> Option<ViewSize> {
        match act {
            Some(Act::GameView { width, height }) => Some([width, height]),
            _ => None,
        }
    }

    #[test]
    fn a_size_is_told_once_it_has_held() {
        let mut report = GameViewReport::default();
        assert_eq!(told(report.due(ME, SIZE, 0.0)), None);
        assert_eq!(told(report.due(ME, SIZE, 0.1)), None, "it has not held");
        let settled = GAME_VIEW_SETTLE_SECONDS;
        assert_eq!(told(report.due(ME, SIZE, settled)), Some(SIZE));
        assert_eq!(told(report.due(ME, SIZE, settled * 4.0)), None, "once");
    }

    #[test]
    fn a_drag_is_told_where_it_stops() {
        let mut report = GameViewReport::default();
        let step = GAME_VIEW_SETTLE_SECONDS / 4.0;
        for (at, width) in [700, 750, 800, 850].into_iter().enumerate() {
            let size = [width, SIZE[1]];
            assert_eq!(told(report.due(ME, size, at as f64 * step)), None);
        }
        assert_eq!(told(report.due(ME, WIDER, 1.0)), None);
        let later = 1.0 + GAME_VIEW_SETTLE_SECONDS;
        assert_eq!(told(report.due(ME, WIDER, later)), Some(WIDER));
    }

    #[test]
    fn a_new_login_hears_the_size_again() {
        let mut report = GameViewReport::default();
        report.due(ME, SIZE, 0.0);
        assert_eq!(told(report.due(ME, SIZE, 1.0)), Some(SIZE));
        assert_eq!(told(report.due(NO_CHARACTER, SIZE, 2.0)), None);
        assert_eq!(told(report.due(ME, SIZE, 3.0)), Some(SIZE));
    }

    #[test]
    fn a_human_asks_names_and_tells_the_house_choice_once() {
        const MOBILE: u32 = 0x0000_00BB;
        let frame = WatchFrame {
            serial: ME,
            human_control: true,
            ..WatchFrame::default()
        };
        let profile = Profile::default();
        let mut reports = ShardReports::default();
        let view = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(800.4, 599.6));
        let acts = reports.acts(&frame, &profile, vec![MOBILE], view, 0.0);
        let house = profile.general.show_house_content;
        assert_eq!(acts, vec![Act::Look(MOBILE), Act::HouseContent(house)]);
        let later = reports.acts(&frame, &profile, Vec::new(), view, 1.0);
        assert_eq!(told(later.into_iter().next()), Some(SIZE));
        let agent = WatchFrame {
            human_control: false,
            ..frame
        };
        assert!(reports
            .acts(&agent, &profile, vec![MOBILE], view, 2.0)
            .is_empty());
    }
}
