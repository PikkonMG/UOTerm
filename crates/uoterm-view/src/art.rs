//! The pictures and the client tables the map draws with, as the scene
//! asks for them. A window that reads the client files makes each picture
//! itself; the web client asks the server for it by its [`ArtRequest`]. The
//! [`WorldArt`] trait is every question the scene asks of the art.

use crate::frame::{WatchEquip, WatchLiveMap, WatchLook};
use crate::geom::{Area, Point, Rgba, Vector};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::RangeInclusive;
use uoterm_nav::{
    mount_of, Action, AnimRules, CursorShape, Deed, ItemTile, LandTile, LightShape, MultiPiece,
    Stance, TextAlign, TileFlagSet, UnicodeStyle,
};
use uoterm_protocol::types::{LAYER_MOUNT, WEAPON_LAYERS};

/// Item graphics that the client never draws.
const NO_DRAW_GRAPHICS: [u16; 3] = [0x0001, 0x21BC, 0x63D3];
const NO_DRAW_RANGE: RangeInclusive<u16> = 0x2198..=0x21A4;

/// False for an item graphic the client never draws.
pub fn is_drawn(graphic: u16) -> bool {
    !NO_DRAW_GRAPHICS.contains(&graphic) && !NO_DRAW_RANGE.contains(&graphic)
}

/// One font of the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UoFont {
    /// A font of `fonts.mul`, in the colors of its pixels or in a hue.
    Ascii(u8),
    /// A Unicode font, in one color.
    Unicode(u8),
}

/// How a block of words is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextLook {
    pub font: UoFont,
    /// A hue number as the client files count them: 1 is the first. Zero
    /// keeps the colors of an ASCII font; `0xFFFF` is white Unicode words.
    pub hue: u16,
    pub align: TextAlign,
    /// The width the words wrap to, or are cut to with `crop`.
    pub width: Option<u32>,
    /// Cut the words short with dots, on one line, instead of wrapping.
    pub crop: bool,
    /// Bold, italic, underline and the black border of Unicode words.
    pub style: UnicodeStyle,
}

impl TextLook {
    const PLAIN: UnicodeStyle = UnicodeStyle {
        bold: false,
        italic: false,
        underline: false,
        border: false,
        extra_height: false,
    };

    pub const fn ascii(font: u8, hue: u16) -> Self {
        Self {
            font: UoFont::Ascii(font),
            hue,
            align: TextAlign::Left,
            width: None,
            crop: false,
            style: Self::PLAIN,
        }
    }

    pub const fn unicode(font: u8, hue: u16) -> Self {
        Self {
            font: UoFont::Unicode(font),
            ..Self::ascii(0, hue)
        }
    }

    /// The words wrap to this width.
    pub const fn wrap(self, width: u32) -> Self {
        Self {
            width: Some(width),
            crop: false,
            ..self
        }
    }

    /// The words stay on one line and are cut short with dots at this width.
    pub const fn cropped(self, width: u32) -> Self {
        Self {
            width: Some(width),
            crop: true,
            ..self
        }
    }

    /// Where each line sits across the width.
    pub const fn aligned(self, align: TextAlign) -> Self {
        Self { align, ..self }
    }

    /// Unicode words with a black ring round the ink.
    pub const fn bordered(self) -> Self {
        let mut style = self.style;
        style.border = true;
        Self { style, ..self }
    }
}

/// Which picture of a mobile to make: what he does, and how far into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pose {
    pub action: Action,
    /// The frame count since the window opened. Each part takes this modulo
    /// its own number of frames.
    pub tick: usize,
}

/// How a figure is painted: the ring round it, and one hue over all of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Paint {
    /// Clear draws no ring.
    pub outline: [u8; 4],
    pub whole_hue: Option<u16>,
}

impl Paint {
    /// The paint of the Modern style: a ring of one color, own hues.
    pub const fn outlined(outline: Rgba) -> Self {
        Self {
            outline: outline.to_array(),
            whole_hue: None,
        }
    }
}

/// The item a mobile rides, when it is a known mount.
pub fn mount_item(look: &WatchLook) -> Option<&WatchEquip> {
    look.equipment
        .iter()
        .find(|item| item.layer == LAYER_MOUNT && mount_of(item.graphic).is_some())
}

/// True when the mobile sits on a mount.
pub fn is_mounted(look: &WatchLook) -> bool {
    mount_item(look).is_some()
}

/// The action that shows a deed of a mobile, such as a swing or a cast.
pub fn deed_action(anim: &AnimRules, look: &WatchLook, deed: Deed) -> Option<Action> {
    anim.deed_action(look.body, deed, is_mounted(look))
}

/// The action of a mobile for the way he holds himself. A person on foot
/// in war mode stands ready, and one with a weapon walks armed.
pub fn stance_action(anim: &AnimRules, look: &WatchLook, action: Action) -> Action {
    if is_mounted(look) {
        return action;
    }
    let armed = look
        .equipment
        .iter()
        .any(|item| WEAPON_LAYERS.contains(&item.layer));
    let stance = Stance {
        armed,
        war: look.war,
    };
    anim.stance_action(look.body, action, stance)
}

