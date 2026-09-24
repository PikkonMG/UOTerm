//! What the classic client knows of each container gump, as the reference client
//! and its `containers.txt` keep it: the sounds it makes,
//! the box its items lie in, and its small picture; which picture a
//! backpack style or the large container option shows; where a dropped
//! item lands; and where a new container gump opens. There is no drawing
//! here.
//!
//! A player may write his own `containers.txt` in the config folder, in the
//! format of the reference client; its lines take the place of the table.

use crate::window::settings::{BackpackStyle, ContainerPlace};
use std::collections::HashMap;
use uoterm_runtime::config::config_dir;

/// The file a player writes his own container gumps into.
pub const CONTAINERS_FILE: &str = "containers.txt";
/// A line that starts with one of these is a comment.
const COMMENT_MARKS: [char; 2] = ['#', ';'];
const FIELD_SEPARATORS: [char; 3] = [' ', '\t', ','];
/// Graphic, open sound, close sound, left, top, right, bottom.
const LEAST_FIELDS: usize = 7;
/// The minimizer of a container is a square of this side.
pub const MINIMIZER_SIDE: i32 = 16;

pub const CHESSBOARD_GUMP: u16 = 0x091A;
pub const BACKGAMMON_GUMP: u16 = 0x092E;
/// The pieces of a game board are gump pictures: the item graphic less
/// this.
pub const BOARD_PIECE_OFFSET: u16 = 11369;
/// The chessboard shows its pieces this much higher than they lie.
pub const CHESSBOARD_LIFT: i32 = 20;
const DEFAULT_BACKPACK_GUMP: u16 = 0x003C;
const SUEDE_BACKPACK_GUMP: u16 = 0x775E;
const POLAR_BEAR_BACKPACK_GUMP: u16 = 0x7760;
const GHOUL_SKIN_BACKPACK_GUMP: u16 = 0x7762;

/// The box a gump with no line in the table keeps its items in.
const UNKNOWN_BOUNDS: Bounds = Bounds::new(44, 65, 186, 159);

// Where a new container gump opens, as the classic client cascades them.
const CASCADE_STEP: f32 = 20.0;
const CASCADE_START: f32 = 40.0;
const CASCADE_LINE_STEP: f32 = 800.0;
const CASCADE_TRIES: usize = 4;
/// A container near a thing of the world opens this far to its right.
const NEAR_THING_GAP: f32 = 40.0;
const HALF: f32 = 2.0;

/// The box of a container gump its items lie in, in its pixels. The right
/// and bottom sides are where an item may lie at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bounds {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// One container gump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerData {
    pub graphic: u16,
    pub open_sound: u16,
    pub close_sound: u16,
    pub bounds: Bounds,
    /// The small picture the gump folds into; zero when it cannot fold.
    pub iconized: u16,
    /// The top left corner of the square that folds the gump.
    pub minimizer: Option<(i32, i32)>,
}

/// Graphic, open sound, close sound, bounds, small picture, minimizer.
type Row = (u16, u16, u16, [i32; 4], u16, Option<(i32, i32)>);

