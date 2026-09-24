//! What the map holds, for an agent that cannot see it: the tiles of a kind
//! in an area, everything on one tile, the parts of the houses and boats in
//! view, the stairs and pads that lead into a dungeon, and whether one point
//! sees another. Every answer is read from the client files, on any map, and
//! not only near the character.

use std::collections::{HashMap, HashSet, VecDeque};

use uoterm_nav::{SightMode, StaticView, TileData, TileFlagSet};

use crate::tile_groups::{flag_from_name, flag_names, TileFacts, TileGroup};

use super::*;

const ARG_MAP: &str = "map";
const ARG_X: &str = "x";
const ARG_Y: &str = "y";
const ARG_Z: &str = "z";
const ARG_RADIUS: &str = "radius";
const ARG_X1: &str = "x1";
const ARG_Y1: &str = "y1";
const ARG_X2: &str = "x2";
const ARG_Y2: &str = "y2";
const ARG_GROUP: &str = "group";
const ARG_FLAGS: &str = "flags";
const ARG_NAME: &str = "name";
const ARG_LAYER: &str = "layer";
const ARG_Z_MIN: &str = "z_min";
const ARG_Z_MAX: &str = "z_max";
const ARG_PAGE: &str = "page";
const ARG_PAGE_SIZE: &str = "page_size";
const ARG_FROM: &str = "from";
const ARG_FROM_X: &str = "from_x";
const ARG_FROM_Y: &str = "from_y";
const ARG_FROM_Z: &str = "from_z";
const ARG_MODE: &str = "mode";
const ARG_TRACE: &str = "trace";

const LAYER_LAND: &str = "land";
const LAYER_STATICS: &str = "statics";
const LAYER_BOTH: &str = "both";

/// How far round a spot a search looks when the caller names no radius, and
/// the widest radius or side it may ask for.
const RADIUS_DEFAULT: u16 = 12;
const RADIUS_MAX: u16 = 64;
const SIDE_MAX: u32 = 2 * RADIUS_MAX as u32 + 1;
/// Results come back a page at a time, nearest first.
pub(super) const PAGE_SIZE_DEFAULT: usize = 50;
pub(super) const PAGE_SIZE_MAX: usize = 200;
const FIRST_PAGE: usize = 1;
/// How far round the character an entrance scan looks by default.
const ENTRANCE_RADIUS_DEFAULT: u16 = 32;
/// The words in the tiledata name of a static that leads down: a stair or a
/// ladder.
const ENTRANCE_WORDS: [&str; 2] = ["stair", "ladder"];
const KIND_PAD: &str = "teleporter";
const KIND_LANDMARK: &str = "landmark";
/// The landmark kinds an entrance scan reports.
const ENTRANCE_LANDMARK_KINDS: [&str; 2] = ["dungeon", "cave"];

const NO_MAP_FILES: &str =
    "that map is not open: the session has no client files for it (set uopath)";
const NEEDS_A_FILTER: &str = "find_tiles needs group, graphics, flags or name";
const BAD_LAYER: &str = "layer must be land, statics or both";
const AREA_TOO_WIDE: &str = "the area is wider than 129 tiles on a side; ask for a smaller one";
const MAP_TILE_NEEDS_SPOT: &str = "map_tile needs x and y";
const NEEDS_MULTI: &str = "multi_parts needs serial (a house or boat in view), or x and y";
const NOT_A_MULTI: &str = "that serial is no house or boat in view";
const NO_MULTI_FILES: &str = "the client multi files are not open";
const BAD_SIGHT_MODE: &str = "mode must be runuo, modernuo, servuo, pol or sphere";

/// What the session keeps for the map questions: the whole tiledata file,
/// read once when first needed.
#[derive(Default)]
pub(super) struct Terrain {
    tiledata: Option<Arc<TileData>>,
    tried: bool,
}

/// The tiledata of the client files, read the first time it is asked for.
/// None without client files.
pub(super) fn tiledata(inner: &mut Inner) -> Option<Arc<TileData>> {
    if !inner.terrain.tried {
        inner.terrain.tried = true;
        inner.terrain.tiledata = inner.uopath.as_deref().and_then(|path| {
            TileData::open(path)
                .map_err(|e| tracing::warn!(error = %e, "failed to read the client tiledata"))
                .ok()
                .map(Arc::new)
        });
    }
    inner.terrain.tiledata.clone()
}

/// The map a call names, or the one underfoot.
fn asked_map(inner: &Inner, args: &Value) -> u8 {
    arg_number(args, ARG_MAP)
        .and_then(|m| u8::try_from(m).ok())
        .unwrap_or_else(|| inner.map_index())
}

/// Runs `read` on the files of map `idx`: the client facet, or the empty grid
/// a session walks before its files are open, which stands only for the map
/// underfoot.
fn on_map<R>(inner: &mut Inner, idx: u8, read: impl FnOnce(&dyn TileQuery) -> R) -> Option<R> {
    match inner.facet_of(idx) {
        Some(facet) => Some(read(facet.as_ref())),
        None if idx == inner.map_index() => Some(read(&inner.map)),
        None => None,
    }
}

