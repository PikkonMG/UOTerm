//! The house designer, apart from how it draws: the part the player builds
//! with (its kind, its style and which of its pieces), whether he takes
//! parts off instead or picks a part off the house with the eyedropper,
//! what a click on the house does, how each storey shows while he designs,
//! and the components, fixtures and cost of the design as the reference
//! client counts them. The Modern panel (`build_ui`) and the classic gump
//! (`classic/house.rs`) change one shared design, which the clicks on the
//! map and the drawing of the house read.

use crate::view::WatchFrame;
use crate::window::control::Act;
use std::cell::RefCell;
use std::rc::Rc;
use uoterm_nav::{HousePart, HousePartKind};

/// How many levels a house can have.
pub const DESIGNER_FLOORS: u8 = 4;
/// The storeys of a house, as an array holds one look for each.
pub const STOREYS: usize = DESIGNER_FLOORS as usize;
/// A plot this many tiles wide or deep has four storeys; a smaller one
/// three.
const LARGE_PLOT: i32 = 13;
const STOREYS_LARGE: u8 = 4;
const STOREYS_SMALL: u8 = 3;
/// A house takes one fixture for this many components.
const COMPONENTS_PER_FIXTURE: i32 = 20;
/// The reference client's share of the floor tiles its limit adds for each storey.
const FLOOR_SHARE: f64 = -0.25;
/// Each component and fixture of a design costs this much gold.
pub const COST_PER_PART: u32 = 500;
/// The first storey lies this high over the foundation, and each storey is
/// this high.
const FIRST_STOREY_DZ: i32 = 7;
const STOREY_HEIGHT: i32 = 20;
/// The reference client's words of the limits of a design (text 1061039).
pub const WORDS_LIMITS: &str = "Components | Fixtures";
pub const WORDS_FIXTURES_ARE: &str = "Fixtures are doors and teleporters.";

/// The kinds of part, and the designer action that puts one on the house.
pub const KINDS: [(HousePartKind, &str); 7] = [
    (HousePartKind::Wall, "add"),
    (HousePartKind::Floor, "add"),
    (HousePartKind::Door, "add"),
    (HousePartKind::Stair, "stair"),
    (HousePartKind::Roof, "roof"),
    (HousePartKind::Misc, "add"),
    (HousePartKind::Teleporter, "add"),
];
/// The steps of the designer that name no part, with their words, in the
/// order the system menu of the classic client lays them.
pub const DESIGN_COMMANDS: [(&str, &str); 6] = [
    ("Backup", "backup"),
    ("Restore", "restore"),
    ("Sync", "sync"),
    ("Clear", "clear"),
    ("Commit", "commit"),
    ("Revert", "revert"),
];
/// The step that leaves the designer.
pub const ACTION_EXIT: &str = "exit";
const ACTION_ADD: &str = "add";
const ACTION_REMOVE: &str = "remove";
const ACTION_REMOVE_ROOF: &str = "remove_roof";

const HINT_BUILD: &str = "Click the house to build here.";
const HINT_REMOVE: &str = "Click a part of the house to take it off.";
const HINT_PICK: &str = "Click a part of the house to build with it.";

/// The design both styles change.
pub type SharedDesign = Rc<RefCell<HouseDesign>>;

/// The action that puts a part of this kind on the house.
pub fn action_for(kind: HousePartKind) -> &'static str {
    KINDS
        .iter()
        .find(|(known, _)| *known == kind)
        .map_or(ACTION_ADD, |(_, action)| *action)
}

/// The action that takes a part of this kind off the house.
pub fn remove_action_for(kind: HousePartKind) -> &'static str {
    if kind == HousePartKind::Roof {
        ACTION_REMOVE_ROOF
    } else {
        ACTION_REMOVE
    }
}

/// The styles of one kind, in the order the catalog lists them.
pub fn styles_of(parts: &[HousePart], kind: HousePartKind) -> Vec<&HousePart> {
    parts.iter().filter(|part| part.kind == kind).collect()
}

