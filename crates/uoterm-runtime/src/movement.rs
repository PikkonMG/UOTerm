use crate::config::{JITTER_PCT, STEP_MOUNT_RUN_MS, STEP_MOUNT_WALK_MS, STEP_RUN_MS, STEP_WALK_MS};
use rand::Rng;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use uoterm_nav::{pathfind, BlockedMove, Obstacles, TileQuery, SAME_MOVE_HEIGHT};
use uoterm_protocol::encode;
use uoterm_protocol::types::{Direction, Point3, Serial};
use uoterm_protocol::EquipItem;
use uoterm_world::DoorItem;

pub const SEQ_MOD: u16 = 256;
pub const SEQ_FIRST: u8 = 0;
pub const SEQ_AFTER_WRAP: u8 = 1;
pub const SEQ_LAST: u8 = 255;
/// How long the server took to confirm a step, measured on a live shard while
/// one step at a time was allowed on the wire. Confirmations of one walk came
/// back at 05.344, 05.643, 05.944, 06.143 and 06.443 seconds: this far apart,
/// because each step waited for the answer to the one before it.
const STEP_ACK_ROUND_TRIP_MS: u64 = 300;
/// How much slower than the measured shard a shard may be before its round
/// trip sets the pace again.
const SLOW_SHARD_MARGIN: u64 = 2;
/// How many requests the server keeps room for. It holds a ring of this many
/// slots for the moves one client has not had an answer to, and expects no
/// more than that on the wire at once.
const SERVER_MOVE_RING_SLOTS: usize = 5;
/// How many of those slots the character leaves the server, so a request that
/// crosses an answer on the wire never fills the ring.
const SERVER_RING_HEADROOM: usize = 1;
/// How many requests the character lets the server owe him.
///
/// One step at a time is a speed limit, not a safety rule. A running person
/// steps every [`STEP_RUN_MS`], a confirmation takes
/// [`STEP_ACK_ROUND_TRIP_MS`] to come back, and a character who waits for
/// each answer therefore steps slower than the person he follows and loses
/// ground on every step. The sequence byte is what lets more than one
/// request be on the wire, and this is the number the reference client uses:
/// the server's ring of [`SERVER_MOVE_RING_SLOTS`] less
/// [`SERVER_RING_HEADROOM`]. It also covers the measured round trip at
/// running pace on a shard [`SLOW_SHARD_MARGIN`] times slower than the
/// measured one, which the assertions below hold it to.
///
/// This changes nothing about where the character is. Only a confirmation
/// moves him, so several steps on the wire mean the tile he is reported on
/// trails the tile he is walking to by at most this many, and every route,
/// scene and decision still reads a tile the server put him on.
/// [`Movement::stepping_from`] is what the next step is aimed from.
pub const IN_FLIGHT_MAX: usize = SERVER_MOVE_RING_SLOTS - SERVER_RING_HEADROOM;
const _: () = assert!(
    IN_FLIGHT_MAX as u64 * STEP_RUN_MS >= SLOW_SHARD_MARGIN * STEP_ACK_ROUND_TRIP_MS,
    "the wire must hold enough running steps to cover a slow shard's round trip"
);
/// How long the character waits for the server to confirm a step before he
/// treats it as lost. Longer than a round trip on any shard worth playing, and
/// short enough that a step the server drops does not hold him still.
pub const STEP_ACK_TIMEOUT: Duration = Duration::from_millis(STEP_WALK_MS * STEP_ACK_STEPS);
/// How many walking steps' worth of time the server has to confirm a step: one
/// for every request the wire holds.
///
/// It has to be at least that, or a character walking a straight line fills the
/// wire more slowly than the server is given to answer the first of those
/// requests, and gives up on a shard that is answering him. The assertion in
/// the tests holds it to that, jitter and all.
const STEP_ACK_STEPS: u64 = IN_FLIGHT_MAX as u64;
pub const FASTWALK_SLOTS: usize = 6;
/// An empty fastwalk slot, and the key a request carries when the stack is
/// empty. A server that never sends a key reads nought as no key at all.
pub const FASTWALK_KEY_EMPTY: u32 = 0;
/// How many walking steps' worth of time a refused tile stays blocked.
const REFUSED_TILE_STEPS: u64 = 150;
/// How long the character remembers a tile the server refused him.
///
/// The largest house in Ultima Online stands on 18 by 18 tiles, and its wall
/// is 68 tiles round. [`REFUSED_TILE_STEPS`] is more than twice that many
/// walking steps, so he finishes the walk around a house with its wall still
/// remembered,
/// instead of forgetting the first corner half way along and turning back into
/// it. A minute is also short enough that a boat that has sailed on, a door
/// somebody shut behind him, or a step the shard refused for a passing reason
/// costs him one long way round and no more.
pub const REFUSED_TILE_MEMORY: Duration = Duration::from_millis(STEP_WALK_MS * REFUSED_TILE_STEPS);
/// How many refused tiles the character remembers at once.
///
/// The wall of the largest house is 68 tiles, so this holds three of them and
/// the boats besides: more than one character meets between one tile expiring
/// and the next. The oldest goes first, so the wall he is walking around now
/// is the wall he keeps, and a session that runs for days cannot grow this
/// without limit.
pub const REFUSED_TILES_MAX: usize = 256;
/// How far apart two refusals of one directed edge may stand in height and
/// still be the same edge: one step of a stair, so a step refused on a slope is
/// recognised again however the ground under it is read.
///
/// It is the window the route finder matches a [`BlockedMove`] on, because this
/// memory and that list must agree on what one crossing is.
pub const EDGE_Z_TOLERANCE: i16 = SAME_MOVE_HEIGHT as i16;
/// How many directed edges the character remembers at once. One for every tile
/// the refused-tile memory holds, which is the most edges those tiles can be
/// reached over before the oldest of them is forgotten.
pub const REFUSED_EDGES_MAX: usize = REFUSED_TILES_MAX;
/// How long a turn on the spot costs before the next request may go out.
///
/// Eighty milliseconds. A turn moves the character nowhere: the server sets
/// the new location to the old one, charges the walk nothing and starts its
/// own pace again from the turn. Charging a turn a whole walking step loses
/// the character a tile of ground on every change of direction, which is
/// ground a person he follows never gives back.
pub const TURN_PACE: Duration = Duration::from_millis(TURN_PACE_MS);
const TURN_PACE_MS: u64 = 80;
/// The pace flag a turn carries on the wire: nobody runs where he stands.
const TURN_RUN_FLAG: bool = false;
/// How many times one trip may be planned again before it is given up.
///
/// A trip that has been planned again this often is one nothing is going to
/// solve: the way is shut, and every new route walks into the same thing.
/// Ending it hands the caller a failure it can act on, where looping hands it
/// a character who never arrives and never says why.
pub const REPLANS_MAX: u32 = 128;
/// The layer a mount is worn on. It sits between [`LAYER_BACKPACK`] and
/// [`LAYER_BANK`] in the same table, and an item on it is the only word the
/// server gives that the character is riding.
///
/// [`LAYER_BACKPACK`]: uoterm_protocol::types::LAYER_BACKPACK
/// [`LAYER_BANK`]: uoterm_protocol::types::LAYER_BANK
pub const MOUNT_LAYER: u8 = 25;
#[cfg(test)]
const MOVE_REQ_SEQ_INDEX: usize = 2;

/// The eight tiles that touch one tile, in wire direction order. Two tiles the
/// same distance away break the tie toward the first of these.
pub const ADJACENT_DIRS: [Direction; 8] = [
    Direction::North,
    Direction::Northeast,
    Direction::East,
    Direction::Southeast,
    Direction::South,
    Direction::Southwest,
    Direction::West,
    Direction::Northwest,
];
/// Where a follower settles: one tile from its target, which is the tile
/// beside it. Nearer than that is the target's own tile, which a follower
/// never stands on. Once it is this near it stops closing and holds.
pub const FOLLOW_NEAR_DISTANCE: u32 = 1;
/// The largest gap a settled follower puts up with. It starts to close again
/// only after the gap opens past this, and then it closes all the way back to
/// [`FOLLOW_NEAR_DISTANCE`].
///
/// The band between the two distances is one tile wide, the narrowest band
/// there is, and it is what stops the step-in step-back cycle that one single
/// stop distance makes: at a gap of exactly this many tiles the follower
/// holds if it was settled and keeps closing if it was closing.
pub const FOLLOW_FAR_DISTANCE: u32 = 2;
/// The target must move at least this far from the tile the queued path was
/// built for before the follower searches for a new path.
///
/// One tile: a follower that keeps this close walks a path one or two steps
/// long, so a path built for a tile the target has already left ends beside
/// nobody. A search over two tiles costs almost nothing, and holding a stale
/// path is exactly the trailing this band is meant to stop. Nought would pay
/// for a search on every tick while the target stands still.
pub const FOLLOW_REPATH_DISTANCE: u32 = 1;

fn same_tile(a: Point3, b: Point3) -> bool {
    a.x == b.x && a.y == b.y
}

/// One step the character is about to ask the server for: the tile it lands
/// on, and the direction that reaches it.
///
/// `arrives_at` carries the height the character stands at once that step is
/// confirmed, and that height is worked out from the map for that one step.
/// The route that planned the walk gives it for a queued step
/// ([`Movement::pop_next_step`]); [`TileQuery::can_step`] gives it for a step
/// no route planned. It is never the height of the tile he leaves. A step
/// that carried the old height wrote it into the world model the moment the
/// server confirmed the step, so walking never changed the character's
/// recorded height at all: on the hillside above the Britain bank one stood
/// at 1419,1709, where the ground is 18, believing she was at height 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NextStep {
    /// The tile the step lands on, at the height the map says he reaches
    /// there.
    pub arrives_at: Point3,
    pub direction: Direction,
}

/// A request the character has sent and the server has not answered yet: a
/// step, or a turn on the spot.
#[derive(Clone, Debug)]
pub struct PendingStep {
    pub sequence: u8,
    pub direction: Direction,
    /// The tile the character stands on once the server confirms this
    /// request, at the height the map worked out for it, and not one moment
    /// before.
    ///
    /// A step carries the tile it was aimed at. A turn moves nobody, so it
    /// carries the tile the requests before it leave him on, which is the tile
    /// he is already standing on when its answer comes back.
    pub arrives_at: Point3,
    pub sent_at: Instant,
    /// True when the request only turned him where he stands.
    ///
    /// A move request in a direction the character does not face turns him and
    /// moves him nowhere: the server sets the new location to the old one and
    /// answers all the same. So the same direction goes out twice, once to
    /// turn and once to step, and only the second of them takes a tile out of
    /// the route.
    pub turn: bool,
}

/// One directed edge the server refused, and how often.
#[derive(Clone, Copy, Debug)]
struct RefusedEdge {
    move_: BlockedMove,
    refusals: u32,
    forget_at: Instant,
}

/// The steps the server refused, each remembered as the directed pair of tiles
/// it crossed: a [`BlockedMove`].
///
/// A tile is not always what a refusal is about. The doorway takes the
/// character from the other side, a fence lets him through the gate one tile
/// along, and the step of a stair is entered from the step below it and from
/// nowhere else. So the crossing is what is counted, and the count is what
/// tells the first refusal of one edge from the second: the first is worth a
/// door and one more try at the same step, and the second is what proves the
/// way shut.
///
/// Heights are matched within [`EDGE_Z_TOLERANCE`], because the same crossing
/// on a slope is read at a slightly different height every time it is planned.
#[derive(Debug, Default)]
pub struct RefusedEdges {
    edges: Vec<RefusedEdge>,
}

/// True while two spots are the two ends of one crossing: the same column, at
/// heights no further apart than one step of a stair.
fn same_edge(held: BlockedMove, from: Point3, to: Point3) -> bool {
    same_edge_end(held.from, from) && same_edge_end(held.to, to)
}

fn same_edge_end(a: Point3, b: Point3) -> bool {
    same_tile(a, b) && (i16::from(a.z) - i16::from(b.z)).abs() <= EDGE_Z_TOLERANCE
}

impl RefusedEdges {
    /// Counts one refusal of the crossing from `from` to `to` and gives back
    /// how many the character has now met on it.
    pub fn refuse(&mut self, from: Point3, to: Point3, now: Instant) -> u32 {
        let forget_at = now + REFUSED_TILE_MEMORY;
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|edge| same_edge(edge.move_, from, to))
        {
            edge.refusals += 1;
            edge.forget_at = forget_at;
            return edge.refusals;
        }
        if self.edges.len() >= REFUSED_EDGES_MAX {
            self.edges.remove(0);
        }
        self.edges.push(RefusedEdge {
            move_: BlockedMove { from, to },
            refusals: 1,
            forget_at,
        });
        1
    }

    /// How many refusals the character remembers on that crossing.
    pub fn refusals(&self, from: Point3, to: Point3) -> u32 {
        self.edges
            .iter()
            .find(|edge| same_edge(edge.move_, from, to))
            .map(|edge| edge.refusals)
            .unwrap_or(0)
    }

    /// Every crossing proven shut, as the route finder reads them. A crossing
    /// shuts the one way in that failed and leaves its tile open from every
    /// other side, which is what a tile of a stair needs.
    pub fn moves(&self) -> Vec<BlockedMove> {
        self.edges.iter().map(|edge| edge.move_).collect()
    }

    /// Takes out every crossing whose time is up, so nothing is remembered for
    /// ever: a door opens and a boat sails.
    pub fn expire(&mut self, now: Instant) {
        self.edges.retain(|edge| now < edge.forget_at);
    }

    /// Forgets every crossing at once, for a new journey.
    pub fn clear(&mut self) {
        self.edges.clear();
    }
}

/// One tile the server refused, and when the character forgets it.
#[derive(Clone, Copy, Debug)]
struct RefusedTile {
    at: Point3,
    forget_at: Instant,
}

/// The tiles the server refused the character, which every route goes around
/// until he forgets them.
///
/// A player house, a wall or post of one, and a boat are all built on the
/// server and stand in no client map file. This client reads the map files, so
/// it calls those tiles open ground, plans a route straight through them, and
/// the server refuses the step. The refusal is the only word it ever gets that
/// anything is there, and this is where that word is kept. No house is
/// understood here, and none needs to be: what is remembered is the one tile
/// that would not take him.
///
/// Tiles are held in the order they were refused. Every one is given the same
/// time, so the front of the queue is both the oldest tile and the first one
/// due to be forgotten.
///
/// One refusal blocks one tile and never a line. A refusal is one fact about
/// one cell, and nothing in it says which way the thing in the way runs: a
/// wall curves, a fence has a gate in it, and a queue of people is no wall at
/// all. Tiles guessed at past the one that was really refused close the very
/// gap the character should walk through.
#[derive(Debug, Default)]
pub struct BlockedTiles {
    tiles: VecDeque<RefusedTile>,
}

impl BlockedTiles {
    /// Records the one tile the server would not let the character enter, and
    /// gives back the tile dropped to make room for it once the memory is
    /// full.
    ///
    /// Exactly one cell, at the tile that was really refused. A tile refused
    /// again is fresh news: its time starts over and it goes to the back of
    /// the queue, so the wall he keeps meeting outlives one he met once and
    /// has walked away from.
    pub fn refuse(&mut self, at: Point3, now: Instant) -> Option<Point3> {
        self.tiles.retain(|held| !same_tile(held.at, at));
        self.tiles.push_back(RefusedTile {
            at,
            forget_at: now + REFUSED_TILE_MEMORY,
        });
        if self.tiles.len() > REFUSED_TILES_MAX {
            return self.tiles.pop_front().map(|dropped| dropped.at);
        }
        None
    }

    /// Forgets every tile at once, and gives them all back. A new destination
    /// is a new journey: a minute is a long time to hold a mark that was made
    /// on the way somewhere else.
    pub fn clear(&mut self) -> Vec<Point3> {
        self.tiles.drain(..).map(|held| held.at).collect()
    }

    /// Takes out every tile whose time is up and gives them back, so nothing
    /// is remembered for ever. A door opens, a boat sails, and a shard refuses
    /// a step for a passing reason.
    pub fn expire(&mut self, now: Instant) -> Vec<Point3> {
        let due = self
            .tiles
            .iter()
            .take_while(|held| now >= held.forget_at)
            .count();
        self.tiles.drain(..due).map(|held| held.at).collect()
    }

