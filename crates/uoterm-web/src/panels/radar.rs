//! The radar of the Modern style: the land round the character, turned as
//! the play field is turned, with the mobiles, the party, the marker and
//! zone files, the named places of the session and the character marked
//! over it. The land is the
//! picture the server makes of the radar colors round a tile
//! (`/v1/map/near/...`), asked for again only when the character walked
//! far from its middle, so the radar reads no map block. Where the land
//! and each mark go is `uoterm_view::map_lay`.

use super::{panel_notches, FrameSpec, Framed, Place, PANEL_RADAR};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::map_lay::{
    mark_layout, near_map_path, near_still_serves, session_markers, zoomed, Lay, MapFiles,
    MarkPlace, Marks, WordsAnchor, MODERN_LOOK, SPAN,
};
use uoterm_view::model::places;
use uoterm_view::ui::launch::RADAR_ID;
use uoterm_view::ui::layout::{first_place, Spot};
use uoterm_view::ui::lists::{
    radar_panel_size, HINT_RADAR, RADAR_FIRST_ZOOM, RADAR_ZOOM_MAX, WORDS_RADAR,
    WORDS_RADAR_NO_FILES,
};
use uoterm_view::ui::places::TITLE_ROW;
use uoterm_view::ui::theme::{css_color, HITS, PANEL_PAD, TRACK};

const HALF: f32 = 2.0;

pub(crate) struct RadarState {
    zoom: f32,
    /// The map and the middle tile of the picture of the land the page
    /// shows.
    near: Option<(u8, (u16, u16))>,
}

impl Default for RadarState {
    fn default() -> Self {
        Self {
            zoom: RADAR_FIRST_ZOOM,
            near: None,
        }
    }
}

/// The radar: its field, the land and the marks, in the points of the
/// field from its top left.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RadarData {
    /// The size of the field.
    pub side: Vector,
    pub land: LandData,
    pub marks: Vec<MarkData>,
    /// The words that show when the land does not come.
    pub no_files: &'static str,
    pub hint: &'static str,
}

/// The picture of the land and how it lies on the field: the CSS matrix
/// `[a, b, c, d, e, f]` that takes a pixel of the picture to the field.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LandData {
    pub path: String,
    pub side: usize,
    pub matrix: [f32; 6],
}

/// One mark over the land, as `map_lay::MarkPlace`, with its color as CSS.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MarkData {
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: String,
    },
    Outline {
        points: Vec<Point>,
        width: f32,
        color: String,
    },
    Words {
        at: Point,
        /// The words hang from the point by their top, or their middle.
        top: bool,
        words: String,
        color: String,
    },
    Dot {
        at: Point,
        radius: f32,
        color: String,
    },
    Square {
        at: Point,
        side: f32,
        color: String,
    },
    Ring {
        at: Point,
        radius: f32,
        width: f32,
        color: String,
    },
    HealthBar {
        track: Place,
        share: f32,
        color: String,
        back: String,
    },
}

impl From<MarkPlace> for MarkData {
    fn from(mark: MarkPlace) -> Self {
        match mark {
            MarkPlace::Line {
                from,
                to,
                width,
                color,
            } => Self::Line {
                from,
                to,
                width,
                color: css_color(color),
            },
            MarkPlace::Outline {
                points,
                width,
                color,
            } => Self::Outline {
                points,
                width,
                color: css_color(color),
            },
            MarkPlace::Words {
                at,
                anchor,
                words,
                color,
            } => Self::Words {
                at,
                top: anchor == WordsAnchor::LeftTop,
                words,
                color: css_color(color),
            },
            MarkPlace::Dot { at, radius, color } => Self::Dot {
                at,
                radius,
                color: css_color(color),
            },
            MarkPlace::Square { at, side, color } => Self::Square {
                at,
                side,
                color: css_color(color),
            },
            MarkPlace::Ring {
                at,
                radius,
                width,
                color,
            } => Self::Ring {
                at,
                radius,
                width,
                color: css_color(color),
            },
            MarkPlace::HealthBar { track, share } => Self::HealthBar {
                track: Place::from(track),
                share,
                color: css_color(HITS),
                back: css_color(TRACK),
            },
        }
    }
}

/// `{"wheel": notches}` zooms; `{"double": true}` makes the radar large or
/// small.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RadarAction {
    Wheel(f32),
    Double(bool),
}

impl WebView {
    pub(super) fn radar_spec(&self) -> FrameSpec {
        let size = radar_panel_size(self.profile.world_map.minimap_large);
        let default = first_place(self.panel_room(), Spot::Radar, size);
        FrameSpec::fixed(RADAR_ID, WORDS_RADAR, default)
            .closable()
            .foldable()
    }