/// What Jev reads about each part of the catalog.
pub fn part_words(part: &HousePart) -> String {
    format!("{}: {}", part.kind.word(), part.name)
}

/// How one storey shows while the player designs, as the reference
/// client's storey button turns through them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StoreyLook {
    #[default]
    Normal,
    SeeThroughContent,
    HideContent,
    SeeThroughFloor,
    HideFloor,
    TranslucentFloor,
    HideAll,
}

const STOREY_LOOKS: [StoreyLook; 7] = [
    StoreyLook::Normal,
    StoreyLook::SeeThroughContent,
    StoreyLook::HideContent,
    StoreyLook::SeeThroughFloor,
    StoreyLook::HideFloor,
    StoreyLook::TranslucentFloor,
    StoreyLook::HideAll,
];

impl StoreyLook {
    /// The next look, round from the last to the first.
    pub fn next(self) -> Self {
        let at = STOREY_LOOKS
            .iter()
            .position(|look| *look == self)
            .unwrap_or(0);
        STOREY_LOOKS[(at + 1) % STOREY_LOOKS.len()]
    }

    pub fn words(self) -> &'static str {
        match self {
            StoreyLook::Normal => "All shown",
            StoreyLook::SeeThroughContent => "Walls see-through",
            StoreyLook::HideContent => "Walls hidden",
            StoreyLook::SeeThroughFloor => "Floor see-through",
            StoreyLook::HideFloor => "Floor hidden",
            StoreyLook::TranslucentFloor => "Floor translucent",
            StoreyLook::HideAll => "All hidden",
        }
    }

    /// The picture of its button, as the reference client's table has it: 0 all shown,
    /// 1 some see-through, 2 some hidden.
    pub fn button_look(self) -> usize {
        match self {
            StoreyLook::Normal => 0,
            StoreyLook::SeeThroughContent
            | StoreyLook::SeeThroughFloor
            | StoreyLook::TranslucentFloor => 1,
            StoreyLook::HideContent | StoreyLook::HideFloor | StoreyLook::HideAll => 2,
        }
    }
}

/// How a piece of the house shows while the player designs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceLook {
    Shown,
    SeeThrough,
    Hidden,
}

/// The storey a piece of the house stands on, from 0, by its height over
/// the foundation. None for a piece under the first storey or over the
/// last.
pub fn storey_of(dz: i32) -> Option<usize> {
    let over = dz - FIRST_STOREY_DZ;
    (0..STOREY_HEIGHT * STOREYS as i32)
        .contains(&over)
        .then_some((over / STOREY_HEIGHT) as usize)
}

/// The kind of part a graphic of the house is, by the catalog. None for a
/// graphic the catalog does not hold, such as the foundation.
pub fn kind_of(parts: &[HousePart], graphic: u16) -> Option<HousePartKind> {
    parts
        .iter()
        .find(|part| part.pieces.contains(&graphic))
        .map(|part| part.kind)
}

/// How a piece of `kind`, `dz` over the foundation, shows by the looks of
/// the storeys, as the reference client's designer marks it. A floor tile follows the
/// floor looks of its storey; any other piece the looks of what stands on
/// it; and all hidden hides both. A piece off the storeys keeps showing,
/// but a floor tile there follows the first storey.
pub fn piece_look(
    looks: &[StoreyLook; STOREYS],
    kind: Option<HousePartKind>,
    dz: i32,
) -> PieceLook {
    let storey = storey_of(dz);
    let look = looks[storey.unwrap_or(0)];
    let on_storey = storey.is_some();
    if kind == Some(HousePartKind::Floor) {
        return match look {
            StoreyLook::HideFloor => PieceLook::Hidden,
            StoreyLook::SeeThroughFloor | StoreyLook::TranslucentFloor => PieceLook::SeeThrough,
            StoreyLook::HideAll if on_storey => PieceLook::Hidden,
            _ => PieceLook::Shown,
        };
    }
    match look {
        _ if !on_storey => PieceLook::Shown,
        StoreyLook::HideContent | StoreyLook::HideAll => PieceLook::Hidden,
        StoreyLook::SeeThroughContent => PieceLook::SeeThrough,
        _ => PieceLook::Shown,
    }
}

