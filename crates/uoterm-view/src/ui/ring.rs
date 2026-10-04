//! The rules of the ring that opens round a right-click: what the human
//! can do with the thing under the mouse, the lines of the shard's own
//! context menu, which click opens it, and where each line stands.

use super::theme::SCREEN_MARGIN;
use crate::act::{Act, DropTo, WHOLE_PILE};
use crate::clicks::PickKind;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Vector};
use std::f32::consts::TAU;

pub const RING_RADIUS: f32 = 96.0;
/// A ring with many lines grows, so the lines do not touch.
const RING_GROW_PER_LINE: f32 = 7.0;
const RING_FREE_LINES: usize = 6;

const WORDS_USE: &str = "Use";
const WORDS_OPEN: &str = "Open";
const WORDS_LOOK: &str = "Look";
const WORDS_ATTACK: &str = "Attack";
const WORDS_FOLLOW: &str = "Follow";
const WORDS_TRADE: &str = "Trade";
const WORDS_LOOT: &str = "Loot";
const WORDS_TAKE: &str = "Take";
const WORDS_PROFILE: &str = "Profile";

/// What kind of thing the ring is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    OnMap(PickKind),
    /// An item in a container panel.
    Packed,
}

/// One line of the ring: its words, whether it may be picked, and the act
/// that picks it.
#[derive(Clone, Debug, PartialEq)]
pub struct RingLine {
    pub words: String,
    pub enabled: bool,
    pub act: Act,
}