    /// Forgets one tile, and says whether one was there to forget. The server
    /// putting the character on a tile is proof that it takes him, whatever it
    /// refused him before.
    pub fn forget(&mut self, at: Point3) -> bool {
        let held = self.tiles.len();
        self.tiles.retain(|tile| !same_tile(tile.at, at));
        self.tiles.len() != held
    }

    /// Every tile still remembered, for the routes that must go around them.
    pub fn tiles(&self) -> Vec<Point3> {
        self.tiles.iter().map(|held| held.at).collect()
    }
}

#[derive(Debug)]
pub struct Movement {
    pub sequence: u8,
    pub in_flight: VecDeque<PendingStep>,
    /// When the next step falls due. See [`Movement::schedule_next`].
    next_step_due: Option<Instant>,
    pub path: VecDeque<Point3>,
    pub goal: Option<Point3>,
    pub fastwalk: [u32; FASTWALK_SLOTS],
    pub last_dir: Direction,
    pub run_override: Option<bool>,
    /// True while the character rides. A mount has a pace of its own, and the
    /// session sets this from the item the server puts on [`MOUNT_LAYER`].
    pub mounted: bool,
    /// The tiles the server refused him, which every route goes around.
    pub blocked: BlockedTiles,
    /// The crossings the server refused him, which say whether a refusal is
    /// the first at that place or the second.
    pub refused_edges: RefusedEdges,
    /// The tile this trip is aimed at, which is what tells one journey from
    /// the next.
    trip_dest: Option<Point3>,
    /// How many times the route of this trip has been planned again.
    replans: u32,
    /// The cell the character is waiting at for somebody to move off it, and
    /// how often he has waited there.
    wait_cell: Option<Point3>,
    /// Waits at [`Movement::wait_cell`] since the last one that blocked it.
    waits: u32,
    /// Waits at [`Movement::wait_cell`] over the whole trip, which nothing but
    /// a new trip resets.
    waits_this_trip: u32,
}

impl Default for Movement {
    fn default() -> Self {
        Self {
            sequence: SEQ_FIRST,
            in_flight: VecDeque::new(),
            next_step_due: None,
            path: VecDeque::new(),
            goal: None,
            fastwalk: [FASTWALK_KEY_EMPTY; FASTWALK_SLOTS],
            last_dir: Direction::North,
            run_override: None,
            mounted: false,
            blocked: BlockedTiles::default(),
            refused_edges: RefusedEdges::default(),
            trip_dest: None,
            replans: 0,
            wait_cell: None,
            waits: 0,
            waits_this_trip: 0,
        }
    }
}

impl Movement {
    /// How long one step of a person takes, give or take: nobody walks to a
    /// metronome, so every step is worth a little more or a little less than
    /// the pace.
    ///
    /// A mount has a pace of its own, twice the pace of the person on it at
    /// both a walk and a run.
    pub fn next_interval(&self, running: bool) -> Duration {
        let base = match (self.mounted, running) {
            (true, true) => STEP_MOUNT_RUN_MS,
            (true, false) => STEP_MOUNT_WALK_MS,
            (false, true) => STEP_RUN_MS,
            (false, false) => STEP_WALK_MS,
        };
        let jitter = (base as u32 * JITTER_PCT) / 100;
        let lo = base.saturating_sub(jitter as u64);
        let hi = base + jitter as u64;
        Duration::from_millis(rand::thread_rng().gen_range(lo..=hi))
    }

    /// True when the character may send a step now: the server owes him fewer
    /// than [`IN_FLIGHT_MAX`] answers, and his next step has fallen due. The
    /// pace is what spaces his steps; the answers only stop him running away
    /// from a server that has stopped listening.
    pub fn ready(&self, now: Instant) -> bool {
        if self.in_flight.len() >= IN_FLIGHT_MAX {
            return false;
        }
        match self.next_step_due {
            // The first step of a walk waits for nobody.
            None => true,
            Some(due) => now >= due,
        }
    }

    /// Sets when the step after this one falls due.
    ///
    /// A pace is a cadence and not a delay: the next step is due `pace` after
    /// the moment this one was *due*, not after the moment it went out. The
    /// walk runs on a [`crate::config::REFLEX_TICK_MS`] tick, so a step always
    /// goes out a little after it falls due; measuring the next one from when
    /// it went out adds that lateness to every step and to the one after it,
    /// and the character walks slower than the pace he is walking at.
    ///
    /// This is the measured fault: a follower behind a running player kept a
    /// gap of 4.2 tiles on average and 18 at worst, because a 200 ms running
    /// step that could only leave on a 100 ms tick left every other time at
    /// 300 ms. Ground lost that way is never won back, because the person she
    /// follows never walks slower to let her.
    ///
    /// A walk that stopped for longer than one step has no cadence left to
    /// keep, and starts a new one from now. That is also what stops the
    /// character from firing off a burst of steps to make up a long wait.
    fn schedule_next(&mut self, now: Instant, pace: Duration) {
        let from = match self.next_step_due {
            Some(due) if now.saturating_duration_since(due) < pace => due,
            _ => now,
        };
        self.next_step_due = Some(from + pace);
    }

    /// Reference client walk sequence: emit 0 first, then 1..=255, wrap to 1
    /// (skip 0).
    pub fn next_sequence(&mut self) -> u8 {
        let seq = self.sequence;
        let next = (u16::from(self.sequence) + 1) % SEQ_MOD;
        self.sequence = if next == 0 {
            SEQ_AFTER_WRAP
        } else {
            next as u8
        };
        seq
    }

    pub fn reset_sequence(&mut self) {
        self.sequence = SEQ_FIRST;
    }

    /// The sequence number and fastwalk key the next move request carries.
    /// Every request the client sends, a step or a turn on the spot, takes the
    /// next pair.
    fn next_ticket(&mut self) -> (u8, u32) {
        (self.next_sequence(), self.take_fastwalk())
    }

    /// Takes one fastwalk key off the stack and leaves the slot empty.
    ///
    /// A key is spent once. The stack is read from the front, the slot it came
    /// from is emptied, and [`FASTWALK_KEY_EMPTY`] comes back when nothing is
    /// left: a key sent twice is what a shard reads as a client walking
    /// faster than a person can.
    fn take_fastwalk(&mut self) -> u32 {
        for slot in self.fastwalk.iter_mut() {
            if *slot != FASTWALK_KEY_EMPTY {
                return std::mem::replace(slot, FASTWALK_KEY_EMPTY);
            }
        }
        FASTWALK_KEY_EMPTY
    }

    /// Puts one new key on the stack, and says whether there was room for it.
    ///
    /// This is what the packet that refills the stack one key at a time calls.
    /// A full stack takes no more: the server sends a key for a request the
    /// character has made, so it cannot owe him more keys than the stack
    /// holds.
    pub fn push_fastwalk(&mut self, key: u32) -> bool {
        let Some(slot) = self
            .fastwalk
            .iter_mut()
            .find(|slot| **slot == FASTWALK_KEY_EMPTY)
        else {
            return false;
        };
        *slot = key;
        true
    }

    /// The tile the next step is aimed from: the tile the last step still
    /// waiting for an answer is aimed at, and `confirmed_at` when the server
    /// owes nothing.
    ///
    /// This is not where the character is. It is where the steps he has
    /// already sent leave him, and it is the only honest start for the step
    /// after them: a step aimed from `confirmed_at` while others are on the
    /// wire walks him back over ground he has already asked to cross. Nothing
    /// here writes a position; the character is reported on `confirmed_at`
    /// until the server confirms each of those steps in turn.
    pub fn stepping_from(&self, confirmed_at: Point3) -> Point3 {
        self.in_flight
            .back()
            .map(|step| step.arrives_at)
            .unwrap_or(confirmed_at)
    }

    /// Builds the packet for one step and holds that step until the server
    /// answers it.
    ///
    /// A step is a request, not a move: nothing here says where the character
    /// is. [`Movement::ack`] hands the step back when the server confirms it,
    /// and the tile it carries is the tile he stands on from then on. That
    /// tile is [`NextStep::arrives_at`], height and all, so whoever asks for
    /// the step is the one that worked the height out: the route for a queued
    /// step, and [`TileQuery::can_step`] for a step no route planned. Nothing
    /// here guesses one.
    pub fn build_step(&mut self, step: NextStep, running: bool, now: Instant) -> Vec<u8> {
        let (sequence, key) = self.next_ticket();
        self.in_flight.push_back(PendingStep {
            sequence,
            direction: step.direction,
            arrives_at: step.arrives_at,
            sent_at: now,
            turn: false,
        });
        self.schedule_next(now, self.next_interval(running));
        self.last_dir = step.direction;
        encode::move_request(step.direction, running, sequence, key)
    }

    /// Turns the character to face `direction` where he stands, and holds that
    /// turn until the server answers it. `None` comes back when he already
    /// faces that way and no packet is needed.
    ///
    /// A move request in a direction the character does not face turns him and
    /// moves him nowhere: the server sets the new location to the old one and
    /// answers all the same. So the turn is a request like any other. It takes
    /// a sequence number, it fills a slot on the wire, and it is answered; what
    /// it never does is take a tile, which is why `standing_on` is the tile the
    /// requests before it leave him on and not a new one.
    ///
    /// A turn costs [`TURN_PACE`] and not a step of the pace he walks at. The
    /// server charges a turn nothing at all and starts its own pace again from
    /// it, so charging a whole walking step here loses a tile of ground on
    /// every change of direction.
    pub fn build_turn(
        &mut self,
        facing: Direction,
        direction: Direction,
        standing_on: Point3,
        now: Instant,
    ) -> Option<Vec<u8>> {
        if facing == direction {
            return None;
        }
        let (sequence, key) = self.next_ticket();
        self.in_flight.push_back(PendingStep {
            sequence,
            direction,
            arrives_at: standing_on,
            sent_at: now,
            turn: true,
        });
        self.last_dir = direction;
        self.schedule_next(now, TURN_PACE);
        Some(encode::move_request(
            direction,
            TURN_RUN_FLAG,
            sequence,
            key,
        ))
    }

    /// The way the character faces once every request already sent has been
    /// answered: the direction of the last of them, and `reported` when the
    /// server owes him nothing.
    ///
    /// This is what the facing of the next step is tested against.
    /// `reported` alone is the facing of some tile behind him whenever a
    /// request is still out, and a step tested against that turns him a second
    /// time in a direction he has already asked to face.
    pub fn facing_after(&self, reported: Direction) -> Direction {
        self.in_flight
            .back()
            .map(|pending| pending.direction)
            .unwrap_or(reported)
    }

    /// The direction of the step the server is refusing: the oldest step still
    /// waiting for an answer. The server answers steps in the order it was
    /// sent them, so the first one it has not confirmed is the one it is
    /// refusing. Read it before [`Movement::reject`] throws that step away.
    pub fn refused_direction(&self) -> Option<Direction> {
        self.in_flight.front().map(|p| p.direction)
    }

    /// Takes the step the server has just confirmed, so the caller can put the
    /// character on the tile it carries.
    ///
    /// Several steps can be waiting for an answer, and the server answers them
    /// in the order it was sent them, so an answer normally matches the oldest
    /// one and takes the character one tile on. An answer that matches a later
    /// step takes the steps before it with it: the server moved him over those
    /// tiles to reach the one this step is aimed at, and that tile is where he
    /// stands. A sequence no waiting step carries, a stale one or the answer
    /// to a turn on the spot, matches nothing and moves nobody.
    pub fn ack(&mut self, sequence: u8) -> Option<PendingStep> {
        let matched = self
            .in_flight
            .iter()
            .position(|step| step.sequence == sequence)?;
        self.in_flight.drain(..matched);
        self.in_flight.pop_front()
    }

    /// True while one of the requests still waiting for an answer carries that
    /// sequence number.
    ///
    /// A refusal on a sequence no request carries is the echo of a refusal
    /// already dealt with: the server drops every request it had after the one
    /// it refused, and answers some of them with a refusal of their own. Acting
    /// on the echo blocks a second tile for a step that was never really
    /// refused.
    pub fn holds_sequence(&self, sequence: u8) -> bool {
        self.in_flight
            .iter()
            .any(|pending| pending.sequence == sequence)
    }

    /// Throws away what one refusal ends on the wire: every request still
    /// waiting for an answer, and the sequence they counted on. The server
    /// drops every request that reached it after the one it refused, so not one
    /// of them is ever going to be answered.
    ///
    /// The tiles those steps were aimed at go back at the head of the route,
    /// oldest first, because not one of those steps happened: they were taken
    /// out of a route the character still has to walk. A turn puts nothing
    /// back, having taken nothing out.
    ///
    /// What becomes of that route is the refusal ladder's to decide: one
    /// refusal is worth a second try at the same step, and another is worth a
    /// new route.
    pub fn refused(&mut self) {
        for request in self.in_flight.drain(..).rev() {
            if !request.turn {
                self.path.push_front(request.arrives_at);
            }
        }
        self.reset_sequence();
    }

    /// Throws away everything one refusal ends: the requests on the wire, the
    /// route they were part of, and the sequence they counted on.
    pub fn reject(&mut self) {
        self.refused();
        self.path.clear();
    }

    /// Holds the walk still for `pause`, so nothing is sent until whatever is
    /// in the way has had time to move.
    pub fn wait(&mut self, now: Instant, pause: Duration) {
        self.next_step_due = Some(now + pause);
    }

    /// Starts the trip to `dest`, and gives back the refused tiles forgotten
    /// because of it.
    ///
    /// A new destination is a new journey. The tiles and crossings the server
    /// refused on the way somewhere else say nothing about this way, and a
    /// minute is a long time to hold a mark made for another walk. Nothing is
    /// forgotten while the destination is the one the trip is already aimed at,
    /// or a walk planned again on every tick would remember nothing at all.
    pub fn begin_trip(&mut self, dest: Point3) -> Vec<Point3> {
        if self.trip_dest.map(|held| same_tile(held, dest)) == Some(true) {
            return Vec::new();
        }
        self.trip_dest = Some(dest);
        self.replans = 0;
        self.clear_waits();
        self.refused_edges.clear();
        self.blocked.clear()
    }

    /// Counts one more route for this trip, and says whether the character may
    /// go on. A trip planned again [`REPLANS_MAX`] times is one nothing is
    /// going to solve, and it ends instead of looping.
    pub fn count_replan(&mut self) -> bool {
        self.replans += 1;
        self.replans <= REPLANS_MAX
    }

    /// Ends the trip: nothing of it is carried into the next one.
    pub fn end_trip(&mut self) {
        self.trip_dest = None;
        self.replans = 0;
        self.clear_waits();
    }

    /// Counts one wait for somebody to move off `cell`, and gives back how
    /// many the character has waited there since the last one that blocked it,
    /// and how many over the whole trip.
    ///
    /// The first count is what says he has waited long enough to treat the cell
    /// as shut, and it starts again once he has. The second is what says the
    /// whole trip is going nowhere, and only a new trip resets it. Waiting at
    /// another cell starts the first count again: he has moved on.
    pub fn waited_at(&mut self, cell: Point3) -> (u32, u32) {
        if self.wait_cell.map(|held| same_tile(held, cell)) != Some(true) {
            self.wait_cell = Some(cell);
            self.waits = 0;
        }
        self.waits += 1;
        self.waits_this_trip += 1;
        (self.waits, self.waits_this_trip)
    }

    /// Forgets the waits at one cell, which the character has stopped waiting
    /// at because he has just blocked it.
    pub fn stop_waiting(&mut self) {
        self.wait_cell = None;
        self.waits = 0;
    }

    fn clear_waits(&mut self) {
        self.stop_waiting();
        self.waits_this_trip = 0;
    }

    pub fn set_fastwalk(&mut self, keys: [u32; FASTWALK_SLOTS]) {
        self.fastwalk = keys;
    }

    pub fn clear_in_flight(&mut self) {
        self.in_flight.clear();
    }

    /// True while the character is in the middle of a walk: a step waits for
    /// its answer, or more of the route is still queued. A route planned while
    /// this holds must start from [`Movement::stepping_from`], because the
    /// tile he is reported on is one he has already asked to leave.
    pub fn walking(&self) -> bool {
        !self.in_flight.is_empty() || !self.path.is_empty()
    }

