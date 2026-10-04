//! The world map. Near the character it shows the land from far above,
//! turned as the play field is turned; in the whole-world view it shows the
//! facet north up, at the zoom step the World Map page keeps. It carries the
//! marker and zone files of the player, the named places of the session,
//! the party and the guild, the mobiles and the houses, the coordinates in
//! tiles and by sextant, and a go-to box. A click walks the character there
//! while the human has control, or answers a target of a place when the
//! World Map page lets it, and Ctrl+click opens the box that marks the
//! tile. Its buttons open the markers manager, mark where the character
//! stands, read the marker and zone files again, and draw the map again.
//! The human moves it by its title and sizes it by its corner, and the
//! profile keeps where he left it. The drawing of the land and the marks is
//! `map_view`'s, shared with the Classic world map gump.

use super::boxes_ui::{Tools, CELL_RADIUS};
use super::bridge;
use super::map_view::{self, MapFilesCache, MapPictures, MarkStyle, Marks, MODERN_LOOK, ZOOM_MIN};
use super::model::world_map;
use super::modern::frame::{self as panel_frame, FrameEvent, PanelSpec};
use super::modern::{MarkersAsk, MarkersUi};
use super::scene::Scene;
use super::settings::Profile;
use super::theme::{self, number_font, text_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Painter, Pos2, Rect, Sense, Vec2};
use uoterm_view::map_lay::Lay;
use uoterm_view::ui::map_panel::MapLook;
use uoterm_view::ui::map_panel::{
    dragged_look, field_click, field_hints, field_of, goto_place, map_first_place, near_lay,
    near_zoom, view_words, whole_turns, world_lay, world_zoom, FieldClick, MapButton, HINT_GOTO,
    MAP_BUTTONS, MAP_ID, NOTE_SECONDS, TOOL_GAP, TOOL_ROW, WORDS_BUILDING, WORDS_GO,
    WORDS_NO_FILES, WORDS_RELOADED, WORDS_TITLE, WORDS_WALK,
};

const GOTO_WIDTH: f32 = 150.0;
const BUTTON_WIDTH: f32 = 52.0;
const WIDE_BUTTON_WIDTH: f32 = 78.0;

pub struct MapUi {
    /// Open or not, and where the whole-world view looks.
    look: MapLook,
    pictures: MapPictures,
    files: MapFilesCache,
    zoom: f32,
    /// The turns of the wheel over the whole-world view that are not yet a
    /// whole zoom step.
    wheel: f32,
    goto: String,
    note: Option<(String, f64)>,
    markers: MarkersUi,
}

/// A small health bar in the colors of the panels.
fn modern_bar(painter: &Painter, track: Rect, share: f32) {
    theme::bar(painter, track, share, theme::HITS);
}

/// The marks of a map in the colors of the Modern style.
pub fn mark_look() -> MarkStyle {
    MarkStyle {
        look: MODERN_LOOK,
        font: text_font(theme::SIZE_SMALL),
        shadowed: false,
        square_dots: false,
        health_bar: modern_bar,
    }
}

/// One button of a tool row, from the right. Gives its place, and moves
/// `right` past it.
fn from_right(row: Rect, right: &mut f32, width: f32) -> Rect {
    let area = Rect::from_min_size(
        Pos2::new(*right - width, row.top()),
        Vec2::new(width, row.height()),
    );
    *right = area.left() - TOOL_GAP;
    area
}

impl MapUi {
    pub fn starting(open: bool) -> Self {
        Self {
            look: MapLook {
                open,
                looking_at: None,
            },
            pictures: MapPictures::default(),
            files: MapFilesCache::default(),
            zoom: ZOOM_MIN,
            wheel: 0.0,
            goto: String::new(),
            note: None,
            markers: MarkersUi::default(),
        }
    }

    pub fn toggle(&mut self) {
        self.look.open = !self.look.open;
    }

    pub fn is_open(&self) -> bool {
        self.look.open
    }

    /// Closes the map, the markers manager and the marker box.
    pub fn close(&mut self) {
        self.look.open = false;
        self.markers.close();
    }

