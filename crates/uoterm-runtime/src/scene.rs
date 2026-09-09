//! What a character can see around itself, in plain words.
//!
//! The world model holds only what the server sends: mobiles, and items that
//! can move. Furniture, walls and floors live in the client map files and never
//! arrive as packets. An agent that reads only the world model is blind to the
//! room it stands in. This module joins both sources into one scene.

use serde::{Deserialize, Serialize};
use uoterm_nav::{StaticView, TileQuery, PERSON_HEIGHT};
use uoterm_protocol::{Direction, Point3};
use uoterm_world::World;

/// How far a described scene reaches. A UO client draws about this many tiles.
pub const SCENE_RADIUS: u16 = 8;
/// Things closer than this are reported as "here" instead of by direction.
pub const SCENE_HERE_DISTANCE: u32 = 0;
/// At most this many surrounding things reach the summary sentence.
pub const SCENE_SUMMARY_CAP: usize = 8;
/// A static this far below the feet still counts as the surface underfoot.
pub const SURFACE_DROP: i8 = PERSON_HEIGHT;
/// A run of touching tiles with the same name is reported as one thing, so a
/// long wall reads as one wall instead of a dozen separate entries.
pub const SCENE_GROUP_REACH: i32 = 1;

/// Where a scene entry came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    /// Map furniture, walls and floors. Never sent by the server.
    Static,
    /// A movable item the server told us about.
    Item,
    /// A player or creature.
    Mobile,
}

impl SceneKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Item => "item",
            Self::Mobile => "mobile",
        }
    }
}

/// One thing the character can see.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneThing {
    pub what: String,
    pub kind: SceneKind,
    pub graphic: u16,
    pub serial: Option<String>,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub dx: i32,
    pub dy: i32,
    pub dist: u32,
    pub direction: String,
    pub blocking: bool,
    /// How many touching tiles this thing covers. A wall covers many; a stool
    /// covers one.
    pub tiles: usize,
}

/// Everything around the character at one moment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub facing: String,
    pub standing_on: String,
    pub here: Vec<SceneThing>,
    pub around: Vec<SceneThing>,
    pub summary: String,
}

/// Compass word for a step from the character towards `dx`, `dy`.
pub fn bearing(dx: i32, dy: i32) -> &'static str {
    if dx == 0 && dy == 0 {
        return "here";
    }
    let ns = match dy.signum() {
        -1 => "north",
        1 => "south",
        _ => "",
    };
    let ew = match dx.signum() {
        -1 => "west",
        1 => "east",
        _ => "",
    };
    match (ns, ew) {
        ("", ew) => ew,
        (ns, "") => ns,
        ("north", "west") => "north-west",
        ("north", "east") => "north-east",
        ("south", "west") => "south-west",
        ("south", "east") => "south-east",
        _ => "here",
    }
}

/// "1 tile west", "3 tiles north-east", or "here".
pub fn distance_phrase(dist: u32, dx: i32, dy: i32) -> String {
    if dist == SCENE_HERE_DISTANCE {
        return "here".into();
    }
    let unit = if dist == 1 { "tile" } else { "tiles" };
    format!("{dist} {unit} {}", bearing(dx, dy))
}

/// A static is on the character's floor when a person of normal height standing
/// at `feet_z` could touch it. Without this test the roof and the cellar of the
/// same building both appear in the scene.
pub fn on_same_floor(feet_z: i8, thing_z: i8) -> bool {
    let dz = i32::from(thing_z) - i32::from(feet_z);
    dz >= -i32::from(PERSON_HEIGHT) && dz <= i32::from(PERSON_HEIGHT)
}

/// The surface a character standing at `feet_z` has under its feet.
pub fn surface_under(feet_z: i8, statics: &[StaticView], land_name: &str) -> String {
    let best = statics
        .iter()
        .filter(|s| s.surface() || s.bridge())
        .filter(|s| {
            let top = i32::from(s.z) + i32::from(s.height);
            top <= i32::from(feet_z) && top >= i32::from(feet_z) - i32::from(SURFACE_DROP)
        })
        .max_by_key(|s| i32::from(s.z) + i32::from(s.height));
    match best {
        Some(s) if !s.name.is_empty() => s.name.clone(),
        _ => land_name.to_string(),
    }
}

