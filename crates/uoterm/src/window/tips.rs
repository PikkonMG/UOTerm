//! The tooltip of the thing under the mouse, in the words of the shard.
//! The window asks once when the mouse rests on a thing, and keeps the
//! answer for a short time, because the words of an item can change.

use super::control::Hand;
use super::theme::{self, text_font, title_font};
use eframe::egui::{self, Id, Pos2, Rect, Vec2};
use std::collections::HashMap;

const REST_SECONDS: f64 = 0.35;
const KEEP_SECONDS: f64 = 10.0;
const TIP_OFFSET: Vec2 = Vec2::new(18.0, 20.0);
const TIP_PAD: Vec2 = Vec2::new(10.0, 8.0);
const TIP_LINE_GAP: f32 = 2.0;

struct Known {
    lines: Vec<String>,
    at: f64,
}

#[derive(Default)]
pub struct Tips {
    known: HashMap<u32, Known>,
    /// The thing the mouse is on, and since when.
    resting: Option<(u32, f64)>,
    asked: Option<u32>,
}

impl Tips {
    /// Call this once in each frame, before the panels point at things.
    pub fn begin(&mut self, hand: &Hand, time: f64) {
        for tip in hand.new_tips() {
            if self.asked == Some(tip.serial) {
                self.asked = None;
            }
            self.known.insert(
                tip.serial,
                Known {
                    lines: tip.lines,
                    at: time,
                },
            );
        }
        self.known.retain(|_, known| time - known.at < KEEP_SECONDS);
    }

    /// The mouse is on this thing now. `fallback` shows until the shard
    /// answers, and when it has no words for the thing.
    pub fn point_at(
        &mut self,
        ui: &egui::Ui,
        hand: &Hand,
        serial: u32,
        fallback: &str,
        footer: &str,
        time: f64,
    ) {
        let since = match self.resting {
            Some((resting, since)) if resting == serial => since,
            _ => {
                self.resting = Some((serial, time));
                time
            }
        };
        let rested = time - since >= REST_SECONDS;
        if rested && !self.known.contains_key(&serial) && self.asked != Some(serial) {
            self.asked = Some(serial);
            hand.want_tip(serial);
        }
        if !rested {
            ui.ctx().request_repaint();
        }
        let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        let lines: Vec<&str> = match self.known.get(&serial) {
            Some(known) if !known.lines.is_empty() => {
                known.lines.iter().map(String::as_str).collect()
            }
            _ if fallback.is_empty() => Vec::new(),
            _ => vec![fallback],
        };
        draw(ui, mouse, &lines, footer);
    }
}

/// A tooltip for a thing the shard has no words for: a skill, a command.
pub fn label(ui: &egui::Ui, words: &str, footer: &str) {
    if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
        draw(ui, mouse, &[words], footer);
    }
}

/// The first line is the name, in the title face. The footer tells what a
/// click does.
fn draw(ui: &egui::Ui, mouse: Pos2, lines: &[&str], footer: &str) {
    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("tips")));
    let mut galleys = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let (font, color) = if i == 0 {
            (title_font(theme::SIZE_PLATE), theme::TEXT)
        } else {
            (text_font(theme::SIZE_SMALL), theme::TEXT_DIM)
        };
        galleys.push((
            painter.layout_no_wrap((*line).to_string(), font, color),
            color,
        ));
    }
    if !footer.is_empty() {
        let font = text_font(theme::SIZE_SMALL);
        galleys.push((
            painter.layout_no_wrap(footer.to_string(), font, theme::GOAL),
            theme::GOAL,
        ));
    }
    if galleys.is_empty() {
        return;
    }
    let width = galleys.iter().map(|(g, _)| g.size().x).fold(0.0, f32::max);
    let height: f32 = galleys.iter().map(|(g, _)| g.size().y + TIP_LINE_GAP).sum();
    let screen = ui.ctx().screen_rect();
    let size = Vec2::new(width, height) + TIP_PAD * 2.0;
    let mut corner = mouse + TIP_OFFSET;
    corner.x = corner.x.min(screen.right() - size.x).max(screen.left());
    corner.y = corner.y.min(screen.bottom() - size.y).max(screen.top());
    let area = Rect::from_min_size(corner, size);
    theme::panel(&painter, area);
    let mut y = area.top() + TIP_PAD.y;
    for (galley, color) in galleys {
        let line_height = galley.size().y;
        painter.galley(Pos2::new(area.left() + TIP_PAD.x, y), galley, color);
        y += line_height + TIP_LINE_GAP;
    }
}
