//! Where each picture goes in one large texture that holds every picture
//! the map has drawn, so the whole map is one mesh and one draw call. The
//! pictures stand on shelves: a row of pictures side by side, and the next
//! row under the tallest of them. A window keeps the texture itself.

use crate::geom::{Area, Point};

/// The side of the texture, in pixels.
pub const ATLAS_SIDE: usize = 4096;
/// Clear pixels between two pictures, so one never bleeds into the next when
/// the map is zoomed.
pub const GUTTER: usize = 1;
/// The side of the plain white square in the top left corner.
pub const WHITE_SIDE: usize = 4;

/// Where one picture lies in the texture, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// Puts pictures on shelves in a square texture. The first place is a plain
/// white square: a shape with no picture takes its color from its vertices
/// alone when it reads that square.
#[derive(Clone, Debug)]
pub struct ShelfPacker {
    side: usize,
    shelf_x: usize,
    shelf_y: usize,
    shelf_height: usize,
    white: Placement,
}

impl ShelfPacker {
    pub fn new(side: usize) -> Self {
        let mut packer = Self {
            side,
            shelf_x: 0,
            shelf_y: 0,
            shelf_height: 0,
            white: Placement {
                x: 0,
                y: 0,
                width: WHITE_SIDE,
                height: WHITE_SIDE,
            },
        };
        packer.reset();
        packer
    }

    /// Forgets every picture. Only the white square stays.
    pub fn reset(&mut self) {
        self.shelf_x = 0;
        self.shelf_y = 0;
        self.shelf_height = 0;
        if let Some(white) = self.place(WHITE_SIDE, WHITE_SIDE) {
            self.white = white;
        }
    }

    /// The plain white square.
    pub fn white(&self) -> Placement {
        self.white
    }

    /// True when a picture of this size fits an empty texture.
    pub fn fits(&self, width: usize, height: usize) -> bool {
        width + GUTTER <= self.side && height + GUTTER <= self.side
    }

    /// The place of the next picture. None when it does not fit in the room
    /// left: then the texture is full until [`ShelfPacker::reset`].
    pub fn place(&mut self, width: usize, height: usize) -> Option<Placement> {
        if !self.fits(width, height) {
            return None;
        }
        let (mut x, mut y) = (self.shelf_x, self.shelf_y);
        let mut shelf_height = self.shelf_height;
        if x + width + GUTTER > self.side {
            y += shelf_height + GUTTER;
            x = 0;
            shelf_height = 0;
        }
        if y + height + GUTTER > self.side {
            return None;
        }
        self.shelf_x = x + width + GUTTER;
        self.shelf_y = y;
        self.shelf_height = shelf_height.max(height);
        Some(Placement {
            x,
            y,
            width,
            height,
        })
    }

    /// The part of the texture a placement covers, from 0 to 1 on each side.
    pub fn uv(&self, placement: Placement) -> Area {
        let side = self.side as f32;
        Area {
            min: Point::new(placement.x as f32 / side, placement.y as f32 / side),
            max: Point::new(
                (placement.x + placement.width) as f32 / side,
                (placement.y + placement.height) as f32 / side,
            ),
        }
    }

    /// A point in the middle of the white square.
    pub fn white_uv(&self) -> Point {
        self.uv(self.white).center()
    }
}

#[cfg(test)]
mod packer_tests {
    use super::*;

    #[test]
    fn pictures_fill_a_shelf_then_open_the_next() {
        // The white square and a gutter take the first five pixels.
        let mut packer = ShelfPacker::new(64);
        let a = packer.place(20, 10).unwrap();
        let b = packer.place(20, 12).unwrap();
        let c = packer.place(20, 5).unwrap();
        assert_eq!((a.y, b.y), (a.y, a.y));
        assert!(c.y > a.y);
        assert_eq!(c.y, 12 + GUTTER, "under the tallest of the shelf");
    }

    #[test]
    fn a_full_atlas_refuses_until_reset() {
        let mut packer = ShelfPacker::new(16);
        assert!(packer.place(64, 64).is_none());
        assert!(packer.place(8, 8).is_some());
        assert!(packer.place(8, 8).is_none(), "the room is used up");
        packer.reset();
        assert!(packer.place(8, 8).is_some());
    }

    #[test]
    fn the_white_square_is_in_the_corner_and_its_middle_is_white() {
        let packer = ShelfPacker::new(ATLAS_SIDE);
        assert_eq!((packer.white().x, packer.white().y), (0, 0));
        let middle = (WHITE_SIDE / 2) as f32 / ATLAS_SIDE as f32;
        assert_eq!(packer.white_uv(), Point::new(middle, middle));
    }
}
