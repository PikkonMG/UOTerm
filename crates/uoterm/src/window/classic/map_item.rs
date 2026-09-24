//! A map item, such as a treasure map or a city map, as the
//! reference client draws it: the land of the map on a frame, its pins with their
//! numbers, and the course line from one pin to the next. When the shard
//! lets the player plot a course, a click on the land puts a pin there, a
//! pin drags to a new place, a double click takes it off, and the button
//! under the map clears every pin. The button over the map asks the shard
//! to start or to stop plotting. Closing the gump forgets the map.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::{WatchFrame, WatchMap};
use crate::window::atlas::Sprite;
use crate::window::control::Act;
use crate::window::model::map_item::LandPicture;
use eframe::egui::{Color32, Pos2, Rect, Vec2};

pub const MAP_ITEM: GumpKind = GumpKind {
    id: well_known::MAP_ITEM,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(MapItemGump::new(serial.unwrap_or_default())),
};

const FRAME: u16 = 0x1432;
/// The frame is this much wider and taller than the map.
const FRAME_ROOM: (i32, i32) = (44, 61);
/// Where the land of the map starts in the gump.
const MAP_AT: (i32, i32) = (24, 31);
const PLOT: u16 = 0x1398;
const STOP_PLOTTING: u16 = 0x1399;
const CLEAR_COURSE: u16 = 0x139A;
/// The buttons sit in the middle less these, over and under the map.
const PLOT_ROOM: i32 = 100;
const STOP_ROOM: i32 = 70;
const CLEAR_ROOM: i32 = 66;
const TOP_BUTTON_Y: i32 = 5;
const CLEAR_BELOW_MAP: i32 = 37;
const COMPASS: u16 = 0x139D;
const COMPASS_ROOM: i32 = 20;
const PIN: u16 = 0x139B;
const PIN_HOVER_HUE: u16 = 0x0035;
/// A pin stands this far right of its place, past its own width.
const PIN_GAP: i32 = 5;
/// A click puts the pin this far right of the pointer, as the classic
/// client sends it.
const CLICK_NUDGE: i32 = 5;
const NUMBER_FONT: u8 = 0;
const NUMBER_HUE: u16 = 0;
const NUMBER_GAP: i32 = 1;
const COURSE: Color32 = Color32::WHITE;
const HALF: i32 = 2;
const NO_HUE: u16 = 0;
const TIP_PIN: &str = "Drag to move the pin. Double-click to take it off.";

fn button(art: u16) -> ButtonArt {
    ButtonArt::new(art, art, art)
}

/// The top left of the picture of a pin, from its place on the map, as the
/// classic client puts it.
fn pin_spot(pixel: (u16, u16), pin: (i32, i32)) -> (i32, i32) {
    (
        i32::from(pixel.0) + pin.0 + PIN_GAP,
        i32::from(pixel.1) + pin.1,
    )
}

/// The place on the map of a pin whose picture has its top left at `spot`.
fn pin_pixel(spot: (i32, i32), pin: (i32, i32)) -> (u16, u16) {
    let along = |at: i32| u16::try_from(at.max(0)).unwrap_or(u16::MAX);
    (along(spot.0 - pin.0 - PIN_GAP), along(spot.1 - pin.1))
}

/// The place a click on the land puts a new pin, from the click in gump
/// pixels.
fn clicked_pixel(click: Vec2) -> (u16, u16) {
    let along = |at: f32| u16::try_from(at.max(0.0) as i32).unwrap_or(u16::MAX);
    (
        along(click.x - MAP_AT.0 as f32 + CLICK_NUDGE as f32),
        along(click.y - MAP_AT.1 as f32),
    )
}

/// A dragged pin stays on the land of the map.
fn kept_on_map(spot: Vec2, map: &WatchMap) -> Vec2 {
    Vec2::new(
        spot.x
            .clamp(MAP_AT.0 as f32, (MAP_AT.0 + i32::from(map.width)) as f32),
        spot.y
            .clamp(MAP_AT.1 as f32, (MAP_AT.1 + i32::from(map.height)) as f32),
    )
}

/// A pin the player drags: which one, where its picture is now, and where
/// the pointer holds it.
struct Dragging {
    pin: usize,
    spot: Vec2,
    grip: Vec2,
}