/// The reference client's table of container gumps.
#[rustfmt::skip]
const STANDARD: &[Row] = &[
    (0x0007, 0x0000, 0x0000, [30, 30, 270, 170], 0, None),
    (0x0009, 0x0000, 0x0000, [20, 85, 124, 196], 0, None),
    (0x003C, 0x0048, 0x0058, [44, 65, 186, 159], 0x0050, Some((105, 162))),
    (0x003D, 0x0048, 0x0058, [29, 34, 137, 128], 0, None),
    (0x003E, 0x002F, 0x002E, [33, 36, 142, 148], 0, None),
    (0x003F, 0x004F, 0x0058, [19, 47, 182, 123], 0, None),
    (0x0040, 0x002D, 0x002C, [16, 38, 152, 125], 0, None),
    (0x0041, 0x004F, 0x0058, [40, 30, 139, 123], 0, None),
    (0x0042, 0x002D, 0x002C, [18, 105, 162, 178], 0, None),
    (0x0043, 0x002D, 0x002C, [16, 51, 184, 124], 0, None),
    (0x0044, 0x002D, 0x002C, [20, 10, 170, 100], 0, None),
    (0x0047, 0x0000, 0x0000, [16, 10, 148, 138], 0, None),
    (0x0048, 0x002F, 0x002E, [16, 10, 154, 94], 0, None),
    (0x0049, 0x002D, 0x002C, [18, 105, 162, 178], 0, None),
    (0x004A, 0x002D, 0x002C, [18, 105, 162, 178], 0, None),
    (0x004B, 0x002D, 0x002C, [16, 51, 184, 124], 0, None),
    (0x004C, 0x002D, 0x002C, [46, 74, 196, 184], 0, None),
    (0x004D, 0x002F, 0x002E, [76, 12, 140, 68], 0, None),
    (0x004E, 0x002D, 0x002C, [24, 18, 100, 152], 0, None),
    (0x004F, 0x002D, 0x002C, [24, 18, 100, 152], 0, None),
    (0x0051, 0x002F, 0x002E, [16, 10, 154, 94], 0, None),
    (0x0052, 0x0000, 0x0000, [0, 0, 110, 62], 0, None),
    (0x0102, 0x004F, 0x0058, [35, 10, 190, 95], 0, None),
    (0x0103, 0x0048, 0x0058, [41, 21, 173, 104], 0, None),
    (0x0104, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x0105, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x0106, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x0107, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x0108, 0x004F, 0x0058, [10, 10, 160, 105], 0, None),
    (0x0109, 0x002D, 0x002C, [10, 10, 160, 105], 0, None),
    (0x010A, 0x002D, 0x002C, [10, 10, 160, 105], 0, None),
    (0x010B, 0x002D, 0x002C, [10, 10, 160, 105], 0, None),
    (0x010C, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x010D, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x010E, 0x002F, 0x002E, [10, 10, 160, 105], 0, None),
    (0x0116, 0x0000, 0x0000, [40, 25, 140, 110], 0, None),
    (0x011A, 0x0000, 0x0000, [10, 65, 125, 160], 0, None),
    (0x011B, 0x0000, 0x0000, [45, 10, 175, 95], 0, None),
    (0x011C, 0x0000, 0x0000, [37, 10, 175, 105], 0, None),
    (0x011D, 0x0000, 0x0000, [43, 10, 165, 110], 0, None),
    (0x011E, 0x0000, 0x0000, [30, 22, 263, 106], 0, None),
    (0x011F, 0x0000, 0x0000, [45, 10, 175, 95], 0, None),
    (0x0120, 0x0000, 0x0000, [56, 30, 160, 107], 0, None),
    (0x0121, 0x0000, 0x0000, [77, 32, 162, 107], 0, None),
    (0x0123, 0x0000, 0x0000, [36, 19, 111, 157], 0, None),
    (0x0484, 0x0000, 0x0000, [0, 45, 175, 125], 0, None),
    (0x058E, 0x0000, 0x0000, [50, 150, 348, 250], 0, None),
    (0x06D3, 0x0000, 0x0000, [10, 65, 125, 160], 0, None),
    (0x06D4, 0x0000, 0x0000, [10, 65, 125, 160], 0, None),
    (0x06D5, 0x0000, 0x0000, [10, 65, 125, 160], 0, None),
    (0x06D6, 0x0000, 0x0000, [10, 65, 125, 160], 0, None),
    (0x06E5, 0x0000, 0x0000, [66, 74, 306, 520], 0, None),
    (0x06E6, 0x0000, 0x0000, [66, 74, 306, 520], 0, None),
    (0x06E7, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x06E8, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x06E9, 0x0000, 0x0000, [60, 80, 318, 324], 0, None),
    (0x06EA, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x091A, 0x0000, 0x0000, [0, 0, 282, 230], 0, None),
    (0x092E, 0x0000, 0x0000, [0, 0, 282, 210], 0, None),
    (0x266A, 0x0000, 0x0000, [16, 51, 184, 124], 0, None),
    (0x266B, 0x0000, 0x0000, [16, 51, 184, 124], 0, None),
    (0x2A63, 0x0187, 0x01C9, [60, 33, 460, 348], 0, None),
    (0x4D0C, 0x0000, 0x0000, [25, 65, 220, 155], 0, None),
    (0x775E, 0x0048, 0x0058, [44, 65, 186, 159], 0x775F, Some((105, 178))),
    (0x7760, 0x0048, 0x0058, [44, 65, 186, 159], 0x7761, Some((105, 178))),
    (0x7762, 0x0048, 0x0058, [44, 65, 186, 159], 0x7763, Some((105, 178))),
    (0x777A, 0x0000, 0x0000, [32, 40, 184, 116], 0, None),
    (0x9CD9, 0x0000, 0x0000, [10, 10, 160, 105], 0, None),
    (0x9CDB, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x9CDD, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x9CDF, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x9CE3, 0x0000, 0x0000, [50, 60, 548, 308], 0, None),
    (0x9CE4, 0x0000, 0x0000, [44, 65, 186, 159], 0, None),
    (0x9CE5, 0x0000, 0x0000, [44, 65, 186, 159], 0, None),
    (0x9CE7, 0x0000, 0x0000, [44, 65, 186, 159], 0, None),
];

