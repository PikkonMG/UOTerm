//! What the mouse points at on the map: the things drawn in the last frame,
//! the tile under it and the height of its floor.

use super::{SceneState, HALF_TILE, PAWN_RING_RX, PAWN_RING_RY, PAWN_TOP_Y, Z_PIXELS};
use crate::art::WorldArt;
use crate::clicks::PickKind;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Vector};

// The search for the tile under the mouse. A floor above the character is
// drawn higher on the screen, so its tile is some rows further down the map.
const PICK_ROW_STEP: f32 = 0.5;
const PICK_ROWS_UP: i32 = -12;
const PICK_ROWS_DOWN: i32 = 40;

/// One thing on the map that the mouse can point at.
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    /// The screen area it was drawn in.
    pub area: Area,
    pub serial: u32,
    pub name: String,
    pub kind: PickKind,
}

/// The screen area of a figure with no picture, from the top of its head
/// to its feet.
pub(super) fn figure_area(foot: Point, zoom: f32) -> Area {
    Area {
        min: foot + Vector::new(-PAWN_RING_RX, PAWN_TOP_Y) * zoom,
        max: foot + Vector::new(PAWN_RING_RX, PAWN_RING_RY) * zoom,
    }
}

impl SceneState {
    /// The things drawn in the last frame, in paint order. The last one is
    /// on top.
    pub fn picks(&self) -> &[Pick] {
        &self.picks
    }

    /// The thing on top under the mouse.
    pub fn thing_at(&self, at: Point) -> Option<&Pick> {
        self.picks.iter().rev().find(|pick| pick.area.contains(at))
    }

    /// The mobiles drawn in a box of the screen, first drawn first.
    pub fn mobiles_in(&self, area: Area) -> Vec<u32> {
        self.picks
            .iter()
            .filter(|pick| pick.kind == PickKind::Mobile && pick.area.intersects(area))
            .map(|pick| pick.serial)
            .collect()
    }

    /// The area the character covers, which the names of the Modern style
    /// keep clear of.
    pub fn character_area(&self, view: Area) -> Area {
        figure_area(self.screen_of(view, self.camera), self.zoom)
    }

    /// The point over a mobile or a thing that is drawn now, where the
    /// words it says float.
    pub fn head_of(&self, view: Area, frame: &WatchFrame, serial: u32) -> Option<Point> {
        if serial == frame.serial {
            return Some(self.character_area(view).center_top());
        }
        self.picks
            .iter()
            .find(|pick| pick.serial == serial)
            .map(|pick| pick.area.center_top())
    }

    /// The tile under the mouse, and the height of its floor. The ground is
    /// not flat, so the tile is the nearest one whose own floor lies under
    /// the mouse. With no client files the floor of the character serves.
    pub fn tile_at(
        &self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        at: Point,
    ) -> (u16, u16, i8) {
        let on_plane = (at - self.projection(view).center()) / (HALF_TILE * self.zoom);
        let camera = self.camera;
        let tile_on_row = |rows_down: f32| {
            let sum = on_plane.y + rows_down;
            let x = camera[0] + (on_plane.x + sum) / 2.0;
            let y = camera[1] + (sum - on_plane.x) / 2.0;
            (x.round().max(0.0) as u16, y.round().max(0.0) as u16)
        };
        let level = (tile_on_row(0.0), frame.z);
        let mut best = level;
        for step in PICK_ROWS_UP..=PICK_ROWS_DOWN {
            let rows_down = step as f32 * PICK_ROW_STEP;
            let (x, y) = tile_on_row(rows_down);
            let Some(floor) = floor_near(art, frame, x, y) else {
                continue;
            };
            let lift_rows = (f32::from(floor) - camera[2]) * Z_PIXELS / HALF_TILE;
            if (lift_rows - rows_down).abs() <= PICK_ROW_STEP {
                best = ((x, y), floor);
            }
        }
        (best.0 .0, best.0 .1, best.1)
    }
}

/// The floor of a tile that is nearest to the height of the character.
fn floor_near(art: &mut dyn WorldArt, frame: &WatchFrame, x: u16, y: u16) -> Option<i8> {
    let cell = art.cell(frame.map, x, y).ready()?;
    let land = cell.land_id.map(|_| i16::from(cell.corners[0]));
    let here = i16::from(frame.z);
    cell.statics
        .iter()
        .filter_map(|s| s.floor)
        .chain(land)
        .min_by_key(|floor| (floor - here).abs())
        .map(|floor| floor.clamp(i16::from(i8::MIN), i16::from(i8::MAX)) as i8)
}

#[cfg(test)]
mod tests {
    use super::super::test_art::NoFiles;
    use super::*;

    const WINDOW: Area = Area {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 100.0, y: 100.0 },
    };
    const MIDDLE: Point = Point::new(50.0, 50.0);

    #[test]
    fn the_tile_under_the_mouse_is_the_tile_that_is_drawn_there() {
        let mut art = NoFiles::default();
        let mut scene = SceneState::new();
        let frame = WatchFrame {
            x: 100,
            y: 100,
            z: 5,
            ..WatchFrame::default()
        };
        scene.follow(&art, &frame, 0.0);
        assert_eq!(
            scene.tile_at(&mut art, WINDOW, &frame, MIDDLE),
            (100, 100, 5)
        );
        let one_east_one_north = MIDDLE + Vector::new(HALF_TILE * 2.0, 0.0);
        assert_eq!(
            scene.tile_at(&mut art, WINDOW, &frame, one_east_one_north),
            (101, 99, 5)
        );
        let drawn_at = scene.screen_of(WINDOW, [97.0, 104.0, 5.0]);
        assert_eq!(
            scene.tile_at(&mut art, WINDOW, &frame, drawn_at),
            (97, 104, 5)
        );
    }

    #[test]
    fn the_thing_on_top_is_the_one_drawn_last() {
        let mut scene = SceneState::new();
        let pick = |serial, kind| Pick {
            area: WINDOW,
            serial,
            name: String::new(),
            kind,
        };
        scene.picks = vec![pick(1, PickKind::Mobile), pick(2, PickKind::Item)];
        assert_eq!(scene.thing_at(MIDDLE).map(|pick| pick.serial), Some(2));
        assert_eq!(scene.mobiles_in(WINDOW), [1]);
        let away = WINDOW.translate(Vector::new(500.0, 0.0));
        assert!(scene.mobiles_in(away).is_empty());
    }
}