    /// Gives up on a step the server never answered inside
    /// [`STEP_ACK_TIMEOUT`], and says whether one was given up.
    ///
    /// The oldest step is the one on the clock, because the server answers in
    /// order: while it is unanswered, no step sent after it can be answered
    /// either. So every step waiting for an answer is lost with it, and the
    /// rest of the route goes too, because that route carried on from tiles
    /// those steps never reached. The caller asks the server where the
    /// character is, and the next tick plans again from the answer.
    pub fn expire_stale(&mut self, now: Instant) -> bool {
        let stale = self
            .in_flight
            .front()
            .is_some_and(|p| now.duration_since(p.sent_at) >= STEP_ACK_TIMEOUT);
        if stale {
            self.in_flight.clear();
            self.path.clear();
        }
        stale
    }

    pub fn set_goal(&mut self, goal: Point3) {
        self.goal = Some(goal);
        self.path.clear();
    }

    /// Stops queuing new steps, and keeps the steps already on the wire so
    /// their answers still land and still move the character.
    pub fn hold(&mut self) {
        self.goal = None;
        self.path.clear();
    }

    /// Gives the whole walk up: the character has been told to do something
    /// else. The steps already on the wire keep their place, because the
    /// server is going to answer them and those answers are the tiles he ends
    /// up on.
    pub fn clear(&mut self) {
        self.hold();
        self.run_override = None;
        self.end_trip();
    }

    pub fn set_path(&mut self, steps: Vec<Point3>, goal: Point3) {
        self.path = steps.into();
        self.goal = Some(goal);
    }

    /// The next step of the queued route, aimed from `from` and left on the
    /// route.
    ///
    /// Every tile of the route carries the height the route worked out for
    /// it, which is the height the character reaches by stepping onto it, and
    /// that tile is handed on whole. Taking only the direction and working the
    /// tile out again from `from` is what carried the old height onto new
    /// ground and left a walker's recorded height fixed while he climbed.
    ///
    /// A tile the character already stands on, and one too far away to reach
    /// in one step, are both dropped: the route has run on past him, or it was
    /// built for a tile he is no longer on.
    ///
    /// The step stays on the route because the direction of it decides what
    /// goes on the wire: a step in a direction he does not face turns him and
    /// moves him nowhere, and the route must still hold that step for the
    /// request after the turn.
    pub fn peek_next_step(&mut self, from: Point3) -> Option<NextStep> {
        while let Some(next) = self.path.front().copied() {
            let dx = next.x as i32 - from.x as i32;
            let dy = next.y as i32 - from.y as i32;
            if same_tile(next, from) || dx.abs() > 1 || dy.abs() > 1 {
                self.path.pop_front();
                continue;
            }
            return Some(NextStep {
                arrives_at: next,
                direction: Direction::from_delta(dx, dy),
            });
        }
        None
    }

    /// The next step of the queued route, taken off it. Call this only for a
    /// step that really goes out as a step: a turn takes no tile out of a
    /// route.
    pub fn pop_next_step(&mut self, from: Point3) -> Option<NextStep> {
        let step = self.peek_next_step(from)?;
        self.path.pop_front();
        Some(step)
    }

    /// True when the character stands on the tile he was walking to and no
    /// step of that walk is still owed an answer.
    pub fn arrived(&self, at: Point3) -> bool {
        self.goal.map(|g| same_tile(g, at)).unwrap_or(false) && !self.walking()
    }
}

/// The share of his stamina a person keeps back to run on: he runs while he
/// still has a quarter of it, and walks once he is below that.
const RUN_STAMINA_SHARE: u32 = 4;

/// True while the character has the stamina to run. A character with none left
/// walks, whatever reason he has to hurry.
pub fn can_run(stam: u16, stam_max: u16) -> bool {
    stam_max == 0 || u32::from(stam) * RUN_STAMINA_SHARE >= u32::from(stam_max)
}

/// True while the character rides.
///
/// The item the server puts on [`MOUNT_LAYER`] is the only word this client
/// gets that he is on a mount: no packet says so in words, and the body he
/// wears does not change. A mount steps twice as fast as the person on it, so
/// this is what [`Movement::next_interval`] is set from.
pub fn is_mounted(equipment: &[EquipItem]) -> bool {
    equipment.iter().any(|worn| worn.layer == MOUNT_LAYER)
}

pub fn should_run(in_town: bool, danger: bool, late: bool, stam: u16, stam_max: u16) -> bool {
    if !can_run(stam, stam_max) {
        return false;
    }
    if danger || late {
        return true;
    }
    !in_town
}

/// The column a body fills, in height units.
///
/// This is the window a door has to stand in to be one the character can reach:
/// the higher of the two grounds, his own and the door's, and one body height
/// up from there. It is the same window the reference client uses. A tighter
/// one misses a door at the top of a stair, where the tile in front of the door
/// stands a course or two below the door's own tile; the two floors of the New
/// Haven inn stand 20 apart, which is still outside it, so a door over his head
/// is still one he can neither reach nor walk through.
pub const DOOR_BODY_COLUMN: i16 = 16;
/// How many clicks on one door the server may leave unanswered before the
/// character treats it as locked. More than this only makes him stand in the
/// doorway.
pub const DOOR_OPEN_ATTEMPTS_MAX: u32 = 3;
/// How long the character waits for the server to answer a click. The answer
/// is one item packet, which comes back inside a round trip, so two walking
/// steps is long enough for a slow shard and short enough that a locked door
/// does not hold him.
pub const DOOR_ANSWER_TIMEOUT: Duration = Duration::from_millis(STEP_WALK_MS * DOOR_ANSWER_STEPS);
/// How many walking steps' worth of time the server has to answer a click.
const DOOR_ANSWER_STEPS: u64 = 2;
/// How long the character has to reach the tile in front of a door before he
/// gives that attempt up. A door he cannot reach in this many steps is one
/// another mobile stands in the way of, or one he can no longer reach at all.
pub const DOOR_REACH_TIMEOUT: Duration = Duration::from_millis(STEP_WALK_MS * DOOR_REACH_STEPS);
/// How many walking steps of room the character is given to reach a door.
const DOOR_REACH_STEPS: u64 = 8;

/// The four tiles a person opens a door from.
///
/// He never opens one from a corner: Ultima Online refuses a diagonal step
/// when either orthogonal neighbour of it is blocked, and a leaf that swings
/// open can land on the corner tile itself and shut the character in.
pub const ORTHOGONAL_DIRS: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// The door that stands between the character and where he is going, the tile
/// he must stand on to open it, and the walk that gets him there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoorApproach {
    pub door: DoorItem,
    pub stand_on: Point3,
    pub steps: Vec<Point3>,
}

/// True while two heights stand in one body column: measured from the higher
/// of the two, the lower is still inside [`DOOR_BODY_COLUMN`] of it. That is
/// what makes them the same floor of a building.
fn on_same_floor(a: i8, b: i8) -> bool {
    (i16::from(a) - i16::from(b)).abs() < DOOR_BODY_COLUMN
}

/// The door standing on one tile of the floor a person at `from_z` is on. A
/// door over his head is one he can neither reach nor walk through.
pub fn door_on_tile(doors: &[DoorItem], x: u16, y: u16, from_z: i8) -> Option<DoorItem> {
    doors
        .iter()
        .find(|door| {
            door.location.x == x && door.location.y == y && on_same_floor(from_z, door.location.z)
        })
        .copied()
}

/// The obstacles of the search that names the door in the way: none at all,
/// of either kind. Read the note on [`door_in_the_way`] before you give this
/// search a list.
const NO_BLOCKERS: Obstacles = Obstacles::NONE;

/// The door a walk to `dest` must pass, and the tile in front of it.
///
/// Two searches run here, and they block on different things on purpose. Do
/// not give them the same list.
///
/// The first search names the door. A route planned around the door items
/// stops in front of a shut door, so the route that names the door in the way
/// is the one planned with the doors left out: the first tile it enters that
/// holds a door on the character's own floor is the door to open. Nothing else
/// blocks that search, and people least of all. A person moves off a tile in a
/// moment; a door stays shut until somebody opens it. A tile the server
/// refused is kept out of it for the same reason and one more: a shut door is
/// exactly what refuses a step, so a search that blocked on the refused tile
/// would hide the very door the refusal was about, and the character would
/// walk around a door he could have opened. This is the failure
/// measured on a live shard: a character inside the New Haven inn was told to
/// walk outside, another player stood in the corridor one tile wide that is
/// the only way out, a search that blocked on that player found no route, and
/// the character was told there was no path instead of being sent to the door.
///
/// The second search is the walk the character really makes, to the tile in
/// front of that door. It blocks on `avoid` and on every door: `avoid` holds
/// the people standing on that walk and the tiles the server has refused him,
/// each of which he must go around, and he must never run through another shut
/// door to reach this one. A door goes in with the people and not with the
/// tiles proven shut, because a door is opened: the tile in front of one door
/// may well be the tile another door stands on, and a walk that ends there is
/// a walk to a door to open. The route finder alone decides what he can reach:
/// it already knows that Ultima Online refuses a diagonal step past a blocked
/// corner.
pub fn door_in_the_way<M: TileQuery + ?Sized>(
    map: &M,
    from: Point3,
    dest: Point3,
    avoid: &Obstacles,
    doors: &[DoorItem],
) -> Option<DoorApproach> {
    let through = pathfind(map, from, dest, &NO_BLOCKERS).ok()?;
    // Each step carries the height the character stands at once he reaches
    // that tile, and that is the floor the door on it must be on. His height
    // where he started names the wrong floor for every step of a walk that
    // climbs, so a door at the top of a stair is one he never sees.
    let door = through
        .steps
        .iter()
        .find_map(|step| door_on_tile(doors, step.x, step.y, step.z))?;
    // The second search, and the only one the people and the refused tiles
    // belong in.
    let mut soft = avoid.soft.to_vec();
    soft.extend(doors.iter().map(|d| d.location));
    let blocked = Obstacles {
        soft: &soft,
        hard: avoid.hard,
        moves: avoid.moves,
    };
    let (stand_on, steps) = ORTHOGONAL_DIRS
        .iter()
        .filter_map(|&dir| door.location.neighbour(dir))
        // These four tiles touch the door, so a second door on one of them is
        // one on the door's own floor, not on the floor the character started
        // from.
        .filter(|tile| door_on_tile(doors, tile.x, tile.y, tile.z).is_none())
        .filter_map(|tile| {
            let path = pathfind(map, from, tile, &blocked).ok()?;
            let steps: Vec<Point3> = path
                .steps
                .iter()
                .map(|s| Point3::new(s.x, s.y, s.z))
                .collect();
            // An empty walk means the character already stands in front of the
            // door, so the tile he stands on is the one he opens it from.
            Some((steps.last().copied().unwrap_or(from), steps))
        })
        .min_by_key(|(_, steps)| steps.len())?;
    Some(DoorApproach {
        door,
        stand_on,
        steps,
    })
}

/// The way a character at `from` looks to see the thing at `to`. `None` when
/// both stand on the same tile, where no turn means anything.
pub fn facing_toward(from: Point3, to: Point3) -> Option<Direction> {
    if same_tile(from, to) {
        return None;
    }
    Some(Direction::from_delta(
        i32::from(to.x) - i32::from(from.x),
        i32::from(to.y) - i32::from(from.y),
    ))
}

/// What the character does about the door in his way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoorPlan {
    /// Walk to the tile in front of the door, then click it once.
    Approach(DoorApproach),
    /// An attempt is already underway: he is walking to a door, or waiting for
    /// the leaf to swing. Nothing new is started, because the second click on
    /// a door is the one that shuts it again.
    Underway,
    /// The door answered a click already and still stands in the way. Clicking
    /// it again would only shut it.
    Opened(DoorItem),
    /// [`DOOR_OPEN_ATTEMPTS_MAX`] clicks and no answer: locked, as far as the
    /// character can tell.
    Locked(DoorItem),
}

/// A door the character is opening.
#[derive(Clone, Copy, Debug)]
struct PendingDoor {
    door: DoorItem,
    stand_on: Point3,
    clicked: bool,
    /// When the attempt is given up: time to reach the door while he walks to
    /// it, and time to answer once he has clicked.
    deadline: Instant,
}

/// Opens the doors that stand between the character and where he is going.
///
/// A door is a server item that blocks the tile it stands on. The server
/// answers a click by sending that item again with a new graphic, a new tile,
/// or both, and one more click shuts it. Which of the two states a door is in
/// cannot be read from its graphic, because door graphics come in open and
/// shut pairs that differ from one door to the next, and reading them wrong is
/// what makes a character shut the door he has just opened. So he clicks once
/// and watches the serial: any change on it is the door answering him.
#[derive(Debug, Default)]
pub struct DoorOpener {
    /// How many clicks on one door the server never answered.
    failures: HashMap<Serial, u32>,
    /// Each door as it looked when the character last clicked it. A door that
    /// looks different now has answered that click.
    clicked: HashMap<Serial, DoorItem>,
    pending: Option<PendingDoor>,
}

impl DoorOpener {
    /// Starts an attempt on the door in the way.
    pub fn plan(&mut self, approach: DoorApproach, now: Instant) -> DoorPlan {
        if self.pending.is_some() {
            return DoorPlan::Underway;
        }
        if self.answered_a_click(approach.door) {
            return DoorPlan::Opened(approach.door);
        }
        if self.failed_attempts(approach.door.serial) >= DOOR_OPEN_ATTEMPTS_MAX {
            return DoorPlan::Locked(approach.door);
        }
        self.pending = Some(PendingDoor {
            door: approach.door,
            stand_on: approach.stand_on,
            clicked: false,
            deadline: now + DOOR_REACH_TIMEOUT,
        });
        DoorPlan::Approach(approach)
    }

    /// The door the character is due to click, without spending the attempt on
    /// it. The caller reads this to aim: the macro he sends names no door, so
    /// he must face the door tile before it goes out, and the turn that aims
    /// him takes a tick of its own.
    pub fn due_at(&self, at: Point3) -> Option<DoorItem> {
        let pending = self.pending?;
        if pending.clicked || !same_tile(pending.stand_on, at) {
            return None;
        }
        Some(pending.door)
    }

    /// The door to face and click now. A person opens a door from the tile in
    /// front of it, so nothing is due until the character stands there, and
    /// one attempt sends one click.
    ///
    /// This spends the attempt, so it belongs where the click really leaves
    /// and nowhere earlier: an attempt spent on a click still waiting to be
    /// aimed is an attempt the character never made.
    pub fn due(&mut self, at: Point3, now: Instant) -> Option<DoorItem> {
        let door = self.due_at(at)?;
        let pending = self.pending.as_mut()?;
        pending.clicked = true;
        pending.deadline = now + DOOR_ANSWER_TIMEOUT;
        self.clicked.insert(door.serial, door);
        Some(door)
    }

    /// True while the character walks to a door or waits for it to answer.
    /// Nothing plans a route in the meantime, or he walks away from the door
    /// he must pass.
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    /// The door on this serial changed. A door that swings is not stuck,
    /// whoever swung it, so the clicks that failed on it are forgotten. True
    /// when it is the door the character is waiting for: his click was
    /// answered and the attempt is over.
    pub fn answered(&mut self, serial: Serial) -> bool {
        self.failures.remove(&serial);
        if self.pending.map(|p| p.door.serial) != Some(serial) {
            return false;
        }
        self.pending = None;
        true
    }

    /// An attempt the character has run out of time on. A click the server
    /// never answered is one failure against that door, so a door that never
    /// answers is given up on in the end; a door he never reached in time
    /// costs him nothing, because that door was never tried.
    pub fn expired(&mut self, now: Instant) -> Option<DoorItem> {
        let pending = self.pending?;
        if now < pending.deadline {
            return None;
        }
        self.pending = None;
        if pending.clicked {
            *self.failures.entry(pending.door.serial).or_insert(0) += 1;
        }
        Some(pending.door)
    }

    /// Gives the attempt up: the character has been told to do something else.
    pub fn give_up(&mut self) {
        self.pending = None;
    }

    /// True when the door no longer looks the way it did when it was clicked.
    /// It has swung since, so another click would only swing it back.
    fn answered_a_click(&self, door: DoorItem) -> bool {
        self.clicked
            .get(&door.serial)
            .is_some_and(|clicked| *clicked != door)
    }

    fn failed_attempts(&self, serial: Serial) -> u32 {
        self.failures.get(&serial).copied().unwrap_or(0)
    }
}

/// What a follower does on one tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowDecision {
    /// Stand still: the target is close enough, or nothing has changed since
    /// the last path search failed.
    Hold,
    /// Walk the path that is already queued.
    Continue,
    /// Build a new path to a tile beside the target.
    Repath,
}

