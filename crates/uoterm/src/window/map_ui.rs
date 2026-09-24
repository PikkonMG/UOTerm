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
use super::control::Act;
use super::map_view::{self, Lay, MapFilesCache, MapPictures, MarkLook, Marks, ZOOM_MIN};
use super::model::world_map::{self, Marker, NEW_MARKER_COLOR};
use super::modern::frame::{self as panel_frame, FrameEvent, PanelSpec, TITLE_ROW};
use super::modern::layout::{self, Spot};
use super::modern::{MarkersAsk, MarkersUi};
use super::scene::Scene;
use super::settings::Profile;
use super::theme::{self, number_font, text_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Painter, Pos2, Rect, Sense, Vec2};

/// The smallest the map field is drawn, whatever the size of the window.
const PANEL_SIDE: f32 = 470.0;
/// On a large screen the map grows to this share of the shorter side of the
/// window, so a person who plays at a high resolution can still read it.
const PANEL_SHARE: f32 = 0.7;
/// The smallest the human can make the map field.
const FIELD_SIDE_SMALLEST: f32 = 200.0;
/// The two rows of tools under the title: the coordinates with the go-to
/// box, and the marker buttons.
const TOOL_ROW: f32 = 26.0;
const TOOL_ROWS: f32 = 2.0;
const TOOL_GAP: f32 = 6.0;
const GOTO_WIDTH: f32 = 150.0;
const BUTTON_WIDTH: f32 = 52.0;
const WIDE_BUTTON_WIDTH: f32 = 78.0;
/// Above the least zoom the land near the character comes closer and
/// shows fewer tiles.
const ZOOM_MAX: f32 = 8.0;
const HALF: f32 = 2.0;
const NOTE_SECONDS: f64 = 5.0;

const MAP_ID: &str = "modern:map";
const WORDS_TITLE: &str = "Map";
const WORDS_NO_FILES: &str = "The map needs the client files.";
const WORDS_GO: &str = "Look";
const WORDS_WALK: &str = "Walk";
const WORDS_WORLD: &str = "World";
const WORDS_NEAR: &str = "Near";
const WORDS_MARKERS: &str = "Markers";
const WORDS_MARK_ME: &str = "Mark me";
const WORDS_RELOAD: &str = "Reload";
const WORDS_RESET: &str = "Redraw";
const WORDS_BUILDING: &str = "Drawing the world...";
const WORDS_NO_PLACE: &str = "Give x y, or a sextant place.";
const WORDS_RELOADED: &str = "The marker and zone files were read again.";
const HINT_GOTO: &str = "x y or 12o 34'N, 56o 7'E";
const HINT_WALK: &str = "Click: walk there. Ctrl+click: mark it. Wheel: zoom.";
const HINT_TARGET: &str = "Click: target the ground there.";
const HINT_ZOOM: &str = "Wheel: zoom.";
const HINT_PAN: &str = "Drag: move the view.";
const HINT_MARKERS: &str = "List, find, change and go to the markers.";
const HINT_MARK_ME: &str = "Mark the place where the character stands.";
const HINT_RELOAD: &str = "Read the marker and zone files again.";
const HINT_RESET: &str = "Draw the land of the map again.";

pub struct MapUi {
    open: bool,
    pictures: MapPictures,
    files: MapFilesCache,
    zoom: f32,
    /// The turns of the wheel over the whole-world view that are not yet a
    /// whole zoom step.
    wheel: f32,
    /// The tile in the middle of the whole-world view, when the player
    /// moved the view or looked for a place.
    looking_at: Option<(u16, u16)>,
    goto: String,
    note: Option<(String, f64)>,
    markers: MarkersUi,
}

/// How wide one side of the map field is in a window of this size.
fn field_side(rect: Rect) -> f32 {
    let room = rect.height().min(rect.width()) * PANEL_SHARE - TITLE_ROW - theme::PANEL_PAD * 2.0;
    room.max(PANEL_SIDE)
}

/// The size of the panel round a field of this side.
fn panel_size(side: f32) -> Vec2 {
    Vec2::new(side, side + TITLE_ROW + (TOOL_ROW + TOOL_GAP) * TOOL_ROWS)
        + Vec2::splat(theme::PANEL_PAD * 2.0)
}

