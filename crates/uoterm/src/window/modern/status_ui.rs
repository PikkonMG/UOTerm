//! The status of the character on the sheet: the three stats with their
//! lock arrows, and every fact of the status in its sections (vitals,
//! belongings, resistances, combat and casting), in two columns the wheel
//! scrolls. The stat rows show on the worn view of the sheet too.

use super::super::boxes_ui::{scrolled, Tools};
use super::super::control::{Act, Hand};
use super::super::deck_ui::{lock_mark, LOCK_SIDE};
use super::super::model::status::{stats, StatLocks, STAT_NAMES};
use super::super::theme::{self, number_font, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Id, Pos2, Rect, Sense, Vec2};
use uoterm_view::ui::lists::{
    column_room, status_lines, StatusLine, STATUS_COLUMNS as COLUMNS, STATUS_LINE as LINE,
};

const HINT_LOCK: &str = "Click: up, down or locked.";

/// A fact: its words at the left of the row, its value at the right.
fn fact(painter: &egui::Painter, row: Rect, words: &str, value: &str) {
    painter.text(
        row.left_center(),
        Align2::LEFT_CENTER,
        words,
        text_font(theme::SIZE_SMALL),
        theme::TEXT_DIM,
    );
    painter.text(
        row.right_center(),
        Align2::RIGHT_CENTER,
        value,
        number_font(theme::SIZE_SMALL),
        theme::TEXT,
    );
}

/// One stat with its lock: a click on the lock turns it up, down or
/// locked, while the human has control.
pub fn stat_row(
    ui: &egui::Ui,
    row: Rect,
    stat: usize,
    frame: &WatchFrame,
    locks: &mut StatLocks,
    hand: &Hand,
) {
    let lock = Rect::from_center_size(
        Pos2::new(row.left() + LOCK_SIDE / 2.0, row.center().y),
        Vec2::splat(LOCK_SIDE),
    );
    let live = frame.human_control;
    let response = ui.interact(lock, Id::new(("stat-lock", stat)), Sense::click());
    let color = if live && response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    lock_mark(ui.painter(), lock, locks.shown(frame, stat), color);
    if live && response.hovered() {
        super::super::tips::label(ui, STAT_NAMES[stat], HINT_LOCK);
    }
    if live && response.clicked() {
        hand.act(Act::Command(locks.turn(frame, stat)));
    }
    let words = Rect::from_min_max(Pos2::new(lock.right() + theme::ROW_GAP, row.top()), row.max);
    fact(
        ui.painter(),
        words,
        STAT_NAMES[stat],
        &stats(frame)[stat].to_string(),
    );
}

/// Draws the status view in `body`.
pub fn draw(
    ui: &egui::Ui,
    body: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    locks: &mut StatLocks,
    first_line: &mut usize,
) {
    let lines = status_lines(frame);
    let (per_column, last_first) = column_room(body.height(), lines.len());
    *first_line = scrolled(ui, body, (*first_line).min(last_first), last_first);
    let column_width = (body.width() - theme::ROW_GAP * 2.0) / COLUMNS as f32;
    let shown = lines.iter().skip(*first_line).take(per_column * COLUMNS);
    for (at, line) in shown.enumerate() {
        let (column, row) = (at / per_column, at % per_column);
        let area = Rect::from_min_size(
            body.left_top()
                + Vec2::new(
                    column as f32 * (column_width + theme::ROW_GAP * 2.0),
                    row as f32 * LINE,
                ),
            Vec2::new(column_width, LINE),
        );
        match line {
            StatusLine::Title(title) => {
                ui.painter().text(
                    area.left_bottom(),
                    Align2::LEFT_BOTTOM,
                    *title,
                    title_font(theme::SIZE_SMALL),
                    theme::GOAL,
                );
            }
            StatusLine::Stat(stat) => stat_row(ui, area, *stat, frame, locks, tools.hand),
            StatusLine::Fact(words, value) => fact(ui.painter(), area, words, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_on_the_lock_of_a_stat_turns_it() {
        use super::super::super::settings::Profile;
        use super::super::testing::{click, draw_frames};
        let frame = WatchFrame {
            human_control: true,
            ..WatchFrame::default()
        };
        let body = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(400.0, 300.0));
        let strength_lock = Pos2::new(body.left() + LOCK_SIDE / 2.0, body.top() + LINE * 1.5);
        let mut locks = StatLocks::default();
        let mut first_line = 0;
        let mut profile = Profile::default();
        draw_frames(&mut profile, &click(strength_lock), |ui, _, tools, _| {
            draw(ui, body, &frame, tools, &mut locks, &mut first_line);
        });
        assert_eq!(locks.shown(&frame, 0), 1, "up turns to down");
        assert_eq!(locks.shown(&frame, 1), 0);
    }
}
