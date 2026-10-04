//! The world map of the Modern style, its markers manager, and the quest
//! arrow. Near the character the map shows the land round him, turned as
//! the play field is turned; in the whole-world view it shows the facet
//! north up from the tiles of the whole-world picture the server makes.
//! The marker and zone files of the map folder come from the kept files of
//! the server; the player's own markers change by checked operations the
//! page posts, which the server makes under a lock. A click walks the
//! character there or targets the ground, while
//! the human has control. Where the land and each mark go and what a click
//! does are `uoterm_view::map_lay` and `ui::map_panel`.

use super::radar::{land_data, LandData, MarkData};
use super::sheet::Choice;
use super::{
    panel_notches, FrameSpec, Framed, NoteData, Place, PANEL_MARKERS, PANEL_MARKER_BOX,
    PANEL_WORLD_MAP,
};
use crate::{WebView, KEPT_PREFIX};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::art::WorldArt;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::map_lay::{
    map_tiles_on, mark_layout, near_still_serves, place_words, session_markers, whole_tile, Lay,
    MapFiles, Marks, MODERN_LOOK, ZOOM_MIN,
};
use uoterm_view::model::world_map::{
    self, marker_fault_words, MapFolder, MarkerChange, MAP_FILES_KEPT, MARKER_COLORS,
};
use uoterm_view::scene::overlays::quest_arrow;
use uoterm_view::ui::layout::{first_place, Spot};
use uoterm_view::ui::map_panel::{
    dragged_look, field_click, field_hints, field_of, goto_place, map_first_place, near_lay,
    near_zoom, view_words, whole_turns, world_lay, world_zoom, FieldClick, MapButton, MapLook,
    HINT_GOTO, MAP_BUTTONS, MAP_ID, NOTE_SECONDS, WORDS_GO, WORDS_NO_FILES, WORDS_RELOADED,
    WORDS_TITLE, WORDS_WALK,
};
use uoterm_view::ui::markers::{
    manager_first_place, row_buttons, row_words, MarkerBox, MarkerButton, BOX_ID, BOX_SIZE,
    HINT_ICON, HINT_SEARCH, MANAGER_ID, WORDS_CANCEL, WORDS_COLOR, WORDS_ICON, WORDS_MANAGER,
    WORDS_NAME, WORDS_NONE_FOUND, WORDS_NO_FILES as WORDS_NO_MARKER_FILES, WORDS_X, WORDS_Y,
};
use uoterm_view::ui::places::TITLE_ROW;
use uoterm_view::ui::quest_arrow::{arrow_act, arrow_click_area, HINT_ARROW};
use uoterm_view::ui::theme::PANEL_PAD;

/// The query that asks the tiles of the map again after Redraw.
const REDRAW_QUERY: &str = "?drawn=";

/// The world map and the markers manager as they stand.
pub(crate) struct WorldMapState {
    /// Open or not, and where the whole-world view looks.
    look: MapLook,
    zoom: f32,
    /// The turns of the wheel over the whole-world view that are not yet a
    /// whole zoom step.
    wheel: f32,
    /// The map and the middle tile of the picture of the land near the
    /// character.
    near: Option<(u8, (u16, u16))>,
    /// Every marker file and zone file of the map folder.
    pub folder: MapFolder,
    /// How many times the player drew the map again.
    redraw: u32,
    note: Option<(String, f64)>,
    /// Where the mouse is over the field.
    hover: Option<Point>,
    markers: MarkersState,
}

impl Default for WorldMapState {
    fn default() -> Self {
        Self {
            look: MapLook::default(),
            zoom: ZOOM_MIN,
            wheel: 0.0,
            near: None,
            folder: MapFolder::default(),
            redraw: 0,
            note: None,
            hover: None,
            markers: MarkersState::default(),
        }
    }
}

#[derive(Default)]
struct MarkersState {
    open: bool,
    /// The box that adds or changes a marker of the own file.
    marker_box: Option<MarkerBox>,
    /// The file whose markers show.
    file: usize,
    search: String,
}

/// One tile of the whole-world picture and where it lies on the field.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapTileData {
    pub path: String,
    pub place: Place,
}