/// The whole turns of the wheel in `wheel`, taken out of it.
fn whole_turns(wheel: &mut f32) -> i32 {
    let turns = wheel.trunc();
    *wheel -= turns;
    turns as i32
}

/// A small health bar in the colors of the panels.
fn modern_bar(painter: &Painter, track: Rect, share: f32) {
    theme::bar(painter, track, share, theme::HITS);
}

/// The marks of a map in the colors of the Modern style.
pub fn mark_look() -> MarkLook {
    MarkLook {
        font: text_font(theme::SIZE_SMALL),
        shadowed: false,
        square_dots: false,
        marker: theme::WAITING,
        waypoint: theme::WAITING,
        multi: theme::FLAT_DOOR,
        party: theme::GOAL,
        guild: theme::MANA,
        goal: theme::GOAL,
        looking: theme::WAITING,
        me: theme::SELF_FIGURE,
        grid: theme::GLASS_EDGE,
        mobile: theme::notoriety_color,
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
            open,
            pictures: MapPictures::default(),
            files: MapFilesCache::default(),
            zoom: ZOOM_MIN,
            wheel: 0.0,
            looking_at: None,
            goto: String::new(),
            note: None,
            markers: MarkersUi::default(),
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Closes the map, the markers manager and the marker box.
    pub fn close(&mut self) {
        self.open = false;
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
        if self.open {
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
        self.open = true;
        self.looking_at = Some((x, y));
        if !profile.world_map.whole_world {
            profile.world_map.whole_world = true;
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
        let spec = PanelSpec {
            id: MAP_ID,
            title: WORDS_TITLE,
            default: layout::first_place(rect, Spot::Middle(0), panel_size(field_side(rect))),
            min_size: Some(panel_size(FIELD_SIDE_SMALLEST)),
            closable: true,
        };
        let panel = panel_frame::place(rect, &spec, profile);
        let body = panel_frame::draw(ui.painter(), panel, WORDS_TITLE);
        let place_row = Rect::from_min_size(body.min, Vec2::new(body.width(), TOOL_ROW));
        let marker_row = place_row.translate(Vec2::new(0.0, TOOL_ROW + TOOL_GAP));
        let field = Rect::from_min_max(
            Pos2::new(body.left(), marker_row.bottom() + TOOL_GAP),
            body.max,
        );
        self.place_row(ui, place_row, frame, tools, profile);
        self.marker_row(ui, marker_row, frame, tools);
        self.field(ui, field, frame, tools, profile);
        if panel_frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.open = false;
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
        let view_words = if whole { WORDS_NEAR } else { WORDS_WORLD };
        if theme::segment_keyed(ui, view, Id::new("map-view"), view_words, theme::TEXT) {
            profile.world_map.whole_world = !whole;
            self.looking_at = None;
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
            match world_map::parse_goto(&self.goto) {
                Some((x, y)) => {
                    self.look_at(x, y, tools, profile);
                    if walked {
                        tools.hand.act(Act::WalkTo { x, y });
                    }
                }
                None => self.note = Some((WORDS_NO_PLACE.to_string(), tools.time)),
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
        let buttons = [
            (WORDS_RESET, HINT_RESET),
            (WORDS_RELOAD, HINT_RELOAD),
            (WORDS_MARK_ME, HINT_MARK_ME),
            (WORDS_MARKERS, HINT_MARKERS),
        ];
        let mut pressed = None;
        for (at, (words, hint)) in buttons.into_iter().enumerate() {
            let area = from_right(row, &mut right, WIDE_BUTTON_WIDTH);
            let key = Id::new(("map-marker-button", at));
            if theme::segment_keyed(ui, area, key, words, theme::TEXT) {
                pressed = Some(words);
            }
            if ui.rect_contains_pointer(area) {
                super::tips::label(ui, hint, "");
            }
        }
        match pressed {
            Some(WORDS_RESET) => self.pictures = MapPictures::default(),
            Some(WORDS_RELOAD) => {
                self.files.reload();
                self.note = Some((WORDS_RELOADED.to_string(), tools.time));
            }
            Some(WORDS_MARK_ME) => self.markers.add(&world_map::marker_on_player(frame)),
            Some(WORDS_MARKERS) => self.markers.toggle(),
            _ => {}
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
            if turns != 0 {
                let options = &mut profile.world_map;
                options.zoom_step = world_map::zoom_step(options.zoom_step, turns);
                tools.keep_profile(profile);
            }
            self.world_view(ui, field, frame, tools.scene, profile, &response)
        } else {
            if notches != 0.0 {
                self.zoom = map_view::zoomed(self.zoom, notches, ZOOM_MAX);
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
            looking_at: self.looking_at,
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
        let (x, y) = map_view::whole_tile(lay.tile(mouse));
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
        let pan = if whole && profile.world_map.free_view {
            HINT_PAN
        } else {
            ""
        };
        if !frame.human_control {
            super::tips::label(ui, HINT_ZOOM, pan);
            return;
        }
        let targets = world_map::targets_ground(frame, &profile.world_map);
        super::tips::label(ui, if targets { HINT_TARGET } else { HINT_WALK }, pan);
        if !response.clicked() {
            return;
        }
        if ui.input(|i| i.modifiers.command) {
            self.markers.add(&Marker {
                name: String::new(),
                map: frame.map,
                x,
                y,
                icon: String::new(),
                color: NEW_MARKER_COLOR.to_string(),
            });
        } else if targets {
            let z = tools.scene.land_z(frame.map, x, y).unwrap_or(frame.z);
            tools.hand.act(Act::TargetGround { x, y, z });
        } else {
            tools.hand.act(Act::WalkTo { x, y });
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
        // The turned picture is a diamond as wide as the field, and the zoom
        // spreads it wider than that.
        let unit = field.width().min(field.height()) / (map_view::SPAN as f32 * HALF) * self.zoom;
        let lay = Lay::Turned {
            center: field.center(),
            from: Vec2::new(f32::from(frame.x), f32::from(frame.y)),
            unit,
        };
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
        let (width, height) = world_map::facet_size(frame.map);
        let fit = (field.width() / f32::from(width)).min(field.height() / f32::from(height));
        let scale = world_map::zoom_points(profile.world_map.zoom_step).max(fit);
        // The whole facet fits at the least zoom; closer, the view follows
        // the character, or stays where the player moved it.
        let middle = match self.looking_at {
            Some((x, y)) => Vec2::new(f32::from(x), f32::from(y)),
            None if scale <= fit => Vec2::new(f32::from(width), f32::from(height)) / HALF,
            None => Vec2::new(f32::from(frame.x), f32::from(frame.y)),
        };
        if response.dragged() && profile.world_map.free_view {
            self.looking_at = Some(map_view::whole_tile(middle - response.drag_delta() / scale));
        }
        let lay = Lay::NorthUp {
            center: field.center(),
            middle,
            scale,
        };
        self.pictures
            .draw_world(&ui.painter().with_clip_rect(field), lay)
            .then_some(lay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_steps_the_world_zoom_by_whole_turns() {
        let mut wheel = 0.6;
        assert_eq!(whole_turns(&mut wheel), 0, "not yet a whole turn");
        wheel += 0.6;
        assert_eq!(whole_turns(&mut wheel), 1);
        assert!((wheel - 0.2).abs() < 0.001, "the rest waits for the next");
        wheel = -2.5;
        assert_eq!(whole_turns(&mut wheel), -2);
    }

    #[test]
    fn a_large_window_gets_a_large_map() {
        let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0));
        assert_eq!(field_side(small), PANEL_SIDE);
        let large = Rect::from_min_size(Pos2::ZERO, Vec2::new(2560.0, 1440.0));
        assert!(field_side(large) > PANEL_SIDE, "a big screen shows more");
        assert!(field_side(large) < large.height(), "it stays in the window");
    }

    #[test]
    fn closing_the_map_closes_its_marker_boxes() {
        let mut map = MapUi::starting(false);
        map.markers
            .add(&world_map::marker_on_player(&WatchFrame::default()));
        map.close();
        assert!(!map.is_open());
    }
}
