//! The standing picture of a body, from `anim.mul` and the files after it.
//!
//! A body is a creature, a person, or a worn item as it shows on a person.
//! Three tables say where a body is: `mobtypes.txt` gives its kind,
//! `Bodyconv.def` moves it to a later file, and `Body.def` gives a body with
//! no pictures of its own the pictures and the hue of a different body.
//! This reader holds the classic MUL files only. A body that lives in the
//! UOP animation files alone gives no picture.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use crate::art::{ArtPixels, PIXEL_DRAWN};
use crate::mul::{capped_len, read_file, slice_at, MapError, IDX_EMPTY};

const ANIM_FILE_COUNT: usize = 6;
const IDX_RECORD: usize = 12;
pub(crate) const DIRECTIONS_STORED: u32 = 5;
const MOBTYPES_NAME: &str = "mobtypes.txt";
const BODYCONV_NAME: &str = "Bodyconv.def";
const BODY_DEF_NAME: &str = "Body.def";
const EQUIPCONV_NAME: &str = "Equipconv.def";

// Where each kind of body starts in an index file, counted in records.
const HIGH_RECORDS_PER_BODY: u32 = 110;
const LOW_RECORDS_PER_BODY: u32 = 65;
const LOW_FIRST_BODY: u32 = 200;
const LOW_FIRST_RECORD: u32 = 22_000;
const PEOPLE_RECORDS_PER_BODY: u32 = 175;
const PEOPLE_FIRST_BODY: u32 = 400;
const PEOPLE_FIRST_RECORD: u32 = 35_000;

// The group of each kind of body for each thing it does: stand, walk, run.
// A monster has no run of its own, so it runs with its walk.
const GROUPS_HIGH: [u32; 3] = [1, 0, 0];
const GROUPS_LOW: [u32; 3] = [2, 0, 1];
const GROUPS_PEOPLE: [u32; 3] = [4, 0, 2];
const GROUPS_PEOPLE_MOUNTED: [u32; 3] = [25, 23, 24];
/// A monster whose pictures lie in the people part of a file has one group.
const GROUPS_MONSTER_IN_PEOPLE_FILE: [u32; 3] = [0, 0, 0];
/// No real direction holds more frames than this. A larger number is a
/// damaged record.
const MAX_FRAMES: usize = 64;

const FLAG_LOW_GROUP_EXTENDED: u32 = 0x0020;
const FLAG_BY_LOW_GROUP: u32 = 0x0040;
const FLAG_BY_PEOPLE_GROUP: u32 = 0x0400;
/// The pictures of the body are in the newer animation packages.
const FLAG_USE_UOP: u32 = 0x1_0000;

const PALETTE_COLORS: usize = 256;
const WORD: usize = 2;
pub(crate) const PALETTE_BYTES: usize = PALETTE_COLORS * WORD;
const DWORD: usize = 4;
const FRAME_HEADER_BYTES: usize = 8;
const RUN_END: u32 = 0x7FFF_7FFF;
const RUN_LENGTH_MASK: u32 = 0x0FFF;
const RUN_OFFSET_MASK: u32 = 0x03FF;
const RUN_OFFSET_SIGN: i32 = 0x0200;
const RUN_OFFSET_RANGE: i32 = 0x0400;
const RUN_X_SHIFT: u32 = 22;
const RUN_Y_SHIFT: u32 = 12;
/// No real frame is wider or taller than this. A larger number is a damaged
/// record.
const FRAME_MAX_SIDE: usize = 1024;
/// A `Body.def` row may name a body that has a row of its own. The walk
/// through such rows stops here.
const BODY_DEF_MAX_HOPS: usize = 4;
/// `Bodyconv.def` files later than this one hold bodies of other kinds at
/// the same numbers.
const FILE_ANIM2: usize = 1;
const FILE_ANIM3: usize = 2;
const ANIM3_FIRST_MONSTER: u16 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BodyKind {
    Monster,
    SeaMonster,
    Animal,
    Person,
}