/// What the player did to a pin in one frame.
enum PinDeed {
    Moved(usize, (u16, u16)),
    Removed(usize),
}

pub struct MapItemGump {
    serial: u32,
    land: LandPicture,
    dragging: Option<Dragging>,
}

impl MapItemGump {
    fn new(serial: u32) -> Self {
        Self {
            serial,
            land: LandPicture::default(),
            dragging: None,
        }
    }

    /// The pins, the lines between them and their numbers. Gives what the
    /// player did to one of them.
    fn pins(&mut self, g: &mut Canvas<'_>, map: &WatchMap) -> Option<PinDeed> {
        let size = g.gump_size(PIN).unwrap_or_default();
        let pin_size = (size.x as i32, size.y as i32);
        let spots: Vec<(i32, i32)> = map
            .pins
            .iter()
            .enumerate()
            .map(|(at, pixel)| match &self.dragging {
                Some(drag) if drag.pin == at => (drag.spot.x as i32, drag.spot.y as i32),
                _ => pin_spot(*pixel, pin_size),
            })
            .collect();
        for pair in spots.windows(HALF as usize) {
            g.line(pair[0], pair[1], COURSE);
        }
        let number_look = TextLook::ascii(NUMBER_FONT, NUMBER_HUE);
        let mut deed = None;
        for (at, (x, y)) in spots.into_iter().enumerate() {
            let hue = if g.hovered(x, y, pin_size.0, pin_size.1) {
                PIN_HOVER_HUE
            } else {
                NO_HUE
            };
            let number = (at + 1).to_string();
            let width = g.measure(&number, &number_look).x as i32;
            g.label(x - width - NUMBER_GAP, y, &number, &number_look);
            if !map.may_plot {
                g.pic(x, y, PIN, hue);
                continue;
            }
            let response = g.pic_button(("pin", at), x, y, PIN, hue);
            g.tooltip(TIP_PIN);
            let pointer = response.interact_pointer_pos().map(|at| g.to_gump(at));
            if response.drag_started() {
                let spot = Vec2::new(x as f32, y as f32);
                self.dragging = pointer.map(|pointer| Dragging {
                    pin: at,
                    spot,
                    grip: pointer - spot,
                });
            }
            if let (Some(drag), Some(pointer)) = (self.dragging.as_mut(), pointer) {
                if drag.pin == at && response.dragged() {
                    drag.spot = kept_on_map(pointer - drag.grip, map);
                }
            }
            if response.drag_stopped() {
                if let Some(drag) = self.dragging.take().filter(|drag| drag.pin == at) {
                    let spot = (drag.spot.x as i32, drag.spot.y as i32);
                    deed = Some(PinDeed::Moved(at, pin_pixel(spot, pin_size)));
                }
            } else if response.double_clicked() {
                deed = Some(PinDeed::Removed(at));
            }
        }
        deed
    }
}