/// A word for a body id, so a scene can say "a human" rather than "body 400".
pub fn body_word(body: u16) -> &'static str {
    const BODY_MALE: u16 = 400;
    const BODY_FEMALE: u16 = 401;
    const BODY_MALE_ELF: u16 = 605;
    const BODY_FEMALE_ELF: u16 = 606;
    const BODY_MALE_GARGOYLE: u16 = 666;
    const BODY_FEMALE_GARGOYLE: u16 = 667;
    match body {
        BODY_MALE | BODY_FEMALE => "a human",
        BODY_MALE_ELF | BODY_FEMALE_ELF => "an elf",
        BODY_MALE_GARGOYLE | BODY_FEMALE_GARGOYLE => "a gargoyle",
        _ => "a creature",
    }
}

/// The top of the surface a character standing at `feet_z` walks on in one tile.
///
/// A tile can hold several stacked statics: stone footings, then floor boards,
/// then a stool. The floor is the highest surface at or below the character's
/// own feet. A surface that rises above the feet is furniture to be mentioned,
/// not ground to walk over, even though a person could climb onto it.
pub fn floor_top(feet_z: i8, statics: &[StaticView]) -> i32 {
    statics
        .iter()
        .filter(|s| (s.surface() || s.bridge()) && !s.impassable())
        .map(|s| i32::from(s.z) + i32::from(s.height))
        .filter(|top| *top <= i32::from(feet_z))
        .max()
        .unwrap_or(i32::from(feet_z))
}

/// Whether a person would mention this static.
///
/// A thing is seen when it starts at the floor or higher and rises above it.
/// A footing that starts below the floor is buried under it, even where the
/// footing is tall enough to poke through, and the floor itself is ground the
/// character walks over without remarking on it. What is left is furniture,
/// walls, trees and doors.
pub fn stands_above_floor(s: &StaticView, floor: i32) -> bool {
    let base = i32::from(s.z);
    let top = base + i32::from(s.height);
    base >= floor && top > floor
}

fn thing_from_static(at: Point3, x: u16, y: u16, s: &StaticView) -> SceneThing {
    let dx = i32::from(x) - i32::from(at.x);
    let dy = i32::from(y) - i32::from(at.y);
    SceneThing {
        what: if s.name.is_empty() {
            format!("object {:#06x}", s.graphic)
        } else {
            s.name.clone()
        },
        kind: SceneKind::Static,
        graphic: s.graphic,
        serial: None,
        x,
        y,
        z: s.z,
        dx,
        dy,
        dist: at.chebyshev(Point3::new(x, y, s.z)),
        direction: bearing(dx, dy).into(),
        blocking: s.impassable(),
        tiles: 1,
    }
}

/// Join touching tiles that hold the same thing into one entry.
///
/// The map stores a wall as one static per tile. Reported one by one, a single
/// wall fills the whole scene. Grouping keeps the nearest tile of each run and
/// counts how many tiles it spans.
fn group_runs(mut things: Vec<SceneThing>) -> Vec<SceneThing> {
    things.sort_by(|a, b| {
        a.what
            .cmp(&b.what)
            .then_with(|| a.dist.cmp(&b.dist))
            .then_with(|| a.x.cmp(&b.x))
            .then_with(|| a.y.cmp(&b.y))
    });
    let mut out: Vec<SceneThing> = Vec::new();
    let mut used = vec![false; things.len()];
    for start in 0..things.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut run = things[start].clone();
        let mut frontier = vec![start];
        while let Some(current) = frontier.pop() {
            for next in 0..things.len() {
                if used[next] || things[next].what != things[current].what {
                    continue;
                }
                let dx = i32::from(things[next].x) - i32::from(things[current].x);
                let dy = i32::from(things[next].y) - i32::from(things[current].y);
                if dx.abs() <= SCENE_GROUP_REACH && dy.abs() <= SCENE_GROUP_REACH {
                    used[next] = true;
                    run.tiles += 1;
                    if things[next].dist < run.dist {
                        let nearest = things[next].clone();
                        let tiles = run.tiles;
                        run = nearest;
                        run.tiles = tiles;
                    }
                    frontier.push(next);
                }
            }
        }
        out.push(run);
    }
    out
}