/// What a body does in a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Stand,
    Walk,
    Run,
    /// An action the shard named by its group in the files of the body:
    /// a swing, a bow, a cast.
    Shown(u8),
}

impl Action {
    /// The group of the action, from the three groups of the kind of body.
    fn group(self, groups: [u32; 3]) -> u32 {
        match self {
            Self::Stand => groups[0],
            Self::Walk => groups[1],
            Self::Run => groups[2],
            Self::Shown(group) => u32::from(group),
        }
    }
}

/// A thing a body does once: the newer animation packet names it in the
/// words of the game, and each kind of body has its own pictures for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Deed {
    Attack,
    Block,
    GetHit,
    Die,
    Fidget,
    Eat,
    Bow,
    Salute,
    CastAtOne,
    CastAtAll,
}

impl Deed {
    /// The deed for the kind and the action of the newer animation packet.
    pub fn from_packet(kind: u16, action: u16) -> Option<Self> {
        Some(match (kind, action) {
            (0, _) => Self::Attack,
            (1 | 2, _) => Self::Block,
            (3, _) => Self::Die,
            (4, _) => Self::GetHit,
            (5, _) => Self::Fidget,
            (6, _) => Self::Eat,
            (7, 0) => Self::Bow,
            (7, _) => Self::Salute,
            (11, 0) => Self::CastAtOne,
            (11, _) => Self::CastAtAll,
            _ => return None,
        })
    }
}

/// How a person holds himself. It picks his stand, walk and run pictures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Stance {
    pub armed: bool,
    pub war: bool,
}

/// The five directions the files hold. The other three are these, mirrored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Facing {
    stored: u32,
    pub mirrored: bool,
}

impl Facing {
    /// From the direction on the wire: 0 for north, then clockwise to 7.
    pub fn from_direction(direction: u8) -> Self {
        let (stored, mirrored) = match direction & 0x07 {
            0 => (3, true),
            1 => (2, true),
            2 => (1, true),
            3 => (0, false),
            4 => (1, false),
            5 => (2, false),
            6 => (3, false),
            _ => (4, false),
        };
        Self { stored, mirrored }
    }
}

/// One frame of a body. `center_x` and `center_y` put it on its tile: the
/// left edge is `center_x` left of the tile center, and the bottom edge is
/// `center_y` above it.
#[derive(Clone, Debug)]
pub struct AnimFrame {
    pub center_x: i32,
    pub center_y: i32,
    pub pixels: ArtPixels,
    /// The hue `Body.def` gives this body. Zero when it gives none.
    pub file_hue: u16,
}

/// What `Equipconv.def` puts in the place of a worn item on one body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquipConv {
    pub anim: u16,
    /// The gump the item shows on a paperdoll of the body.
    pub gump: u16,
    pub hue: u16,
}

struct AnimFile {
    idx: Vec<u8>,
    mul: Mutex<File>,
}

pub struct AnimData {
    files: Vec<Option<AnimFile>>,
    kinds: HashMap<u16, (BodyKind, u32)>,
    /// A body that lives in a later file: the file, and its number there.
    moved: HashMap<u16, (usize, u16)>,
    /// A body that shows as a different body, with a hue.
    shown_as: HashMap<u16, (u16, u16)>,
    equip_conv: HashMap<(u16, u16), EquipConv>,
    /// None when the client files hold no newer animation packages.
    uop: Option<crate::anim_uop::UopAnims>,
}

fn anim_file_names(index: usize) -> (String, String) {
    let number = if index == 0 {
        String::new()
    } else {
        (index + 1).to_string()
    };
    (format!("anim{number}.idx"), format!("anim{number}.mul"))
}

