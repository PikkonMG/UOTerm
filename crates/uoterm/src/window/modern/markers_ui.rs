//! The markers of the world map in the Modern style: the markers manager,
//! which lists the markers of each marker file with a search and lets the
//! player change, remove and go to the markers of his own file, and the
//! box that adds or changes one marker with its name, place, icon and
//! color. The files are `model::world_map`'s, as the classic gumps have
//! them.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::world_map::{
    self, Marker, MarkerFields, MarkerFile, MARKER_COLORS, USER_MARKERS,
};
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use super::rows::Rows;
use eframe::egui::{self, Align2, CornerRadius, Id, Pos2, Rect, Vec2};

const MANAGER_ID: &str = "modern:markers";
const BOX_ID: &str = "modern:marker_box";
const MANAGER_SIZE: Vec2 = Vec2::new(480.0, 420.0);
const MANAGER_LEAST: Vec2 = Vec2::new(320.0, 240.0);
const BOX_SIZE: Vec2 = Vec2::new(320.0, 270.0);
const ROW: f32 = 26.0;
const GAP: f32 = 6.0;
const ROW_BUTTON_WIDTH: f32 = 58.0;
const FIELD_LABEL_WIDTH: f32 = 50.0;
const WORDS_MANAGER: &str = "Markers";
const WORDS_ADD: &str = "Add marker";
const WORDS_EDIT_MARKER: &str = "Edit marker";
const WORDS_EDIT: &str = "Edit";
const WORDS_REMOVE: &str = "Remove";
const WORDS_GO: &str = "Go";
const WORDS_CREATE: &str = "Create";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_NO_FILES: &str = "No marker files. Ctrl+click the map to add a marker.";
const WORDS_NONE_FOUND: &str = "No marker holds these words.";
const WORDS_BAD_FIELDS: &str = "Give a name, and x and y inside the facet.";
const WORDS_X: &str = "X";
const WORDS_Y: &str = "Y";
const WORDS_NAME: &str = "Name";
const WORDS_ICON: &str = "Icon";
const WORDS_COLOR: &str = "Color";
const HINT_SEARCH: &str = "search the markers";
const HINT_ICON: &str = "no icon";

/// What the markers ask of the world map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkersAsk {
    /// Show this place.
    GoTo(u16, u16),
    /// The marker files changed: read them again.
    Changed,
}

/// The box that adds a marker, or changes the one at `editing` in the
/// player's own file.
struct MarkerBox {
    editing: Option<usize>,
    fields: MarkerFields,
    error: Option<String>,
}

#[derive(Default)]
pub struct MarkersUi {
    open: bool,
    files: Vec<MarkerFile>,
    loaded: bool,
    /// The file whose markers show.
    file: usize,
    search: String,
    scroll: f32,
    error: Option<String>,
    marker_box: Option<MarkerBox>,
}