    pub(super) fn radar_data(&mut self, frame: &WatchFrame) -> Option<Framed<RadarData>> {
        if places::is_shut(&self.profile, RADAR_ID) {
            return None;
        }
        let spec = self.radar_spec();
        let area = self.panel_area(&spec);
        let side = Vector::new(
            (area.width() - PANEL_PAD * 2.0).max(0.0),
            (area.height() - PANEL_PAD * 2.0 - TITLE_ROW).max(0.0),
        );
        let here = (frame.x, frame.y);
        let middle = match self.panels.radar.near {
            Some((map, drawn)) if near_still_serves(map, drawn, frame.map, here) => drawn,
            _ => here,
        };
        self.panels.radar.near = Some((frame.map, middle));
        let field = Area::from_min_size(Point::new(0.0, 0.0), side);
        let lay = Lay::Turned {
            center: field.center(),
            from: Vector::new(f32::from(frame.x), f32::from(frame.y)),
            unit: side.x.min(side.y) / (SPAN as f32 * HALF) * self.panels.radar.zoom,
        };
        let session = session_markers(self.hand.reads(), &self.profile, frame.map);
        let files = MapFiles::shown(&self.panels.world_map.folder, &self.profile.world_map);
        let marks = Marks {
            frame,
            map: frame.map,
            profile: &self.profile,
            files: &files,
            session: &session,
            looking_at: None,
        };
        let body = RadarData {
            side,
            land: land_data(frame.map, middle, lay),
            marks: mark_layout(field, lay, &marks, &MODERN_LOOK)
                .into_iter()
                .map(MarkData::from)
                .collect(),
            no_files: WORDS_RADAR_NO_FILES,
            hint: HINT_RADAR,
        };
        Some(self.framed(PANEL_RADAR, &spec, body))
    }

    pub(super) fn radar_action(&mut self, action: Value) {
        match serde_json::from_value::<RadarAction>(action) {
            Ok(RadarAction::Wheel(notches)) => {
                let radar = &mut self.panels.radar;
                radar.zoom = zoomed(radar.zoom, panel_notches(notches), RADAR_ZOOM_MAX);
            }
            Ok(RadarAction::Double(_)) => {
                let large = &mut self.profile.world_map.minimap_large;
                *large = !*large;
                self.keep_profile();
            }
            Err(_) => {}
        }
    }
}

/// The picture of the land round `middle` of `map`, and the matrix that
/// lays its pixels on the field by `lay`: one pixel for each tile, from
/// the north west corner, as the window lays its texture.
pub(super) fn land_data(map: u8, middle: (u16, u16), lay: Lay) -> LandData {
    let half = (SPAN / 2) as f32;
    let left = f32::from(middle.0) - half;
    let top = f32::from(middle.1) - half;
    let origin = lay.screen(left, top);
    let across = lay.screen(left + 1.0, top) - origin;
    let down = lay.screen(left, top + 1.0) - origin;
    LandData {
        path: near_map_path(map, middle.0, middle.1),
        side: SPAN,
        matrix: [across.x, across.y, down.x, down.y, origin.x, origin.y],
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{press, saved_profiles};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, settled};
    use serde_json::json;
    use uoterm_view::map_lay::REDRAW_TILES;

    #[test]
    fn the_radar_shows_the_land_round_the_character_until_it_walks_far() {
        let mut view = settled();
        let radar = view.panel_data(0.0).radar.unwrap().body;
        assert_eq!(radar.land.path, "/v1/map/near/0/1000/1000");
        let [a, b, ..] = radar.land.matrix;
        assert!(a > 0.0 && b > 0.0, "east goes right and down");
        let me = radar
            .marks
            .iter()
            .any(|mark| matches!(mark, MarkData::Dot { .. }));
        assert!(me, "the character is marked");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["self_state"]["location"]["x"] = json!(1000 + REDRAW_TILES / 2);
        view.frame(&watch.to_string(), 0.1);
        let near = view.panel_data(0.1).radar.unwrap().body.land.path;
        assert_eq!(
            near, "/v1/map/near/0/1000/1000",
            "a few steps keep the picture"
        );
        watch["self_state"]["location"]["x"] = json!(1000 + REDRAW_TILES * 2);
        view.frame(&watch.to_string(), 0.2);
        let far = view.panel_data(0.2).radar.unwrap().body.land.path;
        assert_eq!(
            far,
            format!("/v1/map/near/0/{}/1000", 1000 + REDRAW_TILES * 2)
        );
    }

    #[test]
    fn the_wheel_zooms_the_radar_and_a_double_click_makes_it_large() {
        let mut view = settled();
        let small = view.panel_data(0.0).radar.unwrap();
        press(&mut view, PANEL_RADAR, json!({ "wheel": 3.0 }));
        let zoomed = view.panel_data(0.0).radar.unwrap().body.land.matrix[0];
        assert!(zoomed > small.body.land.matrix[0]);
        let out = press(&mut view, PANEL_RADAR, json!({ "double": true }));
        assert!(saved_profiles(&out)[0].world_map.minimap_large);
        assert!(view.panel_data(0.0).radar.unwrap().frame.area.w > small.frame.area.w);
    }
}
