//! The world map gump of the classic client: the
//! facet in a thin frame the player sizes by its corner, turned as the game
//! view unless the player sets it north up, closer or farther by the zoom
//! steps of the wheel. It carries the marker and zone files, the named
//! places of the session, the party, the guild, the mobiles and the houses,
//! and the coordinates. A right click opens its menu of options; a double
//! click keeps it over the other gumps; Ctrl+click adds a marker there; in
//! free view a drag moves the view; a click answers a target of a place when
//! the World Map page lets it. The drawing of the land and the marks is
//! `map_view`'s, shared with the Modern map panel.

use super::canvas::Canvas;
use super::context_menu::{ContextMenu, MenuLine};
use super::map_markers::{LocationGo, UserMarker};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use crate::view::WatchFrame;
use crate::window::control::Act;
use crate::window::map_view::{self, Lay, MapFilesCache, MapPictures, MarkLook, Marks};
use crate::window::model::world_map::{self, Marker};
use crate::window::settings::{Profile, WorldMapOptions};
use crate::window::theme;
use eframe::egui::{Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Sense, Vec2};

pub const WORLD_MAP: GumpKind = GumpKind {
    id: well_known::WORLD_MAP,
    rules: GumpRules {
        right_click_closes: false,
        resizable: Some((FIRST_SIZE, LEAST_SIZE)),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(WorldMap::default()),
};

/// The answers other gumps give the world map: a place to go to, and that
/// the marker files changed.
pub const ANSWER_GO_X: &str = "world_map:go_x";
pub const ANSWER_GO_Y: &str = "world_map:go_y";
pub const ANSWER_RELOAD: &str = "world_map:reload";

const FIRST_SIZE: Vec2 = Vec2::new(400.0, 400.0);
const LEAST_SIZE: Vec2 = Vec2::new(100.0, 100.0);
/// The thin frame, tiled round the map.
const BORDER: i32 = 4;
const TOP_BORDER: u16 = 0x0A8C;
const SIDE_BORDER: u16 = 0x0A8D;
const NO_HUE: u16 = 0;
/// The sizes of the six font styles of the marker names.
const MARKER_FONT_SIZES: [f32; 6] = [11.0, 12.0, 13.0, 14.0, 16.0, 18.0];
const COORDINATES_SIZE: f32 = 14.0;
const COORDINATES_AT: Vec2 = Vec2::new(5.0, 5.0);
const MOUSE_COORDINATES_ROOM: f32 = 15.0;
/// The facets the free view may show, as the reference client numbers them.
const FACET_COUNT: u8 = 6;
const HALF: f32 = 2.0;
// The colors of the marks, as the reference client draws them.
const GRID: Color32 = Color32::from_rgba_premultiplied(56, 56, 56, 56);
const MULTI: Color32 = Color32::from_rgb(169, 169, 169);
const PARTY: Color32 = Color32::YELLOW;
const GUILD: Color32 = Color32::from_rgb(50, 205, 50);
const GOING_TO: Color32 = Color32::from_rgb(127, 255, 212);
const BAR_EMPTY: Color32 = Color32::RED;
const BAR_FULL: Color32 = Color32::from_rgb(100, 149, 237);
const BAR_EDGE: f32 = 1.0;

const WORDS_BUILDING: &str = "Please wait, I'm making the map file...";
const WORDS_MARKERS: &str = "Map Marker Options";
const WORDS_RELOAD_MARKERS: &str = "Reload markers";
const WORDS_FONT_STYLE: &str = "Font Style";
const WORDS_STYLE: &str = "Style";
const WORDS_SHOW_MARKERS: &str = "Show all markers";
const WORDS_MARKER_NAMES: &str = "Show marker names";
const WORDS_SHOW_HIDE: &str = "Show/Hide";
const WORDS_NO_FILES: &str = "No map files";
const WORDS_ZONES: &str = "Grid and Zone Options";
const WORDS_GRID: &str = "Show/Hide 8x8 Grid if Zoomed";
const WORDS_RELOAD_ZONES: &str = "Reload Map Zones";
const WORDS_NO_ZONES: &str = "<none>";
const WORDS_NAMES_BARS: &str = "Names & Healthbars";
const WORDS_YOUR_NAME: &str = "Show your name";
const WORDS_YOUR_BAR: &str = "Show your healthbar";
const WORDS_GROUP_NAME: &str = "Show group name";
const WORDS_GROUP_BAR: &str = "Show group healthbar";
const WORDS_GO_TO: &str = "Go to location";
const WORDS_FLIP: &str = "Flip map";
const WORDS_ON_TOP: &str = "Keep map on top";
const WORDS_FREE_VIEW: &str = "Free view";
const WORDS_FACET: &str = "Map";
const WORDS_PARTY: &str = "Show party members";
const WORDS_MOBILES: &str = "Show mobiles";
const WORDS_MULTIS: &str = "Show houses/boats";
const WORDS_COORDINATES: &str = "Show your coordinates";
const WORDS_SEXTANT: &str = "Show sextant coordinates";
const WORDS_MOUSE: &str = "Show mouse coordinates";
const WORDS_TARGET: &str = "Allow Positional Targeting";
const WORDS_MANAGER: &str = "Markers Manager";
const WORDS_ON_PLAYER: &str = "Add Marker on Player";
const WORDS_RESET: &str = "Reset maps cache";
const WORDS_CLOSE: &str = "Save & Close";

/// What a line of the menu does.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Choice {
    ReloadMarkers,
    FontStyle(u8),
    ShowMarkers,
    MarkerNames,
    MarkerFile(String),
    Grid,
    ReloadZones,
    ZoneFile(String),
    YourName,
    YourBar,
    GroupName,
    GroupBar,
    GoTo,
    Flip,
    OnTop,
    FreeView,
    Facet(u8),
    Party,
    Mobiles,
    Multis,
    Coordinates,
    Sextant,
    Mouse,
    Target,
    Manager,
    OnPlayer,
    Reset,
    Close,
}