/// The world map: its tool rows and its field, in the points of the field
/// from its top left.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorldMapData {
    pub live: bool,
    pub view_words: &'static str,
    pub goto_hint: &'static str,
    pub go: &'static str,
    /// Walk, while the human has control.
    pub walk: Option<&'static str>,
    /// Where the character stands, when the World Map page shows it.
    pub place: Option<String>,
    pub buttons: Vec<ButtonData>,
    pub side: Vector,
    /// The land near the character, turned.
    pub land: Option<LandData>,
    /// The tiles of the whole world, north up.
    pub tiles: Vec<MapTileData>,
    pub marks: Vec<MarkData>,
    pub no_files: &'static str,
    pub hint: &'static str,
    pub pan: &'static str,
    /// The place under the mouse, and the zone it lies in.
    pub mouse: Option<String>,
    pub zone: Option<String>,
    pub note: Option<NoteData>,
}

/// A button with its tip.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ButtonData {
    pub words: &'static str,
    pub hint: &'static str,
}

/// The markers manager: the marker files, a search, and the markers of
/// the open file that hold its words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarkersData {
    pub files: Vec<Choice>,
    pub search: String,
    pub search_hint: &'static str,
    pub rows: Vec<MarkerRow>,
    pub nothing: Option<&'static str>,
}

/// One marker: its place in its file, its words and its buttons.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarkerRow {
    pub at: usize,
    pub words: String,
    pub buttons: Vec<&'static str>,
}

/// The box that adds or changes a marker of the own file: its fields as
/// typed, the colors, and why its fields make no marker.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarkerBoxData {
    pub x: String,
    pub y: String,
    pub name: String,
    pub icon: String,
    /// The place of the color in `colors`.
    pub color: usize,
    pub colors: Vec<&'static str>,
    /// The words of the fields: x, y, name, icon and color.
    pub labels: [&'static str; 5],
    pub icon_hint: &'static str,
    pub error: Option<String>,
    pub submit: &'static str,
    pub cancel: &'static str,
}

/// The fields of the marker box as the player typed them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct TypedFields {
    x: String,
    y: String,
    name: String,
    icon: String,
    color: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MarkerBoxAction {
    Fields(TypedFields),
    Submit(bool),
    Cancel(bool),
}

/// The box round the quest arrow that takes clicks, in the points of the
/// panel layer.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuestArrowData {
    pub place: Place,
    pub hint: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct GotoWords {
    words: String,
    walk: bool,
}

