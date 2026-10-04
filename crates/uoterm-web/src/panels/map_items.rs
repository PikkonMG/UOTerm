//! Two panels about people and places: each map item the character
//! opened, with its pins, and the profile a player wrote about a
//! character. The land of a map is the picture the server makes of its
//! radar colors (`/v1/map-item/...`), with the course line from one pin to
//! the next. While the shard lets the map be drawn on and the human has
//! control, a click puts a pin, a pin drags to a new place, and a double
//! click takes it off. What each does is `uoterm_view::ui::map_item`.

use super::{FrameSpec, Framed, NoteData, PANEL_MAP_ITEM_PREFIX, PANEL_PROFILE};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::{Act, Asker};
use uoterm_view::frame::{WatchFrame, WatchMap};
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::model::map_item::{map_item_path, pixel_at, point_of};
use uoterm_view::ui::map_item::{
    map_item_first_place, map_item_id, map_item_land, place_ask, plotting, profile_first_place,
    profile_words, take_place_answer, MapItemButton, PinDeed, ProfilePanel, HINT_PIN,
    HINT_PIN_MOVE, HINT_PROFILE, HINT_WISH, MAP_ITEM_BUTTONS, WORDS_ASKING, WORDS_CLOSE,
    WORDS_MARK, WORDS_NO_FILES, WORDS_TITLE,
};
use uoterm_view::ui::places::TITLE_ROW;
use uoterm_view::ui::theme::PANEL_PAD;

/// What the map items and the profile keep between frames.
#[derive(Default)]
pub(crate) struct MapItemsState {
    pub profile: ProfilePanel,
    /// The map Jev looks for a place on.
    asked_for: Option<u32>,
    /// Words about the last thing Jev did: failed or not.
    note: Option<(String, bool)>,
}

/// A pin: its number and where it is on the land.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PinData {
    pub number: String,
    pub at: Point,
}

/// A button of the foot of a map item.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapButtonData {
    pub words: &'static str,
    pub waiting: bool,
}

/// A map item: its land, its pins and its buttons, in the points of the
/// land from its top left.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapItemData {
    pub live: bool,
    pub path: String,
    /// The paper round the land, and the land.
    pub paper: Vector,
    pub land: Vector,
    pub edge: f32,
    pub pins: Vec<PinData>,
    /// The player may put, move and take off pins.
    pub plotting: bool,
    pub pin_hint: &'static str,
    pub pin_move_hint: &'static str,
    pub no_files: &'static str,
    pub wish_hint: &'static str,
    pub mark: &'static str,
    pub buttons: Vec<MapButtonData>,
    pub note: Option<NoteData>,
}

/// The profile of a character.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProfileData {
    pub title: String,
    pub words: String,
    /// The words being written, while the owner writes.
    pub writing: Option<String>,
    pub hint: &'static str,
    /// Write or Save, while the human has control of the owner.
    pub write: Option<&'static str>,
    pub close: &'static str,
}