/// The lines of the shard's own context menu for a thing, once they have
/// come.
pub fn shard_lines(frame: &WatchFrame, serial: u32) -> Vec<RingLine> {
    frame
        .context_menu
        .as_ref()
        .filter(|menu| menu.serial == serial)
        .map(|menu| {
            menu.lines
                .iter()
                .map(|line| RingLine {
                    words: line.words.clone(),
                    enabled: line.enabled,
                    act: Act::MenuPick {
                        serial,
                        index: line.index,
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

/// True when a click on a thing opens its context menu: a right click, and
/// in the Classic style with "Hold Shift for context menus" a click of
/// either button with Shift held, as the classic client asks.
pub fn opens_menu(right_click: bool, left_click: bool, shift: bool, shift_needed: bool) -> bool {
    if shift_needed {
        shift && (right_click || left_click)
    } else {
        right_click
    }
}

/// The acts of the window for one kind of thing, in ring order.
pub fn own_lines(subject: Subject, serial: u32, backpack: Option<u32>) -> Vec<(&'static str, Act)> {
    let take = backpack.map(|bag| {
        (
            WORDS_TAKE,
            Act::Move {
                item: serial,
                amount: WHOLE_PILE,
                to: DropTo::Into(bag),
            },
        )
    });
    match subject {
        Subject::OnMap(PickKind::Mobile) => vec![
            (WORDS_LOOK, Act::Look(serial)),
            (WORDS_PROFILE, Act::ProfileRead(serial)),
            (WORDS_ATTACK, Act::Attack(serial)),
            (WORDS_FOLLOW, Act::Follow(serial)),
            (WORDS_TRADE, Act::TradeWith(serial)),
            (WORDS_USE, Act::Use(serial)),
        ],
        Subject::OnMap(PickKind::Corpse) => vec![
            (WORDS_OPEN, Act::Use(serial)),
            (WORDS_LOOT, Act::Loot(serial)),
            (WORDS_LOOK, Act::Look(serial)),
        ],
        Subject::OnMap(PickKind::Item) => [(WORDS_USE, Act::Use(serial))]
            .into_iter()
            .chain(take)
            .chain([(WORDS_LOOK, Act::Look(serial))])
            .collect(),
        Subject::Packed => vec![
            (WORDS_USE, Act::Use(serial)),
            (WORDS_LOOK, Act::Look(serial)),
        ],
    }
}

/// Every line of the ring of a thing: the acts of the window first, then
/// the lines of the shard's menu when they arrive.
pub fn ring_lines(subject: Subject, serial: u32, frame: &WatchFrame) -> Vec<RingLine> {
    let mut lines: Vec<RingLine> = own_lines(subject, serial, frame.backpack())
        .into_iter()
        .map(|(words, act)| RingLine {
            words: words.to_string(),
            enabled: true,
            act,
        })
        .collect();
    lines.extend(shard_lines(frame, serial));
    lines
}

/// The middle of the ring: where the player clicked, held far enough
/// inside `window` that the ring fits.
pub fn ring_center(clicked: Point, window: Area) -> Point {
    let room = window.expand(-(RING_RADIUS + SCREEN_MARGIN));
    Point::new(
        clicked.x.clamp(room.min.x, room.max.x.max(room.min.x)),
        clicked.y.clamp(room.min.y, room.max.y.max(room.min.y)),
    )
}

/// The places of `count` lines round `center`, from the top, clockwise.
pub fn ring_points(center: Point, count: usize) -> Vec<Point> {
    let radius = RING_RADIUS + count.saturating_sub(RING_FREE_LINES) as f32 * RING_GROW_PER_LINE;
    (0..count)
        .map(|i| {
            let angle = TAU * i as f32 / count as f32 - TAU / 4.0;
            center + Vector::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchMenu, WatchMenuLine};

    const ORC: u32 = 9;
    const BAG: u32 = 0x4000_0002;

    #[test]
    fn the_first_line_is_at_the_top_and_each_line_is_as_far_from_the_center() {
        let center = Point::new(400.0, 300.0);
        let points = ring_points(center, 4);
        assert!((points[0].x - center.x).abs() < 0.01 && points[0].y < center.y);
        assert!(points[1].x > center.x, "the ring turns clockwise");
        for point in points {
            assert!(((point - center).length() - RING_RADIUS).abs() < 0.01);
        }
        let many = ring_points(center, RING_FREE_LINES + 4);
        assert!((many[0] - center).length() > RING_RADIUS);
    }

    #[test]
    fn a_ring_near_the_edge_moves_inside_the_window() {
        let window = Area::from_min_size(Point::default(), Vector::new(800.0, 600.0));
        let center = ring_center(Point::new(1.0, 599.0), window);
        assert_eq!(center.x, RING_RADIUS + SCREEN_MARGIN);
        assert_eq!(center.y, 600.0 - RING_RADIUS - SCREEN_MARGIN);
    }

    #[test]
    fn the_shard_lines_are_those_of_the_thing_and_come_after_the_own_ones() {
        let frame = WatchFrame {
            context_menu: Some(WatchMenu {
                serial: ORC,
                lines: vec![WatchMenuLine {
                    index: 3,
                    words: "Open Paperdoll".into(),
                    enabled: true,
                }],
            }),
            ..WatchFrame::default()
        };
        let pick = RingLine {
            words: "Open Paperdoll".to_string(),
            enabled: true,
            act: Act::MenuPick {
                serial: ORC,
                index: 3,
            },
        };
        assert_eq!(shard_lines(&frame, ORC), vec![pick.clone()]);
        assert!(shard_lines(&frame, BAG).is_empty());
        let lines = ring_lines(Subject::Packed, ORC, &frame);
        assert_eq!(lines.last(), Some(&pick));
        assert_eq!(lines.len(), 3);
        assert!(opens_menu(true, false, false, false));
        assert!(!opens_menu(false, true, false, false));
        assert!(!opens_menu(true, false, false, true));
        assert!(opens_menu(false, true, true, true));
    }

    #[test]
    fn a_thing_on_the_ground_can_be_taken_only_with_a_backpack() {
        let item = Subject::OnMap(PickKind::Item);
        let words = |lines: Vec<(&'static str, Act)>| -> Vec<&'static str> {
            lines.into_iter().map(|line| line.0).collect()
        };
        assert_eq!(
            words(own_lines(item, ORC, None)),
            vec![WORDS_USE, WORDS_LOOK]
        );
        assert!(words(own_lines(item, ORC, Some(BAG))).contains(&WORDS_TAKE));
        let mobile = own_lines(Subject::OnMap(PickKind::Mobile), ORC, Some(BAG));
        assert!(mobile.contains(&(WORDS_ATTACK, Act::Attack(ORC))));
    }
}
