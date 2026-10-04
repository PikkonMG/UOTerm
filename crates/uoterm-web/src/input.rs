//! The input of the page: keys by their egui names, the pointer buttons, the
//! wheel, the controller, which field has the keys, and the small actions
//! of the panels. The view keeps the events of one frame until the next
//! tick reads them, as egui gives a window the events of its frame.

use serde::Deserialize;
use serde_json::Value;
use uoterm_view::geom::Point;
use uoterm_view::input::{KeyName, KeyPress, Mods, PointerButton};
use uoterm_view::keys::Focus;
use uoterm_view::pad::PadButton;
use uoterm_view::scene::{WHEEL_POINTS_PER_NOTCH, WHEEL_ZOOM_PER_POINT};

/// One event of the page.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind")]
pub enum InputEvent {
    Key {
        key: KeyName,
        #[serde(default)]
        mods: Mods,
        pressed: bool,
        #[serde(default)]
        repeat: bool,
    },
    /// Words typed: the page gives the characters a key types.
    Text { text: String },
    /// A button went down at `x`, `y`. `double` is the second press of a
    /// double click. A press with no place is one of a key or a pad.
    PointerDown {
        button: PointerButton,
        #[serde(default)]
        mods: Mods,
        #[serde(default)]
        double: bool,
        #[serde(default)]
        x: Option<f32>,
        #[serde(default)]
        y: Option<f32>,
    },
    /// A button came up at `x`, `y`: a click there.
    PointerUp {
        x: f32,
        y: f32,
        button: PointerButton,
        #[serde(default)]
        mods: Mods,
    },
    /// The wheel turned this many notches: positive away from the player,
    /// which is the other sign of a browser's `deltaY`. With Shift held it
    /// turns to the side, as in egui, and the map takes none of it.
    Wheel {
        notches: f32,
        #[serde(default)]
        mods: Mods,
    },
    /// The controller now: the sticks as left x, left y, right x, right y
    /// (y up), and the buttons down in the order they went down.
    Pad {
        sticks: [f32; 4],
        buttons: Vec<PadButton>,
    },
    /// The words in the field of the chat line now. The field types them
    /// itself, as an egui text field does.
    ChatWords { text: String },
    /// Which field has the keys. The words of the chat line live in the
    /// view, so the view knows whether it is empty.
    Focus {
        chat_focused: bool,
        other_field_focused: bool,
    },
    /// A button of a panel: the panel and its own small action.
    Panel { panel: String, action: Value },
}

/// A button that comes up this far from where it went down, in points, or
/// this long after, makes no click, as egui counts a click: it was a drag.
pub const CLICK_DISTANCE: f32 = 6.0;
pub const CLICK_SECONDS: f64 = 0.8;

/// A click of the primary button, where it was let go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Click {
    pub at: Point,
    /// The second click of a double click.
    pub double: bool,
    pub mods: Mods,
}

/// The input of one frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameInput {
    pub presses: Vec<KeyPress>,
    pub texts: Vec<String>,
    pub clicks: Vec<Click>,
    /// Where clicks of the secondary button came up.
    pub menu_clicks: Vec<Point>,
    pub primary_pressed: bool,
    pub secondary_pressed: bool,
    /// How far the wheel turned, in points of `SceneInput::scroll`.
    pub scroll: f32,
    /// How much the wheel with Ctrl held zooms. One is no zoom.
    pub zoom_delta: f32,
}

impl Default for FrameInput {
    fn default() -> Self {
        Self {
            presses: Vec::new(),
            texts: Vec::new(),
            clicks: Vec::new(),
            menu_clicks: Vec::new(),
            primary_pressed: false,
            secondary_pressed: false,
            scroll: 0.0,
            zoom_delta: 1.0,
        }
    }
}

/// What the page told of its input, kept from one event to the next.
#[derive(Default)]
pub struct Inputs {
    frame: FrameInput,
    keys_down: Vec<KeyName>,
    mods: Mods,
    primary_down: bool,
    secondary_down: bool,
    /// The last press of the primary button was the second of a double
    /// click.
    double_down: bool,
    /// Where and when each button went down, when the page told where.
    primary_press: Option<(Point, f64)>,
    secondary_press: Option<(Point, f64)>,
    /// The drag of the primary button was told to the map.
    drag_told: bool,
    chat_focused: bool,
    other_field_focused: bool,
    sticks: [f32; 4],
    pad_buttons: Vec<PadButton>,
}