/// One line of the menu and what it does.
struct Entry {
    line: MenuLine,
    choice: Option<Choice>,
    list: Vec<Entry>,
}

impl Entry {
    fn toggle(words: &str, on: bool, choice: Choice) -> Self {
        Self {
            line: MenuLine::tick(words, on),
            choice: Some(choice),
            list: Vec::new(),
        }
    }

    fn plain(words: &str, choice: Choice) -> Self {
        Self {
            line: MenuLine::plain(words),
            choice: Some(choice),
            list: Vec::new(),
        }
    }

    fn list(words: &str, list: Vec<Entry>) -> Self {
        Self {
            line: MenuLine::list(words, Vec::new()),
            choice: None,
            list,
        }
    }

    fn gap() -> Self {
        Self {
            line: MenuLine::gap(),
            choice: None,
            list: Vec::new(),
        }
    }

    fn words(words: &str) -> Self {
        Self {
            line: MenuLine::plain(words),
            choice: None,
            list: Vec::new(),
        }
    }

    fn menu_line(&self) -> MenuLine {
        MenuLine {
            lines: self.list.iter().map(Entry::menu_line).collect(),
            ..self.line.clone()
        }
    }
}

/// The choice of the line at a path of the menu.
fn chosen(entries: &[Entry], path: &[usize]) -> Option<Choice> {
    let (first, rest) = path.split_first()?;
    let entry = entries.get(*first)?;
    if rest.is_empty() {
        entry.choice.clone()
    } else {
        chosen(&entry.list, rest)
    }
}

/// The toggles of one list of files: each shows while it is not hidden.
fn file_toggles(
    names: &[String],
    hidden: &[String],
    none: &str,
    choice: fn(String) -> Choice,
) -> Vec<Entry> {
    if names.is_empty() {
        return vec![Entry::words(none)];
    }
    names
        .iter()
        .map(|name| {
            Entry::toggle(
                &format!("{WORDS_SHOW_HIDE} '{name}'"),
                !hidden.contains(name),
                choice(name.clone()),
            )
        })
        .collect()
}