/// A click on the field, in its points, with Ctrl down or not.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
struct FieldPress {
    x: f32,
    y: f32,
    ctrl: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MapAction {
    /// Turn between the near view and the whole world.
    View(bool),
    Goto(GotoWords),
    /// A button of the markers row, by its place.
    Button(usize),
    Wheel(f32),
    Click(FieldPress),
    /// A drag over the whole-world view, by this many points.
    Drag(Vector),
    Hover(Option<Point>),
}

/// A press on a button of a marker row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct RowPress {
    at: usize,
    button: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MarkersAction {
    File(usize),
    Search(String),
    Row(RowPress),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct ArrowPress {
    right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ArrowAction {
    Click(ArrowPress),
}

impl WebView {
    /// The marker and zone files of the map folder came.
    pub(crate) fn take_map_folder(&mut self, folder: MapFolder) {
        self.panels.world_map.folder = folder;
    }

    /// Asks for the marker and zone files again.
    fn read_map_folder(&mut self) {
        self.kept_wanted
            .push(format!("{KEPT_PREFIX}{MAP_FILES_KEPT}"));
    }

    /// Opens or closes the world map, as its window command does.
    pub(crate) fn toggle_world_map(&mut self) {
        let map = &mut self.panels.world_map;
        map.look.open = !map.look.open;
    }

    pub(crate) fn world_map_open(&self) -> bool {
        self.panels.world_map.look.open
    }

    /// Closes the map and its markers manager.
    pub(crate) fn close_world_map(&mut self) {
        let map = &mut self.panels.world_map;
        map.look.open = false;
        map.markers.open = false;
        map.markers.marker_box = None;
    }

    /// Shows a place in the whole-world view.
    fn look_at(&mut self, place: (u16, u16)) {
        let look = &mut self.panels.world_map.look;
        if look.look_at(place, &mut self.profile.world_map) {
            self.keep_profile();
        }
    }

    pub(super) fn world_map_spec(&self) -> Option<FrameSpec> {
        if !self.panels.world_map.look.open {
            return None;
        }
        let (default, least) = map_first_place(self.panel_room());
        Some(
            FrameSpec::fixed(MAP_ID, WORDS_TITLE, default)
                .closable()
                .sized(least),
        )
    }

    /// The field of the map in the points of the field, and how it lays
    /// tiles now.
    fn map_field(&self, frame: &WatchFrame, spec: &FrameSpec) -> (Area, Lay) {
        let area = self.panel_area(spec);
        let body = Area::from_min_size(
            Point::new(0.0, 0.0),
            Vector::new(
                (area.width() - PANEL_PAD * 2.0).max(0.0),
                (area.height() - PANEL_PAD * 2.0 - TITLE_ROW).max(0.0),
            ),
        );
        let field = field_of(body);
        let field = Area::from_min_size(Point::new(0.0, 0.0), field.size());
        let map = &self.panels.world_map;
        let lay = if self.profile.world_map.whole_world {
            world_lay(field, frame, &self.profile.world_map, map.look.looking_at)
        } else {
            near_lay(field, frame, map.zoom)
        };
        (field, lay)
    }

    pub(super) fn world_map_data(
        &mut self,
        frame: &WatchFrame,
        time: f64,
    ) -> Option<Framed<WorldMapData>> {
        let spec = self.world_map_spec()?;
        let (field, lay) = self.map_field(frame, &spec);
        let options = self.profile.world_map.clone();
        let whole = options.whole_world;
        let here = (frame.x, frame.y);
        let middle = match self.panels.world_map.near {
            Some((map, drawn)) if near_still_serves(map, drawn, frame.map, here) => drawn,
            _ => here,
        };
        self.panels.world_map.near = Some((frame.map, middle));
        let redraw = format!("{REDRAW_QUERY}{}", self.panels.world_map.redraw);
        let land = (!whole).then(|| {
            let mut land = land_data(frame.map, middle, lay);
            land.path.push_str(&redraw);
            land
        });
        let tiles = if whole {
            map_tiles_on(field, lay, frame.map)
                .into_iter()
                .map(|tile| MapTileData {
                    path: format!("{}{redraw}", tile.path),
                    place: Place::from(tile.area),
                })
                .collect()
        } else {
            Vec::new()
        };
        let session = session_markers(self.hand.reads(), &self.profile, frame.map);
        let state = &self.panels.world_map;
        let files = MapFiles::shown(&state.folder, &options);
        let marks = Marks {
            frame,
            map: frame.map,
            profile: &self.profile,
            files: &files,
            session: &session,
            looking_at: state.look.looking_at,
        };
        let marks = mark_layout(field, lay, &marks, &MODERN_LOOK)
            .into_iter()
            .map(MarkData::from)
            .collect();
        let under = state.hover.map(|at| whole_tile(lay.tile(at)));
        let (hint, pan) = field_hints(frame, &options);
        let body = WorldMapData {
            live: frame.human_control,
            view_words: view_words(whole),
            goto_hint: HINT_GOTO,
            go: WORDS_GO,
            walk: frame.human_control.then_some(WORDS_WALK),
            place: options
                .show_coordinates
                .then(|| place_words(&self.profile, frame.map, frame.x, frame.y)),
            buttons: MAP_BUTTONS
                .iter()
                .map(|button| ButtonData {
                    words: button.words(),
                    hint: button.hint(),
                })
                .collect(),
            side: field.size(),
            land,
            tiles,
            marks,
            no_files: WORDS_NO_FILES,
            hint,
            pan,
            mouse: under
                .filter(|_| options.show_mouse_coordinates)
                .map(|(x, y)| place_words(&self.profile, frame.map, x, y)),
            zone: under.and_then(|(x, y)| files.zone_at(frame.map, x, y)),
            note: state
                .note
                .as_ref()
                .filter(|(_, since)| time - since <= NOTE_SECONDS)
                .map(|(words, _)| NoteData {
                    words: words.clone(),
                    failed: false,
                }),
        };
        Some(self.framed(PANEL_WORLD_MAP, &spec, body))
    }

    pub(super) fn world_map_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Some(spec) = self.world_map_spec() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<MapAction>(action) else {
            return;
        };
        let time = self.hand.time();
        let (_, lay) = self.map_field(&frame, &spec);
        match action {
            MapAction::View(_) => {
                let options = &mut self.profile.world_map;
                options.whole_world = !options.whole_world;
                self.panels.world_map.look.looking_at = None;
                self.keep_profile();
            }
            MapAction::Goto(asked) => match goto_place(&asked.words) {
                Ok(place) => {
                    self.look_at(place);
                    if asked.walk && frame.human_control {
                        self.hand.act(Act::WalkTo {
                            x: place.0,
                            y: place.1,
                        });
                    }
                }
                Err(words) => self.panels.world_map.note = Some((words.to_string(), time)),
            },
            MapAction::Button(at) => match MAP_BUTTONS.get(at) {
                Some(MapButton::Redraw) => {
                    let map = &mut self.panels.world_map;
                    map.redraw += 1;
                    map.near = None;
                }
                Some(MapButton::Reload) => {
                    self.read_map_folder();
                    self.panels.world_map.note = Some((WORDS_RELOADED.to_string(), time));
                }
                Some(MapButton::Markers) => {
                    let markers = &mut self.panels.world_map.markers;
                    markers.open = !markers.open;
                }
                Some(MapButton::MarkMe) => {
                    self.open_marker_box(MarkerBox::adding(&world_map::marker_on_player(&frame)))
                }
                None => {}
            },
            MapAction::Wheel(notches) => {
                let notches = panel_notches(notches);
                if self.profile.world_map.whole_world {
                    let map = &mut self.panels.world_map;
                    map.wheel += notches;
                    let turns = whole_turns(&mut map.wheel);
                    if world_zoom(&mut self.profile.world_map, turns) {
                        self.keep_profile();
                    }
                } else {
                    let map = &mut self.panels.world_map;
                    map.zoom = near_zoom(map.zoom, notches);
                }
            }
            MapAction::Drag(by) => {
                if self.profile.world_map.whole_world && self.profile.world_map.free_view {
                    self.panels.world_map.look.looking_at = dragged_look(lay, by);
                }
            }
            MapAction::Hover(at) => self.panels.world_map.hover = at,
            MapAction::Click(press) => {
                let tile = whole_tile(lay.tile(Point::new(press.x, press.y)));
                let art = &mut self.art;
                let click = field_click(&frame, &self.profile.world_map, tile, press.ctrl, || {
                    art.land_z(frame.map, tile.0, tile.1)
                });
                match click {
                    Some(FieldClick::Act(act)) => self.hand.act(act),
                    Some(FieldClick::Mark(marker)) => {
                        self.open_marker_box(MarkerBox::adding(&marker));
                    }
                    None => {}
                }
            }
        }
    }

    pub(super) fn markers_spec(&self) -> Option<FrameSpec> {
        if !self.panels.world_map.markers.open {
            return None;
        }
        let (default, least) = manager_first_place(self.panel_room());
        Some(
            FrameSpec::fixed(MANAGER_ID, WORDS_MANAGER, default)
                .closable()
                .sized(least),
        )
    }

    pub(super) fn markers_data(&self) -> Option<Framed<MarkersData>> {
        let spec = self.markers_spec()?;
        let state = &self.panels.world_map;
        let files = &state.folder.markers;
        let shown = files.get(state.markers.file);
        let found = shown.map_or_else(Vec::new, |file| {
            world_map::found(&file.markers, &state.markers.search)
        });
        let buttons: Vec<&'static str> = shown.map_or_else(Vec::new, |file| {
            row_buttons(&file.name, true)
                .iter()
                .map(|button| button.words())
                .collect()
        });
        let nothing = match shown {
            None => Some(WORDS_NO_MARKER_FILES),
            Some(_) if found.is_empty() => Some(WORDS_NONE_FOUND),
            Some(_) => None,
        };
        let body = MarkersData {
            files: files
                .iter()
                .enumerate()
                .map(|(at, file)| Choice {
                    words: file.name.clone(),
                    chosen: at == state.markers.file,
                })
                .collect(),
            search: state.markers.search.clone(),
            search_hint: HINT_SEARCH,
            rows: found
                .iter()
                .map(|(at, marker)| MarkerRow {
                    at: *at,
                    words: row_words(marker),
                    buttons: buttons.clone(),
                })
                .collect(),
            nothing,
        };
        Some(self.framed(PANEL_MARKERS, &spec, body))
    }