impl Inputs {
    /// Keeps one event for the next frame, at `now`. The words of the chat
    /// line and a panel action are the view's to take; they are not kept
    /// here.
    pub fn read(&mut self, event: InputEvent, now: f64) {
        match event {
            InputEvent::Key {
                key,
                mods,
                pressed,
                repeat,
            } => {
                self.mods = mods;
                if pressed {
                    if !self.keys_down.contains(&key) {
                        self.keys_down.push(key.clone());
                    }
                } else {
                    self.keys_down.retain(|down| *down != key);
                }
                self.frame.presses.push(KeyPress {
                    key,
                    mods,
                    pressed,
                    repeat,
                });
            }
            InputEvent::Text { text } => self.frame.texts.push(text),
            InputEvent::PointerDown {
                button,
                mods,
                double,
                x,
                y,
            } => {
                self.mods = mods;
                let pressed = x.zip(y).map(|(x, y)| (Point::new(x, y), now));
                match button {
                    PointerButton::Primary => {
                        self.primary_down = true;
                        self.double_down = double;
                        self.frame.primary_pressed = true;
                        self.primary_press = pressed;
                        self.drag_told = false;
                    }
                    PointerButton::Secondary => {
                        self.secondary_down = true;
                        self.frame.secondary_pressed = true;
                        self.secondary_press = pressed;
                    }
                    PointerButton::Middle => {}
                }
            }
            InputEvent::PointerUp { x, y, button, mods } => {
                self.mods = mods;
                let at = Point::new(x, y);
                match button {
                    PointerButton::Primary if self.primary_down => {
                        self.primary_down = false;
                        if is_click(self.primary_press.take(), at, now) {
                            self.frame.clicks.push(Click {
                                at,
                                double: self.double_down,
                                mods,
                            });
                        }
                    }
                    PointerButton::Primary => {}
                    PointerButton::Secondary => {
                        self.secondary_down = false;
                        if let Some(pressed) = self.secondary_press.take() {
                            if is_click(Some(pressed), at, now) {
                                self.frame.menu_clicks.push(at);
                            }
                        }
                    }
                    PointerButton::Middle => {}
                }
            }
            InputEvent::Wheel { notches, mods } => {
                self.mods = mods;
                // egui turns a wheel with Shift held to the side: the map
                // neither scrolls nor zooms by it.
                if mods.shift {
                    return;
                }
                let points = notches * WHEEL_POINTS_PER_NOTCH;
                if mods.ctrl || mods.command {
                    self.frame.zoom_delta *= (WHEEL_ZOOM_PER_POINT * points).exp();
                } else {
                    self.frame.scroll += points;
                }
            }
            InputEvent::Pad { sticks, buttons } => {
                self.sticks = sticks;
                self.pad_buttons = buttons;
            }
            InputEvent::Focus {
                chat_focused,
                other_field_focused,
            } => {
                self.chat_focused = chat_focused;
                self.other_field_focused = other_field_focused;
            }
            InputEvent::ChatWords { .. } | InputEvent::Panel { .. } => {}
        }
    }

    /// The input of the frame that ends now.
    pub fn take_frame(&mut self) -> FrameInput {
        std::mem::take(&mut self.frame)
    }

    /// Where the keys go, with the chat line empty or not.
    pub fn focus(&self, chat_empty: bool) -> Focus {
        Focus::of(self.chat_focused, chat_empty, self.other_field_focused)
    }

    pub fn keys_down(&self) -> &[KeyName] {
        &self.keys_down
    }

    pub fn mods(&self) -> Mods {
        self.mods
    }

    pub fn secondary_down(&self) -> bool {
        self.secondary_down
    }

    pub fn primary_down(&self) -> bool {
        self.primary_down
    }

    /// Where the primary button went down, the first time the mouse at
    /// `mouse` is far enough from there that it drags; None otherwise.
    pub fn drag_start(&mut self, mouse: Option<Point>) -> Option<Point> {
        let (from, _) = self.primary_press?;
        let mouse = mouse?;
        if !self.primary_down || self.drag_told || (mouse - from).length() <= CLICK_DISTANCE {
            return None;
        }
        self.drag_told = true;
        Some(from)
    }

    /// The controller as the page last read it.
    pub fn pad(&self) -> ([f32; 4], &[PadButton]) {
        (self.sticks, &self.pad_buttons)
    }
}