/// A place on the land of a map, in its points.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
struct LandPoint {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
struct PinMove {
    pin: usize,
    x: f32,
    y: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MapItemAction {
    Pin(LandPoint),
    MovePin(PinMove),
    RemovePin(usize),
    /// A button of the foot, by its place.
    Button(usize),
    /// Plain words Jev finds a place for.
    Wish(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProfileAction {
    Write(bool),
    Words(String),
    Close(bool),
}

/// The open map a panel name names, and its place among the open maps.
fn named_map<'f>(frame: &'f WatchFrame, panel: &str) -> Option<(usize, &'f WatchMap)> {
    let serial: u32 = panel.strip_prefix(PANEL_MAP_ITEM_PREFIX)?.parse().ok()?;
    frame
        .maps
        .iter()
        .enumerate()
        .find(|(_, map)| map.serial == serial)
}

impl WebView {
    /// The map items and the profile in one frame: the place Jev found
    /// becomes a pin, and the own profile shows with the first picture
    /// when the window starts with it.
    pub(crate) fn follow_map_items(&mut self, frame: &WatchFrame) {
        for answer in self.hand.new_answers(Asker::MapItem) {
            let state = &mut self.panels.map_items;
            let map = state
                .asked_for
                .and_then(|serial| frame.maps.iter().find(|map| map.serial == serial));
            match take_place_answer(map, answer) {
                Ok(Some(act)) => {
                    self.hand.act(act);
                    state.note = None;
                }
                Ok(None) => {}
                Err(words) => state.note = Some((words, true)),
            }
        }
        if let Some(act) = self.panels.map_items.profile.follow(frame) {
            self.hand.act(act);
        }
    }

    pub(super) fn map_item_spec(&self, frame: &WatchFrame, panel: &str) -> Option<FrameSpec> {
        let (index, map) = named_map(frame, panel)?;
        let default = map_item_first_place(self.panel_room(), index, map);
        let spec = FrameSpec::fixed(&map_item_id(index), WORDS_TITLE, default);
        Some(spec.closable_if(frame.human_control))
    }

    /// The paper and the land of a map in the body of its panel.
    fn map_item_areas(&self, spec: &FrameSpec) -> (Area, Area) {
        let area = self.panel_area(spec);
        let body = Area::from_min_size(
            Point::new(0.0, 0.0),
            Vector::new(
                area.width() - PANEL_PAD * 2.0,
                area.height() - PANEL_PAD * 2.0 - TITLE_ROW,
            ),
        );
        map_item_land(body)
    }

    pub(super) fn map_items_data(&self, frame: &WatchFrame) -> Vec<Framed<MapItemData>> {
        let live = frame.human_control;
        let state = &self.panels.map_items;
        frame
            .maps
            .iter()
            .filter_map(|map| {
                let panel = format!("{PANEL_MAP_ITEM_PREFIX}{}", map.serial);
                let spec = self.map_item_spec(frame, &panel)?;
                let (paper, land) = self.map_item_areas(&spec);
                let on_land = Area::from_min_size(Point::new(0.0, 0.0), land.size());
                let body = MapItemData {
                    live,
                    path: map_item_path(map),
                    paper: paper.size(),
                    land: land.size(),
                    edge: land.min.x - paper.min.x,
                    pins: map
                        .pins
                        .iter()
                        .enumerate()
                        .map(|(at, pixel)| PinData {
                            number: (at + 1).to_string(),
                            at: point_of(map, on_land, *pixel),
                        })
                        .collect(),
                    plotting: plotting(frame, map),
                    pin_hint: HINT_PIN,
                    pin_move_hint: HINT_PIN_MOVE,
                    no_files: WORDS_NO_FILES,
                    wish_hint: HINT_WISH,
                    mark: WORDS_MARK,
                    buttons: if live {
                        MAP_ITEM_BUTTONS
                            .iter()
                            .map(|button| {
                                let (words, waiting) = button.words(map);
                                MapButtonData { words, waiting }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    note: state.note.as_ref().map(|(words, failed)| NoteData {
                        words: words.clone(),
                        failed: *failed,
                    }),
                };
                Some(self.framed(&panel, &spec, body))
            })
            .collect()
    }

    pub(super) fn map_item_action(&mut self, panel: &str, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Some((_, map)) = named_map(&frame, panel) else {
            return;
        };
        let Some(spec) = self.map_item_spec(&frame, panel) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<MapItemAction>(action) else {
            return;
        };
        if !frame.human_control {
            return;
        }
        let (_, land) = self.map_item_areas(&spec);
        let on_land = Area::from_min_size(Point::new(0.0, 0.0), land.size());
        let pixel = |x: f32, y: f32| pixel_at(map, on_land, Point::new(x, y));
        let plots = plotting(&frame, map);
        let act = match action {
            MapItemAction::Pin(at) if plots => {
                let (x, y) = pixel(at.x, at.y);
                Some(Act::MapPin { x, y })
            }
            MapItemAction::MovePin(moved) if plots && moved.pin < map.pins.len() => {
                Some(PinDeed::Moved(moved.pin, pixel(moved.x, moved.y)).act())
            }
            MapItemAction::RemovePin(pin) if plots && pin < map.pins.len() => {
                Some(PinDeed::Removed(pin).act())
            }
            MapItemAction::Button(at) => MAP_ITEM_BUTTONS.get(at).map(|button| button.act(map)),
            MapItemAction::Wish(words) if map.may_plot => {
                if let Some(ask) = place_ask(map, &words) {
                    self.hand.ask(Asker::MapItem, ask);
                    let state = &mut self.panels.map_items;
                    state.asked_for = Some(map.serial);
                    state.note = Some((WORDS_ASKING.to_string(), false));
                }
                None
            }
            _ => None,
        };
        if let Some(act) = act {
            self.hand.act(act);
        }
    }

    /// The close mark of a map item closes it on the shard.
    pub(super) fn close_map_item(&mut self, panel: &str) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        if let Some((_, map)) = named_map(&frame, panel) {
            self.hand.act(MapItemButton::Close.act(map));
        }
    }

    /// Shows the profile of a character, or closes it when it shows his
    /// already, as the Profile button of the bar does.
    pub(crate) fn toggle_profile(&mut self, serial: u32) {
        let profile = &mut self.panels.map_items.profile;
        if profile.shows(serial) {
            profile.close();
        } else {
            let act = profile.show(serial);
            self.hand.act(act);
        }
    }

    /// Shows the profile a line of the ring asked the session for.
    pub(crate) fn show_profile(&mut self, serial: u32) {
        let _asked = self.panels.map_items.profile.show(serial);
    }

    pub(super) fn profile_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let profile = &self.panels.map_items.profile;
        profile.shown?;
        let spec = FrameSpec::fixed(
            uoterm_view::ui::map_item::PROFILE_ID,
            &profile.title(frame),
            profile_first_place(self.panel_room()),
        );
        Some(spec.closable())
    }

    pub(super) fn profile_data(&self, frame: &WatchFrame) -> Option<Framed<ProfileData>> {
        let spec = self.profile_spec(frame)?;
        let profile = &self.panels.map_items.profile;
        let known = profile.known(frame);
        let body = ProfileData {
            title: known.map_or_else(String::new, |kept| kept.title.clone()),
            words: profile_words(known),
            writing: profile.writing.clone(),
            hint: HINT_PROFILE,
            write: profile.write_words(frame),
            close: WORDS_CLOSE,
        };
        Some(self.framed(PANEL_PROFILE, &spec, body))
    }

    pub(super) fn profile_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<ProfileAction>(action) else {
            return;
        };
        let profile = &mut self.panels.map_items.profile;
        match action {
            ProfileAction::Write(_) => {
                if let Some(act) = profile.press_write(&frame) {
                    self.hand.act(act);
                }
            }
            ProfileAction::Words(words) => {
                if let Some(writing) = profile.writing.as_mut() {
                    *writing = words;
                }
            }
            ProfileAction::Close(_) => profile.close(),
        }
    }

    pub(super) fn close_profile(&mut self) {
        self.panels.map_items.profile.close();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, view_with};
    use super::*;
    use crate::out::OutCall;
    use serde_json::json;

    const MAP: u32 = 60;

    fn with_map(control: bool, may_plot: bool) -> WebView {
        view_with(
            "maps",
            json!([{ "serial": MAP, "facet": 1, "start_x": 1000, "start_y": 1200,
                "end_x": 1400, "end_y": 1600, "width": 200, "height": 200,
                "may_plot": may_plot, "pins": [{ "x": 40, "y": 90 }, { "x": 100, "y": 20 }] }]),
            control,
        )
    }

    fn panel() -> String {
        format!("{PANEL_MAP_ITEM_PREFIX}{MAP}")
    }

    #[test]
    fn a_map_item_pins_and_plots_as_the_window() {
        let mut view = with_map(true, true);
        let data = view.panel_data(0.0).map_items.remove(0);
        assert_eq!(data.body.path, "/v1/map-item/1/1000/1200/1400/1600");
        assert_eq!(data.body.pins[1].number, "2");
        let land = data.body.land;
        let frame = view.frame_ref().unwrap().clone();
        let map = &frame.maps[0];
        let on_land = Area::from_min_size(Point::new(0.0, 0.0), land);
        let middle = land / 2.0;
        let out = press(
            &mut view,
            &panel(),
            json!({ "pin": { "x": middle.x, "y": middle.y } }),
        );
        let (x, y) = pixel_at(map, on_land, Point::new(middle.x, middle.y));
        assert_eq!(out_acts(&out), vec![Act::MapPin { x, y }.for_page()]);
        let out = press(
            &mut view,
            &panel(),
            json!({ "move_pin": { "pin": 1, "x": 0.0, "y": 0.0 } }),
        );
        assert_eq!(
            out_acts(&out),
            vec![PinDeed::Moved(1, (0, 0)).act().for_page()]
        );
        let out = press(&mut view, &panel(), json!({ "remove_pin": 0 }));
        assert_eq!(out_acts(&out), vec![PinDeed::Removed(0).act().for_page()]);
        let out = press(&mut view, &panel(), json!({ "button": 1 }));
        assert_eq!(
            out_acts(&out),
            vec![MapItemButton::Plot.act(map).for_page()]
        );
        let out = press(&mut view, &panel(), json!({ "close": true }));
        assert_eq!(out_acts(&out), vec![Act::MapClose(MAP).for_page()]);
        let out = press(&mut view, &panel(), json!({ "wish": "Britain bank" }));
        assert!(matches!(out.as_slice(), [OutCall::Read { .. }]), "{out:?}");
    }

    #[test]
    fn no_pin_moves_without_control_or_while_the_shard_does_not_let_it() {
        let mut view = with_map(false, true);
        assert!(view.panel_data(0.0).map_items[0].body.buttons.is_empty());
        let pin = json!({ "pin": { "x": 1.0, "y": 1.0 } });
        assert!(out_acts(&press(&mut view, &panel(), pin.clone())).is_empty());
        assert!(out_acts(&press(&mut view, &panel(), json!({ "button": 0 }))).is_empty());
        let mut view = with_map(true, false);
        assert!(out_acts(&press(&mut view, &panel(), pin)).is_empty());
        assert!(!view.panel_data(0.0).map_items[0].body.plotting);
    }

    #[test]
    fn the_owner_writes_his_profile_and_no_one_else() {
        let mut view = view_with(
            "profiles",
            json!([{ "serial": crate::tests::ME, "name": "Mara", "title": "the Brave",
                "shard_words": "Young", "own_words": "Hi" }]),
            true,
        );
        view.toggle_profile(crate::tests::ME);
        let data = view.panel_data(0.0).profile.unwrap();
        assert_eq!(data.frame.title, "Mara");
        assert_eq!(data.body.words, "Young\n\nHi");
        press(&mut view, PANEL_PROFILE, json!({ "write": true }));
        press(&mut view, PANEL_PROFILE, json!({ "words": "Hello" }));
        let out = press(&mut view, PANEL_PROFILE, json!({ "write": true }));
        let written = Act::ProfileWrite {
            serial: crate::tests::ME,
            text: "Hello".into(),
        };
        assert_eq!(out_acts(&out), vec![written.for_page()]);
        view.toggle_profile(crate::tests::ME + 1);
        let asked = out_acts(&view.take_out_native());
        assert_eq!(
            asked,
            vec![Act::ProfileRead(crate::tests::ME + 1).for_page()]
        );
        assert!(view.panel_data(0.0).profile.unwrap().body.write.is_none());
        let out = press(&mut view, PANEL_PROFILE, json!({ "write": true }));
        assert!(out_acts(&out).is_empty());
        press(&mut view, PANEL_PROFILE, json!({ "close": true }));
        assert!(view.panel_data(0.0).profile.is_none());
    }
}