/// How an item picture is painted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemPaint {
    pub hue: u16,
    /// The hue covers every pixel, as a hue the window puts on it does.
    pub whole_hue: bool,
    /// A black border marks a cave wall.
    pub border: bool,
}

/// One picture the scene asks for. A request is the same picture each
/// time, so a window keeps each picture under its request.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ArtRequest {
    Land {
        land_id: u16,
        hue: u16,
    },
    /// A land texture that is stretched over a slope.
    Texture {
        texture_id: u16,
        hue: u16,
    },
    Item {
        graphic: u16,
        hue: u16,
        /// The hue covers every pixel, not the grey ones only.
        whole_hue: bool,
        /// A black border marks a cave wall.
        border: bool,
    },
    /// A picture of a gump: a background, a button, a check box.
    Gump {
        gump: u16,
        hue: u16,
        /// The hue covers the grey pixels only, as a body or a worn item
        /// on a paperdoll takes it.
        partial: bool,
    },
    /// A mouse pointer of the classic client.
    Cursor {
        shape: CursorShape,
        war: bool,
        hue: u16,
    },
    /// Words in a UO font.
    Text {
        text: String,
        look: TextLook,
    },
    /// One mobile as he looks now.
    Figure {
        look: WatchLook,
        pose: Pose,
        paint: Paint,
    },
}

impl ArtRequest {
    /// The picture of an item graphic, painted as `paint` says.
    pub fn item(graphic: u16, paint: ItemPaint) -> Self {
        Self::Item {
            graphic,
            hue: paint.hue,
            whole_hue: paint.whole_hue,
            border: paint.border,
        }
    }

    /// A number for the request, the same for equal requests within one
    /// build. Caches keep their pictures under it.
    pub fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// One picture as RGBA bytes, row by row, unmultiplied.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    /// From the top left of the picture to the point that goes on the tile.
    pub anchor: Vector,
}

/// Where one picture lies in the texture of a window, and its size in
/// pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub uv: Area,
    pub width: f32,
    pub height: f32,
    pub anchor: Vector,
}

impl Sprite {
    /// A picture that is a whole texture of its own, anchored at its top
    /// left corner.
    pub const fn whole(width: f32, height: f32) -> Self {
        Self {
            uv: Area {
                min: crate::geom::Point::new(0.0, 0.0),
                max: crate::geom::Point::new(1.0, 1.0),
            },
            width,
            height,
            anchor: Vector::ZERO,
        }
    }
}

/// What the art has for a question: the answer, nothing yet because it is
/// on its way, or nothing because the client files hold none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art<T> {
    Ready(T),
    Pending,
    Missing,
}

impl<T> Art<T> {
    /// The answer, when it is here.
    pub fn ready(self) -> Option<T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Pending | Self::Missing => None,
        }
    }
}

impl<T> From<Option<T>> for Art<T> {
    /// An answer the client files do not hold is missing.
    fn from(found: Option<T>) -> Self {
        found.map_or(Self::Missing, Self::Ready)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellStatic {
    pub graphic: u16,
    pub hue: u16,
    pub z: i8,
    /// The height of its top, when a person can stand on it.
    pub floor: Option<i16>,
    pub flags: TileFlagSet,
    pub height: u8,
}

/// A land tile that is not flat. Its texture is stretched over the slope
/// and each corner is lit by the way it faces.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stretch {
    pub texture_id: u16,
    /// The normals at the top, right, bottom and left corners.
    pub normals: [[f32; 3]; 4],
}

/// One map tile as the window draws it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// None when the land of the tile draws nothing.
    pub land_id: Option<u16>,
    /// The heights of the top, right, bottom and left points of the diamond.
    pub corners: [i8; 4],
    pub statics: Vec<CellStatic>,
    /// Some for a slope that has a texture.
    pub stretch: Option<Stretch>,
    /// The height a slope counts as, as the classic client takes it.
    pub average_z: i8,
    /// The land is water.
    pub wet: bool,
}