/// The numbers of one table row. A row that starts with no digit is a
/// comment. A `#` ends the row.
fn row_numbers(line: &str) -> Option<Vec<i64>> {
    let line = line.split('#').next().unwrap_or("").trim();
    if !line.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some(
        line.split(|c: char| c.is_whitespace() || matches!(c, '{' | '}' | ','))
            .filter(|token| !token.is_empty())
            .map(|token| token.parse::<i64>().unwrap_or(-1))
            .collect(),
    )
}

fn read_text(path: &Path) -> String {
    read_file(path)
        .map(|bytes| bytes.iter().map(|&b| char::from(b)).collect())
        .unwrap_or_default()
}

fn parse_mobtypes(text: &str) -> HashMap<u16, (BodyKind, u32)> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        let mut parts = line.split_whitespace();
        let (Some(id), Some(kind), Some(flags)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let kind = match kind.to_ascii_lowercase().as_str() {
            "monster" => BodyKind::Monster,
            "sea_monster" => BodyKind::SeaMonster,
            "animal" => BodyKind::Animal,
            "human" | "equipment" => BodyKind::Person,
            _ => continue,
        };
        if let (Ok(id), Ok(flags)) = (id.parse::<u16>(), u32::from_str_radix(flags, 16)) {
            out.insert(id, (kind, flags));
        }
    }
    out
}

/// Each column after the first is one later file. The last column that names
/// a body wins.
fn parse_bodyconv(text: &str) -> HashMap<u16, (usize, u16)> {
    let mut out = HashMap::new();
    for row in text.lines().filter_map(row_numbers) {
        let Some(body) = row.first().and_then(|n| u16::try_from(*n).ok()) else {
            continue;
        };
        for (file, moved) in row.iter().enumerate().skip(1) {
            if let (true, Ok(moved)) = (file < ANIM_FILE_COUNT, u16::try_from(*moved)) {
                out.insert(body, (file, moved));
            }
        }
    }
    out
}

/// A row is `body {shown, ...} hue`. The third body of the group is the one
/// that shows when there are three, and the first one when there are fewer.
fn parse_body_def(text: &str) -> HashMap<u16, (u16, u16)> {
    const GROUP_PICK: usize = 2;
    let mut out = HashMap::new();
    for line in text.lines() {
        let (Some(open), Some(close)) = (line.find('{'), line.find('}')) else {
            continue;
        };
        let (Some(head), Some(group), Some(tail)) = (
            row_numbers(&line[..open]),
            row_numbers(&format!("0 {}", &line[open + 1..close])),
            row_numbers(&format!("0 {}", &line[close + 1..])),
        ) else {
            continue;
        };
        let group = &group[1..];
        let shown = group.get(GROUP_PICK).or_else(|| group.first());
        let (Some(body), Some(shown)) = (
            head.first().and_then(|n| u16::try_from(*n).ok()),
            shown.and_then(|n| u16::try_from(*n).ok()),
        ) else {
            continue;
        };
        let hue = tail
            .get(1)
            .and_then(|n| u16::try_from(*n).ok())
            .unwrap_or(0);
        out.entry(body).or_insert((shown, hue));
    }
    out
}

/// A row is `body worn_anim new_anim gump hue`. A gump of zero is the worn
/// animation, and one of -1 the new animation, as the classic client reads
/// them; a row with a gump past the gump files is left out.
fn parse_equipconv(text: &str) -> HashMap<(u16, u16), EquipConv> {
    const COLUMNS: usize = 5;
    const GUMP_COLUMN: usize = 3;
    const GUMP_OF_WORN: i64 = 0;
    const GUMP_OF_NEW: [i64; 2] = [-1, 0xFFFF];
    let mut out = HashMap::new();
    for row in text.lines().filter_map(row_numbers) {
        if row.len() < COLUMNS || row[GUMP_COLUMN] > i64::from(u16::MAX) {
            continue;
        }
        let number = |i: usize| u16::try_from(row[i]).ok();
        if let (Some(body), Some(worn), Some(anim)) = (number(0), number(1), number(2)) {
            let gump = match row[GUMP_COLUMN] {
                GUMP_OF_WORN => worn,
                gump if GUMP_OF_NEW.contains(&gump) => anim,
                _ => number(GUMP_COLUMN).unwrap_or(anim),
            };
            out.insert(
                (body, worn),
                EquipConv {
                    anim,
                    gump,
                    hue: number(4).unwrap_or(0),
                },
            );
        }
    }
    out
}

