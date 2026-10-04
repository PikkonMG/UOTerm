//! The world map: the size of each facet, sextant coordinates, the go-to
//! box, the marker files and zone files of the player, and where the party
//! and the guild stand.
//!
//! Marker files and zone files lie in the `map` folder of the config
//! folder, in the formats the reference client reads: `.csv` and `.usr` lines of
//! `x,y,map,name,icon,color,zoom`, UO Auto Map `.map`, Ultima Mapper `.lua`,
//! and `*.zones.json`. The host reads and writes the files; their text is
//! read and made here. The markers of the player's own file are listed,
//! searched, added, changed and removed here for the markers manager of
//! each style, and the zoom steps of the world map live here.

use crate::frame::WatchFrame;
use crate::settings::WorldMapOptions;
/// The zoom steps of the world map: points for each tile.
pub use crate::settings::MAP_ZOOMS as ZOOMS;
use serde::Deserialize;
use uoterm_protocol::types::TARGET_GROUND;
use uoterm_world::landmarks::Landmarks;

/// The width and the height of each facet, in tiles: Felucca, Trammel, Ilshenar, Malas,
/// Tokuno, Ter Mur.
const FACET_SIZES: [(u16, u16); 6] = [
    (7168, 4096),
    (7168, 4096),
    (2304, 1600),
    (2560, 2048),
    (1448, 1448),
    (1280, 4096),
];
/// Sextant coordinates count from Lord British's throne, on a world 5120
/// by 4096 tiles (ServUO `Sextant`); the lost lands have their own center.
const SEXTANT_CENTER: (i32, i32) = (1323, 1624);
const SEXTANT_WORLD: (i32, i32) = (5120, 4096);
const LOST_LANDS_CENTER: (i32, i32) = (5936, 3112);
const LOST_LANDS_FROM: (i32, i32) = (5120, 2304);
const LOST_LANDS_TO: (i32, i32) = (6144, 4096);
const FELUCCA: u8 = 0;
const TRAMMEL: u8 = 1;
const DEGREES: f64 = 360.0;
const HALF_TURN: f64 = 180.0;
const MINUTES: f64 = 60.0;

/// The marker file the player writes, by its name with no extension.
pub const USER_MARKERS: &str = "user-markers";
/// The extension of the marker file the player writes.
pub const USER_MARKERS_EXTENSION: &str = "usr";
const CSV_EXTENSIONS: [&str; 2] = ["csv", USER_MARKERS_EXTENSION];
const LANDMARK_EXTENSIONS: [&str; 2] = ["map", "lua"];
const ZONES_SUFFIX: &str = ".zones.json";
const EXTENSION_MARK: char = '.';
const CSV_SPLIT: char = ',';
const CSV_LINE_END: &str = "\n";
const CSV_FIELDS_MIN: usize = 4;
/// The color of a marker the player adds with no color of his own.
pub const NEW_MARKER_COLOR: &str = "yellow";
/// The color words a marker may take, as the reference client offers them.
pub const MARKER_COLORS: [&str; 8] = [
    "none", "red", "green", "blue", "purple", "black", "yellow", "white",
];
const NEW_MARKER_ZOOM: u8 = 3;
/// The name a new marker box starts with.
pub const DEFAULT_MARKER_NAME: &str = "MarkerName";
/// A marker the player adds where he stands is blue, as in the reference client.
pub const ON_PLAYER_COLOR: &str = "blue";
const ICON_FIELD: usize = 4;
const COLOR_FIELD: usize = 5;

/// The size of a facet. A facet this list does not know takes the size of
/// the largest.
pub fn facet_size(map: u8) -> (u16, u16) {
    FACET_SIZES
        .get(usize::from(map))
        .copied()
        .unwrap_or(FACET_SIZES[0])
}