/// The full flags of a land tile and its name.
fn land_facts(
    tiles: Option<&TileData>,
    map: &dyn TileQuery,
    x: u16,
    y: u16,
) -> (u16, String, TileFlagSet, i8) {
    let column = map.column(x, y);
    let (name, flags) = match tiles.and_then(|t| t.land(column.land_id)) {
        Some(land) => (land.name.clone(), land.flags),
        None => (
            map.land_name(x, y),
            TileFlagSet(u64::from(column.land_flags)),
        ),
    };
    (column.land_id, name, flags, column.land.z())
}

/// The full flags of an item graphic: from the whole tiledata when it is
/// read, else the low half the map files keep.
fn item_flags(tiles: Option<&TileData>, graphic: u16, low: u32) -> TileFlagSet {
    tiles
        .and_then(|t| t.item(graphic))
        .map_or(TileFlagSet(u64::from(low)), |item| item.flags)
}

/// The name, height and full flags of an item graphic.
fn item_facts(
    tiles: Option<&TileData>,
    map: &dyn TileQuery,
    graphic: u16,
) -> (String, u8, TileFlagSet) {
    match tiles.and_then(|t| t.item(graphic)) {
        Some(item) => (item.name.clone(), item.height, item.flags),
        None => {
            let (low, height) = map.item_stat(graphic).unwrap_or_default();
            (map.item_name(graphic), height, TileFlagSet(u64::from(low)))
        }
    }
}

/// The tiledata weight of an item a player cannot lift.
const WEIGHT_FIXED: u8 = 255;

/// Whether a player may lift an item, and whether it holds other items.
///
/// An item in a container or worn can be lifted. One on the ground can when
/// the shard marks it movable, or when its graphic has a weight a person can
/// lift. It is a container when the shard sent its contents, or when its
/// graphic is one by the tiledata.
pub(super) fn item_kind(
    tiles: Option<&TileData>,
    world: &World,
    item: &uoterm_world::Item,
) -> (bool, bool) {
    let record = tiles.and_then(|t| t.item(item.graphic));
    let movable = item.parent.is_some()
        || item.flags & ITEM_FLAG_MOVABLE != 0
        || record.is_some_and(|r| r.weight != WEIGHT_FIXED);
    let container = world.containers.contains_key(&item.serial)
        || record.is_some_and(|r| r.flags.contains(TileFlagSet::CONTAINER));
    (movable, container)
}

/// A guess at whether a mobile is a creature of the shard and not a player:
/// it has no human body, or nobody can harm it, as a vendor or a healer.
pub(super) fn looks_like_npc(mobile: &uoterm_world::Mobile) -> bool {
    !uoterm_assist::mobiles::is_humanoid(mobile.body) || mobile.notoriety == NOTO_INVULNERABLE
}

/// True when a height is inside the range a find asks for.
pub(super) fn in_z_range(args: &Value, z: i8) -> bool {
    let bound = |key: &str| args.get(key).and_then(Value::as_i64);
    bound(ARG_Z_MIN).is_none_or(|min| i64::from(z) >= min)
        && bound(ARG_Z_MAX).is_none_or(|max| i64::from(z) <= max)
}

/// The page of a list a call asks for: the rows it holds, and the page
/// numbers, from 1.
pub(super) fn one_page<T: Clone>(args: &Value, rows: &[T]) -> (Vec<T>, Value) {
    let size = arg_number(args, ARG_PAGE_SIZE)
        .map_or(PAGE_SIZE_DEFAULT, |n| n as usize)
        .clamp(1, PAGE_SIZE_MAX);
    let pages = rows.len().div_ceil(size).max(FIRST_PAGE);
    let page = arg_number(args, ARG_PAGE)
        .map_or(FIRST_PAGE, |n| n as usize)
        .clamp(FIRST_PAGE, pages);
    let shown = rows
        .iter()
        .skip((page - FIRST_PAGE) * size)
        .take(size)
        .cloned()
        .collect();
    (
        shown,
        json!({ "page": page, "pages": pages, "page_size": size, "total": rows.len() }),
    )
}

/// The tiles an area covers, and the spot distances are counted from.
struct Area {
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    center: (u16, u16),
}