/// What a follower carries between ticks so that it neither searches for a
/// path every tick nor starts again the moment it steps inside the band.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FollowState {
    /// True while the follower closes a gap that opened past the far distance.
    closing: bool,
    /// The tile the target stood on when the queued path was built.
    pathed_from: Option<Point3>,
    /// True when the last path search found no way to the target.
    unreachable: bool,
}

impl FollowState {
    /// What the follower does about the gap it can see.
    ///
    /// `self_at` is the tile the follower ends the steps it has already sent
    /// on, not the tile it is reported on: a follower that reads the gap from
    /// a tile it has already asked to leave keeps closing a gap it has
    /// already closed, and walks a circle round its target.
    pub fn decide(
        &mut self,
        self_at: Point3,
        target_at: Point3,
        path_active: bool,
    ) -> FollowDecision {
        if self_at.chebyshev(target_at) <= FOLLOW_NEAR_DISTANCE {
            *self = Self::default();
            return FollowDecision::Hold;
        }
        if !self.closing {
            if self_at.chebyshev(target_at) <= FOLLOW_FAR_DISTANCE {
                return FollowDecision::Hold;
            }
            self.closing = true;
        }
        let target_moved = match self.pathed_from {
            Some(from) => target_at.chebyshev(from) >= FOLLOW_REPATH_DISTANCE,
            None => true,
        };
        let path_done = !path_active && !self.unreachable;
        if target_moved || path_done {
            self.pathed_from = Some(target_at);
            return FollowDecision::Repath;
        }
        if path_active {
            FollowDecision::Continue
        } else {
            FollowDecision::Hold
        }
    }

    /// Records whether the path the last `Repath` asked for was built. A
    /// follower that has nowhere to go holds until the target moves.
    pub fn mark_path(&mut self, built: bool) {
        self.unreachable = !built;
    }
}

/// The walk a follower makes to keep pace with its target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FollowPlan {
    /// A free tile beside the target, never the tile the target stands on.
    pub dest: Point3,
    pub steps: Vec<Point3>,
    /// The pace of the target. [`follow_pace`] turns it into the pace the
    /// follower keeps, which is faster while it is catching up.
    pub running: bool,
}

/// The pace a follower keeps.
///
/// It runs with a runner, because a follower that walks behind a running
/// player loses a tile on every step of his and never wins one back. It runs
/// while it is further behind than the band allows for the same reason: two
/// people walking at the same pace stay exactly as far apart as they were, so
/// a follower that only ever matches the pace keeps every tile it has already
/// lost. It walks the last tile in, at the pace of the person it is joining,
/// which is what a person does who has caught up.
///
/// Stamina decides in the end, as it does for every other run.
pub fn follow_pace(
    self_at: Point3,
    target_at: Point3,
    target_running: bool,
    stam: u16,
    stam_max: u16,
) -> bool {
    if target_running {
        return true;
    }
    self_at.chebyshev(target_at) > FOLLOW_FAR_DISTANCE && can_run(stam, stam_max)
}

