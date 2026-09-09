use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{ProtocolError, Result};

pub const SERIAL_ITEM_MIN: u32 = 0x4000_0000;
pub const SERIAL_WORLD: u32 = 0xFFFF_FFFF;
pub const VARIABLE_LEN_FLAG: u16 = 0x8000;
pub const MAX_PACKET_LEN: usize = 0x10000;
pub const DIR_RUNNING: u8 = 0x80;
pub const DIR_MASK: u8 = 0x07;
pub const DEFAULT_FONT: u16 = 3;
pub const DEFAULT_SPEECH_HUE: u16 = 0x03B2;
pub const LANGUAGE_ENU: [u8; 4] = *b"ENU\0";

pub const PKT_DAMAGE: u8 = 0x0B;
pub const PKT_CLILOC: u8 = 0xC1;
pub const PKT_CLILOC_AFFIX: u8 = 0xCC;
pub const LOGIN_NEXT_KEY_DEFAULT: u8 = 0xFF;
pub const UNKNOWN_VAR_MIN: usize = 3;
pub const UNKNOWN_VAR_MAX: usize = 8192;
pub const PKT_MOVE: u8 = 0x02;
pub const PKT_ASCII_SPEECH: u8 = 0x03;
pub const PKT_ATTACK: u8 = 0x05;
pub const PKT_DOUBLE_CLICK: u8 = 0x06;
pub const PKT_LIFT: u8 = 0x07;
pub const PKT_DROP: u8 = 0x08;
pub const PKT_SINGLE_CLICK: u8 = 0x09;
pub const PKT_TEXT_COMMAND: u8 = 0x12;
pub const PKT_EQUIP: u8 = 0x13;
pub const PKT_STATUS: u8 = 0x11;
pub const PKT_HEALTH_BAR_STATUS: u8 = 0x17;
pub const PKT_WORLD_ITEM: u8 = 0x1A;
pub const PKT_LOGIN_CONFIRM: u8 = 0x1B;
pub const PKT_ASCII_MESSAGE: u8 = 0x1C;
pub const PKT_DELETE: u8 = 0x1D;
pub const PKT_DRAW_PLAYER: u8 = 0x20;
pub const PKT_MOVE_REJECT: u8 = 0x21;
pub const PKT_MOVE_ACK: u8 = 0x22;
pub const PKT_OPEN_CONTAINER: u8 = 0x24;
pub const PKT_ADD_ITEM: u8 = 0x25;
pub const PKT_DEATH_MENU: u8 = 0x2C;
pub const PKT_EQUIPPED: u8 = 0x2E;
pub const PKT_SWING: u8 = 0x2F;
pub const PKT_QUERY: u8 = 0x34;
pub const PKT_SKILLS: u8 = 0x3A;
/// `0x3B`. The client sends the shopping basket; the server sends the same id
/// back with an empty basket to shut the buy window.
pub const PKT_VENDOR_BUY: u8 = 0x3B;
pub const PKT_CONTAINER_CONTENTS: u8 = 0x3C;
pub const PKT_SOUND_EFFECT: u8 = 0x54;
pub const PKT_PLAY_CHARACTER: u8 = 0x5D;
/// `0x66`. Server: the pages of an open book. Client: one edited page, or a
/// request for a page it has not read yet.
pub const PKT_BOOK_CONTENT: u8 = 0x66;
pub const PKT_TARGET: u8 = 0x6C;
pub const PKT_MUSIC: u8 = 0x6D;
pub const PKT_CHARACTER_ANIMATION: u8 = 0x6E;
pub const PKT_SECURE_TRADE: u8 = 0x6F;
pub const PKT_WAR_MODE: u8 = 0x72;
pub const PKT_PING: u8 = 0x73;
/// `0x74`. The price and the description of every item in a buy window.
pub const PKT_VENDOR_BUY_LIST: u8 = 0x74;
pub const PKT_MOBILE_MOVING: u8 = 0x77;
pub const PKT_MOBILE_INCOMING: u8 = 0x78;
/// `0x7C`. The old-style menu, which is not a gump.
pub const PKT_OPEN_MENU: u8 = 0x7C;
/// `0x7D`. The entry the player picked from a `0x7C` menu.
pub const PKT_MENU_RESPONSE: u8 = 0x7D;
pub const PKT_LOGIN_REQUEST: u8 = 0x80;
pub const PKT_LOGIN_DENIED: u8 = 0x82;
pub const PKT_RELAY: u8 = 0x8C;
pub const PKT_GAME_LOGIN: u8 = 0x91;
/// `0x93`. The cover of a book, in the fixed-width layout of the old client.
pub const PKT_BOOK_HEADER_OLD: u8 = 0x93;
/// `0x9E`. Everything the player may sell to this shopkeeper, with prices.
pub const PKT_VENDOR_SELL_LIST: u8 = 0x9E;
/// `0x9F`. The items the player chose to sell.
pub const PKT_VENDOR_SELL: u8 = 0x9F;
pub const PKT_SELECT_SERVER: u8 = 0xA0;
pub const PKT_UPDATE_HEALTH: u8 = 0xA1;
pub const PKT_UPDATE_MANA: u8 = 0xA2;
pub const PKT_UPDATE_STAM: u8 = 0xA3;
pub const PKT_SERVER_LIST: u8 = 0xA8;
pub const PKT_CHARACTER_LIST: u8 = 0xA9;
pub const PKT_UNICODE_SPEECH: u8 = 0xAD;
pub const PKT_UNICODE_MESSAGE: u8 = 0xAE;
pub const PKT_DEATH: u8 = 0xAF;
pub const PKT_GUMP: u8 = 0xB0;
pub const PKT_GUMP_RESPONSE: u8 = 0xB1;
pub const PKT_FEATURES: u8 = 0xB9;
pub const PKT_CLIENT_VERSION: u8 = 0xBD;
pub const PKT_EXTENDED: u8 = 0xBF;
pub const PKT_SEASON: u8 = 0xBC;
pub const EXT_FASTWALK: u16 = 0x0001;
pub const EXT_MAP_CHANGE: u16 = 0x0008;
/// `0xBF` sub-command that asks for the context menu of one object.
pub const EXT_CONTEXT_MENU_REQUEST: u16 = 0x0013;
/// `0xBF` sub-command that carries the context menu the server built.
pub const EXT_CONTEXT_MENU_DISPLAY: u16 = 0x0014;
/// `0xBF` sub-command that picks one entry of a context menu.
pub const EXT_CONTEXT_MENU_RESPONSE: u16 = 0x0015;
pub const FASTWALK_KEY_COUNT: usize = 6;
pub const PKT_POPUP_MESSAGE: u8 = 0x53;
pub const POPUP_CHAR_IN_WORLD: u8 = 0x05;
pub const POPUP_IDLE_WARNING: u8 = 0x07;
pub const PKT_LOGIN_COMPLETE: u8 = 0x55;
pub const PKT_PAPERDOLL: u8 = 0x88;
pub const PKT_BATCH_QUERY_PROPERTIES: u8 = 0xD6;
pub const PKT_OPL_INFO: u8 = 0xDC;
pub const PKT_COMPRESSED_GUMP: u8 = 0xDD;
/// `0xD4`. The cover of a book, with counted title and author fields.
pub const PKT_BOOK_HEADER: u8 = 0xD4;
pub const PKT_BUFF_DEBUFF: u8 = 0xDF;
pub const PKT_SEED: u8 = 0xEF;
pub const PKT_WORLD_ITEM_SA: u8 = 0xF3;
pub const PKT_CLIENT_TYPE: u8 = 0xE1;
pub const PKT_CLIENT_INFO: u8 = 0xD9;
pub const CLIENT_TYPE_CMD: u16 = 0x0001;
pub const CLIENT_TYPE_CLASSIC: u16 = 0x0000;
pub const CLIENT_INFO_LEN: usize = 0x10C;
pub const CLIENT_INFO_TYPE_NEW: u8 = 0x00;
pub const CLIENT_INFO_VIDEO_DESC_LEN: usize = 64;
pub const CLIENT_INFO_CLIENTS_RUNNING: u8 = 1;
pub const CLIENT_INFO_CLIENTS_INSTALLED: u8 = 1;
pub const LOGIN_CONFIRM_LEN: usize = 37;
pub const MAP_DEFAULT_WIDTH: u16 = 7168;
pub const MAP_DEFAULT_HEIGHT: u16 = 4096;
pub const WORLD_ITEM_SA_LEN: usize = 26;
pub const WORLD_ITEM_SA_UNKNOWN: u16 = 0x0001;
pub const WORLD_ITEM_SA_ITEM: u8 = 0x00;