/// The menu of the map, as the reference client builds it.
fn menu(options: &WorldMapOptions, marker_files: &[String], zone_files: &[String]) -> Vec<Entry> {
    let styles = (1..=MARKER_FONT_SIZES.len() as u8)
        .map(|style| {
            Entry::toggle(
                &format!("{WORDS_STYLE} {style}"),
                options.marker_font_style == style,
                Choice::FontStyle(style),
            )
        })
        .collect();
    let mut markers = vec![
        Entry::plain(WORDS_RELOAD_MARKERS, Choice::ReloadMarkers),
        Entry::list(WORDS_FONT_STYLE, styles),
        Entry::toggle(
            WORDS_SHOW_MARKERS,
            options.show_markers,
            Choice::ShowMarkers,
        ),
        Entry::gap(),
        Entry::toggle(
            WORDS_MARKER_NAMES,
            options.show_marker_names,
            Choice::MarkerNames,
        ),
        Entry::gap(),
    ];
    markers.extend(file_toggles(
        marker_files,
        &options.hidden_marker_files,
        WORDS_NO_FILES,
        Choice::MarkerFile,
    ));
    let mut zones = vec![
        Entry::toggle(WORDS_GRID, options.grid_when_zoomed, Choice::Grid),
        Entry::plain(WORDS_RELOAD_ZONES, Choice::ReloadZones),
        Entry::gap(),
    ];
    zones.extend(file_toggles(
        zone_files,
        &options.hidden_zone_files,
        WORDS_NO_ZONES,
        Choice::ZoneFile,
    ));
    let names = vec![
        Entry::toggle(WORDS_YOUR_NAME, options.show_player_name, Choice::YourName),
        Entry::toggle(WORDS_YOUR_BAR, options.show_player_bar, Choice::YourBar),
        Entry::toggle(
            WORDS_GROUP_NAME,
            options.show_group_names,
            Choice::GroupName,
        ),
        Entry::toggle(WORDS_GROUP_BAR, options.show_group_bars, Choice::GroupBar),
    ];
    let mut free_view = vec![Entry::toggle(
        WORDS_FREE_VIEW,
        options.free_view,
        Choice::FreeView,
    )];
    free_view.extend(
        (0..FACET_COUNT)
            .map(|facet| Entry::plain(&format!("{WORDS_FACET} {facet}"), Choice::Facet(facet))),
    );
    vec![
        Entry::list(WORDS_MARKERS, markers),
        Entry::list(WORDS_ZONES, zones),
        Entry::list(WORDS_NAMES_BARS, names),
        Entry::gap(),
        Entry::plain(WORDS_GO_TO, Choice::GoTo),
        Entry::toggle(WORDS_FLIP, options.flip_map, Choice::Flip),
        Entry::toggle(WORDS_ON_TOP, options.always_on_top, Choice::OnTop),
        Entry::list(WORDS_FREE_VIEW, free_view),
        Entry::gap(),
        Entry::toggle(WORDS_PARTY, options.show_party, Choice::Party),
        Entry::toggle(WORDS_MOBILES, options.show_mobiles, Choice::Mobiles),
        Entry::toggle(WORDS_MULTIS, options.show_multis, Choice::Multis),
        Entry::toggle(
            WORDS_COORDINATES,
            options.show_coordinates,
            Choice::Coordinates,
        ),
        Entry::toggle(WORDS_SEXTANT, options.sextant_coordinates, Choice::Sextant),
        Entry::toggle(WORDS_MOUSE, options.show_mouse_coordinates, Choice::Mouse),
        Entry::toggle(
            WORDS_TARGET,
            options.allow_positional_target,
            Choice::Target,
        ),
        Entry::gap(),
        Entry::plain(WORDS_MANAGER, Choice::Manager),
        Entry::plain(WORDS_ON_PLAYER, Choice::OnPlayer),
        Entry::gap(),
        Entry::plain(WORDS_RESET, Choice::Reset),
        Entry::plain(WORDS_CLOSE, Choice::Close),
    ]
}

/// The words of the character's place at the top of the map.
fn coordinates_words(frame: &WatchFrame, options: &WorldMapOptions) -> String {
    let words = format!(
        "{}, {} ({}) [{}]",
        frame.x, frame.y, frame.z, options.zoom_step
    );
    sextant_line(words, options, frame.map, frame.x, frame.y)
}

