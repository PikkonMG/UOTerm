//! The ring that opens round a right-click: what the human can do with the
//! thing under the mouse. The acts of the window come first, then the lines
//! of the shard's own context menu when they arrive.

use super::control::{Act, DropTo, Hand, WHOLE_PILE};
use super::scene::PickKind;
use super::theme::{self, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use std::f32::consts::TAU;

const RING_RADIUS: f32 = 96.0;
/// A ring with many lines grows, so the lines do not touch.
const RING_GROW_PER_LINE: f32 = 7.0;
const RING_FREE_LINES: usize = 6;
const LINE_PAD: Vec2 = Vec2::new(12.0, 6.0);
const LINE_RADIUS: u8 = 14;
const HUB_RADIUS: f32 = 5.0;

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

struct Ring {
    center: Pos2,
    serial: u32,
    name: String,
    subject: Subject,
}

#[derive(Default)]
pub struct RingUi {
    open: Option<Ring>,
}

/// The acts of the window for one kind of thing, in ring order.
fn own_lines(subject: Subject, serial: u32, backpack: Option<u32>) -> Vec<(&'static str, Act)> {
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

/// The places of `count` lines round `center`, from the top, clockwise.
fn ring_points(center: Pos2, count: usize) -> Vec<Pos2> {
    let radius = RING_RADIUS + count.saturating_sub(RING_FREE_LINES) as f32 * RING_GROW_PER_LINE;
    (0..count)
        .map(|i| {
            let angle = TAU * i as f32 / count as f32 - TAU / 4.0;
            center + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

impl RingUi {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Opens the ring, and asks the shard for its own lines.
    pub fn open_at(
        &mut self,
        center: Pos2,
        serial: u32,
        name: &str,
        subject: Subject,
        hand: &Hand,
    ) {
        self.open = Some(Ring {
            center,
            serial,
            name: name.to_string(),
            subject,
        });
        hand.act(Act::Menu(serial));
    }

    /// Draws the ring. Gives the places it covers, so the map does not
    /// take its clicks.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        hand: &Hand,
        profiles: &mut super::mapitem_ui::ProfileUi,
    ) -> Vec<Rect> {
        let Some(ring) = &self.open else {
            return Vec::new();
        };
        if !frame.human_control {
            self.open = None;
            return Vec::new();
        }
        let mut lines: Vec<(String, bool, Act)> =
            own_lines(ring.subject, ring.serial, frame.backpack())
                .into_iter()
                .map(|(words, act)| (words.to_string(), true, act))
                .collect();
        if let Some(menu) = frame
            .context_menu
            .as_ref()
            .filter(|m| m.serial == ring.serial)
        {
            lines.extend(menu.lines.iter().map(|line| {
                (
                    line.words.clone(),
                    line.enabled,
                    Act::MenuPick {
                        serial: ring.serial,
                        index: line.index,
                    },
                )
            }));
        }
        let room = rect.shrink(RING_RADIUS + theme::SCREEN_MARGIN);
        let center = Pos2::new(
            ring.center
                .x
                .clamp(room.left(), room.right().max(room.left())),
            ring.center
                .y
                .clamp(room.top(), room.bottom().max(room.top())),
        );
        let painter = ui.painter();
        painter.circle_filled(center, HUB_RADIUS, theme::GOAL);
        theme::shadowed_text(
            painter,
            center - Vec2::new(0.0, HUB_RADIUS * 2.0),
            Align2::CENTER_BOTTOM,
            &ring.name,
            title_font(theme::SIZE_PLATE),
            theme::TEXT,
        );
        let mut covered = Vec::new();
        let mut picked = None;
        let points = ring_points(center, lines.len());
        for (i, ((words, enabled, act), point)) in lines.into_iter().zip(points).enumerate() {
            let color = if enabled {
                theme::TEXT
            } else {
                theme::TEXT_FAINT
            };
            let galley = painter.layout_no_wrap(words, text_font(theme::SIZE_BODY), color);
            let area = Rect::from_center_size(point, galley.size() + LINE_PAD * 2.0);
            let response = ui.interact(area, Id::new(("ring-line", i)), Sense::click());
            let fill = if enabled && response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::GLASS
            };
            painter.rect_filled(area, CornerRadius::same(LINE_RADIUS), fill);
            painter.rect_stroke(
                area,
                CornerRadius::same(LINE_RADIUS),
                egui::Stroke::new(1.0, theme::GLASS_EDGE),
                egui::StrokeKind::Inside,
            );
            painter.galley(area.min + LINE_PAD, galley, color);
            if enabled && response.clicked() {
                picked = Some(act);
            }
            covered.push(area);
        }
        let away = ui.input(|i| i.pointer.any_click())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|at| !covered.iter().any(|area| area.contains(at)));
        if let Some(act) = picked {
            if let Act::ProfileRead(serial) = act {
                profiles.show(serial, hand);
            }
            let shard_line = matches!(act, Act::MenuPick { .. });
            hand.act(act);
            if !shard_line {
                hand.act(Act::MenuClose);
            }
            self.open = None;
        } else if away || ui.input(|i| i.key_pressed(Key::Escape)) {
            hand.act(Act::MenuClose);
            self.open = None;
        }
        covered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORC: u32 = 9;
    const BAG: u32 = 0x4000_0002;

    #[test]
    fn the_first_line_is_at_the_top_and_each_line_is_as_far_from_the_center() {
        let center = Pos2::new(400.0, 300.0);
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