/// The reference client puts no more serials than this in one `0xD6` request.
pub const BATCH_QUERY_PROPERTIES_MAX: usize = 15;
/// Format word of a `0xD6` object property list. The reference client drops the
/// list above this.
pub const OPL_LIST_FORMAT: u16 = 1;
/// Unused word between the serial and the revision hash of a `0xD6` list.
pub const OPL_LIST_PADDING: u16 = 0;
/// Cliloc id that ends the property block of a `0xD6` list.
pub const OPL_LIST_TERMINATOR: u32 = 0;

pub const TEXT_CMD_USE_SKILL: u8 = 0x24;
pub const TEXT_CMD_CAST_SPELL: u8 = 0x56;
pub const TEXT_CMD_OPEN_DOOR: u8 = 0x58;

/// Gump id of the `0x24` that opens a shopkeeper's buy window. A server writes
/// it; every other container gets its own gump.
pub const CONTAINER_GUMP_VENDOR_BUY: u16 = 0x0030;
/// Opens a `0x3B` basket that holds items. The reference client writes it in
/// its buy request, and a server acts on nothing else.
pub const VENDOR_BUY_WITH_ITEMS: u8 = 0x02;
/// Opens an empty `0x3B` basket, which buys nothing and shuts the window.
pub const VENDOR_BUY_EMPTY: u8 = 0x00;
/// Byte in front of each item of a `0x3B` basket. The reference client writes
/// this value and a server reads it into a variable it never uses, so no source
/// says what a different value would mean.
pub const VENDOR_BUY_ITEM_LAYER: u8 = 0x1A;
/// A server drops a basket with more items than this.
pub const VENDOR_BUY_ITEM_MAX: usize = 100;
/// A server drops a sell list with this many items or more.
pub const VENDOR_SELL_ITEM_MAX: usize = 100;

