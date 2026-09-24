//! The choices a caller makes for one walk: run or walk, how near the goal
//! is near enough, whether doors on the way are opened, the areas and the
//! creatures to keep away from, whether the roads are preferred, and whether
//! a goal no route reaches is an error. Every route of that walk is planned
//! with them, and each answer says how long the plan took and how much of
//! the map it searched.

use uoterm_nav::{pathfind_flat_with, pathfind_with, AvoidArea, PathError, RouteOptions};

use super::*;

const ARG_RUN: &str = "run";
const ARG_ACCURACY: &str = "accuracy";
const ARG_OPEN_DOORS: &str = "open_doors";
const ARG_AVOID: &str = "avoid";
const ARG_EXACT: &str = "exact";
const ARG_ROADS: &str = "roads";
const ARG_RADIUS: &str = "radius";
const ARG_R: &str = "r";

/// How far round a creature named in `avoid` the route keeps when the
/// caller names no radius.
const AVOID_RADIUS_DEFAULT: u16 = 3;
/// The most tiles a walk may stop short of its goal.
const ACCURACY_MAX: u16 = 18;
/// How many steps ahead a refused route is looked at for what blocks it, and
/// how many more steps past that a patch may rejoin it.
const PATCH_SCAN_STEPS: usize = 5;
const PATCH_ANCHOR_EXTRA: usize = 5;
/// The most nodes the search for a patch opens. A patch is a short detour;
/// anything longer is the work of a full plan.
const PATCH_MAX_NODES: usize = 4096;
const MICROS_PER_MILLI: f64 = 1000.0;

const NO_PATH: &str = "path failed";
const ARG_KIND: &str = "kind";
const ARG_CLOSEST: &str = "closest";
const ARG_MAP: &str = "map";
const ARG_DISTANCE: &str = "distance";
/// How many matches on other maps a landmark search names.
const ELSEWHERE_SHOWN: usize = 10;
const BAD_AVOID: &str =
    "avoid is a list of {x, y, radius} areas and {serial, radius} creatures or items";

/// The choices of the walk under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Trip {
    /// Fixed areas the route keeps out of.
    areas: Vec<AvoidArea>,
    /// Objects the route keeps away from, each with how far: they move, so
    /// each plan reads where they stand now.
    around: Vec<(Serial, u16)>,
    /// The walk ends this many steps from its goal, or on it at zero.
    pub arrive_within: u16,
    /// Doors on the way are opened, as the shard's rules allow.
    pub open_doors: bool,
    /// Grass and forest cost a little more, so the route keeps to roads.
    pub prefer_roads: bool,
    /// How the last route of the walk was planned.
    pub last: Option<RouteStats>,
}

impl Default for Trip {
    /// A walk the session starts on its own: to the tile itself, doors
    /// opened, and the straightest way, which a chase needs.
    fn default() -> Self {
        Self {
            areas: Vec::new(),
            around: Vec::new(),
            arrive_within: 0,
            open_doors: true,
            prefer_roads: false,
            last: None,
        }
    }
}

/// How one route was planned: the nodes the search opened, how long it
/// took, how many steps it has, and whether it had to ignore the heights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RouteStats {
    pub nodes: usize,
    pub micros: u64,
    pub steps: usize,
    pub flat: bool,
}

impl RouteStats {
    pub fn json(&self) -> Value {
        json!({
            "nodes": self.nodes,
            "ms": self.micros as f64 / MICROS_PER_MILLI,
            "steps": self.steps,
            "flat": self.flat,
        })
    }
}

/// What a walk asks beyond its goal: whether it runs, and whether a goal no
/// route reaches is an error instead of a walk as near as it goes.
pub(super) struct WalkAsk {
    pub trip: Trip,
    pub run: Option<bool>,
    pub exact: bool,
}

