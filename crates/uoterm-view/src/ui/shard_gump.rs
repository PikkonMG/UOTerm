//! A gump of the shard (0xB0 and 0xDD), apart from how it draws: the page
//! that shows, the boxes the player ticked, the words he typed in its
//! fields, which pieces a `checkertrans` fades, where the item of a
//! `buttontileart` sits, and the answer a button sends. The pages, the
//! boxes and the fields work at all times; a reply goes to the shard only
//! while the human has control, which the window checks. The Rust window
//! draws the gump with the art of the client, and the browser with the same
//! pictures.

use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Vector};
use crate::ui::gumps::{click_box, layout_ticked, on_page, FIRST_PAGE};
use crate::ui::text_field::TextField;
use std::collections::HashMap;
use uoterm_world::{GumpLayout, GumpPiece, GumpPieceKind, GumpTileArt};

/// A text entry with no limit of its own takes this many chars.
pub const ENTRY_MAX_CHARS: usize = u8::MAX as usize;
/// Words under a `checkertrans` show at this share of their opacity.
pub const UNDER_VEIL: f32 = 0.5;
/// A `checkertrans` is black glass this opaque.
pub const VEIL_ALPHA: f32 = 0.5;
/// Half the room round an item of a `buttontileart`.
const HALF: i32 = 2;

/// A picture a piece is the size of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceArt {
    Gump(u16),
    Item(u16),
}

/// The size a piece takes on the gump, as `checkertrans` measures it.
/// `size_of` gives the size of a picture, when the files have it.
pub fn piece_size(
    piece: &GumpPiece,
    size_of: &mut impl FnMut(PieceArt) -> Option<Vector>,
) -> Vector {
    let size = |w: i32, h: i32| Vector::new(w as f32, h as f32);
    let mut art = |art: PieceArt| size_of(art).unwrap_or(Vector::ZERO);
    match &piece.what {
        GumpPieceKind::Background { w, h, .. }
        | GumpPieceKind::Tiled { w, h, .. }
        | GumpPieceKind::Words { w, h, .. }
        | GumpPieceKind::Entry { w, h, .. }
        | GumpPieceKind::Veil { w, h } => size(*w, *h),
        GumpPieceKind::Image { gump, .. } => art(PieceArt::Gump(*gump)),
        GumpPieceKind::Button {
            normal, art: tile, ..
        } => match tile {
            Some(tile) if tile.w > 0 && tile.h > 0 => size(tile.w, tile.h),
            _ => art(PieceArt::Gump(*normal)),
        },
        GumpPieceKind::Choice { off, .. } => art(PieceArt::Gump(*off)),
        GumpPieceKind::Item { graphic, .. } => art(PieceArt::Item(*graphic)),
    }
}

/// Which pieces a later `checkertrans` of their page lies over, as the
/// classic client fades them.
pub fn veiled(
    shown: &[&GumpPiece],
    mut size_of: impl FnMut(PieceArt) -> Option<Vector>,
) -> Vec<bool> {
    let areas: Vec<Area> = shown
        .iter()
        .map(|piece| {
            Area::from_min_size(
                Point::new(piece.x as f32, piece.y as f32),
                piece_size(piece, &mut size_of),
            )
        })
        .collect();
    let mut faded = vec![false; shown.len()];
    for (at, piece) in shown.iter().enumerate() {
        if !matches!(piece.what, GumpPieceKind::Veil { .. }) {
            continue;
        }
        for before in 0..at {
            let on_page = shown[before].page == 0 || shown[before].page == piece.page;
            faded[before] |= on_page && areas[before].intersects(areas[at]);
        }
    }
    faded
}

/// Where the item of a `buttontileart` at `x`, `y` sits: in the middle of
/// the button, for an item picture of `item`.
pub fn tile_art_place(x: i32, y: i32, tile: &GumpTileArt, item: Vector) -> (i32, i32) {
    (
        x + ((tile.w - item.x as i32) / HALF).max(0),
        y + ((tile.h - item.y as i32) / HALF).max(0),
    )
}

/// What the human did to one gump of the shard before he answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct ShardGumpState {
    pub gump: u32,
    pub page: u32,
    /// The boxes the human set, by their switch.
    ticks: HashMap<u32, bool>,
    fields: HashMap<u16, TextField>,
    /// The first field took the keys already.
    focused: bool,
}

impl ShardGumpState {
    pub fn new(gump: u32) -> Self {
        Self {
            gump,
            page: FIRST_PAGE,
            ticks: HashMap::new(),
            fields: HashMap::new(),
            focused: false,
        }
    }