/// The place in sextant words, as "12o 34'N, 56o 7'E". None where a
/// sextant does not work.
pub fn sextant(map: u8, x: u16, y: u16) -> Option<String> {
    let (x, y) = (i32::from(x), i32::from(y));
    let (width, height) = facet_size(map);
    let old_world = map == FELUCCA || map == TRAMMEL;
    let center = if !old_world || (x < SEXTANT_WORLD.0 && y < SEXTANT_WORLD.1) {
        (x < i32::from(width) && y < i32::from(height)).then_some(SEXTANT_CENTER)?
    } else if (LOST_LANDS_FROM.0..LOST_LANDS_TO.0).contains(&x)
        && (LOST_LANDS_FROM.1..LOST_LANDS_TO.1).contains(&y)
    {
        LOST_LANDS_CENTER
    } else {
        return None;
    };
    let degrees = |from: i32, center: i32, world: i32| {
        let mut angle = f64::from((from - center) * DEGREES as i32) / f64::from(world);
        if angle > HALF_TURN {
            angle = -HALF_TURN + angle % HALF_TURN;
        }
        angle
    };
    let longitude = degrees(x, center.0, SEXTANT_WORLD.0);
    let latitude = degrees(y, center.1, SEXTANT_WORLD.1);
    let words = |angle: f64| {
        let whole = angle.abs();
        (whole.trunc() as i32, ((whole % 1.0) * MINUTES) as i32)
    };
    let (lat, lat_min) = words(latitude);
    let (long, long_min) = words(longitude);
    let south = if latitude >= 0.0 { 'S' } else { 'N' };
    let east = if longitude >= 0.0 { 'E' } else { 'W' };
    Some(format!(
        "{lat}o {lat_min}'{south}, {long}o {long_min}'{east}"
    ))
}

/// The tile a sextant place names, on the old world's grid.
fn from_sextant(lat: f64, south: bool, long: f64, east: bool) -> (u16, u16) {
    let lat = if south { lat } else { DEGREES - lat };
    let long = if east { long } else { DEGREES - long };
    let wrap = |at: f64, world: i32| {
        let at = at as i32;
        (at.rem_euclid(world)) as u16
    };
    (
        wrap(
            f64::from(SEXTANT_CENTER.0) + long * f64::from(SEXTANT_WORLD.0) / DEGREES,
            SEXTANT_WORLD.0,
        ),
        wrap(
            f64::from(SEXTANT_CENTER.1) + lat * f64::from(SEXTANT_WORLD.1) / DEGREES,
            SEXTANT_WORLD.1,
        ),
    )
}

/// The tile the words of the go-to box name: "x y", "x, y", "x:y", or a
/// sextant place such as "12o 34'N, 56o 7'E".
pub fn parse_goto(words: &str) -> Option<(u16, u16)> {
    let upper = words.to_uppercase();
    if upper.contains('N') || upper.contains('S') {
        let numbers: Vec<f64> = upper
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|piece| piece.parse().ok())
            .collect();
        let [lat, lat_min, long, long_min] = numbers.as_slice() else {
            return None;
        };
        let south = upper.contains('S');
        let east = upper.contains('E');
        return Some(from_sextant(
            lat + lat_min / MINUTES,
            south,
            long + long_min / MINUTES,
            east,
        ));
    }
    let numbers: Vec<u16> = words
        .split(|c: char| c == ',' || c == ':' || c.is_whitespace())
        .filter(|piece| !piece.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match numbers.as_slice() {
        [x, y] => Some((*x, *y)),
        _ => None,
    }
}

/// One marked place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub name: String,
    pub map: u8,
    pub x: u16,
    pub y: u16,
    /// The name of the icon of the marker, kept as the file wrote it.
    pub icon: String,
    /// The color word of the file: red, green, blue, purple, black,
    /// yellow, white or none.
    pub color: String,
}

/// The color of a marker or a zone by its word, as the reference client names them.
/// None for "none", which draws nothing.
pub fn color_of(word: &str) -> Option<[u8; 3]> {
    Some(match word.trim().to_lowercase().as_str() {
        "red" => [255, 0, 0],
        "green" => [0, 128, 0],
        "blue" => [0, 0, 255],
        "purple" => [128, 0, 128],
        "black" => [0, 0, 0],
        "yellow" => [255, 255, 0],
        "none" => return None,
        _ => [255, 255, 255],
    })
}

