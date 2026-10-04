//! The tooltip of the thing under the mouse, drawn by the window. Which
//! words it shows and when the shard is asked are `uoterm_view::tips`.

pub use uoterm_view::tips::*;

use super::control::Hand;
use super::theme::{self, text_font, title_font};
use eframe::egui::{self, Id, Pos2, Rect, Vec2};

const TIP_OFFSET: Vec2 = Vec2::new(18.0, 20.0);
const TIP_PAD: Vec2 = Vec2::new(10.0, 8.0);
const TIP_LINE_GAP: f32 = 2.0;

/// The mouse is on this thing now. `fallback` shows until the shard
/// answers, and when it has no words for the thing.
pub fn point_at(
    tips: &mut Tips,
    ui: &egui::Ui,
    hand: &Hand,
    serial: u32,
    fallback: &str,
    footer: &str,
    time: f64,
) {
    point_at_with(tips, ui, hand, serial, fallback, &[], footer, time);
}

/// The tooltip of a thing with lines of the window's own under the
/// shard's words: a compare, or what a bag holds.
#[allow(clippy::too_many_arguments)]
pub fn point_at_with(
    tips: &mut Tips,
    ui: &egui::Ui,
    hand: &Hand,
    serial: u32,
    fallback: &str,
    extra: &[String],
    footer: &str,
    time: f64,
) {
    if !tips.rest_on(serial, time, |serial| hand.want_tip(serial)) {
        ui.ctx().request_repaint();
    }
    let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    draw(ui, mouse, &tips.shown(serial, fallback, extra), footer);
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