/// Entries of a `0x7C` menu are numbered from one in the `0x7D` answer.
/// A server takes one off the index before it uses it.
pub const MENU_FIRST_INDEX: u16 = 1;
/// A `0x7D` whose index is zero picks nothing and shuts the menu. The reference
/// client writes the tail as zeroes for that answer.
pub const MENU_CANCEL_INDEX: u16 = 0;

/// A `0xBF` `0x14` context menu of mode two or above carries whole cliloc
/// numbers. Below it, each entry carries the offset from
/// [`CONTEXT_MENU_CLILOC_BASE`].
pub const CONTEXT_MENU_MODE_CLILOC: u16 = 2;
/// Added to the 16-bit cliloc offset of an old-mode context menu entry.
pub const CONTEXT_MENU_CLILOC_BASE: u32 = 3_000_000;
/// The entry is greyed out and cannot be picked.
pub const CME_FLAG_DISABLED: u16 = 0x0001;
/// The entry opens a submenu.
pub const CME_FLAG_ARROW: u16 = 0x0002;
/// The entry is drawn highlighted.
pub const CME_FLAG_HIGHLIGHTED: u16 = 0x0004;
/// The entry carries its own colour, in a word right after the flags.
pub const CME_FLAG_COLORED: u16 = 0x0020;
/// Flags that make the reference client step over a word it never reads when it
/// parses a popup menu. No server source writes such a word, and no source says
/// what it holds, so the decoder only steps over it.
pub const CME_FLAGS_WITH_SPARE_WORD: u16 = 0x0084;
/// A further flag that makes the reference client step over one more unread
/// word.
pub const CME_FLAG_WITH_SPARE_WORD: u16 = 0x0040;