impl GumpBody for MapItemGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(map) = cx.frame.maps.iter().find(|map| map.serial == self.serial) else {
            return;
        };
        let (width, height) = (i32::from(map.width), i32::from(map.height));
        g.frame(0, 0, width + FRAME_ROOM.0, height + FRAME_ROOM.1, FRAME);
        let (plot, clear) = if map.may_plot {
            let stop_x = (width - STOP_ROOM) / HALF;
            let stop = g.button("stop", stop_x, TOP_BUTTON_Y, button(STOP_PLOTTING));
            let clear_x = (width - CLEAR_ROOM) / HALF;
            let clear_y = height + CLEAR_BELOW_MAP;
            let clear = g.button("clear", clear_x, clear_y, button(CLEAR_COURSE));
            (stop, clear)
        } else {
            let plot_x = (width - PLOT_ROOM) / HALF;
            (g.button("plot", plot_x, TOP_BUTTON_Y, button(PLOT)), false)
        };
        g.pic(width - COMPASS_ROOM, height - COMPASS_ROOM, COMPASS, NO_HUE);
        let ctx = g.ctx().clone();
        let scene = &mut *g.scene;
        let land = self
            .land
            .texture(&ctx, map, |facet, x, y| scene.radar_rgb(facet, x, y));
        if let Some(texture) = land {
            let sprite = Sprite {
                uv: Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                width: width as f32,
                height: height as f32,
                anchor: Vec2::ZERO,
            };
            g.sprite(MAP_AT.0, MAP_AT.1, texture, sprite);
        }
        let mut acts = Vec::new();
        if map.may_plot {
            let land_click = g.click_area("land", MAP_AT.0, MAP_AT.1, width, height);
            if let Some(at) = land_click
                .interact_pointer_pos()
                .filter(|_| land_click.clicked())
            {
                let (x, y) = clicked_pixel(g.to_gump(at));
                acts.push(Act::MapPin { x, y });
            }
        } else {
            self.dragging = None;
        }
        match self.pins(g, map) {
            Some(PinDeed::Moved(pin, (x, y))) => acts.push(Act::MapPinMove {
                pin: pin as u8,
                x,
                y,
            }),
            Some(PinDeed::Removed(pin)) => acts.push(Act::MapPinRemove(pin as u8)),
            None => {}
        }
        if plot {
            acts.push(Act::MapEdit);
        }
        if clear {
            acts.push(Act::MapClear);
        }
        for act in acts {
            cx.act(act);
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::MapClose(self.serial));
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.maps.iter().any(|map| map.serial == self.serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const SERIAL: u32 = 0x4000_0F01;
    const PIN_SIZE: (i32, i32) = (14, 23);

    fn treasure_map(may_plot: bool) -> WatchMap {
        WatchMap {
            serial: SERIAL,
            start_x: 1000,
            start_y: 1200,
            end_x: 1400,
            end_y: 1600,
            width: 200,
            height: 200,
            may_plot,
            pins: vec![(40, 90), (100, 20)],
            ..WatchMap::default()
        }
    }

    #[test]
    fn a_pin_stands_where_the_classic_client_puts_it_and_comes_back() {
        let spot = pin_spot((40, 90), PIN_SIZE);
        assert_eq!(spot, (40 + 14 + PIN_GAP, 90 + 23));
        assert_eq!(pin_pixel(spot, PIN_SIZE), (40, 90));
        assert_eq!(pin_pixel((0, 0), PIN_SIZE), (0, 0), "never off the map");
    }

    #[test]
    fn a_click_on_the_land_names_the_pixel_of_the_new_pin() {
        let click = Vec2::new(MAP_AT.0 as f32 + 10.0, MAP_AT.1 as f32 + 20.0);
        assert_eq!(clicked_pixel(click), (10 + CLICK_NUDGE as u16, 20));
        assert_eq!(clicked_pixel(Vec2::ZERO), (0, 0));
    }

    #[test]
    fn a_dragged_pin_stays_on_the_land() {
        let map = treasure_map(true);
        let far = kept_on_map(Vec2::new(900.0, -5.0), &map);
        assert_eq!(far, Vec2::new(MAP_AT.0 as f32 + 200.0, MAP_AT.1 as f32));
        let inside = Vec2::new(50.0, 60.0);
        assert_eq!(kept_on_map(inside, &map), inside);
    }

    #[test]
    fn the_gump_lives_while_its_map_is_open() {
        let gump = MapItemGump::new(SERIAL);
        let mut frame = WatchFrame::default();
        assert!(!gump.alive(&frame));
        frame.maps.push(treasure_map(false));
        assert!(gump.alive(&frame));
        assert!(!MapItemGump::new(SERIAL + 1).alive(&frame));
    }

    #[test]
    fn a_map_draws_with_the_client_files_while_plotting_or_not() {
        for may_plot in [false, true] {
            let frame = WatchFrame {
                maps: vec![treasure_map(may_plot)],
                ..WatchFrame::default()
            };
            let mut profile = Profile::default();
            let mut manager = GumpManager::default();
            let id = GumpId::of(well_known::MAP_ITEM, SERIAL);
            manager.open(id, &mut profile);
            if !draw_frames(&mut manager, &mut profile, &frame) {
                return;
            }
            assert!(manager.is_open(&id));
            draw_frames(&mut manager, &mut profile, &WatchFrame::default());
            assert!(!manager.is_open(&id), "the map closed");
        }
    }
}
