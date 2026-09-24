//! The mouse pointers of the classic client. They are item pictures of the
//! art files: one set for peace and one for war, each with a pointer for the
//! eight ways on the screen, the drag hand, the plain pointer, the target
//! cross, the wait glass and the text bar.
//!
//! A pointer picture marks its hot spot with green pixels on its top row and
//! its left column, and its frame with black pixels. Those marks and the
//! outer ring of pixels are not drawn.

use std::collections::HashMap;

use uoterm_protocol::Direction;

use crate::art::{ArtData, ArtPixels, PIXEL_DRAWN};

/// The first pointer graphic of the peace set and of the war set.
pub const CURSOR_PEACE_BASE: u16 = 0x206A;
pub const CURSOR_WAR_BASE: u16 = 0x2053;
/// Away from Felucca the peace pointers take this hue.
pub const CURSOR_OTHER_MAP_HUE: u16 = 0x0033;

const FELUCCA: u8 = 0;
const WAYS: u16 = 8;
const DRAG_SLOT: u16 = 8;
const NORMAL_SLOT: u16 = 9;
const TARGET_SLOT: u16 = 12;
const WAIT_SLOT: u16 = 13;
const EDIT_SLOT: u16 = 14;
const COLOR_MASK: u16 = !PIXEL_DRAWN;
const HOT_SPOT_GREEN: u16 = 0x03E0;
const FRAME_BLACK: u16 = 0;

/// One pointer of the classic client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CursorShape {
    /// The arrow that points a way on the screen while the mouse is over
    /// the world. It names the way a person walks toward the mouse.
    Walk(Direction),
    /// The hand that drags an item or a gump.
    Drag,
    /// The plain pointer, over gumps and when no way is named.
    Normal,
    /// The cross of a target request.
    Target,
    /// The glass of a client that is busy.
    Wait,
    /// The bar over a text box.
    Edit,
}

impl CursorShape {
    /// Every pointer the client shows.
    pub const ALL: [CursorShape; 13] = [
        CursorShape::Walk(Direction::North),
        CursorShape::Walk(Direction::Northeast),
        CursorShape::Walk(Direction::East),
        CursorShape::Walk(Direction::Southeast),
        CursorShape::Walk(Direction::South),
        CursorShape::Walk(Direction::Southwest),
        CursorShape::Walk(Direction::West),
        CursorShape::Walk(Direction::Northwest),
        CursorShape::Drag,
        CursorShape::Normal,
        CursorShape::Target,
        CursorShape::Wait,
        CursorShape::Edit,
    ];

    /// The place of the pointer in its set. The arrows count clockwise from
    /// the one that points up the screen, which is the way north-west.
    fn slot(self) -> u16 {
        match self {
            CursorShape::Walk(way) => (way as u16 + 1) % WAYS,
            CursorShape::Drag => DRAG_SLOT,
            CursorShape::Normal => NORMAL_SLOT,
            CursorShape::Target => TARGET_SLOT,
            CursorShape::Wait => WAIT_SLOT,
            CursorShape::Edit => EDIT_SLOT,
        }
    }
}

/// The art graphic of a pointer, in the war set or the peace set.
pub fn cursor_graphic(shape: CursorShape, war: bool) -> u16 {
    let base = if war {
        CURSOR_WAR_BASE
    } else {
        CURSOR_PEACE_BASE
    };
    base + shape.slot()
}

/// The hue of the pointers: the peace pointers away from Felucca are hued,
/// the rest keep their own colors.
pub fn cursor_hue(war: bool, map_index: u8) -> u16 {
    if !war && map_index != FELUCCA {
        CURSOR_OTHER_MAP_HUE
    } else {
        0
    }
}

/// One pointer picture with its marks taken off, and the pixel of it that
/// points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorPicture {
    pub pixels: ArtPixels,
    pub hot_x: usize,
    pub hot_y: usize,
}