/// The skill gains when it is under its cap.
pub const SKILL_LOCK_UP: u8 = 0;
/// The skill falls to make room for others that are still going up.
pub const SKILL_LOCK_DOWN: u8 = 1;
/// The skill neither gains nor falls.
pub const SKILL_LOCK_LOCKED: u8 = 2;

/// `0x6F` opening a trade window. The serial names the other player.
pub const TRADE_DISPLAY: u8 = 0;
/// `0x6F` shutting a trade window. The serial names a trade container.
pub const TRADE_CLOSE: u8 = 1;
/// `0x6F` carrying the two accept flags of an open trade.
pub const TRADE_UPDATE: u8 = 2;
/// `0x6F` carrying the gold and platinum the other player put up.
pub const TRADE_UPDATE_GOLD: u8 = 3;
/// `0x6F` carrying the gold and platinum this player put up.
pub const TRADE_UPDATE_LEDGER: u8 = 4;
/// Width of the name field of a `TRADE_DISPLAY`.
pub const TRADE_NAME_LEN: usize = 30;
/// A `TRADE_DISPLAY` whose name byte is zero carries no name at all.
pub const TRADE_HAS_NAME: u8 = 1;

/// Width of the title of a `0x93` book cover.
pub const BOOK_TITLE_LEN: usize = 60;
/// Width of the author of a `0x93` book cover.
pub const BOOK_AUTHOR_LEN: usize = 30;
/// The client asks for a page by sending a `0x66` whose line count is this.
/// The reference client writes it in place of a line count.
pub const BOOK_PAGE_REQUEST: u16 = 0xFFFF;
/// A `0x66` from the client always carries one page, whether it writes the
/// page or asks for it.
pub const BOOK_PAGE_COUNT_ONE: u16 = 1;
/// A book is writable. A server writes this flag on the book cover, and the
/// byte in front of it, which it always sets, has no name in either source.
pub const BOOK_WRITABLE: u8 = 1;
/// A server drops a page that carries more lines than this.
pub const BOOK_PAGE_LINE_MAX: usize = 8;

pub const PLAY_CHAR_PATTERN: u32 = 0xEDED_EDED;

pub const SPEECH_REGULAR: u8 = 0;
pub const SPEECH_SYSTEM: u8 = 1;
pub const SPEECH_EMOTE: u8 = 2;
pub const SPEECH_WHISPER: u8 = 8;

/// Set on the speech kind of a `0xAD` whose body carries the numbered keyword
/// block. The reference client calls this the encoded message kind, and a
/// server reads the same bit to decide between the keyword body and plain
/// UTF-16BE text.
pub const SPEECH_ENCODED: u8 = 0xC0;
/// A server drops a whole encoded `0xAD` that claims more keywords than this.
pub const SPEECH_KEYWORD_MAX: usize = 50;

pub const TARGET_OBJECT: u8 = 0;
pub const TARGET_FLAG_NONE: u8 = 0;
pub const TARGET_FLAG_CANCEL: u8 = 3;

pub const LAYER_ONE_HANDED: u8 = 1;
pub const LAYER_BACKPACK: u8 = 21;
pub const LAYER_BANK: u8 = 29;

pub const FLAG_FROZEN: u8 = 0x01;
pub const FLAG_POISONED: u8 = 0x04;
pub const FLAG_WAR: u8 = 0x40;
pub const FLAG_HIDDEN: u8 = 0x80;

/// `0x17` health bar colours. The server and the reference client agree: 1 is
/// the poison bar, 2 is the yellow bar a blessed or invulnerable mobile gets.
/// The reference client reads a third colour and does nothing with it, so the
/// decoder keeps every colour id raw.
pub const HEALTH_BAR_POISON: u16 = 1;
pub const HEALTH_BAR_YELLOW: u16 = 2;
/// A `0x17` status byte of zero turns that bar colour off.
pub const HEALTH_BAR_OFF: u8 = 0;
/// A server puts `poison level + 1` in the `0x17` status byte.
pub const HEALTH_BAR_POISON_LEVEL_BIAS: u8 = 1;