/// Words with the sextant place under them, when the page asks for it.
fn sextant_line(words: String, options: &WorldMapOptions, map: u8, x: u16, y: u16) -> String {
    match options
        .sextant_coordinates
        .then(|| world_map::sextant(map, x, y))
        .flatten()
    {
        Some(sextant) => format!("{words}\n{sextant}"),
        None => words,
    }
}

/// A small health bar as the classic world map draws it.
fn classic_bar(painter: &Painter, track: Rect, share: f32) {
    painter.rect_filled(track.expand(BAR_EDGE), CornerRadius::ZERO, Color32::BLACK);
    painter.rect_filled(track, CornerRadius::ZERO, BAR_EMPTY);
    let mut part = track;
    part.set_width(track.width() * share.clamp(0.0, 1.0));
    painter.rect_filled(part, CornerRadius::ZERO, BAR_FULL);
}

fn red(_notoriety: u8) -> Color32 {
    Color32::RED
}

/// The marks in the colors of the classic client.
fn classic_look(options: &WorldMapOptions) -> MarkLook {
    let style = usize::from(options.marker_font_style.max(1)) - 1;
    let size = MARKER_FONT_SIZES[style.min(MARKER_FONT_SIZES.len() - 1)];
    MarkLook {
        font: FontId::proportional(size),
        shadowed: true,
        square_dots: true,
        marker: Color32::WHITE,
        waypoint: Color32::YELLOW,
        multi: MULTI,
        party: PARTY,
        guild: GUILD,
        goal: GOING_TO,
        looking: GOING_TO,
        me: Color32::WHITE,
        grid: GRID,
        mobile: red,
        health_bar: classic_bar,
    }
}

/// What the world map keeps between frames.
#[derive(Default)]
pub struct WorldMap {
    pictures: MapPictures,
    files: MapFilesCache,
    /// The middle of the view while the player moves it himself.
    view: Option<Vec2>,
    /// The facet the free view shows, when it is not the character's.
    facet: Option<u8>,
    /// The middle of the view when a drag began.
    drag_from: Option<Vec2>,
    /// The place the player went to.
    going_to: Option<(u16, u16)>,
    menu: ContextMenu,
    /// The marker and zone files, hidden or not, read when the menu opens.
    file_names: (Vec<String>, Vec<String>),
}

impl WorldMap {
    /// The place in the middle of the view.
    fn middle(&self, frame: &WatchFrame, options: &WorldMapOptions) -> Vec2 {
        match self.view.filter(|_| options.free_view) {
            Some(view) => view,
            None => Vec2::new(f32::from(frame.x), f32::from(frame.y)),
        }
    }

    /// How the view lays the tiles on the field.
    fn lay(&self, field: Rect, middle: Vec2, options: &WorldMapOptions, scale: f32) -> Lay {
        let tile = world_map::zoom_points(options.zoom_step) * scale;
        if options.flip_map {
            Lay::Turned {
                center: field.center(),
                from: middle,
                unit: tile / std::f32::consts::SQRT_2,
            }
        } else {
            Lay::NorthUp {
                center: field.center(),
                middle,
                scale: tile,
            }
        }
    }

    fn take_answers(&mut self, cx: &mut GumpContext<'_>) {
        if cx.take_answer(ANSWER_RELOAD).is_some() {
            self.files.reload();
        }
        if let (Some(x), Some(y)) = (cx.take_answer(ANSWER_GO_X), cx.take_answer(ANSWER_GO_Y)) {
            self.go_to(x, y, cx);
        }
    }

    /// Moves the view to a place, in free view, as the reference client goes to a marker.
    fn go_to(&mut self, x: u16, y: u16, cx: &mut GumpContext<'_>) {
        if !cx.profile.world_map.free_view {
            cx.profile.world_map.free_view = true;
            cx.profile_changed();
        }
        self.view = Some(Vec2::new(f32::from(x), f32::from(y)));
        self.going_to = Some((x, y));
    }