    /// The layout of the gump the shard has open, while it has it.
    pub fn layout<'f>(&self, frame: &'f WatchFrame) -> Option<&'f GumpLayout> {
        frame.gump_layouts.iter().find(|l| l.gump == self.gump)
    }

    /// The pieces of the page that shows, in the order they draw.
    pub fn shown<'l>(&self, layout: &'l GumpLayout) -> Vec<&'l GumpPiece> {
        layout
            .pieces
            .iter()
            .filter(|piece| on_page(piece.page, self.page))
            .collect()
    }

    /// True when the box of `switch` is ticked now.
    pub fn is_ticked(&self, layout: &GumpLayout, switch: u32) -> bool {
        layout_ticked(layout, &self.ticks).contains(&switch)
    }

    /// A click on a box: a radio box clears the others of its group.
    pub fn click_box(&mut self, layout: &GumpLayout, piece: &GumpPiece) {
        click_box(layout, &mut self.ticks, piece);
    }

    /// A press on a button. A button that turns the page turns it here;
    /// gives the button that answers the gump, when it is one.
    pub fn press(&mut self, piece: &GumpPiece) -> Option<u32> {
        let GumpPieceKind::Button { id, to_page, .. } = &piece.what else {
            return None;
        };
        match (id, to_page) {
            (Some(id), _) => Some(*id),
            (None, Some(to_page)) => {
                self.page = *to_page;
                None
            }
            (None, None) => None,
        }
    }

    /// The field of an entry, made with its first words the first time.
    pub fn field(&mut self, id: u16, text: &str, limit: Option<u32>) -> &mut TextField {
        let max = limit.map_or(ENTRY_MAX_CHARS, |limit| limit as usize);
        self.fields
            .entry(id)
            .or_insert_with(|| TextField::new(text).with_max_chars(Some(max)))
    }

    /// The words of a field the human typed in, when he did.
    pub fn typed(&self, id: u16) -> Option<&str> {
        self.fields.get(&id).map(TextField::text)
    }

    /// True the first time it is asked: the first field takes the keys
    /// once, when the gump opens.
    pub fn take_focus(&mut self) -> bool {
        !std::mem::replace(&mut self.focused, true)
    }

    /// The answer of a button: its id, the boxes that are ticked, and the
    /// words of every field.
    pub fn answer(&self, layout: &GumpLayout, button: u32) -> Act {
        Act::GumpButton {
            gump: self.gump,
            button,
            switches: layout_ticked(layout, &self.ticks),
            texts: self
                .fields
                .iter()
                .map(|(id, field)| (*id, field.text().to_string()))
                .collect(),
        }
    }

    /// A right click answers the gump with no button, as the classic
    /// client does. It stays until the shard takes it away.
    pub fn close(&self) -> Act {
        Act::GumpClose(self.gump)
    }
}

/// Where the shard put a gump on the screen.
pub fn gump_first_place(layout: &GumpLayout) -> Point {
    Point::new(layout.x as f32, layout.y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(page: u32, x: i32, y: i32, what: GumpPieceKind) -> GumpPiece {
        GumpPiece {
            page,
            x,
            y,
            what,
            tooltip: None,
            property: None,
        }
    }

    fn button(id: Option<u32>, to_page: Option<u32>) -> GumpPiece {
        piece(
            0,
            0,
            0,
            GumpPieceKind::Button {
                normal: 1,
                pressed: 2,
                id,
                to_page,
                art: None,
            },
        )
    }

    #[test]
    fn a_veil_fades_the_pieces_of_its_page_under_it() {
        let image = piece(1, 0, 0, GumpPieceKind::Image { gump: 5, hue: 0 });
        let far = piece(1, 100, 100, GumpPieceKind::Image { gump: 5, hue: 0 });
        let veil = piece(1, 0, 0, GumpPieceKind::Veil { w: 20, h: 20 });
        let after = piece(1, 0, 0, GumpPieceKind::Image { gump: 5, hue: 0 });
        let shown = [&image, &far, &veil, &after];
        let faded = veiled(&shown, |_| Some(Vector::new(10.0, 10.0)));
        assert_eq!(faded, vec![true, false, false, false]);
    }

    #[test]
    fn a_page_button_turns_the_page_and_an_answer_carries_ticks_and_words() {
        let layout = GumpLayout {
            gump: 9,
            pieces: vec![piece(
                0,
                0,
                0,
                GumpPieceKind::Choice {
                    off: 210,
                    on: 211,
                    switch: 4,
                    radio: false,
                    ticked: false,
                    group: 0,
                },
            )],
            ..GumpLayout::default()
        };
        let mut state = ShardGumpState::new(9);
        assert_eq!(state.press(&button(None, Some(3))), None);
        assert_eq!(state.page, 3);
        state.click_box(&layout, &layout.pieces[0]);
        state.field(7, "", Some(2)).set_text("abc");
        assert_eq!(state.typed(7), Some("ab"), "the limit holds");
        assert_eq!(state.press(&button(Some(1), None)), Some(1));
        assert_eq!(
            state.answer(&layout, 1),
            Act::GumpButton {
                gump: 9,
                button: 1,
                switches: vec![4],
                texts: vec![(7, "ab".into())],
            }
        );
        assert!(state.take_focus());
        assert!(!state.take_focus());
        assert_eq!(state.close(), Act::GumpClose(9));
    }

    #[test]
    fn the_item_of_a_tile_button_sits_in_its_middle() {
        let tile = GumpTileArt {
            graphic: 1,
            hue: 0,
            w: 40,
            h: 30,
        };
        assert_eq!(
            tile_art_place(5, 5, &tile, Vector::new(20.0, 10.0)),
            (15, 15)
        );
        assert_eq!(tile_art_place(5, 5, &tile, Vector::new(60.0, 60.0)), (5, 5));
    }
}