impl WalkAsk {
    /// The choices of a `move_to` or a `route` call. A caller's walk keeps
    /// to the roads unless it says `roads: false`.
    pub fn from_args(args: &Value) -> std::result::Result<Self, String> {
        let flag = |key: &str| args.get(key).and_then(Value::as_bool);
        let mut trip = Trip {
            arrive_within: arg_number(args, ARG_ACCURACY)
                .map_or(0, |n| u16::try_from(n).unwrap_or(u16::MAX))
                .min(ACCURACY_MAX),
            open_doors: flag(ARG_OPEN_DOORS).unwrap_or(true),
            prefer_roads: flag(ARG_ROADS).unwrap_or(true),
            ..Trip::default()
        };
        for entry in args
            .get(ARG_AVOID)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let radius = arg_number(entry, ARG_RADIUS)
                .or_else(|| arg_number(entry, ARG_R))
                .map(|r| u16::try_from(r).unwrap_or(u16::MAX));
            if let Some(serial) = arg_serial_opt(entry, ARG_SERIAL).filter(|s| s.is_valid()) {
                trip.around
                    .push((serial, radius.unwrap_or(AVOID_RADIUS_DEFAULT)));
                continue;
            }
            match (arg_number(entry, "x"), arg_number(entry, "y"), radius) {
                (Some(x), Some(y), Some(radius)) => trip.areas.push(AvoidArea {
                    x: u16::try_from(x).unwrap_or(u16::MAX),
                    y: u16::try_from(y).unwrap_or(u16::MAX),
                    radius,
                }),
                _ => return Err(BAD_AVOID.into()),
            }
        }
        Ok(Self {
            trip,
            run: flag(ARG_RUN),
            exact: flag(ARG_EXACT).unwrap_or(false),
        })
    }
}

impl Trip {
    /// Every area the route keeps out of now: the fixed ones, and one round
    /// each named object where it stands. An object out of view is passed
    /// over.
    pub fn areas(&self, world: &World) -> Vec<AvoidArea> {
        let mut areas = self.areas.clone();
        areas.extend(self.around.iter().filter_map(|(serial, radius)| {
            let at = world.map_location(*serial)?;
            Some(AvoidArea {
                x: at.x,
                y: at.y,
                radius: *radius,
            })
        }));
        areas
    }

    /// True when a walker at `at` is as near `dest` as this walk asks.
    pub fn reached(&self, at: Point3, dest: Point3) -> bool {
        if self.arrive_within == 0 {
            return uoterm_nav::same_spot(at, dest);
        }
        at.chebyshev(dest) <= u32::from(self.arrive_within)
            && (i16::from(at.z) - i16::from(dest.z)).abs()
                <= i16::from(uoterm_nav::SAME_SPOT_HEIGHT)
    }
}

/// What planning one route came to.
pub(super) struct Planned {
    /// The tiles of the route, or why neither search found one: the
    /// height-aware search's reason, then the flat one's.
    pub route: std::result::Result<Vec<Point3>, (PathError, PathError)>,
    pub stats: RouteStats,
}

/// Plans a route from `from` to `dest` with the choices of the walk under
/// way: height-aware first, and with the heights ignored when that finds
/// none.
pub(super) fn plan(inner: &mut Inner, from: Point3, dest: Point3) -> Planned {
    inner.ensure_facet();
    let started = Instant::now();
    let in_the_way = inner.blockers();
    let areas = inner.trip.areas(&inner.world.read());
    let options = RouteOptions {
        avoid: &areas,
        prefer_roads: inner.trip.prefer_roads,
        arrive_within: inner.trip.arrive_within,
        max_nodes: None,
    };
    let tiles = inner.tiles();
    let obstacles = in_the_way.obstacles();
    let first = pathfind_with(&tiles, from, dest, &obstacles, &options);
    let (route, nodes, flat) = match first.outcome {
        Ok(path) => (Ok(path.steps), first.expanded, false),
        Err(height_error) => {
            let second = pathfind_flat_with(&tiles, from, dest, &obstacles, &options);
            let nodes = first.expanded + second.expanded;
            match second.outcome {
                Ok(path) => (Ok(path.steps), nodes, true),
                Err(flat_error) => (Err((height_error, flat_error)), nodes, true),
            }
        }
    };
    let route = route.map(|steps| {
        steps
            .iter()
            .map(|s| Point3::new(s.x, s.y, s.z))
            .collect::<Vec<_>>()
    });
    let stats = RouteStats {
        nodes,
        micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        steps: route.as_ref().map_or(0, Vec::len),
        flat,
    };
    tracing::info!(
        from = %from,
        to = %dest,
        nodes = stats.nodes,
        us = stats.micros,
        steps = stats.steps,
        flat = stats.flat,
        found = route.is_ok(),
        "route planned"
    );
    Planned { route, stats }
}