/// What a plot of foundation takes, as the reference client works it out from its
/// width and depth in tiles: its storeys, and the most components and
/// fixtures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlotLimits {
    pub storeys: u8,
    pub components: u32,
    pub fixtures: u32,
}

pub fn plot_limits(width: i32, depth: i32) -> PlotLimits {
    let storeys = if width >= LARGE_PLOT || depth >= LARGE_PLOT {
        STOREYS_LARGE
    } else {
        STOREYS_SMALL
    };
    let floors = i32::from(storeys);
    let (plot_width, plot_depth) = (width + 1, depth + 1);
    let on_a_floor = (plot_width - 1) * (plot_depth - 1);
    let floor_share = (f64::from(floors * on_a_floor) * FLOOR_SHARE) as i32;
    let components = floors * (on_a_floor + 2 * (plot_width + plot_depth) - 4) - floor_share
        + 2 * plot_width
        + 3 * plot_depth
        - 5;
    let components = components.max(0);
    PlotLimits {
        storeys,
        components: components as u32,
        fixtures: (components / COMPONENTS_PER_FIXTURE) as u32,
    }
}

/// The parts of a design: its components, and its fixtures (doors and
/// teleporters).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DesignCounts {
    pub components: u32,
    pub fixtures: u32,
}

impl DesignCounts {
    /// What the design costs, in gold.
    pub fn cost(self) -> u32 {
        (self.components + self.fixtures) * COST_PER_PART
    }
}

/// The parts of the house the player designs, as the reference client counts them:
/// each piece the catalog holds, doors and teleporters as fixtures.
pub fn design_counts(frame: &WatchFrame) -> DesignCounts {
    let mut counts = DesignCounts::default();
    let Some(house) = designed_house(frame) else {
        return counts;
    };
    for tile in &house.tiles {
        match kind_of(&frame.house_parts, tile.graphic) {
            Some(HousePartKind::Door | HousePartKind::Teleporter) => counts.fixtures += 1,
            Some(_) => counts.components += 1,
            None => {}
        }
    }
    counts
}

/// The limits of the design, in the reference client's words.
pub fn limits_words(limits: PlotLimits) -> String {
    format!(
        "{WORDS_LIMITS}\nMax Components: {}\nMax Fixtures: {}\n{WORDS_FIXTURES_ARE}",
        limits.components, limits.fixtures
    )
}

/// The house the designer is open on, with its tiles.
fn designed_house(frame: &WatchFrame) -> Option<&uoterm_world::DesignedHouse> {
    let serial = frame.designing?.serial;
    frame
        .designed_houses
        .iter()
        .find(|house| house.serial.0 == serial)
}

/// The part the player builds with, whether he takes parts off or picks
/// one off the house, and how each storey shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseDesign {
    pub kind: HousePartKind,
    /// The style of the kind, and which of its pieces.
    pub style: usize,
    pub piece: usize,
    pub removing: bool,
    /// The eyedropper: the next click on the house picks the part it hits.
    pub picking: bool,
    /// How each storey shows, from the ground up.
    pub storeys: [StoreyLook; STOREYS],
}

impl Default for HouseDesign {
    fn default() -> Self {
        Self {
            kind: HousePartKind::Wall,
            style: 0,
            piece: 0,
            removing: false,
            picking: false,
            storeys: [StoreyLook::Normal; STOREYS],
        }
    }
}

impl HouseDesign {
    /// Another kind: its first style and piece.
    pub fn set_kind(&mut self, kind: HousePartKind) {
        self.kind = kind;
        self.style = 0;
        self.piece = 0;
    }

    /// Another style of the kind: its first piece.
    pub fn set_style(&mut self, style: usize) {
        self.style = style;
        self.piece = 0;
    }