/// `0x6D` index that stops the music. A server writes the bytes `6D 1F FF`,
/// which the reference client reads as this index.
pub const MUSIC_INDEX_STOP: u16 = 0x1FFF;

pub const NOTO_INNOCENT: u8 = 1;
pub const NOTO_GREY: u8 = 3;

pub const SKILL_LUMBERJACKING: u16 = 44;

pub const GRAPHIC_HATCHET: u16 = 0x0F43;
pub const GRAPHIC_LOGS: u16 = 0x1BDD;
pub const GRAPHIC_BANDAGE: u16 = 0x0E21;
pub const GRAPHIC_BACKPACK: u16 = 0x0E75;

pub const TREE_GRAPHIC_MIN: u16 = 0x0C95;
pub const TREE_GRAPHIC_MAX: u16 = 0x0CE8;

pub const EXIT_OK: i32 = 0;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_NETWORK: i32 = 3;
pub const EXIT_PROTOCOL: i32 = 4;
pub const EXIT_WORLD: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Serial(pub u32);

impl Serial {
    pub const INVALID: Self = Self(0);
    pub const WORLD: Self = Self(SERIAL_WORLD);

    pub fn is_valid(self) -> bool {
        self.0 != 0 && self.0 != SERIAL_WORLD
    }

    pub fn is_mobile(self) -> bool {
        self.0 != 0 && self.0 < SERIAL_ITEM_MIN
    }

    pub fn is_item(self) -> bool {
        self.0 >= SERIAL_ITEM_MIN && self.0 != SERIAL_WORLD
    }
}

impl fmt::Display for Serial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point3 {
    pub x: u16,
    pub y: u16,
    pub z: i8,
}

impl Point3 {
    pub fn new(x: u16, y: u16, z: i8) -> Self {
        Self { x, y, z }
    }

    pub fn chebyshev(self, other: Self) -> u32 {
        let dx = (self.x as i32 - other.x as i32).unsigned_abs();
        let dy = (self.y as i32 - other.y as i32).unsigned_abs();
        dx.max(dy)
    }

    /// The tile beside this one in `dir`, read at this tile's own height, or
    /// `None` when it falls off the grid.
    ///
    /// This names a neighbour and never where a step lands. The height comes
    /// across unchanged, which is what a caller wants who is looking around
    /// one floor of a building: the four tiles a door is opened from, the
    /// eight a follower may stand on, the one a refusal was about. A step is
    /// a different question, because a step climbs: the height it lands at is
    /// worked out from the map for that one step, and only
    /// `uoterm_nav::TileQuery::can_step` answers it. A walk that took this
    /// answer for the tile it arrives on carried the height of the tile it
    /// left onto new ground, and the character's recorded height then never
    /// changed however far uphill he walked.
    pub fn neighbour(self, dir: Direction) -> Option<Self> {
        let (dx, dy) = dir.delta();
        let x = self.x as i32 + dx;
        let y = self.y as i32 + dy;
        if x < 0 || y < 0 || x > u16::MAX as i32 || y > u16::MAX as i32 {
            return None;
        }
        Some(Self {
            x: x as u16,
            y: y as u16,
            z: self.z,
        })
    }
}

impl fmt::Display for Point3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.x, self.y, self.z)
    }
}

impl FromStr for Point3 {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split(',');
        let x = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or(ProtocolError::Parse {
                id: 0,
                reason: "invalid point x",
            })?;
        let y = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or(ProtocolError::Parse {
                id: 0,
                reason: "invalid point y",
            })?;
        let z = parts.next().map(|p| p.parse().unwrap_or(0)).unwrap_or(0);
        Ok(Self { x, y, z })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    Northeast = 1,
    East = 2,
    Southeast = 3,
    South = 4,
    Southwest = 5,
    West = 6,
    Northwest = 7,
}