/// Every question the scene asks of the art and the client tables. A
/// window that reads the client files answers at once; the web client
/// answers `Pending` until the server sent the answer.
pub trait WorldArt {
    /// True when there are client files at all.
    fn has_art(&self) -> bool;
    /// True when the client files hold the animation files, so the tables
    /// of [`WorldArt::anim`] are the real ones.
    fn has_anim(&self) -> bool;
    fn sprite(&mut self, request: &ArtRequest) -> Art<Sprite>;
    /// A point of the texture that is plain white. A shape with no picture
    /// takes its color from its vertices alone when it reads this point.
    fn white_uv(&self) -> Point;
    fn cell(&mut self, map: u8, x: u16, y: u16) -> Art<&Cell>;
    /// Lays the map blocks an UltimaLive shard changed over the map.
    fn take_live_map(&mut self, live: &WatchLiveMap);
    fn item_tile(&self, graphic: u16) -> Option<&ItemTile>;
    fn land_tile(&self, land_id: u16) -> Option<&LandTile>;
    /// The pieces of a house or a boat.
    fn multi_pieces(&mut self, multi: u16) -> Art<&[MultiPiece]>;
    /// The picture an item shows now. A fire or a fountain goes through
    /// the pictures of its cycle.
    fn shown_graphic(&self, graphic: u16, time_ms: u64) -> u16;
    /// The land tile that shows in a season: in winter grass is snow.
    fn season_land(&self, season: u8, land_id: u16) -> u16;
    /// The item that shows in a season: in autumn a green tree is brown.
    fn season_item(&self, season: u8, graphic: u16) -> u16;
    /// The color of one tile on a map of the world.
    fn radar_rgb(&mut self, map: u8, x: u16, y: u16) -> Option<[u8; 3]>;
    /// The height of the land of one tile.
    fn land_z(&mut self, map: u8, x: u16, y: u16) -> Option<i8>;
    fn light_shape(&mut self, id: u8) -> Art<&LightShape>;
    /// The tables of the animation files.
    fn anim(&self) -> &AnimRules;
    /// How many frames the body of this look has for an action.
    fn frame_count(&mut self, look: &WatchLook, action: Action) -> Art<usize>;
    /// The height of one line of words in a UO font.
    fn line_height(&self, look: &TextLook) -> f32;
    /// The lines words break into in a UO font. None with no UO fonts.
    fn text_lines(&self, text: &str, look: &TextLook) -> Vec<String>;
    /// The color of words in a hue.
    fn text_rgb(&self, hue: u16) -> [u8; 3];
    /// True when the gump draws the pixel at `x`, `y` of its picture.
    fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool;
    /// True when the client files hold the pictures of gumps.
    fn has_gump_art(&self) -> bool;
}

#[cfg(test)]
mod request_tests {
    use super::*;

    #[test]
    fn a_request_goes_on_the_wire_by_kind() {
        let request = ArtRequest::Item {
            graphic: 0x0F6C,
            hue: 0,
            whole_hue: false,
            border: false,
        };
        let text = serde_json::to_string(&request).unwrap();
        assert!(text.contains("\"kind\":\"Item\""));
        assert_eq!(serde_json::from_str::<ArtRequest>(&text).unwrap(), request);
    }

    #[test]
    fn no_draw_graphics_are_left_out() {
        const BARREL: u16 = 0x0E77;
        assert!(is_drawn(BARREL));
        assert!(!is_drawn(NO_DRAW_GRAPHICS[0]));
        assert!(!is_drawn(*NO_DRAW_RANGE.start()));
    }

    #[test]
    fn a_figure_and_words_go_on_the_wire() {
        let figure = ArtRequest::Figure {
            look: WatchLook {
                body: 0x0190,
                ..WatchLook::default()
            },
            pose: Pose {
                action: Action::Walk,
                tick: 3,
            },
            paint: Paint::outlined(Rgba::from_rgb(1, 2, 3)),
        };
        let words = ArtRequest::Text {
            text: "Hail".into(),
            look: TextLook::unicode(1, 0x0035).bordered(),
        };
        for request in [figure, words] {
            let text = serde_json::to_string(&request).unwrap();
            assert_eq!(serde_json::from_str::<ArtRequest>(&text).unwrap(), request);
        }
    }

    #[test]
    fn a_person_in_war_stands_ready_unless_he_rides() {
        const MAN: u16 = 0x0190;
        const HORSE_ITEM: u16 = 0x3EA2;
        /// The war stand of a person, as the files number it.
        const STAND_WAR: Action = Action::Shown(7);
        /// The armed walk of a person.
        const WALK_ARMED: Action = Action::Shown(1);
        let anim = AnimRules::default();
        let soldier = WatchLook {
            body: MAN,
            war: true,
            ..WatchLook::default()
        };
        assert!(!is_mounted(&soldier));
        assert_eq!(stance_action(&anim, &soldier, Action::Stand), STAND_WAR);
        let armed = WatchLook {
            war: false,
            equipment: vec![WatchEquip {
                layer: WEAPON_LAYERS[0],
                ..WatchEquip::default()
            }],
            ..soldier.clone()
        };
        assert_eq!(stance_action(&anim, &armed, Action::Walk), WALK_ARMED);
        let rider = WatchLook {
            equipment: vec![WatchEquip {
                graphic: HORSE_ITEM,
                layer: LAYER_MOUNT,
                ..WatchEquip::default()
            }],
            ..soldier
        };
        assert!(is_mounted(&rider));
        assert_eq!(stance_action(&anim, &rider, Action::Stand), Action::Stand);
        assert!(deed_action(&anim, &rider, Deed::Attack).is_some());
    }

    #[test]
    fn an_answer_the_files_lack_is_missing() {
        assert_eq!(Art::from(Some(3)), Art::Ready(3));
        assert_eq!(Art::<u8>::from(None), Art::Missing);
        assert_eq!(Art::<u8>::Pending.ready(), None);
    }

    #[test]
    fn equal_requests_share_one_key() {
        let a = ArtRequest::Land { land_id: 3, hue: 0 };
        assert_eq!(a.key(), ArtRequest::Land { land_id: 3, hue: 0 }.key());
        assert_ne!(a.key(), ArtRequest::Land { land_id: 4, hue: 0 }.key());
    }
}
