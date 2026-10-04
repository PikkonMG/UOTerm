//! The clicks on the quest arrow of the Modern style: a click with the left
//! or the right button tells the shard (0xBF 0x07), as the classic arrow
//! does, while the human has control. Where the arrow stands is
//! `scene::overlays::quest_arrow`.

use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::Area;

/// The arrow takes clicks this far round its point too, so it is easy to
/// hit.
const CLICK_ROOM: f32 = 6.0;
pub const HINT_ARROW: &str = "The shard points here. Click: tell the shard.";

/// The place the arrow whose box is `arrow` takes clicks in, so the map
/// does not take them.
pub fn arrow_click_area(arrow: Area) -> Area {
    arrow.expand(CLICK_ROOM)
}

/// The act of a click on the arrow, with the right button or the left.
/// None without control.
pub fn arrow_act(frame: &WatchFrame, right: bool) -> Option<Act> {
    frame.human_control.then_some(Act::QuestArrow { right })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Point, Vector};

    #[test]
    fn a_click_on_the_arrow_tells_the_shard_with_control_only() {
        let mut frame = WatchFrame {
            human_control: true,
            ..WatchFrame::default()
        };
        assert_eq!(
            arrow_act(&frame, true),
            Some(Act::QuestArrow { right: true })
        );
        frame.human_control = false;
        assert_eq!(arrow_act(&frame, false), None);
        let arrow = Area::from_min_size(Point::new(10.0, 10.0), Vector::new(4.0, 4.0));
        assert_eq!(arrow_click_area(arrow).width(), 4.0 + CLICK_ROOM * 2.0);
    }
}