impl Direction {
    pub fn from_byte(value: u8) -> Self {
        match value & DIR_MASK {
            1 => Self::Northeast,
            2 => Self::East,
            3 => Self::Southeast,
            4 => Self::South,
            5 => Self::Southwest,
            6 => Self::West,
            7 => Self::Northwest,
            _ => Self::North,
        }
    }

    pub fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::Northeast => (1, -1),
            Self::East => (1, 0),
            Self::Southeast => (1, 1),
            Self::South => (0, 1),
            Self::Southwest => (-1, 1),
            Self::West => (-1, 0),
            Self::Northwest => (-1, -1),
        }
    }

    pub fn from_delta(dx: i32, dy: i32) -> Self {
        match (dx.signum(), dy.signum()) {
            (0, -1) => Self::North,
            (1, -1) => Self::Northeast,
            (1, 0) => Self::East,
            (1, 1) => Self::Southeast,
            (0, 1) => Self::South,
            (-1, 1) => Self::Southwest,
            (-1, 0) => Self::West,
            (-1, -1) => Self::Northwest,
            _ => Self::North,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::Northeast => "northeast",
            Self::East => "east",
            Self::Southeast => "southeast",
            Self::South => "south",
            Self::Southwest => "southwest",
            Self::West => "west",
            Self::Northwest => "northwest",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "n" | "north" | "0" => Some(Self::North),
            "ne" | "northeast" | "1" => Some(Self::Northeast),
            "e" | "east" | "2" => Some(Self::East),
            "se" | "southeast" | "3" => Some(Self::Southeast),
            "s" | "south" | "4" => Some(Self::South),
            "sw" | "southwest" | "5" => Some(Self::Southwest),
            "w" | "west" | "6" => Some(Self::West),
            "nw" | "northwest" | "7" => Some(Self::Northwest),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
    pub patch: u32,
}

impl ClientVersion {
    pub const T2A: Self = Self {
        major: 2,
        minor: 0,
        revision: 7,
        patch: 0,
    };
    pub const MODERN: Self = Self {
        major: 7,
        minor: 0,
        revision: 102,
        patch: 3,
    };

    pub fn as_string(self) -> String {
        format!(
            "{}.{}.{}.{}",
            self.major, self.minor, self.revision, self.patch
        )
    }

    pub fn at_least(self, other: Self) -> bool {
        (self.major, self.minor, self.revision, self.patch)
            >= (other.major, other.minor, other.revision, other.patch)
    }

    pub fn has_container_grid(self) -> bool {
        self.at_least(Self {
            major: 6,
            minor: 0,
            revision: 1,
            patch: 7,
        })
    }

    pub fn has_sa_item_packet(self) -> bool {
        self.at_least(Self {
            major: 7,
            minor: 0,
            revision: 0,
            patch: 0,
        })
    }

    pub fn has_feature_uint32(self) -> bool {
        self.at_least(Self {
            major: 6,
            minor: 0,
            revision: 14,
            patch: 2,
        })
    }

    pub fn has_incoming_equip_hue(self) -> bool {
        self.at_least(Self {
            major: 7,
            minor: 0,
            revision: 33,
            patch: 1,
        })
    }

    pub fn has_prefixed_mobile_incoming(self) -> bool {
        self.has_sa_item_packet()
    }

    /// Clients from 7.0.0.0 up read the `0x17` poison byte as a poison level.
    /// Older clients read it as an on or off flag. The reference client gates
    /// its health bar update on version 7.0.0.0.
    pub fn has_sa_poison_level(self) -> bool {
        self.has_sa_item_packet()
    }
}

impl Default for ClientVersion {
    fn default() -> Self {
        Self::MODERN
    }
}

impl FromStr for ClientVersion {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split('.');
        let major = parse_part(parts.next())?;
        let minor = parse_part(parts.next())?;
        let revision = parse_part(parts.next())?;
        let patch = parse_part(parts.next()).unwrap_or(0);
        Ok(Self {
            major,
            minor,
            revision,
            patch,
        })
    }
}