/// Build the scene around the character.
///
/// `map` supplies the static furniture; `world` supplies people and loose items.
pub fn look_around<M: TileQuery + ?Sized>(world: &World, map: &M, radius: u16) -> Scene {
    let s = &world.self_state;
    let at = s.location;
    let facing = Direction::from_byte(s.direction).name();

    let mut here = Vec::new();
    let mut around = Vec::new();

    let (min_x, max_x) = (at.x.saturating_sub(radius), at.x.saturating_add(radius));
    let (min_y, max_y) = (at.y.saturating_sub(radius), at.y.saturating_add(radius));
    let mut standing_on = String::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if !map.in_bounds(x, y) {
                continue;
            }
            let statics = map.statics_at(x, y);
            if x == at.x && y == at.y {
                standing_on = surface_under(at.z, &statics, &map.land_name(x, y));
            }
            let floor = floor_top(at.z, &statics);
            for st in statics
                .iter()
                .filter(|st| on_same_floor(at.z, st.z) && stands_above_floor(st, floor))
            {
                let thing = thing_from_static(at, x, y, st);
                if thing.dist == SCENE_HERE_DISTANCE {
                    here.push(thing);
                } else {
                    around.push(thing);
                }
            }
        }
    }

    for item in world.items.values().filter(|i| i.parent.is_none()) {
        let dist = at.chebyshev(item.location);
        if dist > u32::from(radius) || !on_same_floor(at.z, item.location.z) {
            continue;
        }
        let dx = i32::from(item.location.x) - i32::from(at.x);
        let dy = i32::from(item.location.y) - i32::from(at.y);
        let what = if item.name.is_empty() {
            map.statics_at(item.location.x, item.location.y)
                .iter()
                .find(|s| s.graphic == item.graphic)
                .map(|s| s.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("object {:#06x}", item.graphic))
        } else {
            item.name.clone()
        };
        let thing = SceneThing {
            what,
            kind: SceneKind::Item,
            graphic: item.graphic,
            serial: Some(item.serial.to_string()),
            x: item.location.x,
            y: item.location.y,
            z: item.location.z,
            dx,
            dy,
            dist,
            direction: bearing(dx, dy).into(),
            blocking: false,
            tiles: 1,
        };
        if dist == SCENE_HERE_DISTANCE {
            here.push(thing);
        } else {
            around.push(thing);
        }
    }

    for mob in world.mobiles.values() {
        if mob.serial == s.serial {
            continue;
        }
        let dist = at.chebyshev(mob.location);
        if dist > u32::from(radius) || !on_same_floor(at.z, mob.location.z) {
            continue;
        }
        let dx = i32::from(mob.location.x) - i32::from(at.x);
        let dy = i32::from(mob.location.y) - i32::from(at.y);
        let what = if mob.name.is_empty() {
            body_word(mob.body).to_string()
        } else {
            format!("{} ({})", mob.name, body_word(mob.body))
        };
        around.push(SceneThing {
            what,
            kind: SceneKind::Mobile,
            graphic: mob.body,
            serial: Some(mob.serial.to_string()),
            x: mob.location.x,
            y: mob.location.y,
            z: mob.location.z,
            dx,
            dy,
            dist,
            direction: bearing(dx, dy).into(),
            blocking: true,
            tiles: 1,
        });
    }

    here.sort_by_key(|t| t.z);
    let mut around = group_runs(around);
    around.sort_by(|a, b| a.dist.cmp(&b.dist).then_with(|| a.what.cmp(&b.what)));
    let summary = summarize(at, facing, &standing_on, &here, &around);

    Scene {
        x: at.x,
        y: at.y,
        z: at.z,
        facing: facing.into(),
        standing_on,
        here,
        around,
        summary,
    }
}

