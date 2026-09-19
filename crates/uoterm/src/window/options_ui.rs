//! The options of the window. For now they are the sound: one volume for
//! each kind of sound, a master volume over them, and a switch for silence.
//! The window saves them when the operator lets go of a slider.

use super::audio::Audio;
use super::theme::{self, number_font, text_font, title_font};
use eframe::egui::{self, Align2, CornerRadius, Id, Pos2, Rect, Sense, Vec2};

const PANEL_WIDTH: f32 = 380.0;
const ROW: f32 = 34.0;
const TITLE_ROW: f32 = 34.0;
const LABEL_WIDTH: f32 = 116.0;
const NUMBER_WIDTH: f32 = 44.0;
const TRACK_HEIGHT: f32 = 6.0;
const KNOB_RADIUS: f32 = 8.0;
const PERCENT: f32 = 100.0;

const WORDS_TITLE: &str = "Sound";
const WORDS_MUTED: &str = "Silence is on";
const WORDS_NOT_MUTED: &str = "Silence is off";
const WORDS_CLOSE: &str = "Close";
const SLIDERS: [&str; 4] = ["Master", "Music", "Sound effects", "Footsteps"];

#[derive(Default)]
pub struct OptionsUi {
    open: bool,
}

/// The value a slider has when the mouse is at `mouse_x` on its track.
fn value_at(track: Rect, mouse_x: f32) -> f32 {
    ((mouse_x - track.left()) / track.width()).clamp(0.0, 1.0)
}

impl OptionsUi {
    /// Where the panel is when it is open, so the map does not take its
    /// clicks. `notes` is how many lines of notes the panel shows.
    pub fn panel(&self, rect: Rect, notes: usize) -> Option<Rect> {
        let rows = SLIDERS.len() + 2 + notes;
        let size = Vec2::new(
            PANEL_WIDTH,
            TITLE_ROW + rows as f32 * ROW + theme::PANEL_PAD * 2.0,
        );
        self.open
            .then(|| Rect::from_center_size(rect.center(), size))
    }

    /// Opens the panel, or closes it. The button is in the control bar.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Draws the panel when it is open.
    pub fn draw(&mut self, ui: &egui::Ui, rect: Rect, audio: &mut Audio) {
        let notes = usize::from(!audio.note().is_empty());
        let Some(panel) = self.panel(rect, notes) else {
            return;
        };
        let painter = ui.painter();
        theme::panel(painter, panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        painter.text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let mut y = inner.top() + TITLE_ROW;
        if notes > 0 {
            painter.text(
                Pos2::new(inner.left(), y + ROW / 2.0),
                Align2::LEFT_CENTER,
                audio.note(),
                text_font(theme::SIZE_BODY),
                theme::WAITING,
            );
            y += ROW;
        }
        let before = audio.settings;
        let mut released = false;
        let values = [
            &mut audio.settings.master,
            &mut audio.settings.music,
            &mut audio.settings.effects,
            &mut audio.settings.footsteps,
        ];
        for (label, value) in SLIDERS.into_iter().zip(values) {
            let row =
                Rect::from_min_size(Pos2::new(inner.left(), y), Vec2::new(inner.width(), ROW));
            released |= slider(ui, row, label, value);
            y += ROW;
        }
        let muted_words = if audio.settings.muted {
            WORDS_MUTED
        } else {
            WORDS_NOT_MUTED
        };
        let (_, toggled) = theme::button(ui, Pos2::new(inner.left(), y), muted_words, theme::TEXT);
        if toggled {
            audio.settings.muted = !audio.settings.muted;
        }
        y += ROW;
        let (_, closed) = theme::button(ui, Pos2::new(inner.left(), y), WORDS_CLOSE, theme::TEXT);
        if closed {
            self.open = false;
        }
        if audio.settings != before {
            audio.settings_changed();
        }
        if released || toggled {
            audio.settings.save();
        }
    }
}

/// One volume slider. True when the operator let go of it.
fn slider(ui: &egui::Ui, row: Rect, label: &str, value: &mut f32) -> bool {
    let painter = ui.painter();
    painter.text(
        row.left_center(),
        Align2::LEFT_CENTER,
        label,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    let track = Rect::from_min_max(
        Pos2::new(
            row.left() + LABEL_WIDTH,
            row.center().y - TRACK_HEIGHT / 2.0,
        ),
        Pos2::new(
            row.right() - NUMBER_WIDTH,
            row.center().y + TRACK_HEIGHT / 2.0,
        ),
    );
    let touch = track.expand2(Vec2::new(KNOB_RADIUS, KNOB_RADIUS * 1.5));
    let response = ui.interact(
        touch,
        Id::new(("options-slider", label)),
        Sense::click_and_drag(),
    );
    if let Some(mouse) = response.interact_pointer_pos() {
        *value = value_at(track, mouse.x);
    }
    let radius = CornerRadius::same(theme::BAR_RADIUS);
    painter.rect_filled(track, radius, theme::TRACK);
    let mut fill = track;
    fill.set_width(track.width() * *value);
    painter.rect_filled(fill, radius, theme::GOAL);
    let knob = if response.hovered() || response.dragged() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    painter.circle_filled(Pos2::new(fill.right(), track.center().y), KNOB_RADIUS, knob);
    painter.text(
        row.right_center(),
        Align2::RIGHT_CENTER,
        format!("{:.0}", *value * PERCENT),
        number_font(theme::SIZE_BODY),
        theme::TEXT,
    );
    response.drag_stopped() || response.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slider_reads_the_mouse_along_its_track() {
        let track = Rect::from_min_size(Pos2::new(100.0, 0.0), Vec2::new(200.0, TRACK_HEIGHT));
        assert_eq!(value_at(track, 100.0), 0.0);
        assert_eq!(value_at(track, 200.0), 0.5);
        assert_eq!(value_at(track, 300.0), 1.0);
        assert_eq!(value_at(track, 50.0), 0.0);
        assert_eq!(value_at(track, 900.0), 1.0);
    }

    #[test]
    fn the_panel_has_a_place_only_when_it_is_open() {
        let window = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
        let mut options = OptionsUi::default();
        assert!(options.panel(window, 0).is_none());
        options.toggle();
        let panel = options.panel(window, 0).unwrap();
        assert_eq!(panel.center(), window.center());
        assert!(options.panel(window, 1).unwrap().height() > panel.height());
    }
}
