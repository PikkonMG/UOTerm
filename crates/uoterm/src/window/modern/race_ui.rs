//! The race change panel of the Modern style (`0xBF` `0x2A`): the hair and
//! beard styles in drop-down lists at the left, the figure of the new looks
//! in the middle, the skin, hair and beard colors at the right with the
//! palette of the color the player picks, and the buttons that send the
//! looks or keep the old ones. The picks follow `model::race_change`, as the
//! classic gump does.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::model::race_change::{paints, palette, palette_columns, style_lists, Paint};
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::hue_ui;
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};
use uoterm_view::ui::lists::{race_change_words, race_preview_look};
use uoterm_view::ui::places::FOOT_ROW;
use uoterm_view::ui::shard_asks::{
    race_first_place, RacePanel, HINT_COLOR, RACE_ID, WORDS_CHANGE, WORDS_KEEP,
};
use uoterm_world::RaceChange;

const SIDE_WIDTH: f32 = 180.0;
const LABEL_ROW: f32 = 20.0;
const CONTROL_ROW: f32 = 28.0;
const PART_GAP: f32 = 12.0;
const SWATCH_EDGE: f32 = 1.0;
const MARK_WIDTH: f32 = 2.0;

/// The panel, the picks of the player, and the palette that is open.
#[derive(Default)]
pub struct RaceUi {
    race: RacePanel,
}