impl Area {
    /// A rectangle `x1,y1`-`x2,y2`, or a square of `radius` round `x`,`y`
    /// (the character when none is named).
    fn asked(args: &Value, here: Point3) -> std::result::Result<Self, String> {
        let corner = |kx: &str, ky: &str| match (arg_number(args, kx), arg_number(args, ky)) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        };
        let to_u16 = |n: u32| u16::try_from(n).unwrap_or(u16::MAX);
        if let (Some((ax, ay)), Some((bx, by))) = (corner(ARG_X1, ARG_Y1), corner(ARG_X2, ARG_Y2)) {
            let (x0, x1) = (ax.min(bx), ax.max(bx));
            let (y0, y1) = (ay.min(by), ay.max(by));
            if x1 - x0 >= SIDE_MAX || y1 - y0 >= SIDE_MAX {
                return Err(AREA_TOO_WIDE.into());
            }
            let center = match corner(ARG_X, ARG_Y) {
                Some((x, y)) => (to_u16(x), to_u16(y)),
                None => (here.x, here.y),
            };
            return Ok(Self {
                x0: to_u16(x0),
                y0: to_u16(y0),
                x1: to_u16(x1),
                y1: to_u16(y1),
                center,
            });
        }
        let (cx, cy) =
            corner(ARG_X, ARG_Y).map_or((here.x, here.y), |(x, y)| (to_u16(x), to_u16(y)));
        let radius = arg_number(args, ARG_RADIUS)
            .map_or(RADIUS_DEFAULT, to_u16)
            .min(RADIUS_MAX);
        Ok(Self {
            x0: cx.saturating_sub(radius),
            y0: cy.saturating_sub(radius),
            x1: cx.saturating_add(radius),
            y1: cy.saturating_add(radius),
            center: (cx, cy),
        })
    }

    fn tiles(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        (self.y0..=self.y1).flat_map(move |y| (self.x0..=self.x1).map(move |x| (x, y)))
    }

    fn dist(&self, x: u16, y: u16) -> u32 {
        Point3::new(x, y, 0).chebyshev(Point3::new(self.center.0, self.center.1, 0))
    }
}

/// What a tile search matches.
struct TileFilter {
    group: Option<TileGroup>,
    graphics: Vec<u16>,
    flags: TileFlagSet,
    word: Option<String>,
    land: bool,
    statics: bool,
    z_min: Option<i8>,
    z_max: Option<i8>,
}

impl TileFilter {
    fn asked(args: &Value) -> std::result::Result<Self, String> {
        let group = match args.get(ARG_GROUP).and_then(Value::as_str) {
            None => None,
            Some(name) => Some(TileGroup::from_name(name).ok_or_else(|| {
                format!("group must be one of {}", TileGroup::names().join(", "))
            })?),
        };
        let graphics: Vec<u16> = args
            .get(ARG_GRAPHICS)
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|g| g.as_u64().and_then(|g| u16::try_from(g).ok()))
                    .collect()
            })
            .unwrap_or_default();
        let mut flags = TileFlagSet::NONE;
        for name in args
            .get(ARG_FLAGS)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            flags = flags
                | flag_from_name(name).ok_or_else(|| format!("no tile flag named '{name}'"))?;
        }
        let word = args
            .get(ARG_NAME)
            .and_then(Value::as_str)
            .map(|w| w.trim().to_ascii_lowercase())
            .filter(|w| !w.is_empty());
        if group.is_none() && graphics.is_empty() && flags == TileFlagSet::NONE && word.is_none() {
            return Err(NEEDS_A_FILTER.into());
        }
        let (land, statics) = match args.get(ARG_LAYER).and_then(Value::as_str) {
            None => (true, true),
            Some(layer) if layer.eq_ignore_ascii_case(LAYER_BOTH) => (true, true),
            Some(layer) if layer.eq_ignore_ascii_case(LAYER_LAND) => (true, false),
            Some(layer) if layer.eq_ignore_ascii_case(LAYER_STATICS) => (false, true),
            Some(_) => return Err(BAD_LAYER.into()),
        };
        let z = |key: &str| {
            args.get(key)
                .and_then(Value::as_i64)
                .map(|z| z.clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8)
        };
        Ok(Self {
            group,
            graphics,
            flags,
            word,
            land,
            statics,
            z_min: z(ARG_Z_MIN),
            z_max: z(ARG_Z_MAX),
        })
    }

    fn matches(&self, tile: &TileFacts<'_>, z: i8) -> bool {
        self.group.is_none_or(|group| group.holds(tile))
            && (self.graphics.is_empty() || self.graphics.contains(&tile.id))
            && tile.flags.contains(self.flags)
            && self.word.as_deref().is_none_or(|word| tile.named(word))
            && self.z_min.is_none_or(|min| z >= min)
            && self.z_max.is_none_or(|max| z <= max)
    }
}

/// One tile a search found.
#[derive(Clone)]
struct FoundTile {
    x: u16,
    y: u16,
    z: i8,
    land: bool,
    id: u16,
    name: String,
    flags: TileFlagSet,
    dist: u32,
}

impl FoundTile {
    fn json(&self) -> Value {
        json!({
            "x": self.x,
            "y": self.y,
            "z": self.z,
            "layer": if self.land { LAYER_LAND } else { LAYER_STATICS },
            "graphic": self.id,
            "name": self.name,
            "flags": flag_names(self.flags),
            "dist": self.dist,
        })
    }
}

