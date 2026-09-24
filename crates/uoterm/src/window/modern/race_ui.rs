//! The race change panel of the Modern style (`0xBF` `0x2A`): the hair and
//! beard styles in drop-down lists at the left, the figure of the new looks
//! in the middle, the skin, hair and beard colors at the right with the
//! palette of the color the player picks, and the buttons that send the
//! looks or keep the old ones. The picks follow `model::race_change`, as the
//! classic gump does.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::model::race_change::{
    doll_body, paints, palette, palette_columns, style_lists, Paint, RacePicks,
};
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::hue_ui;
use super::layout::{self, Spot};
use crate::view::{WatchEquip, WatchFrame, WatchLook};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};
use uoterm_protocol::types::{LAYER_BEARD, LAYER_HAIR};
use uoterm_world::{Race, RaceChange};

pub const RACE_ID: &str = "modern:race_change";
const WIDTH: f32 = 600.0;
const HEIGHT: f32 = 420.0;
const SIDE_WIDTH: f32 = 180.0;
const LABEL_ROW: f32 = 20.0;
const CONTROL_ROW: f32 = 28.0;
const PART_GAP: f32 = 12.0;
const FOOT_ROW: f32 = 40.0;
const SWATCH_EDGE: f32 = 1.0;
const MARK_WIDTH: f32 = 2.0;
/// A worn hair or beard of the preview has no serial.
const NO_SERIAL: u32 = 0;
const NO_STYLE: u16 = 0;

const WORDS_TITLE: &str = "Race change";
const WORDS_HUMAN: &str = "Human";
const WORDS_ELF: &str = "Elf";
const WORDS_GARGOYLE: &str = "Gargoyle";
const WORDS_MAN: &str = "man";
const WORDS_WOMAN: &str = "woman";
const WORDS_CHANGE: &str = "Change";
const WORDS_KEEP: &str = "Keep my looks";
const HINT_COLOR: &str = "Click: pick from the palette.";

/// The words of the race and the sex the shard asks the player to be.
fn change_words(change: RaceChange) -> String {
    let race = match change.race {
        Race::Human => WORDS_HUMAN,
        Race::Elf => WORDS_ELF,
        Race::Gargoyle => WORDS_GARGOYLE,
    };
    let sex = if change.female {
        WORDS_WOMAN
    } else {
        WORDS_MAN
    };
    format!("{WORDS_TITLE}: {race} {sex}")
}

/// The look of the figure with the new looks.
fn preview_look(change: RaceChange, picks: &RacePicks) -> WatchLook {
    let looks = picks.looks(change);
    let worn = [
        (looks.hair, looks.hair_hue, LAYER_HAIR),
        (looks.beard, looks.beard_hue, LAYER_BEARD),
    ];
    WatchLook {
        body: doll_body(change),
        hue: looks.skin_hue,
        equipment: worn
            .into_iter()
            .filter(|(graphic, ..)| *graphic != NO_STYLE)
            .map(|(graphic, hue, layer)| WatchEquip {
                serial: NO_SERIAL,
                graphic,
                layer,
                hue,
            })
            .collect(),
        ..WatchLook::default()
    }
}

/// The panel, the picks of the player, and the palette that is open.
#[derive(Default)]
pub struct RaceUi {
    picks: RacePicks,
    picking: Option<Paint>,
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
            self.picking = None;
            return None;
        };
        if self.picks.change != Some(change) {
            self.picking = None;
        }
        self.picks.follow(change);
        let live = frame.human_control;
        let title = change_words(change);
        let spec = PanelSpec {
            id: RACE_ID,
            title: &title,
            default: layout::first_place(rect, Spot::Middle(0), Vec2::new(WIDTH, HEIGHT)),
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
        self.preview(ui, middle, change, frame, tools);
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
                    .act(Act::RaceChange(Some(self.picks.looks(change))));
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
            let place = self.picks.style_place(part);
            let shown = styles.get(*place).map_or("", |style| style.words);
            ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| {
                egui::ComboBox::from_id_salt(("race-style", part as u8))
                    .selected_text(shown)
                    .width(list.width())
                    .show_ui(ui, |ui| {
                        for (at, style) in styles.iter().enumerate() {
                            ui.selectable_value(place, at, style.words);
                        }
                    });
            });
            top = list.bottom() + PART_GAP;
        }
    }

    /// The figure with the new looks.
    fn preview(
        &self,
        ui: &egui::Ui,
        area: Rect,
        change: RaceChange,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let look = preview_look(change, &self.picks);
        if let Some((texture, sprite)) = tools.scene.doll_picture(frame.map, &look) {
            let shown = theme::fit(area, sprite.width, sprite.height);
            ui.painter()
                .image(texture, shown, sprite.uv, Color32::WHITE);
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
            hue_ui::swatch(ui, swatch, self.picks.hue(change, paint), tools);
            if self.picking == Some(paint) {
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
                self.picking = (self.picking != Some(paint)).then_some(paint);
            }
            top = swatch.bottom() + PART_GAP;
        }
        if let Some(paint) = self.picking.filter(|_| live) {
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
        let place = self.picks.hue_place(paint);
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
            if at == *place {
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
            *place = at;
            self.picking = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::draw_frames;
    use super::*;
    use uoterm_world::BODY_HUMAN_MALE;

    const HUMAN_MAN: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };

    #[test]
    fn the_preview_wears_the_picked_hair_and_beard_on_the_new_body() {
        let mut picks = RacePicks::default();
        picks.follow(HUMAN_MAN);
        assert!(preview_look(HUMAN_MAN, &picks).equipment.is_empty(), "bald");
        picks.hair = 1;
        picks.beard = 1;
        let look = preview_look(HUMAN_MAN, &picks);
        assert_eq!(look.body, BODY_HUMAN_MALE);
        let layers: Vec<u8> = look.equipment.iter().map(|worn| worn.layer).collect();
        assert_eq!(layers, vec![LAYER_HAIR, LAYER_BEARD]);
        assert_eq!(change_words(HUMAN_MAN), "Race change: Human man");
    }

    #[test]
    fn the_panel_shows_while_the_shard_waits_and_a_new_request_starts_over() {
        let frame = WatchFrame {
            human_control: true,
            race_change: Some(HUMAN_MAN),
            ..WatchFrame::default()
        };
        let mut race = RaceUi::default();
        race.picks.follow(HUMAN_MAN);
        race.picks.hair = 2;
        let mut profile = Profile::default();
        let mut shown = None;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = race.draw(ui, rect, &frame, tools, profile);
        });
        assert!(shown.is_some());
        assert_eq!(race.picks.hair, 2, "the same request keeps the picks");
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = race.draw(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(shown.is_none());
    }
}