/// Takes the marks off a pointer picture and reads its hot spot.
fn cursor_picture(mut pixels: ArtPixels) -> CursorPicture {
    let (width, height) = (pixels.width, pixels.height);
    let (mut hot_x, mut hot_y) = (0, 0);
    for y in 0..height {
        for x in 0..width {
            let pixel = &mut pixels.colors[y * width + x];
            if *pixel & PIXEL_DRAWN == 0 {
                continue;
            }
            let color = *pixel & COLOR_MASK;
            if color == HOT_SPOT_GREEN {
                if x == 0 {
                    hot_y = y;
                }
                if y == 0 {
                    hot_x = x;
                }
            }
            let edge = x == 0 || y == 0 || x + 1 == width || y + 1 == height;
            if color == HOT_SPOT_GREEN || color == FRAME_BLACK || edge {
                *pixel = 0;
            }
        }
    }
    CursorPicture {
        pixels,
        hot_x,
        hot_y,
    }
}

/// Every pointer of both sets, ready to draw.
#[derive(Clone, Debug, Default)]
pub struct CursorSet {
    pictures: HashMap<(CursorShape, bool), CursorPicture>,
}

impl CursorSet {
    /// Reads every pointer from the art files. A pointer the files do not
    /// hold is left out.
    pub fn load(art: &ArtData) -> Self {
        let pictures = CursorShape::ALL
            .into_iter()
            .flat_map(|shape| [(shape, false), (shape, true)])
            .filter_map(|(shape, war)| {
                let pixels = art.item(cursor_graphic(shape, war))?;
                Some(((shape, war), cursor_picture(pixels)))
            })
            .collect();
        Self { pictures }
    }

    /// One pointer, in the war set or the peace set.
    pub fn get(&self, shape: CursorShape, war: bool) -> Option<&CursorPicture> {
        self.pictures.get(&(shape, war))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: u16 = 0x7FFF | PIXEL_DRAWN;
    const GREEN: u16 = HOT_SPOT_GREEN | PIXEL_DRAWN;
    const BLACK: u16 = FRAME_BLACK | PIXEL_DRAWN;
    const SIDE: usize = 5;

    #[test]
    fn the_arrows_count_from_the_one_that_points_up_the_screen() {
        assert_eq!(
            cursor_graphic(CursorShape::Walk(Direction::Northwest), false),
            0x206A
        );
        assert_eq!(
            cursor_graphic(CursorShape::Walk(Direction::North), false),
            0x206B
        );
        assert_eq!(
            cursor_graphic(CursorShape::Walk(Direction::West), true),
            0x2053 + 7
        );
        assert_eq!(cursor_graphic(CursorShape::Normal, false), 0x2073);
        assert_eq!(cursor_graphic(CursorShape::Target, true), 0x205F);
        assert_eq!(cursor_graphic(CursorShape::Edit, false), 0x2078);
    }

    #[test]
    fn peace_pointers_away_from_felucca_are_hued() {
        assert_eq!(cursor_hue(false, 0), 0);
        assert_eq!(cursor_hue(false, 1), CURSOR_OTHER_MAP_HUE);
        assert_eq!(cursor_hue(true, 1), 0);
    }

    #[test]
    fn the_marks_come_off_and_the_green_marks_name_the_hot_spot() {
        let mut pixels = ArtPixels::clear(SIDE, SIDE);
        pixels.colors[2] = GREEN;
        pixels.colors[3 * SIDE] = GREEN;
        pixels.colors[SIDE + 4] = INK;
        pixels.colors[2 * SIDE + 2] = BLACK;
        pixels.colors[2 * SIDE + 1] = INK;
        let cursor = cursor_picture(pixels);
        assert_eq!((cursor.hot_x, cursor.hot_y), (2, 3));
        assert_eq!(cursor.pixels.colors[2], 0);
        assert_eq!(cursor.pixels.colors[3 * SIDE], 0);
        assert_eq!(cursor.pixels.colors[SIDE + 4], 0);
        assert_eq!(cursor.pixels.colors[2 * SIDE + 2], 0);
        assert_eq!(cursor.pixels.colors[2 * SIDE + 1], INK);
    }

    #[test]
    fn the_real_art_holds_every_pointer_when_client_files_are_here() {
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let cursors = CursorSet::load(&ArtData::open(dir).unwrap());
        for shape in CursorShape::ALL {
            for war in [false, true] {
                let cursor = cursors.get(shape, war).unwrap();
                assert!(cursor.hot_x < cursor.pixels.width);
                assert!(cursor.hot_y < cursor.pixels.height);
                assert!(cursor.pixels.colors.iter().any(|c| c & PIXEL_DRAWN != 0));
            }
        }
    }
}