/// `find_tiles`: the land and the statics of a kind in an area, on any map,
/// nearest first and a page at a time.
pub(super) fn find_tiles(inner: &mut Inner, args: &Value) -> ToolResult {
    let filter = match TileFilter::asked(args) {
        Ok(filter) => filter,
        Err(why) => return ToolResult::err(why),
    };
    let here = inner.world.read().self_state.location;
    let area = match Area::asked(args, here) {
        Ok(area) => area,
        Err(why) => return ToolResult::err(why),
    };
    let idx = asked_map(inner, args);
    let tiles = tiledata(inner);
    let found = on_map(inner, idx, |map| {
        let mut found = Vec::new();
        for (x, y) in area.tiles().filter(|(x, y)| map.in_bounds(*x, *y)) {
            let dist = area.dist(x, y);
            if filter.land {
                let (id, name, flags, z) = land_facts(tiles.as_deref(), map, x, y);
                let facts = TileFacts {
                    id,
                    land: true,
                    name: &name,
                    flags,
                };
                if filter.matches(&facts, z) {
                    found.push(FoundTile {
                        x,
                        y,
                        z,
                        land: true,
                        id,
                        name: name.clone(),
                        flags,
                        dist,
                    });
                }
            }
            if filter.statics {
                for s in map.statics_at(x, y) {
                    let flags = item_flags(tiles.as_deref(), s.graphic, s.flags);
                    let facts = TileFacts {
                        id: s.graphic,
                        land: false,
                        name: &s.name,
                        flags,
                    };
                    if filter.matches(&facts, s.z) {
                        found.push(FoundTile {
                            x,
                            y,
                            z: s.z,
                            land: false,
                            id: s.graphic,
                            name: s.name.clone(),
                            flags,
                            dist,
                        });
                    }
                }
            }
        }
        found
    });
    let Some(mut found) = found else {
        return ToolResult::err(NO_MAP_FILES);
    };
    found.sort_by_key(|t| (t.dist, t.y, t.x, t.z));
    let (shown, paging) = one_page(args, &found);
    let mut result = json!({
        "map": idx,
        "center": { "x": area.center.0, "y": area.center.1 },
        "tiles": shown.iter().map(FoundTile::json).collect::<Vec<_>>(),
    });
    merge(&mut result, paging);
    ToolResult::ok(result)
}

/// Puts every field of `extra` into `into`.
fn merge(into: &mut Value, extra: Value) {
    if let (Some(into), Value::Object(extra)) = (into.as_object_mut(), extra) {
        into.extend(extra);
    }
}

/// One part of a house or a boat, where it stands on the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MultiPart {
    pub multi: Serial,
    pub graphic: u16,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// A part of a house a player designed, not of the shape in the files.
    pub designed: bool,
}