/// Picks the free tile beside the target that costs the follower the least
/// walking, and the path to it. `None` means no tile beside the target can be
/// reached.
///
/// The eight tiles are tried in turn, nearest first, and the first one with a
/// walk to it is the one taken. A tile a mobile stands on is passed over, and
/// so is a tile the server has already refused: the refused one is passed over
/// twice, once here and once by the route finder, which refuses a walk that
/// ends on it. That is what stops a follower asking for the tile its own
/// memory says it cannot enter.
pub fn follow_plan<M: TileQuery + ?Sized>(
    map: &M,
    self_at: Point3,
    target_at: Point3,
    target_running: bool,
    obstacles: &Obstacles,
) -> Option<FollowPlan> {
    let mut slots: Vec<Point3> = ADJACENT_DIRS
        .iter()
        .filter_map(|&dir| target_at.neighbour(dir))
        // Every one of these tiles touches the target, so each is read from
        // the target's height and never from the follower's. Read from the
        // follower's height, a tile beside a target at the top of a ramp
        // answers for a floor the follower is not on. All eight are refused
        // that way, and a follower at the foot of the ramp reports there is
        // no way to a tile beside the target it can see above it. The one
        // query gives the answer and the height the tile stands at, which is
        // the height the walk to it ends on.
        .filter_map(|slot| {
            if !map.in_bounds(slot.x, slot.y) {
                return None;
            }
            let tile = map.tile_from(target_at.z, slot.x, slot.y);
            tile.walkable()
                .then(|| Point3::new(slot.x, slot.y, tile.z))
        })
        .filter(|slot| same_tile(*slot, self_at) || !obstacles.blocks(*slot))
        .collect();
    slots.sort_by_key(|slot| self_at.chebyshev(*slot));
    slots.into_iter().find_map(|dest| {
        let path = pathfind(map, self_at, dest, obstacles).ok()?;
        Some(FollowPlan {
            dest,
            steps: path
                .steps
                .iter()
                .map(|s| Point3::new(s.x, s.y, s.z))
                .collect(),
            running: target_running,
        })
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use uoterm_nav::client_data_dir_from_env;
    use uoterm_nav::MockMap;

    const GRID: u16 = 32;
    const ROW: u16 = 8;
    const ME_X: u16 = 4;
    const TARGET_X: u16 = 20;
    const WALL_X: u16 = 12;
    const GAP_Y: u16 = 2;
    /// The move request packet id.
    const MOVE_REQ_ID: u8 = 0x02;
    /// The two paces a request goes out at.
    const WALKING: bool = false;
    const RUNNING: bool = true;
    /// The slowest a walking step can be, jitter and all. Wait this long for
    /// each step and the pace of a person never holds the next one back.
    const SLOWEST_WALK: Duration =
        Duration::from_millis(STEP_WALK_MS + (STEP_WALK_MS * JITTER_PCT as u64) / 100);

    /// The mobiles standing on these tiles, and nothing proven shut. A person
    /// steps off his tile in a moment, which is the whole of what makes him
    /// soft.
    fn standing_on(mobiles: &[Point3]) -> Obstacles<'_> {
        Obstacles {
            soft: mobiles,
            hard: &[],
            moves: &[],
        }
    }

    /// A step across level ground: the tile beside him, at the height he is
    /// already at.
    ///
    /// The tests below walk the flat, where the height a step arrives at is
    /// the height it left, and what they measure is the sequence, the pace and
    /// the cap on the wire. The heights a climb reaches are read from a map,
    /// in the tests that climb.
    fn step_across_level_ground(from: Point3, direction: Direction) -> NextStep {
        NextStep {
            arrives_at: from
                .neighbour(direction)
                .expect("the tile beside him is on the grid"),
            direction,
        }
    }

    /// The step the character has sent and the server has not answered yet.
    fn in_flight(m: &Movement) -> PendingStep {
        m.in_flight
            .front()
            .cloned()
            .expect("a step is waiting for its answer")
    }

    #[test]
    fn ack_matches_sequence() {
        let mut m = Movement::default();
        let now = Instant::now();
        assert!(m.ready(now));
        let pkt = m.build_step(
            step_across_level_ground(Point3::new(10, 10, 0), Direction::East),
            false,
            now,
        );
        assert_eq!(pkt[0], MOVE_REQ_ID);
        let sent = in_flight(&m);
        assert_eq!(sent.sequence, SEQ_FIRST);
        assert_eq!(pkt[MOVE_REQ_SEQ_INDEX], SEQ_FIRST);
        assert_eq!(
            m.ack(SEQ_LAST).map(|p| p.sequence),
            None,
            "a sequence the character never sent confirms nothing"
        );
        assert_eq!(m.ack(sent.sequence).unwrap().arrives_at.x, 11);
        assert!(m.in_flight.is_empty());
    }

    /// How many requests the wire holds. The server keeps a ring of five slots
    /// for the moves one client has not had an answer to, and the reference
    /// client puts four on the wire: one slot of headroom, so a request that
    /// crosses an answer never fills the ring.
    ///
    /// Three, which this client used before, is one step short of the wire the
    /// reference keeps and one step of ground given away on a long run.
    #[test]
    fn the_wire_holds_one_request_fewer_than_the_server_ring() {
        assert_eq!(IN_FLIGHT_MAX, 4);
        assert_eq!(
            IN_FLIGHT_MAX + SERVER_RING_HEADROOM,
            SERVER_MOVE_RING_SLOTS,
            "and it leaves the server its slot of headroom"
        );
    }

    /// The rule this client walks by: the character asks, and only the answer
    /// moves him. He may have [`IN_FLIGHT_MAX`] questions out at once, which
    /// is what keeps him level with a runner, and one more than that he will
    /// not ask until an answer comes back.
    #[test]
    fn the_wire_holds_several_steps_and_no_more_than_the_cap() {
        const {
            assert!(
                IN_FLIGHT_MAX > 1,
                "one step at a time makes the round trip the speed limit"
            )
        };
        let mut m = Movement::default();
        let now = Instant::now();
        // Far enough past the pace of a person that only an unanswered step
        // can be holding him back: every one of these steps is built at `now`,
        // and each of them puts the one after it a step of the cadence further
        // out.
        let paced = now + SLOWEST_WALK * IN_FLIGHT_MAX as u32;
        for sent in 1..=IN_FLIGHT_MAX {
            assert!(m.ready(paced), "{sent} steps out is inside the cap");
            let from = m.stepping_from(AT_THE_INN_DOOR_CORNER);
            m.build_step(step_across_level_ground(from, Direction::East), false, now);
            assert_eq!(m.in_flight.len(), sent);
        }
        assert!(
            !m.ready(paced),
            "the cap is full, so the next step waits for an answer"
        );
        let newest = m.in_flight.back().expect("the last step out").arrives_at;
        let oldest = in_flight(&m).sequence;
        m.ack(oldest);
        assert!(m.ready(paced), "one answer makes room for one more step");
        assert_eq!(
            m.stepping_from(AT_THE_INN_DOOR_CORNER),
            newest,
            "and the next step is aimed from the far end of the steps still out"
        );
    }

    /// The cap and the timeout must not fight. Filling the wire at the slowest
    /// pace a person walks at has to take less time than the server is given
    /// to answer the first of those steps, or a character walking a straight
    /// line gives up on a shard that is answering him.
    #[test]
    fn the_wire_fills_before_the_server_runs_out_of_time_to_answer() {
        let fills_in = SLOWEST_WALK * (IN_FLIGHT_MAX as u32 - 1);
        assert!(
            fills_in < STEP_ACK_TIMEOUT,
            "a full wire takes {fills_in:?} to fill and the answer is due in {STEP_ACK_TIMEOUT:?}"
        );
    }

    /// Every step on the wire is aimed at the tile after the one before it, so
    /// the answers walk the character along one tile at a time, in order, and
    /// he is never reported ahead of the server.
    #[test]
    fn several_steps_on_the_wire_are_confirmed_one_tile_at_a_time_in_order() {
        let mut m = Movement::default();
        let now = Instant::now();
        let start = AT_THE_INN_DOOR_CORNER;
        let mut sequences = Vec::new();
        for _ in 0..IN_FLIGHT_MAX {
            let from = m.stepping_from(start);
            m.build_step(step_across_level_ground(from, Direction::East), false, now);
            sequences.push(m.in_flight.back().expect("the step just sent").sequence);
        }
        for (i, sequence) in sequences.iter().enumerate() {
            let step = m.ack(*sequence).expect("the answer to a step that is out");
            assert_eq!(
                step.arrives_at.x,
                start.x + i as u16 + 1,
                "answer {i} moves him exactly one tile on"
            );
            assert_eq!(
                m.in_flight.len(),
                IN_FLIGHT_MAX - i - 1,
                "and takes exactly one step off the wire"
            );
        }
    }

    /// One refusal ends every step on the wire. The server threw them all
    /// away, so holding any of them would move the character onto a tile the
    /// shard never let him have.
    #[test]
    fn a_refusal_throws_away_every_step_on_the_wire() {
        let mut m = Movement::default();
        let now = Instant::now();
        let start = AT_THE_INN_DOOR_CORNER;
        for _ in 0..IN_FLIGHT_MAX {
            let from = m.stepping_from(start);
            m.build_step(step_across_level_ground(from, Direction::East), false, now);
        }
        m.set_path(vec![IN_FRONT_OF_THE_INN_DOOR, INN_DOORWAY], INN_DOORWAY);
        assert_eq!(m.in_flight.len(), IN_FLIGHT_MAX);
        assert_eq!(
            m.refused_direction(),
            Some(Direction::East),
            "the oldest step out is the one the server is refusing"
        );
        m.reject();
        assert!(m.in_flight.is_empty(), "every step out is thrown away");
        assert!(m.path.is_empty(), "and the route they were part of");
        assert_eq!(m.sequence, SEQ_FIRST, "and the sequence starts again");
        assert_eq!(
            m.stepping_from(start),
            start,
            "so the next step is aimed from the tile the server says he is on"
        );
    }

    /// A step the server never answers must not wedge the character, however
    /// many steps are on the wire behind it. The oldest one is the one on the
    /// clock, because nothing sent after it can be answered before it; when
    /// its time is up every step out is given up on, the route they were part
    /// of goes with them, and he is free to walk again from wherever the
    /// server says he stands.
    #[test]
    fn a_step_the_server_never_answers_is_given_up_on() {
        let mut m = Movement::default();
        let now = Instant::now();
        m.set_path(vec![IN_FRONT_OF_THE_INN_DOOR, INN_DOORWAY], INN_DOORWAY);
        let start = AT_THE_INN_DOOR_CORNER;
        let pace = Duration::from_millis(STEP_WALK_MS);
        for i in 0..IN_FLIGHT_MAX {
            let from = m.stepping_from(start);
            m.build_step(
                step_across_level_ground(from, Direction::East),
                false,
                now + pace * i as u32,
            );
        }
        assert!(
            !m.expire_stale(now + STEP_ACK_TIMEOUT - ONE_MILLISECOND),
            "the server is given its time to answer"
        );
        assert!(m.expire_stale(now + STEP_ACK_TIMEOUT));
        assert!(
            m.in_flight.is_empty(),
            "every step waiting on the lost one is thrown away too"
        );
        assert!(
            m.path.is_empty(),
            "and so is the route that carried on from the tiles they never reached"
        );
        assert!(!m.walking());
        assert!(
            m.ready(now + STEP_ACK_TIMEOUT + pace * IN_FLIGHT_MAX as u32),
            "so nothing holds him still but the pace of a person"
        );
        assert_eq!(
            m.stepping_from(start),
            start,
            "and he aims again from the tile the server put him on"
        );
    }

    #[test]
    fn first_step_sequence_is_zero() {
        let mut m = Movement::default();
        let now = Instant::now();
        let pkt = m.build_step(
            step_across_level_ground(Point3::new(10, 10, 0), Direction::East),
            false,
            now,
        );
        let first = in_flight(&m);
        assert_eq!(first.sequence, SEQ_FIRST);
        assert_eq!(pkt[MOVE_REQ_SEQ_INDEX], SEQ_FIRST);
        m.ack(first.sequence);
        m.build_step(
            step_across_level_ground(Point3::new(11, 10, 0), Direction::East),
            false,
            now,
        );
        assert_eq!(in_flight(&m).sequence, SEQ_AFTER_WRAP);
    }

    #[test]
    fn sequence_wraps_from_255_to_1() {
        let mut m = Movement {
            sequence: SEQ_LAST,
            ..Movement::default()
        };
        let now = Instant::now();
        m.build_step(
            step_across_level_ground(Point3::new(10, 10, 0), Direction::East),
            false,
            now,
        );
        let last = in_flight(&m);
        assert_eq!(last.sequence, SEQ_LAST);
        m.ack(last.sequence);
        let pkt = m.build_step(
            step_across_level_ground(Point3::new(11, 10, 0), Direction::East),
            false,
            now,
        );
        let wrapped = in_flight(&m);
        assert_eq!(wrapped.sequence, SEQ_AFTER_WRAP);
        assert_eq!(pkt[MOVE_REQ_SEQ_INDEX], SEQ_AFTER_WRAP);
        assert_ne!(wrapped.sequence, SEQ_FIRST);
    }

    #[test]
    fn next_sequence_starts_at_zero_then_skips_zero_on_wrap() {
        let mut m = Movement::default();
        assert_eq!(m.next_sequence(), SEQ_FIRST);
        assert_eq!(m.next_sequence(), SEQ_AFTER_WRAP);
        m = Movement {
            sequence: SEQ_LAST,
            ..Movement::default()
        };
        assert_eq!(m.next_sequence(), SEQ_LAST);
        assert_eq!(m.next_sequence(), SEQ_AFTER_WRAP);
        m.reset_sequence();
        assert_eq!(m.next_sequence(), SEQ_FIRST);
    }

    #[test]
    fn reject_clears_path() {
        let mut m = Movement::default();
        m.set_path(
            vec![Point3::new(1, 0, 0), Point3::new(2, 0, 0)],
            Point3::new(2, 0, 0),
        );
        m.reject();
        assert!(m.path.is_empty());
        assert!(m.in_flight.is_empty());
        assert_eq!(m.sequence, SEQ_FIRST);
    }

    #[test]
    fn reject_resets_next_sequence_to_zero() {
        let mut m = Movement::default();
        let now = Instant::now();
        m.build_step(
            step_across_level_ground(Point3::new(10, 10, 0), Direction::East),
            false,
            now,
        );
        let first = in_flight(&m);
        assert_eq!(first.sequence, SEQ_FIRST);
        m.ack(first.sequence);
        m.build_step(
            step_across_level_ground(Point3::new(11, 10, 0), Direction::East),
            false,
            now,
        );
        assert_eq!(in_flight(&m).sequence, SEQ_AFTER_WRAP);
        m.reject();
        assert!(m.in_flight.is_empty());
        let pkt = m.build_step(
            step_across_level_ground(Point3::new(10, 10, 0), Direction::East),
            false,
            now,
        );
        assert_eq!(in_flight(&m).sequence, SEQ_FIRST);
        assert_eq!(pkt[MOVE_REQ_SEQ_INDEX], SEQ_FIRST);
    }

    /// A route one tile long keeps its one step, and that step is handed on
    /// whole: the tile the route named, at the height the route worked out
    /// for it.
    #[test]
    fn adjacent_goal_keeps_one_step() {
        const UP_ONE_STAIR: i8 = uoterm_nav::STEP_HEIGHT;
        let mut m = Movement::default();
        let uphill = Point3::new(11, 10, UP_ONE_STAIR);
        m.set_path(vec![uphill], uphill);
        assert_eq!(m.path.len(), 1);
        assert_eq!(
            m.pop_next_step(Point3::new(10, 10, 0)),
            Some(NextStep {
                arrives_at: uphill,
                direction: Direction::East,
            }),
            "the height the route worked out comes with the step, not the one he stands at"
        );
    }

    /// The fastwalk keys of a full stack, in the order the server sends them.
    const FASTWALK_STACK: [u32; FASTWALK_SLOTS] = [1, 2, 3, 4, 5, 6];
    /// Where the fastwalk key sits in a move request.
    const MOVE_REQ_KEY_INDEX: usize = 3;

    /// The key one move request carries.
    fn fastwalk_key_of(pkt: &[u8]) -> u32 {
        u32::from_be_bytes([
            pkt[MOVE_REQ_KEY_INDEX],
            pkt[MOVE_REQ_KEY_INDEX + 1],
            pkt[MOVE_REQ_KEY_INDEX + 2],
            pkt[MOVE_REQ_KEY_INDEX + 3],
        ])
    }

    /// A key is spent once and never sent again. Reading the stack round and
    /// round put key one back on the wire on the seventh move, which is what a
    /// shard reads as a client walking faster than a person can.
    #[test]
    fn a_fastwalk_key_is_spent_once_and_the_stack_empties() {
        let mut m = Movement::default();
        m.set_fastwalk(FASTWALK_STACK);
        let now = Instant::now();
        let mut at = Point3::new(0, 0, 0);
        let mut sent = Vec::new();
        for _ in 0..FASTWALK_SLOTS {
            let pkt = m.build_step(step_across_level_ground(at, Direction::East), false, now);
            at = at.neighbour(Direction::East).expect("the tile beside him");
            sent.push(fastwalk_key_of(&pkt));
            m.in_flight.clear();
        }
        assert_eq!(sent, FASTWALK_STACK.to_vec(), "each key once, in order");
        let empty = m.build_step(step_across_level_ground(at, Direction::East), false, now);
        assert_eq!(
            fastwalk_key_of(&empty),
            FASTWALK_KEY_EMPTY,
            "and an empty stack sends no key at all, never key one again"
        );
    }

    /// The packet that refills the stack sends one key at a time, and this is
    /// the way in for it.
    #[test]
    fn one_new_fastwalk_key_goes_on_the_stack() {
        const ONE_MORE_KEY: u32 = 77;
        let mut m = Movement::default();
        let now = Instant::now();
        assert!(m.push_fastwalk(ONE_MORE_KEY), "an empty stack has room");
        let pkt = m.build_step(
            step_across_level_ground(Point3::new(0, 0, 0), Direction::East),
            false,
            now,
        );
        assert_eq!(fastwalk_key_of(&pkt), ONE_MORE_KEY);
        m.set_fastwalk(FASTWALK_STACK);
        assert!(
            !m.push_fastwalk(ONE_MORE_KEY),
            "a full stack takes no more: the server cannot owe him more than it holds"
        );
    }

    /// A mount steps twice as fast as the person on it, at a walk and at a run
    /// alike. Without the branch a rider walked at the pace of his own legs and
    /// lost half a tile on every step.
    #[test]
    fn a_mount_steps_at_the_pace_of_a_mount() {
        let mut m = Movement::default();
        let slowest = |base: u64| Duration::from_millis(base + (base * JITTER_PCT as u64) / 100);
        assert!(m.next_interval(RUNNING) <= slowest(STEP_RUN_MS));
        assert!(m.next_interval(WALKING) <= slowest(STEP_WALK_MS));
        m.mounted = true;
        assert!(
            m.next_interval(RUNNING) <= slowest(STEP_MOUNT_RUN_MS),
            "a mount at a run is faster than a person at a run"
        );
        assert!(
            m.next_interval(WALKING) <= slowest(STEP_MOUNT_WALK_MS),
            "and a mount at a walk is faster than a person at a walk"
        );
        assert!(
            m.next_interval(WALKING)
                >= Duration::from_millis(
                    STEP_MOUNT_WALK_MS - (STEP_MOUNT_WALK_MS * JITTER_PCT as u64) / 100
                )
        );
    }

    /// The item on the mount layer is the only word the server gives that the
    /// character rides.
    #[test]
    fn a_rider_is_known_by_the_item_on_the_mount_layer() {
        const A_HORSE: u16 = 0x3E9F;
        let worn = |layer: u8| EquipItem {
            serial: INN_DOOR_SERIAL,
            graphic: A_HORSE,
            layer,
            hue: 0,
        };
        assert!(!is_mounted(&[]));
        assert!(!is_mounted(&[worn(uoterm_protocol::types::LAYER_BACKPACK)]));
        assert!(is_mounted(&[
            worn(uoterm_protocol::types::LAYER_ONE_HANDED),
            worn(MOUNT_LAYER)
        ]));
    }

    #[test]
    fn should_run_uses_u32_stamina_math() {
        assert!(!should_run(false, true, true, 10, 50));
        assert!(should_run(false, true, false, u16::MAX, u16::MAX));
    }

    /// Ground truth measured on a live shard: the door of the New Haven inn
    /// that stopped a follower. The server sends it as an item, and the inn
    /// holds no door static at all. One click moved it to another tile and
    /// changed its graphic; a second click shut it again.
    const INN_DOOR_SERIAL: Serial = Serial(1_073_744_340);
    const INN_DOOR_SHUT_GRAPHIC: u16 = 1701;
    const INN_DOOR_OPEN_GRAPHIC: u16 = 1702;
    const INN_DOORWAY: Point3 = Point3 {
        x: 3506,
        y: 2526,
        z: 27,
    };
    const INN_DOOR_SWUNG_TO: Point3 = Point3 {
        x: 3505,
        y: 2527,
        z: 27,
    };
    /// The tile in front of the inn door: orthogonally north of it.
    const IN_FRONT_OF_THE_INN_DOOR: Point3 = Point3 {
        x: 3506,
        y: 2525,
        z: 27,
    };
    /// Where the follower stood when the doorway refused him: a corner tile,
    /// diagonal from the door, which a person never opens a door from.
    const AT_THE_INN_DOOR_CORNER: Point3 = Point3 {
        x: 3505,
        y: 2525,
        z: 27,
    };
    const ONE_MILLISECOND: Duration = Duration::from_millis(1);

    fn inn_door(graphic: u16, at: Point3) -> DoorItem {
        DoorItem {
            serial: INN_DOOR_SERIAL,
            graphic,
            location: at,
        }
    }

    fn shut_inn_door() -> DoorItem {
        inn_door(INN_DOOR_SHUT_GRAPHIC, INN_DOORWAY)
    }

    /// The walk the character makes to the tile in front of the inn door.
    fn approach(door: DoorItem) -> DoorApproach {
        DoorApproach {
            door,
            stand_on: IN_FRONT_OF_THE_INN_DOOR,
            steps: vec![IN_FRONT_OF_THE_INN_DOOR],
        }
    }

    /// The column the inn door stands in. The way out of the inn is this
    /// column and nothing beside it.
    const INN_CORRIDOR_X: u16 = INN_DOORWAY.x;
    /// The wall that closes the north end of the corridor.
    const INN_CORRIDOR_WALL_Y: u16 = 2520;
    /// A mock map holds every tile from 0,0, so it must reach one column past
    /// the corridor and one row past the tile the character walks to.
    const INN_CORRIDOR_MAP_WIDTH: u16 = INN_CORRIDOR_X + 2;
    const INN_CORRIDOR_MAP_HEIGHT: u16 = OUT_OF_THE_INN.y + 2;
    /// Where the character stands: in the corridor, north of the door.
    const INSIDE_THE_INN: Point3 = Point3 {
        x: INN_CORRIDOR_X,
        y: 2523,
        z: INN_GROUND_Z,
    };
    /// Where the other player stood while the character was told there was no
    /// way out: in the corridor, south of the door.
    const SOMEBODY_IN_THE_CORRIDOR: Point3 = Point3 {
        x: INN_CORRIDOR_X,
        y: 2528,
        z: INN_GROUND_Z,
    };
    /// The tile the character was told to walk to, south of that player.
    const OUT_OF_THE_INN: Point3 = Point3 {
        x: INN_CORRIDOR_X,
        y: 2530,
        z: INN_GROUND_Z,
    };

    /// The way out of the New Haven inn, measured on a live shard: a corridor
    /// exactly one tile wide, straight down the column the inn door stands in,
    /// with the shut door in it and another player standing past the door.
    pub(crate) struct InnCorridor {
        pub(crate) map: MockMap,
        /// Where the character stands, north of the door.
        pub(crate) character: Point3,
        /// The shut door, the one thing between him and the street.
        pub(crate) door: DoorItem,
        /// The tile he must stand on to open that door.
        pub(crate) in_front_of_the_door: Point3,
        /// The other player, in the corridor south of the door.
        pub(crate) somebody_in_the_way: Point3,
        /// Where he was told to walk, south of that player.
        pub(crate) outside: Point3,
    }

    /// The corridor, the door in it, and everybody standing in it.
    pub(crate) fn inn_corridor() -> InnCorridor {
        let mut map = MockMap::new(INN_CORRIDOR_MAP_WIDTH, INN_CORRIDOR_MAP_HEIGHT);
        for y in INN_CORRIDOR_WALL_Y..INN_CORRIDOR_MAP_HEIGHT {
            map.set_block(INN_CORRIDOR_X - 1, y, true);
            map.set_block(INN_CORRIDOR_X + 1, y, true);
            map.set_z(INN_CORRIDOR_X, y, INN_GROUND_Z);
        }
        // The wall closes the north end and the edge of the map closes the
        // south end, so the corridor is all the ground there is to stand on.
        map.set_block(INN_CORRIDOR_X, INN_CORRIDOR_WALL_Y, true);
        InnCorridor {
            map,
            character: INSIDE_THE_INN,
            door: shut_inn_door(),
            in_front_of_the_door: IN_FRONT_OF_THE_INN_DOOR,
            somebody_in_the_way: SOMEBODY_IN_THE_CORRIDOR,
            outside: OUT_OF_THE_INN,
        }
    }

    #[test]
    fn a_refused_step_remembers_the_tile_it_was_aimed_at() {
        let mut m = Movement::default();
        let now = Instant::now();
        m.build_step(
            step_across_level_ground(IN_FRONT_OF_THE_INN_DOOR, Direction::South),
            false,
            now,
        );
        let refused = m.refused_direction().expect("the step is still in flight");
        assert_eq!(refused, Direction::South);
        assert_eq!(
            IN_FRONT_OF_THE_INN_DOOR.neighbour(refused),
            Some(INN_DOORWAY),
            "the refused tile is the one beside the position the server reports"
        );
        m.reject();
        assert_eq!(
            m.refused_direction(),
            None,
            "the refusal empties the step queue"
        );
    }

    /// Ground truth measured on a live shard around the Britain bank in
    /// Felucca, map index 0. In two minutes the server confirmed 627 steps and
    /// refused 89, and 16 routes were abandoned. The refusals were not spread
    /// out: 35 of them came off this one tile, 23 off 1493,1626 and 10 off
    /// 1494,1622. The client map files were read at every one and report open
    /// ground with no impassable static at all; at two of them all eight
    /// neighbouring tiles read as walkable and the character was still
    /// refused. What stands there is a player house, which is built on the
    /// server and is in no client map file.
    const REFUSED_35_TIMES: Point3 = Point3 {
        x: 1438,
        y: 1659,
        z: 10,
    };
    const REFUSED_23_TIMES: Point3 = Point3 {
        x: 1493,
        y: 1626,
        z: 10,
    };

    #[test]
    fn a_refused_tile_is_blocked_for_its_time_and_then_forgotten() {
        let mut blocked = BlockedTiles::default();
        let now = Instant::now();
        assert_eq!(
            blocked.refuse(REFUSED_35_TIMES, now),
            None,
            "nothing is dropped while there is room"
        );
        assert_eq!(
            blocked.tiles(),
            vec![REFUSED_35_TIMES],
            "so every route goes around it"
        );
        assert!(
            blocked
                .expire(now + REFUSED_TILE_MEMORY - ONE_MILLISECOND)
                .is_empty(),
            "it is blocked for its full time"
        );
        assert_eq!(blocked.tiles(), vec![REFUSED_35_TIMES]);
        assert_eq!(
            blocked.expire(now + REFUSED_TILE_MEMORY),
            vec![REFUSED_35_TIMES],
            "and then forgotten, because doors open and boats sail"
        );
        assert!(
            blocked.tiles().is_empty(),
            "so the tile is one a route may use again"
        );
    }

    #[test]
    fn a_tile_refused_again_is_remembered_from_the_second_refusal() {
        let mut blocked = BlockedTiles::default();
        let now = Instant::now();
        blocked.refuse(REFUSED_35_TIMES, now);
        blocked.refuse(REFUSED_23_TIMES, now);
        let later = now + REFUSED_TILE_MEMORY;
        blocked.refuse(REFUSED_35_TIMES, later);
        assert_eq!(
            blocked.expire(later),
            vec![REFUSED_23_TIMES],
            "the tile refused once is forgotten on time"
        );
        assert_eq!(
            blocked.tiles(),
            vec![REFUSED_35_TIMES],
            "and the one refused again is held once, from the second refusal"
        );
        assert_eq!(
            blocked.expire(later + REFUSED_TILE_MEMORY),
            vec![REFUSED_35_TIMES]
        );
    }

    /// The wall the character walks along while the memory fills: one tile of
    /// it for every tile the memory holds, and one more than that.
    fn wall_tile(i: usize) -> Point3 {
        Point3::new(
            REFUSED_35_TIMES.x + i as u16,
            REFUSED_35_TIMES.y,
            REFUSED_35_TIMES.z,
        )
    }

    #[test]
    fn the_oldest_refused_tile_is_dropped_first_when_the_memory_is_full() {
        let mut blocked = BlockedTiles::default();
        let now = Instant::now();
        for i in 0..REFUSED_TILES_MAX {
            assert_eq!(
                blocked.refuse(wall_tile(i), now),
                None,
                "tile {i} is inside the cap"
            );
        }
        assert_eq!(blocked.tiles().len(), REFUSED_TILES_MAX);
        let one_too_many = wall_tile(REFUSED_TILES_MAX);
        assert_eq!(
            blocked.refuse(one_too_many, now),
            Some(wall_tile(0)),
            "the oldest tile is the one that makes room"
        );
        assert_eq!(
            blocked.tiles().len(),
            REFUSED_TILES_MAX,
            "and the memory stays at the cap, however long the session runs"
        );
        assert!(
            !blocked.tiles().iter().any(|t| same_tile(*t, wall_tile(0))),
            "the oldest tile is gone"
        );
        assert!(
            blocked.tiles().iter().any(|t| same_tile(*t, one_too_many)),
            "and the newest is held"
        );
    }

    /// The wall the character meets, running north to south through the tile
    /// that refused him 35 times: `i` tiles south of it.
    fn wall_north_south(i: i32) -> Point3 {
        Point3::new(
            REFUSED_35_TIMES.x,
            (i32::from(REFUSED_35_TIMES.y) + i) as u16,
            REFUSED_35_TIMES.z,
        )
    }

    /// The measured fault this rule replaces. An earlier version of this
    /// client guessed that three refusals in a line meant a wall running eight
    /// more tiles past each end, and stamped sixteen tiles it had never met.
    /// No real client does that, and on a wall that curves, a fence with a gate
    /// in it, or a queue of people, those guessed tiles close the very gap the
    /// character should walk through.
    ///
    /// One refusal blocks one cell, at radius nought, however many refusals
    /// come before it and whatever line they lie on.
    #[test]
    fn one_refusal_blocks_one_cell_and_never_a_line() {
        const REFUSALS_IN_A_LINE: i32 = 4;
        let mut blocked = BlockedTiles::default();
        let now = Instant::now();
        for met in 0..REFUSALS_IN_A_LINE {
            assert_eq!(
                blocked.refuse(wall_north_south(met), now),
                None,
                "nothing is dropped while there is room"
            );
            assert_eq!(
                blocked.tiles().len(),
                met as usize + 1,
                "refusal {met} blocks one cell and no more"
            );
        }
        for held in blocked.tiles() {
            assert!(
                (0..REFUSALS_IN_A_LINE).any(|met| same_tile(wall_north_south(met), held)),
                "{held} was never refused, so nothing may plan around it"
            );
        }
        for beyond in [
            wall_north_south(REFUSALS_IN_A_LINE),
            wall_north_south(-1),
            Point3::new(
                REFUSED_35_TIMES.x + 1,
                REFUSED_35_TIMES.y,
                REFUSED_35_TIMES.z,
            ),
        ] {
            assert!(
                !blocked.tiles().iter().any(|held| same_tile(*held, beyond)),
                "{beyond} is open ground he has never been refused at"
            );
        }
    }

    /// The server putting the character on a tile is proof that it takes him,
    /// so that tile stops being one any route goes around.
    #[test]
    fn a_tile_he_is_put_on_is_forgotten() {
        let mut blocked = BlockedTiles::default();
        let now = Instant::now();
        blocked.refuse(wall_north_south(0), now);
        blocked.refuse(wall_north_south(1), now);
        assert!(blocked.forget(wall_north_south(1)), "the server took him");
        assert!(
            !blocked.forget(wall_north_south(1)),
            "and there is nothing left there to forget"
        );
        assert_eq!(blocked.tiles(), vec![wall_north_south(0)]);
    }

    /// A new destination is a new journey, and the marks made on the way
    /// somewhere else say nothing about this way.
    #[test]
    fn a_new_destination_forgets_every_refused_tile() {
        const MET_ON_THE_WAY: usize = 3;
        let mut m = Movement::default();
        let now = Instant::now();
        assert!(
            m.begin_trip(REFUSED_35_TIMES).is_empty(),
            "the first trip starts with nothing to forget"
        );
        for met in 0..MET_ON_THE_WAY as i32 {
            m.blocked.refuse(wall_north_south(met), now);
        }
        m.refused_edges
            .refuse(wall_north_south(0), wall_north_south(1), now);
        assert!(
            m.begin_trip(REFUSED_35_TIMES).is_empty(),
            "the same destination is the same trip, so nothing is forgotten"
        );
        assert_eq!(m.blocked.tiles().len(), MET_ON_THE_WAY);
        let forgotten = m.begin_trip(REFUSED_23_TIMES);
        assert_eq!(
            forgotten.len(),
            MET_ON_THE_WAY,
            "every mark goes with the old journey"
        );
        assert!(m.blocked.tiles().is_empty());
        assert!(m.refused_edges.moves().is_empty());
    }

    /// A trip that has been planned again this often is going nowhere, and it
    /// ends instead of looping.
    #[test]
    fn a_trip_is_given_up_after_the_cap_on_replans() {
        let mut m = Movement::default();
        m.begin_trip(REFUSED_35_TIMES);
        for replan in 1..=REPLANS_MAX {
            assert!(m.count_replan(), "replan {replan} is inside the cap");
        }
        assert!(!m.count_replan(), "and one more ends the trip");
        m.begin_trip(REFUSED_23_TIMES);
        assert!(m.count_replan(), "a new trip starts the count again");
    }

    /// A crossing is not a tile. The step of a stair is entered from the step
    /// below it and from nowhere else, so a refusal is counted on the pair of
    /// tiles it crossed and in the one direction it crossed them.
    #[test]
    fn a_refused_crossing_is_counted_in_the_one_direction_it_failed() {
        let mut edges = RefusedEdges::default();
        let now = Instant::now();
        let from = wall_north_south(0);
        let to = wall_north_south(1);
        assert_eq!(edges.refusals(from, to), 0, "nothing is remembered yet");
        assert_eq!(edges.refuse(from, to, now), 1, "the first refusal of it");
        assert_eq!(
            edges.refuse(to, from, now),
            1,
            "the other way over the same pair is another crossing"
        );
        assert_eq!(edges.refuse(from, to, now), 2, "the second refusal of it");
        let higher = Point3::new(to.x, to.y, to.z + EDGE_Z_TOLERANCE as i8);
        assert_eq!(
            edges.refuse(from, higher, now),
            3,
            "the same crossing read one step of a stair higher is the same crossing"
        );
        let another_floor = Point3::new(to.x, to.y, to.z + EDGE_Z_TOLERANCE as i8 + 1);
        assert_eq!(
            edges.refuse(from, another_floor, now),
            1,
            "and one further up than that is a crossing of its own"
        );
        assert_eq!(
            edges.moves().len(),
            3,
            "each is one shut move the route finder plans around: {:?}",
            edges.moves()
        );
        edges.expire(now + REFUSED_TILE_MEMORY);
        assert!(
            edges.moves().is_empty(),
            "and none of them is remembered for ever"
        );
    }

    #[test]
    fn a_door_is_clicked_once_and_not_again_once_its_graphic_changes() {
        let mut opener = DoorOpener::default();
        let now = Instant::now();
        assert_eq!(
            opener.plan(approach(shut_inn_door()), now),
            DoorPlan::Approach(approach(shut_inn_door()))
        );
        assert_eq!(
            opener.plan(approach(shut_inn_door()), now),
            DoorPlan::Underway,
            "nothing starts a second attempt on a door already in hand"
        );
        assert_eq!(
            opener.due(AT_THE_INN_DOOR_CORNER, now),
            None,
            "a person does not open a door from the corner beside it"
        );
        assert_eq!(
            opener.due(IN_FRONT_OF_THE_INN_DOOR, now),
            Some(shut_inn_door()),
            "he clicks it from the tile in front of it"
        );
        assert_eq!(
            opener.due(IN_FRONT_OF_THE_INN_DOOR, now),
            None,
            "one attempt is one click"
        );
        // The server answers on the same serial with the open graphic. This
        // door keeps its tile, so only the graphic says it moved.
        let swung = inn_door(INN_DOOR_OPEN_GRAPHIC, INN_DOORWAY);
        assert!(opener.answered(INN_DOOR_SERIAL), "the door answered him");
        assert!(!opener.waiting(), "so the attempt is over");
        assert_eq!(
            opener.plan(approach(swung), now),
            DoorPlan::Opened(swung),
            "the door he opened is never clicked again: that click would shut it"
        );
        assert_eq!(opener.due(IN_FRONT_OF_THE_INN_DOOR, now), None);
    }

    #[test]
    fn a_door_is_clicked_once_and_not_again_once_its_tile_changes() {
        let mut opener = DoorOpener::default();
        let now = Instant::now();
        opener.plan(approach(shut_inn_door()), now);
        assert_eq!(
            opener.due(IN_FRONT_OF_THE_INN_DOOR, now),
            Some(shut_inn_door())
        );
        // The measured answer: the same serial one tile away, with the open
        // graphic. The doorway it stood in is clear now.
        let swung = inn_door(INN_DOOR_OPEN_GRAPHIC, INN_DOOR_SWUNG_TO);
        assert!(opener.answered(INN_DOOR_SERIAL));
        assert_eq!(
            opener.plan(approach(swung), now),
            DoorPlan::Opened(swung),
            "a door that has moved has answered, so it is left alone"
        );
        // A door that keeps its graphic and only moves has answered as well.
        let mut opener = DoorOpener::default();
        opener.plan(approach(shut_inn_door()), now);
        opener.due(IN_FRONT_OF_THE_INN_DOOR, now);
        opener.answered(INN_DOOR_SERIAL);
        let moved = inn_door(INN_DOOR_SHUT_GRAPHIC, INN_DOOR_SWUNG_TO);
        assert_eq!(opener.plan(approach(moved), now), DoorPlan::Opened(moved));
    }

    #[test]
    fn only_the_door_the_character_waits_for_answers_him() {
        let mut opener = DoorOpener::default();
        let now = Instant::now();
        opener.plan(approach(shut_inn_door()), now);
        opener.due(IN_FRONT_OF_THE_INN_DOOR, now);
        assert!(
            !opener.answered(Serial(INN_DOOR_SERIAL.0 + 1)),
            "another door swinging nearby is not his door answering"
        );
        assert!(opener.waiting(), "so he is still waiting on his own");
        assert!(opener.answered(INN_DOOR_SERIAL));
    }

    #[test]
    fn a_door_that_never_answers_is_given_up_on_after_the_cap() {
        let mut opener = DoorOpener::default();
        let mut now = Instant::now();
        for attempt in 1..=DOOR_OPEN_ATTEMPTS_MAX {
            assert_eq!(
                opener.plan(approach(shut_inn_door()), now),
                DoorPlan::Approach(approach(shut_inn_door())),
                "attempt {attempt} is inside the cap"
            );
            assert_eq!(
                opener.due(IN_FRONT_OF_THE_INN_DOOR, now),
                Some(shut_inn_door())
            );
            assert_eq!(
                opener.expired(now + DOOR_ANSWER_TIMEOUT - ONE_MILLISECOND),
                None,
                "the server is given its time to answer"
            );
            assert_eq!(
                opener.expired(now + DOOR_ANSWER_TIMEOUT),
                Some(shut_inn_door()),
                "and a click it never answers is one failed attempt"
            );
            assert!(!opener.waiting());
            now += DOOR_ANSWER_TIMEOUT;
        }
        assert_eq!(
            opener.plan(approach(shut_inn_door()), now),
            DoorPlan::Locked(shut_inn_door()),
            "past the cap the door counts as locked"
        );
        assert_eq!(
            opener.due(IN_FRONT_OF_THE_INN_DOOR, now),
            None,
            "and the character stops pushing at it"
        );
    }

    #[test]
    fn a_door_that_moves_at_last_forgets_the_clicks_that_failed() {
        let mut opener = DoorOpener::default();
        let mut now = Instant::now();
        for _ in 0..DOOR_OPEN_ATTEMPTS_MAX {
            opener.plan(approach(shut_inn_door()), now);
            opener.due(IN_FRONT_OF_THE_INN_DOOR, now);
            now += DOOR_ANSWER_TIMEOUT;
            opener.expired(now);
        }
        assert_eq!(
            opener.plan(approach(shut_inn_door()), now),
            DoorPlan::Locked(shut_inn_door())
        );
        // Someone else opens the door and shuts it again. It moves, so it was
        // never stuck, and the character may push at it once more.
        assert!(
            !opener.answered(INN_DOOR_SERIAL),
            "he was not waiting on it: this door is not his click coming back"
        );
        assert_eq!(
            opener.plan(approach(shut_inn_door()), now),
            DoorPlan::Approach(approach(shut_inn_door())),
            "a door that was locked and is now free opens again"
        );
    }

    #[test]
    fn a_door_he_cannot_reach_in_time_costs_no_failed_attempt() {
        let mut opener = DoorOpener::default();
        let now = Instant::now();
        opener.plan(approach(shut_inn_door()), now);
        assert_eq!(
            opener.expired(now + DOOR_REACH_TIMEOUT - ONE_MILLISECOND),
            None,
            "he is given time to walk to the door"
        );
        assert_eq!(
            opener.expired(now + DOOR_REACH_TIMEOUT),
            Some(shut_inn_door())
        );
        for _ in 0..DOOR_OPEN_ATTEMPTS_MAX {
            assert_eq!(
                opener.plan(approach(shut_inn_door()), now),
                DoorPlan::Approach(approach(shut_inn_door())),
                "a door he never reached is not a door that would not open"
            );
            opener.give_up();
        }
    }

    #[test]
    fn giving_up_leaves_the_character_free_to_walk_away() {
        let mut opener = DoorOpener::default();
        let now = Instant::now();
        opener.plan(approach(shut_inn_door()), now);
        assert!(opener.waiting());
        opener.give_up();
        assert!(!opener.waiting());
        assert_eq!(opener.due(IN_FRONT_OF_THE_INN_DOOR, now), None);
    }

    #[test]
    fn a_character_turns_to_the_door_only_when_it_does_not_face_it() {
        assert_eq!(
            facing_toward(IN_FRONT_OF_THE_INN_DOOR, INN_DOORWAY),
            Some(Direction::South)
        );
        assert_eq!(
            facing_toward(INN_DOORWAY, INN_DOORWAY),
            None,
            "no turn means anything on the tile you stand on"
        );
        let mut m = Movement::default();
        let now = Instant::now();
        assert_eq!(
            m.build_turn(
                Direction::South,
                Direction::South,
                IN_FRONT_OF_THE_INN_DOOR,
                now
            ),
            None,
            "a character that already faces the door sends nothing"
        );
        let turn = m
            .build_turn(
                Direction::North,
                Direction::South,
                IN_FRONT_OF_THE_INN_DOOR,
                now,
            )
            .expect("a character that faces away turns first");
        assert_eq!(
            turn,
            encode::move_request(Direction::South, WALKING, SEQ_FIRST, FASTWALK_KEY_EMPTY)
        );
        let waiting = in_flight(&m);
        assert!(waiting.turn, "a turn is a request the server answers");
        assert_eq!(
            waiting.arrives_at, IN_FRONT_OF_THE_INN_DOOR,
            "and it carries the tile he is already standing on, never a new one"
        );
    }

    /// A turn on the spot costs [`TURN_PACE`] and not a step of the pace he
    /// walks at. The server charges a turn nothing and starts its own pace
    /// again from it, so a turn charged as a walking step gives away a whole
    /// tile of ground on every change of direction.
    #[test]
    fn a_turn_costs_far_less_than_a_step() {
        assert!(
            TURN_PACE < Duration::from_millis(STEP_MOUNT_RUN_MS),
            "a turn must cost less than the fastest step there is, and it costs {TURN_PACE:?}"
        );
        let mut m = Movement::default();
        let now = Instant::now();
        m.build_turn(
            Direction::North,
            Direction::South,
            AT_THE_INN_DOOR_CORNER,
            now,
        )
        .expect("he faces away, so he turns");
        assert!(
            !m.ready(now + TURN_PACE - ONE_MILLISECOND),
            "the turn is worth its own pace"
        );
        assert!(
            m.ready(now + TURN_PACE),
            "and the step after it goes out that soon, not a walking step later"
        );
    }

    /// The one gap in the wall of [`walled_map`], with a door standing in it.
    const DOORWAY: Point3 = Point3 {
        x: WALL_X,
        y: GAP_Y,
        z: 0,
    };
    /// The corner tile beside that doorway, the way the follower stood when the
    /// live door refused him. A person never opens a door from a corner.
    const BESIDE_THE_DOORWAY: Point3 = Point3 {
        x: WALL_X - 1,
        y: GAP_Y - 1,
        z: 0,
    };
    /// The tile the leaf lands on when it swings out of the doorway.
    const DOOR_SWUNG_TO: Point3 = Point3 {
        x: WALL_X - 1,
        y: GAP_Y + 1,
        z: 0,
    };
    /// The second door of a pair, standing on the tile in front of the first.
    const INNER_DOORWAY: Point3 = Point3 {
        x: WALL_X - 1,
        y: GAP_Y,
        z: 0,
    };
    /// Two floors of a building stand this far apart, which is more than
    /// [`DOOR_MAX_Z_GAP`]: a door up there is on the floor above.
    const FLOOR_ABOVE_Z: i8 = 20;

    /// A door of the walled map, on the tile given.
    fn wall_door(at: Point3) -> DoorItem {
        DoorItem {
            serial: INN_DOOR_SERIAL,
            graphic: INN_DOOR_SHUT_GRAPHIC,
            location: at,
        }
    }

    #[test]
    fn a_route_stops_in_front_of_a_shut_door_and_never_plans_through_it() {
        let map = walled_map();
        let door = wall_door(DOORWAY);
        assert!(
            pathfind(&map, BESIDE_THE_DOORWAY, target(), &Obstacles::NONE).is_ok(),
            "the doorway is the one way out, and the map alone calls it walkable"
        );
        assert_eq!(
            pathfind(
                &map,
                BESIDE_THE_DOORWAY,
                target(),
                &standing_on(&[door.location])
            )
            .err(),
            Some(uoterm_nav::PathError::Unreachable),
            "the door item makes its own tile blocked, so no route runs through it"
        );
        let way = door_in_the_way(
            &map,
            BESIDE_THE_DOORWAY,
            target(),
            &Obstacles::NONE,
            &[door],
        )
        .expect("the door in the doorway is the thing in the way");
        assert_eq!(way.door, door);
        assert!(
            !way.steps.iter().any(|s| same_tile(*s, door.location)),
            "the walk stops in front of the door: {way:?}"
        );
        assert_eq!(
            way.steps.last().copied(),
            Some(way.stand_on),
            "and it ends on the tile he opens the door from: {way:?}"
        );
    }

    #[test]
    fn the_character_stands_in_front_of_the_door_never_at_its_corner() {
        let map = walled_map();
        let door = wall_door(DOORWAY);
        let way = door_in_the_way(
            &map,
            BESIDE_THE_DOORWAY,
            target(),
            &Obstacles::NONE,
            &[door],
        )
        .expect("a door in the way");
        assert_ne!(
            way.stand_on, BESIDE_THE_DOORWAY,
            "he leaves the corner he bumped at before he opens the door"
        );
        let dx = i32::from(way.stand_on.x) - i32::from(door.location.x);
        let dy = i32::from(way.stand_on.y) - i32::from(door.location.y);
        assert_eq!(
            (dx.abs() + dy.abs(), dx.abs().max(dy.abs())),
            (1, 1),
            "the tile he opens the door from touches it on one side only: {way:?}"
        );
        assert!(
            ORTHOGONAL_DIRS
                .iter()
                .filter_map(|&dir| door.location.neighbour(dir))
                .any(|tile| same_tile(tile, way.stand_on)),
            "and it is one of the four tiles in front of a door: {way:?}"
        );
        assert_eq!(
            facing_toward(way.stand_on, door.location),
            Some(Direction::East),
            "he faces the door before he opens it"
        );
    }

    #[test]
    fn a_route_is_planned_again_once_the_door_has_swung() {
        let map = walled_map();
        let shut = wall_door(DOORWAY);
        let open = wall_door(DOOR_SWUNG_TO);
        let way = door_in_the_way(
            &map,
            BESIDE_THE_DOORWAY,
            target(),
            &Obstacles::NONE,
            &[shut],
        )
        .expect("the shut door is in the way");
        let out = pathfind(&map, way.stand_on, target(), &standing_on(&[open.location]))
            .expect("the doorway the leaf has swung off is open again");
        assert!(
            out.steps
                .iter()
                .any(|s| same_tile(Point3::new(s.x, s.y, s.z), DOORWAY)),
            "the route out runs through the doorway: {out:?}"
        );
        assert_eq!(
            door_in_the_way(&map, way.stand_on, target(), &Obstacles::NONE, &[open]),
            None,
            "and no door stands in the way any more"
        );
    }

    #[test]
    fn a_door_the_route_does_not_use_is_left_shut() {
        let map = walled_map();
        assert_eq!(
            door_in_the_way(
                &map,
                BESIDE_THE_DOORWAY,
                me(),
                &Obstacles::NONE,
                &[wall_door(DOORWAY)]
            ),
            None,
            "a walk that stays on this side of the wall passes no door"
        );
    }

    /// The measured failure. A character inside the New Haven inn was told to
    /// walk outside. Another player stood in the corridor one tile wide that
    /// is the only way out, and the character was told there was no path at
    /// all instead of being sent to the shut door in front of him. Which door
    /// is in the way is a question about doors, and a person standing on the
    /// way is no answer to it.
    #[test]
    fn the_door_is_found_while_somebody_stands_in_the_corridor() {
        let inn = inn_corridor();
        let people = [inn.somebody_in_the_way];
        assert_eq!(
            pathfind(&inn.map, inn.character, inn.outside, &standing_on(&people)).err(),
            Some(uoterm_nav::PathError::Unreachable),
            "the corridor is one tile wide, so the player in it stops every walk down it"
        );
        let way = door_in_the_way(
            &inn.map,
            inn.character,
            inn.outside,
            &standing_on(&people),
            &[inn.door],
        )
        .expect("the shut door is still the thing in the way");
        assert_eq!(way.door, inn.door);
        assert_eq!(
            way.stand_on, inn.in_front_of_the_door,
            "he is sent to the tile he opens the door from: {way:?}"
        );
        assert_eq!(
            way.steps.last().copied(),
            Some(way.stand_on),
            "and the walk ends there: {way:?}"
        );
        assert!(
            !way.steps
                .iter()
                .any(|s| same_tile(*s, inn.somebody_in_the_way)),
            "it never crosses the player in the corridor: {way:?}"
        );
    }

    /// The walk to the door is movement the character really makes, so it
    /// blocks on the people the way any other walk does.
    #[test]
    fn the_walk_to_the_door_goes_around_a_person_standing_on_it() {
        let map = walled_map();
        let door = wall_door(DOORWAY);
        let clear = door_in_the_way(&map, me(), target(), &Obstacles::NONE, &[door])
            .expect("the door in the doorway is the thing in the way");
        let stood_on = clear.steps[clear.steps.len() / 2];
        assert_ne!(
            stood_on, clear.stand_on,
            "the person stands on the way to the door, not on the tile in front of it"
        );
        let around = door_in_the_way(&map, me(), target(), &standing_on(&[stood_on]), &[door])
            .expect("a person on the way is no reason to leave the door shut");
        assert_eq!(around.door, clear.door);
        assert_eq!(
            around.stand_on, clear.stand_on,
            "he still opens the door from the tile in front of it: {around:?}"
        );
        assert!(
            !around.steps.iter().any(|s| same_tile(*s, stood_on)),
            "and he walks around the person to reach it: {around:?}"
        );
        assert_eq!(around.steps.last().copied(), Some(around.stand_on));
    }

    /// Leaving the people out of the search that names the door must not
    /// invent one. A way that is shut for any other reason is a path failure,
    /// and the character is told so.
    #[test]
    fn a_person_on_the_way_is_never_reported_as_a_door() {
        let map = walled_map();
        let walk = pathfind(&map, BESIDE_THE_DOORWAY, me(), &Obstacles::NONE)
            .expect("both stand inside the wall");
        let midpoint = walk.steps[walk.steps.len() / 2];
        let stood_on = Point3::new(midpoint.x, midpoint.y, midpoint.z);
        assert_eq!(
            door_in_the_way(
                &map,
                BESIDE_THE_DOORWAY,
                me(),
                &standing_on(&[stood_on]),
                &[wall_door(DOORWAY)]
            ),
            None,
            "the walk passes no door, and the person standing on it is not one"
        );
    }

    #[test]
    fn the_nearest_door_of_a_pair_is_the_one_he_opens() {
        let map = walled_map();
        let outer = wall_door(DOORWAY);
        let inner = DoorItem {
            serial: Serial(INN_DOOR_SERIAL.0 + 1),
            ..wall_door(INNER_DOORWAY)
        };
        let way = door_in_the_way(&map, me(), target(), &Obstacles::NONE, &[outer, inner])
            .expect("the pair of doors stands in the way");
        assert_eq!(
            way.door, inner,
            "he opens the first door of the pair, not the one behind it"
        );
        assert!(
            ![outer.location, inner.location]
                .iter()
                .any(|at| same_tile(*at, way.stand_on)),
            "and he never stands on a door to open one: {way:?}"
        );
    }

    /// The window a door has to stand in is the whole body column, and not
    /// half of it.
    ///
    /// A person opens a door from the tile in front of it, and on a stair that
    /// tile stands a course or two below the door's own tile. This client used
    /// ten, which is the tightest window of any source and hid every door
    /// between eleven and fifteen courses up. [`DOOR_BODY_COLUMN`] is the
    /// window the reference client uses: the higher of the two grounds, and
    /// one body height up from there.
    #[test]
    fn a_door_up_a_course_or_two_is_still_a_door_on_his_own_floor() {
        /// The height of the tile a person opens the door from: his own feet.
        const AT_HIS_FEET: i8 = 0;
        /// A door standing this far above him, which is more than the window
        /// this client used before and less than a whole body. Every source
        /// but this one calls it a door on his own floor.
        const A_COURSE_OR_TWO_UP: i8 = 12;
        /// A door a whole body above him, which is the floor over his head.
        const A_WHOLE_BODY_UP: i8 = DOOR_BODY_COLUMN as i8;
        const {
            assert!(
                A_COURSE_OR_TWO_UP < A_WHOLE_BODY_UP,
                "the two cases must lie either side of the window, or this proves nothing"
            )
        };
        let door_at = |z: i8| wall_door(Point3::new(DOORWAY.x, DOORWAY.y, z));

        let up_a_stair = door_at(A_COURSE_OR_TWO_UP);
        assert_eq!(
            door_on_tile(&[up_a_stair], DOORWAY.x, DOORWAY.y, AT_HIS_FEET),
            Some(up_a_stair),
            "a door at the top of a stair is one he can reach and walk through"
        );
        let overhead = door_at(A_WHOLE_BODY_UP);
        assert_eq!(
            door_on_tile(&[overhead], DOORWAY.x, DOORWAY.y, AT_HIS_FEET),
            None,
            "and a door a whole body above him is on the floor over his head"
        );
    }

    #[test]
    fn a_door_on_the_floor_above_is_not_the_one_in_the_way() {
        let map = walled_map();
        let overhead = wall_door(Point3::new(DOORWAY.x, DOORWAY.y, FLOOR_ABOVE_Z));
        assert_eq!(
            door_on_tile(&[overhead], DOORWAY.x, DOORWAY.y, BESIDE_THE_DOORWAY.z),
            None,
            "a door over his head is one he can neither reach nor walk through"
        );
        assert_eq!(
            door_in_the_way(
                &map,
                BESIDE_THE_DOORWAY,
                target(),
                &Obstacles::NONE,
                &[overhead]
            ),
            None
        );
    }

    #[test]
    fn a_door_the_character_cannot_walk_up_to_is_no_plan_at_all() {
        let mut map = walled_map();
        // Wall the doorway off from the inside. The way out is shut for a
        // reason no door explains, and the session records a path failure.
        for y in 0..GRID {
            map.set_block(WALL_X - 1, y, true);
        }
        assert_eq!(
            door_in_the_way(
                &map,
                me(),
                target(),
                &Obstacles::NONE,
                &[wall_door(DOORWAY)]
            ),
            None,
            "a door he cannot walk up to is a path failure, not a door to open"
        );
    }

    fn me() -> Point3 {
        Point3::new(ME_X, ROW, 0)
    }

    fn target() -> Point3 {
        Point3::new(TARGET_X, ROW, 0)
    }

    /// A wall from edge to edge with one gap, so only one way through exists.
    fn walled_map() -> MockMap {
        let mut map = MockMap::new(GRID, GRID);
        for y in 0..GRID {
            map.set_block(WALL_X, y, true);
        }
        map.set_block(WALL_X, GAP_Y, false);
        map
    }

    fn assert_beside_target(plan: &FollowPlan, map: &MockMap, from: Point3, target_at: Point3) {
        assert!(
            !same_tile(plan.dest, target_at),
            "a follower never stands on the target: {plan:?}"
        );
        assert_eq!(plan.dest.chebyshev(target_at), 1, "{plan:?}");
        assert!(map.can_walk(plan.dest.x, plan.dest.y), "{plan:?}");
        assert!(
            !plan.steps.iter().any(|s| same_tile(*s, target_at)),
            "the walk never crosses the target's own tile: {plan:?}"
        );
        assert_eq!(
            plan.steps.last().map(|s| (s.x, s.y)),
            Some((plan.dest.x, plan.dest.y)),
            "{plan:?}"
        );
        for dir in ADJACENT_DIRS {
            let Some(slot) = target_at.neighbour(dir) else {
                continue;
            };
            if map.can_walk(slot.x, slot.y) {
                assert!(
                    from.chebyshev(plan.dest) <= from.chebyshev(slot),
                    "{slot:?} is nearer than the chosen {plan:?}"
                );
            }
        }
    }

    #[test]
    fn follow_walks_beside_the_target_never_onto_it() {
        let map = MockMap::new(GRID, GRID);
        let plan = follow_plan(&map, me(), target(), false, &standing_on(&[target()]))
            .expect("open ground has a tile beside the target");
        assert_beside_target(&plan, &map, me(), target());
    }

    #[test]
    fn follow_reaches_a_tile_beside_a_target_behind_a_wall() {
        let map = walled_map();
        let plan = follow_plan(&map, me(), target(), false, &standing_on(&[target()]))
            .expect("the gap makes the target reachable");
        assert_beside_target(&plan, &map, me(), target());
        assert!(
            plan.steps.iter().any(|s| s.x == WALL_X && s.y == GAP_Y),
            "the only way through is the gap: {plan:?}"
        );
    }

    #[test]
    fn follow_skips_a_tile_another_mobile_stands_on() {
        let map = MockMap::new(GRID, GRID);
        let free = follow_plan(&map, me(), target(), false, &standing_on(&[target()]))
            .expect("a free tile");
        let people = [target(), free.dest];
        let plan = follow_plan(&map, me(), target(), false, &standing_on(&people))
            .expect("another free tile");
        assert_ne!(plan.dest, free.dest);
        assert_beside_target(&plan, &map, me(), target());
    }

    /// The measured failure, at the tile it happened on. The tile a follower
    /// wants is the one nearest to it, and the server may already have refused
    /// that very tile. It takes the next one instead of asking for the refused
    /// one again, and the walk to it goes around the refused tile as well.
    #[test]
    fn follow_takes_the_next_tile_when_the_one_it_wants_is_refused() {
        let map = MockMap::new(GRID, GRID);
        let person = [target()];
        let wanted = follow_plan(&map, me(), target(), false, &standing_on(&person))
            .expect("the nearest tile beside the target");
        let refused = [wanted.dest];
        let plan = follow_plan(
            &map,
            me(),
            target(),
            false,
            &Obstacles {
                soft: &person,
                hard: &refused,
                moves: &[],
            },
        )
        .expect("another tile beside the target");
        assert_ne!(
            plan.dest, wanted.dest,
            "the tile the server refused is not asked for again: {plan:?}"
        );
        assert_eq!(
            plan.dest.chebyshev(target()),
            1,
            "and the follower still ends up beside the target: {plan:?}"
        );
        assert!(
            !plan.steps.iter().any(|s| same_tile(*s, wanted.dest)),
            "the walk to it does not cross the refused tile either: {plan:?}"
        );
        assert_eq!(
            plan.steps.last().map(|s| (s.x, s.y)),
            Some((plan.dest.x, plan.dest.y)),
            "and it ends on the tile it picked: {plan:?}"
        );
    }

    #[test]
    fn follow_finds_nothing_when_the_target_is_walled_in() {
        let mut map = MockMap::new(GRID, GRID);
        for dir in ADJACENT_DIRS {
            let slot = target().neighbour(dir).expect("neighbour is on the grid");
            map.set_block(slot.x, slot.y, true);
        }
        assert_eq!(
            follow_plan(&map, me(), target(), false, &standing_on(&[target()])),
            None
        );
    }

    #[test]
    fn follow_matches_the_target_pace() {
        let map = MockMap::new(GRID, GRID);
        let running =
            follow_plan(&map, me(), target(), true, &standing_on(&[target()])).expect("a plan");
        let walking =
            follow_plan(&map, me(), target(), false, &standing_on(&[target()])).expect("a plan");
        assert!(running.running, "a follower runs after a runner");
        assert!(!walking.running, "a follower walks after a walker");
    }

    /// The tightened band, driven as a walk and not as one reading. A gap of
    /// [`FOLLOW_FAR_DISTANCE`] tiles is the band itself, and it is the reading
    /// that means nothing on its own: a settled follower holds there, and a
    /// closing one keeps closing through it until it is back at
    /// [`FOLLOW_NEAR_DISTANCE`].
    #[test]
    fn follow_settles_beside_the_target_and_closes_once_past_the_band() {
        const START_X: u16 = 4;
        const {
            assert!(
                FOLLOW_NEAR_DISTANCE < FOLLOW_FAR_DISTANCE,
                "one single stop distance is the step-in step-back wobble"
            )
        };
        // Follower x, target x, a path is queued, the decision that must come.
        let script: [(u16, u16, bool, FollowDecision); 8] = [
            // Settled beside the target: nothing to do.
            (START_X, START_X + 1, false, FollowDecision::Hold),
            // The target takes one step. Still inside the band, still settled.
            (START_X, START_X + 2, false, FollowDecision::Hold),
            // Past the band: it starts closing.
            (START_X, START_X + 3, false, FollowDecision::Repath),
            // Back inside the band, but closing now, and the target has not
            // left the tile that path was built for.
            (START_X + 1, START_X + 3, true, FollowDecision::Continue),
            // The target moved on, so the path built for its old tile is
            // stale, band or no band.
            (START_X + 2, START_X + 4, true, FollowDecision::Repath),
            // Beside it again: it settles, and forgets it was ever closing.
            (START_X + 3, START_X + 4, true, FollowDecision::Hold),
            // Settled, so the band holds it still once more.
            (START_X + 3, START_X + 5, false, FollowDecision::Hold),
            (START_X + 3, START_X + 6, false, FollowDecision::Repath),
        ];
        let mut state = FollowState::default();
        for (i, &(self_x, target_x, path_active, want)) in script.iter().enumerate() {
            let self_at = Point3::new(self_x, ROW, 0);
            let target_at = Point3::new(target_x, ROW, 0);
            assert_eq!(
                state.decide(self_at, target_at, path_active),
                want,
                "step {i}: gap {}",
                self_at.chebyshev(target_at)
            );
        }
    }

    #[test]
    fn follow_keeps_the_path_until_the_target_moves_far_enough() {
        let mut state = FollowState::default();
        let far = Point3::new(ME_X + FOLLOW_FAR_DISTANCE as u16 + 1, ROW, 0);
        assert_eq!(state.decide(me(), far, false), FollowDecision::Repath);
        state.mark_path(true);
        let short = Point3::new(far.x, far.y + FOLLOW_REPATH_DISTANCE as u16 - 1, 0);
        assert_eq!(state.decide(me(), short, true), FollowDecision::Continue);
        let worth_it = Point3::new(far.x, far.y + FOLLOW_REPATH_DISTANCE as u16, 0);
        assert_eq!(state.decide(me(), worth_it, true), FollowDecision::Repath);
    }

    /// What the session leans on when a door swings: it throws the follow
    /// state away, and a fresh state plans a route again at once. Without that
    /// the follower holds still, because its last search found no way out and
    /// the target it follows has not moved.
    #[test]
    fn a_follower_that_gave_up_plans_again_once_the_door_has_opened() {
        let mut state = FollowState::default();
        let far = Point3::new(ME_X + FOLLOW_FAR_DISTANCE as u16 + 1, ROW, 0);
        assert_eq!(state.decide(me(), far, false), FollowDecision::Repath);
        state.mark_path(false);
        assert_eq!(
            state.decide(me(), far, false),
            FollowDecision::Hold,
            "the shut door left it with nowhere to go"
        );
        let mut state = FollowState::default();
        assert_eq!(
            state.decide(me(), far, false),
            FollowDecision::Repath,
            "the door is open now, so the way out is searched for again"
        );
    }

    #[test]
    fn follow_holds_when_no_path_exists_until_the_target_moves() {
        let mut state = FollowState::default();
        let far = Point3::new(ME_X + FOLLOW_FAR_DISTANCE as u16 + 1, ROW, 0);
        assert_eq!(state.decide(me(), far, false), FollowDecision::Repath);
        state.mark_path(false);
        assert_eq!(state.decide(me(), far, false), FollowDecision::Hold);
        assert_eq!(state.decide(me(), far, false), FollowDecision::Hold);
        let moved = Point3::new(far.x, far.y + FOLLOW_REPATH_DISTANCE as u16, 0);
        assert_eq!(state.decide(me(), moved, false), FollowDecision::Repath);
    }

    /// Trammel, the facet the New Haven inn stands on.
    const TRAMMEL_MAP_INDEX: u8 = 1;
    /// Ground truth measured on a live shard: two people on the ground floor of
    /// the New Haven inn, six tiles apart in a straight line. The inn has two
    /// floors.
    const INN_GROUND_Z: i8 = 27;
    const INN_FOLLOWER: Point3 = Point3 {
        x: 3506,
        y: 2521,
        z: INN_GROUND_Z,
    };
    const INN_TARGET: Point3 = Point3 {
        x: 3506,
        y: 2527,
        z: INN_GROUND_Z,
    };

    #[test]
    fn follow_inside_a_building_with_two_floors_keeps_the_ground_floor() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = uoterm_nav::MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let plan = follow_plan(
            &map,
            INN_FOLLOWER,
            INN_TARGET,
            false,
            &standing_on(&[INN_TARGET]),
        )
        .expect("a tile beside the target on the inn floor");
        assert_eq!(
            plan.dest.chebyshev(INN_TARGET),
            1,
            "a follower stands beside the target: {plan:?}"
        );
        let dest = map.tile_from(INN_GROUND_Z, plan.dest.x, plan.dest.y);
        assert!(dest.walkable(), "{plan:?} lands on {dest:?}");
        assert_eq!(
            dest.z, INN_GROUND_Z,
            "the tile the follower picks is on the ground floor, not the floor above: {dest:?}"
        );
        for step in &plan.steps {
            assert_eq!(
                step.z, INN_GROUND_Z,
                "every step stays on the ground floor: {:?}",
                plan.steps
            );
        }
    }

    /// Felucca, the facet the Britain bank stands on.
    const FELUCCA_MAP_INDEX: u8 = 0;
    /// Ground truth measured on a live shard beside the Britain bank. The
    /// follower stood at the foot of a ramp that climbs two height units for
    /// every tile south, and the person she followed stood at the top of it.
    /// Seven times over she reported there was no way to a tile beside him.
    const RAMP_FOLLOWER: Point3 = Point3 {
        x: 1423,
        y: 1700,
        z: 0,
    };
    const RAMP_TARGET: Point3 = Point3 {
        x: 1421,
        y: 1712,
        z: 20,
    };

    #[test]
    fn follow_up_a_ramp_picks_a_tile_at_the_height_of_the_target() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = uoterm_nav::MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        for dir in ADJACENT_DIRS {
            let slot = RAMP_TARGET
                .neighbour(dir)
                .expect("the tile beside him is on the map");
            assert!(
                !map.can_walk_from(RAMP_FOLLOWER.z, slot.x, slot.y),
                "read from the follower's own height {},{} is no ground at all, which is how all \
                 eight tiles beside him were refused",
                slot.x,
                slot.y
            );
        }

        let plan = follow_plan(
            &map,
            RAMP_FOLLOWER,
            RAMP_TARGET,
            false,
            &standing_on(&[RAMP_TARGET]),
        )
        .expect("a tile beside the target at the top of the ramp");
        assert_eq!(
            plan.dest.chebyshev(RAMP_TARGET),
            1,
            "a follower stands beside the target: {plan:?}"
        );
        assert_eq!(
            plan.dest.z, RAMP_TARGET.z,
            "the tile it picks is at the target's height, not the follower's: {plan:?}"
        );
        assert!(
            map.can_walk_from(RAMP_TARGET.z, plan.dest.x, plan.dest.y),
            "and it is ground a person beside the target can stand on: {plan:?}"
        );
        assert_eq!(
            plan.steps.last().map(|s| (s.x, s.y, s.z)),
            Some((plan.dest.x, plan.dest.y, plan.dest.z)),
            "the walk ends on the tile it picked: {plan:?}"
        );
        let mut climbed = RAMP_FOLLOWER.z;
        for step in &plan.steps {
            assert_eq!(
                step.z,
                map.tile_from(climbed, step.x, step.y).z,
                "every step stands on the ground of its own tile: {plan:?}"
            );
            climbed = step.z;
        }
        assert_eq!(
            climbed, RAMP_TARGET.z,
            "and the walk ends level with the target: {plan:?}"
        );
    }

    /// Ground truth measured on a live shard in the woodland, map index 0, in
    /// the same run as the refusals below. The follower stayed at height 30
    /// while the person she followed went down a long slope, and the gap
    /// opened to eleven tiles and stayed there.
    const DESCENT_FOLLOWER: Point3 = Point3 {
        x: 1420,
        y: 1528,
        z: 30,
    };
    const DESCENT_TARGET: Point3 = Point3 {
        x: 1411,
        y: 1518,
        z: 8,
    };
    /// The tile the server refused her at, which her memory held all the
    /// while. It stands one step off the top of that slope, and the client
    /// files call it open ground.
    const DESCENT_REFUSED: Point3 = Point3 {
        x: 1419,
        y: 1527,
        z: 30,
    };

    /// The descent read against the client files. The slope is not what
    /// stopped her: the height-aware search walks her down it, and it does so
    /// with the tile her memory holds shut. A destination below her is neither
    /// refused by the goal check nor by anything the two kinds of obstacle do,
    /// so the fault the refusals came from is not the fault here.
    ///
    /// What cannot plan this walk is the flat fallback. It keeps the walker at
    /// the height he started at and reads every tile from there, so ground
    /// more than one person's height below him is no ground at all: only the
    /// height-aware search descends, and a walk it cannot plan is one no
    /// fallback rescues.
    #[test]
    fn the_measured_descent_is_planned_and_the_refused_tile_does_not_stop_it() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = uoterm_nav::MulMap::open(&dir, FELUCCA_MAP_INDEX).expect("open felucca map 0");
        let person = [DESCENT_TARGET];
        let memory = [DESCENT_REFUSED];
        let plan = follow_plan(
            &map,
            DESCENT_FOLLOWER,
            DESCENT_TARGET,
            true,
            &Obstacles {
                soft: &person,
                hard: &memory,
                moves: &[],
            },
        )
        .expect("a tile beside the person at the foot of the slope");
        assert_eq!(
            plan.dest.chebyshev(DESCENT_TARGET),
            1,
            "the follower ends up beside the person it follows: {plan:?}"
        );
        assert!(
            i16::from(DESCENT_FOLLOWER.z) - i16::from(plan.dest.z)
                > i16::from(uoterm_nav::STEP_HEIGHT),
            "and further below where it stood than any one step reaches: {plan:?}"
        );
        assert!(
            !plan
                .steps
                .iter()
                .any(|s| same_tile(*s, DESCENT_REFUSED) || same_tile(*s, DESCENT_TARGET)),
            "the way down crosses neither the refused tile nor the person: {plan:?}"
        );
        let mut walked = DESCENT_FOLLOWER.z;
        for step in &plan.steps {
            assert_eq!(
                step.z,
                map.tile_from(walked, step.x, step.y).z,
                "every step stands on the ground of its own tile: {plan:?}"
            );
            walked = step.z;
        }
        assert_eq!(
            uoterm_nav::pathfind_flat(&map, DESCENT_FOLLOWER, DESCENT_TARGET, &Obstacles::NONE)
                .err(),
            Some(uoterm_nav::PathError::Unreachable),
            "the flat fallback holds the walker at the height he started at, so it cannot go \
             down a slope this long at all"
        );
    }

    /// A stair the shape of the ramp measured beside the Britain bank, one
    /// tile wide so the only way up is straight up it. It rises further in one
    /// tile than [`DOOR_MAX_Z_GAP`], so the height the character starts at
    /// names a floor of its own for every tile of the climb.
    const STAIR_RISE: i8 = 2;
    const STAIR_TILES: u16 = 12;
    const STAIR_COLUMN: u16 = 0;
    /// The tile the door stands on, up the stair and out of the reach of the
    /// height the character starts at.
    const STAIR_DOOR_Y: u16 = 9;
    /// Where he is told to walk: past that door, further up.
    const PAST_THE_STAIR_DOOR_Y: u16 = 10;

    /// The height of the stair `y` tiles from its foot.
    fn stair_z(y: u16) -> i8 {
        (y as i16 * i16::from(STAIR_RISE)) as i8
    }

    fn stair() -> MockMap {
        let mut map = MockMap::new(STAIR_COLUMN + 1, STAIR_TILES);
        for y in 0..STAIR_TILES {
            map.set_z(STAIR_COLUMN, y, stair_z(y));
        }
        map
    }

    fn stair_door(y: u16) -> DoorItem {
        DoorItem {
            serial: INN_DOOR_SERIAL,
            graphic: INN_DOOR_SHUT_GRAPHIC,
            location: Point3::new(STAIR_COLUMN, y, stair_z(y)),
        }
    }

    /// Which floor a door is on is answered from the height of the step that
    /// reaches it. Answered from the height the character started at, a door
    /// at the top of a stair stands on a floor of its own, and he walks into
    /// it instead of opening it.
    #[test]
    fn a_door_up_a_stair_is_judged_from_the_step_that_reaches_it() {
        let map = stair();
        let foot = Point3::new(STAIR_COLUMN, 0, stair_z(0));
        let past = Point3::new(
            STAIR_COLUMN,
            PAST_THE_STAIR_DOOR_Y,
            stair_z(PAST_THE_STAIR_DOOR_Y),
        );
        let door = stair_door(STAIR_DOOR_Y);
        assert!(
            !on_same_floor(foot.z, door.location.z),
            "read from the foot of the stair the door is on a floor of its own, or the test \
             proves nothing"
        );

        let way = door_in_the_way(&map, foot, past, &Obstacles::NONE, &[door])
            .expect("the door up the stair is the one in the way");
        assert_eq!(way.door, door);
        assert_eq!(
            way.stand_on,
            Point3::new(STAIR_COLUMN, STAIR_DOOR_Y - 1, stair_z(STAIR_DOOR_Y - 1)),
            "he opens it from the tile below it, standing on that tile's own ground: {way:?}"
        );
        assert_eq!(
            way.steps.last().copied(),
            Some(way.stand_on),
            "and the walk to it ends there: {way:?}"
        );
    }

    /// The tile a follower picks carries the height of its own ground, which
    /// is the height the walk to it ends on. A destination that carries the
    /// target's height instead names a tile at a height nobody stands at as
    /// soon as the ground under the two of them is not level.
    #[test]
    fn the_tile_a_follower_picks_carries_the_height_of_its_own_ground() {
        let map = stair();
        let follower = Point3::new(STAIR_COLUMN, 0, stair_z(0));
        let target = Point3::new(STAIR_COLUMN, STAIR_DOOR_Y, stair_z(STAIR_DOOR_Y));
        let below = STAIR_DOOR_Y - 1;
        let plan = follow_plan(&map, follower, target, false, &standing_on(&[target]))
            .expect("the tile below the target on the stair");
        assert_eq!(
            plan.dest,
            Point3::new(STAIR_COLUMN, below, stair_z(below)),
            "the follower stands on the ground of the tile below him, not at his height"
        );
        assert_ne!(
            plan.dest.z, target.z,
            "the stair puts them a step apart: {plan:?}"
        );
        assert_eq!(
            plan.steps.last().copied(),
            Some(plan.dest),
            "the last step of the walk and the tile it was planned to must agree: {plan:?}"
        );
    }
}