fn parse_part(part: Option<&str>) -> Result<u32> {
    part.and_then(|p| p.parse().ok())
        .ok_or(ProtocolError::Parse {
            id: PKT_CLIENT_VERSION,
            reason: "invalid client version",
        })
}

impl fmt::Display for ClientVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Era {
    T2a,
    #[default]
    Modern,
}

impl Era {
    pub fn default_version(self) -> ClientVersion {
        match self {
            Self::T2a => ClientVersion::T2A,
            Self::Modern => ClientVersion::MODERN,
        }
    }
}

impl FromStr for Era {
    type Err = ProtocolError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "t2a" | "era.t2a" | "classic" => Ok(Self::T2a),
            "modern" | "era.modern" | "sa" | "hs" => Ok(Self::Modern),
            _ => Err(ProtocolError::Parse {
                id: 0,
                reason: "era must be t2a or modern",
            }),
        }
    }
}

impl fmt::Display for Era {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::T2a => write!(f, "t2a"),
            Self::Modern => write!(f, "modern"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_classifies_item_and_mobile() {
        assert!(Serial(0x0000_00AB).is_mobile());
        assert!(Serial(0x4000_0001).is_item());
        assert!(!Serial::INVALID.is_valid());
    }

    /// The context menu flags hold the server values, and the pair the
    /// reference client tests together to step over a word it never reads is
    /// the highlight flag joined to a bit no source names.
    #[test]
    fn context_menu_flags_hold_the_server_values() {
        const CME_FLAG_UNNAMED: u16 = 0x0080;
        assert_eq!(CME_FLAG_DISABLED, 0x01);
        assert_eq!(CME_FLAG_ARROW, 0x02);
        assert_eq!(CME_FLAG_HIGHLIGHTED, 0x04);
        assert_eq!(CME_FLAG_COLORED, 0x20);
        assert_eq!(
            CME_FLAGS_WITH_SPARE_WORD,
            CME_FLAG_HIGHLIGHTED | CME_FLAG_UNNAMED
        );
        assert_eq!(CME_FLAG_WITH_SPARE_WORD, 0x40);
        assert_eq!(CME_FLAGS_WITH_SPARE_WORD & CME_FLAG_COLORED, 0);
        assert_eq!(CME_FLAG_WITH_SPARE_WORD & CME_FLAG_COLORED, 0);
    }

    /// Skill locks hold the values of the server skill lock enumeration.
    #[test]
    fn skill_locks_hold_the_server_values() {
        assert_eq!(
            (SKILL_LOCK_UP, SKILL_LOCK_DOWN, SKILL_LOCK_LOCKED),
            (0, 1, 2)
        );
    }

    /// The trade kinds hold the values of the server trade flag enumeration, in
    /// the order that enumeration declares them.
    #[test]
    fn trade_kinds_hold_the_server_values() {
        assert_eq!(TRADE_DISPLAY, 0);
        assert_eq!(TRADE_CLOSE, 1);
        assert_eq!(TRADE_UPDATE, 2);
        assert_eq!(TRADE_UPDATE_GOLD, 3);
        assert_eq!(TRADE_UPDATE_LEDGER, 4);
    }

    /// The neighbour keeps the height it was read at, whatever the ground
    /// there really is. That is the whole of what this answers, and it is why
    /// no walk may take it for the tile a step arrives on.
    #[test]
    fn the_neighbour_north_keeps_the_height_it_was_read_at() {
        const ON_A_HILLSIDE: i8 = 18;
        let p = Point3::new(100, 100, ON_A_HILLSIDE);
        let n = p.neighbour(Direction::North).unwrap();
        assert_eq!(n, Point3::new(100, 99, ON_A_HILLSIDE));
    }

    #[test]
    fn version_parses() {
        let v: ClientVersion = "7.0.102.3".parse().unwrap();
        assert!(v.has_sa_item_packet());
        assert!(v.has_prefixed_mobile_incoming());
        assert!(v.has_container_grid());
        assert!(v.has_feature_uint32());
        assert!(!ClientVersion::T2A.has_prefixed_mobile_incoming());
    }
}