    fn pick(&mut self, choice: Choice, frame: &WatchFrame, cx: &mut GumpContext<'_>) {
        let options = &mut cx.profile.world_map;
        match choice {
            Choice::ReloadMarkers | Choice::ReloadZones => self.files.reload(),
            Choice::FontStyle(style) => options.marker_font_style = style,
            Choice::ShowMarkers => options.show_markers = !options.show_markers,
            Choice::MarkerNames => options.show_marker_names = !options.show_marker_names,
            Choice::MarkerFile(name) => {
                world_map::flip_hidden(&mut options.hidden_marker_files, name);
            }
            Choice::Grid => options.grid_when_zoomed = !options.grid_when_zoomed,
            Choice::ZoneFile(name) => world_map::flip_hidden(&mut options.hidden_zone_files, name),
            Choice::YourName => options.show_player_name = !options.show_player_name,
            Choice::YourBar => options.show_player_bar = !options.show_player_bar,
            Choice::GroupName => options.show_group_names = !options.show_group_names,
            Choice::GroupBar => options.show_group_bars = !options.show_group_bars,
            Choice::Flip => options.flip_map = !options.flip_map,
            Choice::OnTop => options.always_on_top = !options.always_on_top,
            Choice::FreeView => {
                options.free_view = !options.free_view;
                if !options.free_view {
                    self.facet = None;
                }
            }
            Choice::Facet(facet) => {
                options.free_view = true;
                self.facet = Some(facet);
                let (width, height) = world_map::facet_size(facet);
                self.view = Some(Vec2::new(f32::from(width), f32::from(height)) / HALF);
            }
            Choice::Party => options.show_party = !options.show_party,
            Choice::Mobiles => options.show_mobiles = !options.show_mobiles,
            Choice::Multis => options.show_multis = !options.show_multis,
            Choice::Coordinates => options.show_coordinates = !options.show_coordinates,
            Choice::Sextant => options.sextant_coordinates = !options.sextant_coordinates,
            Choice::Mouse => options.show_mouse_coordinates = !options.show_mouse_coordinates,
            Choice::Target => {
                options.allow_positional_target = !options.allow_positional_target;
            }
            Choice::GoTo => {
                cx.open_with(
                    GumpId::one(well_known::LOCATION_GO),
                    Box::new(LocationGo::default()),
                );
                return;
            }
            Choice::Manager => {
                cx.open(GumpId::one(well_known::MARKERS_MANAGER));
                return;
            }
            Choice::OnPlayer => {
                cx.open_with(
                    GumpId::one(well_known::USER_MARKER),
                    Box::new(UserMarker::adding(world_map::marker_on_player(frame))),
                );
                return;
            }
            Choice::Reset => {
                self.pictures = MapPictures::default();
                return;
            }
            Choice::Close => {
                cx.close(cx.me);
                return;
            }
        }
        cx.profile_changed();
    }

    /// The clicks on the field: a drag of the free view, a target of a
    /// place, a marker, and the double click that keeps the map on top.
    fn clicks(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        field: (i32, i32, i32, i32),
        lay: Lay,
        middle: Vec2,
    ) {
        let (x, y, w, h) = field;
        let free = cx.profile.world_map.free_view;
        let (click, double) = if free {
            g.click_area("field", x, y, w, h);
            let rect = g.area(x, y, Vec2::new(w as f32, h as f32));
            let response = g.ui().interact(
                rect,
                Id::new(("classic-world-map-field", cx.me)),
                Sense::click_and_drag(),
            );
            if response.drag_started() {
                self.drag_from = Some(middle);
            }
            if let (Some(from), Some(now), Some(origin)) = (
                self.drag_from.filter(|_| response.dragged()),
                response.interact_pointer_pos(),
                g.ui().input(|i| i.pointer.press_origin()),
            ) {
                self.view = Some(from + lay.tile(origin) - lay.tile(now));
            }
            if response.drag_stopped() {
                self.drag_from = None;
            }
            let at = response
                .clicked()
                .then(|| response.interact_pointer_pos())
                .flatten();
            (at, response.double_clicked())
        } else {
            let at = g.body_click().map(|at| g.at(at.x as i32, at.y as i32));
            (at, g.body_double_click())
        };
        if double {
            cx.profile.world_map.always_on_top = !cx.profile.world_map.always_on_top;
            cx.profile_changed();
            return;
        }
        let Some(at) = click else {
            return;
        };
        let (tile_x, tile_y) = map_view::whole_tile(lay.tile(at));
        let frame = cx.frame;
        let ctrl = g.ui().input(|i| i.modifiers.ctrl);
        if ctrl {
            let marker = Marker {
                name: String::new(),
                map: self.facet.unwrap_or(frame.map),
                x: tile_x,
                y: tile_y,
                icon: String::new(),
                color: world_map::MARKER_COLORS[0].to_string(),
            };
            cx.open_with(
                GumpId::one(well_known::USER_MARKER),
                Box::new(UserMarker::adding(marker)),
            );
        } else if world_map::targets_ground(frame, &cx.profile.world_map) {
            let z = g.scene.land_z(frame.map, tile_x, tile_y).unwrap_or(frame.z);
            cx.act(Act::TargetGround {
                x: tile_x,
                y: tile_y,
                z,
            });
        }
    }
}