fn default_kind(body: u16, file: usize) -> BodyKind {
    match file {
        FILE_ANIM2 if body < LOW_FIRST_BODY as u16 => BodyKind::Monster,
        FILE_ANIM2 => BodyKind::Animal,
        FILE_ANIM3 if body < ANIM3_FIRST_MONSTER => BodyKind::Animal,
        FILE_ANIM3 if body < PEOPLE_FIRST_BODY as u16 => BodyKind::Monster,
        _ if body < LOW_FIRST_BODY as u16 => BodyKind::Monster,
        _ if body < PEOPLE_FIRST_BODY as u16 => BodyKind::Animal,
        _ => BodyKind::Person,
    }
}

/// The first index record of a body, and its group for each action.
const PEOPLE_WALK_ARMED: u8 = 1;
const PEOPLE_RUN_ARMED: u8 = 3;
const PEOPLE_STAND_WAR: u8 = 7;
const PEOPLE_WALK_WAR: u8 = 15;

/// The group of a deed for a person on foot, a person on a mount, a
/// monster and an animal. None when that kind has no pictures for it.
fn deed_groups(deed: Deed) -> [Option<u8>; 4] {
    match deed {
        Deed::Attack => [Some(9), Some(26), Some(4), Some(5)],
        Deed::Block => [Some(30), None, Some(15), Some(7)],
        Deed::GetHit => [Some(20), None, Some(10), Some(7)],
        Deed::Die => [Some(21), None, Some(2), Some(8)],
        Deed::Fidget => [Some(5), None, Some(17), Some(9)],
        Deed::Eat => [Some(34), None, Some(11), Some(3)],
        Deed::Bow => [Some(32), None, None, None],
        Deed::Salute => [Some(33), None, None, None],
        Deed::CastAtOne => [Some(16), None, Some(12), None],
        Deed::CastAtAll => [Some(17), None, Some(12), None],
    }
}

/// The action numbers of a body in the newer packages. They are the
/// numbers of its kind in the classic files.
fn uop_groups(kind: BodyKind, mounted: bool) -> [u32; 3] {
    match kind {
        BodyKind::Monster => GROUPS_HIGH,
        BodyKind::SeaMonster | BodyKind::Animal => GROUPS_LOW,
        BodyKind::Person if mounted => GROUPS_PEOPLE_MOUNTED,
        BodyKind::Person => GROUPS_PEOPLE,
    }
}

fn first_record_and_groups(
    body: u16,
    kind: BodyKind,
    flags: u32,
    mounted: bool,
) -> Option<(u32, [u32; 3])> {
    let body = u32::from(body);
    let high = Some((body * HIGH_RECORDS_PER_BODY, GROUPS_HIGH));
    let low = body
        .checked_sub(LOW_FIRST_BODY)
        .map(|n| (n * LOW_RECORDS_PER_BODY + LOW_FIRST_RECORD, GROUPS_LOW));
    let people = |groups: [u32; 3]| {
        body.checked_sub(PEOPLE_FIRST_BODY)
            .map(|n| (n * PEOPLE_RECORDS_PER_BODY + PEOPLE_FIRST_RECORD, groups))
    };
    let by_flags = || {
        if flags & FLAG_BY_PEOPLE_GROUP != 0 {
            people(GROUPS_MONSTER_IN_PEOPLE_FILE)
        } else if flags & FLAG_BY_LOW_GROUP != 0 {
            low
        } else {
            high
        }
    };
    match kind {
        BodyKind::Monster => by_flags(),
        BodyKind::SeaMonster => high.map(|(first, _)| (first, GROUPS_LOW)),
        BodyKind::Animal if flags & FLAG_LOW_GROUP_EXTENDED != 0 => by_flags(),
        BodyKind::Animal => low,
        BodyKind::Person if mounted => people(GROUPS_PEOPLE_MOUNTED),
        BodyKind::Person => people(GROUPS_PEOPLE),
    }
}