    pub(super) fn markers_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<MarkersAction>(action) else {
            return;
        };
        let state = &mut self.panels.world_map;
        match action {
            MarkersAction::File(at) if at < state.folder.markers.len() => {
                state.markers.file = at;
            }
            MarkersAction::Search(words) => state.markers.search = words,
            MarkersAction::Row(press) => {
                let Some(file) = state.folder.markers.get(state.markers.file) else {
                    return;
                };
                let button = row_buttons(&file.name, true).get(press.button);
                let Some(marker) = file.markers.get(press.at).cloned() else {
                    return;
                };
                match button {
                    Some(MarkerButton::Go) => self.look_at((marker.x, marker.y)),
                    Some(MarkerButton::Edit) => {
                        self.open_marker_box(MarkerBox::editing(press.at, &marker));
                    }
                    Some(MarkerButton::Remove) => self.change_markers(&MarkerChange::Remove {
                        at: press.at,
                        expected: marker,
                    }),
                    None => {}
                }
            }
            MarkersAction::File(_) => {}
        }
    }

    pub(super) fn close_markers(&mut self) {
        self.panels.world_map.markers.open = false;
    }

    fn open_marker_box(&mut self, marker_box: MarkerBox) {
        self.panels.world_map.markers.marker_box = Some(marker_box);
    }

    /// Sends a change of the own marker file to the server.
    fn change_markers(&mut self, change: &MarkerChange) {
        self.art.post_marker_change(change);
    }

    /// The answers to the changes of the own marker file: a change made
    /// shuts the box; one not made says why. The files are read again
    /// either way, as the window reads them again.
    pub(crate) fn follow_marker_changes(&mut self, time: f64) {
        for answer in self.art.take_marker_answers() {
            let markers = &mut self.panels.world_map.markers;
            let words = match answer {
                Ok(()) => {
                    markers.marker_box = None;
                    None
                }
                Err(status) => Some(marker_fault_words(status).to_string()),
            };
            if let Some(words) = words {
                match markers.marker_box.as_mut() {
                    Some(marker_box) => marker_box.error = Some(words),
                    None => self.panels.world_map.note = Some((words, time)),
                }
            }
            self.read_map_folder();
        }
    }

    pub(super) fn marker_box_spec(&self) -> Option<FrameSpec> {
        let marker_box = self.panels.world_map.markers.marker_box.as_ref()?;
        let default = first_place(self.panel_room(), Spot::Middle(0), BOX_SIZE);
        Some(FrameSpec::fixed(BOX_ID, marker_box.title(), default).closable())
    }

    pub(super) fn marker_box_data(&self) -> Option<Framed<MarkerBoxData>> {
        let spec = self.marker_box_spec()?;
        let marker_box = self.panels.world_map.markers.marker_box.as_ref()?;
        let fields = &marker_box.fields;
        let body = MarkerBoxData {
            x: fields.x.clone(),
            y: fields.y.clone(),
            name: fields.name.clone(),
            icon: fields.icon.clone(),
            color: fields.color,
            colors: MARKER_COLORS.to_vec(),
            labels: [WORDS_X, WORDS_Y, WORDS_NAME, WORDS_ICON, WORDS_COLOR],
            icon_hint: HINT_ICON,
            error: marker_box.error.clone(),
            submit: marker_box.submit_words(),
            cancel: WORDS_CANCEL,
        };
        Some(self.framed(PANEL_MARKER_BOX, &spec, body))
    }

    pub(super) fn marker_box_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<MarkerBoxAction>(action) else {
            return;
        };
        let Some(marker_box) = self.panels.world_map.markers.marker_box.as_mut() else {
            return;
        };
        match action {
            MarkerBoxAction::Fields(typed) => {
                let fields = &mut marker_box.fields;
                fields.x = typed.x;
                fields.y = typed.y;
                fields.name = typed.name;
                fields.icon = typed.icon;
                fields.color = typed.color.min(MARKER_COLORS.len() - 1);
            }
            MarkerBoxAction::Submit(_) => {
                if let Some(change) = marker_box.submit() {
                    self.change_markers(&change);
                }
            }
            MarkerBoxAction::Cancel(_) => self.close_marker_box(),
        }
    }

    pub(super) fn close_marker_box(&mut self) {
        self.panels.world_map.markers.marker_box = None;
    }

    /// The box round the quest arrow that takes clicks, while the shard
    /// points one.
    pub(super) fn quest_arrow_data(&self, frame: &WatchFrame) -> Option<QuestArrowData> {
        let (_, arrow) = quest_arrow(&self.scene.projection(self.view), frame)?;
        let scale = self.profile.video.ui_scale.max(f32::EPSILON);
        let area = arrow_click_area(arrow);
        Some(QuestArrowData {
            place: Place {
                x: area.min.x / scale,
                y: area.min.y / scale,
                w: area.width() / scale,
                h: area.height() / scale,
            },
            hint: frame.human_control.then_some(HINT_ARROW),
        })
    }

    pub(super) fn quest_arrow_action(&mut self, action: Value) {
        let Some(frame) = self.frame.as_ref() else {
            return;
        };
        let Ok(ArrowAction::Click(press)) = serde_json::from_value::<ArrowAction>(action) else {
            return;
        };
        if let Some(act) = arrow_act(frame, press.right) {
            self.hand.act(act);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, saved_profiles, view_with};
    use super::super::PANEL_QUEST_ARROW;
    use super::*;
    use crate::tests::settled;
    use serde_json::json;
    use uoterm_view::map_lay::MAP_PICTURE_PREFIX;
    use uoterm_view::model::world_map::{Marker, MARKER_CHANGE_PATH};
    use uoterm_view::settings::WorldMapOptions;

    fn open_map(control: bool) -> WebView {
        let mut view = view_with("human_control", json!(control), control);
        view.toggle_world_map();
        view
    }

    /// The middle of the field of the open map.
    fn field_middle(view: &mut WebView) -> Vector {
        view.panel_data(0.0).world_map.unwrap().body.side / 2.0
    }

    #[test]
    fn a_click_on_the_field_walks_there_as_the_window() {
        let mut view = open_map(true);
        let middle = field_middle(&mut view);
        let out = press(
            &mut view,
            PANEL_WORLD_MAP,
            json!({ "click": { "x": middle.x, "y": middle.y, "ctrl": false } }),
        );
        let frame = view.frame_ref().unwrap().clone();
        let options = WorldMapOptions::default();
        let Some(FieldClick::Act(same)) =
            field_click(&frame, &options, (frame.x, frame.y), false, || None)
        else {
            panic!("a walk");
        };
        assert_eq!(out_acts(&out), vec![same.for_page()]);
        let out = press(
            &mut view,
            PANEL_WORLD_MAP,
            json!({ "click": { "x": middle.x, "y": middle.y, "ctrl": true } }),
        );
        assert!(out_acts(&out).is_empty(), "Ctrl marks the place");
        let marking = view.panel_data(0.0).marker_box.unwrap().body;
        assert_eq!(
            (marking.x, marking.y),
            (frame.x.to_string(), frame.y.to_string())
        );
    }

    #[test]
    fn the_whole_world_shows_its_tiles_and_the_wheel_steps_its_zoom() {
        let mut view = open_map(true);
        let near = view.panel_data(0.0).world_map.unwrap().body;
        assert!(near.land.is_some() && near.tiles.is_empty());
        let out = press(&mut view, PANEL_WORLD_MAP, json!({ "view": true }));
        assert!(saved_profiles(&out)[0].world_map.whole_world);
        let world = view.panel_data(0.0).world_map.unwrap().body;
        assert!(world.land.is_none());
        assert!(world.tiles[0].path.starts_with(MAP_PICTURE_PREFIX));
        let step = view.profile.world_map.zoom_step;
        press(&mut view, PANEL_WORLD_MAP, json!({ "wheel": 3.0 }));
        assert_ne!(view.profile.world_map.zoom_step, step);
        press(&mut view, PANEL_WORLD_MAP, json!({ "button": 0 }));
        let redrawn = view.panel_data(0.0).world_map.unwrap().body;
        assert_ne!(redrawn.tiles[0].path, world.tiles[0].path, "asked again");
    }

    #[test]
    fn walk_goes_to_a_place_and_bad_words_say_so() {
        let mut view = open_map(true);
        let out = press(
            &mut view,
            PANEL_WORLD_MAP,
            json!({ "goto": { "words": "1434 1699", "walk": true } }),
        );
        let acts = out_acts(&out);
        assert_eq!(acts, vec![Act::WalkTo { x: 1434, y: 1699 }.for_page()]);
        assert!(view.profile.world_map.whole_world, "the map looks there");
        press(
            &mut view,
            PANEL_WORLD_MAP,
            json!({ "goto": { "words": "nowhere", "walk": false } }),
        );
        let note = view.panel_data(0.0).world_map.unwrap().body.note.unwrap();
        assert_eq!(note.words, uoterm_view::ui::map_panel::WORDS_NO_PLACE);
    }

    #[test]
    fn the_field_does_not_act_without_control() {
        let mut view = open_map(false);
        let middle = field_middle(&mut view);
        assert!(view.panel_data(0.0).world_map.unwrap().body.walk.is_none());
        let click = json!({ "click": { "x": middle.x, "y": middle.y, "ctrl": false } });
        assert!(out_acts(&press(&mut view, PANEL_WORLD_MAP, click)).is_empty());
        let walk = json!({ "goto": { "words": "1 1", "walk": true } });
        assert!(out_acts(&press(&mut view, PANEL_WORLD_MAP, walk)).is_empty());
    }

    #[test]
    fn the_marker_files_show_on_the_maps_and_go_shows_a_marker() {
        let mut view = settled();
        let path = format!("{KEPT_PREFIX}{MAP_FILES_KEPT}");
        assert!(view.data_wanted_native().contains(&path), "asked once");
        let folder = json!({ "markers": [{ "name": "towns", "markers": [
            { "name": "Bank", "map": 0, "x": 1001, "y": 1001, "icon": "", "color": "" }] }],
            "zones": [] });
        view.data_arrived_native(&path, &folder);
        let radar = view.panel_data(0.0).radar.unwrap().body;
        let named = radar
            .marks
            .iter()
            .any(|mark| matches!(mark, MarkData::Words { words, .. } if words == "Bank"));
        assert!(named, "the radar shows the marker");
        view.toggle_world_map();
        press(&mut view, PANEL_WORLD_MAP, json!({ "button": 3 }));
        let markers = view.panel_data(0.0).markers.unwrap().body;
        assert_eq!(markers.rows[0].words, "Bank  1001, 1001  ");
        assert_eq!(markers.rows[0].buttons, vec![MarkerButton::Go.words()]);
        press(
            &mut view,
            PANEL_MARKERS,
            json!({ "row": { "at": 0, "button": 0 } }),
        );
        assert_eq!(view.panels.world_map.look.looking_at, Some((1001, 1001)));
        press(&mut view, PANEL_WORLD_MAP, json!({ "button": 1 }));
        assert!(
            view.data_wanted_native().contains(&path),
            "Reload reads again"
        );
    }

    /// The changes of the own marker file the view sent to the server.
    fn sent_changes(view: &mut WebView) -> Vec<(String, MarkerChange)> {
        view.art
            .take_posts()
            .into_iter()
            .filter(|post| post.path == MARKER_CHANGE_PATH)
            .map(|post| (post.key, serde_json::from_value(post.body).unwrap()))
            .collect()
    }

    #[test]
    fn mark_me_adds_a_marker_through_the_box_and_reads_the_files_again() {
        let mut view = open_map(true);
        view.data_wanted_native();
        let mark_me = MAP_BUTTONS
            .iter()
            .position(|button| *button == MapButton::MarkMe)
            .unwrap();
        press(&mut view, PANEL_WORLD_MAP, json!({ "button": mark_me }));
        let frame = view.frame_ref().unwrap().clone();
        let mut same = MarkerBox::adding(&world_map::marker_on_player(&frame));
        let fields = json!({ "fields": { "x": "1000", "y": "1000", "name": "Home",
            "icon": "", "color": 1 } });
        press(&mut view, PANEL_MARKER_BOX, fields);
        same.fields.name = "Home".into();
        same.fields.color = 1;
        press(&mut view, PANEL_MARKER_BOX, json!({ "submit": true }));
        let sent = sent_changes(&mut view);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].1, same.submit().unwrap());
        view.post_arrived(&sent[0].0, &json!({ "changed": true }).to_string());
        view.tick_native(0.2, crate::tests::VIEW, None);
        assert!(
            view.panel_data(0.2).marker_box.is_none(),
            "kept: the box shuts"
        );
        let path = format!("{KEPT_PREFIX}{MAP_FILES_KEPT}");
        assert!(view.data_wanted_native().contains(&path));
    }

    #[test]
    fn bad_fields_and_a_stale_file_say_why_and_change_nothing() {
        let mut view = open_map(true);
        press(&mut view, PANEL_WORLD_MAP, json!({ "button": 2 }));
        let bad = json!({ "fields": { "x": "x", "y": "1", "name": "A", "icon": "", "color": 0 } });
        press(&mut view, PANEL_MARKER_BOX, bad);
        press(&mut view, PANEL_MARKER_BOX, json!({ "submit": true }));
        assert!(sent_changes(&mut view).is_empty());
        let error = view.panel_data(0.0).marker_box.unwrap().body.error;
        assert_eq!(error.as_deref(), Some(world_map::WORDS_INVALID_MARKER));
        let good = json!({ "fields": { "x": "1", "y": "1", "name": "A", "icon": "", "color": 0 } });
        press(&mut view, PANEL_MARKER_BOX, good);
        press(&mut view, PANEL_MARKER_BOX, json!({ "submit": true }));
        let sent = sent_changes(&mut view);
        view.post_missing(&sent[0].0, world_map::MARKER_STALE_STATUS);
        view.tick_native(0.2, crate::tests::VIEW, None);
        let error = view.panel_data(0.2).marker_box.unwrap().body.error;
        assert_eq!(error.as_deref(), Some(world_map::WORDS_STALE_MARKERS));
        press(&mut view, PANEL_MARKER_BOX, json!({ "submit": true }));
        let sent = sent_changes(&mut view);
        view.post_missing(&sent[0].0, 0);
        view.tick_native(0.3, crate::tests::VIEW, None);
        let error = view.panel_data(0.3).marker_box.unwrap().body.error;
        assert_eq!(
            error.as_deref(),
            Some(world_map::WORDS_MARKER_NOT_CHANGED),
            "a change no answer came to does not say the file changed"
        );
    }

    #[test]
    fn the_own_file_edits_and_removes_its_markers_by_their_place() {
        let mut view = settled();
        let path = format!("{KEPT_PREFIX}{MAP_FILES_KEPT}");
        let camp = json!({ "name": "Camp", "map": 0, "x": 5, "y": 6, "icon": "", "color": "red" });
        let folder = json!({ "markers": [{ "name": world_map::USER_MARKERS, "markers": [camp] }],
            "zones": [] });
        view.data_arrived_native(&path, &folder);
        view.toggle_world_map();
        press(&mut view, PANEL_WORLD_MAP, json!({ "button": 3 }));
        let rows = view.panel_data(0.0).markers.unwrap().body.rows;
        let words: Vec<&str> = row_buttons(world_map::USER_MARKERS, true)
            .iter()
            .map(|button| button.words())
            .collect();
        assert_eq!(rows[0].buttons, words);
        let marker: Marker = serde_json::from_value(camp).unwrap();
        press(
            &mut view,
            PANEL_MARKERS,
            json!({ "row": { "at": 0, "button": 0 } }),
        );
        assert_eq!(
            view.panel_data(0.0).marker_box.unwrap().frame.title,
            MarkerBox::editing(0, &marker).title()
        );
        press(
            &mut view,
            PANEL_MARKERS,
            json!({ "row": { "at": 0, "button": 1 } }),
        );
        let removed = MarkerChange::Remove {
            at: 0,
            expected: marker,
        };
        assert_eq!(sent_changes(&mut view)[0].1, removed);
    }

    #[test]
    fn a_click_on_the_quest_arrow_tells_the_shard_with_control_only() {
        let arrow = json!({ "x": 1010, "y": 990 });
        let mut view = view_with("quest_arrow", arrow.clone(), true);
        assert!(view.panel_data(0.0).quest_arrow.unwrap().hint.is_some());
        let out = press(
            &mut view,
            PANEL_QUEST_ARROW,
            json!({ "click": { "right": true } }),
        );
        assert_eq!(
            out_acts(&out),
            vec![Act::QuestArrow { right: true }.for_page()]
        );
        let mut view = view_with("quest_arrow", arrow, false);
        let out = press(
            &mut view,
            PANEL_QUEST_ARROW,
            json!({ "click": { "right": false } }),
        );
        assert!(out_acts(&out).is_empty());
    }
}