/// The markers of a CSV marker file. A line that does not read is skipped.
pub fn parse_markers_csv(text: &str) -> Vec<Marker> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(CSV_SPLIT).map(str::trim).collect();
            if fields.len() < CSV_FIELDS_MIN {
                return None;
            }
            Some(Marker {
                x: fields[0].parse().ok()?,
                y: fields[1].parse().ok()?,
                map: fields[2].parse().ok()?,
                name: fields[3].to_string(),
                icon: fields
                    .get(ICON_FIELD)
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
                color: fields
                    .get(COLOR_FIELD)
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

/// One file of markers, by its name with no extension.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerFile {
    pub name: String,
    pub markers: Vec<Marker>,
}

/// The kind of a file of the map folder, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapFile<'a> {
    /// A marker file of lines of `x,y,map,name,icon,color,zoom`.
    Csv { stem: &'a str },
    /// A UO Auto Map or Ultima Mapper marker file.
    Landmarks { stem: &'a str },
    /// A zone file.
    Zones { stem: &'a str },
}

impl<'a> MapFile<'a> {
    /// The kind of a file by its name. None for a file the map does not
    /// read.
    pub fn of(file_name: &'a str) -> Option<Self> {
        if let Some(stem) = file_name.strip_suffix(ZONES_SUFFIX) {
            return Some(Self::Zones { stem });
        }
        let (stem, extension) = file_name.rsplit_once(EXTENSION_MARK)?;
        let is = |known: &[&str]| known.iter().any(|e| extension.eq_ignore_ascii_case(e));
        if stem.is_empty() {
            None
        } else if is(&CSV_EXTENSIONS) {
            Some(Self::Csv { stem })
        } else if is(&LANDMARK_EXTENSIONS) {
            Some(Self::Landmarks { stem })
        } else {
            None
        }
    }

    /// The name of the file with no extension, as the menus show it.
    pub fn stem(self) -> &'a str {
        match self {
            Self::Csv { stem } | Self::Landmarks { stem } | Self::Zones { stem } => stem,
        }
    }
}

/// The names of every marker file and every zone file of a folder,
/// sorted, from the names of its files.
pub fn file_names<'a>(names: impl IntoIterator<Item = &'a str>) -> (Vec<String>, Vec<String>) {
    let (mut markers, mut zones) = (Vec::new(), Vec::new());
    for file in names.into_iter().filter_map(MapFile::of) {
        match file {
            MapFile::Zones { stem } => zones.push(stem.to_string()),
            MapFile::Csv { stem } | MapFile::Landmarks { stem } => markers.push(stem.to_string()),
        }
    }
    markers.sort();
    zones.sort();
    (markers, zones)
}

/// The markers of the text of a marker file, read by the format of its
/// name. None for a file that holds no markers.
pub fn parse_markers(file_name: &str, text: &str) -> Option<Vec<Marker>> {
    match MapFile::of(file_name)? {
        MapFile::Csv { .. } => Some(parse_markers_csv(text)),
        MapFile::Landmarks { .. } => Some(
            Landmarks::from_text(file_name, text)
                .find(None, None)
                .into_iter()
                .map(|place| Marker {
                    name: place.name.clone(),
                    map: place.map,
                    x: place.at.x,
                    y: place.at.y,
                    icon: String::new(),
                    color: String::new(),
                })
                .collect(),
        ),
        MapFile::Zones { .. } => None,
    }
}

/// One marker as a line of a CSV marker file. A comma of the name would
/// start a new field, so it becomes a space.
pub fn csv_line(marker: &Marker) -> String {
    format!(
        "{},{},{},{},{},{},{NEW_MARKER_ZOOM}",
        marker.x,
        marker.y,
        marker.map,
        marker.name.replace(CSV_SPLIT, " "),
        marker.icon,
        marker.color
    )
}

/// A CSV marker file of these markers, a line for each.
pub fn markers_csv(markers: &[Marker]) -> String {
    markers
        .iter()
        .map(|marker| csv_line(marker) + CSV_LINE_END)
        .collect()
}

/// The markers after the player kept one: a new one at the end, or one
/// in the place of the one at `editing`.
pub fn kept_marker(
    mut markers: Vec<Marker>,
    editing: Option<usize>,
    marker: Marker,
) -> Vec<Marker> {
    match editing.and_then(|at| markers.get_mut(at)) {
        Some(old) => *old = marker,
        None => markers.push(marker),
    }
    markers
}

/// The markers less the one at `at`.
pub fn removed_marker(mut markers: Vec<Marker>, at: usize) -> Vec<Marker> {
    if at < markers.len() {
        markers.remove(at);
    }
    markers
}

/// The markers of a list whose names hold the search words, with their
/// places in the list.
pub fn found<'a>(markers: &'a [Marker], search: &str) -> Vec<(usize, &'a Marker)> {
    let search = search.trim().to_lowercase();
    markers
        .iter()
        .enumerate()
        .filter(|(_, marker)| search.is_empty() || marker.name.to_lowercase().contains(&search))
        .collect()
}