/// True when a button that went down at `pressed` and came up at `at` at
/// `now` makes a click. A press of no place clicks where it comes up.
fn is_click(pressed: Option<(Point, f64)>, at: Point, now: f64) -> bool {
    pressed.is_none_or(|(from, when)| {
        (at - from).length() <= CLICK_DISTANCE && now - when <= CLICK_SECONDS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(value: Value) -> InputEvent {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn a_button_that_moved_away_drags_and_makes_no_click() {
        let mut inputs = Inputs::default();
        inputs.read(
            event(json!({ "kind": "PointerDown", "button": "Primary", "x": 10, "y": 10 })),
            0.0,
        );
        assert_eq!(inputs.drag_start(Some(Point::new(12.0, 10.0))), None);
        let far = Some(Point::new(40.0, 10.0));
        assert_eq!(inputs.drag_start(far), Some(Point::new(10.0, 10.0)));
        assert_eq!(inputs.drag_start(far), None, "a drag starts once");
        inputs.read(
            event(json!({ "kind": "PointerUp", "x": 40, "y": 10, "button": "Primary" })),
            0.1,
        );
        assert!(inputs.take_frame().clicks.is_empty());
    }

    #[test]
    fn a_short_right_click_asks_for_the_menu_and_a_long_hold_does_not() {
        let mut inputs = Inputs::default();
        let down = json!({ "kind": "PointerDown", "button": "Secondary", "x": 5, "y": 5 });
        let up = json!({ "kind": "PointerUp", "x": 5, "y": 5, "button": "Secondary" });
        inputs.read(event(down.clone()), 0.0);
        inputs.read(event(up.clone()), 0.1);
        assert_eq!(inputs.take_frame().menu_clicks, [Point::new(5.0, 5.0)]);
        inputs.read(event(down), 1.0);
        inputs.read(event(up), 1.0 + CLICK_SECONDS * 2.0);
        assert!(inputs.take_frame().menu_clicks.is_empty(), "it walked");
    }

    #[test]
    fn the_events_of_the_page_read_by_their_kind() {
        let key = event(json!({ "kind": "Key", "key": "F1", "pressed": true }));
        assert_eq!(
            key,
            InputEvent::Key {
                key: KeyName("F1".into()),
                mods: Mods::default(),
                pressed: true,
                repeat: false
            }
        );
        let pad = event(json!({ "kind": "Pad", "sticks": [0, 0, 0, 0], "buttons": ["South"] }));
        assert_eq!(
            pad,
            InputEvent::Pad {
                sticks: [0.0; 4],
                buttons: vec![PadButton::South]
            }
        );
        let panel = event(json!({ "kind": "Panel", "panel": "hotbar", "action": { "press": 0 } }));
        assert!(matches!(panel, InputEvent::Panel { panel, .. } if panel == "hotbar"));
    }

    #[test]
    fn a_press_and_a_release_make_a_click_and_a_held_key_stays_down() {
        let mut inputs = Inputs::default();
        inputs.read(
            event(json!({ "kind": "Key", "key": "Up", "pressed": true })),
            0.0,
        );
        inputs.read(
            event(json!({ "kind": "PointerDown", "button": "Primary", "double": true })),
            0.0,
        );
        inputs.read(
            event(json!({ "kind": "PointerUp", "x": 3, "y": 4, "button": "Primary" })),
            0.0,
        );
        inputs.read(event(json!({ "kind": "Wheel", "notches": 1 })), 0.0);
        let frame = inputs.take_frame();
        assert_eq!(
            frame.clicks,
            [Click {
                at: Point::new(3.0, 4.0),
                double: true,
                mods: Mods::default()
            }]
        );
        assert!(frame.primary_pressed);
        assert_eq!(frame.scroll, WHEEL_POINTS_PER_NOTCH);
        assert_eq!(inputs.keys_down(), [KeyName("Up".into())]);
        assert_eq!(inputs.take_frame(), FrameInput::default(), "one frame only");
    }

    #[test]
    fn the_wheel_with_ctrl_zooms_and_scrolls_nothing() {
        let mut inputs = Inputs::default();
        inputs.read(
            event(
                json!({ "kind": "Wheel", "notches": 1, "mods": { "ctrl": true, "alt": false, "shift": false, "command": true } }),
            ),
            0.0,
        );
        let frame = inputs.take_frame();
        assert_eq!(frame.scroll, 0.0);
        assert!(frame.zoom_delta > 1.0);
    }

    /// egui turns a wheel with Shift held to the side, so it neither
    /// scrolls nor zooms the map.
    #[test]
    fn the_wheel_with_shift_scrolls_and_zooms_nothing() {
        let mut inputs = Inputs::default();
        for ctrl in [false, true] {
            inputs.read(
                event(
                    json!({ "kind": "Wheel", "notches": 1, "mods": { "ctrl": ctrl, "alt": false, "shift": true, "command": ctrl } }),
                ),
                0.0,
            );
            assert_eq!(inputs.take_frame(), FrameInput::default(), "ctrl {ctrl}");
        }
    }
}