fn summarize(
    at: Point3,
    facing: &str,
    standing_on: &str,
    here: &[SceneThing],
    around: &[SceneThing],
) -> String {
    let ground = if standing_on.is_empty() {
        "open ground".to_string()
    } else {
        standing_on.to_string()
    };
    let mut out = format!(
        "You stand on {ground} at {},{},{} facing {facing}.",
        at.x, at.y, at.z
    );
    for thing in here.iter().filter(|t| t.what != ground) {
        out.push_str(&format!(" {} is on your tile.", thing.what));
    }
    let mut said: Vec<&str> = Vec::new();
    for thing in around {
        if said.len() >= SCENE_SUMMARY_CAP {
            break;
        }
        if said.contains(&thing.what.as_str()) {
            continue;
        }
        said.push(&thing.what);
        out.push_str(&format!(
            " {} is {}.",
            thing.what,
            distance_phrase(thing.dist, thing.dx, thing.dy)
        ));
    }
    if around.is_empty() && here.is_empty() {
        out.push_str(" Nothing else is within reach.");
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use uoterm_nav::client_data_dir_from_env;
    use uoterm_nav::{MockMap, TILE_IMPASSABLE, TILE_SURFACE};
    use uoterm_protocol::Serial;
    use uoterm_world::{Mobile, World};

    const FLOOR_HEIGHT: u8 = 0;

    fn floor(name: &str, z: i8) -> StaticView {
        StaticView {
            graphic: 1,
            name: name.into(),
            z,
            height: FLOOR_HEIGHT,
            flags: TILE_SURFACE,
        }
    }

    #[test]
    fn bearing_names_all_eight_directions() {
        assert_eq!(bearing(0, -1), "north");
        assert_eq!(bearing(0, 1), "south");
        assert_eq!(bearing(1, 0), "east");
        assert_eq!(bearing(-1, 0), "west");
        assert_eq!(bearing(-1, -1), "north-west");
        assert_eq!(bearing(1, -1), "north-east");
        assert_eq!(bearing(-1, 1), "south-west");
        assert_eq!(bearing(1, 1), "south-east");
        assert_eq!(bearing(0, 0), "here");
    }

    #[test]
    fn distance_phrase_uses_singular_for_one_tile() {
        assert_eq!(distance_phrase(1, -1, 0), "1 tile west");
        assert_eq!(distance_phrase(3, 0, 1), "3 tiles south");
        assert_eq!(distance_phrase(0, 0, 0), "here");
    }

    #[test]
    fn same_floor_excludes_the_roof_and_the_cellar() {
        let feet = 27;
        assert!(on_same_floor(feet, 27));
        assert!(on_same_floor(feet, 27 + PERSON_HEIGHT));
        assert!(!on_same_floor(feet, 47));
        assert!(!on_same_floor(feet, 76));
        assert!(!on_same_floor(feet, 0));
    }

    #[test]
    fn surface_under_prefers_the_highest_static_below_the_feet() {
        let statics = vec![floor("stone", 22), floor("wooden boards", 27)];
        assert_eq!(surface_under(27, &statics, "grass"), "wooden boards");
        assert_eq!(surface_under(22, &statics, "grass"), "stone");
    }

    #[test]
    fn surface_under_falls_back_to_the_land_tile() {
        assert_eq!(surface_under(0, &[], "forest"), "forest");
    }

    #[test]
    fn body_word_names_the_player_races() {
        assert_eq!(body_word(400), "a human");
        assert_eq!(body_word(401), "a human");
        assert_eq!(body_word(605), "an elf");
        assert_eq!(body_word(666), "a gargoyle");
        assert_eq!(body_word(0x0011), "a creature");
    }

    const STOOL_HEIGHT: u8 = 4;
    const WALL_HEIGHT: u8 = 20;
    const FOOTING_HEIGHT: u8 = 10;
    const FLOOR_Z: i8 = 27;
    const FOOTING_Z: i8 = 22;

    fn furniture(name: &str, z: i8, height: u8, flags: u32) -> StaticView {
        StaticView {
            graphic: 2,
            name: name.into(),
            z,
            height,
            flags,
        }
    }

    #[test]
    fn floor_top_is_the_highest_surface_at_or_below_the_feet() {
        let tile = vec![
            furniture("stone", FOOTING_Z, FOOTING_HEIGHT, TILE_SURFACE),
            floor("wooden boards", FLOOR_Z),
            furniture("stool", FLOOR_Z, STOOL_HEIGHT, TILE_SURFACE),
        ];
        assert_eq!(floor_top(FLOOR_Z, &tile), i32::from(FLOOR_Z));
    }

    #[test]
    fn floor_top_falls_back_to_the_feet_without_a_surface() {
        assert_eq!(floor_top(FLOOR_Z, &[]), i32::from(FLOOR_Z));
    }

    #[test]
    fn only_things_standing_on_the_floor_are_seen() {
        let floor_z = i32::from(FLOOR_Z);
        let boards = floor("wooden boards", FLOOR_Z);
        let footing = furniture("stone", FOOTING_Z, FOOTING_HEIGHT, TILE_SURFACE);
        let stool = furniture("stool", FLOOR_Z, STOOL_HEIGHT, TILE_SURFACE);
        let wall = furniture("plaster wall", FLOOR_Z, WALL_HEIGHT, TILE_IMPASSABLE);

        assert!(!stands_above_floor(&boards, floor_z), "the floor is ground");
        assert!(
            !stands_above_floor(&footing, floor_z),
            "a footing under the floor is buried, even when it is tall"
        );
        assert!(stands_above_floor(&stool, floor_z), "a stool is furniture");
        assert!(stands_above_floor(&wall, floor_z), "a wall is seen");
    }

    #[test]
    fn a_wall_run_is_reported_once_with_its_length() {
        let at = Point3::new(10, 10, 0);
        let wall = furniture("plaster wall", 0, WALL_HEIGHT, TILE_IMPASSABLE);
        let mut cells = Vec::new();
        for step in 0..5u16 {
            cells.push(thing_from_static(at, 12, 8 + step, &wall));
        }
        cells.push(thing_from_static(at, 7, 14, &wall));
        let grouped = group_runs(cells);
        assert_eq!(grouped.len(), 2, "one run plus one lone tile: {grouped:?}");
        let run = grouped.iter().max_by_key(|t| t.tiles).unwrap();
        assert_eq!(run.tiles, 5);
        assert_eq!(run.dist, 2, "the run reports its nearest tile");
    }

    #[test]
    fn scene_names_a_nearby_person_and_the_ground() {
        let mut world = World::default();
        world.self_state.serial = Serial(100);
        world.self_state.location = Point3::new(5, 5, 0);
        world.mobiles.insert(
            Serial(7),
            Mobile {
                serial: Serial(7),
                name: "Pikkon".into(),
                body: 400,
                hue: 0,
                location: Point3::new(4, 4, 0),
                direction: 0,
                running: false,
                notoriety: 1,
                flags: 0,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            },
        );
        let map = MockMap::new(16, 16);
        let scene = look_around(&world, &map, SCENE_RADIUS);
        assert!(scene
            .summary
            .contains("Pikkon (a human) is 1 tile north-west"));
        assert_eq!(scene.around.len(), 1);
        assert_eq!(scene.around[0].kind, SceneKind::Mobile);
    }

    #[test]
    fn scene_skips_the_character_itself() {
        let mut world = World::default();
        world.self_state.serial = Serial(100);
        world.self_state.location = Point3::new(5, 5, 0);
        world.mobiles.insert(
            Serial(100),
            Mobile {
                serial: Serial(100),
                name: "Mara".into(),
                body: 401,
                hue: 0,
                location: Point3::new(5, 5, 0),
                direction: 0,
                running: false,
                notoriety: 1,
                flags: 0,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            },
        );
        let map = MockMap::new(16, 16);
        let scene = look_around(&world, &map, SCENE_RADIUS);
        assert!(scene.around.is_empty());
        assert!(scene.summary.contains("Nothing else is within reach"));
    }

    /// Trammel, the facet the New Haven inn stands on.
    const TRAMMEL: u8 = 1;
    /// A table in the common room on the ground floor of the inn.
    const INN_TABLE_ROOM: Point3 = Point3 {
        x: 3504,
        y: 2516,
        z: 27,
    };

    /// The scene of a character who stands at `at` with nobody else about, so
    /// everything reported comes from the map.
    fn scene_at(map: &dyn TileQuery, at: Point3) -> Scene {
        let mut world = World::default();
        world.self_state.serial = Serial(0x8C55);
        world.self_state.location = at;
        look_around(&world, map, SCENE_RADIUS)
    }

    #[test]
    fn the_new_haven_inn_reads_like_a_room() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = uoterm_nav::MulMap::open(&dir, TRAMMEL).expect("open trammel");
        let scene = scene_at(&map, INN_TABLE_ROOM);

        assert_eq!(
            scene.standing_on, "wooden boards",
            "the inn has a plank floor: {}",
            scene.summary
        );
        for expected in ["stool", "table", "stone wall"] {
            assert!(
                scene.around.iter().any(|t| t.what == expected),
                "the inn should hold a {expected}: {}",
                scene.summary
            );
        }
        assert!(
            !scene.around.iter().any(|t| t.what == "stone"),
            "the footing under the floor boards is buried and must not be seen: {}",
            scene.summary
        );
        assert!(
            !scene.around.iter().any(|t| t.what == "wooden boards"),
            "the floor is ground, not an object: {}",
            scene.summary
        );
        let wall = scene
            .around
            .iter()
            .filter(|t| t.what == "stone wall")
            .max_by_key(|t| t.tiles)
            .expect("a stone wall");
        assert!(
            wall.tiles > 1,
            "a wall spans several tiles and must be reported once: {wall:?}"
        );
    }

    /// A tile of the inn that carries both floors: the common room below and a
    /// bedroom above. Asked with no height, `tile` answers the floor above.
    const INN_TWO_FLOOR_TILE: Point3 = Point3 {
        x: 3506,
        y: 2526,
        z: 27,
    };
    const INN_UPPER_Z: i8 = 47;
    /// Furniture that stands on one floor only, measured from the client files.
    const GROUND_ONLY_THING: &str = "bench";
    const UPPER_ONLY_THING: &str = "bed";

    /// `look_around` asks the map three questions, and none of them can answer
    /// for the wrong floor.
    ///
    /// `statics_at` takes no height and picks no floor: it reports every static
    /// on the tile with its own height, so the scene, not the map, chooses the
    /// floor, and it chooses with `on_same_floor` against the character's own
    /// height. `land_name` reads the land cell, and a cell holds one land tile
    /// whatever height asks; the storeys of a building are statics. `in_bounds`
    /// compares against the width and the height of the map alone.
    #[test]
    fn the_scene_reads_one_floor_although_the_map_reports_them_all() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = uoterm_nav::MulMap::open(&dir, TRAMMEL).expect("open trammel");
        let (x, y) = (INN_TWO_FLOOR_TILE.x, INN_TWO_FLOOR_TILE.y);

        let statics = map.statics_at(x, y);
        for floor in [INN_TWO_FLOOR_TILE.z, INN_UPPER_Z] {
            assert!(
                statics.iter().any(|s| s.z == floor),
                "statics_at must report the floor at z {floor}: {statics:?}"
            );
        }
        assert!(
            !map.land_name(x, y).is_empty(),
            "the land cell must have a name"
        );
        for floor in [INN_TWO_FLOOR_TILE.z, INN_UPPER_Z] {
            assert_eq!(
                map.tile_from(floor, x, y).land_z,
                map.tile(x, y).land_z,
                "the land cell has one answer, whatever height asks"
            );
        }
        assert_eq!(
            map.tile(x, y).z,
            INN_UPPER_Z,
            "asked with no height this tile answers the floor above, or the test proves nothing"
        );

        let ground = scene_at(&map, INN_TWO_FLOOR_TILE);
        let upper = scene_at(&map, Point3::new(x, y, INN_UPPER_Z));
        for (scene, feet) in [(&ground, INN_TWO_FLOOR_TILE.z), (&upper, INN_UPPER_Z)] {
            for thing in scene.here.iter().chain(&scene.around) {
                assert!(
                    on_same_floor(feet, thing.z),
                    "a character at z {feet} must not see {thing:?}: {}",
                    scene.summary
                );
            }
        }
        assert!(
            ground.around.iter().any(|t| t.what == GROUND_ONLY_THING),
            "the common room holds a {GROUND_ONLY_THING}: {}",
            ground.summary
        );
        assert!(
            !ground.around.iter().any(|t| t.what == UPPER_ONLY_THING),
            "a {UPPER_ONLY_THING} stands upstairs and must not reach the ground floor: {}",
            ground.summary
        );
        assert!(
            upper.around.iter().any(|t| t.what == UPPER_ONLY_THING),
            "the bedroom above holds a {UPPER_ONLY_THING}: {}",
            upper.summary
        );
    }
}
