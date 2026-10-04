//! The ring that opens round a right-click: what the human can do with the
//! thing under the mouse. The acts of the window come first, then the lines
//! of the shard's own context menu when they arrive.

pub use uoterm_view::ui::ring::{opens_menu, Subject};

use super::bridge;
use super::control::{Act, Hand};
use super::theme::{self, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_view::ui::ring::{ring_center, ring_lines, ring_points, RingLine};

const LINE_PAD: Vec2 = Vec2::new(12.0, 6.0);
const LINE_RADIUS: u8 = 14;
const HUB_RADIUS: f32 = 5.0;

struct Ring {
    center: Pos2,
    serial: u32,
    name: String,
    subject: Subject,
}

#[derive(Default)]
pub struct RingUi {
    open: Option<Ring>,
    /// The Classic style shows the menu of the shard as a classic gump
    /// instead of the ring.
    classic: bool,
    /// Where and for what thing the Classic style asked the shard for its
    /// menu, for the window to open the gump.
    classic_asked: Option<(Pos2, u32)>,
}

impl RingUi {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Opens the ring, and asks the shard for its own lines. In the
    /// Classic style only the shard is asked, and the window opens its gump.
    pub fn open_at(
        &mut self,
        center: Pos2,
        serial: u32,
        name: &str,
        subject: Subject,
        hand: &Hand,
    ) {
        if self.classic {
            self.classic_asked = Some((center, serial));
        } else {
            self.open = Some(Ring {
                center,
                serial,
                name: name.to_string(),
                subject,
            });
        }
        hand.act(Act::Menu(serial));
    }

    /// Follows the style of the window: the Classic style asks for the
    /// gump of the shard's menu instead of the ring.
    pub fn set_classic(&mut self, classic: bool) {
        self.classic = classic;
    }

    /// Where and for what thing the Classic style asked for a menu since
    /// the last frame.
    pub fn take_classic_ask(&mut self) -> Option<(Pos2, u32)> {
        self.classic_asked.take()
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
        let lines = ring_lines(ring.subject, ring.serial, frame);
        let center = bridge::pos2(ring_center(bridge::point(ring.center), bridge::area(rect)));
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
        let points = ring_points(bridge::point(center), lines.len());
        for (i, (line, point)) in lines.into_iter().zip(points).enumerate() {
            let RingLine {
                words,
                enabled,
                act,
            } = line;
            let point = bridge::pos2(point);
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