/// Every part of one house or boat in view: the design the shard sent for
/// a house a player built, or else the shape of the client multi files.
pub(super) fn parts_of(inner: &Inner, building: &uoterm_world::MultiItem) -> Vec<MultiPart> {
    let at = building.location;
    let place = |graphic: u16, dx: i32, dy: i32, dz: i32, designed: bool| MultiPart {
        multi: building.serial,
        graphic,
        x: i32::from(at.x) + dx,
        y: i32::from(at.y) + dy,
        z: i32::from(at.z) + dz,
        designed,
    };
    if let Some(design) = inner.play.designed_house(building.serial) {
        return design
            .tiles
            .iter()
            .map(|t| place(t.graphic, t.dx, t.dy, t.dz, true))
            .collect();
    }
    inner
        .multi_shapes
        .as_deref()
        .map(|shapes| {
            shapes
                .pieces(building.multi_id)
                .iter()
                .map(|p| {
                    place(
                        p.graphic,
                        i32::from(p.dx),
                        i32::from(p.dy),
                        i32::from(p.dz),
                        false,
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The multi each map tile of a house or boat in view belongs to.
pub(super) fn multi_tiles(inner: &Inner) -> HashMap<(i32, i32), Serial> {
    let buildings: Vec<uoterm_world::MultiItem> =
        inner.world.read().multis.values().copied().collect();
    buildings
        .iter()
        .flat_map(|building| parts_of(inner, building))
        .map(|part| ((part.x, part.y), part.multi))
        .collect()
}

/// `multi_parts`: every part of one house or boat, or every part of any of
/// them on one tile, a page at a time.
pub(super) fn multi_parts(inner: &mut Inner, args: &Value) -> ToolResult {
    let wanted = arg_serial_opt(args, ARG_SERIAL).filter(|s| s.is_valid());
    let tile = match (arg_number(args, ARG_X), arg_number(args, ARG_Y)) {
        (Some(x), Some(y)) => Some((x as i32, y as i32)),
        _ => None,
    };
    if wanted.is_none() && tile.is_none() {
        return ToolResult::err(NEEDS_MULTI);
    }
    let buildings: Vec<uoterm_world::MultiItem> = {
        let world = inner.world.read();
        match wanted {
            Some(serial) => match world.multis.get(&serial) {
                Some(building) => vec![*building],
                None => return ToolResult::err(NOT_A_MULTI),
            },
            None => world.multis.values().copied().collect(),
        }
    };
    let parts: Vec<MultiPart> = buildings
        .iter()
        .flat_map(|building| parts_of(inner, building))
        .filter(|part| tile.is_none_or(|(x, y)| (part.x, part.y) == (x, y)))
        .collect();
    if parts.is_empty()
        && inner.multi_shapes.is_none()
        && buildings
            .iter()
            .any(|b| inner.play.designed_house(b.serial).is_none())
    {
        return ToolResult::err(NO_MULTI_FILES);
    }
    let tiles = tiledata(inner);
    let (shown, paging) = one_page(args, &parts);
    let rows: Vec<Value> = shown
        .iter()
        .map(|part| {
            let (name, height, flags) =
                item_facts(tiles.as_deref(), inner.map_files(), part.graphic);
            json!({
                "multi": part.multi,
                "graphic": part.graphic,
                "name": name,
                "x": part.x,
                "y": part.y,
                "z": part.z,
                "height": height,
                "flags": flag_names(flags),
                "designed": part.designed,
            })
        })
        .collect();
    let mut result = json!({
        "multis": buildings.iter().map(|b| json!({
            "serial": b.serial,
            "multi_id": b.multi_id,
            "location": b.location,
            "designed": inner.play.designed_house(b.serial).is_some(),
        })).collect::<Vec<_>>(),
        "parts": rows,
    });
    merge(&mut result, paging);
    ToolResult::ok(result)
}

/// `map_tile`: everything on one tile of any map: the land with its flags,
/// each static, the items and the parts of houses and boats on it, and
/// whether a person can stand there.
pub(super) fn map_tile(inner: &mut Inner, args: &Value) -> ToolResult {
    let (Some(x), Some(y)) = (arg_number(args, ARG_X), arg_number(args, ARG_Y)) else {
        return ToolResult::err(MAP_TILE_NEEDS_SPOT);
    };
    let (x, y) = (x as u16, y as u16);
    let standing = inner.world.read().self_state.location.z;
    let z = asked_z(args, standing);
    let idx = asked_map(inner, args);
    let here_map = idx == inner.map_index();
    let tiles = tiledata(inner);
    // On the map underfoot the houses, boats and items in view count too.
    let stand = if here_map {
        inner.ensure_facet();
        Some(inner.tiles().tile_from(z, x, y))
    } else {
        None
    };
    let read = on_map(inner, idx, |map| {
        let (land_id, land_name, land_flags, land_z) = land_facts(tiles.as_deref(), map, x, y);
        let statics: Vec<Value> = map
            .statics_at(x, y)
            .iter()
            .map(|s: &StaticView| {
                json!({
                    "graphic": s.graphic,
                    "name": s.name,
                    "z": s.z,
                    "height": s.height,
                    "hue": s.hue,
                    "flags": flag_names(item_flags(tiles.as_deref(), s.graphic, s.flags)),
                })
            })
            .collect();
        (
            stand.unwrap_or_else(|| map.tile_from(z, x, y)),
            json!({
                "id": land_id,
                "name": land_name,
                "z": land_z,
                "flags": flag_names(land_flags),
            }),
            statics,
        )
    });
    let Some((tile, land, statics)) = read else {
        return ToolResult::err(NO_MAP_FILES);
    };
    let (items, parts) = if here_map {
        let items: Vec<Value> = {
            let world = inner.world.read();
            world
                .items
                .values()
                .filter(|i| i.parent.is_none() && (i.location.x, i.location.y) == (x, y))
                .map(|i| {
                    json!({
                        "serial": i.serial,
                        "graphic": i.graphic,
                        "name": i.name,
                        "z": i.location.z,
                        "hue": i.hue,
                        "amount": i.amount,
                    })
                })
                .collect()
        };
        let buildings: Vec<uoterm_world::MultiItem> =
            inner.world.read().multis.values().copied().collect();
        let parts: Vec<Value> = buildings
            .iter()
            .flat_map(|b| parts_of(inner, b))
            .filter(|p| (p.x, p.y) == (i32::from(x), i32::from(y)))
            .map(|p| {
                let (name, height, flags) =
                    item_facts(tiles.as_deref(), inner.map_files(), p.graphic);
                json!({
                    "multi": p.multi,
                    "graphic": p.graphic,
                    "name": name,
                    "z": p.z,
                    "height": height,
                    "flags": flag_names(flags),
                })
            })
            .collect();
        (items, parts)
    } else {
        (Vec::new(), Vec::new())
    };
    ToolResult::ok(json!({
        "map": idx,
        "x": x,
        "y": y,
        "walkable": tile.walkable(),
        "door": tile.door,
        "z": tile.z,
        "land": land,
        "statics": statics,
        "items": items,
        "multi_parts": parts,
    }))
}

/// Where a line of sight starts or ends: a serial in view, or a spot. A
/// spot's height is the floor there when none is named.
fn sight_end(
    inner: &mut Inner,
    args: &Value,
    serial_key: &str,
    (kx, ky, kz): (&str, &str, &str),
) -> std::result::Result<Option<Point3>, String> {
    if let Some(serial) = arg_serial_opt(args, serial_key).filter(|s| s.is_valid()) {
        return sight_point(inner, serial)
            .map(Some)
            .ok_or_else(|| format!("{TOOL_LINE_OF_SIGHT}: {serial} is not in view"));
    }
    let (Some(x), Some(y)) = (arg_number(args, kx), arg_number(args, ky)) else {
        return Ok(None);
    };
    let (x, y) = (x as u16, y as u16);
    let standing = inner.world.read().self_state.location.z;
    let z = match args.get(kz).and_then(Value::as_i64) {
        Some(z) => z.clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8,
        None => surface_z(inner, standing, x, y),
    };
    Ok(Some(uoterm_nav::eyes_at(Point3::new(x, y, z))))
}

/// `line_of_sight`: whether one point sees another, from the character's
/// eyes or any point or object, by the rules of the shard's family, with
/// the line tile by tile when asked.
pub(super) fn line_of_sight(inner: &mut Inner, args: &Value) -> ToolResult {
    let mode = match args.get(ARG_MODE).and_then(Value::as_str) {
        Some(name) => match SightMode::from_name(name) {
            Some(mode) => mode,
            None => return ToolResult::err(BAD_SIGHT_MODE),
        },
        None => inner.agents.config.options.sight_mode,
    };
    let to = match sight_end(inner, args, ARG_SERIAL, (ARG_X, ARG_Y, ARG_Z)) {
        Ok(Some(to)) => to,
        Ok(None) => {
            return ToolResult::err(format!("{TOOL_LINE_OF_SIGHT} needs serial, or x and y"))
        }
        Err(why) => return ToolResult::err(why),
    };
    let from = match sight_end(inner, args, ARG_FROM, (ARG_FROM_X, ARG_FROM_Y, ARG_FROM_Z)) {
        Ok(Some(from)) => from,
        Ok(None) => uoterm_nav::eyes_at(inner.world.read().self_state.location),
        Err(why) => return ToolResult::err(why),
    };
    inner.ensure_facet();
    let trace = uoterm_nav::sight_trace(&inner.tiles(), from, to, mode);
    let mut result = json!({
        "in_sight": trace.in_sight,
        "from": from,
        "aim": to,
        "mode": mode,
    });
    if args.get(ARG_TRACE).and_then(Value::as_bool) == Some(true) {
        result["points"] = json!(trace.points);
    } else if let Some(stop) = trace.points.iter().find(|p| p.blocker.is_some()) {
        result["blocked_at"] = json!(stop);
    }
    ToolResult::ok(result)
}

/// One place a scan found that may lead into a dungeon.
#[derive(Clone)]
struct Entrance {
    kind: String,
    name: String,
    at: Point3,
    tiles: usize,
    dist: u32,
}

/// `find_entrances`: the stairs and ladders on the map round a spot, each
/// run of them once, with the teleporter pads the character has learned and
/// the dungeon landmarks there, nearest first.
pub(super) fn find_entrances(inner: &mut Inner, args: &Value) -> ToolResult {
    let here = inner.world.read().self_state.location;
    let mut asked = args.clone();
    if asked.get(ARG_RADIUS).is_none() {
        asked[ARG_RADIUS] = json!(ENTRANCE_RADIUS_DEFAULT);
    }
    let area = match Area::asked(&asked, here) {
        Ok(area) => area,
        Err(why) => return ToolResult::err(why),
    };
    let idx = asked_map(inner, args);
    let spots = on_map(inner, idx, |map| {
        let mut spots: HashMap<(u16, u16), (i8, String)> = HashMap::new();
        for (x, y) in area.tiles().filter(|(x, y)| map.in_bounds(*x, *y)) {
            if let Some(s) = map.statics_at(x, y).into_iter().find(|s| {
                let name = s.name.to_ascii_lowercase();
                ENTRANCE_WORDS.iter().any(|word| name.contains(word))
            }) {
                spots.insert((x, y), (s.z, s.name));
            }
        }
        spots
    });
    let Some(spots) = spots else {
        return ToolResult::err(NO_MAP_FILES);
    };
    let center = Point3::new(area.center.0, area.center.1, here.z);
    let mut found: Vec<Entrance> = runs_of(&spots)
        .into_iter()
        .filter_map(|run| {
            let (&(x, y), (z, name)) = run
                .iter()
                .map(|tile| (tile, &spots[tile]))
                .min_by_key(|(tile, _)| area.dist(tile.0, tile.1))?;
            let word = ENTRANCE_WORDS
                .iter()
                .find(|w| name.to_ascii_lowercase().contains(**w))
                .copied()
                .unwrap_or_default();
            Some(Entrance {
                kind: word.to_string(),
                name: name.clone(),
                at: Point3::new(x, y, *z),
                tiles: run.len(),
                dist: area.dist(x, y),
            })
        })
        .collect();
    for pad in inner.teleporters.on_map(idx) {
        let dist = area.dist(pad.from.x, pad.from.y);
        if dist <= area.dist(area.x0, area.y0) {
            found.push(Entrance {
                kind: KIND_PAD.into(),
                name: format!("to {}", pad.to),
                at: pad.from,
                tiles: 1,
                dist,
            });
        }
    }
    if let Some(marks) = inner.landmarks.clone() {
        for mark in marks.find(None, Some(idx)).into_iter().filter(|m| {
            let kind = m.kind.to_ascii_lowercase();
            ENTRANCE_LANDMARK_KINDS.iter().any(|k| kind.contains(k))
        }) {
            let dist = center.chebyshev(mark.at);
            if dist <= area.dist(area.x0, area.y0) {
                found.push(Entrance {
                    kind: KIND_LANDMARK.into(),
                    name: mark.name.clone(),
                    at: mark.at,
                    tiles: 1,
                    dist,
                });
            }
        }
    }
    found.sort_by_key(|e| e.dist);
    let (shown, paging) = one_page(args, &found);
    let mut result = json!({
        "map": idx,
        "entrances": shown.iter().map(|e| json!({
            "kind": e.kind,
            "name": e.name,
            "location": e.at,
            "tiles": e.tiles,
            "dist": e.dist,
        })).collect::<Vec<_>>(),
    });
    merge(&mut result, paging);
    ToolResult::ok(result)
}

/// The runs of touching tiles in a set: tiles side by side or corner to
/// corner are one run, as the steps of one stair are.
fn runs_of<V>(tiles: &HashMap<(u16, u16), V>) -> Vec<Vec<(u16, u16)>> {
    let mut left: HashSet<(u16, u16)> = tiles.keys().copied().collect();
    let mut runs = Vec::new();
    let mut ordered: Vec<(u16, u16)> = left.iter().copied().collect();
    ordered.sort_unstable();
    for start in ordered {
        if !left.remove(&start) {
            continue;
        }
        let mut run = vec![start];
        let mut queue = VecDeque::from([start]);
        while let Some((x, y)) = queue.pop_front() {
            for dir in Direction::ALL {
                let (dx, dy) = dir.delta();
                let (Ok(nx), Ok(ny)) = (
                    u16::try_from(i32::from(x) + dx),
                    u16::try_from(i32::from(y) + dy),
                ) else {
                    continue;
                };
                if left.remove(&(nx, ny)) {
                    run.push((nx, ny));
                    queue.push_back((nx, ny));
                }
            }
        }
        runs.push(run);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;

    const HERE: Point3 = Point3 {
        x: 100,
        y: 100,
        z: 0,
    };

    fn standing_at(at: Point3) -> Inner {
        let inner = test_session();
        inner.world.write().self_state.location = at;
        inner
    }

    #[test]
    fn water_is_found_nearest_first_a_page_at_a_time() {
        let mut inner = standing_at(HERE);
        for (x, y) in [(103, 100), (101, 100), (110, 100)] {
            inner.map.set_wet(x, y, true);
        }
        let found = find_tiles(&mut inner, &json!({ "group": "water", "page_size": 2 }));
        assert!(found.ok, "{found:?}");
        let tiles = found.result["tiles"].as_array().unwrap();
        assert_eq!(found.result["total"], json!(3));
        assert_eq!(found.result["pages"], json!(2));
        assert_eq!(tiles[0]["x"], json!(101));
        assert_eq!(tiles[1]["x"], json!(103));
        assert!(tiles[0]["flags"]
            .as_array()
            .unwrap()
            .contains(&json!("wet")));
        let second = find_tiles(
            &mut inner,
            &json!({ "group": "water", "page_size": 2, "page": 2 }),
        );
        assert_eq!(second.result["tiles"][0]["x"], json!(110));
    }

    #[test]
    fn a_tile_search_reads_a_rectangle_flags_and_heights() {
        let mut inner = standing_at(HERE);
        inner.map.set_block(50, 50, true);
        inner.map.set_block(52, 51, true);
        inner.map.set_z(52, 51, 20);
        let args = json!({
            "x1": 48, "y1": 48, "x2": 55, "y2": 55,
            "flags": ["impassable"], "layer": "land", "z_min": 10,
        });
        let found = find_tiles(&mut inner, &args);
        assert!(found.ok, "{found:?}");
        assert_eq!(found.result["total"], json!(1));
        assert_eq!(found.result["tiles"][0]["x"], json!(52));
        assert!(!find_tiles(&mut inner, &json!({})).ok);
        assert!(!find_tiles(&mut inner, &json!({ "group": "castle" })).ok);
        let wide = json!({ "flags": ["wet"], "x1": 0, "y1": 0, "x2": 500, "y2": 10 });
        assert!(!find_tiles(&mut inner, &wide).ok);
        assert!(!find_tiles(&mut inner, &json!({ "flags": ["wet"], "map": 3 })).ok);
    }

    #[test]
    fn a_tile_names_its_land_flags_and_what_stands_on_it() {
        let mut inner = standing_at(HERE);
        inner.map.set_wet(HERE.x + 1, HERE.y, true);
        inner.map.set_block(HERE.x + 1, HERE.y, true);
        let tile = map_tile(&mut inner, &json!({ "x": HERE.x + 1, "y": HERE.y }));
        assert!(tile.ok, "{tile:?}");
        assert_eq!(tile.result["walkable"], json!(false));
        let flags = tile.result["land"]["flags"].as_array().unwrap();
        assert!(flags.contains(&json!("wet")) && flags.contains(&json!("impassable")));
        assert!(tile.result["statics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_line_of_sight_goes_from_any_point_with_its_trace() {
        let mut inner = standing_at(HERE);
        let args = json!({
            "from_x": 10, "from_y": 10, "from_z": 0,
            "x": 14, "y": 10, "z": 0, "trace": true, "mode": "sphere",
        });
        let seen = line_of_sight(&mut inner, &args);
        assert!(seen.ok, "{seen:?}");
        assert_eq!(seen.result["in_sight"], json!(true));
        assert_eq!(seen.result["mode"], json!("sphere"));
        assert!(!seen.result["points"].as_array().unwrap().is_empty());
        assert!(!line_of_sight(&mut inner, &json!({ "x": 1, "y": 1, "mode": "uox" })).ok);
        assert!(!line_of_sight(&mut inner, &json!({})).ok);
    }

    #[test]
    fn a_multi_needs_a_serial_or_a_tile() {
        let mut inner = standing_at(HERE);
        assert!(!multi_parts(&mut inner, &json!({})).ok);
        assert!(!multi_parts(&mut inner, &json!({ "serial": 0x4000_0100u32 })).ok);
    }

    #[test]
    fn the_parts_of_a_designed_house_are_named_by_house_and_by_tile() {
        const HOUSE: Serial = Serial(0x4000_0D01);
        const WALL: u16 = 0x0064;
        const FLOOR: u16 = 0x0031;
        const MODE_WITH_PLACES: u8 = 0;
        let mut inner = standing_at(HERE);
        let foundation = Point3::new(HERE.x + 5, HERE.y, 0);
        inner.world.write().multis.insert(
            HOUSE,
            uoterm_world::MultiItem {
                serial: HOUSE,
                multi_id: 0x13EC,
                location: foundation,
            },
        );
        // Each tile of a mode 0 plane: its graphic, then its x, y and z
        // offsets.
        let mut data = Vec::new();
        for (graphic, dx, dy) in [(WALL, -1i8, 0i8), (FLOOR, 0, 0), (FLOOR, 1, 0)] {
            data.extend(graphic.to_be_bytes());
            data.extend([dx as u8, dy as u8, 0]);
        }
        let house = uoterm_protocol::CustomHouse {
            serial: HOUSE,
            revision: 1,
            planes: vec![uoterm_protocol::HousePlane {
                z_index: 0,
                mode: MODE_WITH_PLACES,
                data,
            }],
        };
        let bounds = uoterm_world::HouseBounds {
            min_x: -1,
            min_y: 0,
            max_x: 1,
            max_y: 0,
        };
        super::super::play::on_custom_house(&mut inner, &house, Some(bounds));
        let all = multi_parts(&mut inner, &json!({ "serial": HOUSE.0 }));
        assert!(all.ok, "{all:?}");
        assert_eq!(all.result["total"], json!(3));
        assert_eq!(all.result["parts"][0]["designed"], json!(true));
        let at = multi_parts(
            &mut inner,
            &json!({ "x": foundation.x - 1, "y": foundation.y }),
        );
        assert_eq!(at.result["total"], json!(1));
        assert_eq!(at.result["parts"][0]["graphic"], json!(WALL));
        let tile = map_tile(&mut inner, &json!({ "x": foundation.x, "y": foundation.y }));
        assert_eq!(tile.result["multi_parts"][0]["graphic"], json!(FLOOR));
        assert_eq!(
            multi_tiles(&inner).get(&(i32::from(foundation.x) + 1, i32::from(foundation.y))),
            Some(&HOUSE)
        );
    }

    #[test]
    fn touching_tiles_are_one_run() {
        let tiles: HashMap<(u16, u16), ()> = [(1, 1), (2, 2), (3, 2), (10, 10)]
            .into_iter()
            .map(|t| (t, ()))
            .collect();
        let mut sizes: Vec<usize> = runs_of(&tiles).iter().map(Vec::len).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![1, 3]);
    }

    #[test]
    fn a_page_past_the_end_is_the_last_page() {
        let rows: Vec<u32> = (0..5).collect();
        let (shown, paging) = one_page(&json!({ "page": 9, "page_size": 2 }), &rows);
        assert_eq!(shown, vec![4]);
        assert_eq!(paging["page"], json!(3));
    }
}