/// Mends the route a few steps ahead of the walker, where a mobile or a
/// refused crossing now stands in the way, with a short detour to a later
/// step of the same route. True when it did, and the rest of the route is
/// kept; false when the full plan must be made again.
pub(super) fn patch_ahead(inner: &mut Inner) -> bool {
    let route: Vec<Point3> = inner.movement.path.iter().copied().collect();
    if route.is_empty() {
        return false;
    }
    inner.ensure_facet();
    let from = inner
        .movement
        .stepping_from(inner.world.read().self_state.location);
    let in_the_way = inner.avoided();
    let areas = inner.trip.areas(&inner.world.read());
    let tiles = inner.tiles();
    let obstacles = in_the_way.obstacles();
    let looked_at = route.len().min(PATCH_SCAN_STEPS + PATCH_ANCHOR_EXTRA);
    let mut previous = from;
    let mut first_blocked = None;
    for (i, step) in route.iter().take(looked_at).enumerate() {
        let blocked = obstacles.blocks(*step)
            || inner.movement.refused_edges.refusals(previous, *step) > 0
            || tiles.can_step(previous, step.x, step.y).is_none();
        if blocked && i < PATCH_SCAN_STEPS && first_blocked.is_none() {
            first_blocked = Some(i);
        }
        previous = *step;
    }
    let Some(first_blocked) = first_blocked else {
        return false;
    };
    let options = RouteOptions {
        avoid: &areas,
        prefer_roads: inner.trip.prefer_roads,
        arrive_within: 0,
        max_nodes: Some(PATCH_MAX_NODES),
    };
    let first_anchor = (PATCH_SCAN_STEPS - 1).max(first_blocked + 1);
    for anchor in first_anchor..looked_at {
        let target = route[anchor];
        if obstacles.blocks(target) {
            continue;
        }
        let Ok(patch) = pathfind_with(&tiles, from, target, &obstacles, &options).outcome else {
            continue;
        };
        let mended: VecDeque<Point3> = patch
            .steps
            .iter()
            .map(|s| Point3::new(s.x, s.y, s.z))
            .chain(route[anchor + 1..].iter().copied())
            .collect();
        tracing::info!(
            blocked_step = first_blocked + 1,
            rejoins_at = anchor + 1,
            detour = patch.steps.len(),
            route = mended.len(),
            "patched the route round a block ahead"
        );
        drop(tiles);
        inner.movement.path = mended;
        return true;
    }
    false
}

/// Where a `move_to` or a `route` goes: the tile x, y, or the nearest
/// landmark of a name. A spot named with no height is the surface there
/// nearest the height asked, or the walker's own.
fn destination(inner: &mut Inner, args: &Value) -> std::result::Result<Point3, &'static str> {
    let asked = walk_dest(inner, args)?;
    let named_height =
        args.get("z").and_then(Value::as_i64).is_some() || args.get("name").is_some();
    let z = if named_height {
        asked.z
    } else {
        inner.world.read().self_state.location.z
    };
    Ok(Point3::new(
        asked.x,
        asked.y,
        surface_z(inner, z, asked.x, asked.y),
    ))
}