impl AnimData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let files: Vec<Option<AnimFile>> = (0..ANIM_FILE_COUNT)
            .map(|i| {
                let (idx, mul) = anim_file_names(i);
                Some(AnimFile {
                    idx: read_file(&dir.join(idx)).ok()?,
                    mul: Mutex::new(File::open(dir.join(mul)).ok()?),
                })
            })
            .collect();
        if !matches!(files.first(), Some(Some(_))) {
            return Err(MapError::Missing("anim.mul"));
        }
        Ok(Self {
            files,
            kinds: parse_mobtypes(&read_text(&dir.join(MOBTYPES_NAME))),
            moved: parse_bodyconv(&read_text(&dir.join(BODYCONV_NAME))),
            shown_as: parse_body_def(&read_text(&dir.join(BODY_DEF_NAME))),
            equip_conv: parse_equipconv(&read_text(&dir.join(EQUIPCONV_NAME))),
            uop: crate::anim_uop::UopAnims::open(dir),
        })
    }

    /// The action that shows a deed of a body. None when the body has no
    /// pictures for it.
    pub fn deed_action(&self, body: u16, deed: Deed, mounted: bool) -> Option<Action> {
        let (file, body_in_file) = self.moved.get(&body).copied().unwrap_or((0, body));
        let (kind, _) = self.kind_of(body, body_in_file, file);
        let column = match kind {
            BodyKind::Person if mounted => 1,
            BodyKind::Person => 0,
            BodyKind::Monster => 2,
            BodyKind::SeaMonster | BodyKind::Animal => 3,
        };
        deed_groups(deed)[column].map(Action::Shown)
    }

    /// The action of a person on foot for the way he holds himself. A
    /// person in war mode stands ready, and one with a weapon in his hand
    /// swings his arms in a different way. Other bodies keep their action.
    pub fn stance_action(&self, body: u16, action: Action, stance: Stance) -> Action {
        if !self.is_person(body) {
            return action;
        }
        match (action, stance.war, stance.armed) {
            (Action::Stand, true, _) => Action::Shown(PEOPLE_STAND_WAR),
            (Action::Walk, true, _) => Action::Shown(PEOPLE_WALK_WAR),
            (Action::Walk, false, true) => Action::Shown(PEOPLE_WALK_ARMED),
            (Action::Run, _, true) => Action::Shown(PEOPLE_RUN_ARMED),
            (other, ..) => other,
        }
    }

    /// What a worn item with animation `worn_anim` shows as on `body`.
    pub fn equip_conv(&self, body: u16, worn_anim: u16) -> Option<EquipConv> {
        self.equip_conv.get(&(body, worn_anim)).copied()
    }

    /// True when the pictures of this body are those of a person, so worn
    /// items show on it.
    pub fn is_person(&self, body: u16) -> bool {
        let (file, body_in_file) = self.moved.get(&body).copied().unwrap_or((0, body));
        self.kind_of(body, body_in_file, file).0 == BodyKind::Person
    }

    /// The kind `mobtypes.txt` gives the body. When it gives none, the kind
    /// comes from the number the body has in its file.
    fn kind_of(&self, body: u16, body_in_file: u16, file: usize) -> (BodyKind, u32) {
        self.kinds
            .get(&body)
            .copied()
            .unwrap_or((default_kind(body_in_file, file), 0))
    }

    /// Each frame of a body that does `action` and looks toward `facing`.
    /// `mounted` is for a person who sits on a mount. A body with no
    /// pictures for a walk or a run gives its standing pictures.
    pub fn frames(
        &self,
        body: u16,
        facing: Facing,
        action: Action,
        mounted: bool,
    ) -> Option<Vec<Option<AnimFrame>>> {
        let mut body = body;
        let mut file_hue = 0;
        for _ in 0..BODY_DEF_MAX_HOPS {
            let (file, body_in_file) = self.moved.get(&body).copied().unwrap_or((0, body));
            let (kind, flags) = self.kind_of(body, body_in_file, file);
            let read = |action: Action| {
                if flags & FLAG_USE_UOP != 0 {
                    let group = action.group(uop_groups(kind, mounted));
                    return self.uop.as_ref()?.frames(body, group, facing.stored);
                }
                let (first, groups) = first_record_and_groups(body_in_file, kind, flags, mounted)?;
                let record = first + action.group(groups) * DIRECTIONS_STORED + facing.stored;
                self.read_frames(file, record)
            };
            if let Some(frames) = read(action).or_else(|| read(Action::Stand)) {
                return Some(
                    frames
                        .into_iter()
                        .map(|frame| {
                            frame.map(|(center_x, center_y, pixels)| AnimFrame {
                                center_x,
                                center_y,
                                pixels,
                                file_hue,
                            })
                        })
                        .collect(),
                );
            }
            match self.shown_as.get(&body) {
                Some((shown, hue)) if file == 0 && *shown != body => {
                    body = *shown;
                    file_hue = *hue;
                }
                _ => return None,
            }
        }
        None
    }

    fn read_frames(&self, file: usize, record: u32) -> Option<Vec<Option<(i32, i32, ArtPixels)>>> {
        let file = self.files.get(file)?.as_ref()?;
        let rec = slice_at(&file.idx, record as usize * IDX_RECORD, IDX_RECORD)?;
        let offset = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]);
        let len = u32::from_le_bytes([rec[4], rec[5], rec[6], rec[7]]);
        if offset == IDX_EMPTY || len == IDX_EMPTY || len == 0 {
            return None;
        }
        let mut mul = file.mul.lock().ok()?;
        let file_len = mul.metadata().ok()?.len();
        let mut data = vec![0u8; capped_len(file_len, u64::from(offset), len)];
        mul.seek(SeekFrom::Start(u64::from(offset))).ok()?;
        mul.read_exact(&mut data).ok()?;
        decode_frames(&data)
    }
}

