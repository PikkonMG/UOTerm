//! Two panels about people and places, apart from how they draw: each map
//! item the character opened, with its pins, and the profile a player
//! wrote about a character. Their places, their words, where the land of a
//! map lies, what its buttons and its pins send, the place Jev finds for
//! plain words, and what the profile window shows and writes. The picture
//! of a map is `model::map_item`'s.

use crate::act::{Act, Answer, Ask};
use crate::frame::{WatchFrame, WatchMap, WatchProfile};
use crate::geom::{Area, Point, Vector};
use crate::model::map_item::pixel_of;
use crate::ui::layout::{first_place, Spot};
use crate::ui::places::TITLE_ROW;
use crate::ui::theme::{PANEL_PAD, ROW_GAP};

/// A map item is the panel `"{MAP_PLACE_ID}{n}"`, from 1.
pub const MAP_PLACE_ID: &str = "modern:map_item:";
pub const PROFILE_ID: &str = "modern:profile";
/// The row of the title of the profile, under the title of its panel.
pub const PROFILE_TITLE_ROW: f32 = 32.0;
pub const FOOT_ROW: f32 = 40.0;
pub const PIN_RADIUS: f32 = 4.0;
/// A pin takes clicks this far round it.
pub const PIN_REACH: f32 = 8.0;
pub const PIN_RING: f32 = 1.5;
pub const COURSE_WIDTH: f32 = 1.5;
pub const PAPER_EDGE: f32 = 2.0;
pub const MARK_WIDTH: f32 = 70.0;
pub const FIELD_ROW: f32 = 30.0;
pub const GAP: f32 = 10.0;

pub const WORDS_TITLE: &str = "Map";
pub const WORDS_CLOSE: &str = "Close";
pub const WORDS_NO_FILES: &str = "The picture needs the client files.";
pub const HINT_PIN: &str = "Click: put a pin here.";
pub const HINT_PIN_MOVE: &str = "Drag: move the pin.  Double-click: take it off.";
pub const WORDS_MARK: &str = "Mark";
pub const HINT_WISH: &str = "Say the place in plain words, for example: Britain bank";
pub const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
pub const WORDS_ASKING: &str = "Jev looks for the place...";

const PROFILE_WIDTH: f32 = 420.0;
pub const PROFILE_ROWS: usize = 8;
pub const PROFILE_LINE: f32 = 20.0;
pub const WORDS_PROFILE: &str = "Profile";
const WORDS_WRITE: &str = "Write";
const WORDS_SAVE: &str = "Save";
pub const HINT_PROFILE: &str = "What your character says about himself";

/// A button of the foot of a map item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapItemButton {
    /// Take every pin off.
    Clear,
    /// Start or stop plotting the course.
    Plot,
    Close,
}

pub const MAP_ITEM_BUTTONS: [MapItemButton; 3] = [
    MapItemButton::Clear,
    MapItemButton::Plot,
    MapItemButton::Close,
];

impl MapItemButton {
    /// Its words, and true when they show as waiting: Plot reads Stop
    /// while the player plots.
    pub fn words(self, map: &WatchMap) -> (&'static str, bool) {
        match self {
            Self::Clear => ("Clear pins", false),
            Self::Plot if map.may_plot => ("Stop plotting", true),
            Self::Plot => ("Plot course", false),
            Self::Close => (WORDS_CLOSE, false),
        }
    }

    /// The act of a press on it.
    pub fn act(self, map: &WatchMap) -> Act {
        match self {
            Self::Clear => Act::MapClear,
            Self::Plot => Act::MapEdit,
            Self::Close => Act::MapClose(map.serial),
        }
    }
}

/// What the player did to a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinDeed {
    /// The pin at a place of the list went to a pixel of the picture.
    Moved(usize, (u16, u16)),
    /// A double click took the pin off.
    Removed(usize),
}

impl PinDeed {
    pub fn act(self) -> Act {
        let pin = |at: usize| u8::try_from(at).unwrap_or(u8::MAX);
        match self {
            Self::Moved(at, (x, y)) => Act::MapPinMove { pin: pin(at), x, y },
            Self::Removed(at) => Act::MapPinRemove(pin(at)),
        }
    }
}

/// The id of the place of the map item at `index` of the open maps.
pub fn map_item_id(index: usize) -> String {
    format!("{MAP_PLACE_ID}{}", index + 1)
}