/// `move_to`: plans a route with the choices of the call and walks it at a
/// person's pace. A goal no route reaches is walked toward as far as it
/// goes, unless the call asks for it exactly.
pub(super) fn move_to(inner: &mut Inner, args: &Value) -> ToolResult {
    let ask = match WalkAsk::from_args(args) {
        Ok(ask) => ask,
        Err(why) => return ToolResult::err(why),
    };
    let dest = match destination(inner, args) {
        Ok(dest) => dest,
        Err(why) => return ToolResult::err(why),
    };
    let here = inner.world.read().self_state.location;
    inner.trip = ask.trip;
    inner.goal = Goal::Travel { dest };
    inner.world.write().goal = Goal::Travel { dest }.name().into();
    if queue_move(inner, dest) {
        inner.movement.run_override = ask.run;
        let mut moving = ToolResult::action(TOOL_MOVE_TO);
        if let Goal::Travel { dest: heading_to } = inner.goal {
            moving.result["heading_to"] = json!(heading_to);
        }
        if let Some(stats) = inner.trip.last {
            moving.result["route"] = stats.json();
        }
        return moving;
    }
    let reason = inner
        .last_path_fail_reason
        .clone()
        .unwrap_or_else(|| NO_PATH.into());
    let stats = inner.trip.last.map(|stats| stats.json());
    if ask.exact {
        inner.goal = Goal::Idle;
        inner.world.write().goal = Goal::Idle.name().into();
        inner.trip = Trip::default();
        return ToolResult::err(reason);
    }
    // The goal could not be reached in one route: a wall or an up-high spot
    // with no way to it, a gap only a gate or teleporter crosses, or ground
    // too far past what one search covers. Rather than a bare failure, walk
    // to the nearest spot on the way and tell the agent the reason, so it can
    // call move_to again from there or pick another goal.
    //
    // A learned teleporter pad whose far side reaches the goal beats a blind
    // partial walk: stepping on it puts her where a plan works.
    if let Some(pad) = leg_route_through_pad(inner, here, dest) {
        inner.goal = Goal::Travel { dest: pad };
        inner.world.write().goal = Goal::Travel { dest: pad }.name().into();
        inner.movement.run_override = ask.run;
        return ToolResult::ok(json!({
            "partial": true,
            "goal": dest,
            "heading_to": pad,
            "via": "teleporter",
            "reason": reason,
            "route": stats,
            "hint": "walking to a known teleporter pad toward the goal; step on it, then call move_to again",
        }));
    }
    match walk_partway(inner, here, dest) {
        Some(waypoint) => {
            inner.goal = Goal::Travel { dest: waypoint };
            inner.world.write().goal = Goal::Travel { dest: waypoint }.name().into();
            inner.movement.run_override = ask.run;
            ToolResult::ok(json!({
                "partial": true,
                "goal": dest,
                "heading_to": waypoint,
                "reason": reason,
                "route": stats,
                "hint": "could not reach the goal; walking to the nearest reachable spot, then call move_to again from there",
            }))
        }
        None => ToolResult::err(reason),
    }
}

/// `route`: plans the walk `move_to` would take with the same choices, and
/// says whether it gets there, its tiles, and how long the plan took,
/// without a step.
pub(super) fn route(inner: &mut Inner, args: &Value) -> ToolResult {
    let ask = match WalkAsk::from_args(args) {
        Ok(ask) => ask,
        Err(why) => return ToolResult::err(why),
    };
    let dest = match destination(inner, args) {
        Ok(dest) => dest,
        Err(why) => return ToolResult::err(why),
    };
    let from = inner
        .movement
        .stepping_from(inner.world.read().self_state.location);
    // The walk under way keeps its own choices; this plan borrows the
    // session for the length of one search.
    let under_way = std::mem::replace(&mut inner.trip, ask.trip);
    let planned = plan(inner, from, dest);
    inner.trip = under_way;
    match planned.route {
        Ok(steps) => ToolResult::ok(json!({
            "reachable": true,
            "goal": dest,
            "ends_at": steps.last().copied().unwrap_or(from),
            "steps": steps.len(),
            "route": steps,
            "search": planned.stats.json(),
        })),
        Err((height_error, flat_error)) => ToolResult::ok(json!({
            "reachable": false,
            "goal": dest,
            "why": format!("{height_error}; flat: {flat_error}"),
            "search": planned.stats.json(),
        })),
    }
}

/// One landmark as a search gives it back.
fn landmark_json(mark: &crate::landmarks::Landmark, dist: Option<u32>) -> Value {
    json!({
        "name": mark.name,
        "map": mark.map,
        "location": mark.at,
        "dist": dist,
        "kind": mark.kind,
    })
}