impl MarkersUi {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.loaded = false;
    }

    /// Closes the manager and the marker box.
    pub fn close(&mut self) {
        self.open = false;
        self.marker_box = None;
    }

    /// Opens the box that adds a marker, filled from `marker`.
    pub fn add(&mut self, marker: &Marker) {
        self.marker_box = Some(MarkerBox {
            editing: None,
            fields: MarkerFields::of(marker),
            error: None,
        });
    }

    /// Draws the manager and the marker box when they show. Gives their
    /// places and what they ask of the map.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> (Vec<Rect>, Vec<MarkersAsk>) {
        let mut covered = Vec::new();
        let mut asks = Vec::new();
        if self.open {
            if !self.loaded {
                self.files = world_map::load_markers(&world_map::map_dir(), &[]);
                self.file = self.file.min(self.files.len().saturating_sub(1));
                self.loaded = true;
            }
            covered.push(self.manager(ui, rect, tools, profile, &mut asks));
        }
        if self.marker_box.is_some() {
            covered.push(self.marker_box(ui, rect, tools, profile, &mut asks));
        }
        if asks.contains(&MarkersAsk::Changed) {
            self.loaded = false;
        }
        (covered, asks)
    }

    fn manager(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        tools: &Tools<'_>,
        profile: &mut Profile,
        asks: &mut Vec<MarkersAsk>,
    ) -> Rect {
        let spec = PanelSpec {
            id: MANAGER_ID,
            title: WORDS_MANAGER,
            default: layout::first_place(rect, Spot::Middle(0), MANAGER_SIZE),
            min_size: Some(MANAGER_LEAST),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_MANAGER);
        let tabs = Rect::from_min_size(body.min, Vec2::new(body.width(), ROW));
        if !self.files.is_empty() {
            let width =
                (tabs.width() - GAP * (self.files.len() - 1) as f32) / self.files.len() as f32;
            for (at, file) in self.files.iter().enumerate() {
                let area = Rect::from_min_size(
                    tabs.min + Vec2::new(at as f32 * (width + GAP), 0.0),
                    Vec2::new(width, ROW),
                );
                let color = if at == self.file {
                    theme::GOAL
                } else {
                    theme::TEXT_DIM
                };
                if theme::segment_keyed(ui, area, Id::new(("marker-file", at)), &file.name, color) {
                    self.file = at;
                    self.scroll = 0.0;
                }
            }
        }
        let search = Rect::from_min_size(
            Pos2::new(body.left(), tabs.bottom() + GAP),
            Vec2::new(body.width(), ROW),
        );
        ui.painter()
            .rect_filled(search, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            search,
            egui::TextEdit::singleline(&mut self.search)
                .id(Id::new("marker-search"))
                .frame(false)
                .hint_text(HINT_SEARCH)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        if typed.changed() {
            self.scroll = 0.0;
        }
        let list = Rect::from_min_max(Pos2::new(body.left(), search.bottom() + GAP), body.max);
        self.rows(ui, list, asks);
        if let Some(error) = &self.error {
            ui.painter().text(
                body.left_bottom(),
                Align2::LEFT_BOTTOM,
                error,
                text_font(theme::SIZE_SMALL),
                theme::ALARM,
            );
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.open = false;
        }
        panel
    }

    /// The markers of the open file that hold the search words.
    fn rows(&mut self, ui: &mut egui::Ui, list: Rect, asks: &mut Vec<MarkersAsk>) {
        let mut rows = Rows::new(ui, list, MANAGER_ID, self.scroll);
        let Some(file) = self.files.get(self.file) else {
            rows.words(WORDS_NO_FILES, theme::TEXT_FAINT);
            self.scroll = rows.finish();
            return;
        };
        let editable = file.name == USER_MARKERS;
        let shown = world_map::found(&file.markers, &self.search);
        if shown.is_empty() {
            rows.words(WORDS_NONE_FOUND, theme::TEXT_FAINT);
        }
        let buttons: &[(&str, egui::Color32)] = if editable {
            &[
                (WORDS_EDIT, theme::TEXT),
                (WORDS_REMOVE, theme::ALARM),
                (WORDS_GO, theme::GOAL),
            ]
        } else {
            &[(WORDS_GO, theme::GOAL)]
        };
        let mut pressed = None;
        for (at, marker) in &shown {
            let words = format!(
                "{}  {}, {}  {}",
                marker.name, marker.x, marker.y, marker.color
            );
            if let Some(button) = rows.labeled("marker", *at, &words, buttons, ROW_BUTTON_WIDTH) {
                pressed = Some((*at, (*marker).clone(), button));
            }
        }
        self.scroll = rows.finish();
        let Some((at, marker, button)) = pressed else {
            return;
        };
        let go = buttons.len() - 1;
        match button {
            button if button == go => asks.push(MarkersAsk::GoTo(marker.x, marker.y)),
            0 => {
                self.marker_box = Some(MarkerBox {
                    editing: Some(at),
                    fields: MarkerFields::of(&marker),
                    error: None,
                });
            }
            _ => match world_map::remove_user_marker(&world_map::map_dir(), at) {
                Ok(()) => asks.push(MarkersAsk::Changed),
                Err(error) => self.error = Some(error.to_string()),
            },
        }
    }

    fn marker_box(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        tools: &Tools<'_>,
        profile: &mut Profile,
        asks: &mut Vec<MarkersAsk>,
    ) -> Rect {
        let Some(marker_box) = self.marker_box.as_mut() else {
            return Rect::NOTHING;
        };
        let title = if marker_box.editing.is_some() {
            WORDS_EDIT_MARKER
        } else {
            WORDS_ADD
        };
        let spec = PanelSpec {
            id: BOX_ID,
            title,
            default: layout::first_place(rect, Spot::Middle(0), BOX_SIZE),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, title);
        let (kept, cancelled) = ui
            .scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                box_fields(ui, marker_box)
            })
            .inner;
        let mut closed = cancelled;
        if kept {
            match marker_box.fields.marker() {
                None => marker_box.error = Some(WORDS_BAD_FIELDS.to_string()),
                Some(marker) => {
                    let dir = world_map::map_dir();
                    match world_map::keep_user_marker(&dir, marker_box.editing, marker) {
                        Ok(()) => {
                            asks.push(MarkersAsk::Changed);
                            closed = true;
                        }
                        Err(error) => marker_box.error = Some(error.to_string()),
                    }
                }
            }
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            closed = true;
        }
        if closed {
            self.marker_box = None;
        }
        panel
    }
}