impl GumpBody for WorldMap {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        self.take_answers(cx);
        let frame = cx.frame;
        let size = g.size().unwrap_or(FIRST_SIZE);
        let (width, height) = (size.x as i32, size.y as i32);
        if !cx.profile.world_map.always_on_top {
            g.pic_tiled(0, 0, width, BORDER, TOP_BORDER, NO_HUE);
            g.pic_tiled(0, height - BORDER, width, BORDER, TOP_BORDER, NO_HUE);
            g.pic_tiled(0, 0, BORDER, height, SIDE_BORDER, NO_HUE);
            g.pic_tiled(width - BORDER, 0, BORDER, height, SIDE_BORDER, NO_HUE);
        }
        let inner = (BORDER, BORDER, width - BORDER * 2, height - BORDER * 2);
        g.fill(inner.0, inner.1, inner.2, inner.3, Color32::BLACK);
        let field = g.area(inner.0, inner.1, Vec2::new(inner.2 as f32, inner.3 as f32));
        let scale = cx.profile.video.ui_scale;
        let turns = g.wheel_turns(inner.0, inner.1, inner.2, inner.3);
        let step = world_map::zoom_step(cx.profile.world_map.zoom_step, turns);
        if step != cx.profile.world_map.zoom_step {
            cx.profile.world_map.zoom_step = step;
            cx.profile_changed();
        }
        let options = cx.profile.world_map.clone();
        let facet = self
            .facet
            .filter(|_| options.free_view)
            .unwrap_or(frame.map);
        let middle = self.middle(frame, &options);
        let lay = self.lay(field, middle, &options, scale);
        let painter = g.ui().painter().with_clip_rect(field);
        let ctx = g.ctx().clone();
        if self.pictures.grow_world(&ctx, g.scene, facet) {
            ctx.request_repaint();
        }
        let near = map_view::whole_tile(middle);
        let has_world = self.pictures.draw_world(&painter, lay);
        if self.pictures.make_near(&ctx, g.scene, facet, near) {
            self.pictures.draw_near(&painter, lay);
        } else if !has_world {
            painter.text(
                field.center(),
                Align2::CENTER_CENTER,
                WORDS_BUILDING,
                FontId::proportional(COORDINATES_SIZE),
                Color32::WHITE,
            );
        }
        let session = map_view::session_markers(cx.readings, cx.profile, facet);
        let marks = Marks {
            frame,
            map: facet,
            profile: cx.profile,
            files: self.files.get(cx.profile),
            session: &session,
            looking_at: self.going_to,
        };
        map_view::overlays(&painter, field, lay, &marks, &classic_look(&options));
        let words_font = FontId::proportional(COORDINATES_SIZE);
        if options.show_coordinates {
            theme::shadowed_text(
                &painter,
                field.min + COORDINATES_AT * scale,
                Align2::LEFT_TOP,
                &coordinates_words(frame, &options),
                words_font.clone(),
                Color32::WHITE,
            );
        }
        let pointer = g.ctx().pointer_hover_pos().filter(|at| field.contains(*at));
        if let (true, Some(at)) = (options.show_mouse_coordinates, pointer) {
            let (x, y) = map_view::whole_tile(lay.tile(at));
            let words = sextant_line(format!("{x} {y}"), &options, facet, x, y);
            let corner = Pos2::new(
                field.left() + COORDINATES_AT.x * scale,
                field.bottom() - MOUSE_COORDINATES_ROOM * scale,
            );
            theme::shadowed_text(
                &painter,
                corner,
                Align2::LEFT_BOTTOM,
                &words,
                words_font,
                Color32::WHITE,
            );
        }
        self.clicks(g, cx, inner, lay, middle);
        if g.right_click() {
            if let Some(at) = g.ctx().pointer_latest_pos() {
                self.file_names = world_map::file_names(&world_map::map_dir());
                self.menu.open_at(at);
            }
        }
        if self.menu.is_open() {
            let entries = menu(
                &cx.profile.world_map,
                &self.file_names.0,
                &self.file_names.1,
            );
            let lines: Vec<MenuLine> = entries.iter().map(Entry::menu_line).collect();
            if let Some(path) = self.menu.show(g, cx.me, scale, &lines) {
                if let Some(choice) = chosen(&entries, &path) {
                    self.pick(choice, frame, cx);
                }
            }
        }
    }

    fn on_top(&self, profile: &Profile) -> bool {
        profile.world_map.always_on_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::testing::draw_frames;

    #[test]
    fn the_menu_picks_its_options_and_toggles_files() {
        let options = WorldMapOptions::default();
        let entries = menu(&options, &["towns".into()], &[]);
        assert_eq!(chosen(&entries, &[4]), Some(Choice::GoTo));
        assert_eq!(
            chosen(&entries, &[0, 6]),
            Some(Choice::MarkerFile("towns".into()))
        );
        assert_eq!(chosen(&entries, &[0, 1, 2]), Some(Choice::FontStyle(3)));
        assert_eq!(chosen(&entries, &[1, 3]), None, "no zone file is a note");
        assert_eq!(chosen(&entries, &[3]), None, "a gap does nothing");
        let lines: Vec<MenuLine> = entries.iter().map(Entry::menu_line).collect();
        assert_eq!(lines[0].lines[6].ticked, Some(true));
    }

    #[test]
    fn the_coordinates_carry_the_height_the_zoom_and_the_sextant() {
        let frame = WatchFrame {
            x: 1434,
            y: 1699,
            z: 5,
            map: 1,
            ..WatchFrame::default()
        };
        let mut options = WorldMapOptions::default();
        assert_eq!(coordinates_words(&frame, &options), "1434, 1699 (5) [4]");
        options.sextant_coordinates = true;
        assert!(coordinates_words(&frame, &options).ends_with("6o 35'S, 7o 48'E"));
    }

    #[test]
    fn the_view_follows_the_character_until_the_player_frees_it() {
        let frame = WatchFrame {
            x: 100,
            y: 200,
            ..WatchFrame::default()
        };
        let mut map = WorldMap {
            view: Some(Vec2::new(5.0, 6.0)),
            ..WorldMap::default()
        };
        let mut options = WorldMapOptions::default();
        assert_eq!(map.middle(&frame, &options), Vec2::new(100.0, 200.0));
        options.free_view = true;
        assert_eq!(map.middle(&frame, &options), Vec2::new(5.0, 6.0));
        map.view = None;
        assert_eq!(map.middle(&frame, &options), Vec2::new(100.0, 200.0));
        let field = Rect::from_min_size(Pos2::ZERO, Vec2::splat(200.0));
        options.flip_map = false;
        let lay = map.lay(field, Vec2::new(100.0, 200.0), &options, 1.0);
        assert!((lay.tile_points() - world_map::ZOOMS[4]).abs() < 0.001);
        options.flip_map = true;
        let turned = map.lay(field, Vec2::new(100.0, 200.0), &options, 1.0);
        let east = turned.screen(101.0, 200.0) - turned.screen(100.0, 200.0);
        assert!(
            (east.length() - world_map::ZOOMS[4]).abs() < 0.001,
            "a tile keeps its size turned"
        );
    }

    #[test]
    fn the_map_draws_with_the_client_files() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        manager.open(GumpId::one(well_known::WORLD_MAP), &mut profile);
        let frame = WatchFrame {
            x: 1434,
            y: 1699,
            map: 1,
            ..WatchFrame::default()
        };
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&GumpId::one(well_known::WORLD_MAP)));
    }
}
