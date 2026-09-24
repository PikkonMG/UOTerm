//! A column of rows inside a panel that may hold more than it shows: each
//! call lays one row under the last, and the wheel scrolls the column. A
//! row out of sight draws nothing and takes no click.

use super::super::atlas::Sprite;
use super::super::boxes_ui::CELL_RADIUS;
use super::super::theme::{self, text_font};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Vec2};

pub const ROW: f32 = 26.0;

/// One thing to pick by its picture: the picture, when the client files
/// have it, and its name.
pub type Picked = (Option<(egui::TextureId, Sprite)>, String);
const GAP: f32 = 4.0;
const WHEEL_STEP: f32 = 1.0;

pub struct Rows<'a> {
    ui: &'a mut egui::Ui,
    body: Rect,
    /// The top of the next row, before the scroll.
    y: f32,
    scroll: f32,
    key: &'a str,
}

impl<'a> Rows<'a> {
    /// A column in `body`, scrolled `scroll` points down. The wheel over
    /// the body moves the scroll.
    pub fn new(ui: &'a mut egui::Ui, body: Rect, key: &'a str, scroll: f32) -> Self {
        let turned = ui.input(|i| {
            let over = i.pointer.hover_pos().is_some_and(|p| body.contains(p));
            if over {
                i.raw_scroll_delta.y
            } else {
                0.0
            }
        });
        Self {
            ui,
            body,
            y: body.top(),
            scroll: (scroll - turned * WHEEL_STEP).max(0.0),
            key,
        }
    }

    /// The scroll to keep for the next frame, held so the last row stays in
    /// reach.
    pub fn finish(self) -> f32 {
        let content = self.y - self.body.top();
        self.scroll.min((content - self.body.height()).max(0.0))
    }

    /// The place of the next row, and whether it shows.
    fn next(&mut self, height: f32) -> Option<Rect> {
        let top = self.y - self.scroll;
        self.y += height + GAP;
        let row = Rect::from_min_size(
            Pos2::new(self.body.left(), top),
            Vec2::new(self.body.width(), height),
        );
        (row.top() >= self.body.top() && row.bottom() <= self.body.bottom()).then_some(row)
    }

    fn id(&self, part: &str, index: usize) -> Id {
        Id::new((self.key, part, index))
    }

    /// Words on a row of their own.
    pub fn words(&mut self, words: &str, color: Color32) {
        if let Some(row) = self.next(ROW) {
            self.ui.painter().text(
                row.left_center(),
                Align2::LEFT_CENTER,
                words,
                text_font(theme::SIZE_BODY),
                color,
            );
        }
    }

    /// A row of buttons of equal width. Gives the one pressed.
    pub fn buttons(&mut self, part: &str, labels: &[(&str, Color32)]) -> Option<usize> {
        let row = self.next(ROW)?;
        let count = labels.len().max(1) as f32;
        let width = (row.width() - GAP * (count - 1.0)) / count;
        let mut pressed = None;
        for (at, (words, color)) in labels.iter().enumerate() {
            let area = Rect::from_min_size(
                row.left_top() + Vec2::new(at as f32 * (width + GAP), 0.0),
                Vec2::new(width, row.height()),
            );
            if theme::segment_keyed(self.ui, area, self.id(part, at), words, *color) {
                pressed = Some(at);
            }
        }
        pressed
    }

    /// Words on the left and buttons on the right. Gives the one pressed.
    pub fn labeled(
        &mut self,
        part: &str,
        index: usize,
        words: &str,
        buttons: &[(&str, Color32)],
        button_width: f32,
    ) -> Option<usize> {
        let row = self.next(ROW)?;
        self.ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            words,
            text_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        let mut pressed = None;
        let mut right = row.right();
        for (at, (label, color)) in buttons.iter().enumerate().rev() {
            let area = Rect::from_min_size(
                Pos2::new(right - button_width, row.top()),
                Vec2::new(button_width, row.height()),
            );
            right = area.left() - GAP;
            let id = Id::new((self.key, part, index, at));
            if theme::segment_keyed(self.ui, area, id, label, *color) {
                pressed = Some(at);
            }
        }
        pressed
    }

    /// A field of words, and a button after it. Gives true when the button
    /// was pressed or Enter was.
    pub fn field(
        &mut self,
        part: &str,
        words: &mut String,
        hint: &str,
        button: &str,
        button_width: f32,
    ) -> bool {
        let Some(row) = self.next(ROW) else {
            return false;
        };
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - button_width - GAP, row.bottom()),
        );
        self.ui
            .painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = self.ui.put(
            field,
            egui::TextEdit::singleline(words)
                .id(self.id(part, 0))
                .frame(false)
                .hint_text(hint)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        let entered = typed.lost_focus() && self.ui.input(|i| i.key_pressed(egui::Key::Enter));
        let area = Rect::from_min_max(Pos2::new(field.right() + GAP, row.top()), row.max);
        let pressed = theme::segment_keyed(self.ui, area, self.id(part, 1), button, theme::TEXT);
        pressed || entered
    }

    /// A row of item pictures to pick one from. Gives the one clicked.
    pub fn pictures(&mut self, part: &str, pictures: &[Picked]) -> Option<usize> {
        let row = self.next(ROW * 2.0)?;
        let side = row.height();
        let mut picked = None;
        for (at, (picture, name)) in pictures.iter().enumerate() {
            let cell = Rect::from_min_size(
                row.left_top() + Vec2::new(at as f32 * (side + GAP), 0.0),
                Vec2::splat(side),
            );
            if cell.right() > row.right() {
                break;
            }
            let response = self
                .ui
                .interact(cell, self.id(part, at), egui::Sense::click());
            let fill = if response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::TRACK
            };
            self.ui
                .painter()
                .rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
            if let Some((texture, sprite)) = picture {
                self.ui.painter().image(
                    *texture,
                    theme::fit(cell, sprite.width, sprite.height),
                    sprite.uv,
                    Color32::WHITE,
                );
            }
            if response.hovered() {
                super::super::tips::label(self.ui, name, "");
            }
            if response.clicked() {
                picked = Some(at);
            }
        }
        picked
    }
}