    /// The part and the piece the player builds with, when the catalog has
    /// them.
    pub fn picked<'a>(&self, parts: &'a [HousePart]) -> Option<(&'a HousePart, u16)> {
        let styles = styles_of(parts, self.kind);
        let part = *styles.get(self.style)?;
        let piece = *part.pieces.get(self.piece)?;
        Some((part, piece))
    }

    /// Turns the look of one storey, from 0, to the next.
    pub fn turn_storey(&mut self, storey: usize) {
        if let Some(look) = self.storeys.get_mut(storey) {
            *look = look.next();
        }
    }

    /// Goes to a level of the house: every storey shows again, as in
    /// the reference client. Gives the act that asks the shard.
    pub fn go_to_floor(&mut self, level: u8) -> Act {
        self.storeys = [StoreyLook::Normal; STOREYS];
        Act::HouseFloor(level)
    }

    /// Turns the eyedropper on or off. It and removing go one at a time.
    pub fn toggle_picking(&mut self) {
        self.picking = !self.picking;
        self.removing = false;
    }

    /// Turns removing on or off. It and the eyedropper go one at a time.
    pub fn toggle_removing(&mut self) {
        self.removing = !self.removing;
        self.picking = false;
    }

    /// Picks the part of the house at a tile, the piece nearest to height
    /// `z`, as the reference client's eyedropper does. False when no part the catalog
    /// holds stands there.
    fn pick_from_house(&mut self, frame: &WatchFrame, x: i32, y: i32, z: i32) -> bool {
        let Some(house) = designed_house(frame) else {
            return false;
        };
        let Some(foundation) = frame.multis.iter().find(|m| m.serial == house.serial.0) else {
            return false;
        };
        let (dx, dy) = (x - i32::from(foundation.x), y - i32::from(foundation.y));
        let dz = z - i32::from(foundation.z);
        let parts = &frame.house_parts;
        let picked = house
            .tiles
            .iter()
            .filter(|tile| tile.dx == dx && tile.dy == dy)
            .filter_map(|tile| {
                let place = parts
                    .iter()
                    .position(|part| part.pieces.contains(&tile.graphic))?;
                let piece = parts[place]
                    .pieces
                    .iter()
                    .position(|p| *p == tile.graphic)?;
                Some(((tile.dz - dz).abs(), place, piece))
            })
            .min_by_key(|(distance, ..)| *distance);
        let Some((_, place, piece)) = picked else {
            return false;
        };
        self.pick_part(parts, place);
        self.piece = piece;
        true
    }

    /// Picks the part at this place of the catalog, with its first piece.
    pub fn pick_part(&mut self, parts: &[HousePart], place: usize) {
        let Some(part) = parts.get(place) else {
            return;
        };
        self.kind = part.kind;
        self.style = styles_of(parts, part.kind)
            .iter()
            .position(|known| known.name == part.name)
            .unwrap_or(0);
        self.piece = 0;
    }

    /// What a click on the house does: build the picked part, or take one
    /// off; with the eyedropper, pick the part it hits and send nothing.
    /// None when the designer is shut.
    pub fn click_on_house(&mut self, frame: &WatchFrame, x: i32, y: i32, z: i32) -> Option<Act> {
        frame.designing?;
        if self.picking {
            if self.pick_from_house(frame, x, y, z) {
                self.picking = false;
            }
            return None;
        }
        let (part, graphic) = self.picked(&frame.house_parts)?;
        let action = if self.removing {
            remove_action_for(part.kind)
        } else {
            action_for(part.kind)
        };
        Some(Act::HouseEdit {
            action,
            graphic,
            x,
            y,
            z,
        })
    }

    /// The words beside the mouse while the designer is open.
    pub fn hint(&self) -> &'static str {
        if self.picking {
            HINT_PICK
        } else if self.removing {
            HINT_REMOVE
        } else {
            HINT_BUILD
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Vec<HousePart> {
        vec![
            HousePart {
                kind: HousePartKind::Wall,
                name: "Dark Wood".into(),
                pieces: vec![10, 7, 12],
            },
            HousePart {
                kind: HousePartKind::Wall,
                name: "Stone".into(),
                pieces: vec![20, 22],
            },
            HousePart {
                kind: HousePartKind::Roof,
                name: "Tile Roof".into(),
                pieces: vec![11314],
            },
        ]
    }

    fn designing() -> WatchFrame {
        WatchFrame {
            designing: Some(crate::view::WatchDesigning {
                serial: 70,
                floor: 1,
                ..crate::view::WatchDesigning::default()
            }),
            house_parts: catalog(),
            ..WatchFrame::default()
        }
    }

    /// A designed house at 100, 200 with a stone wall and a door on the
    /// first storey, a floor tile on the second, and a foundation piece
    /// the catalog does not hold.
    fn with_house() -> WatchFrame {
        const FOUNDATION: u16 = 0x0496;
        const DOOR: u16 = 0x0675;
        const FLOOR: u16 = 0x0519;
        let mut frame = designing();
        frame.house_parts.push(HousePart {
            kind: HousePartKind::Door,
            name: "Wood Door".into(),
            pieces: vec![DOOR],
        });
        frame.house_parts.push(HousePart {
            kind: HousePartKind::Floor,
            name: "Plank".into(),
            pieces: vec![FLOOR],
        });
        frame.multis.push(crate::view::WatchMulti {
            serial: 70,
            multi_id: 0x13EC,
            x: 100,
            y: 200,
            z: 0,
        });
        let tile = |graphic, dx, dy, dz| uoterm_world::HouseTile {
            graphic,
            dx,
            dy,
            dz,
        };
        frame.designed_houses.push(uoterm_world::DesignedHouse {
            serial: uoterm_protocol::types::Serial(70),
            revision: 1,
            tiles: vec![
                tile(22, 1, 1, 7),
                tile(DOOR, 2, 1, 7),
                tile(FLOOR, 1, 1, 27),
                tile(FOUNDATION, 0, 0, 0),
            ],
        });
        frame
    }

    #[test]
    fn a_plot_takes_the_storeys_and_parts_classicuo_works_out() {
        assert_eq!(
            plot_limits(7, 7),
            PlotLimits {
                storeys: 3,
                components: 302,
                fixtures: 15,
            }
        );
        assert_eq!(plot_limits(13, 7).storeys, 4);
        assert_eq!(plot_limits(7, 13).storeys, 4);
        assert!(limits_words(plot_limits(7, 7)).contains("Max Components: 302"));
    }

    #[test]
    fn the_parts_of_the_design_count_as_components_and_fixtures() {
        let counts = design_counts(&with_house());
        assert_eq!(
            counts,
            DesignCounts {
                components: 2,
                fixtures: 1,
            }
        );
        assert_eq!(counts.cost(), 3 * COST_PER_PART);
        assert_eq!(design_counts(&designing()), DesignCounts::default());
    }

    #[test]
    fn each_storey_turns_through_its_looks_and_hides_its_pieces() {
        assert_eq!(storey_of(7), Some(0));
        assert_eq!(storey_of(26), Some(0));
        assert_eq!(storey_of(27), Some(1));
        assert_eq!(storey_of(86), Some(3));
        assert_eq!(storey_of(87), None);
        assert_eq!(storey_of(0), None, "the foundation");
        let mut design = HouseDesign::default();
        for _ in 0..4 {
            design.turn_storey(1);
        }
        assert_eq!(design.storeys[1], StoreyLook::HideFloor);
        let wall = Some(HousePartKind::Wall);
        let floor = Some(HousePartKind::Floor);
        let looks = design.storeys;
        assert_eq!(piece_look(&looks, floor, 27), PieceLook::Hidden);
        assert_eq!(piece_look(&looks, wall, 27), PieceLook::Shown);
        assert_eq!(
            piece_look(&looks, floor, 7),
            PieceLook::Shown,
            "another storey"
        );
        let hide_walls = [StoreyLook::HideContent; STOREYS];
        assert_eq!(piece_look(&hide_walls, wall, 7), PieceLook::Hidden);
        assert_eq!(
            piece_look(&hide_walls, wall, 0),
            PieceLook::Shown,
            "off the storeys"
        );
        let all = [StoreyLook::HideAll; STOREYS];
        assert_eq!(piece_look(&all, floor, 7), PieceLook::Hidden);
        let see = [StoreyLook::SeeThroughContent; STOREYS];
        assert_eq!(piece_look(&see, wall, 7), PieceLook::SeeThrough);
        assert_eq!(StoreyLook::HideAll.next(), StoreyLook::Normal);
        assert_eq!(StoreyLook::TranslucentFloor.button_look(), 1);
        assert_eq!(design.go_to_floor(2), Act::HouseFloor(2));
        assert_eq!(
            design.storeys,
            [StoreyLook::Normal; STOREYS],
            "all show again"
        );
    }

    #[test]
    fn the_eyedropper_picks_the_part_it_hits_and_sends_nothing() {
        let frame = with_house();
        let mut design = HouseDesign::default();
        design.toggle_removing();
        design.toggle_picking();
        assert!(design.picking && !design.removing, "one tool at a time");
        assert_eq!(design.hint(), HINT_PICK);
        assert_eq!(design.click_on_house(&frame, 101, 201, 7), None);
        assert!(!design.picking, "one pick, then it builds again");
        assert_eq!(
            (design.kind, design.style, design.piece),
            (HousePartKind::Wall, 1, 1),
            "the stone wall, second piece"
        );
        design.toggle_picking();
        assert_eq!(design.click_on_house(&frame, 101, 201, 27), None);
        assert_eq!(
            design.kind,
            HousePartKind::Floor,
            "the piece nearest the height"
        );
        design.toggle_picking();
        assert_eq!(design.click_on_house(&frame, 150, 250, 7), None);
        assert!(design.picking, "nothing there, so it still waits");
    }

    #[test]
    fn a_click_builds_the_picked_piece_of_the_picked_style() {
        let mut design = HouseDesign {
            style: 1,
            piece: 1,
            ..HouseDesign::default()
        };
        let frame = designing();
        assert_eq!(
            design.click_on_house(&frame, 3, 4, 0),
            Some(Act::HouseEdit {
                action: ACTION_ADD,
                graphic: 22,
                x: 3,
                y: 4,
                z: 0,
            })
        );
        let mut roof = HouseDesign {
            kind: HousePartKind::Roof,
            removing: true,
            ..HouseDesign::default()
        };
        assert_eq!(
            roof.click_on_house(&frame, 1, 1, 20),
            Some(Act::HouseEdit {
                action: ACTION_REMOVE_ROOF,
                graphic: 11314,
                x: 1,
                y: 1,
                z: 20,
            })
        );
        assert_eq!(roof.hint(), HINT_REMOVE);
    }

    #[test]
    fn a_click_does_nothing_while_the_designer_is_shut() {
        let frame = WatchFrame {
            house_parts: catalog(),
            ..WatchFrame::default()
        };
        assert_eq!(HouseDesign::default().click_on_house(&frame, 3, 4, 0), None);
    }

    #[test]
    fn a_picked_part_takes_its_kind_and_style_and_a_new_kind_starts_over() {
        let mut design = HouseDesign::default();
        design.pick_part(&catalog(), 1);
        assert_eq!(
            (design.kind, design.style, design.piece),
            (HousePartKind::Wall, 1, 0)
        );
        design.piece = 1;
        design.set_kind(HousePartKind::Roof);
        assert_eq!((design.style, design.piece), (0, 0));
        assert_eq!(part_words(&catalog()[2]), "roof: Tile Roof");
        assert_eq!(action_for(HousePartKind::Stair), "stair");
    }
}