impl RaceUi {
    /// Draws the panel while the shard waits for new looks. Gives its place.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let Some(change) = frame.race_change else {
            self.race.stop();
            return None;
        };
        self.race.follow(change);
        let live = frame.human_control;
        let title = race_change_words(change);
        let spec = PanelSpec {
            id: RACE_ID,
            title: &title,
            default: bridge::rect(race_first_place(bridge::area(rect))),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &title);
        let columns =
            Rect::from_min_max(body.min, Pos2::new(body.right(), body.bottom() - FOOT_ROW));
        let left = Rect::from_min_size(columns.min, Vec2::new(SIDE_WIDTH, columns.height()));
        let right = Rect::from_min_max(
            Pos2::new(columns.right() - SIDE_WIDTH, columns.top()),
            columns.max,
        );
        let middle = Rect::from_min_max(
            Pos2::new(left.right() + PART_GAP, columns.top()),
            Pos2::new(right.left() - PART_GAP, columns.bottom()),
        );
        ui.add_enabled_ui(live, |ui| self.styles(ui, left, change));
        self.preview(ui, middle, change, tools);
        self.colors(ui, right, change, tools, live);
        let mut keep = false;
        if live {
            let foot = Pos2::new(body.left(), columns.bottom() + theme::ROW_GAP);
            let (change_area, changed) = theme::button(ui, foot, WORDS_CHANGE, theme::GOAL);
            let at = Pos2::new(change_area.right() + theme::ROW_GAP, foot.y);
            let (_, kept) = theme::button(ui, at, WORDS_KEEP, theme::TEXT_DIM);
            keep = kept;
            if changed {
                tools
                    .hand
                    .act(Act::RaceChange(Some(self.race.picks.looks(change))));
            }
        }
        let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if live && (keep || closed) {
            tools.hand.act(Act::RaceChange(None));
        }
        Some(panel)
    }

    /// The drop-down lists of the hair and the beard styles.
    fn styles(&mut self, ui: &mut egui::Ui, area: Rect, change: RaceChange) {
        let mut top = area.top();
        for (part, (_, words), styles) in style_lists(change) {
            ui.painter().text(
                Pos2::new(area.left(), top),
                Align2::LEFT_TOP,
                words,
                text_font(theme::SIZE_BODY),
                theme::TEXT_DIM,
            );
            let list = Rect::from_min_size(
                Pos2::new(area.left(), top + LABEL_ROW),
                Vec2::new(area.width(), CONTROL_ROW),
            );
            let mut place = *self.race.picks.style_place(part);
            let shown = styles.get(place).map_or("", |style| style.words);
            ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| {
                egui::ComboBox::from_id_salt(("race-style", part as u8))
                    .selected_text(shown)
                    .width(list.width())
                    .show_ui(ui, |ui| {
                        for (at, style) in styles.iter().enumerate() {
                            ui.selectable_value(&mut place, at, style.words);
                        }
                    });
            });
            self.race.pick_style(part, place);
            top = list.bottom() + PART_GAP;
        }
    }

    /// The figure with the new looks.
    fn preview(&self, ui: &egui::Ui, area: Rect, change: RaceChange, tools: &mut Tools<'_>) {
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let look = race_preview_look(change, &self.race.picks);
        if let Some((texture, sprite)) = tools.scene.doll_picture(&look) {
            let shown = theme::fit(area, sprite.width, sprite.height);
            ui.painter()
                .image(texture, shown, bridge::rect(sprite.uv), Color32::WHITE);
        }
    }

    /// The color of each part, and the palette of the one being picked.
    fn colors(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        change: RaceChange,
        tools: &Tools<'_>,
        live: bool,
    ) {
        let mut top = area.top();
        for (paint, (_, words)) in paints(change) {
            ui.painter().text(
                Pos2::new(area.left(), top),
                Align2::LEFT_TOP,
                words,
                text_font(theme::SIZE_BODY),
                theme::TEXT_DIM,
            );
            let swatch = Rect::from_min_size(
                Pos2::new(area.left(), top + LABEL_ROW),
                Vec2::new(area.width(), CONTROL_ROW),
            );
            let response =
                ui.interact(swatch, Id::new(("race-color", paint as u8)), Sense::click());
            hue_ui::swatch(ui, swatch, self.race.picks.hue(change, paint), tools);
            if self.race.picking == Some(paint) {
                ui.painter().rect_stroke(
                    swatch,
                    CornerRadius::same(CELL_RADIUS),
                    Stroke::new(SWATCH_EDGE, theme::GOAL),
                    egui::StrokeKind::Inside,
                );
            }
            if response.hovered() && live {
                super::super::tips::label(ui, HINT_COLOR, "");
            }
            if response.clicked() && live {
                self.race.click_paint(paint);
            }
            top = swatch.bottom() + PART_GAP;
        }
        if let Some(paint) = self.race.picking.filter(|_| live) {
            let room = Rect::from_min_max(Pos2::new(area.left(), top), area.max);
            self.palette(ui, room, change, paint, tools);
        }
    }

    /// The hues of one color. A click picks one and shuts the palette.
    fn palette(
        &mut self,
        ui: &egui::Ui,
        room: Rect,
        change: RaceChange,
        paint: Paint,
        tools: &Tools<'_>,
    ) {
        let hues = palette(change, paint);
        let columns = palette_columns(hues.len());
        let rows = hues.len().div_ceil(columns).max(1);
        let cell = Vec2::new(room.width() / columns as f32, room.height() / rows as f32);
        let place = *self.race.picks.hue_place(paint);
        let mut picked = None;
        for (at, hue) in hues.iter().enumerate() {
            let area = Rect::from_min_size(
                room.left_top()
                    + Vec2::new(
                        (at % columns) as f32 * cell.x,
                        (at / columns) as f32 * cell.y,
                    ),
                cell,
            );
            ui.painter()
                .rect_filled(area, CornerRadius::ZERO, tools.scene.words_color(*hue));
            if at == place {
                ui.painter().rect_stroke(
                    area,
                    CornerRadius::ZERO,
                    Stroke::new(MARK_WIDTH, theme::TEXT),
                    egui::StrokeKind::Inside,
                );
            }
            let key = Id::new(("race-hue", paint as u8, at));
            if ui.interact(area, key, Sense::click()).clicked() {
                picked = Some(at);
            }
        }
        if let Some(at) = picked {
            self.race.pick_hue(paint, at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::draw_frames;
    use super::*;
    use uoterm_world::Race;

    const HUMAN_MAN: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };

    #[test]
    fn the_panel_shows_while_the_shard_waits_and_a_new_request_starts_over() {
        let frame = WatchFrame {
            human_control: true,
            race_change: Some(HUMAN_MAN),
            ..WatchFrame::default()
        };
        let mut race = RaceUi::default();
        race.race.follow(HUMAN_MAN);
        race.race.picks.hair = 2;
        let mut profile = Profile::default();
        let mut shown = None;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = race.draw(ui, rect, &frame, tools, profile);
        });
        assert!(shown.is_some());
        assert_eq!(race.race.picks.hair, 2, "the same request keeps the picks");
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = race.draw(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(shown.is_none());
    }
}