fn dword_at(data: &[u8], at: usize) -> Option<u32> {
    slice_at(data, at, DWORD).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn short_at(data: &[u8], at: usize) -> Option<i32> {
    slice_at(data, at, WORD).map(|b| i32::from(i16::from_le_bytes([b[0], b[1]])))
}

/// Ten bits with a sign, as the run header holds its two offsets.
fn run_offset(header: u32, shift: u32) -> i32 {
    let value = ((header >> shift) & RUN_OFFSET_MASK) as i32;
    if value & RUN_OFFSET_SIGN != 0 {
        value - RUN_OFFSET_RANGE
    } else {
        value
    }
}

/// A direction is a palette of 256 colors, a count of frames, one offset for
/// each frame, and the frames. The offsets count from the end of the palette.
/// A frame is a header and runs. Each run says where it goes, how long it is,
/// and then one palette number for each pixel.
/// An empty frame keeps its place as None. A worn item then stays in step
/// with the body that wears it.
fn decode_frames(data: &[u8]) -> Option<Vec<Option<(i32, i32, ArtPixels)>>> {
    let palette = slice_at(data, 0, PALETTE_BYTES)?;
    let count = (dword_at(data, PALETTE_BYTES)? as usize).min(MAX_FRAMES);
    let frames: Vec<_> = (0..count)
        .map(|i| {
            let offset = dword_at(data, PALETTE_BYTES + DWORD * (i + 1))?;
            decode_frame(data, palette, PALETTE_BYTES + offset as usize)
        })
        .collect();
    frames.iter().any(Option::is_some).then_some(frames)
}

pub(crate) fn decode_frame(
    data: &[u8],
    palette: &[u8],
    frame: usize,
) -> Option<(i32, i32, ArtPixels)> {
    let center_x = short_at(data, frame)?;
    let center_y = short_at(data, frame + WORD)?;
    let width = usize::try_from(short_at(data, frame + WORD * 2)?).ok()?;
    let height = usize::try_from(short_at(data, frame + WORD * 3)?).ok()?;
    if width == 0 || height == 0 || width > FRAME_MAX_SIDE || height > FRAME_MAX_SIDE {
        return None;
    }
    let mut pixels = ArtPixels {
        width,
        height,
        colors: vec![0; width * height],
    };
    let mut at = frame + FRAME_HEADER_BYTES;
    loop {
        let header = dword_at(data, at)?;
        at += DWORD;
        if header == RUN_END {
            break;
        }
        let run = (header & RUN_LENGTH_MASK) as usize;
        let x = run_offset(header, RUN_X_SHIFT) + center_x;
        let y = run_offset(header, RUN_Y_SHIFT) + center_y + height as i32;
        let indexes = slice_at(data, at, run)?;
        at += run;
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            continue;
        };
        if y >= height || x + run > width {
            continue;
        }
        for (i, index) in indexes.iter().enumerate() {
            let color = usize::from(*index) * WORD;
            pixels.colors[y * width + x + i] =
                u16::from_le_bytes([palette[color], palette[color + 1]]) | PIXEL_DRAWN;
        }
    }
    Some((center_x, center_y, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY_HORSE: u16 = 200;
    const BODY_MAN: u16 = 400;
    const BODY_OGRE: u16 = 1;
    const SOUTH: u8 = 4;
    const EAST: u8 = 2;

    #[test]
    fn the_newer_packet_names_a_deed_and_each_kind_of_body_has_its_group() {
        assert_eq!(Deed::from_packet(0, 3), Some(Deed::Attack));
        assert_eq!(Deed::from_packet(7, 0), Some(Deed::Bow));
        assert_eq!(Deed::from_packet(7, 1), Some(Deed::Salute));
        assert_eq!(Deed::from_packet(11, 0), Some(Deed::CastAtOne));
        assert_eq!(Deed::from_packet(9, 0), None);
        let [on_foot, mounted, monster, animal] = deed_groups(Deed::Attack);
        assert_eq!(
            (on_foot, mounted, monster, animal),
            (Some(9), Some(26), Some(4), Some(5))
        );
        assert_eq!(deed_groups(Deed::Bow)[2], None, "a monster does not bow");
    }

    #[test]
    fn a_body_of_the_newer_packages_has_pictures_when_client_files_are_here() {
        const BODY_GARGOYLE_MAN: u16 = 666;
        let Some(dir) = crate::mul::client_data_dir_from_env() else {
            return;
        };
        let anim = AnimData::open(&dir).unwrap();
        let south = Facing::from_direction(4);
        let stand = anim
            .frames(BODY_GARGOYLE_MAN, south, Action::Stand, false)
            .unwrap();
        let walk = anim
            .frames(BODY_GARGOYLE_MAN, south, Action::Walk, false)
            .unwrap();
        assert!(stand.iter().flatten().count() > 0);
        assert!(walk.len() > 1 && walk.iter().flatten().count() > 1);
        let frame = walk.iter().flatten().next().unwrap();
        assert!(frame.pixels.colors.iter().any(|c| c & PIXEL_DRAWN != 0));
    }

    #[test]
    fn each_kind_of_body_starts_at_its_own_record() {
        let at = |body, kind| first_record_and_groups(body, kind, 0, false);
        assert_eq!(at(BODY_OGRE, BodyKind::Monster), Some((110, GROUPS_HIGH)));
        assert_eq!(at(BODY_HORSE, BodyKind::Animal), Some((22_000, GROUPS_LOW)));
        assert_eq!(
            at(BODY_MAN, BodyKind::Person),
            Some((35_000, GROUPS_PEOPLE))
        );
        assert_eq!(
            first_record_and_groups(BODY_MAN, BodyKind::Person, 0, true),
            Some((35_000, GROUPS_PEOPLE_MOUNTED))
        );
        assert_eq!(Action::Run.group(GROUPS_PEOPLE), 2);
        assert_eq!(Action::Shown(9).group(GROUPS_PEOPLE), 9);
        assert_eq!(at(BODY_OGRE, BodyKind::Animal), None);
    }

    #[test]
    fn east_is_south_mirrored() {
        let south = Facing::from_direction(SOUTH);
        let east = Facing::from_direction(EAST);
        assert_eq!(south.stored, east.stored);
        assert!(east.mirrored && !south.mirrored);
    }

    #[test]
    fn tables_read_rows_and_skip_comments() {
        let kinds = parse_mobtypes("# c\n5\tANIMAL\t2A # bird\n400 HUMAN 0\n");
        assert_eq!(kinds[&5], (BodyKind::Animal, 0x2A));
        assert_eq!(kinds[&BODY_MAN].0, BodyKind::Person);
        let moved = parse_bodyconv("\"# text\"\n157\t1\t-1\t-1\t-1\n20\t4\t-1\t9\t-1\n");
        assert_eq!(moved[&157], (1, 1));
        assert_eq!(moved[&20], (3, 9));
        let shown = parse_body_def("11 {28} 1401\n12 {1, 2, 3} 7\n");
        assert_eq!(shown[&11], (28, 1401));
        assert_eq!(shown[&12], (3, 7));
        let conv = parse_equipconv(
            "401\t1249 1250 61250\t0\t#\tHuman M to F\n605 5 6 0 0\n605 7 8 -1 0\n",
        );
        assert_eq!(
            conv[&(401, 1249)],
            EquipConv {
                anim: 1250,
                gump: 61250,
                hue: 0
            }
        );
        assert_eq!(conv[&(605, 5)].gump, 5);
        assert_eq!(conv[&(605, 7)].gump, 8);
    }

    #[test]
    fn a_run_lands_at_its_offset_from_the_center() {
        const WHITE: u16 = 0x7FFF;
        const WHITE_INDEX: u8 = 1;
        let mut data = vec![0u8; PALETTE_BYTES];
        data[2..4].copy_from_slice(&WHITE.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend(8u32.to_le_bytes());
        // Center (1, 0), 3 wide, 2 high. One run of 2 at x -1, y -1.
        for value in [1i16, 0, 3, 2] {
            data.extend(value.to_le_bytes());
        }
        let minus_one = RUN_OFFSET_MASK;
        data.extend(((minus_one << RUN_X_SHIFT) | (minus_one << RUN_Y_SHIFT) | 2).to_le_bytes());
        data.extend([WHITE_INDEX, WHITE_INDEX]);
        data.extend(RUN_END.to_le_bytes());
        let frames = decode_frames(&data).unwrap();
        let (center_x, center_y, pixels) = frames[0].as_ref().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!((*center_x, *center_y), (1, 0));
        let drawn = WHITE | PIXEL_DRAWN;
        assert_eq!(pixels.colors, vec![0, 0, 0, drawn, drawn, 0]);
    }

    #[test]
    fn real_files_give_a_man_and_a_horse() {
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let anim = AnimData::open(dir).unwrap();
        let south = Facing::from_direction(SOUTH);
        let man = anim.frames(BODY_MAN, south, Action::Stand, false).unwrap();
        assert!(man[0].as_ref().unwrap().pixels.height > 40);
        let walk = anim.frames(BODY_MAN, south, Action::Walk, false).unwrap();
        assert!(
            walk.len() > man.len(),
            "a walk has more frames than a stand"
        );
        assert!(anim.frames(BODY_HORSE, south, Action::Run, false).is_some());
        assert!(anim.is_person(BODY_MAN) && !anim.is_person(BODY_HORSE));
    }
}
