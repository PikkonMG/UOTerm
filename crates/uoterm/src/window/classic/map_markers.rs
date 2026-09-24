//! The gumps that serve the world map, as the reference client has them: the
//! markers manager that lists the markers of each marker file with a
//! search, and lets the player change, remove and go to the markers of his
//! own file; the box that adds or changes one marker; and the box that
//! moves the map to typed coordinates.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use super::world_map::{ANSWER_GO_X, ANSWER_GO_Y, ANSWER_RELOAD};
use crate::window::model::world_map::{
    self, Marker, MarkerFields, MarkerFile, MARKER_COLORS, USER_MARKERS,
};
use eframe::egui::{Color32, Pos2};

pub const MARKERS_MANAGER: GumpKind = GumpKind {
    id: well_known::MARKERS_MANAGER,
    rules: GumpRules {
        first_place: Pos2::new(50.0, 50.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(MarkersManager::default()),
};

pub const USER_MARKER: GumpKind = GumpKind {
    id: well_known::USER_MARKER,
    rules: GumpRules {
        kept: false,
        first_place: Pos2::new(250.0, 150.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(UserMarker::adding(unnamed_marker())),
};

pub const LOCATION_GO: GumpKind = GumpKind {
    id: well_known::LOCATION_GO,
    rules: GumpRules {
        modal: true,
        kept: false,
        first_place: Pos2::new(300.0, 250.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(LocationGo::default()),
};

/// The answer that tells the markers manager to read the files again.
const ANSWER_MANAGER_RELOAD: &str = "markers_manager:reload";
const ANSWERED: u16 = 1;
/// The white words with a black ring of these gumps.
const WORDS_FONT: u8 = 1;
const WORDS_HUE: u16 = 0xFFFF;
const ERROR_HUE: u16 = 0x0021;
/// The dark see-through backgrounds, in the darkest color of a hue.
const DARK_HUE: u16 = 999;
const BLACK_HUE: u16 = 0;
const FIELD_FRAME: u16 = 0x0BB8;
const LINE_COLOR: Color32 = Color32::GRAY;
const LINE: i32 = 1;

// The markers manager.
const MANAGER_WIDTH: i32 = 620;
const MANAGER_HEIGHT: i32 = 500;
const MANAGER_OPACITY: f32 = 0.95;
const LEGEND_Y: i32 = 10;
const LEGEND_RULE_Y: i32 = 30;
/// Each head of the list: its words, where it stands and how wide it is.
const LEGEND: [(&str, i32, u32); 8] = [
    ("Icon", 5, 185),
    ("Name", 50, 185),
    ("X", 315, 35),
    ("Y", 380, 35),
    ("Color", 420, 35),
    ("Edit", 475, 35),
    ("Remove", 505, 40),
    ("GoTo", 550, 40),
];
const SEARCH_X: i32 = MANAGER_WIDTH / 2 - 150;
const SEARCH_Y: i32 = 40;
const SEARCH_WORDS: &str = "Search:";
const SEARCH_WORDS_WIDTH: u32 = 50;
const SEARCH_FIELD_X: i32 = 50;
const SEARCH_FIELD_WIDTH: i32 = 200;
const SEARCH_TEXT_INSET: i32 = 3;
const SEARCH_MAX_CHARS: usize = 30;
const SEARCH_BUTTON: ButtonArt = ButtonArt::new(0x0FB7, 0x0FB9, 0);
const SEARCH_BUTTON_X: i32 = 250;
const CLEAR_BUTTON: ButtonArt = ButtonArt::new(0x0FB1, 0x0FB2, 0);
const CLEAR_BUTTON_X: i32 = 285;
const LIST_AREA: (i32, i32, i32, i32) = (10, 80, MANAGER_WIDTH - 20, 370);
const ROW_HEIGHT: i32 = 25;
/// Each part of a row: where it stands, and how wide its words may be.
const ROW_NAME: (i32, u32) = (30, 280);
const ROW_X: (i32, u32) = (305, 35);
const ROW_Y: (i32, u32) = (350, 35);
const ROW_COLOR: (i32, u32) = (410, 35);
const EDIT_BUTTON: ButtonArt = ButtonArt::new(0x0FAB, 0x0FAC, 0);
const EDIT_X: i32 = 470;
const REMOVE_BUTTON: ButtonArt = ButtonArt::new(0x0FB1, 0x0FB2, 0);
const REMOVE_X: i32 = 505;
const GO_BUTTON: ButtonArt = ButtonArt::new(0x0FA5, 0x0FA7, 0);
const GO_X: i32 = 540;
const FILES_HEIGHT: i32 = 40;
const FILES_Y: i32 = MANAGER_HEIGHT - FILES_HEIGHT;
const FILE_BUTTON_LEAST: i32 = 50;

// The marker box.
const MARKER_WIDTH: i32 = 320;
const MARKER_HEIGHT: i32 = 220;
const MARKER_OPACITY: f32 = 0.7;
const MARKER_TITLE: (i32, i32) = (100, 3);
const WORDS_ADD: &str = "Add Marker";
const WORDS_EDIT_MARKER: &str = "Edit Marker";
const FIELDS_X: i32 = 5;
const FIELDS_Y: i32 = 25;
const FIELDS_STEP: i32 = 30;
const FIELD_LABEL_ROOM: i32 = 40;
const FIELD_HEIGHT: i32 = 25;
const NUMBER_FIELD_WIDTH: i32 = 90;
const NAME_FIELD_WIDTH: i32 = 250;
const NUMBER_MAX_CHARS: usize = 10;
const NAME_MAX_CHARS: usize = 25;
const MARKER_BUTTONS_Y: i32 = MARKER_HEIGHT - 30;
const MARKER_BUTTON_WIDTH: i32 = 60;
const OK_BUTTON_X: i32 = 13;
const CANCEL_BUTTON_X: i32 = 78;
const ERROR_Y: i32 = MARKER_BUTTONS_Y - 20;
const WORDS_CREATE: &str = "Create";
const WORDS_EDIT: &str = "Edit";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_X: &str = "X";
const WORDS_Y: &str = "Y";
const WORDS_NAME: &str = "Name";
const WORDS_COLOR: &str = "Color";

// The go-to box.
const GO_WIDTH: i32 = 250;
const GO_HEIGHT: i32 = 150;
const GO_OPACITY: f32 = 0.7;
const GO_WORDS: &str = "Enter Location";
const GO_WORDS_AT: (i32, i32) = (12, 12);
const GO_WORDS_ROOM: i32 = 90;
const GO_FIELD_ROOM: i32 = 94;
const GO_FIELD_X: i32 = 20;
const GO_FIELD_TOP: i32 = 20;
const GO_FIELD_PAD: i32 = 10;
const GO_FIELD_GAP: i32 = 5;
const GO_TEXT_INSET: (i32, i32) = (6, 2);
const LOCATION_OK: ButtonArt = ButtonArt::new(0x0481, 0x0482, 0x0483);
const GO_BUTTON_GAP: i32 = 12;
const GO_EXAMPLES_GAP: i32 = 28;
const GO_EXAMPLES: &str = "Examples:\n 1639, 1532\n 100o25'S,40o04'E\n 9 14'N 91 37'W";
/// The words turn green when they name a place, and red when they do not.
const PLACE_HUE: u16 = 0x0040;
const NO_PLACE_HUE: u16 = 0x0033;

fn words_look() -> TextLook {
    TextLook::unicode(WORDS_FONT, WORDS_HUE).bordered()
}

/// A marker with no name yet, for a box the kind opens by itself.
fn unnamed_marker() -> Marker {
    Marker {
        name: String::new(),
        map: 0,
        x: 0,
        y: 0,
        icon: String::new(),
        color: MARKER_COLORS[0].to_string(),
    }
}

/// Tells the world map and the manager that the marker files changed.
fn markers_changed(cx: &mut GumpContext<'_>) {
    cx.answer(ANSWER_RELOAD, ANSWERED);
    cx.answer(ANSWER_MANAGER_RELOAD, ANSWERED);
}

/// Moves the world map to a place, opening it when it is shut.
fn show_on_map(cx: &mut GumpContext<'_>, x: u16, y: u16) {
    cx.open(GumpId::one(well_known::WORLD_MAP));
    cx.answer(ANSWER_GO_X, x);
    cx.answer(ANSWER_GO_Y, y);
}

/// The markers manager.
pub struct MarkersManager {
    files: Vec<MarkerFile>,
    /// The file whose markers show.
    file: usize,
    search: TextField,
    /// The search words the list was filtered by last.
    searched: String,
    error: Option<String>,
    loaded: bool,
}

impl Default for MarkersManager {
    fn default() -> Self {
        let mut search = TextField::new("");
        search.max_chars = Some(SEARCH_MAX_CHARS);
        Self {
            files: Vec::new(),
            file: 0,
            search,
            searched: String::new(),
            error: None,
            loaded: false,
        }
    }
}

impl MarkersManager {
    fn load(&mut self) {
        self.files = world_map::load_markers(&world_map::map_dir(), &[]);
        self.file = self.file.min(self.files.len().saturating_sub(1));
        self.loaded = true;
    }

    fn legend_and_search(&mut self, g: &mut Canvas<'_>) {
        let look = words_look();
        for (words, x, width) in LEGEND {
            g.label(x, LEGEND_Y, words, &look.cropped(width));
        }
        g.fill(0, LEGEND_RULE_Y, MANAGER_WIDTH, LINE, LINE_COLOR);
        g.label(
            SEARCH_X,
            SEARCH_Y,
            SEARCH_WORDS,
            &look.cropped(SEARCH_WORDS_WIDTH),
        );
        let field_x = SEARCH_X + SEARCH_FIELD_X;
        g.frame(
            field_x,
            SEARCH_Y,
            SEARCH_FIELD_WIDTH,
            FIELD_HEIGHT,
            FIELD_FRAME,
        );
        let typed = g.text_box(
            "search",
            field_x + SEARCH_TEXT_INSET,
            SEARCH_Y + SEARCH_TEXT_INSET,
            SEARCH_FIELD_WIDTH - SEARCH_TEXT_INSET,
            FIELD_HEIGHT,
            &mut self.search,
            &look,
        );
        let button_y = SEARCH_Y + LINE;
        if g.button(
            "search-button",
            SEARCH_X + SEARCH_BUTTON_X,
            button_y,
            SEARCH_BUTTON,
        ) || typed.submitted
        {
            self.searched = self.search.text().to_string();
        }
        if g.button(
            "clear-button",
            SEARCH_X + CLEAR_BUTTON_X,
            button_y,
            CLEAR_BUTTON,
        ) {
            self.search.set_text("");
            self.searched.clear();
        }
    }

    /// The rows of the markers of the open file that hold the search words.
    fn rows(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(file) = self.files.get(self.file) else {
            return;
        };
        let editable = file.name == USER_MARKERS;
        let markers = file.markers.clone();
        let shown = world_map::found(&markers, &self.searched);
        let mut removed = None;
        let (x, y, w, h) = LIST_AREA;
        g.scroll_area(("markers", self.file), x, y, w, h, |g| {
            let look = TextLook::unicode(WORDS_FONT, WORDS_HUE);
            for (row, (at, marker)) in shown.iter().enumerate() {
                let top = row as i32 * ROW_HEIGHT;
                g.label(ROW_NAME.0, top, &marker.name, &look.cropped(ROW_NAME.1));
                g.label(ROW_X.0, top, &marker.x.to_string(), &look.cropped(ROW_X.1));
                g.label(ROW_Y.0, top, &marker.y.to_string(), &look.cropped(ROW_Y.1));
                g.label(ROW_COLOR.0, top, &marker.color, &look.cropped(ROW_COLOR.1));
                if editable {
                    if g.button(("edit", *at), EDIT_X, top, EDIT_BUTTON) {
                        cx.open_with(
                            GumpId::one(well_known::USER_MARKER),
                            Box::new(UserMarker::editing(*at, (*marker).clone())),
                        );
                    }
                    if g.button(("remove", *at), REMOVE_X, top, REMOVE_BUTTON) {
                        removed = Some(*at);
                    }
                }
                if g.button(("go", *at), GO_X, top, GO_BUTTON) {
                    show_on_map(cx, marker.x, marker.y);
                }
            }
            shown.len() as i32 * ROW_HEIGHT
        });
        if let Some(at) = removed {
            match world_map::remove_user_marker(&world_map::map_dir(), at) {
                Ok(()) => markers_changed(cx),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }

    /// The buttons of the files at the foot, one for each file.
    fn file_buttons(&mut self, g: &mut Canvas<'_>) {
        g.fill(0, FILES_Y, MANAGER_WIDTH, LINE, LINE_COLOR);
        if self.files.is_empty() {
            return;
        }
        let width = (MANAGER_WIDTH / self.files.len() as i32).max(FILE_BUTTON_LEAST);
        let look = words_look();
        let names: Vec<String> = self.files.iter().map(|file| file.name.clone()).collect();
        for (at, name) in names.iter().enumerate() {
            let x = width * at as i32;
            if g.nice_button(
                ("file", at),
                x,
                FILES_Y,
                width,
                FILES_HEIGHT,
                name,
                &look,
                at == self.file,
            ) {
                self.file = at;
            }
            g.tooltip(name);
            g.fill(x, FILES_Y, LINE, FILES_HEIGHT, LINE_COLOR);
        }
    }
}

impl GumpBody for MarkersManager {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if !self.loaded || cx.take_answer(ANSWER_MANAGER_RELOAD).is_some() {
            self.load();
        }
        g.shade(
            LINE,
            LINE,
            MANAGER_WIDTH,
            MANAGER_HEIGHT,
            DARK_HUE,
            MANAGER_OPACITY,
        );
        g.fill(0, 0, MANAGER_WIDTH, LINE, LINE_COLOR);
        g.fill(0, 0, LINE, MANAGER_HEIGHT, LINE_COLOR);
        g.fill(0, MANAGER_HEIGHT, MANAGER_WIDTH, LINE, LINE_COLOR);
        g.fill(MANAGER_WIDTH, 0, LINE, MANAGER_HEIGHT, LINE_COLOR);
        self.legend_and_search(g);
        self.rows(g, cx);
        self.file_buttons(g);
        if let Some(error) = &self.error {
            let look = TextLook::unicode(WORDS_FONT, ERROR_HUE).bordered();
            g.label(LIST_AREA.0, FILES_Y - ROW_HEIGHT, error, &look);
        }
    }
}

/// The box that adds a marker to the player's own file, or changes one of
/// its markers.
pub struct UserMarker {
    /// The place of the marker in the player's file, when it is changed.
    editing: Option<usize>,
    map: u8,
    icon: String,
    x: TextField,
    y: TextField,
    name: TextField,
    color: usize,
    error: Option<String>,
}

impl UserMarker {
    fn new(editing: Option<usize>, marker: Marker) -> Self {
        let fields = MarkerFields::of(&marker);
        let number = |words: &str| {
            let mut field = TextField::new(words);
            field.numeric = true;
            field.with_max_chars(Some(NUMBER_MAX_CHARS))
        };
        Self {
            editing,
            map: fields.map,
            x: number(&fields.x),
            y: number(&fields.y),
            name: TextField::new(&fields.name).with_max_chars(Some(NAME_MAX_CHARS)),
            color: fields.color,
            icon: fields.icon,
            error: None,
        }
    }

    /// A box that adds a new marker, filled from `marker`.
    pub fn adding(marker: Marker) -> Self {
        Self::new(None, marker)
    }

    /// A box that changes the marker at a place of the player's file.
    pub fn editing(at: usize, marker: Marker) -> Self {
        Self::new(Some(at), marker)
    }

    /// The marker the fields make, when they are right.
    fn marker(&self) -> Option<Marker> {
        MarkerFields {
            map: self.map,
            icon: self.icon.clone(),
            x: self.x.text().to_string(),
            y: self.y.text().to_string(),
            name: self.name.text().to_string(),
            color: self.color,
        }
        .marker()
    }

    /// Writes the marker to the player's file. True when it was written.
    fn keep(&mut self, cx: &mut GumpContext<'_>) -> bool {
        let Some(marker) = self.marker() else {
            return false;
        };
        match world_map::keep_user_marker(&world_map::map_dir(), self.editing, marker) {
            Ok(()) => {
                markers_changed(cx);
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }
}

impl GumpBody for UserMarker {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let look = words_look();
        g.shade(0, 0, MARKER_WIDTH, MARKER_HEIGHT, BLACK_HUE, MARKER_OPACITY);
        let title = if self.editing.is_some() {
            WORDS_EDIT_MARKER
        } else {
            WORDS_ADD
        };
        g.label(MARKER_TITLE.0, MARKER_TITLE.1, title, &look);
        let field_x = FIELDS_X + FIELD_LABEL_ROOM;
        let fields = [
            (WORDS_X, NUMBER_FIELD_WIDTH),
            (WORDS_Y, NUMBER_FIELD_WIDTH),
            (WORDS_NAME, NAME_FIELD_WIDTH),
        ];
        let mut submitted = false;
        for (row, (words, width)) in fields.into_iter().enumerate() {
            let y = FIELDS_Y + FIELDS_STEP * row as i32;
            g.frame(field_x, y, width, FIELD_HEIGHT, FIELD_FRAME);
            let field = match row {
                0 => &mut self.x,
                1 => &mut self.y,
                _ => &mut self.name,
            };
            submitted |= g
                .text_box(
                    ("field", row),
                    field_x,
                    y,
                    width,
                    FIELD_HEIGHT,
                    field,
                    &look,
                )
                .submitted;
            g.label(FIELDS_X, y, words, &look);
        }
        let color_y = FIELDS_Y + FIELDS_STEP * fields.len() as i32;
        g.combobox(
            "color",
            field_x,
            color_y,
            NAME_FIELD_WIDTH,
            &MARKER_COLORS,
            &mut self.color,
        );
        g.label(FIELDS_X, color_y, WORDS_COLOR, &look);
        if let Some(error) = &self.error {
            let error_look = TextLook::unicode(WORDS_FONT, ERROR_HUE).bordered();
            g.label(FIELDS_X, ERROR_Y, error, &error_look);
        }
        let ok_words = if self.editing.is_some() {
            WORDS_EDIT
        } else {
            WORDS_CREATE
        };
        let ok = g.nice_button(
            "ok",
            OK_BUTTON_X,
            MARKER_BUTTONS_Y,
            MARKER_BUTTON_WIDTH,
            FIELD_HEIGHT,
            ok_words,
            &look,
            false,
        );
        if (ok || submitted) && self.keep(cx) {
            cx.close(cx.me);
        }
        if g.nice_button(
            "cancel",
            CANCEL_BUTTON_X,
            MARKER_BUTTONS_Y,
            MARKER_BUTTON_WIDTH,
            FIELD_HEIGHT,
            WORDS_CANCEL,
            &look,
            false,
        ) {
            cx.close(cx.me);
        }
    }
}

/// The box that moves the world map to typed coordinates or a sextant
/// place.
#[derive(Default)]
pub struct LocationGo {
    field: TextField,
    focused: bool,
}

impl LocationGo {
    /// Moves the map to the place the words name. True when they name one.
    fn go(&self, cx: &mut GumpContext<'_>) -> bool {
        match world_map::parse_goto(self.field.text()) {
            Some((x, y)) => {
                show_on_map(cx, x, y);
                true
            }
            None => false,
        }
    }
}

impl GumpBody for LocationGo {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.shade(0, 0, GO_WIDTH, GO_HEIGHT, DARK_HUE, GO_OPACITY);
        let look = TextLook::unicode(WORDS_FONT, WORDS_HUE);
        let words_look = look.wrap((GO_WIDTH - GO_WORDS_ROOM) as u32);
        let words = g.label(GO_WORDS_AT.0, GO_WORDS_AT.1, GO_WORDS, &words_look);
        let field_width = GO_WIDTH - GO_FIELD_ROOM;
        let frame_y = GO_FIELD_TOP + words.y as i32 + GO_FIELD_GAP;
        g.frame(
            GO_FIELD_X,
            frame_y,
            field_width + GO_FIELD_PAD,
            FIELD_HEIGHT,
            FIELD_FRAME,
        );
        let hue = if world_map::parse_goto(self.field.text()).is_some() {
            PLACE_HUE
        } else {
            NO_PLACE_HUE
        };
        let field_look = TextLook::unicode(WORDS_FONT, hue).bordered();
        let (text_x, text_y) = (GO_FIELD_X + GO_TEXT_INSET.0, frame_y + GO_TEXT_INSET.1);
        let typed = g.text_box(
            "location",
            text_x,
            text_y,
            field_width,
            FIELD_HEIGHT,
            &mut self.field,
            &field_look,
        );
        if !self.focused {
            g.focus("location");
            self.focused = true;
        }
        let pressed = g.button(
            "go",
            text_x + field_width + GO_BUTTON_GAP,
            text_y,
            LOCATION_OK,
        );
        g.label(
            text_x - GO_TEXT_INSET.0,
            text_y + GO_EXAMPLES_GAP,
            GO_EXAMPLES,
            &words_look,
        );
        if (pressed || typed.submitted) && self.go(cx) {
            cx.close(cx.me);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_box_fills_its_fields_from_the_marker() {
        let place = UserMarker::adding(Marker {
            map: 1,
            x: 1434,
            y: 1699,
            color: "Blue".into(),
            ..unnamed_marker()
        });
        assert_eq!(place.name.text(), world_map::DEFAULT_MARKER_NAME);
        assert_eq!(MARKER_COLORS[place.color], "blue");
        assert_eq!(
            place.marker().map(|made| (made.x, made.y)),
            Some((1434, 1699))
        );
        let changed = UserMarker::editing(
            2,
            Marker {
                name: "Yew".into(),
                ..unnamed_marker()
            },
        );
        assert_eq!(changed.editing, Some(2));
        assert_eq!(changed.name.text(), "Yew");
        assert!(changed.x.numeric && !changed.name.numeric);
    }
}