/// The place of a color word in `MARKER_COLORS`, the first for a word it
/// does not hold.
pub fn color_index(word: &str) -> usize {
    MARKER_COLORS
        .iter()
        .position(|color| color.eq_ignore_ascii_case(word))
        .unwrap_or_default()
}

/// The fields of the box that adds or changes a marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerFields {
    pub map: u8,
    pub icon: String,
    pub x: String,
    pub y: String,
    pub name: String,
    /// The place of its color in `MARKER_COLORS`.
    pub color: usize,
}

impl MarkerFields {
    /// The fields of a marker. A marker with no name takes the default
    /// name.
    pub fn of(marker: &Marker) -> Self {
        let name = if marker.name.is_empty() {
            DEFAULT_MARKER_NAME
        } else {
            &marker.name
        };
        Self {
            map: marker.map,
            icon: marker.icon.clone(),
            x: marker.x.to_string(),
            y: marker.y.to_string(),
            name: name.to_string(),
            color: color_index(&marker.color),
        }
    }

    /// The marker the fields make, when they are right: numbers inside the
    /// facet and a name. A comma would start a new field of the marker
    /// file, so it is left out.
    pub fn marker(&self) -> Option<Marker> {
        let (width, height) = facet_size(self.map);
        let x: u16 = self.x.trim().parse().ok().filter(|x| *x <= width)?;
        let y: u16 = self.y.trim().parse().ok().filter(|y| *y <= height)?;
        let name = self.name.replace(CSV_SPLIT, "");
        (!name.trim().is_empty()).then(|| Marker {
            name,
            map: self.map,
            x,
            y,
            icon: self.icon.clone(),
            color: MARKER_COLORS[self.color.min(MARKER_COLORS.len() - 1)].to_string(),
        })
    }
}

/// A marker with no name yet where the character stands, as "Add Marker on
/// Player" makes it.
pub fn marker_on_player(frame: &WatchFrame) -> Marker {
    Marker {
        name: String::new(),
        map: frame.map,
        x: frame.x,
        y: frame.y,
        icon: String::new(),
        color: ON_PLAYER_COLOR.to_string(),
    }
}

/// The step of the zoom after turns of the wheel, held in the list: up
/// comes closer.
pub fn zoom_step(step: u8, turns: i32) -> u8 {
    let last = ZOOMS.len() as i32 - 1;
    (i32::from(step).min(last) + turns).clamp(0, last) as u8
}

/// The points one tile takes at a zoom step.
pub fn zoom_points(step: u8) -> f32 {
    ZOOMS[usize::from(step).min(ZOOMS.len() - 1)]
}

/// True when a click on a world map answers the target cursor with the
/// ground there, as the World Map page lets it.
pub fn targets_ground(frame: &WatchFrame, options: &WorldMapOptions) -> bool {
    options.allow_positional_target && frame.target_cursor && frame.target_kind == TARGET_GROUND
}

/// True when the World Map page hides the file of this stem.
pub fn is_hidden(hidden: &[String], stem: &str) -> bool {
    hidden.iter().any(|known| known == stem)
}

/// Hides a file when it shows, and shows it when it is hidden.
pub fn flip_hidden(hidden: &mut Vec<String>, name: String) {
    match hidden.iter().position(|known| *known == name) {
        Some(at) => {
            hidden.remove(at);
        }
        None => hidden.push(name),
    }
}