/// The large picture that takes the place of a container gump with the
/// "Use large containers" option.
const LARGE_GUMPS: [(u16, u16); 9] = [
    (0x0048, 0x06E8),
    (0x0049, 0x9CDF),
    (0x0051, 0x06E7),
    (0x003E, 0x06E9),
    (0x004D, 0x06EA),
    (0x004E, 0x06E6),
    (0x004F, 0x06E5),
    (0x004A, 0x9CDD),
    (0x0044, 0x9CE3),
];

fn from_row(
    (graphic, open_sound, close_sound, [left, top, right, bottom], iconized, minimizer): Row,
) -> ContainerData {
    ContainerData {
        graphic,
        open_sound,
        close_sound,
        bounds: Bounds::new(left, top, right, bottom),
        iconized,
        minimizer,
    }
}

/// Every container gump the client knows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerTable {
    rows: HashMap<u16, ContainerData>,
}

impl ContainerTable {
    /// The reference client's table.
    pub fn standard() -> Self {
        Self {
            rows: STANDARD.iter().map(|row| (row.0, from_row(*row))).collect(),
        }
    }

    /// The player's `containers.txt` in the config folder, or the standard
    /// table when he wrote none.
    pub fn load() -> Self {
        std::fs::read_to_string(config_dir().join(CONTAINERS_FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_else(|_| Self::standard())
    }

    /// The lines of a `containers.txt`: graphic, open sound, close sound,
    /// left, top, right, bottom, and then the small picture and the corner
    /// of the minimizer when the gump folds. A line that does not read is
    /// left out.
    pub fn parse(text: &str) -> Self {
        let rows = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with(COMMENT_MARKS))
            .filter_map(parse_line)
            .map(|data| (data.graphic, data))
            .collect();
        Self { rows }
    }

    /// The data of a container gump. A gump the table does not know keeps
    /// its items in the box of the backpack and makes no sound.
    pub fn get(&self, graphic: u16) -> ContainerData {
        self.rows.get(&graphic).copied().unwrap_or(ContainerData {
            graphic,
            open_sound: 0,
            close_sound: 0,
            bounds: UNKNOWN_BOUNDS,
            iconized: 0,
            minimizer: None,
        })
    }
}

fn parse_line(line: &str) -> Option<ContainerData> {
    let fields: Vec<&str> = line
        .split(FIELD_SEPARATORS)
        .filter(|field| !field.is_empty())
        .collect();
    if fields.len() < LEAST_FIELDS {
        return None;
    }
    let number = |at: usize| fields.get(at).and_then(|field| field.parse::<i32>().ok());
    let short = |at: usize| fields.get(at).and_then(|field| field.parse::<u16>().ok());
    let (minimizer_x, minimizer_y) = (number(8).unwrap_or(0), number(9).unwrap_or(0));
    Some(ContainerData {
        graphic: short(0)?,
        open_sound: short(1)?,
        close_sound: short(2)?,
        bounds: Bounds::new(number(3)?, number(4)?, number(5)?, number(6)?),
        iconized: short(7).unwrap_or(0),
        minimizer: (minimizer_x != 0 || minimizer_y != 0).then_some((minimizer_x, minimizer_y)),
    })
}

/// The picture of the character's own backpack in a style.
pub fn backpack_gump(style: BackpackStyle) -> u16 {
    match style {
        BackpackStyle::Default => DEFAULT_BACKPACK_GUMP,
        BackpackStyle::Suede => SUEDE_BACKPACK_GUMP,
        BackpackStyle::PolarBear => POLAR_BEAR_BACKPACK_GUMP,
        BackpackStyle::GhoulSkin => GHOUL_SKIN_BACKPACK_GUMP,
    }
}

/// The large picture of a container gump, when it has one.
pub fn large_gump(gump: u16) -> Option<u16> {
    LARGE_GUMPS
        .iter()
        .find(|(small, _)| *small == gump)
        .map(|(_, large)| *large)
}

/// What the gump of a container shows: the backpack in its style, a large
/// picture when the option asks for it, or the gump the shard sent. A
/// picture the client files lack (`has_art` says no) is not used.
pub fn shown_gump(
    gump: u16,
    backpack: Option<BackpackStyle>,
    large: bool,
    mut has_art: impl FnMut(u16) -> bool,
) -> u16 {
    let wanted = match backpack {
        Some(style) => Some(backpack_gump(style)),
        None if large => large_gump(gump),
        None => None,
    };
    wanted.filter(|picture| has_art(*picture)).unwrap_or(gump)
}

/// A board of a game: its pieces are gump pictures and it does not scale.
pub fn is_game_board(gump: u16) -> bool {
    gump == CHESSBOARD_GUMP || gump == BACKGAMMON_GUMP
}

/// Where a dropped item lands in a container gump, in the pixels of the
/// gump, as the classic client works it out. `mouse` is where the mouse is
/// on the gump and `picture` the size of the item picture, both in pixels
/// of the gump as it is drawn, at the container `scale`. With relative
/// drag and drop, `mouse` already holds the pull of where the item was
/// grabbed.
pub fn drop_spot(bounds: Bounds, scale: f32, mouse: (i32, i32), picture: (i32, i32)) -> (u16, u16) {
    let scaled = |side: i32| (side as f32 * scale) as i32;
    let (left, top) = (scaled(bounds.left), scaled(bounds.top));
    let (right, bottom) = (scaled(bounds.right), scaled(bounds.bottom));
    let (width, height) = picture;
    let mut x = mouse.0 - width / 2;
    let mut y = mouse.1 - height / 2;
    if x + width > right {
        x = right - width;
    }
    if y + height > bottom {
        y = bottom - height;
    }
    x = x.max(left);
    y = y.max(top);
    let unscaled = |side: i32| (side as f32 / scale).max(0.0) as u16;
    (unscaled(x), unscaled(y))
}

/// The place the cascade of container gumps starts from, in window points.
pub fn cascade_start(zoom: f32) -> (f32, f32) {
    (CASCADE_START * zoom, CASCADE_START * zoom)
}

/// The next place of the cascade the classic client opens containers in,
/// from the last one, in window points. `zoom` scales the steps of the
/// cascade to the window.
pub fn cascade(last: (f32, f32), size: (f32, f32), screen: (f32, f32), zoom: f32) -> (f32, f32) {
    let (step, start, line) = (
        CASCADE_STEP * zoom,
        CASCADE_START * zoom,
        CASCADE_LINE_STEP * zoom,
    );
    let (mut x, mut y) = last;
    let (width, height) = size;
    let passed = (1..=CASCADE_TRIES).find(|_| {
        if x + width + step > screen.0 {
            x = start;
            y = if y + height + line > screen.1 {
                start
            } else {
                y + line
            };
            false
        } else if y + height + step > screen.1 {
            x = if x + width + line > screen.0 {
                start
            } else {
                x + line
            };
            y = start;
            false
        } else {
            true
        }
    });
    match passed {
        None => (start, start),
        Some(1) => (x + step, y + step),
        Some(_) => (x, y),
    }
}

/// Where a container gump opens when its place is overridden, before it is
/// kept on the screen. `near` is the place of the thing that holds it on
/// the screen, when it is known; `last_dragged` the middle of the container
/// gump dragged last.
pub fn overridden_place(
    rule: ContainerPlace,
    size: (f32, f32),
    screen: (f32, f32),
    near: Option<(f32, f32)>,
    last_dragged: Option<(f32, f32)>,
) -> Option<(f32, f32)> {
    let (width, height) = size;
    let (mut x, mut y) = match rule {
        ContainerPlace::NearContainer => near?,
        ContainerPlace::TopRight => (screen.0 - width, 0.0),
        ContainerPlace::LastDragged | ContainerPlace::RememberEach => {
            let (middle_x, middle_y) = last_dragged?;
            (middle_x - width / HALF, middle_y - height / HALF)
        }
    };
    if x + width > screen.0 {
        x -= width;
    }
    if y + height > screen.1 {
        y -= height;
    }
    Some((x, y))
}

/// The place near a thing of the world a container opens at: a little to
/// its right, its middle at the height of the thing.
pub fn near_thing(thing: (f32, f32), height: f32, zoom: f32) -> (f32, f32) {
    (thing.0 + NEAR_THING_GAP * zoom, thing.1 - height / HALF)
}

/// The place near the gump of the container that holds it: half its own
/// width to the right, at the same height.
pub fn near_parent_gump(parent: (f32, f32), width: f32) -> (f32, f32) {
    (parent.0 + width / HALF, parent.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_knows_the_backpack_and_an_unknown_gump_gets_the_default_box() {
        let table = ContainerTable::standard();
        let pack = table.get(DEFAULT_BACKPACK_GUMP);
        assert_eq!(pack.open_sound, 0x0048);
        assert_eq!(pack.bounds, Bounds::new(44, 65, 186, 159));
        assert_eq!(pack.iconized, 0x0050);
        assert_eq!(pack.minimizer, Some((105, 162)));
        let unknown = table.get(0x1234);
        assert_eq!(unknown.bounds, UNKNOWN_BOUNDS);
        assert_eq!((unknown.open_sound, unknown.iconized), (0, 0));
    }

    #[test]
    fn a_containers_file_reads_its_lines_and_skips_comments_and_bad_lines() {
        let table = ContainerTable::parse(
            "# FORMAT\n; another comment\n60 72 88 44 65 186 159 80 105 162\n9,0,0,20,85,124,196\nbad line\n",
        );
        assert_eq!(table.get(60).minimizer, Some((105, 162)));
        assert_eq!(table.get(60).iconized, 80);
        assert_eq!(table.get(9).bounds, Bounds::new(20, 85, 124, 196));
        assert_eq!(table.get(9).minimizer, None);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn the_shown_gump_follows_the_backpack_style_and_the_large_option() {
        let every = |_: u16| true;
        let none = |_: u16| false;
        assert_eq!(
            shown_gump(0x003C, Some(BackpackStyle::PolarBear), false, every),
            POLAR_BEAR_BACKPACK_GUMP
        );
        assert_eq!(
            shown_gump(0x003C, Some(BackpackStyle::Suede), false, none),
            0x003C
        );
        assert_eq!(shown_gump(0x0048, None, true, every), 0x06E8);
        assert_eq!(shown_gump(0x0048, None, false, every), 0x0048);
        assert_eq!(shown_gump(0x0009, None, true, every), 0x0009);
    }

    #[test]
    fn a_drop_lands_centered_on_the_mouse_and_inside_the_box() {
        let bounds = Bounds::new(44, 65, 186, 159);
        assert_eq!(drop_spot(bounds, 1.0, (100, 100), (20, 10)), (90, 95));
        // Past the right and the bottom it is pulled back in.
        assert_eq!(drop_spot(bounds, 1.0, (190, 170), (20, 10)), (166, 149));
        // Before the left and the top it is pushed in.
        assert_eq!(drop_spot(bounds, 1.0, (0, 0), (20, 10)), (44, 65));
        // At twice the size the place is halved back to gump pixels.
        assert_eq!(drop_spot(bounds, 2.0, (200, 200), (40, 20)), (90, 95));
    }

    #[test]
    fn containers_cascade_and_start_again_at_the_edge() {
        let screen = (1000.0, 800.0);
        assert_eq!(
            cascade((40.0, 40.0), (200.0, 200.0), screen, 1.0),
            (60.0, 60.0)
        );
        // At the right edge it goes back to the start of the next line,
        // which is off the screen, so it starts over at the top.
        assert_eq!(
            cascade((900.0, 40.0), (200.0, 200.0), screen, 1.0),
            (40.0, 40.0)
        );
    }

    #[test]
    fn an_overridden_place_follows_its_rule_and_stays_on_the_screen() {
        let screen = (1000.0, 800.0);
        let size = (200.0, 100.0);
        assert_eq!(
            overridden_place(ContainerPlace::TopRight, size, screen, None, None),
            Some((800.0, 0.0))
        );
        assert_eq!(
            overridden_place(
                ContainerPlace::LastDragged,
                size,
                screen,
                None,
                Some((500.0, 400.0))
            ),
            Some((400.0, 350.0))
        );
        assert_eq!(
            overridden_place(
                ContainerPlace::NearContainer,
                size,
                screen,
                Some((900.0, 750.0)),
                None
            ),
            Some((700.0, 650.0))
        );
        assert_eq!(
            overridden_place(ContainerPlace::NearContainer, size, screen, None, None),
            None
        );
        assert_eq!(near_thing((100.0, 200.0), 100.0, 1.0), (140.0, 150.0));
        assert_eq!(near_parent_gump((100.0, 200.0), 60.0), (130.0, 200.0));
    }
}