/// `find_landmarks`: the named places of the marker file on one map, by
/// name and kind, nearest first; `closest` gives the nearest one of a kind
/// alone. When the map underfoot has none and the call named no map, the
/// matches on the other maps are named, so the agent knows it must travel
/// there first.
pub(super) fn find_landmarks(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(marks) = inner.landmarks.clone() else {
        return ToolResult::err(NO_MARKERS_LOADED);
    };
    let name = args.get("name").and_then(Value::as_str);
    let closest = args.get(ARG_CLOSEST).and_then(Value::as_str);
    let kind = closest.or_else(|| args.get(ARG_KIND).and_then(Value::as_str));
    let within = arg_number(args, ARG_DISTANCE);
    let (here, here_map) = {
        let world = inner.world.read();
        (world.self_state.location, world.self_state.map)
    };
    let asked_map = arg_number(args, ARG_MAP).and_then(|m| u8::try_from(m).ok());
    let map = asked_map.unwrap_or(here_map);
    // A distance only means something on the map the character stands on;
    // a landmark on another map has none, so a distance filter drops it, the
    // same way find_items drops an item it cannot place.
    let dist_of =
        |m: &crate::landmarks::Landmark| (m.map == here_map).then(|| here.chebyshev(m.at));
    let mut found: Vec<(&crate::landmarks::Landmark, Option<u32>)> = marks
        .find_of_kind(name, kind, Some(map))
        .into_iter()
        .map(|m| (m, dist_of(m)))
        .filter(|(_, dist)| !within.is_some_and(|w| dist.is_none_or(|d| d > w)))
        .collect();
    // Nearest first, then the ones off this map that have no distance.
    found.sort_by_key(|(_, dist)| dist.unwrap_or(u32::MAX));
    if closest.is_some() {
        found.truncate(1);
    }
    let places: Vec<Value> = found
        .iter()
        .map(|(m, dist)| landmark_json(m, *dist))
        .collect();
    if !places.is_empty() || asked_map.is_some() {
        return ToolResult::ok(json!(places));
    }
    let elsewhere: Vec<&crate::landmarks::Landmark> = marks
        .find_of_kind(name, kind, None)
        .into_iter()
        .filter(|m| m.map != map)
        .collect();
    if elsewhere.is_empty() {
        return ToolResult::ok(json!(places));
    }
    let mut maps: Vec<u8> = elsewhere.iter().map(|m| m.map).collect();
    maps.sort_unstable();
    maps.dedup();
    ToolResult::ok(json!({
        "places": places,
        "note": format!(
            "none on map {map}; {} on other maps ({}): travel there first",
            elsewhere.len(),
            maps.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")
        ),
        "elsewhere": elsewhere
            .iter()
            .take(ELSEWHERE_SHOWN)
            .map(|m| landmark_json(m, None))
            .collect::<Vec<_>>(),
    }))
}