/// The fields of the marker box and its buttons. Gives whether the marker
/// is to be kept, and whether the box was cancelled.
fn box_fields(ui: &mut egui::Ui, marker_box: &mut MarkerBox) -> (bool, bool) {
    let fields = &mut marker_box.fields;
    let mut entered = false;
    egui::Grid::new("marker-box-fields")
        .num_columns(2)
        .spacing([GAP, GAP])
        .show(ui, |ui| {
            for (words, value, hint) in [
                (WORDS_X, &mut fields.x, ""),
                (WORDS_Y, &mut fields.y, ""),
                (WORDS_NAME, &mut fields.name, ""),
                (WORDS_ICON, &mut fields.icon, HINT_ICON),
            ] {
                ui.add_sized([FIELD_LABEL_WIDTH, ROW], egui::Label::new(words));
                let typed = ui.add(egui::TextEdit::singleline(value).hint_text(hint));
                entered |= typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.end_row();
            }
            ui.add_sized([FIELD_LABEL_WIDTH, ROW], egui::Label::new(WORDS_COLOR));
            egui::ComboBox::from_id_salt("marker-box-color")
                .selected_text(MARKER_COLORS[fields.color.min(MARKER_COLORS.len() - 1)])
                .show_ui(ui, |ui| {
                    for (at, color) in MARKER_COLORS.iter().enumerate() {
                        ui.selectable_value(&mut fields.color, at, *color);
                    }
                });
            ui.end_row();
        });
    if let Some(error) = &marker_box.error {
        ui.colored_label(theme::ALARM, error);
    }
    let words = if marker_box.editing.is_some() {
        WORDS_EDIT
    } else {
        WORDS_CREATE
    };
    ui.horizontal(|ui| {
        let kept = ui.button(words).clicked() || entered;
        let cancelled = ui.button(WORDS_CANCEL).clicked();
        (kept, cancelled)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;

    #[test]
    fn adding_opens_the_box_and_closing_shuts_both() {
        let mut markers = MarkersUi::default();
        markers.add(&world_map::marker_on_player(&WatchFrame::default()));
        let fields = &markers.marker_box.as_ref().unwrap().fields;
        assert_eq!(fields.name, world_map::DEFAULT_MARKER_NAME);
        markers.toggle();
        assert!(markers.open);
        markers.close();
        assert!(!markers.open && markers.marker_box.is_none());
    }
}