/// One zone of a zone file: a named area drawn in a color.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub label: String,
    pub color: String,
    pub corners: Vec<(u16, u16)>,
}

/// The zones of one file, and the facet they lie on.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneFile {
    pub name: String,
    pub map: u8,
    pub zones: Vec<Zone>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ZonesJson {
    map_index: u8,
    zones: Vec<ZoneJson>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ZoneJson {
    label: String,
    color: String,
    polygon: Vec<Vec<u16>>,
}

/// Reads one zone file, named `name`. None when it does not read.
pub fn parse_zones_json(name: &str, text: &str) -> Option<ZoneFile> {
    let json: ZonesJson = serde_json::from_str(text).ok()?;
    Some(ZoneFile {
        name: name.to_string(),
        map: json.map_index,
        zones: json
            .zones
            .into_iter()
            .map(|zone| Zone {
                label: zone.label,
                color: zone.color,
                corners: zone
                    .polygon
                    .iter()
                    .filter_map(|corner| Some((*corner.first()?, *corner.get(1)?)))
                    .collect(),
            })
            .collect(),
    })
}

/// True when a tile lies inside a zone's corners.
pub fn inside(corners: &[(u16, u16)], x: u16, y: u16) -> bool {
    let (x, y) = (f32::from(x), f32::from(y));
    let mut inside = false;
    let mut last = corners.len().wrapping_sub(1);
    for (at, &(ax, ay)) in corners.iter().enumerate() {
        let (bx, by) = corners[last];
        let (ax, ay, bx, by) = (f32::from(ax), f32::from(ay), f32::from(bx), f32::from(by));
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            inside = !inside;
        }
        last = at;
    }
    inside
}

/// A member of the party or the guild on the map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupMember {
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub hits_percent: Option<u8>,
    pub guild: bool,
}