/// The size of the panel of a map item: its paper and the rows under it.
pub fn map_item_size(map: &WatchMap) -> Vector {
    let paper = Vector::new(f32::from(map.width.max(1)), f32::from(map.height.max(1)));
    paper
        + Vector::new(
            PANEL_PAD * 2.0,
            TITLE_ROW + FIELD_ROW + GAP * 2.0 + FOOT_ROW + PANEL_PAD * 2.0,
        )
}

/// Where the map item at `index` first stands in a window: each in its
/// own place.
pub fn map_item_first_place(window: Area, index: usize, map: &WatchMap) -> Area {
    first_place(window, Spot::Middle(index), map_item_size(map))
}

/// Where the buttons of the foot of a map item or of the profile start, in
/// the body of its panel.
pub fn foot_place(body: Area) -> Point {
    Point::new(body.min.x, body.max.y - FOOT_ROW + ROW_GAP)
}

/// The paper of a map in the body of its panel, and the land inside its
/// edge. The rows are laid from the bottom up, so each one stays inside the
/// panel whatever size the map has.
pub fn map_item_land(body: Area) -> (Area, Area) {
    let foot_top = foot_place(body).y;
    let wish_top = foot_top - GAP - FIELD_ROW;
    let paper = Area::from_two_points(body.min, Point::new(body.max.x, wish_top - GAP));
    let land = Area::from_two_points(
        paper.min + Vector::splat(PAPER_EDGE),
        paper.max - Vector::splat(PAPER_EDGE),
    );
    (paper, land)
}

/// The player may put, move and take off pins now.
pub fn plotting(frame: &WatchFrame, map: &WatchMap) -> bool {
    frame.human_control && map.may_plot
}

/// The question to Jev for the place plain words mean on a map. None for
/// no words.
pub fn place_ask(map: &WatchMap, wish: &str) -> Option<Ask> {
    let wish = wish.trim();
    (!wish.is_empty()).then(|| Ask::PlaceOnMap {
        wish: wish.to_string(),
        map: map.facet,
        from: (map.start_x, map.start_y),
        to: (map.end_x, map.end_y),
    })
}

/// Takes the place Jev found: its tile becomes a pin of the map it was
/// asked for, while that map is open. The words of a failure as an error.
pub fn take_place_answer(map: Option<&WatchMap>, answer: Answer) -> Result<Option<Act>, String> {
    match (answer, map) {
        (Answer::Place(Ok((x, y))), Some(map)) => {
            let (x, y) = pixel_of(map, x, y);
            Ok(Some(Act::MapPin { x, y }))
        }
        (Answer::Place(Err(words)), _) => Err(words),
        // The map item asks only for a place, and its map is gone.
        _ => Ok(None),
    }
}

/// The size of the profile panel.
pub fn profile_size() -> Vector {
    Vector::new(
        PROFILE_WIDTH,
        TITLE_ROW
            + PROFILE_TITLE_ROW
            + PROFILE_ROWS as f32 * PROFILE_LINE
            + FOOT_ROW
            + PANEL_PAD * 2.0,
    )
}

/// Where the profile first stands in a window.
pub fn profile_first_place(window: Area) -> Area {
    first_place(window, Spot::Middle(0), profile_size())
}

/// The words of a profile: what the shard writes, then what the owner
/// wrote.
pub fn profile_words(known: Option<&WatchProfile>) -> String {
    let shard = known.map_or("", |kept| kept.shard_words.as_str());
    let own = known.map_or("", |kept| kept.own_words.as_str());
    if shard.is_empty() {
        own.to_string()
    } else {
        format!("{shard}\n\n{own}")
    }
}

/// The window that shows the profile of a character. Only the owner of a
/// character may change his profile, so the shard refuses the rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfilePanel {
    /// The character whose profile shows, and the words being written.
    pub shown: Option<u32>,
    pub writing: Option<String>,
    /// Show the profile of the character with the first picture.
    show_own: bool,
}

impl ProfilePanel {
    /// The window as it starts. An open one shows the profile of the
    /// character as soon as the first picture names him.
    pub fn starting(show_own: bool) -> Self {
        Self {
            show_own,
            ..Self::default()
        }
    }

    /// Shows the profile of a character. Gives the act that asks the
    /// session for it.
    pub fn show(&mut self, serial: u32) -> Act {
        self.shown = Some(serial);
        self.writing = None;
        Act::ProfileRead(serial)
    }

    pub fn close(&mut self) {
        self.shown = None;
        self.writing = None;
    }

    pub fn shows(&self, serial: u32) -> bool {
        self.shown == Some(serial)
    }