    /// Draws the map when it is open, and the markers manager and the
    /// marker box when they show. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        if self.look.open {
            covered.push(self.map_panel(ui, rect, frame, tools, profile));
        }
        let (marker_panels, asks) = self.markers.draw(ui, rect, tools, profile);
        covered.extend(marker_panels);
        for ask in asks {
            match ask {
                MarkersAsk::GoTo(x, y) => self.look_at(x, y, tools, profile),
                MarkersAsk::Changed => self.files.reload(),
            }
        }
        covered
    }

    /// Opens the whole-world view on a place.
    fn look_at(&mut self, x: u16, y: u16, tools: &Tools<'_>, profile: &mut Profile) {
        if self.look.look_at((x, y), &mut profile.world_map) {
            tools.keep_profile(profile);
        }
    }

    fn map_panel(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let (default, least) = map_first_place(bridge::area(rect));
        let spec = PanelSpec {
            id: MAP_ID,
            title: WORDS_TITLE,
            default: bridge::rect(default),
            min_size: Some(bridge::vec2(least)),
            closable: true,
        };
        let panel = panel_frame::place(rect, &spec, profile);
        let body = panel_frame::draw(ui.painter(), panel, WORDS_TITLE);
        let place_row = Rect::from_min_size(body.min, Vec2::new(body.width(), TOOL_ROW));
        let marker_row = place_row.translate(Vec2::new(0.0, TOOL_ROW + TOOL_GAP));
        let field = bridge::rect(field_of(bridge::area(body)));
        self.place_row(ui, place_row, frame, tools, profile);
        self.marker_row(ui, marker_row, frame, tools);
        self.field(ui, field, frame, tools, profile);
        if panel_frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.look.open = false;
        }
        panel
    }

    /// The go-to box, the buttons, and where the character stands.
    fn place_row(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let whole = profile.world_map.whole_world;
        let mut right = row.right();
        let view = from_right(row, &mut right, BUTTON_WIDTH);
        let walk = from_right(row, &mut right, BUTTON_WIDTH);
        let go = from_right(row, &mut right, BUTTON_WIDTH);
        let field = from_right(row, &mut right, GOTO_WIDTH);
        if theme::segment_keyed(
            ui,
            view,
            Id::new("map-view"),
            view_words(whole),
            theme::TEXT,
        ) {
            profile.world_map.whole_world = !whole;
            self.look.looking_at = None;
            tools.keep_profile(profile);
        }
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.goto)
                .id(Id::new("map-goto"))
                .frame(false)
                .hint_text(HINT_GOTO)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        let entered = typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let looked =
            theme::segment_keyed(ui, go, Id::new("map-go"), WORDS_GO, theme::TEXT) || entered;
        let walked = frame.human_control
            && theme::segment_keyed(ui, walk, Id::new("map-walk"), WORDS_WALK, theme::GOAL);
        if looked || walked {
            match goto_place(&self.goto) {
                Ok((x, y)) => {
                    self.look_at(x, y, tools, profile);
                    if walked {
                        tools.hand.act(uoterm_view::act::Act::WalkTo { x, y });
                    }
                }
                Err(words) => self.note = Some((words.to_string(), tools.time)),
            }
        }
        if profile.world_map.show_coordinates {
            ui.painter().text(
                Pos2::new(row.left(), row.center().y),
                Align2::LEFT_CENTER,
                map_view::place_words(profile, frame.map, frame.x, frame.y),
                number_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            );
        }
    }

    /// The buttons of the markers and of the map files.
    fn marker_row(&mut self, ui: &egui::Ui, row: Rect, frame: &WatchFrame, tools: &Tools<'_>) {
        let mut right = row.right();
        let mut pressed = None;
        for (at, button) in MAP_BUTTONS.into_iter().enumerate() {
            let area = from_right(row, &mut right, WIDE_BUTTON_WIDTH);
            let key = Id::new(("map-marker-button", at));
            if theme::segment_keyed(ui, area, key, button.words(), theme::TEXT) {
                pressed = Some(button);
            }
            if ui.rect_contains_pointer(area) {
                super::tips::label(ui, button.hint(), "");
            }
        }
        match pressed {
            Some(MapButton::Redraw) => self.pictures = MapPictures::default(),
            Some(MapButton::Reload) => {
                self.files.reload();
                self.note = Some((WORDS_RELOADED.to_string(), tools.time));
            }
            Some(MapButton::MarkMe) => self.markers.add(&world_map::marker_on_player(frame)),
            Some(MapButton::Markers) => self.markers.toggle(),
            None => {}
        }
    }

    fn field(
        &mut self,
        ui: &egui::Ui,
        field: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let whole = profile.world_map.whole_world;
        let response = ui.interact(field, Id::new("world-map"), Sense::click_and_drag());
        let notches = map_view::wheel_notches(ui, &response);
        let lay = if whole {
            self.wheel += notches;
            let turns = whole_turns(&mut self.wheel);
            if world_zoom(&mut profile.world_map, turns) {
                tools.keep_profile(profile);
            }
            self.world_view(ui, field, frame, tools.scene, profile, &response)
        } else {
            if notches != 0.0 {
                self.zoom = near_zoom(self.zoom, notches);
            }
            self.near_view(ui, field, frame, tools.scene)
        };
        let Some(lay) = lay else {
            ui.painter().text(
                field.center(),
                Align2::CENTER_CENTER,
                WORDS_NO_FILES,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
            return;
        };
        let painter = ui.painter().with_clip_rect(field);
        let session = map_view::session_markers(tools.readings, profile, frame.map);
        let marks = Marks {
            frame,
            map: frame.map,
            profile,
            files: self.files.get(profile),
            session: &session,
            looking_at: self.look.looking_at,
        };
        map_view::overlays(&painter, field, lay, &marks, &mark_look());
        if let Some((words, since)) = &self.note {
            if tools.time - since > NOTE_SECONDS {
                self.note = None;
            } else {
                painter.text(
                    field.left_top(),
                    Align2::LEFT_TOP,
                    words,
                    text_font(theme::SIZE_SMALL),
                    theme::WAITING,
                );
            }
        }
        let Some(mouse) = response.hover_pos() else {
            return;
        };
        let (x, y) = map_view::whole_tile(lay.tile(bridge::point(mouse)));
        if profile.world_map.show_mouse_coordinates {
            theme::shadowed_text(
                &painter,
                field.left_bottom(),
                Align2::LEFT_BOTTOM,
                &map_view::place_words(profile, frame.map, x, y),
                number_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
        }
        if let Some(label) = self.files.get(profile).zone_at(frame.map, x, y) {
            theme::shadowed_text(
                &painter,
                field.right_bottom(),
                Align2::RIGHT_BOTTOM,
                &label,
                text_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
        }
        let (hint, pan) = field_hints(frame, &profile.world_map);
        super::tips::label(ui, hint, pan);
        if !response.clicked() {
            return;
        }
        let ctrl = ui.input(|i| i.modifiers.command);
        let scene = &mut *tools.scene;
        match field_click(frame, &profile.world_map, (x, y), ctrl, || {
            scene.land_z(frame.map, x, y)
        }) {
            Some(FieldClick::Mark(marker)) => self.markers.add(&marker),
            Some(FieldClick::Act(act)) => tools.hand.act(act),
            None => {}
        }
    }

    /// The land near the character, turned. None without the client files.
    fn near_view(
        &mut self,
        ui: &egui::Ui,
        field: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
    ) -> Option<Lay> {
        if !self
            .pictures
            .make_near(ui.ctx(), scene, frame.map, (frame.x, frame.y))
        {
            return None;
        }
        let lay = near_lay(bridge::area(field), frame, self.zoom);
        self.pictures
            .draw_near(&ui.painter().with_clip_rect(field), lay);
        Some(lay)
    }

    /// The whole facet, north up, at the zoom step of the World Map page,
    /// and never smaller than the field. A drag moves the view when the
    /// page lets the view go free; else it follows the character.
    fn world_view(
        &mut self,
        ui: &egui::Ui,
        field: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        profile: &Profile,
        response: &egui::Response,
    ) -> Option<Lay> {
        if self.pictures.grow_world(ui.ctx(), scene, frame.map) {
            ui.ctx().request_repaint();
            ui.painter().text(
                field.center_top(),
                Align2::CENTER_TOP,
                WORDS_BUILDING,
                text_font(theme::SIZE_SMALL),
                theme::WAITING,
            );
        }
        let lay = world_lay(
            bridge::area(field),
            frame,
            &profile.world_map,
            self.look.looking_at,
        );
        if response.dragged() && profile.world_map.free_view {
            self.look.looking_at = dragged_look(lay, bridge::vector(response.drag_delta()));
        }
        self.pictures
            .draw_world(&ui.painter().with_clip_rect(field), lay)
            .then_some(lay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_the_map_closes_its_marker_boxes() {
        let mut map = MapUi::starting(false);
        map.markers
            .add(&world_map::marker_on_player(&WatchFrame::default()));
        map.close();
        assert!(!map.is_open());
    }
}