/// `landmarks_info`: what marker data the session read: whether any, how
/// many places, and how many on each map and of each kind.
pub(super) fn landmarks_info(inner: &Inner) -> ToolResult {
    match inner.landmarks.as_deref() {
        None => ToolResult::ok(json!({ "loaded": false, "hint": NO_MARKERS_LOADED })),
        Some(marks) => ToolResult::ok(json!({
            "loaded": true,
            "count": marks.len(),
            "maps": marks.per_map(),
            "kinds": marks.kinds(),
        })),
    }
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

    #[test]
    fn the_choices_of_a_walk_are_read_from_its_call() {
        let args = json!({
            "run": true,
            "accuracy": 2,
            "open_doors": false,
            "exact": true,
            "avoid": [{ "x": 5, "y": 6, "radius": 2 }, { "serial": 0x0000_0055u32 }],
        });
        let asked = WalkAsk::from_args(&args).unwrap();
        assert_eq!(asked.run, Some(true));
        assert!(asked.exact);
        assert_eq!(asked.trip.arrive_within, 2);
        assert!(!asked.trip.open_doors);
        assert!(asked.trip.prefer_roads);
        assert_eq!(
            asked.trip.areas,
            vec![AvoidArea {
                x: 5,
                y: 6,
                radius: 2
            }]
        );
        assert_eq!(
            asked.trip.around,
            vec![(Serial(0x55), AVOID_RADIUS_DEFAULT)]
        );
        assert!(WalkAsk::from_args(&json!({ "avoid": [{ "x": 5 }] })).is_err());
    }

    #[test]
    fn a_walk_that_may_stop_short_is_reached_near_its_goal() {
        let near = Trip {
            arrive_within: 2,
            ..Trip::default()
        };
        let goal = Point3::new(HERE.x + 2, HERE.y, 0);
        assert!(near.reached(HERE, goal));
        assert!(!Trip::default().reached(HERE, goal));
        assert!(!near.reached(HERE, Point3::new(HERE.x + 3, HERE.y, 0)));
    }

    #[test]
    fn a_plan_says_how_much_it_searched_and_keeps_out_of_areas() {
        let mut inner = test_session();
        inner.world.write().self_state.location = HERE;
        let dest = Point3::new(HERE.x + 10, HERE.y, 0);
        inner.trip = WalkAsk::from_args(&json!({
            "avoid": [{ "x": HERE.x + 5, "y": HERE.y, "radius": 2 }],
        }))
        .unwrap()
        .trip;
        let planned = plan(&mut inner, HERE, dest);
        let route = planned.route.unwrap();
        assert!(planned.stats.nodes > 0);
        assert_eq!(planned.stats.steps, route.len());
        assert!(route
            .iter()
            .all(|p| p.chebyshev(Point3::new(HERE.x + 5, HERE.y, 0)) > 1));
    }

    #[test]
    fn a_route_names_its_search_and_an_exact_walk_to_a_wall_fails() {
        let mut inner = test_session();
        inner.world.write().self_state.location = HERE;
        let planned = route(
            &mut inner,
            &json!({ "x": HERE.x + 6, "y": HERE.y, "accuracy": 1 }),
        );
        assert!(planned.ok, "{planned:?}");
        assert_eq!(planned.result["reachable"], json!(true));
        assert_eq!(planned.result["steps"], json!(5));
        assert!(planned.result["search"]["nodes"].as_u64().unwrap() > 0);
        assert_eq!(inner.trip, Trip::default(), "a plan leaves the walk alone");
        let wall = Point3::new(HERE.x + 3, HERE.y, 0);
        inner.map.set_block(wall.x, wall.y, true);
        let refused = move_to(
            &mut inner,
            &json!({ "x": wall.x, "y": wall.y, "exact": true }),
        );
        assert!(!refused.ok);
        assert_eq!(inner.goal, Goal::Idle);
        let near = move_to(
            &mut inner,
            &json!({ "x": wall.x, "y": wall.y, "accuracy": 1, "run": true }),
        );
        assert!(near.ok, "{near:?}");
        assert_eq!(near.result["heading_to"]["x"], json!(wall.x - 1));
        assert_eq!(inner.movement.run_override, Some(true));
    }

    #[test]
    fn landmarks_are_found_by_kind_nearest_first_and_named_on_other_maps() {
        let mut inner = test_session();
        inner.world.write().self_state.location = HERE;
        assert!(!find_landmarks(&mut inner, &json!({})).ok, "no marker file");
        assert_eq!(landmarks_info(&inner).result["loaded"], json!(false));
        let text = format!(
            "3\n-BANK: {} {} 0 Near Bank \n-BANK: {} {} 0 Far Bank \n+MOONGATE: 10 10 1 Trammel Gate \n",
            HERE.x + 2,
            HERE.y,
            HERE.x + 40,
            HERE.y
        );
        inner.landmarks = Some(Arc::new(crate::landmarks::Landmarks::parse_uoam_map(&text)));
        let banks = find_landmarks(&mut inner, &json!({ "kind": "bank" }));
        assert_eq!(banks.result[0]["name"], json!("Near Bank"));
        assert_eq!(banks.result.as_array().unwrap().len(), 2);
        let nearest = find_landmarks(&mut inner, &json!({ "closest": "bank" }));
        assert_eq!(nearest.result.as_array().unwrap().len(), 1);
        let gate = find_landmarks(&mut inner, &json!({ "kind": "moongate" }));
        assert_eq!(gate.result["elsewhere"][0]["map"], json!(1));
        assert!(gate.result["note"].as_str().unwrap().contains("other maps"));
        let info = landmarks_info(&inner).result;
        assert_eq!(info["count"], json!(3));
        assert_eq!(info["kinds"]["bank"], json!(2));
    }

    #[test]
    fn a_mobile_that_steps_onto_the_route_is_walked_round() {
        let mut inner = test_session();
        inner.world.write().self_state.location = HERE;
        let route: Vec<Point3> = (1..=8)
            .map(|dx| Point3::new(HERE.x + dx, HERE.y, 0))
            .collect();
        inner
            .movement
            .set_path(route.clone(), *route.last().unwrap());
        assert!(!patch_ahead(&mut inner), "nothing blocks it yet");
        inner.movement.blocked.refuse(route[1], Instant::now());
        assert!(patch_ahead(&mut inner));
        let mended: Vec<Point3> = inner.movement.path.iter().copied().collect();
        assert!(!mended.contains(&route[1]));
        assert_eq!(mended.last(), route.last());
    }
}