    /// Shows the character's own profile with the first picture that names
    /// him, when the window starts open.
    pub fn follow(&mut self, frame: &WatchFrame) -> Option<Act> {
        (frame.serial != 0 && std::mem::take(&mut self.show_own)).then(|| self.show(frame.serial))
    }

    /// The profile that shows, as the shard sent it.
    pub fn known<'f>(&self, frame: &'f WatchFrame) -> Option<&'f WatchProfile> {
        let serial = self.shown?;
        frame.profiles.iter().find(|kept| kept.serial == serial)
    }

    /// The title of the window: the name of the character, when known.
    pub fn title(&self, frame: &WatchFrame) -> String {
        let name = self.known(frame).map_or("", |kept| kept.name.as_str());
        if name.is_empty() {
            WORDS_PROFILE.to_string()
        } else {
            name.to_string()
        }
    }

    /// The words of the button that writes, while the human has control
    /// of the character whose profile shows.
    pub fn write_words(&self, frame: &WatchFrame) -> Option<&'static str> {
        let mine = self.shown == Some(frame.serial);
        (frame.human_control && mine).then_some(if self.writing.is_some() {
            WORDS_SAVE
        } else {
            WORDS_WRITE
        })
    }

    /// A press on Write or Save: Write opens the words of the owner for
    /// writing, and Save sends them.
    pub fn press_write(&mut self, frame: &WatchFrame) -> Option<Act> {
        self.write_words(frame)?;
        let serial = self.shown?;
        match self.writing.take() {
            Some(words) => Some(Act::ProfileWrite {
                serial,
                text: words,
            }),
            None => {
                let own = self
                    .known(frame)
                    .map_or_else(String::new, |kept| kept.own_words.clone());
                self.writing = Some(own);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> WatchMap {
        WatchMap {
            serial: 60,
            start_x: 1000,
            start_y: 1200,
            end_x: 1400,
            end_y: 1600,
            width: 200,
            height: 200,
            may_plot: true,
            ..WatchMap::default()
        }
    }

    #[test]
    fn the_buttons_and_the_pins_of_a_map_send_their_acts() {
        let map = map();
        assert_eq!(MapItemButton::Plot.words(&map), ("Stop plotting", true));
        assert_eq!(MapItemButton::Close.act(&map), Act::MapClose(60));
        assert_eq!(
            PinDeed::Moved(1, (5, 6)).act(),
            Act::MapPinMove { pin: 1, x: 5, y: 6 }
        );
        assert_eq!(PinDeed::Removed(300).act(), Act::MapPinRemove(u8::MAX));
        assert_eq!(
            take_place_answer(Some(&map), Answer::Place(Ok((1200, 1400)))),
            Ok(Some(Act::MapPin { x: 100, y: 100 }))
        );
        assert_eq!(take_place_answer(None, Answer::Place(Ok((1, 1)))), Ok(None));
        assert!(place_ask(&map, " ").is_none());
    }

    #[test]
    fn the_land_lies_inside_the_paper_above_the_rows() {
        let body = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(200.0, 300.0));
        let (paper, land) = map_item_land(body);
        assert_eq!(
            paper.max.y,
            300.0 - FOOT_ROW + ROW_GAP - GAP * 2.0 - FIELD_ROW
        );
        assert_eq!(land.min, Point::new(PAPER_EDGE, PAPER_EDGE));
    }

    #[test]
    fn only_the_owner_writes_his_profile() {
        let mut frame = WatchFrame {
            serial: 7,
            human_control: true,
            profiles: vec![WatchProfile {
                serial: 7,
                name: "Mara".into(),
                own_words: "Hi".into(),
                ..WatchProfile::default()
            }],
            ..WatchFrame::default()
        };
        let mut panel = ProfilePanel::starting(true);
        assert_eq!(panel.follow(&frame), Some(Act::ProfileRead(7)));
        assert_eq!(panel.follow(&frame), None, "once");
        assert_eq!(panel.title(&frame), "Mara");
        assert_eq!(panel.press_write(&frame), None);
        assert_eq!(panel.writing.as_deref(), Some("Hi"));
        assert_eq!(panel.write_words(&frame), Some(WORDS_SAVE));
        assert_eq!(
            panel.press_write(&frame),
            Some(Act::ProfileWrite {
                serial: 7,
                text: "Hi".into(),
            })
        );
        frame.human_control = false;
        assert_eq!(panel.press_write(&frame), None);
        panel.show(8);
        assert_eq!(panel.title(&frame), WORDS_PROFILE);
    }
}