/// Where each party member in sight and each tracked member stands, on the
/// facet of the character.
pub fn group_on_map(frame: &WatchFrame) -> Vec<GroupMember> {
    let mut members: Vec<GroupMember> = frame
        .party_members
        .iter()
        .filter(|member| member.serial != frame.serial)
        .filter_map(|member| {
            let seen = frame.mobiles.iter().find(|m| m.serial == member.serial)?;
            Some(GroupMember {
                name: member.name.clone(),
                x: seen.x,
                y: seen.y,
                hits_percent: member.hits_percent,
                guild: false,
            })
        })
        .collect();
    for tracked in frame
        .tracked_members
        .iter()
        .filter(|tracked| tracked.map == frame.map)
    {
        if !members.iter().any(|m| m.name == tracked.name) {
            members.push(GroupMember {
                name: tracked.name.clone(),
                x: tracked.x,
                y: tracked.y,
                hits_percent: tracked.hits_percent,
                guild: tracked.guild,
            });
        }
    }
    members
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchMobile, WatchPartyMember, WatchTrackedMember};

    #[test]
    fn a_file_is_hidden_only_when_its_stem_is_in_the_hidden_list() {
        let hidden = vec!["dungeons".to_string()];
        assert!(is_hidden(&hidden, "dungeons"));
        assert!(!is_hidden(&hidden, "towns"));
        assert!(!is_hidden(&[], "dungeons"));
    }

    #[test]
    fn a_facet_has_its_size_and_an_unknown_one_the_largest() {
        assert_eq!(facet_size(2), (2304, 1600));
        assert_eq!(facet_size(99), (7168, 4096));
    }

    #[test]
    fn the_throne_is_the_middle_of_the_sextant_and_a_place_comes_back() {
        assert_eq!(sextant(1, 1323, 1624).as_deref(), Some("0o 0'S, 0o 0'E"));
        let britain = sextant(1, 1434, 1699).unwrap();
        assert_eq!(britain, "6o 35'S, 7o 48'E");
        let (x, y) = parse_goto(&britain).unwrap();
        assert!(x.abs_diff(1434) <= 1 && y.abs_diff(1699) <= 1, "{x} {y}");
        assert!(sextant(1, 7000, 100).is_none(), "past the sextant world");
        assert!(sextant(1, 5900, 3100).is_some(), "the lost lands have one");
    }

    #[test]
    fn the_goto_box_reads_a_tile_or_nothing() {
        assert_eq!(parse_goto("1434 1699"), Some((1434, 1699)));
        assert_eq!(parse_goto("1434, 1699"), Some((1434, 1699)));
        assert_eq!(parse_goto("1331:745"), Some((1331, 745)));
        assert_eq!(parse_goto("Britain"), None);
        assert_eq!(parse_goto("1 2 3"), None);
    }

    fn named(name: &str) -> Marker {
        Marker {
            name: name.into(),
            map: 0,
            x: 0,
            y: 0,
            icon: String::new(),
            color: MARKER_COLORS[0].to_string(),
        }
    }

    #[test]
    fn the_search_keeps_the_markers_whose_names_hold_its_words() {
        let markers = [
            named("Britain Bank"),
            named("Yew Gate"),
            named("bank of Vesper"),
        ];
        let hits: Vec<usize> = found(&markers, " BANK ")
            .iter()
            .map(|(at, _)| *at)
            .collect();
        assert_eq!(hits, vec![0, 2]);
        assert_eq!(found(&markers, "").len(), 3);
    }

    #[test]
    fn marker_fields_make_a_marker_only_when_they_are_right() {
        let mut fields = MarkerFields::of(&Marker {
            map: 1,
            x: 1434,
            y: 1699,
            color: "Blue".into(),
            ..named("")
        });
        assert_eq!(fields.name, DEFAULT_MARKER_NAME);
        assert_eq!(MARKER_COLORS[fields.color], "blue");
        fields.name = "My, camp".into();
        let made = fields.marker().unwrap();
        assert_eq!(
            (made.name.as_str(), made.x, made.y),
            ("My camp", 1434, 1699)
        );
        fields.x = "9000".into();
        assert!(fields.marker().is_none(), "past the edge of the facet");
        fields.x = "10".into();
        fields.name = " ".into();
        assert!(fields.marker().is_none(), "a marker needs a name");
        assert_eq!(MarkerFields::of(&named("Yew")).name, "Yew");
        let frame = WatchFrame {
            x: 5,
            y: 6,
            map: 2,
            ..WatchFrame::default()
        };
        let mine = marker_on_player(&frame);
        assert_eq!(
            (mine.map, mine.x, mine.y, mine.color.as_str()),
            (2, 5, 6, ON_PLAYER_COLOR)
        );
    }

    #[test]
    fn a_kept_marker_is_added_or_changed_and_a_removed_one_is_cut() {
        let markers = kept_marker(Vec::new(), None, named("Camp"));
        let markers = kept_marker(markers, None, named("Mine"));
        let markers = kept_marker(markers, Some(1), named("Cave"));
        let names: Vec<&str> = markers.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["Camp", "Cave"]);
        let markers = kept_marker(markers, Some(9), named("Far"));
        assert_eq!(markers.len(), 3, "a place past the list adds");
        let markers = removed_marker(markers, 0);
        assert_eq!(markers[0].name, "Cave");
        assert_eq!(removed_marker(markers, 9).len(), 2);
    }

    #[test]
    fn the_wheel_steps_the_zoom_inside_its_list_and_files_hide_and_show() {
        assert_eq!(zoom_step(4, 1), 5);
        assert_eq!(zoom_step(4, -1), 3);
        assert_eq!(zoom_step(0, -1), 0);
        assert_eq!(zoom_step(9, 1), 9);
        assert_eq!(zoom_step(200, 0), 9, "a step past the list is its end");
        assert_eq!(zoom_points(200), ZOOMS[ZOOMS.len() - 1]);
        let mut hidden = Vec::new();
        flip_hidden(&mut hidden, "towns".into());
        assert_eq!(hidden, vec!["towns".to_string()]);
        flip_hidden(&mut hidden, "towns".into());
        assert!(hidden.is_empty());
        let mut frame = WatchFrame {
            target_cursor: true,
            target_kind: TARGET_GROUND,
            ..WatchFrame::default()
        };
        let mut options = WorldMapOptions::default();
        assert!(!targets_ground(&frame, &options), "the page must let it");
        options.allow_positional_target = true;
        assert!(targets_ground(&frame, &options));
        frame.target_kind = 0;
        assert!(!targets_ground(&frame, &options), "a target of a thing");
    }

    #[test]
    fn csv_markers_read_and_a_marker_file_is_read_by_its_name() {
        let text = "1434,1699,1,Britain Bank,bank,yellow,3\nbad line\n100,200,0,Yew\n";
        let markers = parse_markers_csv(text);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].name, "Britain Bank");
        assert_eq!(markers[1].color, "");
        assert_eq!(color_of("Yellow"), Some([255, 255, 0]));
        assert_eq!(color_of("none"), None);
        let uoam = parse_markers("towns.map", "3\n+BANK: 1434 1699 1 Britain Bank\n").unwrap();
        assert_eq!((uoam[0].x, uoam[0].name.as_str()), (1434, "Britain Bank"));
        assert_eq!(parse_markers("old.CSV", "1,1,0,Hidden\n").unwrap().len(), 1);
        assert!(parse_markers("towns.zones.json", "{}").is_none());
        assert!(parse_markers("notes.txt", "").is_none());
        let place = Marker {
            name: "My, camp".into(),
            map: 1,
            x: 10,
            y: 20,
            icon: "exit".into(),
            color: NEW_MARKER_COLOR.into(),
        };
        let again = parse_markers_csv(&markers_csv(std::slice::from_ref(&place)));
        assert_eq!(again[0].name, "My  camp", "a comma of the name is a space");
        assert_eq!((again[0].icon.as_str(), again[0].x), ("exit", 10));
    }

    #[test]
    fn zones_read_and_a_tile_is_inside_or_out() {
        let text = r#"{ "MapIndex": 1, "Zones": [ { "Label": "Britain", "Color": "red",
            "Polygon": [[0, 0], [10, 0], [10, 10], [0, 10]] } ] }"#;
        let file = parse_zones_json("towns", text).unwrap();
        assert_eq!((file.map, file.zones.len()), (1, 1));
        assert!(inside(&file.zones[0].corners, 5, 5));
        assert!(!inside(&file.zones[0].corners, 15, 5));
        assert!(parse_zones_json("bad", "{}").is_none());
        let names = ["towns.zones.json", "camps.csv", "notes.txt", "Trips.LUA"];
        assert_eq!(
            file_names(names),
            (
                vec!["Trips".to_string(), "camps".to_string()],
                vec!["towns".to_string()]
            )
        );
        assert_eq!(
            MapFile::of("user-markers.usr").map(MapFile::stem),
            Some(USER_MARKERS)
        );
        assert_eq!(MapFile::of(".csv"), None);
    }

    #[test]
    fn the_party_in_sight_and_the_tracked_members_show() {
        let frame = WatchFrame {
            serial: 1,
            party_members: vec![
                WatchPartyMember {
                    serial: 1,
                    name: "Me".into(),
                    hits_percent: None,
                    ..WatchPartyMember::default()
                },
                WatchPartyMember {
                    serial: 2,
                    name: "Ann".into(),
                    hits_percent: Some(50),
                    ..WatchPartyMember::default()
                },
            ],
            mobiles: vec![WatchMobile {
                serial: 2,
                x: 30,
                y: 40,
                ..WatchMobile::default()
            }],
            tracked_members: vec![
                WatchTrackedMember {
                    name: "Bob".into(),
                    x: 5,
                    y: 6,
                    guild: true,
                    ..WatchTrackedMember::default()
                },
                WatchTrackedMember {
                    name: "Far".into(),
                    map: 3,
                    ..WatchTrackedMember::default()
                },
            ],
            ..WatchFrame::default()
        };
        let group = group_on_map(&frame);
        let names: Vec<&str> = group.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["Ann", "Bob"]);
        assert!(group[1].guild && (group[0].x, group[0].y) == (30, 40));
    }
}

#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn markers_survive_the_csv_round_trip() {
        let text = "1424,1693,0,Britain bank,bank,green,3\n";
        let markers = parse_markers_csv(text);
        assert_eq!(markers_csv(&markers), text);
    }
}
