//! One TCP session: login, dispatch, tools, reflex tick.
#![allow(clippy::items_after_test_module)]

use crate::building::{building_tiles, multi_id};
use crate::config::{
    ConnectOptions, EncryptionMode, PING_INTERVAL_MS, REFLEX_TICK_MS, STEP_RUN_MS, STEP_WALK_MS,
};
use crate::error::{Result, RuntimeError};
use crate::harvest;
use crate::loot::{LootJob, LootStep};
use crate::manager::{shared_multi_shapes, FacetCache};
use crate::movement::{
    self, door_in_the_way, facing_toward, follow_plan, DoorOpener, DoorPlan, FollowDecision,
    FollowState, Movement,
};
use crate::persona::{Persona, SpeechPolicy};
use crate::reflex::{self, bandage_self_ms, heal_potion_lock_ms, ReflexAction};
use crate::scene;
use crate::tools::{
    Goal, ToolCall, ToolResult, TOOL_ATTACK, TOOL_CANCEL_GOAL, TOOL_CAN_WALK, TOOL_CAST, TOOL_DROP,
    TOOL_EMOTE, TOOL_EQUIP, TOOL_FIND_ITEMS, TOOL_FIND_MOBILES, TOOL_FOLLOW, TOOL_GUMP_CLOSE,
    TOOL_GUMP_RESPOND, TOOL_JOURNAL_SEARCH, TOOL_LIFT, TOOL_LOOK_AROUND, TOOL_LOOT, TOOL_MAP_TILE,
    TOOL_MOVE_TO, TOOL_OBSERVE, TOOL_OPEN_CONTAINER, TOOL_OPEN_DOOR, TOOL_SAY, TOOL_SET_GOAL,
    TOOL_SET_PERSONA, TOOL_SINGLE_CLICK, TOOL_STOP, TOOL_TARGET, TOOL_TRADE_OFFER, TOOL_UNEQUIP,
    TOOL_USE, TOOL_USE_SKILL, TOOL_WAIT_TARGET, TOOL_WALK, TOOL_WAR_MODE, TOOL_WHISPER,
};
use parking_lot::RwLock;
use rand::Rng;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, interval_at, MissedTickBehavior};
use uoterm_nav::{
    pathfind, pathfind_flat, BlockedMove, ClilocData, MapError, MockMap, MulMap, MultiData,
    Obstacles, TileQuery,
};
use uoterm_protocol::crypto::{for_mode, IdentityCipher, StreamCipher};
use uoterm_protocol::encode;
use uoterm_protocol::frame::GameDecoder;
use uoterm_protocol::lengths::PacketTable;
use uoterm_protocol::types::*;
use uoterm_protocol::{parse_with_version, GroundItem, Inbound};
use uoterm_world::{DoorUpdate, MultiUpdate, World};

const CMD_QUEUE_CAP: usize = 64;
const READ_BUF_LEN: usize = 8192;
const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(8);
const LOGIN_DEADLINE: Duration = Duration::from_secs(15);
const LOGIN_READ_TIMEOUT: Duration = Duration::from_secs(5);
const LOGIN_WORLD_DRAIN: Duration = Duration::from_millis(100);
const LOGIN_CHAR_IN_WORLD: &str = "character already in world";
const LOGIN_INCOMPLETE: &str = "login did not complete";
/// One shared budget covering use, lift and the bandage command.
const ACTION_BUDGET: Duration = Duration::from_millis(1000);
const CLILOC_ACTION_TOO_SOON: u32 = 500_119;
const ACTION_TOO_SOON_WORDS: &str = "wait to perform another action";
const CLILOC_BANDAGE_START: u32 = 500_956;
const BANDAGE_STARTED_WORDS: &str = "begin applying the bandages";
/// The facet index the text database is cached under. It is not a map.
const CLILOC_CACHE_INDEX: u8 = 0;
const OUTBOUND_CAP: usize = 256;
/// Ask again for names the server never answered, so nothing stays nameless.
const NAME_RETRY: Duration = Duration::from_secs(20);
/// What the log says when no tile beside the followed mobile can be reached.
const FOLLOW_NO_WAY: &str = "no way to a tile beside the followed mobile";
/// What the log says when a door will not open however often it is clicked.
const DOOR_LOCKED: &str = "door will not open";
/// What the log says when the leaf the character opened stands on the tile he
/// still has to cross. Clicking it again would only shut it.
const DOOR_STILL_BLOCKS: &str = "open door still blocks the way";
/// Why a refused tile is forgotten: it has been remembered its full
/// [`movement::REFUSED_TILE_MEMORY`].
const FORGOT_TIME_UP: &str = "its time is up";
/// Why a refused tile is forgotten: it was the oldest, and
/// [`movement::REFUSED_TILES_MAX`] tiles were already remembered.
const FORGOT_MEMORY_FULL: &str = "it was the oldest and the memory is full";
/// Why a refused tile is forgotten: the server has just put the character on
/// it, which is proof it takes him.
const FORGOT_HE_STANDS_ON_IT: &str = "he stands on it";
/// Why a refused tile is forgotten: the character has been told to walk
/// somewhere else, and the mark was made on the way to somewhere he is no
/// longer going.
const FORGOT_NEW_DESTINATION: &str = "a new destination was set";
const MOCK_GRID: u16 = 2048;
#[cfg(test)]
const GREEDY_STEP_CAP: usize = 64;
#[cfg(test)]
const PLAYER_STEP_CAP: usize = 256;
const PATH_FAIL_LOG_EVERY: Duration = Duration::from_secs(5);
const HOLD_STEP_CAP: usize = 50;
const HOLD_MS_NONE: u64 = 0;
const WALK_STEPS_ONE: usize = 1;
const SPEECH_REJECTED: &str = "speech rejected by persona policy";
const SPEECH_RATE_LIMITED: &str = "persona chat rate exceeded";
/// The facet a session reads before the shard says which one it is on.
const START_MAP_INDEX: u8 = 0;
/// The word the server writes when the character has no stamina left to move
/// with. It is read out of the line in lower case, so the shard may write it
/// in whatever case it likes.
const FATIGUED_WORD: &str = "fatigued";
/// The number of that same line in the client string file, which a shard sends
/// instead of the words. It reaches this client as the number behind a `#`.
const CLILOC_TOO_FATIGUED: u32 = 500_110;
/// How a numbered line is written once it has been read off the wire.
const CLILOC_PREFIX: &str = "#";
/// How recently the server must have said the character is too tired for a
/// refusal to be about that and not about the way ahead.
const FATIGUE_WINDOW: Duration = Duration::from_millis(1500);
/// How long he rests before he tries again once he is told he is too tired.
/// Stamina comes back on its own, and nothing else has to happen.
const FATIGUE_WAIT: Duration = Duration::from_millis(2000);
/// How long he waits for somebody to step off the tile he wants. A person is
/// off his tile in about this long, and waiting costs nothing but the wait.
const MOBILE_WAIT: Duration = Duration::from_millis(500);
/// How far apart in height a mobile and the cell may stand and still be on the
/// same floor. Half a body: further than that and the person is upstairs.
const MOBILE_SAME_FLOOR_Z: i16 = 8;
/// How often he waits at one cell before he treats it as shut after all. A
/// person who has not moved in this many tries is not going to.
const WAITS_BLOCK_CELL: u32 = 15;
/// How often he waits over the whole trip before he gives the trip up. The
/// count that blocks a cell starts again each time; this one does not, so a
/// character shut in by a crowd stops instead of shuffling for ever.
const WAITS_GIVE_UP: u32 = 25;
/// How long he pauses before walking the new route after a refusal he has
/// planned his way around. Long enough that the route is planned once and not
/// on every tick, and short enough that nobody sees him stop.
const REPLAN_RESUME: Duration = Duration::from_millis(150);
/// How long he gives a door to swing before he tries the same step again.
const DOOR_RETRY_WAIT: Duration = Duration::from_millis(700);
/// How many refusals of one crossing are the first one. Past that the way is
/// shut and the cell is marked.
const EDGE_REFUSALS_FIRST: u32 = 1;
/// How long he pauses before the new route once a crossing is proven shut.
/// It is a random span so that two characters refused at one wall do not walk
/// into it again together on the same tick.
const SHUT_RESUME_MIN_MS: u64 = 200;
const SHUT_RESUME_MAX_MS: u64 = 400;
/// What the log and the caller are told when a trip ends because the character
/// has waited too long for the same way to open.
const WAITED_TOO_LONG: &str = "waited too long for the way to open";
/// What they are told when a trip ends because no route it plans gets anywhere.
const REPLANNED_TOO_OFTEN: &str = "the route was planned again too many times";

enum SessionCmd {
    Tool(ToolCall, oneshot::Sender<ToolResult>),
    Shutdown,
}

struct HandleInner {
    tx: mpsc::Sender<SessionCmd>,
}

impl Drop for HandleInner {
    fn drop(&mut self) {
        let _ = self.tx.try_send(SessionCmd::Shutdown);
    }
}

#[derive(Clone)]
pub struct SessionHandle {
    pub id: String,
    pub world: Arc<RwLock<World>>,
    inner: Arc<HandleInner>,
}

impl SessionHandle {
    pub async fn call(&self, tool: ToolCall) -> ToolResult {
        let (tx, rx) = oneshot::channel();
        if self
            .inner
            .tx
            .send(SessionCmd::Tool(tool, tx))
            .await
            .is_err()
        {
            return ToolResult::err("session closed");
        }
        match tokio::time::timeout(TOOL_CALL_TIMEOUT, rx).await {
            Ok(Ok(r)) => r,
            Ok(Err(_)) => ToolResult::err("session closed"),
            Err(_) => ToolResult::err("tool timed out"),
        }
    }

    pub async fn shutdown(&self) {
        let _ = self.inner.tx.send(SessionCmd::Shutdown).await;
    }

    pub fn observe_json(&self) -> Value {
        serde_json::to_value(self.world.read().observe_default()).unwrap_or(Value::Null)
    }

    pub fn snapshot(&self) -> Value {
        self.observe_json()
    }

    pub fn logged_in(&self) -> bool {
        self.world.read().logged_in
    }
}

pub async fn start(
    id: String,
    opts: ConnectOptions,
    facets: Arc<FacetCache<MulMap>>,
    multi_shapes: Arc<FacetCache<MultiData>>,
    clilocs: Arc<FacetCache<ClilocData>>,
) -> Result<SessionHandle> {
    let world = Arc::new(RwLock::new(World::new()));
    let (tx, rx) = mpsc::channel(CMD_QUEUE_CAP);
    let (login_tx, login_rx) = oneshot::channel::<Result<()>>();
    let handle = SessionHandle {
        id: id.clone(),
        world: world.clone(),
        inner: Arc::new(HandleInner { tx }),
    };
    tokio::spawn(async move {
        if let Err(e) =
            run_session(id, opts, facets, multi_shapes, clilocs, world, rx, login_tx).await
        {
            tracing::error!(error = %e, "session ended");
        }
    });
    match login_rx.await {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(RuntimeError::Network("session ended before login".into())),
    }
}

struct Inner {
    id: String,
    world: Arc<RwLock<World>>,
    goal: Goal,
    persona: Persona,
    speech: SpeechPolicy,
    outbound: VecDeque<Vec<u8>>,
    compressed: bool,
    encryption: EncryptionMode,
    cipher: Box<dyn StreamCipher>,
    decoder: GameDecoder,
    map: MockMap,
    uopath: Option<PathBuf>,
    /// Facets this session reads, shared with every other session on the same
    /// client directory. The session holds them only while it runs.
    maps: HashMap<u8, Arc<MulMap>>,
    facets: Arc<FacetCache<MulMap>>,
    /// The shapes of every house and boat the client files describe, shared
    /// with every other session on the same client directory. `None` when the
    /// session was given no client directory, or when those files would not
    /// open: a building is then only known from the steps the server refuses.
    multi_shapes: Option<Arc<MultiData>>,
    movement: Movement,
    doors: DoorOpener,
    era: Era,
    version: ClientVersion,
    follow: Option<Serial>,
    follow_state: FollowState,
    cliloc: Option<Arc<ClilocData>>,
    next_action_at: Instant,
    next_bandage_at: Instant,
    next_heal_potion_at: Instant,
    attack_sent: Option<Serial>,
    target_intent: Option<Serial>,
    loot: Option<LootJob>,
    sent_drop: Option<Serial>,
    last_event_seq: u64,
    last_name_retry: Instant,
    last_path_fail: Option<(Point3, Instant)>,
    /// When the server last said the character was too tired to move. A
    /// refusal that close behind it is about his stamina and about nothing in
    /// the way.
    last_fatigued: Option<Instant>,
}

/// The tiles a walk must go around, kept apart by what proves each one shut,
/// and owned so that the route finder can borrow both lists at once.
///
/// The split is the whole of the difference between a thing that moves and a
/// thing that does not: see [`Obstacles`].
struct InTheWay {
    /// The mobiles and the doors: each is off its tile in a moment.
    soft: Vec<Point3>,
    /// The tiles the server refused, and the walls of the buildings the
    /// character can see: each is proven shut.
    hard: Vec<Point3>,
    /// The crossings the server refused, each shut in the one direction it was
    /// refused in and open from every other side.
    moves: Vec<BlockedMove>,
}

impl InTheWay {
    /// The three lists as the route finder reads them.
    fn obstacles(&self) -> Obstacles<'_> {
        Obstacles {
            soft: &self.soft,
            hard: &self.hard,
            moves: &self.moves,
        }
    }
}

impl Inner {
    fn map_index(&self) -> u8 {
        self.world.read().self_state.map
    }

    /// The tiles this session walks on: the client facet once it is open, and
    /// the empty grid before that.
    fn tiles(&self) -> &dyn TileQuery {
        self.maps
            .get(&self.map_index())
            .map(|m| m.as_ref() as &dyn TileQuery)
            .unwrap_or(&self.map)
    }

    /// Where every other mobile stands that a walk has to go around.
    ///
    /// The facet decides. On a Trammel-ruleset facet the server lets anyone
    /// walk over anyone else, so nobody is in the way there and this is empty.
    fn mobiles(&self) -> Vec<Point3> {
        self.world.read().blocking_mobile_tiles()
    }

    /// Every door the character can see, as the door logic reads them.
    fn doors_seen(&self) -> Vec<uoterm_world::DoorItem> {
        self.world.read().doors.values().copied().collect()
    }

    /// Every tile the walls of a building the character can see close to him
    /// on the floor he is standing on.
    ///
    /// The character's own floor is the tile the steps already on the wire
    /// leave him on, which is the floor every route is planned from.
    fn building_walls(&self) -> Vec<Point3> {
        let Some(shapes) = self.multi_shapes.as_deref() else {
            return Vec::new();
        };
        let (buildings, feet_z) = {
            let world = self.world.read();
            (
                world.multis_seen(),
                self.movement.stepping_from(world.self_state.location).z,
            )
        };
        if buildings.is_empty() {
            return Vec::new();
        }
        building_tiles(shapes, &buildings, feet_z, &self.doors_seen())
    }

    /// Every tile a walk goes around whatever else stands in the way: the
    /// mobiles, the walls of the buildings the character can see, and the
    /// tiles the server has refused him.
    ///
    /// The mobiles are soft and the rest is hard. A player house is built on
    /// the server and stands in no client map file, so the map calls its tiles
    /// open ground. Two things say otherwise. The item packet that names the
    /// multi is the first, and the shape in the client files then gives every
    /// wall of it before he ever walks into one. The refusal is the second,
    /// and it is all he has for a building the client files do not describe.
    /// Neither is going to move, so a walk to one of those tiles is refused
    /// rather than walked; a person standing where the walk ends will have
    /// stepped off it by the time the character arrives.
    fn avoided(&self) -> InTheWay {
        let mut hard = self.building_walls();
        hard.extend(self.movement.blocked.tiles());
        InTheWay {
            soft: self.mobiles(),
            hard,
            moves: self.movement.refused_edges.moves(),
        }
    }

    /// Every tile a walk must go around: [`Inner::avoided`], and the doors.
    ///
    /// A shut door stands on the tile it blocks and an open one has swung off
    /// it, so a route planned around the door items stops in front of a shut
    /// door and runs straight through an open doorway. Without this a route
    /// runs into a closed leaf, because the client map data calls a door tile
    /// walkable whichever way the door stands.
    ///
    /// A door is soft, as a person is. It is not proven shut: it is opened,
    /// and [`door_route`] is what opens it. A walk that ends in a doorway is a
    /// walk to a door the character can open, so it is planned and not
    /// refused.
    fn blockers(&self) -> InTheWay {
        let mut out = self.avoided();
        out.soft.extend(self.world.read().door_tiles());
        out
    }

    fn ensure_facet(&mut self) {
        let idx = self.map_index();
        if self.maps.contains_key(&idx) {
            return;
        }
        let Some(path) = self.uopath.clone() else {
            return;
        };
        match shared_facet(&self.facets, &path, idx) {
            Ok(m) => {
                tracing::info!(
                    map = idx,
                    opens = self.facets.opens(),
                    live = self.facets.live(),
                    "map facet ready"
                );
                self.maps.insert(idx, m);
            }
            Err(e) => tracing::warn!(map = idx, error = %e, "failed to open map facet"),
        }
    }

    fn start_cipher(&mut self, seed: u32) {
        self.cipher = for_mode(self.encryption, seed, self.version);
        tracing::debug!(cipher = self.cipher.name(), seed, "session cipher start");
    }

    fn reset_cipher_for_game(&mut self, seed: u32) {
        self.cipher.reset_for_game(seed);
        tracing::debug!(
            cipher = self.cipher.name(),
            seed,
            "session cipher game reset"
        );
    }
}

/// Takes one facet from the runtime cache, reading the client files only when
/// no other session holds that facet already.
fn shared_facet(
    facets: &FacetCache<MulMap>,
    uopath: &Path,
    map_index: u8,
) -> std::result::Result<Arc<MulMap>, MapError> {
    facets.get_or_load(uopath, map_index, || {
        tracing::info!(map = map_index, path = %uopath.display(), "opening map facet");
        MulMap::open(uopath, map_index)
    })
}

fn shared_cliloc(
    clilocs: &FacetCache<ClilocData>,
    uopath: &Path,
) -> std::result::Result<Arc<ClilocData>, MapError> {
    clilocs.get_or_load(uopath, CLILOC_CACHE_INDEX, || {
        tracing::info!(path = %uopath.display(), "opening the client text database");
        ClilocData::open(uopath)
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_session(
    id: String,
    opts: ConnectOptions,
    facets: Arc<FacetCache<MulMap>>,
    multi_shapes: Arc<FacetCache<MultiData>>,
    clilocs: Arc<FacetCache<ClilocData>>,
    world: Arc<RwLock<World>>,
    mut rx: mpsc::Receiver<SessionCmd>,
    login_tx: oneshot::Sender<Result<()>>,
) -> Result<()> {
    // The world-item packet is two bytes shorter below client 7.0.9.0, and
    // three other packets move with it. Reading the wrong width there loses
    // two bytes per item and the whole stream then runs out of step.
    let table = PacketTable::for_version(opts.era, opts.version);
    let mut maps = HashMap::new();
    if let Some(path) = &opts.uopath {
        match shared_facet(&facets, path, START_MAP_INDEX) {
            Ok(m) => {
                maps.insert(START_MAP_INDEX, m);
            }
            Err(e) => {
                let msg = format!("uopath: {e}");
                let err = RuntimeError::Config(msg.clone());
                let _ = login_tx.send(Err(RuntimeError::Config(msg)));
                return Err(err);
            }
        }
    }
    // The shapes of the houses and boats. They are read once for the whole
    // client directory, and a directory that has none costs the character the
    // buildings and nothing else, so this never stops a login: without it he
    // learns of a house from the steps the server refuses him, as he did
    // before the client files were read at all.
    let shapes =
        opts.uopath
            .as_deref()
            .and_then(|path| match shared_multi_shapes(&multi_shapes, path) {
                Ok(shapes) => {
                    tracing::info!(
                        multis = shapes.multi_count(),
                        opens = multi_shapes.opens(),
                        live = multi_shapes.live(),
                        "multi shapes ready"
                    );
                    Some(shapes)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to read the client multi files");
                    None
                }
            });
    let cliloc = opts
        .uopath
        .as_deref()
        .and_then(|path| match shared_cliloc(&clilocs, path) {
            Ok(text) => {
                tracing::info!(
                    messages = text.message_count(),
                    opens = clilocs.opens(),
                    live = clilocs.live(),
                    "text database ready"
                );
                Some(text)
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to read the client text database");
                None
            }
        });
    let mut inner = Inner {
        id,
        world,
        goal: Goal::Idle,
        persona: opts.persona.clone().unwrap_or_else(Persona::lumberjack_yew),
        speech: SpeechPolicy::default(),
        outbound: VecDeque::new(),
        compressed: false,
        encryption: opts.encryption,
        cipher: Box::new(IdentityCipher),
        decoder: GameDecoder::with_version(table, opts.version),
        map: MockMap::new(MOCK_GRID, MOCK_GRID),
        uopath: opts.uopath.clone(),
        maps,
        facets,
        multi_shapes: shapes,
        movement: Movement::default(),
        doors: DoorOpener::default(),
        era: opts.era,
        version: opts.version,
        follow: None,
        follow_state: FollowState::default(),
        cliloc,
        next_action_at: Instant::now() - ACTION_BUDGET,
        next_bandage_at: Instant::now() - ACTION_BUDGET,
        next_heal_potion_at: Instant::now() - ACTION_BUDGET,
        attack_sent: None,
        target_intent: None,
        loot: None,
        sent_drop: None,
        last_path_fail: None,
        last_fatigued: None,
        last_event_seq: 0,
        last_name_retry: Instant::now(),
    };
    let (mut reader, mut writer) = match login(&opts, &mut inner).await {
        Ok(pair) => {
            let _ = login_tx.send(Ok(()));
            pair
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = login_tx.send(Err(RuntimeError::Network(msg.clone())));
            return Err(e);
        }
    };

    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(OUTBOUND_CAP);
    tokio::spawn(async move {
        while let Some(pkt) = out_rx.recv().await {
            if writer.write_all(&pkt).await.is_err() {
                break;
            }
        }
    });

    let mut buf = vec![0u8; READ_BUF_LEN];
    let ping_period = Duration::from_millis(PING_INTERVAL_MS);
    let tick_period = Duration::from_millis(REFLEX_TICK_MS);
    let mut ping = interval_at(tokio::time::Instant::now() + ping_period, ping_period);
    let mut tick = interval(tick_period);
    ping.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            cmd = rx.recv() => {
                match cmd {
                    None | Some(SessionCmd::Shutdown) => break,
                    Some(SessionCmd::Tool(call, reply)) => {
                        let result = handle_tool(&mut inner, call);
                        let _ = reply.send(result);
                    }
                }
            }
            n = reader.read(&mut buf) => {
                let n = n.map_err(|e| RuntimeError::Network(e.to_string()))?;
                if n == 0 { break; }
                read_from_the_wire(&mut inner, &mut buf[..n]);
            }
            _ = ping.tick() => { inner.outbound.push_back(encode::ping(1)); }
            _ = tick.tick() => {
                reflex_tick(&mut inner);
                pump_doors(&mut inner);
                pump_movement(&mut inner, Instant::now());
                pump_names(&mut inner);
                harvest_new_events(&mut inner);
            }
        }
        flush_out(&mut inner, &out_tx);
    }
    Ok(())
}

fn flush_out(inner: &mut Inner, out_tx: &mpsc::Sender<Vec<u8>>) {
    flush_sealed(&mut inner.outbound, inner.cipher.as_mut(), out_tx);
}

fn flush_sealed(
    outbound: &mut VecDeque<Vec<u8>>,
    cipher: &mut dyn StreamCipher,
    out_tx: &mpsc::Sender<Vec<u8>>,
) {
    while !outbound.is_empty() {
        let permit = match out_tx.try_reserve() {
            Ok(permit) => permit,
            Err(mpsc::error::TrySendError::Full(())) => return,
            Err(mpsc::error::TrySendError::Closed(())) => {
                outbound.clear();
                return;
            }
        };
        let Some(pkt) = outbound.pop_front() else {
            return;
        };
        permit.send(seal_bytes(cipher, pkt));
    }
}

/// The packet id and the byte length that name one outbound packet in the log.
///
/// Without this the log holds every packet the server sent and none of the
/// ones the client sent, so an action that appears to do nothing cannot be
/// told from an action whose packet never left.
///
/// A packet with no bytes has no id and is not worth a line.
fn outbound_shape(pkt: &[u8]) -> Option<(u8, usize)> {
    Some((*pkt.first()?, pkt.len()))
}

/// The reference client encrypts the built packet on send. Huffman is
/// inbound-only.
/// If both applied, Huffman would run before encrypt.
///
/// Every packet the client sends passes through here, so this is the one place
/// that can name it in the log, and the last one: after the cipher runs, every
/// byte is ciphertext and the id cannot be read back.
fn seal_bytes(cipher: &mut dyn StreamCipher, mut pkt: Vec<u8>) -> Vec<u8> {
    if let Some((id, bytes)) = outbound_shape(&pkt) {
        tracing::trace!(id = format!("{id:#04x}"), bytes, "outbound packet");
    }
    cipher.encrypt(&mut pkt);
    pkt
}

fn seal_outbound(inner: &mut Inner, pkt: Vec<u8>) -> Vec<u8> {
    seal_bytes(inner.cipher.as_mut(), pkt)
}

/// Game stream: decrypt (no-op for None), then Huffman via GameDecoder.
/// Login replies stay plaintext (crypto.rs / the reference client).
fn ingest_wire(inner: &mut Inner, data: &mut [u8]) -> Vec<Inbound> {
    if inner.compressed {
        inner.cipher.decrypt(data);
    }
    ingest(inner, data)
}

/// Everything one read off the socket sets going: the packets themselves, and
/// then the walk.
///
/// The walk is pumped here and not on the tick alone. The answer to a step is
/// what frees the wire for the next one, so a walk pumped only on the tick
/// leaves every step waiting up to one whole tick after its answer landed, and
/// a character loses that much ground on every step of a run.
fn read_from_the_wire(inner: &mut Inner, data: &mut [u8]) -> Vec<Inbound> {
    let msgs = ingest_wire(inner, data);
    pump_movement(inner, Instant::now());
    msgs
}

async fn tcp_write(writer: &mut tokio::net::tcp::OwnedWriteHalf, bytes: &[u8]) -> Result<()> {
    writer
        .write_all(bytes)
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))
}

async fn tcp_flush(writer: &mut tokio::net::tcp::OwnedWriteHalf) -> Result<()> {
    writer
        .flush()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))
}

async fn write_sealed(
    inner: &mut Inner,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    pkt: Vec<u8>,
) -> Result<()> {
    tcp_write(writer, &seal_outbound(inner, pkt)).await
}

async fn send_client_identity(
    inner: &mut Inner,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    opts: &ConnectOptions,
) -> Result<()> {
    write_sealed(inner, writer, encode::client_version(opts.version)).await?;
    Ok(())
}

async fn login(
    opts: &ConnectOptions,
    inner: &mut Inner,
) -> Result<(
    tokio::net::tcp::OwnedReadHalf,
    tokio::net::tcp::OwnedWriteHalf,
)> {
    let addr = format!("{}:{}", opts.host, opts.port);
    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    let _ = stream.set_nodelay(true);
    let (mut reader, mut writer) = stream.into_split();
    let (login_seed, seed_pkt) = opening_seed(opts);
    tcp_write(&mut writer, &seed_pkt).await?;
    inner.start_cipher(login_seed);
    write_sealed(
        inner,
        &mut writer,
        encode::login_request(&opts.account, &opts.password, opts.next_login_key),
    )
    .await?;
    let mut buf = vec![0u8; READ_BUF_LEN];
    let deadline = tokio::time::Instant::now() + LOGIN_DEADLINE;
    let mut seen_server_list = false;
    let mut seen_relay = false;
    let mut seen_chars = false;
    let mut seen_confirm = false;
    while tokio::time::Instant::now() < deadline {
        let n = tokio::time::timeout(LOGIN_READ_TIMEOUT, reader.read(&mut buf))
            .await
            .map_err(|_| RuntimeError::Network("login timeout".into()))?
            .map_err(|e| RuntimeError::Network(e.to_string()))?;
        if n == 0 {
            return Err(RuntimeError::Network("login closed".into()));
        }
        let packets = ingest_wire(inner, &mut buf[..n]);
        for msg in &packets {
            if let Some(err) = login_abort(msg) {
                return Err(err);
            }
            match msg {
                Inbound::ServerList { servers, .. } => {
                    seen_server_list = true;
                    let idx = select_shard(servers, opts.shard.as_deref());
                    write_sealed(inner, &mut writer, encode::select_server(idx)).await?;
                }
                Inbound::Relay { ip, port, auth_id } => {
                    seen_relay = true;
                    let stay = stay_on_relay(opts, *ip, *port);
                    let dest = relay_addr(opts, *ip, *port);
                    tracing::info!(
                        relay_ip = ?ip,
                        relay_port = port,
                        stay,
                        dest = dest.as_str(),
                        "login relay 0x8C"
                    );
                    if !stay {
                        drop(reader);
                        drop(writer);
                        let stream = TcpStream::connect(&dest)
                            .await
                            .map_err(|e| RuntimeError::Network(e.to_string()))?;
                        let _ = stream.set_nodelay(true);
                        let split = stream.into_split();
                        reader = split.0;
                        writer = split.1;
                        inner.decoder.reset();
                    }
                    inner.reset_cipher_for_game(*auth_id);
                    if game_login_needs_raw_seed(stay, opts.era) {
                        tcp_write(&mut writer, &encode::seed(*auth_id)).await?;
                    }
                    write_sealed(
                        inner,
                        &mut writer,
                        encode::game_login(*auth_id, &opts.account, &opts.password),
                    )
                    .await?;
                    tcp_flush(&mut writer).await?;
                    inner.compressed = true;
                }
                Inbound::CharacterList { characters } => {
                    seen_chars = true;
                    let slot = characters
                        .iter()
                        .position(|c| c.name.eq_ignore_ascii_case(&opts.character))
                        .or_else(|| characters.iter().position(|c| !c.name.is_empty()))
                        .ok_or_else(|| RuntimeError::NoCharacter(opts.character.clone()))?;
                    let name = characters[slot].name.clone();
                    tracing::info!(
                        slot,
                        name = name.as_str(),
                        count = characters.len(),
                        "login character list"
                    );
                    send_client_identity(inner, &mut writer, opts).await?;
                    write_sealed(
                        inner,
                        &mut writer,
                        encode::play_character(slot as u32, &name, opts.client_flag(), 0),
                    )
                    .await?;
                    tcp_flush(&mut writer).await?;
                    inner.world.write().self_state.name = name;
                }
                Inbound::VersionRequest => {
                    tracing::info!("login version request 0xBD");
                    send_client_identity(inner, &mut writer, opts).await?;
                    tcp_flush(&mut writer).await?;
                }
                Inbound::LoginConfirm {
                    serial, x, y, z, ..
                } => {
                    seen_confirm = true;
                    tracing::info!(serial = serial.0, x, y, z, "login confirm 0x1B");
                }
                Inbound::LoginComplete => {
                    tracing::info!("login complete 0x55");
                }
                _ => {}
            }
        }
        if packets.iter().any(inbound_enters_world) || inner.world.read().logged_in {
            inner.world.write().logged_in = true;
            drain_login_world(inner, &mut reader, &mut buf).await?;
            return Ok((reader, writer));
        }
    }
    Err(RuntimeError::Network(format!(
        "{LOGIN_INCOMPLETE} (server_list={seen_server_list} relay={seen_relay} chars={seen_chars} confirm={seen_confirm})"
    )))
}

fn inbound_enters_world(msg: &Inbound) -> bool {
    matches!(msg, Inbound::LoginConfirm { .. } | Inbound::LoginComplete)
}

async fn drain_login_world(
    inner: &mut Inner,
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    buf: &mut [u8],
) -> Result<()> {
    loop {
        match tokio::time::timeout(LOGIN_WORLD_DRAIN, reader.read(buf)).await {
            Ok(Ok(0)) => return Ok(()),
            Ok(Ok(n)) => {
                let _ = ingest_wire(inner, &mut buf[..n]);
            }
            Ok(Err(e)) => return Err(RuntimeError::Network(e.to_string())),
            Err(_) => return Ok(()),
        }
    }
}

fn login_abort(msg: &Inbound) -> Option<RuntimeError> {
    match msg {
        Inbound::LoginDenied { reason } => Some(RuntimeError::LoginDenied(*reason)),
        Inbound::PopupMessage { reason } if *reason == POPUP_CHAR_IN_WORLD => {
            Some(RuntimeError::Network(LOGIN_CHAR_IN_WORLD.into()))
        }
        Inbound::PopupMessage { reason } if *reason != POPUP_IDLE_WARNING => {
            Some(RuntimeError::Network(format!("login popup {reason}")))
        }
        _ => None,
    }
}

fn opening_seed(opts: &ConnectOptions) -> (u32, Vec<u8>) {
    let mut rng = rand::thread_rng();
    let mut seed: u32 = rng.gen();
    if seed == 0 {
        seed = 1;
    }
    let bytes = match opts.era {
        Era::Modern => encode::seed_ext(seed, opts.version),
        Era::T2a => {
            while seed.to_be_bytes()[0] == PKT_SEED {
                seed = rng.gen();
            }
            encode::seed(seed)
        }
    };
    (seed, bytes)
}

fn select_shard(servers: &[uoterm_protocol::ServerEntry], wanted: Option<&str>) -> u16 {
    if let Some(name) = wanted {
        if let Some(s) = servers.iter().find(|s| s.name.eq_ignore_ascii_case(name)) {
            return s.index;
        }
    }
    servers.first().map(|s| s.index).unwrap_or(0)
}

fn game_login_needs_raw_seed(stay: bool, era: Era) -> bool {
    !(stay && era == Era::Modern)
}

fn stay_on_relay(opts: &ConnectOptions, ip: [u8; 4], port: u16) -> bool {
    if !opts.stay_on_socket {
        return false;
    }
    let addr = Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
    if addr.is_unspecified() {
        return true;
    }
    let host_ip = opts.host.parse::<Ipv4Addr>().ok();
    let same_host = host_ip == Some(addr) || (addr.is_loopback() && host_is_loopback(&opts.host));
    same_host && port == opts.port
}

fn host_is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<Ipv4Addr>()
            .map(|a| a.is_loopback())
            .unwrap_or(false)
}

fn relay_addr(opts: &ConnectOptions, ip: [u8; 4], port: u16) -> String {
    let addr = Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
    if addr.is_unspecified() {
        format!("{}:{port}", opts.host)
    } else {
        format!("{addr}:{port}")
    }
}

#[cfg(test)]
mod relay_tests {
    use super::*;
    use crate::config::JITTER_PCT;
    use uoterm_nav::client_data_dir_from_env;
    use uoterm_protocol::types::{ClientVersion, Era, LOGIN_NEXT_KEY_DEFAULT};
    use uoterm_world::{render_radar, RadarOptions, TileKind, RADAR_SIZE};

    /// A character on the ground floor of the New Haven inn, and the floor
    /// above him. The map answers differently for each, so `map_tile` and
    /// `can_walk` must default to the floor the character is on.
    const INN_GROUND_Z: i8 = 27;
    const INN_UPPER_Z: i8 = 47;
    const BELOW_SEA_LEVEL_Z: i8 = -5;
    /// Trammel, the facet the New Haven inn stands on.
    const TRAMMEL_MAP_INDEX: u8 = 1;

    #[test]
    fn a_map_question_defaults_to_the_floor_the_character_is_on() {
        assert_eq!(
            asked_z(&json!({"x": 3506, "y": 2523}), INN_GROUND_Z),
            INN_GROUND_Z,
            "no z named means the character's own floor"
        );
        assert_eq!(
            asked_z(
                &json!({"x": 3506, "y": 2523, "z": INN_UPPER_Z}),
                INN_GROUND_Z
            ),
            INN_UPPER_Z,
            "a named z asks about that floor instead"
        );
        assert_eq!(
            asked_z(&json!({"z": BELOW_SEA_LEVEL_Z}), INN_GROUND_Z),
            BELOW_SEA_LEVEL_Z,
            "a negative z is a real height, not a missing one"
        );
    }

    /// Felucca, the facet the Britain bank stands on.
    const FELUCCA_MAP_INDEX: u8 = 0;
    /// Two tiles at the top of the ramp measured beside the Britain bank, both
    /// twenty height units above the ground at its foot.
    const ABOVE_THE_RAMP: Point3 = Point3 {
        x: 1421,
        y: 1712,
        z: 20,
    };
    const ACROSS_THE_TOP_X: u16 = 1420;
    const ACROSS_THE_TOP_Y: u16 = 1715;
    /// The height a walk used to be planned to when the caller named none.
    const NO_HEIGHT_NAMED: i8 = 0;

    /// A move with no height named is a move on the character's own floor.
    /// Sea level is somebody else's floor: a character at the top of the ramp
    /// beside the Britain bank is twenty height units above it, and a walk
    /// planned to a tile down there is a walk he is told he cannot make.
    #[test]
    fn a_move_with_no_height_named_walks_on_the_character_own_floor() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let mut inner = test_session();
        inner.uopath = Some(dir);
        {
            let mut world = inner.world.write();
            world.self_state.map = FELUCCA_MAP_INDEX;
            world.self_state.location = ABOVE_THE_RAMP;
        }
        let moved = handle_tool(
            &mut inner,
            ToolCall {
                name: TOOL_MOVE_TO.into(),
                args: json!({ "x": ACROSS_THE_TOP_X, "y": ACROSS_THE_TOP_Y }),
            },
        );
        assert!(moved.ok, "{moved:?}");
        assert_eq!(
            inner.movement.goal,
            Some(Point3::new(
                ACROSS_THE_TOP_X,
                ACROSS_THE_TOP_Y,
                ABOVE_THE_RAMP.z
            )),
            "the walk is planned to his own floor"
        );
        assert!(
            inner.movement.walking(),
            "and the route to it is queued: {:?}",
            inner.movement.path
        );
        assert!(
            !inner
                .tiles()
                .can_walk_from(NO_HEIGHT_NAMED, ACROSS_THE_TOP_X, ACROSS_THE_TOP_Y),
            "read from sea level that tile is no ground at all, or the test proves nothing"
        );

        let travel = handle_tool(
            &mut inner,
            ToolCall {
                name: TOOL_SET_GOAL.into(),
                args: json!({
                    "goal": Goal::Travel { dest: ABOVE_THE_RAMP }.name(),
                    "x": ACROSS_THE_TOP_X,
                    "y": ACROSS_THE_TOP_Y,
                }),
            },
        );
        assert!(travel.ok, "{travel:?}");
        assert_eq!(
            inner.goal,
            Goal::Travel {
                dest: Point3::new(ACROSS_THE_TOP_X, ACROSS_THE_TOP_Y, ABOVE_THE_RAMP.z)
            },
            "a travel goal with no height named is a goal on his own floor too"
        );
    }

    /// Where the character of the radar test stands: the ground floor of the
    /// New Haven inn, in the gap of a wall that carries the floor above.
    const INN_GROUND_X: u16 = 3506;
    const INN_GROUND_Y: u16 = 2526;
    /// Measured from the client files. Of the 21 by 21 tiles the radar draws,
    /// this many answer for the floor above when no height is given, and every
    /// one of them draws open floor where the character's own floor holds a
    /// wall.
    const RADAR_TILES_OF_THE_FLOOR_ABOVE: usize = 17;
    /// The tile east of the character: a wall he cannot pass, and open boards
    /// of the storey above.
    const WALL_EAST_DX: i32 = 1;
    const SAME_ROW_DY: i32 = 0;

    /// The graphic of the inn door that stopped a follower on a live shard.
    const INN_DOOR_GRAPHIC: u16 = 1701;
    /// One tile around the doorway: near enough to catch any static door of it.
    const DOORWAY_SWEEP: u16 = 1;

    /// Why the world model must record the door items the server sends: the
    /// client files hold no door of the New Haven inn at all, and they call its
    /// doorway walkable, so the map alone plans a route into a shut door.
    #[test]
    fn the_new_haven_inn_doorway_is_a_server_item_and_no_map_static() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        assert!(
            map.is_door_graphic(INN_DOOR_GRAPHIC),
            "the client data must call graphic {INN_DOOR_GRAPHIC} a door"
        );
        assert!(
            map.doors_near(INN_GROUND_X, INN_GROUND_Y, DOORWAY_SWEEP)
                .is_empty(),
            "the inn holds no door static, so only the item the server sends names its door"
        );
        assert!(
            map.tile_from(INN_GROUND_Z, INN_GROUND_X, INN_GROUND_Y)
                .walkable(),
            "the doorway reads as walkable, which is why a route runs into a shut door"
        );
    }

    /// The mark the radar draws `dx`,`dy` tiles from the character.
    fn radar_mark(radar: &str, dx: i32, dy: i32) -> char {
        let half = i32::from(RADAR_SIZE) / 2;
        let row = radar
            .lines()
            .nth((dy + half) as usize)
            .unwrap_or_else(|| panic!("radar has no row {dy}"));
        row.chars()
            .nth((dx + half) as usize)
            .unwrap_or_else(|| panic!("radar row {dy} has no cell {dx}"))
    }

    #[test]
    fn the_radar_draws_the_floor_the_character_stands_on() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let map = MulMap::open(&dir, TRAMMEL_MAP_INDEX).expect("open trammel map 1");
        let mut world = World::default();
        world.self_state.location = Point3::new(INN_GROUND_X, INN_GROUND_Y, INN_GROUND_Z);

        let mine = radar_from_floor(&world, &map, INN_GROUND_Z).radar;
        let height_less = render_radar(&world, RadarOptions::default(), |x, y| {
            map.tile(x, y).radar_char()
        });
        assert_ne!(
            mine, height_less,
            "the two floors of the inn must not draw alike"
        );

        let block = TileKind::Block.as_char();
        let walk = TileKind::Walk.as_char();
        assert_eq!(
            radar_mark(&mine, WALL_EAST_DX, SAME_ROW_DY),
            block,
            "the tile east of the character is a wall on his own floor:\n{mine}"
        );
        assert_eq!(
            radar_mark(&height_less, WALL_EAST_DX, SAME_ROW_DY),
            walk,
            "asked with no height the same tile reads as open floor:\n{height_less}"
        );

        let half = i32::from(RADAR_SIZE) / 2;
        let mut of_the_floor_above = 0;
        for dy in -half..=half {
            for dx in -half..=half {
                let drawn = radar_mark(&mine, dx, dy);
                if drawn == radar_mark(&height_less, dx, dy) {
                    continue;
                }
                of_the_floor_above += 1;
                let x = (i32::from(INN_GROUND_X) + dx) as u16;
                let y = (i32::from(INN_GROUND_Y) + dy) as u16;
                assert_eq!(drawn, block, "the radar must draw a wall at {x},{y}");
                assert!(
                    !map.can_walk_from(INN_GROUND_Z, x, y),
                    "the radar must agree with the character's own floor at {x},{y}"
                );
                assert!(
                    map.can_walk(x, y),
                    "and the height-less answer must be the floor above at {x},{y}"
                );
            }
        }
        assert_eq!(
            of_the_floor_above, RADAR_TILES_OF_THE_FLOOR_ABOVE,
            "the inn must hold the floor above, or the test proves nothing:\n{mine}"
        );
    }

    /// The live fault: with one stop distance the follower paced 3, 2, 0, 3, 2
    /// tiles. It walked onto the target's own tile and out again. The band
    /// must hold it still while the target is near, and the destination must
    /// always be a tile beside the target.
    #[test]
    fn follow_stops_next_to_target_and_walks_when_far() {
        const GRID: u16 = 64;
        const ROW: u16 = 20;
        const ME_X: u16 = 20;
        let map = MockMap::new(GRID, GRID);
        let me = Point3::new(ME_X, ROW, 0);
        let mut state = FollowState::default();
        for gap in 1..=movement::FOLLOW_FAR_DISTANCE as u16 {
            let target = Point3::new(ME_X + gap, ROW, 0);
            assert_eq!(
                state.decide(me, target, false),
                FollowDecision::Hold,
                "gap {gap} is inside the band"
            );
        }
        let target = Point3::new(ME_X + movement::FOLLOW_FAR_DISTANCE as u16 + 1, ROW, 0);
        assert_eq!(state.decide(me, target, false), FollowDecision::Repath);
        let standing_there = [target];
        let plan = follow_plan(
            &map,
            me,
            target,
            false,
            &Obstacles {
                soft: &standing_there,
                hard: &[],
                moves: &[],
            },
        )
        .expect("a tile beside the target");
        assert_ne!(
            (plan.dest.x, plan.dest.y),
            (target.x, target.y),
            "a follower never stands on the target"
        );
        assert_eq!(plan.dest.chebyshev(target), 1);
        assert!(!plan
            .steps
            .iter()
            .any(|s| s.x == target.x && s.y == target.y));
    }

    /// Ground truth measured on a live shard, tile for tile. The character
    /// genuinely stood here, on the ground floor of New Haven, for 26 seconds
    /// with no command sent.
    const MEASURED_START: Point3 = Point3 {
        x: 3505,
        y: 2525,
        z: INN_GROUND_Z,
    };
    /// One step east. The client used to report her here the instant it queued
    /// the step, and held that for three seconds. She was never on this tile.
    const MEASURED_EAST: Point3 = Point3 {
        x: 3506,
        y: 2525,
        z: INN_GROUND_Z,
    };
    /// One step south of that, the second tile of the measured walk.
    const MEASURED_SOUTH: Point3 = Point3 {
        x: 3506,
        y: 2526,
        z: INN_GROUND_Z,
    };
    /// One step south again, the end of the measured walk.
    const MEASURED_END: Point3 = Point3 {
        x: 3506,
        y: 2527,
        z: INN_GROUND_Z,
    };
    /// How long each measured fiction stood before the next command.
    const MEASURED_HOLD: Duration = Duration::from_secs(3);
    /// Most of these tests walk, so the pace and the packets are the walking
    /// ones. The ones about several steps on the wire run, because that is
    /// what a follower does behind a runner.
    const WALKING: bool = false;
    const RUNNING: bool = true;
    /// The fastwalk key a session holds until the server sends its own.
    const NO_FASTWALK_KEY: u32 = 0;
    /// One walking step's pace, jitter and all: wait this long after a step
    /// and the pace of a person never holds the next one back. No longer than
    /// that, because a test that waits longer per step runs the server out of
    /// time to answer the steps still on the wire behind it.
    const STEP_PACE: Duration = paced(STEP_WALK_MS);
    /// The same at a run.
    const RUN_PACE: Duration = paced(STEP_RUN_MS);

    /// The longest a step of `base` milliseconds can be held back by the
    /// jitter [`Movement::next_interval`] adds.
    const fn paced(base: u64) -> Duration {
        Duration::from_millis(base + (base * JITTER_PCT as u64) / 100)
    }

    /// A session with no socket: everything the movement pump reads, and
    /// nothing it does not. It is handed the time, so a test walks a route
    /// without waiting for one.
    fn test_session() -> Inner {
        let now = Instant::now();
        Inner {
            id: "movement-test".into(),
            world: Arc::new(RwLock::new(World::new())),
            goal: Goal::Idle,
            persona: Persona::lumberjack_yew(),
            speech: SpeechPolicy::default(),
            outbound: VecDeque::new(),
            compressed: false,
            encryption: EncryptionMode::None,
            cipher: Box::new(IdentityCipher),
            decoder: GameDecoder::with_version(
                PacketTable::for_era(Era::Modern),
                ClientVersion::MODERN,
            ),
            map: MockMap::new(MOCK_GRID, MOCK_GRID),
            uopath: None,
            maps: HashMap::new(),
            facets: Arc::new(FacetCache::default()),
            multi_shapes: None,
            movement: Movement::default(),
            doors: DoorOpener::default(),
            era: Era::Modern,
            version: ClientVersion::MODERN,
            follow: None,
            follow_state: FollowState::default(),
            cliloc: None,
            next_action_at: now - ACTION_BUDGET,
            next_bandage_at: now - ACTION_BUDGET,
            next_heal_potion_at: now - ACTION_BUDGET,
            attack_sent: None,
            target_intent: None,
            loot: None,
            sent_drop: None,
            last_path_fail: None,
            last_fatigued: None,
            last_event_seq: 0,
            last_name_retry: now,
        }
    }

    /// Stands the character on a tile the way a server packet does, facing the
    /// way the route starts, and gives him that route to walk at a walking
    /// pace.
    ///
    /// He faces it because a person who has just walked this way already does,
    /// and a step in the direction he faces goes out as a step. A step in any
    /// other direction turns him first and moves him nowhere; the tests that
    /// measure the turn set the facing themselves.
    fn walks_from(inner: &mut Inner, at: Point3, route: Vec<Point3>) {
        let facing = route.first().and_then(|first| facing_toward(at, *first));
        {
            let mut world = inner.world.write();
            world.self_state.location = at;
            if let Some(facing) = facing {
                world.self_state.direction = facing as u8;
            }
        }
        inner.movement.run_override = Some(WALKING);
        let dest = *route.last().expect("a route has a destination");
        inner.movement.set_path(route, dest);
    }

    /// The same route at a run, which is the pace a follower keeps up with a
    /// runner at and the pace the wire fills at.
    fn runs_from(inner: &mut Inner, at: Point3, route: Vec<Point3>) {
        walks_from(inner, at, route);
        inner.movement.run_override = Some(RUNNING);
    }

    /// The tile the client reports the character on.
    fn reported_at(inner: &Inner) -> Point3 {
        inner.world.read().self_state.location
    }

    /// One reflex tick, the shortest span a walk can be held back by.
    const ONE_TICK: Duration = Duration::from_millis(REFLEX_TICK_MS);
    /// The two ways a shard says the character has no stamina left to move
    /// with: in words, and as the number of the same line in the client string
    /// file.
    const TOO_FATIGUED_IN_WORDS: &str = "You are too fatigued to move.";
    const TOO_FATIGUED_AS_A_NUMBER: &str = "#500110";
    /// Where the text of a system message starts in the packet the server
    /// writes: past the id, the length, the serial, the graphic, the kind, the
    /// hue, the font and the thirty bytes of the speaker's name.
    const ASCII_MESSAGE_NAME_LEN: usize = 30;

    /// One line of speech from the server, as it sends a system message: no
    /// speaker, and the text in plain bytes.
    fn system_message(text: &str) -> Vec<u8> {
        let mut pkt = vec![PKT_ASCII_MESSAGE, 0, 0];
        pkt.extend_from_slice(&Serial::INVALID.0.to_be_bytes());
        pkt.extend_from_slice(&u16::MAX.to_be_bytes());
        pkt.push(SPEECH_SYSTEM);
        pkt.extend_from_slice(&NO_HUE.to_be_bytes());
        pkt.extend_from_slice(&NO_HUE.to_be_bytes());
        pkt.extend_from_slice(&[0; ASCII_MESSAGE_NAME_LEN]);
        pkt.extend_from_slice(text.as_bytes());
        pkt.push(0);
        let len = pkt.len() as u16;
        pkt[1..3].copy_from_slice(&len.to_be_bytes());
        pkt
    }

    /// How many requests one step of a route can cost: the turn that aims it,
    /// where the character does not already face that way, and the step
    /// itself.
    const REQUESTS_PER_STEP: usize = 2;

    /// Pumps until a real step is on the wire, answering the turn that aims it
    /// where the route changes direction. Gives back that step.
    fn step_onto_the_wire(inner: &mut Inner, now: &mut Instant) -> movement::PendingStep {
        for _ in 0..REQUESTS_PER_STEP {
            pump_movement(inner, *now);
            let sent = inner
                .movement
                .in_flight
                .back()
                .cloned()
                .expect("a request goes out");
            if !sent.turn {
                return sent;
            }
            accept_move_ack(inner, sent.sequence);
            *now += STEP_PACE;
        }
        panic!("a step never went out");
    }

    /// Walks one step of the route and has the server agree to it, turning
    /// first where the route changes direction. Gives back the step.
    fn walk_one_step(inner: &mut Inner, now: &mut Instant) -> movement::PendingStep {
        let step = step_onto_the_wire(inner, now);
        accept_move_ack(inner, step.sequence);
        *now += STEP_PACE;
        step
    }

    /// The move requests the character has queued for the wire.
    fn steps_sent(inner: &Inner) -> Vec<Vec<u8>> {
        inner
            .outbound
            .iter()
            .filter(|pkt| pkt.first() == Some(&PKT_MOVE))
            .cloned()
            .collect()
    }

    /// One step, as the server sees it leave.
    fn step_request(direction: Direction, sequence: u8) -> Vec<u8> {
        encode::move_request(direction, WALKING, sequence, NO_FASTWALK_KEY)
    }

    /// A straight walk east from `from`, as many steps long as the test needs.
    fn route_east(from: Point3, steps: usize) -> Vec<Point3> {
        (1..=steps)
            .map(|i| Point3::new(from.x + i as u16, from.y, from.z))
            .collect()
    }

    /// A hillside the shape of the one above the Britain bank: the ground
    /// climbs [`uoterm_nav::STEP_HEIGHT`] for every tile south, which is as
    /// steep as a person walks. Every tile of it stands at a height of its
    /// own, so a walk that carried one height along the slope is caught at
    /// the first step.
    const HILL_X: u16 = 10;
    const HILL_FOOT_Y: u16 = 10;
    const HILL_TILES: u16 = 6;
    const HILL_FOOT_Z: i8 = 0;

    /// The height of the hillside on the tile `y` tiles down the map.
    fn hill_z(y: u16) -> i8 {
        let climbed = (i32::from(y) - i32::from(HILL_FOOT_Y)) * i32::from(uoterm_nav::STEP_HEIGHT);
        (i32::from(HILL_FOOT_Z) + climbed) as i8
    }

    /// One tile of that hillside, at the height of its own ground.
    fn hill_tile(y: u16) -> Point3 {
        Point3::new(HILL_X, y, hill_z(y))
    }

    /// The hillside as a map to walk on.
    fn hillside() -> MockMap {
        let mut map = MockMap::new(MOCK_GRID, MOCK_GRID);
        for step in 0..=HILL_TILES {
            let y = HILL_FOOT_Y + step;
            map.set_z(HILL_X, y, hill_z(y));
        }
        map
    }

    /// Stands the character on that hillside, the way a server packet does,
    /// with the slope cut into the map his session walks on.
    fn stands_on_the_hillside(inner: &mut Inner, y: u16) {
        inner.map = hillside();
        inner.world.write().self_state.location = hill_tile(y);
    }

    /// The refusal packet: its id, the sequence it refuses, the tile the
    /// character really stands on and the way he faces on it.
    const MOVE_REJECT_LEN: usize = 8;
    /// The way a refused character is left facing. A refusal says which way he
    /// looks; no walk cares, so every test uses the one direction.
    const FACING_AFTER_A_REFUSAL: Direction = Direction::North;

    /// One refusal, as the server sends it: the sequence it answers, the tile
    /// it carries, which is where the character really stands, and the way he
    /// is left facing. Every request he had on the wire is thrown away with it.
    ///
    /// The sequence must be one he is waiting on, or the client reads the
    /// packet as the echo of a refusal it has already dealt with.
    fn refusal_at(sequence: u8, at: Point3) -> [u8; MOVE_REJECT_LEN] {
        [
            PKT_MOVE_REJECT,
            sequence,
            (at.x >> 8) as u8,
            at.x as u8,
            (at.y >> 8) as u8,
            at.y as u8,
            FACING_AFTER_A_REFUSAL as u8,
            at.z as u8,
        ]
    }

    /// A step is a request, not a move. The character asks to walk east and
    /// stays where the server put him; only the answer to that request moves
    /// him, and only onto the tile the step was aimed at.
    #[test]
    fn a_queued_step_moves_nobody_and_its_answer_does() {
        let mut inner = test_session();
        walks_from(&mut inner, MEASURED_START, vec![MEASURED_EAST]);
        pump_movement(&mut inner, Instant::now());
        assert_eq!(
            steps_sent(&inner),
            vec![step_request(Direction::East, movement::SEQ_FIRST)],
            "the step east goes out"
        );
        assert_eq!(
            reported_at(&inner),
            MEASURED_START,
            "and the character has not moved: the shard has not agreed to it yet"
        );
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(
            reported_at(&inner),
            MEASURED_EAST,
            "the answer puts him on the tile the step was aimed at"
        );
        assert_eq!(
            inner.world.read().self_state.direction,
            Direction::East as u8,
            "facing east, the way that step took him"
        );
        assert!(!inner.movement.walking(), "and the walk is over");
    }

    /// The measured fault itself. From 3505,2525 the character sends one step
    /// east. The server never answers it. For the three seconds the old client
    /// reported 3506,2525, the tile reported must stay 3505,2525, because that
    /// is the last tile the server put her on.
    #[test]
    fn an_unanswered_step_east_leaves_the_reported_tile_alone() {
        let mut inner = test_session();
        walks_from(&mut inner, MEASURED_START, vec![MEASURED_EAST]);
        let now = Instant::now();
        pump_movement(&mut inner, now);
        let tick = Duration::from_millis(REFLEX_TICK_MS);
        let mut waited = Duration::ZERO;
        while waited < MEASURED_HOLD {
            waited += tick;
            pump_movement(&mut inner, now + waited);
            assert_eq!(
                reported_at(&inner),
                MEASURED_START,
                "no answer came, so nothing may move her: {waited:?} in"
            );
        }
        assert_eq!(
            steps_sent(&inner).len(),
            1,
            "and she never builds a second step on the first"
        );
        assert_eq!(
            inner
                .outbound
                .iter()
                .filter(|pkt| **pkt == encode::resync())
                .count(),
            1,
            "the lost step asks the server once where she is"
        );
        assert!(
            !inner.movement.walking(),
            "the lost step and its route are gone, so nothing wedges her"
        );
    }

    /// The speed cap measured on a live shard: with one step allowed on the
    /// wire the character sent a step every 300 ms, the round trip, while a
    /// runner steps every 200 ms, so he lost ground on every step. The second
    /// step of a route must go out at the pace of a person, and must not wait
    /// for the answer to the first.
    #[test]
    fn the_next_step_goes_out_at_the_pace_and_waits_for_no_answer() {
        const STRAIGHT_STEPS: usize = 2;
        let mut inner = test_session();
        // Straight on, so the pace is all that spaces these two steps: a
        // change of direction costs a turn first, which is measured on its own.
        let route = route_east(MEASURED_START, STRAIGHT_STEPS);
        let second_tile = route[1];
        walks_from(&mut inner, MEASURED_START, route);
        let now = Instant::now();
        pump_movement(&mut inner, now);
        assert_eq!(
            steps_sent(&inner),
            vec![step_request(Direction::East, movement::SEQ_FIRST)],
            "the first step goes out"
        );
        pump_movement(&mut inner, now + STEP_PACE);
        assert_eq!(
            steps_sent(&inner),
            vec![
                step_request(Direction::East, movement::SEQ_FIRST),
                step_request(Direction::East, movement::SEQ_AFTER_WRAP),
            ],
            "and the second follows a step's pace later, unanswered as the first still is"
        );
        assert_eq!(
            reported_at(&inner),
            MEASURED_START,
            "neither step has moved him: the shard has agreed to nothing yet"
        );
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(
            reported_at(&inner),
            MEASURED_EAST,
            "the first answer moves him one tile, and only one"
        );
        accept_move_ack(&mut inner, movement::SEQ_AFTER_WRAP);
        assert_eq!(reported_at(&inner), second_tile, "and so does the second");
    }

    /// The worst of the measured faults. A move request in a direction the
    /// character does not face turns him and moves him nowhere: the server
    /// sets the new location to the old one and answers all the same. So the
    /// same direction has to go out twice, once to turn and once to step.
    ///
    /// Against the code before this test the client sent the step alone and
    /// credited the tile to its answer, so every change of direction put its
    /// idea of where he stood one tile ahead of the truth, and one further
    /// ahead on every corner after that.
    #[test]
    fn a_change_of_direction_turns_him_first_and_the_turn_takes_no_tile() {
        let mut inner = test_session();
        walks_from(
            &mut inner,
            MEASURED_START,
            vec![MEASURED_EAST, MEASURED_SOUTH],
        );
        let now = Instant::now();
        pump_movement(&mut inner, now);
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(reported_at(&inner), MEASURED_EAST, "the step east lands");

        pump_movement(&mut inner, now + STEP_PACE);
        let turn = inner
            .movement
            .in_flight
            .back()
            .cloned()
            .expect("a request goes out");
        assert!(
            turn.turn,
            "he faces east and the route turns south, so he turns"
        );
        assert_eq!(
            inner.movement.path.len(),
            1,
            "and the turn takes no tile out of the route: {:?}",
            inner.movement.path
        );
        accept_move_ack(&mut inner, turn.sequence);
        assert_eq!(
            reported_at(&inner),
            MEASURED_EAST,
            "the answer to a turn moves him nowhere at all"
        );
        assert_eq!(
            inner.world.read().self_state.direction,
            Direction::South as u8,
            "it only changes the way he faces"
        );

        pump_movement(&mut inner, now + STEP_PACE + movement::TURN_PACE);
        let step = inner
            .movement
            .in_flight
            .back()
            .cloned()
            .expect("the step follows the turn");
        assert!(
            !step.turn,
            "the same direction again, and this one is the step"
        );
        assert_eq!(step.direction, Direction::South);
        accept_move_ack(&mut inner, step.sequence);
        assert_eq!(
            reported_at(&inner),
            MEASURED_SOUTH,
            "and that answer is the one that moves him"
        );
    }

    /// A runner fills the wire to the cap without one answer coming back, and
    /// only the cap holds the next step. This is the fix for the speed limit:
    /// the round trip no longer spaces the steps, the running pace does.
    #[test]
    fn a_runner_fills_the_wire_to_the_cap_and_one_answer_makes_room_for_one() {
        let full = movement::IN_FLIGHT_MAX;
        let mut inner = test_session();
        runs_from(
            &mut inner,
            MEASURED_START,
            route_east(MEASURED_START, full + 1),
        );
        let now = Instant::now();
        for sent in 0..full {
            pump_movement(&mut inner, now + RUN_PACE * sent as u32);
            assert_eq!(
                steps_sent(&inner).len(),
                sent + 1,
                "no answer has come back, and {} steps are out",
                sent + 1
            );
        }
        pump_movement(&mut inner, now + RUN_PACE * full as u32);
        assert_eq!(
            steps_sent(&inner).len(),
            full,
            "the cap is full, so this step waits though its pace has passed"
        );
        assert_eq!(
            reported_at(&inner),
            MEASURED_START,
            "and not one of the steps out has moved him"
        );
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        pump_movement(&mut inner, now + RUN_PACE * (full + 1) as u32);
        assert_eq!(
            steps_sent(&inner).len(),
            full + 1,
            "one answer makes room for exactly one more step"
        );
    }

    /// The rule the whole walk is built on, under the load that used to break
    /// it: three steps out and one answer back moves the character one tile.
    /// The tile he is reported on trails the tile he is walking to, and never
    /// runs ahead of the shard.
    #[test]
    fn the_reported_tile_never_runs_ahead_of_the_answers() {
        const STEPS_OUT: usize = 3;
        const {
            assert!(
                movement::IN_FLIGHT_MAX >= STEPS_OUT,
                "the cap must let three steps out for this to prove anything"
            )
        };
        let route = vec![MEASURED_EAST, MEASURED_SOUTH, MEASURED_END];
        let mut inner = test_session();
        walks_from(&mut inner, MEASURED_START, route);
        let now = Instant::now();
        for sent in 0..STEPS_OUT {
            pump_movement(&mut inner, now + STEP_PACE * sent as u32);
        }
        assert_eq!(
            steps_sent(&inner),
            vec![
                step_request(Direction::East, movement::SEQ_FIRST),
                step_request(Direction::South, movement::SEQ_AFTER_WRAP),
                step_request(Direction::South, movement::SEQ_AFTER_WRAP + 1),
            ],
            "three steps are on the wire, each aimed on from the one before it"
        );
        assert_eq!(
            reported_at(&inner),
            MEASURED_START,
            "and the character has not moved a tile"
        );
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(
            reported_at(&inner),
            MEASURED_EAST,
            "one answer, one tile, and no further"
        );
        assert!(
            inner.movement.walking(),
            "the other two steps are still owed their answers"
        );
    }

    /// One answer, as the server sends it: the sequence it confirms and the
    /// notoriety of the character it moves.
    const MOVE_ACK_NOTORIETY: u8 = 1;

    fn move_ack(sequence: u8) -> Vec<u8> {
        vec![PKT_MOVE_ACK, sequence, MOVE_ACK_NOTORIETY]
    }

    /// The answer to a step is what frees the wire for the next one, so the
    /// walk is pumped the moment that answer lands and not only on the next
    /// tick.
    ///
    /// Pumped on the tick alone, every step of a full wire leaves up to a
    /// whole [`REFLEX_TICK_MS`] after its slot opened, and a character loses
    /// that much ground on every step of a long run.
    #[test]
    fn an_answer_off_the_wire_sends_the_next_step_at_once() {
        let full = movement::IN_FLIGHT_MAX;
        let mut inner = test_session();
        walks_from(
            &mut inner,
            MEASURED_START,
            route_east(MEASURED_START, full + 1),
        );
        // Fill the wire at the pace of a person, ending level with now, so
        // that the cap is the only thing holding the next step back.
        let start = Instant::now() - STEP_PACE * full as u32;
        for sent in 0..full {
            pump_movement(&mut inner, start + STEP_PACE * sent as u32);
        }
        assert_eq!(inner.movement.in_flight.len(), full, "the wire is full");
        assert_eq!(steps_sent(&inner).len(), full);
        let oldest = inner
            .movement
            .in_flight
            .front()
            .cloned()
            .expect("a step is waiting for its answer");

        read_from_the_wire(&mut inner, &mut move_ack(oldest.sequence));
        assert_eq!(
            reported_at(&inner),
            oldest.arrives_at,
            "the answer moves him one tile"
        );
        assert_eq!(
            steps_sent(&inner).len(),
            full + 1,
            "and the step it made room for goes out in the same breath, \
             without waiting for a tick"
        );
        assert_eq!(
            inner.movement.in_flight.len(),
            full,
            "so the wire stays full"
        );
    }

    /// A refusal ends every step on the wire at once. The server threw them
    /// all away, so the character snaps to the tile the refusal carries and
    /// starts his counting again from there.
    #[test]
    fn a_refusal_clears_every_step_on_the_wire_and_resets_the_sequence() {
        let mut inner = test_session();
        let route = route_east(MEASURED_START, movement::IN_FLIGHT_MAX);
        walks_from(&mut inner, MEASURED_START, route);
        let now = Instant::now();
        for sent in 0..movement::IN_FLIGHT_MAX {
            pump_movement(&mut inner, now + STEP_PACE * sent as u32);
        }
        assert_eq!(
            inner.movement.in_flight.len(),
            movement::IN_FLIGHT_MAX,
            "the wire is full before the refusal"
        );
        ingest(&mut inner, &refusal_at(movement::SEQ_FIRST, MEASURED_START));
        assert!(
            inner.movement.in_flight.is_empty(),
            "every step the server threw away is gone"
        );
        assert_eq!(
            inner.movement.path.len(),
            movement::IN_FLIGHT_MAX,
            "the route is kept: one refusal is worth one more try at the same step, \
             and the step that was refused goes back at the head of it"
        );
        assert_eq!(
            reported_at(&inner),
            MEASURED_START,
            "he stands where the refusal says he stands"
        );
        assert_eq!(
            inner.movement.sequence,
            movement::SEQ_FIRST,
            "and counts from the start again, as the server does"
        );
        walks_from(&mut inner, MEASURED_START, vec![MEASURED_EAST]);
        pump_movement(
            &mut inner,
            now + STEP_PACE * (movement::IN_FLIGHT_MAX + 1) as u32,
        );
        assert_eq!(
            steps_sent(&inner).last().cloned(),
            Some(step_request(Direction::East, movement::SEQ_FIRST)),
            "so the next step goes out on the first sequence, from the tile he is on"
        );
    }

    /// Open ground the mock map answers for, well inside its grid.
    const FOLLOW_ROW: u16 = 1000;
    const FOLLOW_START_X: u16 = 1000;

    /// The measured fault, driven as the walk it was measured on. The operator
    /// walked a long route and the gap was read 45 times: mean 4.2, median 1,
    /// worst 18. The median says she holds station beside him when he stops.
    /// The mean and the worst say she never wins back a tile she has lost, and
    /// she lost one every other step, because a running step of
    /// [`STEP_RUN_MS`] could only leave on a [`REFLEX_TICK_MS`] tick and the
    /// step after it was measured from when it left.
    ///
    /// Two things keep the gap shut, and this drives both. The pace is a
    /// cadence, so she walks at the pace she is walking at and not slower. And
    /// she runs while she is further behind than the band allows, because two
    /// people walking at one pace stay exactly as far apart as they were.
    ///
    /// The band is not the fault and is not touched: she settles at
    /// [`movement::FOLLOW_NEAR_DISTANCE`] and starts closing past
    /// [`movement::FOLLOW_FAR_DISTANCE`], and the one tile past the band the
    /// gap reaches here is the hysteresis doing its work. Against the code
    /// before this test the gap ran away to 12 tiles and this failed.
    const FOLLOW_ROUTE_TILES: u16 = 40;
    /// The pace the operator kept: one tile every walking step.
    const TARGET_PACE: Duration = Duration::from_millis(STEP_WALK_MS);
    /// The clock the session runs its walk on.
    const SESSION_TICK: Duration = Duration::from_millis(REFLEX_TICK_MS);
    /// How long she is given to settle beside him once he stands still. One
    /// tick for every tile of the route is longer than she can ever need.
    const SETTLE_TICKS: usize = FOLLOW_ROUTE_TILES as usize;
    /// The widest gap a follower may open while the target keeps moving: the
    /// band, and the one tile the hysteresis costs before she starts closing.
    const FOLLOW_GAP_MAX: u32 = movement::FOLLOW_FAR_DISTANCE + 1;

    /// One tick of a session that is following: the target walks its own
    /// clock, she plans and steps on hers, and the shard answers her step.
    fn follow_and_step(inner: &mut Inner, target_at: Point3, now: Instant) {
        follow_tick(inner, target_at, WALKING);
        pump_movement(inner, now);
        if let Some(step) = inner.movement.in_flight.front().cloned() {
            accept_move_ack(inner, step.sequence);
        }
    }

    #[test]
    fn a_follower_keeps_the_gap_inside_the_band_while_the_target_walks_on() {
        let mut inner = test_session();
        let start = Point3::new(FOLLOW_START_X, FOLLOW_ROW, 0);
        inner.world.write().self_state.location = start;
        let mut target_at = Point3::new(
            FOLLOW_START_X + movement::FOLLOW_NEAR_DISTANCE as u16,
            FOLLOW_ROW,
            0,
        );
        assert_eq!(
            start.chebyshev(target_at),
            movement::FOLLOW_NEAR_DISTANCE,
            "she starts beside him, settled, which is where the fault started"
        );

        let mut now = Instant::now();
        let mut stepped_at = now;
        let mut walked = 0;
        let mut gaps: Vec<u32> = Vec::new();
        while walked < FOLLOW_ROUTE_TILES {
            if now.duration_since(stepped_at) >= TARGET_PACE {
                stepped_at = now;
                walked += 1;
                target_at = Point3::new(target_at.x + 1, target_at.y, target_at.z);
            }
            follow_and_step(&mut inner, target_at, now);
            gaps.push(reported_at(&inner).chebyshev(target_at));
            now += SESSION_TICK;
        }

        let worst = gaps.iter().copied().max().unwrap_or_default();
        assert!(
            worst <= FOLLOW_GAP_MAX,
            "the gap never opens past {FOLLOW_GAP_MAX} over {} readings, and the worst is {worst}: {gaps:?}",
            gaps.len()
        );

        // He stands still, and she holds station inside the band. Which tile
        // of the band she holds on is the hysteresis and not the fault: a
        // follower settled beside him puts up with the gap his next step
        // opens, which is what stops her stepping in and out on the spot.
        for _ in 0..SETTLE_TICKS {
            follow_and_step(&mut inner, target_at, now);
            now += SESSION_TICK;
        }
        let settled = reported_at(&inner).chebyshev(target_at);
        assert!(
            settled <= movement::FOLLOW_FAR_DISTANCE,
            "and she holds station beside him once he stops, at {settled} tiles"
        );
    }

    /// The other half of the measured fault, and the half a steady pace on its
    /// own can never mend. She was found 18 tiles behind: one long stall, one
    /// refusal, one climb the route had to be planned again for, and the
    /// ground is lost. Two people walking at one pace stay exactly as far
    /// apart as they were, so a follower that only ever matches the pace of
    /// the person ahead keeps every tile it has already lost, for as long as
    /// he keeps walking.
    ///
    /// She must close it the way a person does, by moving faster than the
    /// person she is catching up with. Against the code before this test she
    /// walked at his pace and the gap never shut at all.
    const FOLLOW_LEFT_BEHIND: u16 = 6;

    #[test]
    fn a_follower_left_behind_closes_the_gap_while_the_target_walks_on() {
        let mut inner = test_session();
        let start = Point3::new(FOLLOW_START_X, FOLLOW_ROW, 0);
        inner.world.write().self_state.location = start;
        let mut target_at = Point3::new(FOLLOW_START_X + FOLLOW_LEFT_BEHIND, FOLLOW_ROW, 0);
        assert!(
            start.chebyshev(target_at) > FOLLOW_GAP_MAX,
            "she starts well behind him, where the measurement found her"
        );

        let mut now = Instant::now();
        let mut stepped_at = now;
        let mut walked = 0;
        let mut gaps: Vec<u32> = Vec::new();
        while walked < FOLLOW_ROUTE_TILES {
            if now.duration_since(stepped_at) >= TARGET_PACE {
                stepped_at = now;
                walked += 1;
                target_at = Point3::new(target_at.x + 1, target_at.y, target_at.z);
            }
            follow_and_step(&mut inner, target_at, now);
            gaps.push(reported_at(&inner).chebyshev(target_at));
            now += SESSION_TICK;
        }

        let caught_up = gaps
            .iter()
            .position(|gap| *gap <= movement::FOLLOW_FAR_DISTANCE)
            .unwrap_or_else(|| panic!("she never catches him up at all: {gaps:?}"));
        let after: Vec<u32> = gaps[caught_up..].to_vec();
        let worst = after.iter().copied().max().unwrap_or_default();
        assert!(
            worst <= FOLLOW_GAP_MAX,
            "and once she has, the gap stays shut for the rest of his route: {after:?}"
        );
    }

    /// A follower behind a runner keeps steps on the wire, so the tile it is
    /// reported on trails the tile it is walking to. It must read the gap from
    /// the tile those steps leave it on: from the reported tile it would close
    /// a gap it has already closed, and walk a circle round the target it is
    /// about to stand beside.
    #[test]
    fn a_follower_reads_the_gap_from_the_steps_it_has_already_sent() {
        let start = Point3::new(FOLLOW_START_X, FOLLOW_ROW, 0);
        // A walk that ends beside the target, and one tile of it still queued.
        let route = vec![
            Point3::new(FOLLOW_START_X + 1, FOLLOW_ROW, 0),
            Point3::new(FOLLOW_START_X + 2, FOLLOW_ROW, 0),
            Point3::new(FOLLOW_START_X + 3, FOLLOW_ROW + 1, 0),
            Point3::new(FOLLOW_START_X + 4, FOLLOW_ROW + 1, 0),
        ];
        let beside_the_target = route[2];
        let target_at = Point3::new(FOLLOW_START_X + 4, FOLLOW_ROW, 0);
        let mut inner = test_session();
        inner.follow = Some(SOMEBODY_ELSE);
        inner
            .world
            .write()
            .mobiles
            .insert(SOMEBODY_ELSE, standing_at(target_at));
        runs_from(&mut inner, start, route);
        let now = Instant::now();
        // Three steps and the turn that aims the last of them, which changes
        // direction: four requests, which is the whole of what the wire holds.
        let requests_out = movement::IN_FLIGHT_MAX;
        for sent in 0..requests_out {
            pump_movement(&mut inner, now + RUN_PACE * sent as u32);
        }
        assert_eq!(
            inner.movement.in_flight.len(),
            requests_out,
            "three steps and the turn that aims the last are out"
        );
        assert_eq!(reported_at(&inner), start, "and none of them has moved him");
        assert_eq!(
            inner.movement.stepping_from(reported_at(&inner)),
            beside_the_target,
            "but they leave him beside the target"
        );
        assert_eq!(
            FollowState::default().decide(reported_at(&inner), target_at, true),
            FollowDecision::Repath,
            "the tile he is reported on is four tiles off the target, and would send him closing"
        );
        follow_tick(&mut inner, target_at, RUNNING);
        assert!(
            inner.movement.path.is_empty(),
            "he is beside the target once those steps land, so he stops queuing more: {:?}",
            inner.movement.path
        );
        assert_eq!(
            inner.movement.in_flight.len(),
            requests_out,
            "and holds on to the requests the server is going to answer"
        );
        // Each answer puts him on the tile its own request carries: a step on
        // the tile it was aimed at, and the turn on the tile he already
        // stands on.
        let owed: Vec<movement::PendingStep> = inner.movement.in_flight.iter().cloned().collect();
        for (i, request) in owed.iter().enumerate() {
            accept_move_ack(&mut inner, request.sequence);
            assert_eq!(
                reported_at(&inner),
                request.arrives_at,
                "answer {i} puts him on the tile that request carries"
            );
        }
        assert_eq!(
            reported_at(&inner),
            beside_the_target,
            "and the last of them leaves him beside the target"
        );
    }

    /// The measured walk, done the way this client now walks it: east, south,
    /// south, one confirmed step at a time, ending on the tile it was aimed at
    /// with the arrival recorded from that same tile.
    /// How many changes of direction the measured walk has in it: east, then
    /// south. Each one costs a turn before the step.
    const TURNS_ON_THE_MEASURED_WALK: usize = 1;

    #[test]
    fn a_route_is_walked_one_confirmed_step_at_a_time() {
        let route = [MEASURED_EAST, MEASURED_SOUTH, MEASURED_END];
        let mut inner = test_session();
        walks_from(&mut inner, MEASURED_START, route.to_vec());
        let mut now = Instant::now();
        for (i, tile) in route.iter().enumerate() {
            let step = walk_one_step(&mut inner, &mut now);
            assert_eq!(
                step.arrives_at, *tile,
                "step {i} is aimed where the route says"
            );
            assert_eq!(reported_at(&inner), *tile, "step {i} is confirmed");
        }
        assert_eq!(
            steps_sent(&inner),
            vec![
                step_request(Direction::East, movement::SEQ_FIRST),
                // East to south is a change of direction, so the same
                // direction goes out twice: once to turn, once to step.
                step_request(Direction::South, movement::SEQ_AFTER_WRAP),
                step_request(Direction::South, movement::SEQ_AFTER_WRAP + 1),
                step_request(Direction::South, movement::SEQ_AFTER_WRAP + 2),
            ],
            "the steps go out in the order the route names them"
        );
        pump_movement(&mut inner, now);
        assert_eq!(
            steps_sent(&inner).len(),
            route.len() + TURNS_ON_THE_MEASURED_WALK,
            "the walk is done, and nothing more goes out"
        );
        assert_eq!(
            inner
                .world
                .read()
                .events
                .iter()
                .filter(|e| e.kind == uoterm_world::EventKind::Arrived)
                .map(|e| e.text.clone())
                .collect::<Vec<_>>(),
            vec![format!("{MEASURED_END}")],
            "and the arrival is recorded on the tile the server confirmed"
        );
    }

    /// The server refuses a step and says where the character really stands.
    /// He snaps to that tile, throws the route away and asks the server to say
    /// it all again.
    #[test]
    fn a_refusal_snaps_the_character_to_the_tile_it_carries() {
        let mut inner = test_session();
        walks_from(
            &mut inner,
            MEASURED_EAST,
            vec![MEASURED_SOUTH, MEASURED_END],
        );
        pump_movement(&mut inner, Instant::now());
        ingest(&mut inner, &refusal_at(movement::SEQ_FIRST, MEASURED_START));
        assert_eq!(
            reported_at(&inner),
            MEASURED_START,
            "the tile the refusal carries is where he is"
        );
        assert_eq!(
            inner.world.read().self_state.direction,
            FACING_AFTER_A_REFUSAL as u8,
            "facing the way it says as well"
        );
        assert!(
            inner.movement.in_flight.is_empty(),
            "every request the server threw away is gone"
        );
        assert_eq!(
            inner.movement.path.front().copied(),
            Some(MEASURED_SOUTH),
            "and the refused step goes back at the head of the route, for one more try"
        );
        assert!(
            inner.outbound.iter().any(|pkt| *pkt == encode::resync()),
            "and he asks the server to say where everything is"
        );
    }

    /// Walking changes how high the character stands. The route works out the
    /// height each step arrives at, and the answer to that step is what writes
    /// it down, so one step up the hillside raises his recorded height by the
    /// rise of that tile and one step back down lowers it again.
    #[test]
    fn a_confirmed_step_uphill_raises_the_recorded_height_and_downhill_lowers_it() {
        let mut inner = test_session();
        stands_on_the_hillside(&mut inner, HILL_FOOT_Y);
        let foot = hill_tile(HILL_FOOT_Y);
        let up = hill_tile(HILL_FOOT_Y + 1);
        assert_eq!(foot.z, HILL_FOOT_Z);
        assert_eq!(up.z, HILL_FOOT_Z + uoterm_nav::STEP_HEIGHT);

        let now = Instant::now();
        walks_from(&mut inner, foot, vec![up]);
        pump_movement(&mut inner, now);
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(
            reported_at(&inner),
            up,
            "the step uphill puts him on that tile at the height of its own ground"
        );
        assert_eq!(
            reported_at(&inner).z,
            HILL_FOOT_Z + uoterm_nav::STEP_HEIGHT,
            "which is higher than the tile he left"
        );

        walks_from(&mut inner, up, vec![foot]);
        pump_movement(&mut inner, now + STEP_PACE);
        accept_move_ack(&mut inner, movement::SEQ_AFTER_WRAP);
        assert_eq!(
            reported_at(&inner),
            foot,
            "and the step back down puts him at the height down there"
        );
        assert_eq!(reported_at(&inner).z, HILL_FOOT_Z);
    }

    /// A walk up the hillside, one confirmed step at a time. Every step stands
    /// at the height of its own tile, not only the one the walk ends on: a
    /// walk that wrote the height down once would leave him climbing six
    /// tiles and reporting the height of the first.
    #[test]
    fn a_route_uphill_records_the_height_of_every_step_and_not_only_the_last() {
        let mut inner = test_session();
        stands_on_the_hillside(&mut inner, HILL_FOOT_Y);
        let foot = hill_tile(HILL_FOOT_Y);
        let route: Vec<Point3> = (1..=HILL_TILES)
            .map(|i| hill_tile(HILL_FOOT_Y + i))
            .collect();
        walks_from(&mut inner, foot, route.clone());

        let mut now = Instant::now();
        for (taken, tile) in route.iter().enumerate() {
            pump_movement(&mut inner, now);
            accept_move_ack(&mut inner, taken as u8);
            assert_eq!(
                reported_at(&inner),
                *tile,
                "step {taken} of the climb stands on its own tile"
            );
            assert_eq!(
                reported_at(&inner).z,
                hill_z(tile.y),
                "at the height of the ground there"
            );
            now += STEP_PACE;
        }
        assert_eq!(
            reported_at(&inner).z,
            hill_z(HILL_FOOT_Y + HILL_TILES),
            "and the top of the hillside is as high as the climb took him"
        );
    }

    /// A refusal is the server's word on where the character stands, height
    /// and all. He snaps to the tile it carries at the height it carries,
    /// whatever height the walk was expecting to reach.
    #[test]
    fn a_refusal_snaps_the_character_to_the_height_it_carries() {
        let mut inner = test_session();
        stands_on_the_hillside(&mut inner, HILL_FOOT_Y);
        let foot = hill_tile(HILL_FOOT_Y);
        let up = hill_tile(HILL_FOOT_Y + 1);
        walks_from(&mut inner, foot, vec![up, hill_tile(HILL_FOOT_Y + 2)]);
        pump_movement(&mut inner, Instant::now());
        assert_ne!(
            up.z, foot.z,
            "the step the walk expected climbs, or the test proves nothing"
        );

        ingest(&mut inner, &refusal_at(movement::SEQ_FIRST, foot));
        assert_eq!(
            reported_at(&inner),
            foot,
            "the tile the refusal carries is where he is"
        );
        assert_eq!(
            reported_at(&inner).z,
            HILL_FOOT_Z,
            "at the height the refusal carries, not the one the step was aimed at"
        );
        assert!(
            inner.movement.in_flight.is_empty(),
            "and every request the server threw away is gone"
        );
    }

    /// A follower climbs with the person it follows. The walk a follower plans
    /// carries the height of every tile of the slope, and each confirmed step
    /// writes that height down: it was a follow up a hillside that the fault
    /// was measured on, and a follower whose height never changed was a
    /// follower who lost the person above her.
    #[test]
    fn a_follower_climbing_a_slope_records_the_height_of_every_step() {
        const TARGET_TILES_AHEAD: u16 = 3;
        let mut inner = test_session();
        stands_on_the_hillside(&mut inner, HILL_FOOT_Y);
        let target_at = hill_tile(HILL_FOOT_Y + TARGET_TILES_AHEAD);
        follow_tick(&mut inner, target_at, WALKING);

        let climb: Vec<Point3> = inner.movement.path.iter().copied().collect();
        assert!(
            !climb.is_empty(),
            "the follower has a way up to the person above her"
        );
        assert_eq!(
            climb,
            (1..=climb.len())
                .map(|i| hill_tile(HILL_FOOT_Y + i as u16))
                .collect::<Vec<_>>(),
            "every tile of the climb carries the height of its own ground"
        );

        let mut now = Instant::now();
        for (taken, tile) in climb.iter().enumerate() {
            walk_one_step(&mut inner, &mut now);
            assert_eq!(
                reported_at(&inner).z,
                hill_z(tile.y),
                "step {taken} of the follow is recorded at the height of the ground it reached"
            );
        }
        assert_eq!(
            reported_at(&inner).chebyshev(target_at),
            movement::FOLLOW_NEAR_DISTANCE,
            "and she ends up beside the person she follows"
        );
    }

    /// The measured fault, on the ground it was measured on.
    ///
    /// A character following a player up the hillside above the Britain bank
    /// sat at 1419,1709 believing her height was 1. The ground there is 18.
    /// Her height had never changed because every step she took carried the
    /// height of the tile she left. A step confirmed onto that tile must
    /// record the ground of it, and from that height she plans the climb the
    /// player walked ahead of her.
    const HILLSIDE_ABOVE_THE_BANK_X: u16 = 1419;
    const HILLSIDE_ABOVE_THE_BANK_Y: u16 = 1709;
    /// The ground of that tile, read from the client files.
    const HILLSIDE_GROUND_Z: i8 = 18;
    /// The height she believed she was at, carried up the hillside from the
    /// flat below it.
    const HILLSIDE_BELIEVED_Z: i8 = 1;
    /// One tile north, where she stepped from. Its ground is one unit lower.
    const HILLSIDE_NORTH_Y: u16 = HILLSIDE_ABOVE_THE_BANK_Y - 1;
    /// The next tile south, and a tile further up the slope where the hilltop
    /// levels off. Both are heights she could not plan to while she believed
    /// she stood at 1.
    const HILLSIDE_NEXT_Y: u16 = HILLSIDE_ABOVE_THE_BANK_Y + 1;
    const HILLSIDE_NEXT_Z: i8 = 19;
    const HILLSIDE_TOP_Y: u16 = 1717;
    const HILLSIDE_TOP_Z: i8 = 20;

    /// A session standing on the Felucca hillside, with the client files open.
    fn on_the_hillside_above_the_bank(dir: PathBuf, at: Point3) -> Inner {
        let mut inner = test_session();
        inner.uopath = Some(dir);
        {
            let mut world = inner.world.write();
            world.self_state.map = FELUCCA_MAP_INDEX;
            world.self_state.location = at;
        }
        inner.ensure_facet();
        inner
    }

    #[test]
    fn the_measured_hillside_records_its_own_ground_and_plans_the_climb_from_it() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        // She stands one tile north of it, believing the height she carried up
        // from the flat.
        let stale = Point3::new(
            HILLSIDE_ABOVE_THE_BANK_X,
            HILLSIDE_NORTH_Y,
            HILLSIDE_BELIEVED_Z,
        );
        let mut inner = on_the_hillside_above_the_bank(dir, stale);
        let arrives = Point3::new(
            HILLSIDE_ABOVE_THE_BANK_X,
            HILLSIDE_ABOVE_THE_BANK_Y,
            HILLSIDE_GROUND_Z,
        );
        assert_eq!(
            inner
                .tiles()
                .tile_from(HILLSIDE_GROUND_Z, arrives.x, arrives.y)
                .z,
            HILLSIDE_GROUND_Z,
            "the client files put the ground of that tile at {HILLSIDE_GROUND_Z}"
        );

        assert!(
            queue_move(&mut inner, arrives),
            "the step onto the hillside is planned"
        );
        pump_movement(&mut inner, Instant::now());
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(
            reported_at(&inner),
            arrives,
            "the step confirmed onto that tile records the ground of it"
        );
        assert_eq!(
            reported_at(&inner).z,
            HILLSIDE_GROUND_Z,
            "height {HILLSIDE_GROUND_Z}, and never the {HILLSIDE_BELIEVED_Z} she carried up"
        );

        // From the ground she really stands on, the climb the player walked
        // ahead of her plans, step by step, up to the height of each tile.
        let next = Point3::new(HILLSIDE_ABOVE_THE_BANK_X, HILLSIDE_NEXT_Y, HILLSIDE_NEXT_Z);
        assert!(queue_move(&mut inner, next), "the next step up plans");
        assert_eq!(
            inner.movement.path.back().copied(),
            Some(next),
            "and it ends on that tile at height {HILLSIDE_NEXT_Z}"
        );

        let top = Point3::new(HILLSIDE_ABOVE_THE_BANK_X, HILLSIDE_TOP_Y, HILLSIDE_TOP_Z);
        inner.movement.hold();
        assert!(
            queue_move(&mut inner, top),
            "and so does the walk to the top"
        );
        assert_eq!(
            inner.movement.path.back().copied(),
            Some(top),
            "which ends on the level ground at height {HILLSIDE_TOP_Z}"
        );
        assert!(
            inner
                .movement
                .path
                .iter()
                .all(|step| step.z >= HILLSIDE_NEXT_Z),
            "every step of it climbs and none of them carries the height she came from: {:?}",
            inner.movement.path
        );
    }

    /// Being told to stop ends the walk, but not the step already on the wire.
    /// The server is going to allow that step, and the tile it lands on is
    /// where the character really stands, so its answer still counts. Dropping
    /// it would leave him reported one tile behind himself.
    #[test]
    fn stopping_still_counts_the_step_already_on_the_wire() {
        let mut inner = test_session();
        walks_from(
            &mut inner,
            MEASURED_START,
            vec![MEASURED_EAST, MEASURED_SOUTH],
        );
        pump_movement(&mut inner, Instant::now());
        let stopped = handle_tool(
            &mut inner,
            ToolCall {
                name: TOOL_STOP.into(),
                args: json!({}),
            },
        );
        assert!(stopped.ok, "{stopped:?}");
        assert!(
            inner.movement.path.is_empty(),
            "the rest of the route is dropped"
        );
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(
            reported_at(&inner),
            MEASURED_EAST,
            "the step the server allowed put him there all the same"
        );
    }

    /// The serial the world files the other player under.
    const SOMEBODY_ELSE: Serial = Serial(0x0100_0001);
    /// The body of a human player.
    const PLAYER_BODY: u16 = 400;
    const NO_HUE: u16 = 0;
    /// Everything about the other player a walk does not care about.
    const BLANK: u8 = 0;

    /// Another player, standing on one tile. Only that tile matters to a walk.
    fn standing_at(at: Point3) -> uoterm_world::Mobile {
        uoterm_world::Mobile {
            serial: SOMEBODY_ELSE,
            name: String::new(),
            body: PLAYER_BODY,
            hue: NO_HUE,
            location: at,
            direction: Direction::North as u8,
            running: false,
            notoriety: BLANK,
            flags: BLANK,
            hits: None,
            hits_max: None,
            equipment: Vec::new(),
        }
    }

    /// The measured failure, from the move down. A character inside the New
    /// Haven inn is told to walk outside while another player stands in the
    /// corridor one tile wide that is the only way out. Every walk down that
    /// corridor fails while the player stands in it, so the move must send the
    /// character to the shut door instead of reporting no path at all.
    ///
    /// The mobiles reach [`door_in_the_way`] on their own here, and they must
    /// block the walk to the door without hiding the door itself.
    #[test]
    fn a_player_in_the_corridor_does_not_hide_the_door_from_a_move() {
        let inn = movement::tests::inn_corridor();
        let mut inner = test_session();
        inner.map = inn.map;
        {
            let mut world = inner.world.write();
            world.self_state.location = inn.character;
            world.note_door(inn.door.serial, inn.door.graphic, inn.door.location);
            world
                .mobiles
                .insert(SOMEBODY_ELSE, standing_at(inn.somebody_in_the_way));
        }
        assert!(
            queue_move(&mut inner, inn.outside),
            "the way out is a shut door, and the player in the corridor does not hide it"
        );
        assert_eq!(
            inner.movement.path.back().copied(),
            Some(inn.in_front_of_the_door),
            "he walks to the tile he opens the door from: {:?}",
            inner.movement.path
        );
        assert!(
            !inner
                .movement
                .path
                .iter()
                .any(|step| step.x == inn.somebody_in_the_way.x
                    && step.y == inn.somebody_in_the_way.y),
            "and never onto the player standing in the corridor: {:?}",
            inner.movement.path
        );
        assert!(
            inner
                .world
                .read()
                .events
                .iter()
                .all(|ev| ev.kind != uoterm_world::EventKind::PathFailed),
            "a door in the way is not a path failure"
        );
    }

    /// Ground truth measured on a live shard around the Britain bank in
    /// Felucca, map index 0, with every packet traced. In two minutes the
    /// server confirmed 627 steps, refused 89 and 16 routes were abandoned.
    /// The refusals piled up on single tiles: 35 came off this one, 23 off
    /// 1493,1626 and 10 off 1494,1622. The client map files were read at every
    /// one of them and report open ground with no impassable static at all,
    /// and at two of them all eight neighbouring tiles read as walkable while
    /// the character was still refused. A player house stands there; it is
    /// built on the server and is in no client map file, so the refusal is the
    /// only word this client ever gets that anything is in the way.
    ///
    /// The mock ground is flat, so the measured height of 10 is written here
    /// in words and not in the tile. What the refusals are about is x and y.
    const REFUSED_FROM: Point3 = Point3 {
        x: 1438,
        y: 1659,
        z: 0,
    };
    /// The wall of the house, running north to south one tile east of her.
    /// Nothing in the client map files says that it is there.
    const HOUSE_WALL_X: u16 = REFUSED_FROM.x + 1;
    const HOUSE_WALL_HALF: u16 = 4;
    const HOUSE_WALL_FROM_Y: u16 = REFUSED_FROM.y - HOUSE_WALL_HALF;
    const HOUSE_WALL_TO_Y: u16 = REFUSED_FROM.y + HOUSE_WALL_HALF;
    const HOUSE_WALL_TILES: usize = (HOUSE_WALL_HALF * 2 + 1) as usize;
    /// The tile she was refused at 35 times: due east of her, in that wall.
    const THE_HOUSE_WALL: Point3 = Point3 {
        x: HOUSE_WALL_X,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };
    /// Where she was walking: straight past the house.
    const PAST_THE_HOUSE: Point3 = Point3 {
        x: REFUSED_FROM.x + 3,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };
    /// How many ticks the walk past the house is given. Long enough for the
    /// way around a wall of [`HOUSE_WALL_TILES`] tiles, and far short of the
    /// 35 refusals one tile drew on the live shard.
    const HOUSE_WALK_TICKS: usize = 60;

    /// The wall of the house, as only the server knows it. The mock map calls
    /// every one of these tiles open ground, exactly as the client files do.
    fn in_the_house_wall(at: Point3) -> bool {
        at.x == HOUSE_WALL_X && (HOUSE_WALL_FROM_Y..=HOUSE_WALL_TO_Y).contains(&at.y)
    }

    /// The long side of a building, running north to south one tile east of
    /// her, as only the server knows it. The largest house in Ultima Online
    /// stands on 18 by 18 tiles, so this is the longest wall she can meet.
    const LONG_WALL_HALF: u16 = 9;
    const LONG_WALL_FROM_Y: u16 = REFUSED_FROM.y - LONG_WALL_HALF;
    const LONG_WALL_TO_Y: u16 = REFUSED_FROM.y + LONG_WALL_HALF - 1;
    const LONG_WALL_TILES: usize = (LONG_WALL_HALF * 2) as usize;
    /// Where she is walking: straight past that wall.
    const PAST_THE_LONG_WALL: Point3 = Point3 {
        x: REFUSED_FROM.x + 3,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };
    /// How many ticks the walk past that wall is given. The way round the end
    /// of an 18 tile wall is under thirty steps, and this is far short of the
    /// 68 refusals one building drew on the live shard.
    const LONG_WALL_WALK_TICKS: usize = 120;

    /// True while that tile stands in the long wall.
    fn in_the_long_wall(at: Point3) -> bool {
        at.x == HOUSE_WALL_X && (LONG_WALL_FROM_Y..=LONG_WALL_TO_Y).contains(&at.y)
    }

    /// True when the queued route crosses that tile.
    fn route_crosses(inner: &Inner, at: Point3) -> bool {
        inner
            .movement
            .path
            .iter()
            .any(|step| step.x == at.x && step.y == at.y)
    }

    /// Clears the wire and the queued route, so the next route is planned from
    /// the tile the server has the character on and carries nothing over.
    fn ready_to_plan_again(inner: &mut Inner) {
        inner.movement.clear_in_flight();
        inner.movement.hold();
    }

    // The tests below walk one refusal down each rung of the ladder, in the
    // order the rungs are tried. A refusal has many causes and only one of
    // them is a wall, so going straight from a refusal to a blocked tile marks
    // a tile over and over for reasons no mark can fix: that is what filed 68
    // refusals against one tile beside the Britain bank.

    /// One refusal makes the server drop every request that reached it after
    /// the one it refused, and it answers some of those with a refusal of
    /// their own. Acting on the echo would block a second tile for a step that
    /// was never really refused.
    #[test]
    fn a_refusal_on_a_sequence_he_is_not_waiting_on_is_a_stale_echo() {
        let mut inner = test_session();
        let mut now = Instant::now();
        // Two steps out. The server refuses the first and drops the second,
        // and answers the second with a refusal of its own.
        walks_from(&mut inner, REFUSED_FROM, route_east(REFUSED_FROM, 2));
        let first = step_onto_the_wire(&mut inner, &mut now);
        now += STEP_PACE;
        pump_movement(&mut inner, now);
        let dropped = inner
            .movement
            .in_flight
            .back()
            .cloned()
            .expect("the second step is out");
        assert_ne!(first.sequence, dropped.sequence);
        assert_eq!(
            refuse_step(&mut inner, first.sequence, REFUSED_FROM, now),
            Refusal::TryTheDoor,
            "the first refusal is the real one"
        );

        // He has already put the refused step back and asked again, so the
        // wire holds a request of its own by the time the echo lands.
        now += DOOR_RETRY_WAIT + STEP_PACE;
        let asked_again = step_onto_the_wire(&mut inner, &mut now);
        assert_eq!(
            refuse_step(&mut inner, dropped.sequence, REFUSED_FROM, now),
            Refusal::StaleEcho,
            "the echo carries a sequence he is not waiting on"
        );
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "so the echo marks nothing"
        );
        assert_eq!(
            inner.movement.in_flight.front().map(|out| out.sequence),
            Some(asked_again.sequence),
            "and it does not throw away the request he really is waiting on"
        );
    }

    /// A shard says the character is too tired to move and then refuses his
    /// step. That is his own stamina and nothing in the way, so he rests and
    /// tries again rather than marking the ground in front of him.
    #[test]
    fn a_refusal_just_after_the_server_says_he_is_too_tired_is_about_his_stamina() {
        for said in [TOO_FATIGUED_IN_WORDS, TOO_FATIGUED_AS_A_NUMBER] {
            let mut inner = test_session();
            let mut now = Instant::now();
            walks_from(&mut inner, REFUSED_FROM, vec![THE_HOUSE_WALL]);
            let step = step_onto_the_wire(&mut inner, &mut now);
            ingest(&mut inner, &system_message(said));
            assert_eq!(
                refuse_step(&mut inner, step.sequence, REFUSED_FROM, now),
                Refusal::Fatigued,
                "{said:?} is the server saying he has no stamina left"
            );
            assert!(
                inner.movement.blocked.tiles().is_empty(),
                "nothing is in his way, so nothing is marked"
            );
            assert!(
                !inner.movement.ready(now + FATIGUE_WAIT - ONE_TICK),
                "and he rests before he asks again"
            );
        }
    }

    /// The server does not test mobiles for a player move at all: it turns the
    /// step into a shove and refuses the shove unless stamina is full. So a
    /// refusal at an occupied tile is a moment's business, and a character who
    /// marks it walks the long way round every person he meets.
    #[test]
    fn a_refusal_at_an_occupied_tile_waits_for_the_person_to_move_on() {
        let mut inner = test_session();
        let mut now = Instant::now();
        inner
            .world
            .write()
            .mobiles
            .insert(SOMEBODY_ELSE, standing_at(THE_HOUSE_WALL));
        walks_from(&mut inner, REFUSED_FROM, vec![THE_HOUSE_WALL]);
        let step = step_onto_the_wire(&mut inner, &mut now);
        assert_eq!(
            refuse_step(&mut inner, step.sequence, REFUSED_FROM, now),
            Refusal::Waiting
        );
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "a person standing there is no wall"
        );
        assert!(
            !inner.movement.ready(now + MOBILE_WAIT - ONE_TICK),
            "he waits for the person to step off"
        );
    }

    /// A person who has not moved in this many tries is not going to, so the
    /// tile is treated as shut after all. The count starts again once it is,
    /// and the count over the whole trip is what ends a trip that is going
    /// nowhere.
    #[test]
    fn waiting_at_one_tile_long_enough_blocks_it_and_then_ends_the_trip() {
        let mut inner = test_session();
        let mut now = Instant::now();
        inner
            .world
            .write()
            .mobiles
            .insert(SOMEBODY_ELSE, standing_at(THE_HOUSE_WALL));
        let mut blocked_at = Vec::new();
        let mut ended = None;
        for wait in 1..=WAITS_GIVE_UP {
            inner.movement.hold();
            walks_from(&mut inner, REFUSED_FROM, vec![THE_HOUSE_WALL]);
            let step = step_onto_the_wire(&mut inner, &mut now);
            let what = refuse_step(&mut inner, step.sequence, REFUSED_FROM, now);
            // He waits for the person to step off, and asks again once that
            // wait and the pace of a person are both up.
            now += MOBILE_WAIT + STEP_PACE;
            match what {
                Refusal::Waiting => {}
                Refusal::WaitedOut => blocked_at.push(wait),
                Refusal::GaveUp => {
                    ended = Some(wait);
                    break;
                }
                other => panic!("wait {wait} answered as {other:?}"),
            }
        }
        assert_eq!(
            blocked_at,
            vec![WAITS_BLOCK_CELL],
            "he blocks the tile once he has waited at it long enough"
        );
        assert_eq!(
            inner.movement.blocked.tiles(),
            vec![THE_HOUSE_WALL],
            "and that is the tile he blocks"
        );
        assert_eq!(
            ended,
            Some(WAITS_GIVE_UP),
            "and the whole trip ends once he has waited this often over it"
        );
        assert!(
            inner
                .world
                .read()
                .events
                .iter()
                .any(|ev| ev.kind == uoterm_world::EventKind::PathFailed),
            "so the caller is told, and can choose somewhere else"
        );
    }

    /// How far a tile beside him stands: one step.
    const ONE_STEP_AWAY: u32 = 1;

    /// A house is a dynamic item the server sent, and the client files give
    /// every wall of it. So a refusal at one of those walls is already
    /// explained: he plans around the building he can see rather than marking
    /// its tiles one at a time, which is how one building drew 68 refusals.
    #[test]
    fn a_refusal_at_a_building_he_can_see_plans_around_it_and_marks_nothing() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let shapes = Arc::new(MultiData::open(&dir).expect("the client multi files"));
        let wall = stone_house_wall(&shapes);
        let into_the_wall = *wall
            .iter()
            .find(|tile| tile.chebyshev(REFUSED_FROM) == ONE_STEP_AWAY)
            .expect("a wall of the house stands beside her");
        let mut inner = test_session();
        inner.multi_shapes = Some(shapes);
        inner.world.write().self_state.location = REFUSED_FROM;
        ingest(
            &mut inner,
            &world_item(STONE_HOUSE_SERIAL, STONE_HOUSE_GRAPHIC, STONE_HOUSE_AT),
        );
        let mut now = Instant::now();
        walks_from(&mut inner, REFUSED_FROM, vec![into_the_wall]);
        let step = step_onto_the_wire(&mut inner, &mut now);
        assert_eq!(
            refuse_step(&mut inner, step.sequence, REFUSED_FROM, now),
            Refusal::Building,
            "the house is on the wire already, so the refusal needs no mark to explain it"
        );
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "and nothing is marked"
        );
    }

    /// A trip planned again this often is going nowhere, and it ends instead
    /// of looping. Without the cap a character with no way through plans, is
    /// refused, plans again and never arrives and never says why.
    #[test]
    fn a_trip_ends_once_its_route_has_been_planned_again_too_often() {
        let mut inner = test_session();
        inner.world.write().self_state.location = REFUSED_FROM;
        inner.goal = Goal::Travel {
            dest: PAST_THE_HOUSE,
        };
        queue_move(&mut inner, PAST_THE_HOUSE);
        for replan in 1..=movement::REPLANS_MAX {
            replan_the_route(&mut inner);
            assert!(
                inner.movement.goal.is_some(),
                "replan {replan} is inside the cap"
            );
        }
        replan_the_route(&mut inner);
        assert!(
            inner.movement.goal.is_none(),
            "and one more ends the trip instead of looping"
        );
        assert!(
            inner
                .world
                .read()
                .events
                .iter()
                .any(|ev| ev.kind == uoterm_world::EventKind::PathFailed),
            "so the caller is told the walk is going nowhere"
        );
    }

    /// A new destination is a new journey. The tiles the server refused on the
    /// way somewhere else say nothing about this way, and a minute is a long
    /// time to plan around a mark made for a walk he is no longer making.
    #[test]
    fn a_new_destination_forgets_the_tiles_refused_on_the_way_to_the_old_one() {
        let mut inner = test_session();
        let now = Instant::now();
        inner.world.write().self_state.location = REFUSED_FROM;
        queue_move(&mut inner, PAST_THE_HOUSE);
        inner.movement.blocked.refuse(THE_HOUSE_WALL, now);
        ready_to_plan_again(&mut inner);

        queue_move(&mut inner, PAST_THE_HOUSE);
        assert_eq!(
            inner.movement.blocked.tiles(),
            vec![THE_HOUSE_WALL],
            "the same destination is the same journey, so the mark is kept"
        );
        ready_to_plan_again(&mut inner);

        queue_move(&mut inner, IN_THE_WAY);
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "and a new destination drops it"
        );
    }

    /// A refusal says one thing about the world: that the tile the character
    /// was trying to enter would not take him. Blocking the tile he stands on
    /// instead would wall him in where he is.
    #[test]
    fn a_refusal_blocks_the_tile_he_tried_to_enter_and_not_the_one_he_stands_on() {
        let mut inner = test_session();
        let mut now = Instant::now();
        walks_from(&mut inner, REFUSED_FROM, vec![THE_HOUSE_WALL]);
        let first = step_onto_the_wire(&mut inner, &mut now);
        ingest(&mut inner, &refusal_at(first.sequence, REFUSED_FROM));
        assert_eq!(
            reported_at(&inner),
            REFUSED_FROM,
            "the refusal leaves her on the tile it carries"
        );
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "the first refusal is worth a door and one more try, and marks nothing"
        );
        assert!(
            inner.outbound.iter().any(|pkt| *pkt == encode::open_door()),
            "so she asks for the door that may be standing in it"
        );

        // The leaf has had its time to swing and nothing has, so the same step
        // goes out again and is refused again.
        now += DOOR_RETRY_WAIT + STEP_PACE;
        let second = step_onto_the_wire(&mut inner, &mut now);
        assert_eq!(
            second.arrives_at, THE_HOUSE_WALL,
            "the same step, aimed at the same tile"
        );
        ingest(&mut inner, &refusal_at(second.sequence, REFUSED_FROM));
        assert_eq!(
            inner.movement.blocked.tiles(),
            vec![THE_HOUSE_WALL],
            "and the second refusal blocks the tile east of her, which is the one she asked for"
        );
    }

    /// A refused tile blocks every route the character plans, and stops
    /// blocking them once its time is up.
    #[test]
    fn a_route_goes_around_a_refused_tile_until_the_memory_of_it_runs_out() {
        let mut inner = test_session();
        let now = Instant::now();
        inner.world.write().self_state.location = REFUSED_FROM;
        assert!(
            queue_move(&mut inner, PAST_THE_HOUSE),
            "the map alone calls the way open"
        );
        assert!(
            route_crosses(&inner, THE_HOUSE_WALL),
            "and sends her straight through the house: {:?}",
            inner.movement.path
        );

        inner.movement.blocked.refuse(THE_HOUSE_WALL, now);
        ready_to_plan_again(&mut inner);
        assert!(queue_move(&mut inner, PAST_THE_HOUSE));
        assert!(
            !route_crosses(&inner, THE_HOUSE_WALL),
            "the refused tile is a blocker now, so the route goes around it: {:?}",
            inner.movement.path
        );

        pump_movement(&mut inner, now + movement::REFUSED_TILE_MEMORY);
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "its time is up, because a boat sails and a door opens"
        );
        ready_to_plan_again(&mut inner);
        assert!(queue_move(&mut inner, PAST_THE_HOUSE));
        assert!(
            route_crosses(&inner, THE_HOUSE_WALL),
            "and the tile is one a route may use again: {:?}",
            inner.movement.path
        );
    }

    /// The server putting the character on a tile is proof that it takes her,
    /// whatever it refused her there before.
    #[test]
    fn standing_on_a_refused_tile_forgets_it() {
        let mut inner = test_session();
        let now = Instant::now();
        inner.movement.blocked.refuse(THE_HOUSE_WALL, now);
        walks_from(&mut inner, REFUSED_FROM, vec![THE_HOUSE_WALL]);
        pump_movement(&mut inner, now);
        assert_eq!(
            inner.movement.blocked.tiles(),
            vec![THE_HOUSE_WALL],
            "the step is a request, and answers nothing on its own"
        );
        accept_move_ack(&mut inner, movement::SEQ_FIRST);
        assert_eq!(reported_at(&inner), THE_HOUSE_WALL);
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "and the answer that put her there proves the tile takes her"
        );
    }

    /// The measured failure itself, driven as a walk. A character beside the
    /// Britain bank is told to walk past a player house. The house is on the
    /// server and in no map file, so every route is planned straight through
    /// its wall and the server refuses the step; with nothing remembered she
    /// tries the same step again, which is how one tile drew 35 refusals in
    /// two minutes. She must be refused at a tile once and then go around it.
    #[test]
    fn a_refused_step_is_never_tried_twice_and_the_walk_goes_around_the_house() {
        let mut inner = test_session();
        inner.world.write().self_state.location = REFUSED_FROM;
        inner.goal = Goal::Travel {
            dest: PAST_THE_HOUSE,
        };
        let mut now = Instant::now();
        let mut refused: Vec<Point3> = Vec::new();
        for _ in 0..HOUSE_WALK_TICKS {
            pump_movement(&mut inner, now);
            now += RUN_PACE;
            let Some(step) = inner.movement.in_flight.front().cloned() else {
                continue;
            };
            // The shard answers: the wall of the house refuses the step and
            // says where she really stands, and open ground confirms it.
            if in_the_house_wall(step.arrives_at) {
                refused.push(step.arrives_at);
                let refusal = refusal_at(step.sequence, reported_at(&inner));
                ingest(&mut inner, &refusal);
            } else {
                accept_move_ack(&mut inner, step.sequence);
            }
            if reported_at(&inner) == PAST_THE_HOUSE {
                break;
            }
        }
        assert_eq!(
            reported_at(&inner),
            PAST_THE_HOUSE,
            "she walks around the house and arrives; refused at {refused:?}"
        );
        assert_cell_asked_at_most_twice(&refused);
        assert!(
            refused.len() <= HOUSE_WALL_TILES * ASKS_PER_CELL,
            "and no more of the wall is met than it holds: {refused:?}"
        );
    }

    /// How often one cell may be asked for before it is proven shut: once to
    /// try the door that may be standing in it, and once to prove there is
    /// none. The second refusal is the one that marks the cell, and nothing
    /// asks for it again after that.
    const ASKS_PER_CELL: usize = 2;

    /// Checks that no cell was asked for more often than that.
    fn assert_cell_asked_at_most_twice(refused: &[Point3]) {
        for cell in refused {
            let asks = refused
                .iter()
                .filter(|other| other.x == cell.x && other.y == cell.y)
                .count();
            assert!(
                asks <= ASKS_PER_CELL,
                "{cell} was asked for {asks} times: {refused:?}"
            );
        }
    }

    /// The measured fault, driven as a walk. Beside the Britain bank one tile
    /// drew 68 refusals and 70 tiles were filed away, every refusal at the
    /// same building: she bumped a corner, remembered that one tile, planned
    /// again, walked into the tile beside it, and worked her way down the wall
    /// one refusal at a time.
    ///
    /// The cure is not to guess where the rest of the wall runs. It is to
    /// stop asking for a crossing that has already failed: a refusal marks the
    /// one cell it was really about, the crossing into it is remembered in the
    /// one direction it failed, and every route after it plans around both. So
    /// she meets some of a long wall, never the same cell more than
    /// [`ASKS_PER_CELL`] times, and gets past it.
    #[test]
    fn refusals_against_one_wall_are_never_repeated_and_she_still_gets_past() {
        let mut inner = test_session();
        inner.world.write().self_state.location = REFUSED_FROM;
        inner.goal = Goal::Travel {
            dest: PAST_THE_LONG_WALL,
        };
        let mut now = Instant::now();
        let mut refused: Vec<Point3> = Vec::new();
        for _ in 0..LONG_WALL_WALK_TICKS {
            pump_movement(&mut inner, now);
            now += RUN_PACE;
            let Some(step) = inner.movement.in_flight.front().cloned() else {
                continue;
            };
            if in_the_long_wall(step.arrives_at) && !step.turn {
                refused.push(step.arrives_at);
                let refusal = refusal_at(step.sequence, reported_at(&inner));
                ingest(&mut inner, &refusal);
            } else {
                accept_move_ack(&mut inner, step.sequence);
            }
            if reported_at(&inner) == PAST_THE_LONG_WALL {
                break;
            }
        }
        assert_eq!(
            reported_at(&inner),
            PAST_THE_LONG_WALL,
            "she gets past the wall; refused at {refused:?}"
        );
        assert_cell_asked_at_most_twice(&refused);
        assert!(
            refused.len() < LONG_WALL_TILES * ASKS_PER_CELL,
            "and she does not feel along every tile of it: {refused:?}"
        );
        for held in inner.movement.blocked.tiles() {
            assert!(
                in_the_long_wall(held),
                "{held} is open ground she was never refused at, so nothing may plan around it"
            );
        }
    }

    /// Ground truth measured on a live shard in the woodland, map index 0,
    /// with every packet traced. A character keeping up with another player
    /// was refused 28 times at one tile, and her memory recorded that tile 27
    /// separate times: the memory worked, and she walked into the tile again
    /// all the same, because the tile she was walking to was the tile the
    /// memory held, and the route finder threw its own memory away at the
    /// destination.
    ///
    /// The mock ground is flat, so the measured height of 30 is written here
    /// in words and not in the tile. What the refusals are about is x and y.
    const WOODLAND_FOLLOWER: Point3 = Point3 {
        x: 1420,
        y: 1528,
        z: 0,
    };
    /// The tile the server refused her at, northwest of her and beside the
    /// player she was keeping up with. The client map files call it open
    /// ground.
    const WOODLAND_REFUSED: Point3 = Point3 {
        x: WOODLAND_FOLLOWER.x - 1,
        y: WOODLAND_FOLLOWER.y - 1,
        z: WOODLAND_FOLLOWER.z,
    };
    /// The player she was keeping up with, one tile past that on the same
    /// line, so the tile the server refuses is the tile beside him and the
    /// nearest one to her.
    const WOODLAND_PLAYER: Point3 = Point3 {
        x: WOODLAND_REFUSED.x - 1,
        y: WOODLAND_REFUSED.y - 1,
        z: WOODLAND_FOLLOWER.z,
    };
    /// Where that player walks on to, far enough off that a follower closes
    /// the gap again and the tile the server refuses lies straight across the
    /// way to him.
    const WOODLAND_PLAYER_WALKED_ON: Point3 = Point3 {
        x: WOODLAND_PLAYER.x - 1,
        y: WOODLAND_PLAYER.y - 1,
        z: WOODLAND_FOLLOWER.z,
    };
    /// How many ticks she is given to ask for that tile. Far short of the 28
    /// refusals the live shard answered in one run.
    const WOODLAND_TICKS: usize = 20;

    /// True when the queued step is aimed at that tile, whatever height it
    /// carries.
    fn aimed_at(step: &movement::PendingStep, at: Point3) -> bool {
        step.arrives_at.x == at.x && step.arrives_at.y == at.y
    }

    /// The measured failure itself, driven as a sequence. She is sent to the
    /// tile beside the player she is keeping up with, the server refuses her
    /// there, and she must never ask for that tile again while she remembers
    /// it. Before the two kinds of obstacle were told apart she asked for it
    /// on every tick: the memory held the tile, and the route finder lifted
    /// its own memory off the destination.
    #[test]
    fn a_refused_tile_beside_the_player_is_never_asked_for_twice() {
        let mut inner = test_session();
        inner.world.write().self_state.location = WOODLAND_FOLLOWER;
        inner
            .world
            .write()
            .mobiles
            .insert(SOMEBODY_ELSE, standing_at(WOODLAND_PLAYER));
        inner.goal = Goal::Travel {
            dest: WOODLAND_REFUSED,
        };
        let mut now = Instant::now();
        let mut asked: Vec<Point3> = Vec::new();
        for _ in 0..WOODLAND_TICKS {
            pump_movement(&mut inner, now);
            now += RUN_PACE;
            let Some(step) = inner.movement.in_flight.front().cloned() else {
                continue;
            };
            // The shard answers: that one tile refuses her and says where she
            // really stands, and open ground confirms the step.
            if aimed_at(&step, WOODLAND_REFUSED) {
                asked.push(step.arrives_at);
                let refusal = refusal_at(step.sequence, reported_at(&inner));
                ingest(&mut inner, &refusal);
            } else {
                accept_move_ack(&mut inner, step.sequence);
            }
        }
        assert_cell_asked_at_most_twice(&asked);
        assert_eq!(
            asked.len(),
            ASKS_PER_CELL,
            "she asks for that tile once to try a door and once to prove there is none, \
             and then never again: {asked:?}"
        );
        assert_eq!(
            inner.movement.blocked.tiles(),
            vec![WOODLAND_REFUSED],
            "the tile is remembered, and the memory is what refuses the walk to it"
        );
        assert!(
            inner
                .world
                .read()
                .events
                .iter()
                .any(|ev| ev.kind == uoterm_world::EventKind::PathFailed),
            "and the caller is told the way is shut, so it can choose somewhere else"
        );

        // The player walks on, and she keeps up with him. The tile she
        // remembers lies straight across the way to him, and the tile beside
        // him she now walks to is another one.
        inner
            .world
            .write()
            .mobiles
            .insert(SOMEBODY_ELSE, standing_at(WOODLAND_PLAYER_WALKED_ON));
        follow_tick(&mut inner, WOODLAND_PLAYER_WALKED_ON, WALKING);
        let route: Vec<Point3> = inner.movement.path.iter().copied().collect();
        assert!(!route.is_empty(), "she has a way to keep up with him");
        assert_eq!(
            inner.movement.goal.map(|g| (g.x, g.y)),
            route.last().map(|step| (step.x, step.y)),
            "the walk ends on the tile it was planned to: {route:?}"
        );
        assert_eq!(
            inner
                .movement
                .goal
                .map(|goal| goal.chebyshev(WOODLAND_PLAYER_WALKED_ON)),
            Some(movement::FOLLOW_NEAR_DISTANCE),
            "beside the player she is keeping up with: {route:?}"
        );
        assert!(
            !route_crosses(&inner, WOODLAND_REFUSED),
            "and the tile the server refused is not on it: {route:?}"
        );
    }

    /// The small stone house: multi id 0x0064 in the client files, 145 pieces
    /// of which 56 are solid, standing on 7 by 7 tiles.
    const STONE_HOUSE_ID: u16 = 0x0064;
    /// How the server puts that house on the wire: the multi bit on the
    /// graphic of an ordinary item.
    const STONE_HOUSE_GRAPHIC: u16 = STONE_HOUSE_ID | ITEM_GRAPHIC_MULTI;
    const STONE_HOUSE_SERIAL: Serial = Serial(0x4000_0064);
    /// How far the wall of that house stands from the multi item itself.
    const STONE_HOUSE_HALF: u16 = 3;
    /// Where the house stands: its west wall is [`THE_HOUSE_WALL`], the tile
    /// that refused her 35 times in two minutes.
    const STONE_HOUSE_AT: Point3 = Point3 {
        x: THE_HOUSE_WALL.x + STONE_HOUSE_HALF,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };
    /// One tile past the east wall, so the walk she was making crosses the
    /// whole house.
    const PAST_THE_STONE_HOUSE: Point3 = Point3 {
        x: STONE_HOUSE_AT.x + STONE_HOUSE_HALF + 1,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };
    /// The course of wall a person meets, measured from the client files: the
    /// wall stands on the floor of the house, which is this far above the
    /// house itself. It runs round all four sides but the doorway.
    const STONE_HOUSE_WALL_DZ: i16 = 7;
    /// How many ticks the walk round that house is given. The way round a
    /// 7 by 7 house is under twenty steps.
    const STONE_HOUSE_WALK_TICKS: usize = 120;

    /// A legacy world item packet, which is how every shard sends an item to a
    /// client that did not log in as a Stygian Abyss one.
    fn world_item(serial: Serial, graphic: u16, at: Point3) -> Vec<u8> {
        let mut w = uoterm_protocol::buf::PacketWriter::with_variable(PKT_WORLD_ITEM);
        w.serial(serial).u16(graphic).u16(at.x).u16(at.y).i8(at.z);
        w.finish_variable().expect("a world item packet")
    }

    fn delete_item(serial: Serial) -> Vec<u8> {
        let mut w = uoterm_protocol::buf::PacketWriter::new(PKT_DELETE);
        w.serial(serial);
        w.finish()
    }

    /// The bit a shard sets on the serial of the legacy world item packet to
    /// say that an amount follows the graphic. Every multi carries an amount
    /// of one, so every house on that packet has it set. Both server families
    /// write the bit the same way.
    const WORLD_ITEM_HAS_AMOUNT: u32 = 0x8000_0000;
    /// The bit set on `x` to say that a byte follows `y`: the direction of the
    /// item on one server family, its light level on the other. A boat carries
    /// one.
    const WORLD_ITEM_HAS_BYTE_AFTER_Y: u16 = 0x8000;
    /// The bits set on `y` to say that a hue and then a flags byte follow the
    /// height.
    const WORLD_ITEM_HAS_HUE: u16 = 0x8000;
    const WORLD_ITEM_HAS_FLAGS: u16 = 0x4000;
    /// What is left of `y` once those two bits are taken off it.
    const WORLD_ITEM_Y_MASK: u16 = 0x3FFF;
    /// The amount every multi carries.
    const ONE_BUILDING: u16 = 1;
    /// A boat under way: it faces a direction, and it is painted.
    const BOAT_FACING: u8 = 2;
    const BOAT_HUE: u16 = 0x0026;
    const BOAT_FLAGS: u8 = 0x20;
    /// A boat that is no house: multi id 0x0000 is the smallest hull in the
    /// client files, so its graphic on the legacy packet is the multi bit and
    /// nothing else.
    const BOAT_ID: u16 = 0x0000;
    const BOAT_SERIAL: Serial = Serial(0x4000_00B0);
    /// The command word every Stygian Abyss world item packet opens with.
    const WORLD_ITEM_SA_COMMAND: u16 = 0x0001;
    /// The graphic increment byte, which no multi ever carries.
    const NO_GRAPHIC_INCREMENT: u8 = 0;
    /// The light level of a building, and the flags of one.
    const NO_LIGHT: u8 = 0;
    const NO_FLAGS: u8 = 0;
    /// The two bytes a High Seas client gets on the end of that packet.
    const WORLD_ITEM_SA_HIGH_SEAS_TAIL: u16 = 0;

    /// The legacy world item packet, `0x1A`, byte for byte as both emulators
    /// write it for a building: the multi bit on the graphic, the amount bit
    /// on the serial with the amount behind it, and the optional direction,
    /// hue and flags a boat under way carries.
    fn world_item_as_a_shard_writes_it(
        serial: Serial,
        graphic: u16,
        at: Point3,
        facing: u8,
        hue: u16,
        flags: u8,
    ) -> Vec<u8> {
        let mut w = uoterm_protocol::buf::PacketWriter::with_variable(PKT_WORLD_ITEM);
        let x = if facing == 0 {
            at.x
        } else {
            at.x | WORLD_ITEM_HAS_BYTE_AFTER_Y
        };
        let y = (at.y & WORLD_ITEM_Y_MASK)
            | if hue == 0 { 0 } else { WORLD_ITEM_HAS_HUE }
            | if flags == 0 { 0 } else { WORLD_ITEM_HAS_FLAGS };
        w.u32(serial.0 | WORLD_ITEM_HAS_AMOUNT)
            .u16(graphic)
            .u16(ONE_BUILDING)
            .u16(x)
            .u16(y);
        if facing != 0 {
            w.u8(facing);
        }
        w.i8(at.z);
        if hue != 0 {
            w.u16(hue);
        }
        if flags != 0 {
            w.u8(flags);
        }
        w.finish_variable().expect("a world item packet")
    }

    /// The Stygian Abyss world item packet, `0xF3`, byte for byte as a server
    /// writes it: the type byte fourth, the raw multi id in the graphic, and
    /// the amount written twice.
    fn world_item_sa_as_a_shard_writes_it(
        kind: u8,
        serial: Serial,
        graphic: u16,
        at: Point3,
    ) -> Vec<u8> {
        let mut w = uoterm_protocol::buf::PacketWriter::new(PKT_WORLD_ITEM_SA);
        w.u16(WORLD_ITEM_SA_COMMAND)
            .u8(kind)
            .serial(serial)
            .u16(graphic)
            .u8(NO_GRAPHIC_INCREMENT)
            .u16(ONE_BUILDING)
            .u16(ONE_BUILDING)
            .u16(at.x)
            .u16(at.y)
            .i8(at.z)
            .u8(NO_LIGHT)
            .u16(NO_HUE)
            .u8(NO_FLAGS)
            .u16(WORLD_ITEM_SA_HIGH_SEAS_TAIL);
        w.finish()
    }

    /// A building must be recognised on either packet, from the bytes that
    /// packet really carries.
    ///
    /// Which of the two a shard sends is not the client's choice: it depends
    /// on the client version the session logged in with, and on a modern
    /// session it is the Stygian Abyss one. Nothing had ever driven that form
    /// end to end, and a rule that is right in one place and never read in
    /// the other is a building nobody sees.
    #[test]
    fn a_building_is_recognised_on_the_packet_of_either_era() {
        let multis = |inner: &Inner| inner.world.read().multis_seen();

        let mut legacy = test_session();
        ingest(
            &mut legacy,
            &world_item_as_a_shard_writes_it(
                STONE_HOUSE_SERIAL,
                STONE_HOUSE_GRAPHIC,
                STONE_HOUSE_AT,
                0,
                NO_HUE,
                NO_FLAGS,
            ),
        );
        assert_eq!(
            multis(&legacy),
            vec![uoterm_world::MultiItem {
                serial: STONE_HOUSE_SERIAL,
                multi_id: STONE_HOUSE_ID,
                location: STONE_HOUSE_AT,
            }],
            "the multi bit on the legacy packet names the house, amount bit and all"
        );

        // A boat under way, which carries every optional field that packet
        // has. Read one byte wrong and the building lands on the wrong tile.
        ingest(
            &mut legacy,
            &world_item_as_a_shard_writes_it(
                BOAT_SERIAL,
                BOAT_ID | ITEM_GRAPHIC_MULTI,
                PAST_THE_HOUSE,
                BOAT_FACING,
                BOAT_HUE,
                BOAT_FLAGS,
            ),
        );
        assert!(
            multis(&legacy).contains(&uoterm_world::MultiItem {
                serial: BOAT_SERIAL,
                multi_id: BOAT_ID,
                location: PAST_THE_HOUSE,
            }),
            "and a boat with a heading, a hue and flags on the same packet: {:?}",
            multis(&legacy)
        );

        let mut modern = test_session();
        ingest(
            &mut modern,
            &world_item_sa_as_a_shard_writes_it(
                WORLD_ITEM_SA_TYPE_MULTI,
                STONE_HOUSE_SERIAL,
                STONE_HOUSE_ID,
                STONE_HOUSE_AT,
            ),
        );
        assert_eq!(
            multis(&modern),
            vec![uoterm_world::MultiItem {
                serial: STONE_HOUSE_SERIAL,
                multi_id: STONE_HOUSE_ID,
                location: STONE_HOUSE_AT,
            }],
            "and the type byte on the Stygian Abyss packet names the same house"
        );

        // The same packet carrying an ordinary item, with the multi bit on its
        // graphic for good measure. The type byte is the only rule there.
        ingest(
            &mut modern,
            &world_item_sa_as_a_shard_writes_it(
                uoterm_protocol::types::WORLD_ITEM_SA_ITEM,
                Serial(0x4000_0001),
                STONE_HOUSE_GRAPHIC,
                PAST_THE_HOUSE,
            ),
        );
        assert_eq!(
            multis(&modern).len(),
            1,
            "an ordinary item on that packet is no building: {:?}",
            multis(&modern)
        );
    }

    /// The wall of the house as the shard knows it, read out of the client
    /// multi files and not out of the code under test: every tile that carries
    /// a solid piece of the course of wall the house stands on.
    fn stone_house_wall(shapes: &MultiData) -> Vec<Point3> {
        shapes
            .pieces(STONE_HOUSE_ID)
            .iter()
            .filter(|piece| piece.blocks() && piece.dz == STONE_HOUSE_WALL_DZ)
            .map(|piece| {
                Point3::new(
                    (i32::from(STONE_HOUSE_AT.x) + i32::from(piece.dx)) as u16,
                    (i32::from(STONE_HOUSE_AT.y) + i32::from(piece.dy)) as u16,
                    STONE_HOUSE_AT.z,
                )
            })
            .collect()
    }

    /// The item packet is the only word the client gets that a building is
    /// there, so the record must follow it: it appears with the packet and it
    /// goes when the server deletes the item.
    #[test]
    fn a_building_is_recorded_from_its_item_packet_and_dropped_with_it() {
        let mut inner = test_session();
        ingest(
            &mut inner,
            &world_item(STONE_HOUSE_SERIAL, STONE_HOUSE_GRAPHIC, STONE_HOUSE_AT),
        );
        assert_eq!(
            inner.world.read().multis_seen(),
            vec![uoterm_world::MultiItem {
                serial: STONE_HOUSE_SERIAL,
                multi_id: STONE_HOUSE_ID,
                location: STONE_HOUSE_AT,
            }],
            "the multi bit on the graphic names the house and the shape under it"
        );

        let logs = Serial(0x4000_0001);
        ingest(&mut inner, &world_item(logs, GRAPHIC_LOGS, REFUSED_FROM));
        assert_eq!(
            inner.world.read().multis_seen().len(),
            1,
            "an ordinary item is no building"
        );

        ingest(&mut inner, &delete_item(STONE_HOUSE_SERIAL));
        assert!(
            inner.world.read().multis_seen().is_empty(),
            "and the record goes when the server deletes the item"
        );
    }

    /// The measured failure itself, and what reading the client files does
    /// about it. A character beside the Britain bank in Felucca is told to
    /// walk past a player house. The map files call every tile of that house
    /// open ground, so the route runs straight through its wall and the server
    /// refuses the step; one tile drew 35 refusals in two minutes that way.
    ///
    /// The house is an item the server already sends her, and its shape is in
    /// the client files, so she now goes round it from the first step and is
    /// refused nowhere at all. Before the buildings were read this test failed
    /// on the last assertion: she was refused at the wall before she went
    /// round it.
    #[test]
    fn a_house_in_view_is_planned_around_from_the_first_step() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let shapes = Arc::new(MultiData::open(&dir).expect("the client multi files"));
        let wall = stone_house_wall(&shapes);
        assert!(
            wall.iter()
                .any(|tile| tile.x == THE_HOUSE_WALL.x && tile.y == THE_HOUSE_WALL.y),
            "the house must stand on the tile that refused her, or the test proves nothing"
        );

        let mut blind = test_session();
        blind.world.write().self_state.location = REFUSED_FROM;
        assert!(
            queue_move(&mut blind, PAST_THE_STONE_HOUSE),
            "the map alone calls the way open"
        );
        assert!(
            wall.iter().any(|tile| route_crosses(&blind, *tile)),
            "and sends her straight through the wall of the house: {:?}",
            blind.movement.path
        );

        let mut inner = test_session();
        inner.multi_shapes = Some(shapes);
        inner.world.write().self_state.location = REFUSED_FROM;
        ingest(
            &mut inner,
            &world_item(STONE_HOUSE_SERIAL, STONE_HOUSE_GRAPHIC, STONE_HOUSE_AT),
        );
        inner.goal = Goal::Travel {
            dest: PAST_THE_STONE_HOUSE,
        };
        let mut now = Instant::now();
        let mut refused: Vec<Point3> = Vec::new();
        for _ in 0..STONE_HOUSE_WALK_TICKS {
            pump_movement(&mut inner, now);
            now += RUN_PACE;
            let Some(step) = inner.movement.in_flight.front().cloned() else {
                continue;
            };
            // The shard answers: the wall of the house refuses the step and
            // says where she really stands, and open ground confirms it.
            let into_the_wall = wall
                .iter()
                .any(|tile| tile.x == step.arrives_at.x && tile.y == step.arrives_at.y);
            if into_the_wall {
                refused.push(step.arrives_at);
                let refusal = refusal_at(step.sequence, reported_at(&inner));
                ingest(&mut inner, &refusal);
            } else {
                accept_move_ack(&mut inner, step.sequence);
            }
            if reported_at(&inner) == PAST_THE_STONE_HOUSE {
                break;
            }
        }
        assert_eq!(
            reported_at(&inner),
            PAST_THE_STONE_HOUSE,
            "she walks round the house and arrives; refused at {refused:?}"
        );
        assert!(
            refused.is_empty(),
            "and asks for no tile of the house at all: {refused:?}"
        );
    }

    /// Where a player stands between the character and where he is going.
    const IN_THE_WAY: Point3 = Point3 {
        x: REFUSED_FROM.x + 1,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };
    /// Two tiles past him, straight on.
    const PAST_HIM: Point3 = Point3 {
        x: REFUSED_FROM.x + 3,
        y: REFUSED_FROM.y,
        z: REFUSED_FROM.z,
    };

    /// Read from the server source: the free movement rule is set on every
    /// facet but Felucca, and a step onto an occupied tile is gated only by the
    /// shove check, which does nothing at all when that rule is set.
    /// So on those facets the server lets the character walk straight through
    /// anybody, and a route that goes around people is longer than it needs to
    /// be for no reason.
    #[test]
    fn a_player_blocks_a_route_on_felucca_and_not_on_a_free_movement_facet() {
        let mut inner = test_session();
        {
            let mut world = inner.world.write();
            world.self_state.location = REFUSED_FROM;
            world.self_state.map = FELUCCA_MAP_INDEX;
            world.mobiles.insert(SOMEBODY_ELSE, standing_at(IN_THE_WAY));
        }
        assert!(queue_move(&mut inner, PAST_HIM));
        assert!(
            !route_crosses(&inner, IN_THE_WAY),
            "on Felucca a shove costs stamina, so he goes around: {:?}",
            inner.movement.path
        );

        inner.world.write().self_state.map = TRAMMEL_MAP_INDEX;
        ready_to_plan_again(&mut inner);
        assert!(queue_move(&mut inner, PAST_HIM));
        assert!(
            route_crosses(&inner, IN_THE_WAY),
            "and on Trammel the same player is no obstacle at all: {:?}",
            inner.movement.path
        );
    }

    /// The door must still be opened, and never merely walked around. A shut
    /// door is the commonest thing on a shard that refuses a step, and the one
    /// the character can do something about, so the door is asked first: a
    /// tile a door stands on is the door logic's, and is never remembered as a
    /// wall to go around.
    /// The open-door macro names no door. The server offsets one tile in the
    /// direction the character faces and opens whatever door it finds there,
    /// so a character facing the wrong way opens nothing and is told nothing.
    ///
    /// Measured on a live shard: she stood one tile south of a bank door
    /// facing east, the tool sent the macro, and the door never moved.
    #[test]
    fn the_open_door_tool_turns_to_face_the_door_first() {
        const STANDING_AT: Point3 = Point3 {
            x: 1438,
            y: 1693,
            z: 0,
        };
        const DOOR_TO_THE_NORTH: Point3 = Point3 {
            x: 1438,
            y: 1692,
            z: 0,
        };
        const DOOR_SERIAL: Serial = Serial(0x4002_021B);
        const DOOR_GRAPHIC: u16 = 1653;

        let mut inner = test_session();
        {
            let mut world = inner.world.write();
            world.self_state.location = STANDING_AT;
            world.self_state.direction = Direction::East as u8;
            world.note_door(DOOR_SERIAL, DOOR_GRAPHIC, DOOR_TO_THE_NORTH);
        }
        inner.outbound.clear();

        face_the_nearest_door(&mut inner, Instant::now());
        inner.outbound.push_back(encode::open_door());

        let turned = inner.outbound.iter().position(|pkt| {
            pkt.first() == Some(&PKT_MOVE) && pkt.get(1) == Some(&(Direction::North as u8))
        });
        let asked = inner
            .outbound
            .iter()
            .position(|pkt| *pkt == encode::open_door());
        assert!(
            turned.is_some(),
            "she must turn toward the door before asking for it"
        );
        assert!(
            turned < asked,
            "and the turn must go out first, or the macro aims at the old facing"
        );
    }

    #[test]
    fn a_refusal_at_a_shut_door_opens_the_door_and_blocks_no_tile() {
        let inn = movement::tests::inn_corridor();
        let mut inner = test_session();
        inner.map = inn.map;
        inner
            .world
            .write()
            .note_door(inn.door.serial, inn.door.graphic, inn.door.location);
        walks_from(
            &mut inner,
            inn.in_front_of_the_door,
            vec![inn.door.location],
        );
        let now = Instant::now();
        pump_movement(&mut inner, now);
        assert_eq!(
            inner.movement.refused_direction(),
            Some(Direction::South),
            "the step she is about to be refused is the one into the doorway"
        );
        ingest(
            &mut inner,
            &refusal_at(movement::SEQ_FIRST, inn.in_front_of_the_door),
        );
        assert!(
            inner.movement.blocked.tiles().is_empty(),
            "the doorway is a door to open, not a wall to remember"
        );
        assert!(
            queue_move(&mut inner, inn.outside),
            "so the way out is still planned"
        );
        pump_doors(&mut inner);
        assert!(
            inner.outbound.iter().any(|pkt| *pkt == encode::open_door()),
            "and she opens the door instead of walking around it"
        );
        assert!(
            !inner
                .outbound
                .iter()
                .any(|pkt| *pkt == encode::double_click(inn.door.serial)),
            "with the macro that names no door, which cannot pick the wrong one of a pair"
        );
    }

    const TEST_HOST: &str = "10.10.44.2";
    const TEST_PORT: u16 = 2593;
    const UNSPECIFIED: [u8; 4] = [0, 0, 0, 0];
    const LAN_IP: [u8; 4] = [192, 168, 150, 103];
    const SAME_IP: [u8; 4] = [10, 10, 44, 2];

    fn opts(stay: bool) -> ConnectOptions {
        ConnectOptions {
            host: TEST_HOST.into(),
            port: TEST_PORT,
            account: "a".into(),
            password: "p".into(),
            shard: None,
            character: "c".into(),
            version: ClientVersion::MODERN,
            era: Era::Modern,
            uopath: None,
            persona: None,
            stay_on_socket: stay,
            next_login_key: LOGIN_NEXT_KEY_DEFAULT,
            encryption: EncryptionMode::None,
        }
    }

    #[test]
    fn encryption_default_is_none() {
        assert_eq!(opts(false).encryption, EncryptionMode::None);
    }

    #[test]
    fn osi_mode_is_stored() {
        let mut o = opts(false);
        o.encryption = EncryptionMode::Osi;
        assert_eq!(o.encryption, EncryptionMode::Osi);
    }

    /// Every packet the client sends must be named in the log, and named by
    /// what it really is. The id is the first byte of the built packet, which
    /// only holds while the packet is still plaintext.
    #[test]
    fn an_outbound_packet_is_named_by_its_id_and_its_length() {
        const PING_SEQUENCE: u8 = 1;
        const PING_LEN: usize = 2;
        const RESYNC_LEN: usize = 3;

        let ping = encode::ping(PING_SEQUENCE);
        assert_eq!(outbound_shape(&ping), Some((PKT_PING, PING_LEN)));

        let resync = encode::resync();
        assert_eq!(outbound_shape(&resync), Some((PKT_MOVE_ACK, RESYNC_LEN)));

        let double_click = encode::double_click(Serial(0x4000_0001));
        assert_eq!(
            outbound_shape(&double_click).map(|(id, _)| id),
            Some(PKT_DOUBLE_CLICK)
        );

        assert_eq!(outbound_shape(&[]), None, "an empty packet has no id");
    }

    /// The id has to be read before the cipher runs. After it, the first byte
    /// is ciphertext and names the wrong packet.
    #[test]
    fn sealing_hides_the_packet_id_from_the_log() {
        let seed = 0x1234_5678;
        let mut cipher = for_mode(EncryptionMode::Osi, seed, ClientVersion::MODERN);
        let pkt = encode::ping(1);
        let plain = outbound_shape(&pkt).expect("a built packet has an id");
        let sealed = seal_bytes(cipher.as_mut(), pkt);
        assert_eq!(plain.0, PKT_PING);
        assert_ne!(
            outbound_shape(&sealed),
            Some(plain),
            "reading the id after sealing would log the wrong packet"
        );
    }

    #[test]
    fn flush_full_channel_does_not_advance_osi_cipher() {
        const CHANNEL_CAP: usize = 2;
        let (tx, _rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAP);
        for _ in 0..CHANNEL_CAP {
            tx.try_send(vec![0]).unwrap();
        }
        let seed = 0xAABB_CCDD;
        let mut cipher = for_mode(EncryptionMode::Osi, seed, ClientVersion::MODERN);
        let mut control = for_mode(EncryptionMode::Osi, seed, ClientVersion::MODERN);
        let pkt = vec![PKT_PING, 0x01];
        let mut outbound = VecDeque::from([pkt.clone()]);
        flush_sealed(&mut outbound, cipher.as_mut(), &tx);
        assert_eq!(outbound.len(), 1);
        assert_eq!(outbound.front().unwrap(), &pkt);
        let probe = vec![0x11, 0x22, 0x33];
        let mut advanced = probe.clone();
        let mut expected = probe;
        cipher.encrypt(&mut advanced);
        control.encrypt(&mut expected);
        assert_eq!(advanced, expected);
    }

    #[test]
    fn social_goal_emits_at_most_one_say_per_interval() {
        let mut speech = SpeechPolicy::default();
        let persona = Persona::lumberjack_yew();
        let world = World::new();
        let mut sent = 0u32;
        for _ in 0..20 {
            match reflex::tick(&world, &persona, &Goal::Social) {
                ReflexAction::Say(text) => {
                    if speech_allowed(&mut speech, &persona, text, SPEECH_REGULAR).is_ok() {
                        sent += 1;
                    }
                }
                other => panic!("expected say, got {other:?}"),
            }
        }
        assert_eq!(sent, 1);
    }

    #[test]
    fn none_seal_keeps_plaintext() {
        let mut cipher = for_mode(EncryptionMode::None, 0xAABB_CCDD, ClientVersion::MODERN);
        let pkt = vec![PKT_PING, 0x01];
        let wire = seal_bytes(cipher.as_mut(), pkt.clone());
        assert_eq!(wire, pkt);
    }

    #[test]
    fn osi_seal_encrypts_then_decrypt_recovers() {
        let seed = 0x1122_3344;
        let mut enc = for_mode(EncryptionMode::Osi, seed, ClientVersion::MODERN);
        let mut dec = for_mode(EncryptionMode::Osi, seed, ClientVersion::MODERN);
        let pkt = vec![PKT_LOGIN_REQUEST, 0x01, 0x02, 0x03];
        let wire = seal_bytes(enc.as_mut(), pkt.clone());
        assert_ne!(wire, pkt);
        let mut back = wire;
        dec.decrypt(&mut back);
        assert_eq!(back, pkt);
    }

    #[test]
    fn osi_cipher_resets_for_game_seed() {
        let seed = 0x1111_2222;
        let mut login = for_mode(EncryptionMode::Osi, seed, ClientVersion::T2A);
        let mut game = for_mode(EncryptionMode::Osi, seed, ClientVersion::T2A);
        game.reset_for_game(seed);
        let mut a = vec![PKT_GAME_LOGIN, 0x00, 0x11, 0x22];
        let mut b = a.clone();
        login.encrypt(&mut a);
        game.encrypt(&mut b);
        assert_ne!(a, b);
    }

    #[test]
    fn unspecified_reconnects_when_stay_off() {
        let o = opts(false);
        assert!(!stay_on_relay(&o, UNSPECIFIED, TEST_PORT));
        assert_eq!(
            relay_addr(&o, UNSPECIFIED, TEST_PORT),
            format!("{TEST_HOST}:{TEST_PORT}")
        );
    }

    #[test]
    fn unspecified_stays_when_stay_on() {
        let o = opts(true);
        assert!(stay_on_relay(&o, UNSPECIFIED, TEST_PORT));
    }

    #[test]
    fn other_lan_ip_reconnects_when_stay_off() {
        let o = opts(false);
        assert!(!stay_on_relay(&o, LAN_IP, TEST_PORT));
        assert_eq!(relay_addr(&o, LAN_IP, TEST_PORT), "192.168.150.103:2593");
    }

    #[test]
    fn same_host_stays_when_stay_on() {
        let o = opts(true);
        assert!(stay_on_relay(&o, SAME_IP, TEST_PORT));
    }

    #[test]
    fn modern_stay_skips_raw_game_seed() {
        assert!(!game_login_needs_raw_seed(true, Era::Modern));
        assert!(game_login_needs_raw_seed(false, Era::Modern));
        assert!(game_login_needs_raw_seed(true, Era::T2a));
    }

    #[test]
    fn greedy_line_steps_toward_dest() {
        let from = Point3::new(3507, 2513, 27);
        let dest = Point3::new(3510, 2513, 27);
        let p = greedy_line(from, dest);
        assert_eq!(p.last().copied(), Some(dest));
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn player_path_goes_around_block() {
        let mut map = MockMap::new(8, 8);
        map.set_block(1, 0, true);
        let from = Point3::new(0, 0, 0);
        let dest = Point3::new(2, 0, 0);
        let p = player_path(&map, from, dest);
        assert!(p.iter().any(|s| s.x == 1 && s.y == 1), "{p:?}");
        assert_eq!(p.last().map(|s| (s.x, s.y)), Some((2, 0)));
    }

    /// A walk held in one direction is planned by no route, so every tile it
    /// queues carries the height the map answers for that one step. Held down
    /// the hillside those heights fall at every step; the height of the tile
    /// he leaves would queue the height of the hilltop all the way down.
    #[test]
    fn a_held_walk_takes_the_height_of_every_tile_from_the_map() {
        const HELD_STEPS: usize = 3;
        let map = hillside();
        let top = hill_tile(HILL_FOOT_Y + HILL_TILES);
        let north = hold_path(&map, top, Direction::North, HELD_STEPS);
        assert_eq!(
            north,
            (1..=HELD_STEPS)
                .map(|i| hill_tile(HILL_FOOT_Y + HILL_TILES - i as u16))
                .collect::<Vec<_>>(),
            "every tile of the walk down carries the height of its own ground"
        );
        let foot = hill_tile(HILL_FOOT_Y);
        let south = hold_path(&map, foot, Direction::South, HELD_STEPS);
        assert_eq!(
            south,
            (1..=HELD_STEPS)
                .map(|i| hill_tile(HILL_FOOT_Y + i as u16))
                .collect::<Vec<_>>(),
            "and every tile of the walk back up does the same"
        );
    }

    /// The map is asked for the height of each held step, and a step it has no
    /// height for is a step it has just refused. The walk ends there rather
    /// than queueing a tile with a height nobody worked out.
    #[test]
    fn a_held_walk_ends_where_the_map_has_no_height_for_the_next_step() {
        const HELD_STEPS: usize = 3;
        const WALLED_OFF: u16 = HILL_FOOT_Y + 2;
        let mut map = hillside();
        map.set_block(HILL_X, WALLED_OFF, true);
        let walked = hold_path(&map, hill_tile(HILL_FOOT_Y), Direction::South, HELD_STEPS);
        assert_eq!(
            walked,
            vec![hill_tile(HILL_FOOT_Y + 1)],
            "the walk stops in front of the tile the map holds nobody up on"
        );
    }

    #[test]
    fn hold_step_count_one_without_hold_ms() {
        assert_eq!(hold_step_count(HOLD_MS_NONE, true), WALK_STEPS_ONE);
        assert_eq!(hold_step_count(STEP_RUN_MS * 10, true), 10);
        assert_eq!(hold_step_count(1, false), WALK_STEPS_ONE);
    }

    #[test]
    fn popup_char_in_world_aborts_login() {
        let err = login_abort(&Inbound::PopupMessage {
            reason: POPUP_CHAR_IN_WORLD,
        })
        .expect("popup must abort login");
        assert!(err.to_string().contains(LOGIN_CHAR_IN_WORLD), "{err}");
    }

    #[test]
    fn idle_popup_does_not_abort_login() {
        assert!(login_abort(&Inbound::PopupMessage {
            reason: POPUP_IDLE_WARNING,
        })
        .is_none());
    }

    #[test]
    fn login_confirm_enters_world() {
        assert!(inbound_enters_world(&Inbound::LoginConfirm {
            serial: Serial(0xAA),
            body: 0x190,
            x: 1,
            y: 1,
            z: 0,
            direction: 0,
            map_width: 7168,
            map_height: 4096,
        }));
        assert!(inbound_enters_world(&Inbound::LoginComplete));
        assert!(!inbound_enters_world(&Inbound::VersionRequest));
    }

    #[test]
    fn visit_new_events_skips_seen_seq_without_clone() {
        let events = [
            uoterm_world::Event {
                seq: 1,
                kind: uoterm_world::EventKind::LoggedIn,
                unix_ms: 0,
                serial: None,
                text: String::new(),
            },
            uoterm_world::Event {
                seq: 2,
                kind: uoterm_world::EventKind::Arrived,
                unix_ms: 0,
                serial: None,
                text: "1,2,3".into(),
            },
        ];
        let mut last = 0;
        let mut seen = Vec::new();
        visit_new_events(&events, &mut last, |ev| seen.push(ev.seq));
        assert_eq!(seen, vec![1, 2]);
        assert_eq!(last, 2);
        seen.clear();
        visit_new_events(&events, &mut last, |ev| seen.push(ev.seq));
        assert!(seen.is_empty());
        assert_eq!(last, 2);
    }

    /// A directory that holds no client files, so the open must fail.
    const MISSING_UOPATH: &str = "/uoterm-has-no-client-here";
    /// A cache that opened nothing.
    const NO_OPENS: u64 = 0;
    /// A cache that holds no facet.
    const NO_LIVE_FACETS: usize = 0;

    #[test]
    fn a_failed_open_leaves_nothing_in_the_cache() {
        let facets = FacetCache::<MulMap>::default();
        let err = shared_facet(&facets, Path::new(MISSING_UOPATH), START_MAP_INDEX);
        assert!(err.is_err(), "an empty directory must not open a facet");
        assert_eq!(facets.opens(), NO_OPENS);
        assert_eq!(facets.live(), NO_LIVE_FACETS);
    }

    #[test]
    fn a_numbered_speech_line_becomes_english_in_the_journal() {
        const REFUSAL: u32 = 1_001_018;
        const SENTENCE: &str = "You cannot perform negative acts on your target.";
        let mut inner = test_session();
        inner.cliloc = Some(Arc::new(ClilocData::from_entries(
            [(REFUSAL, SENTENCE.to_string())].into_iter().collect(),
        )));
        let mut w = uoterm_protocol::buf::PacketWriter::with_variable(PKT_CLILOC);
        w.u32(1)
            .u16(0)
            .u8(0)
            .u16(0)
            .u16(3)
            .u32(REFUSAL)
            .ascii_fixed("System", 30)
            .u16(0);
        let bytes = w.finish_variable().unwrap();
        ingest(&mut inner, &bytes);
        let line = inner.world.read().journal.last_lines(1)[0].text.clone();
        assert_eq!(line, SENTENCE);
    }

    #[test]
    fn attack_is_sent_once_until_the_fight_ends() {
        const ENEMY: Serial = Serial(0x0000_1234);
        let mut inner = test_session();
        {
            let mut world = inner.world.write();
            world.logged_in = true;
            world.self_state.war = true;
        }
        send_attack(&mut inner, ENEMY);
        send_attack(&mut inner, ENEMY);
        let attacks = inner
            .outbound
            .iter()
            .filter(|p| p.first() == Some(&PKT_ATTACK))
            .count();
        assert_eq!(attacks, 1);
        inner.attack_sent = None;
        inner.world.write().combatant = None;
        let mut ended = uoterm_protocol::buf::PacketWriter::new(PKT_COMBATANT);
        ended.serial(Serial::INVALID);
        ingest(&mut inner, &ended.finish());
        assert!(inner.attack_sent.is_none());
        send_attack(&mut inner, ENEMY);
        let attacks = inner
            .outbound
            .iter()
            .filter(|p| p.first() == Some(&PKT_ATTACK))
            .count();
        assert_eq!(attacks, 2);
    }

    #[test]
    fn war_mode_stays_on_during_a_fight() {
        let mut inner = test_session();
        inner.world.write().combatant = Some(Serial(0x0000_1234));
        send_war_mode(&mut inner, false);
        assert!(
            inner.outbound.is_empty(),
            "turning war off would clear the fight"
        );
    }

    #[test]
    fn a_target_intent_is_answered_when_the_cursor_arrives() {
        const TREE: Serial = Serial(0x4000_0010);
        const CURSOR_ID: u32 = 9;
        let mut inner = test_session();
        inner.target_intent = Some(TREE);
        let mut w = uoterm_protocol::buf::PacketWriter::new(PKT_TARGET);
        w.u8(0)
            .u32(CURSOR_ID)
            .u8(0)
            .u32(0)
            .u16(0)
            .u16(0)
            .u8(0)
            .i8(0)
            .u16(0);
        ingest(&mut inner, &w.finish());
        assert!(inner.target_intent.is_none());
        assert!(inner.world.read().pending_target.is_none());
        assert!(
            inner
                .outbound
                .iter()
                .any(|p| p.first() == Some(&PKT_TARGET)),
            "the cursor must be answered in the packet handler"
        );
    }

    #[test]
    fn drop_into_the_pack_uses_auto_place_coordinates() {
        const ITEM: Serial = Serial(0x4000_0001);
        const PACK: Serial = Serial(0x4000_0002);
        let p = encode::drop_into_container(ITEM, PACK, Some(0));
        assert_eq!(&p[5..9], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn bandage_command_has_no_cursor() {
        const BANDAGE: Serial = Serial(0x4000_0101);
        const SELF: Serial = Serial(0x0000_00AB);
        let p = encode::bandage_target(BANDAGE, SELF);
        assert_eq!(p.len(), BANDAGE_TARGET_LEN);
        assert_eq!(p[0], PKT_EXTENDED);
        assert_eq!(&p[3..5], &EXT_BANDAGE_TARGET.to_be_bytes());
    }

    #[test]
    fn action_budget_is_one_thousand_milliseconds() {
        assert_eq!(ACTION_BUDGET, Duration::from_millis(1000));
        let mut inner = test_session();
        assert!(action_ready(&inner));
        mark_action(&mut inner);
        assert!(!action_ready(&inner));
    }
}

fn ingest(inner: &mut Inner, data: &[u8]) -> Vec<Inbound> {
    let raw = if inner.compressed {
        inner.decoder.push_compressed(data)
    } else {
        inner.decoder.push_plain(data)
    };
    let raw = match raw {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "decoder reset after framing error");
            inner.decoder.reset();
            inner.outbound.push_back(encode::resync());
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    let self_serial = inner.world.read().self_state.serial;
    for pkt in raw {
        tracing::trace!(
            id = format!("{:#04x}", pkt.id),
            bytes = pkt.bytes.len(),
            "inbound packet"
        );
        match parse_with_version(&pkt.bytes, inner.version) {
            Ok(msg) => {
                // One packet may be a bundle of several. Unpack it and treat
                // each one as if it had arrived on its own, or every item
                // inside it is silently dropped.
                let bundled = match msg {
                    Inbound::PacketList(entries) => entries,
                    one => vec![one],
                };
                for mut msg in bundled {
                    if let Inbound::Speech(line) = &mut msg {
                        if let Some(cliloc) = &inner.cliloc {
                            line.text = cliloc.render_line(&line.text);
                        }
                    }
                    match &msg {
                        Inbound::MoveAck { sequence, .. } => {
                            accept_move_ack(inner, *sequence);
                        }
                        Inbound::MoveReject {
                            sequence, x, y, z, ..
                        } => {
                            // The world model snaps the character to the tile the
                            // refusal carries when it applies the packet below.
                            refuse_step(inner, *sequence, Point3::new(*x, *y, *z), Instant::now());
                        }
                        Inbound::FastwalkKeys(keys) => {
                            inner.movement.set_fastwalk(*keys);
                        }
                        Inbound::FastwalkKeyAdd(key) => {
                            // This packet hands out one replacement key, unlike
                            // the packet above which replaces the whole stack.
                            // Without it the stack empties and never refills.
                            inner.movement.push_fastwalk(*key);
                        }
                        Inbound::Speech(line) if says_too_fatigued(&line.text) => {
                            inner.last_fatigued = Some(Instant::now());
                        }
                        Inbound::Speech(line) if says_action_too_soon(&line.text) => {
                            inner.next_action_at = Instant::now() + ACTION_BUDGET;
                        }
                        Inbound::DrawPlayer { serial, .. } if *serial == self_serial => {
                            inner.movement.clear_in_flight();
                        }
                        Inbound::CombatantChanged { serial } if !serial.is_valid() => {
                            inner.attack_sent = None;
                        }
                        _ => {}
                    }
                    let stood_at = inner.world.read().self_state.location;
                    {
                        let mut world = inner.world.write();
                        world.apply(&msg);
                    }
                    if let Inbound::Target(cursor) = &msg {
                        if let Some(serial) = inner.target_intent.take() {
                            inner
                                .outbound
                                .push_back(encode::target_object(cursor.id, serial, 0, 0, 0, 0));
                            inner.world.write().clear_target();
                        }
                    }
                    if let Inbound::LiftRejected { .. } = &msg {
                        inner.world.write().holding = None;
                    }
                    if let Inbound::Speech(line) = &msg {
                        if says_bandage_started(&line.text) {
                            let dex = inner.world.read().self_state.dex;
                            inner.next_bandage_at =
                                Instant::now() + Duration::from_millis(bandage_self_ms(dex));
                        }
                    }
                    // Only the server may move the character, so every move is
                    // worth naming. Without this a wrong position has no author and
                    // the fault can only be guessed at.
                    let stands_at = inner.world.read().self_state.location;
                    if stands_at != stood_at {
                        tracing::debug!(
                            packet = format!("{:#04x}", pkt.id),
                            from = %stood_at,
                            to = %stands_at,
                            "a packet moved the character"
                        );
                    }
                    if let Inbound::WorldItem(item) = &msg {
                        note_door_item(inner, item);
                        note_multi_item(inner, item);
                    }
                    harvest_new_events(inner);
                    if matches!(&msg, Inbound::MapChange { .. }) {
                        inner.ensure_facet();
                    }
                    out.push(msg);
                }
            }
            Err(e) => tracing::warn!(error = %e, id = pkt.id, "packet parse skipped"),
        }
    }
    out
}

/// Puts the character on the tile the server has just confirmed.
///
/// This is the only way a walk moves him. Sending a step says nothing about
/// where he is: until this answer comes back he stands where the server last
/// put him, so every reader of his position reads fact and never a guess.
/// Several requests may be waiting for their answers, and each answer takes him
/// one tile on, in the order the server sends them. A sequence that matches
/// no request in flight moves nobody.
///
/// The answer to a turn moves nobody either. The server answers a turn exactly
/// as it answers a step, so the two can only be told apart by what the client
/// asked for, and a turn asked for no tile: it carries the tile the requests
/// before it leave him on, which is the tile he is already standing on. That
/// is the whole of it. Crediting a turn a tile is what put this client one tile
/// ahead of the truth, and one further ahead on every change of direction.
fn accept_move_ack(inner: &mut Inner, sequence: u8) {
    let Some(step) = inner.movement.ack(sequence) else {
        tracing::debug!(sequence, "an answer matched no request in flight");
        return;
    };
    tracing::debug!(
        sequence,
        to = %step.arrives_at,
        turn = step.turn,
        "the server confirmed a request"
    );
    {
        let mut w = inner.world.write();
        w.self_state.location = step.arrives_at;
        w.self_state.direction = step.direction as u8;
    }
    if step.turn {
        return;
    }
    // The server has just walked him onto that tile, so whatever it refused
    // him there before is over.
    forget_refused_tile(inner, step.arrives_at, FORGOT_HE_STANDS_ON_IT);
}

/// Remembers the one tile the server would not let the character enter.
///
/// Exactly one cell, at radius nought, and never a line guessed at past it. A
/// refusal is one fact about one crossing, and nothing in it says how far the
/// thing in the way runs: a wall curves, a fence has a gate in it, and a queue
/// of people is no wall at all. Tiles guessed at close the very gap he should
/// walk through.
///
/// The door is asked first, and a tile a door stands on is never remembered.
/// A shut door is the commonest thing on a shard that refuses a step, and it
/// is one the character can do something about: the door items already block
/// its tile, so the route stops in front of it, [`door_route`] names it and he
/// opens it. Remembering it as well would leave him walking around the very
/// doorway he had just opened until the memory ran out. What is left is every
/// refusal no door explains, which is the player house or the boat this
/// memory is for.
fn remember_refused_tile(inner: &mut Inner, at: Point3, now: Instant) {
    let doors = inner.doors_seen();
    if let Some(door) = movement::door_on_tile(&doors, at.x, at.y, at.z) {
        tracing::debug!(
            at = %at,
            serial = %door.serial,
            "the server refused a doorway, which the door logic opens"
        );
        return;
    }
    tracing::debug!(at = %at, "a tile the server refused is remembered");
    if let Some(dropped) = inner.movement.blocked.refuse(at, now) {
        log_forgotten(dropped, FORGOT_MEMORY_FULL);
    }
}

/// True when that line of speech is the server saying the character is too
/// tired to move.
///
/// The shard writes it either way: as plain words, or as the number of the
/// same line in the client string file, which reaches this client as
/// `#`[`CLILOC_TOO_FATIGUED`]. Both are the same fact, and it is one the
/// character must wait out rather than mark a tile for.
fn says_too_fatigued(text: &str) -> bool {
    text.to_ascii_lowercase().contains(FATIGUED_WORD)
        || text.starts_with(&format!("{CLILOC_PREFIX}{CLILOC_TOO_FATIGUED}"))
}

fn says_action_too_soon(text: &str) -> bool {
    text.to_ascii_lowercase().contains(ACTION_TOO_SOON_WORDS)
        || text.starts_with(&format!("{CLILOC_PREFIX}{CLILOC_ACTION_TOO_SOON}"))
}

fn says_bandage_started(text: &str) -> bool {
    text.to_ascii_lowercase().contains(BANDAGE_STARTED_WORDS)
        || text.starts_with(&format!("{CLILOC_PREFIX}{CLILOC_BANDAGE_START}"))
}

/// What the character does about one refusal, and why. Each is one rung of
/// [`refuse_step`], tried in this order and stopping at the first that fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// The sequence is not one the character is waiting on: the echo of a
    /// refusal already dealt with. Nothing is marked and nothing is planned.
    StaleEcho,
    /// The server has just said he is too tired to move.
    Fatigued,
    /// Somebody stands on the cell. The server does not test mobiles for a
    /// player move at all: it turns the step into a shove, and a shove is
    /// refused unless stamina is full. So the way is nearly always open again
    /// in a moment.
    Waiting,
    /// He has waited at that one cell long enough to treat it as shut.
    WaitedOut,
    /// He has waited over the whole trip long enough to give it up.
    GaveUp,
    /// A building the server sent stands on the cell.
    Building,
    /// The first refusal of that crossing: worth a door and one more try.
    TryTheDoor,
    /// The second refusal of that crossing: the way is shut.
    Shut,
}

/// What the character does about a step the server refused.
///
/// The ladder is tried in order and stops at the first rung that fits, because
/// the reasons are not equally likely and only one of them is a wall. Going
/// straight from a refusal to a blocked tile is what filed 68 refusals against
/// one tile beside the Britain bank: the tile was marked over and over for
/// reasons no mark could fix.
///
/// Waiting is right for a mobile in particular. The server does not test
/// mobiles for a player move: it turns the step into a shove, and refuses the
/// shove unless stamina is full. A refusal at an occupied tile is therefore
/// nearly always a moment's business and not a wall, and a character who marks
/// it walks the long way round every person he meets.
fn refuse_step(inner: &mut Inner, sequence: u8, at: Point3, now: Instant) -> Refusal {
    // The oldest request on the wire is the one being refused: the server
    // answers in the order it was sent, so nothing sent after it can be
    // answered first.
    let Some(refused) = inner.movement.in_flight.front().cloned() else {
        tracing::debug!(sequence, at = %at, "a refusal for a step that is not on the wire");
        return Refusal::StaleEcho;
    };
    if !inner.movement.holds_sequence(sequence) {
        tracing::debug!(
            sequence,
            at = %at,
            "a refusal on a sequence he is not waiting on: the echo of one already dealt with"
        );
        return Refusal::StaleEcho;
    }
    // The tile he was trying to enter, at the height the step was aimed at,
    // and the tile he was stepping from, which is the one the refusal carries.
    let cell = Point3::new(refused.arrives_at.x, refused.arrives_at.y, at.z);
    inner.movement.refused();
    inner.outbound.push_back(encode::resync());
    tracing::debug!(at = %at, cell = %cell, direction = ?refused.direction, "the server refused a step");

    let what = judge_refusal(inner, at, cell, now);
    match what {
        // An echo is judged above, before anything is thrown away, so this
        // rung is never reached from here and nothing is owed on it.
        Refusal::StaleEcho => {}
        Refusal::Fatigued => {
            inner.movement.wait(now, FATIGUE_WAIT);
            replan_the_route(inner);
        }
        Refusal::Waiting => {
            inner.movement.wait(now, MOBILE_WAIT);
            replan_the_route(inner);
        }
        Refusal::WaitedOut => {
            inner.movement.stop_waiting();
            remember_refused_tile(inner, cell, now);
            inner.movement.wait(now, REPLAN_RESUME);
            replan_the_route(inner);
        }
        Refusal::GaveUp => stop_the_trip(inner, cell, WAITED_TOO_LONG.into()),
        Refusal::Building => {
            inner.movement.wait(now, REPLAN_RESUME);
            replan_the_route(inner);
        }
        Refusal::TryTheDoor => {
            open_the_door_ahead(inner, at, refused.direction, now);
            // The same step goes out again once the leaf has had time to
            // swing: the refusal put it back at the head of the route.
            // Nothing is marked, because one refusal at a doorway is the door.
            inner.movement.wait(now, DOOR_RETRY_WAIT);
        }
        Refusal::Shut => {
            remember_refused_tile(inner, cell, now);
            let resume = rand::thread_rng().gen_range(SHUT_RESUME_MIN_MS..=SHUT_RESUME_MAX_MS);
            inner.movement.wait(now, Duration::from_millis(resume));
            replan_the_route(inner);
        }
    }
    tracing::debug!(cell = %cell, ?what, "the refusal is answered");
    what
}

/// Which rung of the ladder this refusal belongs on. Nothing here changes
/// anything: [`refuse_step`] acts on the answer.
fn judge_refusal(inner: &mut Inner, at: Point3, cell: Point3, now: Instant) -> Refusal {
    if inner
        .last_fatigued
        .is_some_and(|when| now.saturating_duration_since(when) <= FATIGUE_WINDOW)
    {
        return Refusal::Fatigued;
    }
    if somebody_stands_on(inner, cell) {
        let (waits, this_trip) = inner.movement.waited_at(cell);
        if this_trip >= WAITS_GIVE_UP {
            return Refusal::GaveUp;
        }
        if waits >= WAITS_BLOCK_CELL {
            return Refusal::WaitedOut;
        }
        return Refusal::Waiting;
    }
    if inner
        .building_walls()
        .iter()
        .any(|wall| wall.x == cell.x && wall.y == cell.y)
    {
        return Refusal::Building;
    }
    let refusals = inner.movement.refused_edges.refuse(at, cell, now);
    if refusals <= EDGE_REFUSALS_FIRST {
        Refusal::TryTheDoor
    } else {
        Refusal::Shut
    }
}

/// True while another mobile stands on that cell on the character's own floor.
///
/// Every mobile counts, whatever the facet's rules say about who may walk
/// through whom: the server has just refused the step, and the person standing
/// there is the plainest reason for it.
fn somebody_stands_on(inner: &Inner, cell: Point3) -> bool {
    inner.world.read().mobiles.values().any(|mobile| {
        mobile.location.x == cell.x
            && mobile.location.y == cell.y
            && (i16::from(mobile.location.z) - i16::from(cell.z)).abs() <= MOBILE_SAME_FLOOR_Z
    })
}

/// Turns the character to the door beside him, so the open-door macro has
/// something to aim at.
///
/// The macro names no door. The server offsets one tile in the direction the
/// character faces and opens whatever door it finds there, so a character
/// facing the wrong way opens nothing and is told nothing. The turn is the
/// whole of the aim.
fn face_the_nearest_door(inner: &mut Inner, now: Instant) {
    /// A door the character can reach without walking is on one of the eight
    /// tiles around him, so no further than this in either direction.
    const WITHIN_REACH: u32 = 1;

    let (at, reported) = {
        let world = inner.world.read();
        (
            world.self_state.location,
            Direction::from_byte(world.self_state.direction),
        )
    };
    let facing = inner.movement.facing_after(reported);
    let nearest = inner
        .world
        .read()
        .door_tiles()
        .into_iter()
        .filter(|tile| at.chebyshev(*tile) <= WITHIN_REACH)
        .min_by_key(|tile| at.chebyshev(*tile));
    let Some(door) = nearest else {
        return;
    };
    if let Some(toward) = movement::facing_toward(at, door) {
        if let Some(turn) = inner.movement.build_turn(facing, toward, at, now) {
            inner.outbound.push_back(turn);
        }
    }
}

/// Turns the character to the tile that refused him and asks the server to
/// open whatever door stands in it.
///
/// The macro names no door. The server opens the door in the tile the
/// character faces, so it cannot pick the wrong one and he needs no walk to
/// the door item to send it. The turn is what aims it.
fn open_the_door_ahead(inner: &mut Inner, at: Point3, toward: Direction, now: Instant) {
    let facing = Direction::from_byte(inner.world.read().self_state.direction);
    if let Some(turn) = inner.movement.build_turn(facing, toward, at, now) {
        inner.outbound.push_back(turn);
    }
    inner.outbound.push_back(encode::open_door());
}

/// Ends a trip that is going nowhere, and tells the caller why so it can
/// choose somewhere else.
fn stop_the_trip(inner: &mut Inner, at: Point3, reason: String) {
    inner.movement.hold();
    inner.movement.end_trip();
    inner.follow_state = FollowState::default();
    note_path_failure(inner, at, reason);
}

/// Throws the queued route away and counts it against the trip's budget. A
/// trip planned again [`movement::REPLANS_MAX`] times is one nothing is going
/// to solve, and it ends instead of looping.
fn replan_the_route(inner: &mut Inner) {
    inner.movement.path.clear();
    inner.follow_state = FollowState::default();
    if !inner.movement.count_replan() {
        let at = inner.world.read().self_state.location;
        stop_the_trip(inner, at, REPLANNED_TOO_OFTEN.into());
    }
}

/// Forgets one refused tile, and says so when there was one to forget.
fn forget_refused_tile(inner: &mut Inner, at: Point3, why: &str) {
    if inner.movement.blocked.forget(at) {
        log_forgotten(at, why);
    }
}

fn log_forgotten(at: Point3, why: &str) {
    tracing::debug!(at = %at, why, "a tile the server refused is forgotten");
}

/// Ask the server for the names of things we can see but cannot name.
///
/// A UO server never volunteers display names. Without this the world model
/// shows every person and item as a blank string.
fn pump_names(inner: &mut Inner) {
    if inner.last_name_retry.elapsed() >= NAME_RETRY {
        inner.last_name_retry = Instant::now();
        inner.world.write().names.retry_unanswered();
    }
    let batch = inner
        .world
        .write()
        .names
        .take_batch(BATCH_QUERY_PROPERTIES_MAX);
    if !batch.is_empty() {
        inner
            .outbound
            .push_back(encode::batch_query_properties(&batch));
    }
}

/// Describe what the character can see, in words an agent can act on.
fn scene_value(inner: &mut Inner, radius: u16) -> Value {
    inner.ensure_facet();
    let idx = inner.map_index();
    let world = inner.world.read();
    let scene = match inner.maps.get(&idx) {
        Some(mul) => scene::look_around(&world, mul.as_ref(), radius),
        None => scene::look_around(&world, &inner.map, radius),
    };
    serde_json::to_value(scene).unwrap_or_else(|_| json!({}))
}

fn harvest_new_events(inner: &mut Inner) {
    let id = inner.id.clone();
    let world = inner.world.read();
    visit_new_events(&world.events, &mut inner.last_event_seq, |ev| {
        harvest::append(&id, ev);
    });
}

fn visit_new_events<'a>(
    events: &'a [uoterm_world::Event],
    last_seq: &mut u64,
    mut visit: impl FnMut(&'a uoterm_world::Event),
) {
    for ev in events {
        if ev.seq > *last_seq {
            visit(ev);
            *last_seq = ev.seq;
        }
    }
}

fn drop_grid(inner: &Inner) -> Option<u8> {
    if inner.version.has_container_grid() {
        Some(0)
    } else {
        None
    }
}

fn apply_path(inner: &mut Inner, points: Vec<Point3>, dest: Point3) -> bool {
    let path_len = points.len();
    inner.movement.set_path(points, dest);
    inner.world.write().set_nav(Some(dest), path_len);
    inner.last_path_fail = None;
    true
}

/// Keeps pace with a followed mobile: walks to a free tile beside it, holds
/// still inside the close-enough band, and searches for a path only when that
/// search can pay for itself.
fn follow_tick(inner: &mut Inner, target_at: Point3, target_running: bool) {
    // A door attempt is underway: the character is walking to the door he must
    // pass, or waiting for the leaf to swing. A new route now would walk him
    // away from it, and a second click would shut it again.
    if inner.doors.waiting() {
        return;
    }
    // The tile the steps already on the wire leave him on. It is no guess:
    // every one of those steps is a request he has made, and a refusal or a
    // lost answer throws the whole lot away and starts again from the tile the
    // server names. Reading the gap from the tile he is reported on instead
    // would have him close a gap he has already closed, and walk a circle
    // round the target.
    let confirmed_at = inner.world.read().self_state.location;
    let self_at = inner.movement.stepping_from(confirmed_at);
    let path_active = inner.movement.walking();
    match inner.follow_state.decide(self_at, target_at, path_active) {
        FollowDecision::Continue => {}
        FollowDecision::Hold => inner.movement.hold(),
        FollowDecision::Repath => {
            inner.ensure_facet();
            let in_the_way = inner.blockers();
            let plan = follow_plan(
                inner.tiles(),
                self_at,
                target_at,
                target_running,
                &in_the_way.obstacles(),
            );
            match plan {
                Some(plan) => {
                    // The pace, and the whole of what keeps the gap shut: a
                    // follower that only matches the pace of the person ahead
                    // keeps every tile it has already lost.
                    let (stam, stam_max) = {
                        let world = inner.world.read();
                        (world.self_state.stam, world.self_state.stam_max)
                    };
                    inner.movement.run_override = Some(movement::follow_pace(
                        self_at,
                        target_at,
                        plan.running,
                        stam,
                        stam_max,
                    ));
                    apply_path(inner, plan.steps, plan.dest);
                    inner.follow_state.mark_path(true);
                }
                // No tile beside the target can be reached. A shut door is the
                // most common reason inside a building, and the walk to it is
                // the follower's path until it opens.
                None => match door_route(inner, self_at, target_at) {
                    DoorRoute::Underway => inner.follow_state.mark_path(true),
                    DoorRoute::Blocked => {
                        inner.movement.hold();
                        inner.follow_state.mark_path(false);
                    }
                    DoorRoute::None => {
                        inner.movement.hold();
                        inner.follow_state.mark_path(false);
                        note_path_failure(inner, self_at, FOLLOW_NO_WAY.into());
                    }
                },
            }
        }
    }
}

/// Keeps the world's record of door items in step with the item packets the
/// server sends.
///
/// A door that swings comes back as an ordinary item packet on the same serial
/// carrying a new graphic, a new tile, or both. That packet is the only proof
/// the server acted on a click, and it puts the leaf on a tile no queued route
/// knows about, so every route is planned again.
fn note_door_item(inner: &mut Inner, item: &GroundItem) {
    let idx = inner.map_index();
    let Some(is_door) = inner
        .maps
        .get(&idx)
        .map(|mul| mul.is_door_graphic(item.graphic))
    else {
        return;
    };
    if !is_door {
        inner.world.write().forget_door(item.serial);
        return;
    }
    let at = Point3::new(item.x, item.y, item.z);
    let update = inner.world.write().note_door(item.serial, item.graphic, at);
    if update != DoorUpdate::Swung {
        return;
    }
    tracing::info!(serial = %item.serial, at = %at, graphic = item.graphic, "a door swung");
    if inner.doors.answered(item.serial) {
        replan_the_route(inner);
    }
}

/// Keeps the world's record of the buildings in step with the item packets the
/// server sends.
///
/// A house or a boat reaches the client as an ordinary item whose graphic
/// names a multi, and the packet the server wrote is what says so. Nothing
/// here reads the shape: the record holds which multi it is and where it
/// stands, and [`Inner::building_walls`] turns that into the tiles its walls
/// close.
///
/// A building that comes into view or sails to another tile invalidates every
/// queued route, exactly as a door that swings does, because a route planned
/// before it was known runs through it.
fn note_multi_item(inner: &mut Inner, item: &GroundItem) {
    let Some(multi_id) = multi_id(item) else {
        inner.world.write().forget_multi(item.serial);
        return;
    };
    let at = Point3::new(item.x, item.y, item.z);
    let update = inner.world.write().note_multi(item.serial, multi_id, at);
    if update == MultiUpdate::Unchanged {
        return;
    }
    tracing::info!(
        serial = %item.serial,
        at = %at,
        multi = format!("{multi_id:#06x}"),
        ?update,
        "a building is in view"
    );
    // Not while he is walking to a door or waiting for the leaf to swing: that
    // walk is his route, and throwing it away would leave him standing still
    // until the attempt ran out of time. The door that swings plans the route
    // again itself, and the building is known by then.
    if !inner.doors.waiting() {
        replan_the_route(inner);
    }
}

/// What stands between the character and where he is going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DoorRoute {
    /// No door explains it. The way is shut for another reason.
    None,
    /// He is walking to a door, or waiting for the leaf to swing.
    Underway,
    /// The door will not let him through, and the failure is recorded.
    Blocked,
}

/// Sends the character to the tile in front of the door that stands in his way,
/// so a route that stops in front of a shut door can go on once it is open.
///
/// The walk keeps `dest` as its destination: the door is on the way there, not
/// the end of the journey, and a character who thinks he has arrived at a door
/// stops walking.
///
/// [`Inner::avoided`] goes in, and never [`Inner::blockers`]: the people and
/// the refused tiles block the walk to the tile in front of the door, and
/// nothing else. [`door_in_the_way`] adds the doors to that walk itself, and
/// names the door with a search of its own that neither a person nor a refused
/// tile blocks: somebody standing on the way is no reason to tell the
/// character there is no door in front of him, and the tile a shut door
/// refused him is the door itself.
fn door_route(inner: &mut Inner, from: Point3, dest: Point3) -> DoorRoute {
    inner.ensure_facet();
    let avoid = inner.avoided();
    let doors = inner.doors_seen();
    let Some(way) = door_in_the_way(inner.tiles(), from, dest, &avoid.obstacles(), &doors) else {
        return DoorRoute::None;
    };
    match inner.doors.plan(way, Instant::now()) {
        DoorPlan::Approach(way) => {
            tracing::info!(
                serial = %way.door.serial,
                door = %way.door.location,
                stand_on = %way.stand_on,
                "walking to the door in the way"
            );
            apply_path(inner, way.steps, dest);
            DoorRoute::Underway
        }
        DoorPlan::Underway => DoorRoute::Underway,
        DoorPlan::Opened(door) => {
            note_path_failure(
                inner,
                dest,
                format!("{DOOR_STILL_BLOCKS} at {}", door.location),
            );
            DoorRoute::Blocked
        }
        DoorPlan::Locked(door) => {
            note_path_failure(inner, dest, format!("{DOOR_LOCKED} at {}", door.location));
            DoorRoute::Blocked
        }
    }
}

/// Opens the door the character has walked up to. He turns to it first, the way
/// a player does, and asks once: the second ask is the one that shuts it
/// again.
///
/// What he sends is the macro that names no door: the server opens whatever
/// door stands in the tile he faces. It cannot pick the wrong door of a pair,
/// and it needs no serial, so it works on a door whose item packet this client
/// never saw. Our own budget of attempts is kept, which is more careful than
/// asking for ever.
fn pump_doors(inner: &mut Inner) {
    if !inner.doors.waiting() {
        return;
    }
    let now = Instant::now();
    if let Some(door) = inner.doors.expired(now) {
        tracing::info!(serial = %door.serial, door = %door.location, "the door did not answer");
    }
    // The tile the server last put him on. He is due to click a door only once
    // the server has confirmed the step that brought him in front of it, so he
    // never reaches for a door from a tile he has not reached.
    let (at, reported) = {
        let w = inner.world.read();
        (
            w.self_state.location,
            Direction::from_byte(w.self_state.direction),
        )
    };
    // The way he will face once the wire is empty, not the way the world model
    // last heard about. A walk acknowledgement carries no facing, so the world
    // still holds whichever direction some earlier packet set. Reading that
    // makes the turn below look unnecessary, and the macro then asks the
    // server to open a door in whatever tile he happened to face.
    let facing = inner.movement.facing_after(reported);
    let Some(door) = inner.doors.due(at, now) else {
        return;
    };
    // The turn goes to the server, and the server says which way he ends up
    // facing. Writing that facing here would be the same guess the walk used
    // to make. It is also what aims the macro below: the server opens the door
    // in the tile he faces.
    if let Some(toward) = facing_toward(at, door.location) {
        if let Some(turn) = inner.movement.build_turn(facing, toward, at, now) {
            inner.outbound.push_back(turn);
        }
    }
    inner.outbound.push_back(encode::open_door());
}

fn action_ready(inner: &Inner) -> bool {
    Instant::now() >= inner.next_action_at
}

fn mark_action(inner: &mut Inner) {
    inner.next_action_at = Instant::now() + ACTION_BUDGET;
}

fn backpack_serial(world: &uoterm_world::World) -> Option<Serial> {
    world
        .self_state
        .equipment
        .iter()
        .find(|eq| eq.layer == LAYER_BACKPACK)
        .map(|eq| eq.serial)
}

fn send_war_mode(inner: &mut Inner, on: bool) {
    if !on && inner.world.read().fighting() {
        return;
    }
    inner.outbound.push_back(encode::war_mode(on));
}

fn archer_move_locked(inner: &Inner) -> bool {
    let graphic = inner.world.read().equipped_weapon_graphic();
    if !graphic.is_some_and(is_ranged_weapon) {
        return false;
    }
    let Some(last) = inner.movement.in_flight.back() else {
        return false;
    };
    Instant::now().saturating_duration_since(last.sent_at)
        < Duration::from_millis(ARCHER_MOVE_LOCK_MS)
}

fn send_attack(inner: &mut Inner, serial: Serial) {
    if inner.attack_sent == Some(serial) {
        return;
    }
    if inner.world.read().combatant == Some(serial) {
        return;
    }
    inner.attack_sent = Some(serial);
    inner.outbound.push_back(encode::attack(serial));
}

fn store_or_answer_target(inner: &mut Inner, serial: Serial) {
    let cursor = inner.world.read().pending_target.clone();
    if let Some(cursor) = cursor {
        inner
            .outbound
            .push_back(encode::target_object(cursor.id, serial, 0, 0, 0, 0));
        inner.world.write().clear_target();
        inner.target_intent = None;
    } else {
        inner.target_intent = Some(serial);
    }
}

fn pump_loot(inner: &mut Inner) {
    let Some(job) = inner.loot.clone() else {
        return;
    };
    let world = inner.world.read().clone();
    match job.step(&world, action_ready(inner)) {
        LootStep::Walk { x, y, z } => {
            let _ = queue_move(inner, Point3::new(x, y, z));
        }
        LootStep::Open(serial) => {
            inner.outbound.push_back(encode::double_click(serial));
            mark_action(inner);
        }
        LootStep::Lift { serial, amount } => {
            inner.outbound.push_back(encode::lift(serial, amount));
            inner.world.write().holding = Some(serial);
            inner.sent_drop = None;
        }
        LootStep::Drop { serial, dest } => {
            if inner.sent_drop != Some(serial) {
                inner.outbound.push_back(encode::drop_into_container(
                    serial,
                    dest,
                    drop_grid(inner),
                ));
                inner.sent_drop = Some(serial);
            }
        }
        LootStep::Wait => {}
        LootStep::Done | LootStep::Fail(_) => {
            inner.loot = None;
        }
    }
}

fn send_bandage_self(inner: &mut Inner, world: &uoterm_world::World) {
    if Instant::now() < inner.next_bandage_at || !action_ready(inner) {
        return;
    }
    let Some(item) = world.find_item_graphic(GRAPHIC_BANDAGE) else {
        return;
    };
    inner
        .outbound
        .push_back(encode::bandage_target(item.serial, world.self_state.serial));
    mark_action(inner);
    inner.next_bandage_at =
        Instant::now() + Duration::from_millis(bandage_self_ms(world.self_state.dex));
}

fn reflex_tick(inner: &mut Inner) {
    let world = inner.world.read().clone();
    if !world.logged_in {
        return;
    }
    if inner.loot.is_some() {
        pump_loot(inner);
        return;
    }
    if let Some(serial) = inner.follow {
        if let Some(mob) = world.mobiles.get(&serial) {
            follow_tick(inner, mob.location, mob.running);
            return;
        }
    }
    match reflex::tick(&world, &inner.persona, &inner.goal) {
        ReflexAction::None => {
            if inner.world.read().fighting() {
                inner.movement.hold();
            }
        }
        ReflexAction::Say(text) => {
            let _ = queue_speech(inner, text, SPEECH_REGULAR);
        }
        ReflexAction::MoveTo { x, y, z } => {
            let _ = queue_move(inner, Point3::new(x, y, z));
        }
        ReflexAction::Attack(serial) => {
            inner.movement.hold();
            if archer_move_locked(inner) {
                return;
            }
            send_attack(inner, serial);
        }
        ReflexAction::WarMode(on) => send_war_mode(inner, on),
        ReflexAction::Use(serial) => {
            if let Some(item) = world.items.get(&serial) {
                if item.graphic == GRAPHIC_POTION_HEAL && Instant::now() < inner.next_heal_potion_at
                {
                    return;
                }
            }
            if !action_ready(inner) {
                return;
            }
            inner.outbound.push_back(encode::double_click(serial));
            mark_action(inner);
            if let Some(item) = world.items.get(&serial) {
                if item.graphic == GRAPHIC_POTION_HEAL {
                    inner.next_heal_potion_at =
                        Instant::now() + Duration::from_millis(heal_potion_lock_ms(&item.name));
                }
                if item.graphic == GRAPHIC_HATCHET {
                    if let Some(tree) = world.find_items(None, None, None).into_iter().find(|i| {
                        i.parent.is_none()
                            && (TREE_GRAPHIC_MIN..=TREE_GRAPHIC_MAX).contains(&i.graphic)
                    }) {
                        inner.target_intent = Some(tree.serial);
                    }
                }
            }
        }
        ReflexAction::UseSkill(id) => {
            inner.outbound.push_back(encode::use_skill(id));
        }
        ReflexAction::Target(serial) => store_or_answer_target(inner, serial),
        ReflexAction::BandageSelf => send_bandage_self(inner, &world),
    }
}

/// Warns and records a path failure at most once per `PATH_FAIL_LOG_EVERY`
/// for the same point, so a walker that has nowhere to go cannot flood the
/// log. `at` is the point that stays the same while the failure lasts: the
/// destination of a move, and the follower's own tile for a follow.
fn note_path_failure(inner: &mut Inner, at: Point3, reason: String) {
    let now = Instant::now();
    let quiet = matches!(
        inner.last_path_fail,
        Some((prev, when)) if prev == at && now.duration_since(when) < PATH_FAIL_LOG_EVERY
    );
    if quiet {
        return;
    }
    tracing::warn!(at = %at, error = %reason, "path failed");
    inner.world.write().push_event(uoterm_world::Event::new(
        uoterm_world::EventKind::PathFailed,
        None,
        reason,
    ));
    inner.last_path_fail = Some((at, now));
}

fn queue_move(inner: &mut Inner, dest: Point3) -> bool {
    // A walk to that tile is already under way, whether the next step is
    // queued or waiting for its answer. Planning it again would only replace
    // the route with itself.
    if inner.movement.goal == Some(dest) && inner.movement.walking() {
        return true;
    }
    // A new destination is a new journey, so the marks made on the way
    // somewhere else are dropped. A minute is a long time to plan around a
    // tile that was in the way of another walk.
    for forgotten in inner.movement.begin_trip(dest) {
        log_forgotten(forgotten, FORGOT_NEW_DESTINATION);
    }
    // The tile the steps already on the wire leave him on, which is the tile
    // the server last put him on whenever it owes him nothing. A route
    // planned from anywhere else is a route from a tile he is not going to be
    // standing on when he walks it.
    let from = inner
        .movement
        .stepping_from(inner.world.read().self_state.location);
    let in_the_way = inner.blockers();
    let obstacles = in_the_way.obstacles();
    inner.ensure_facet();
    match pathfind(inner.tiles(), from, dest, &obstacles) {
        Ok(p) => {
            let points: Vec<Point3> = p.steps.iter().map(|s| Point3::new(s.x, s.y, s.z)).collect();
            apply_path(inner, points, dest)
        }
        Err(e) => match pathfind_flat(inner.tiles(), from, dest, &obstacles) {
            Ok(p) => {
                let points: Vec<Point3> =
                    p.steps.iter().map(|s| Point3::new(s.x, s.y, s.z)).collect();
                tracing::warn!(
                    error = %e,
                    steps = points.len(),
                    "z-path failed; using walkable tiles"
                );
                apply_path(inner, points, dest)
            }
            // Both searches go around the doors, so a way that is shut only by
            // a door leaves the character in front of it, not lost.
            Err(flat_err) => match door_route(inner, from, dest) {
                DoorRoute::Underway => true,
                DoorRoute::Blocked => false,
                DoorRoute::None => {
                    note_path_failure(inner, dest, format!("{e}; flat: {flat_err}"));
                    false
                }
            },
        },
    }
}

#[cfg(test)]
fn greedy_line(from: Point3, dest: Point3) -> Vec<Point3> {
    let mut x = from.x;
    let mut y = from.y;
    let mut out = Vec::new();
    while (x != dest.x || y != dest.y) && out.len() < GREEDY_STEP_CAP {
        if x < dest.x {
            x += 1;
        } else if x > dest.x {
            x -= 1;
        }
        if y < dest.y {
            y += 1;
        } else if y > dest.y {
            y -= 1;
        }
        out.push(Point3::new(x, y, dest.z));
    }
    out
}

#[cfg(test)]
fn player_step_toward(map: &dyn TileQuery, from: Point3, dest: Point3) -> Option<Point3> {
    let mut best: Option<(u32, bool, u16, u16)> = None;
    const DIRS: [Direction; 8] = [
        Direction::North,
        Direction::Northeast,
        Direction::East,
        Direction::Southeast,
        Direction::South,
        Direction::Southwest,
        Direction::West,
        Direction::Northwest,
    ];
    for dir in DIRS {
        let Some(next) = from.neighbour(dir) else {
            continue;
        };
        if !map.can_walk(next.x, next.y) {
            continue;
        }
        let (dx, dy) = dir.delta();
        if dx != 0 && dy != 0 {
            let sx = (from.x as i32 + dx) as u16;
            let sy = (from.y as i32 + dy) as u16;
            if !map.can_walk(sx, from.y) || !map.can_walk(from.x, sy) {
                continue;
            }
        }
        let d = Point3::new(next.x, next.y, from.z).chebyshev(dest);
        let cardinal = dx == 0 || dy == 0;
        let cand = (d, !cardinal, next.x, next.y);
        if match best {
            None => true,
            Some(b) => cand < b,
        } {
            best = Some(cand);
        }
    }
    best.map(|(_, _, x, y)| Point3::new(x, y, from.z))
}

#[cfg(test)]
fn player_path(map: &dyn TileQuery, from: Point3, dest: Point3) -> Vec<Point3> {
    let mut at = from;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert((at.x, at.y));
    while (at.x != dest.x || at.y != dest.y) && out.len() < PLAYER_STEP_CAP {
        let Some(next) = player_step_toward(map, at, dest) else {
            break;
        };
        if !seen.insert((next.x, next.y)) {
            break;
        }
        if next.chebyshev(dest) > at.chebyshev(dest) && out.len() > 4 {
            break;
        }
        out.push(next);
        at = next;
    }
    out
}

fn hold_step_count(hold_ms: u64, running: bool) -> usize {
    if hold_ms == HOLD_MS_NONE {
        return WALK_STEPS_ONE;
    }
    let interval = if running { STEP_RUN_MS } else { STEP_WALK_MS };
    let n = hold_ms.div_ceil(interval.max(1)) as usize;
    n.clamp(WALK_STEPS_ONE, HOLD_STEP_CAP)
}

/// The tiles a walk held in one direction lands on, each at the height the map
/// says the character reaches there.
///
/// No route plans this walk, so the height of every step is asked of the map
/// one step at a time, with [`TileQuery::can_step`]: the same whole movement
/// test a planned route puts to each of its steps. Taking the height of the
/// tile he leaves instead would put a height he has never been at on the tile
/// the server confirms.
///
/// The walk ends at the first step the map has no answer for. That answer is
/// the arrival height, so a step without one is a step with no height to
/// record, and the map has just said it is not a step he can take.
fn hold_path<M: TileQuery + ?Sized>(
    map: &M,
    from: Point3,
    dir: Direction,
    steps: usize,
) -> Vec<Point3> {
    let mut at = from;
    let mut out = Vec::new();
    for _ in 0..steps {
        let Some(next) = at.neighbour(dir) else {
            break;
        };
        let Some(arrives_at_z) = map.can_step(at, next.x, next.y) else {
            break;
        };
        at = Point3::new(next.x, next.y, arrives_at_z);
        out.push(at);
    }
    out
}

fn walk_hold(inner: &mut Inner, args: &Value) -> ToolResult {
    let dir_raw = args
        .get("direction")
        .or_else(|| args.get("dir"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let Some(dir) = Direction::from_name(dir_raw) else {
        return ToolResult::err("walk needs direction n|ne|e|se|s|sw|w|nw");
    };
    let running = args
        .get("running")
        .or_else(|| args.get("run"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let hold_ms = args
        .get("hold_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(HOLD_MS_NONE);
    let asked_for = hold_step_count(hold_ms, running);
    // The walk carries on from the end of the steps already on the wire, so a
    // walk ordered in the middle of one does not double back over them.
    let from = inner
        .movement
        .stepping_from(inner.world.read().self_state.location);
    // The map is what says how high each of these steps lands, so the facet
    // is opened before it is asked.
    inner.ensure_facet();
    let points = hold_path(inner.tiles(), from, dir, asked_for);
    let Some(dest) = points.last().copied() else {
        return ToolResult::err("cannot step that way");
    };
    let steps = points.len();
    inner.follow = None;
    inner.goal = Goal::Idle;
    inner.world.write().goal = Goal::Idle.name().into();
    inner.movement.run_override = Some(running);
    inner.movement.set_path(points, dest);
    ToolResult::ok(json!({
        "action": "walk",
        "direction": dir_raw,
        "running": running,
        "steps": steps,
        "dest": format!("{dest}")
    }))
}

/// Walks the queued route at the pace of a person, and moves the character
/// only when the server says so.
///
/// He sends a step and does not move. Neither does the world model:
/// [`accept_move_ack`] puts him on the next tile when the server confirms
/// that step, and until then every tool, route and description reads the tile
/// the server last put him on. Several steps may be on the wire at once, so
/// that tile trails the one he is walking to; what spaces the steps is the
/// pace of a person, never the wait for an answer.
fn pump_movement(inner: &mut Inner, now: Instant) {
    if inner.movement.expire_stale(now) {
        tracing::debug!("no answer to a step; asking the server where the character is");
        inner.outbound.push_back(encode::resync());
    }
    // The memory is swept on the clock the walk runs on, before anything below
    // plans a route from it, so a tile is blocked for its full
    // [`movement::REFUSED_TILE_MEMORY`] and at most one tick more.
    for forgotten in inner.movement.blocked.expire(now) {
        log_forgotten(forgotten, FORGOT_TIME_UP);
    }
    inner.movement.refused_edges.expire(now);
    let (from, stam, stam_max, mounted) = {
        let world = inner.world.read();
        (
            world.self_state.location,
            world.self_state.stam,
            world.self_state.stam_max,
            // The pace of a mount is not the pace of the person on it, and the
            // item the server puts on the mount layer is the only word this
            // client gets that he is riding.
            movement::is_mounted(&world.self_state.equipment),
        )
    };
    inner.movement.mounted = mounted;
    let travel = matches!(inner.goal, Goal::Travel { .. });
    let danger = matches!(inner.goal, Goal::Flee | Goal::Hunt);
    let running = inner
        .movement
        .run_override
        .unwrap_or_else(|| movement::should_run(false, danger || travel, false, stam, stam_max));
    if inner.movement.arrived(from) && inner.movement.goal.is_some() {
        inner.world.write().push_event(uoterm_world::Event::new(
            uoterm_world::EventKind::Arrived,
            None,
            format!("{from}"),
        ));
        inner.movement.goal = None;
        inner.movement.run_override = None;
        if travel {
            inner.goal = Goal::Idle;
            inner.world.write().goal = Goal::Idle.name().into();
        }
    }
    if let Goal::Travel { dest } = inner.goal {
        // Not while the walk is still under way: the queued route already
        // goes there. Not while a door is being opened either: the walk to
        // that door is the route now, and planning again would only walk him
        // away from it.
        if !inner.movement.walking()
            && !inner.doors.waiting()
            && (from.x != dest.x || from.y != dest.y)
        {
            queue_move(inner, dest);
        }
    }
    if !inner.movement.ready(now) {
        return;
    }
    // The tile this step is aimed from: the end of the steps already on the
    // wire. `from` is the tile he is reported on, and only a confirmation
    // changes that; a step aimed from `from` while others are still out would
    // send him back over ground he has already asked to cross.
    let stepping_from = inner.movement.stepping_from(from);
    // The tile the route named, at the height the route worked out for it.
    // That height is what the world model records when the server confirms
    // the step, so it is taken from the route and never from the tile he
    // leaves. It stays on the route until it really goes out as a step.
    let Some(step) = inner.movement.peek_next_step(stepping_from) else {
        return;
    };
    // A move request in a direction the character does not face turns him and
    // moves him nowhere: the server sets the new location to the old one and
    // answers all the same. So the same direction goes out twice, once to turn
    // and once to step, and the turn takes no tile out of the route. Sending
    // the step alone left every change of direction crediting a tile he had
    // never moved to, and the client's idea of where he stood ran a tile
    // further ahead of the truth on every corner.
    let facing = inner.movement.facing_after(Direction::from_byte(
        inner.world.read().self_state.direction,
    ));
    if facing != step.direction {
        if let Some(turn) = inner
            .movement
            .build_turn(facing, step.direction, stepping_from, now)
        {
            tracing::debug!(from = ?facing, to = ?step.direction, "turning before the step");
            inner.outbound.push_back(turn);
        }
        return;
    }
    inner.movement.pop_next_step(stepping_from);
    let pkt = inner.movement.build_step(step, running, now);
    inner.outbound.push_back(pkt);
}

fn handle_tool(inner: &mut Inner, call: ToolCall) -> ToolResult {
    let args = &call.args;
    match call.name.as_str() {
        TOOL_OBSERVE => ToolResult::ok(observe_value(inner)),
        TOOL_LOOK_AROUND => {
            let radius = args
                .get("radius")
                .and_then(|v| v.as_u64())
                .map(|r| r as u16)
                .unwrap_or(scene::SCENE_RADIUS);
            ToolResult::ok(scene_value(inner, radius))
        }
        TOOL_FIND_MOBILES => {
            let name = args.get("name").and_then(|v| v.as_str());
            let graphic = args
                .get("graphic")
                .and_then(|v| v.as_u64())
                .map(|n| n as u16);
            let dist = args
                .get("distance")
                .and_then(|v| v.as_u64())
                .map(|n| n as u16);
            let found: Vec<_> = inner
                .world
                .read()
                .find_mobiles(name, graphic, dist)
                .iter()
                .map(|m| json!({"serial": m.serial, "name": m.name, "body": m.body}))
                .collect();
            ToolResult::ok(json!(found))
        }
        TOOL_FIND_ITEMS => {
            let graphic = args
                .get("graphic")
                .and_then(|v| v.as_u64())
                .map(|n| n as u16);
            // A serial an agent read out of an observation is written
            // `0x40000123`, so the container it names has to be read the same
            // way every other serial argument is.
            let container = arg_serial_opt(args, "container");
            let name = args.get("name").and_then(|v| v.as_str());
            let found: Vec<_> = inner
                .world
                .read()
                .find_items(graphic, container, name)
                .iter()
                .map(|i| {
                    json!({
                        "serial": i.serial,
                        "graphic": i.graphic,
                        "amount": i.amount,
                        "name": i.name,
                        "container": i.parent,
                    })
                })
                .collect();
            ToolResult::ok(json!(found))
        }
        TOOL_JOURNAL_SEARCH => {
            let q = args
                .get("q")
                .or_else(|| args.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            ToolResult::ok(json!(inner
                .world
                .read()
                .journal
                .search(q)
                .iter()
                .map(|e| &e.text)
                .collect::<Vec<_>>()))
        }
        TOOL_MAP_TILE => {
            let x = args.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let y = args.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let z = asked_z(args, inner.world.read().self_state.location.z);
            inner.ensure_facet();
            let t = inner.tiles().tile_from(z, x, y);
            ToolResult::ok(json!({"walkable": t.walkable(), "door": t.door, "z": t.z}))
        }
        TOOL_CAN_WALK => {
            let x = args.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let y = args.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let z = asked_z(args, inner.world.read().self_state.location.z);
            inner.ensure_facet();
            ToolResult::ok(json!(inner.tiles().can_walk_from(z, x, y)))
        }
        TOOL_SAY => speak(inner, args, SPEECH_REGULAR),
        TOOL_WHISPER => speak(inner, args, SPEECH_WHISPER),
        TOOL_EMOTE => {
            if !inner.persona.allow_emote {
                return ToolResult::err("persona does not asterisk-emote");
            }
            speak(inner, args, SPEECH_EMOTE)
        }
        TOOL_MOVE_TO => {
            let Some(x) = args.get("x").and_then(|v| v.as_u64()) else {
                return ToolResult::err("move_to needs x");
            };
            let Some(y) = args.get("y").and_then(|v| v.as_u64()) else {
                return ToolResult::err("move_to needs y");
            };
            let z = asked_z(args, inner.world.read().self_state.location.z);
            let dest = Point3::new(x as u16, y as u16, z);
            inner.goal = Goal::Travel { dest };
            inner.world.write().goal = Goal::Travel { dest }.name().into();
            if queue_move(inner, dest) {
                ToolResult::action(TOOL_MOVE_TO)
            } else {
                ToolResult::err("path failed")
            }
        }
        TOOL_WALK => walk_hold(inner, args),
        TOOL_OPEN_DOOR => {
            face_the_nearest_door(inner, Instant::now());
            inner.outbound.push_back(encode::open_door());
            ToolResult::action(TOOL_OPEN_DOOR)
        }
        TOOL_STOP | TOOL_CANCEL_GOAL => {
            inner.goal = Goal::Idle;
            inner.follow = None;
            inner.doors.give_up();
            inner.movement.clear();
            inner.world.write().goal = Goal::Idle.name().into();
            ToolResult::ok(json!(Goal::Idle.name()))
        }
        TOOL_USE | TOOL_OPEN_CONTAINER => {
            if !action_ready(inner) {
                return ToolResult::err("must wait to perform another action");
            }
            inner
                .outbound
                .push_back(encode::double_click(arg_serial(args, "serial")));
            mark_action(inner);
            ToolResult::action(TOOL_USE)
        }
        TOOL_LOOT => {
            let serial = arg_serial(args, "serial");
            let Some(pack) = backpack_serial(&inner.world.read()) else {
                return ToolResult::err("no backpack");
            };
            inner.loot = Some(LootJob::new(serial, pack));
            inner.sent_drop = None;
            pump_loot(inner);
            ToolResult::action(TOOL_LOOT)
        }
        TOOL_SINGLE_CLICK => {
            inner
                .outbound
                .push_back(encode::single_click(arg_serial(args, "serial")));
            ToolResult::action(TOOL_SINGLE_CLICK)
        }
        TOOL_ATTACK => {
            send_attack(inner, arg_serial(args, "serial"));
            ToolResult::action(TOOL_ATTACK)
        }
        TOOL_WAR_MODE => {
            let on = args.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
            if !on && inner.world.read().fighting() {
                return ToolResult::err("war mode stays on during a fight");
            }
            send_war_mode(inner, on);
            ToolResult::action(TOOL_WAR_MODE)
        }
        TOOL_LIFT => {
            if !action_ready(inner) {
                return ToolResult::err("must wait to perform another action");
            }
            let serial = arg_serial(args, "serial");
            let amount = args
                .get("amount")
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .max(1) as u16;
            inner.outbound.push_back(encode::lift(serial, amount));
            inner.world.write().holding = Some(serial);
            ToolResult::action(TOOL_LIFT)
        }
        TOOL_DROP => {
            let dest = args
                .get("dest")
                .and_then(|v| v.as_u64())
                .map(|n| Serial(n as u32))
                .unwrap_or(Serial::WORLD);
            let grid = drop_grid(inner);
            let serial = arg_serial(args, "serial");
            if dest.is_item() {
                inner
                    .outbound
                    .push_back(encode::drop_into_container(serial, dest, grid));
            } else {
                let loc = inner.world.read().self_state.location;
                inner
                    .outbound
                    .push_back(encode::drop(serial, loc.x, loc.y, loc.z, dest, grid));
            }
            ToolResult::action(TOOL_DROP)
        }
        TOOL_EQUIP => {
            let me = inner.world.read().self_state.serial;
            inner.outbound.push_back(encode::equip(
                arg_serial(args, "serial"),
                args.get("layer").and_then(|v| v.as_u64()).unwrap_or(1) as u8,
                me,
            ));
            ToolResult::action(TOOL_EQUIP)
        }
        TOOL_CAST => {
            inner.outbound.push_back(encode::cast_spell(
                args.get("spell").and_then(|v| v.as_u64()).unwrap_or(1) as u16,
            ));
            ToolResult::action(TOOL_CAST)
        }
        TOOL_USE_SKILL => {
            inner.outbound.push_back(encode::use_skill(
                args.get("skill")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(u64::from(SKILL_LUMBERJACKING)) as u16,
            ));
            ToolResult::action(TOOL_USE_SKILL)
        }
        TOOL_TARGET => {
            if args.get("serial").is_some() {
                store_or_answer_target(inner, arg_serial(args, "serial"));
                ToolResult::action(TOOL_TARGET)
            } else if let Some(cursor) = inner.world.read().pending_target.clone() {
                inner.outbound.push_back(encode::cancel_target(cursor.id));
                inner.world.write().clear_target();
                inner.target_intent = None;
                ToolResult::action(TOOL_TARGET)
            } else {
                inner.target_intent = None;
                ToolResult::err("must have a target cursor")
            }
        }
        TOOL_WAIT_TARGET => {
            ToolResult::ok(json!({ "pending": inner.world.read().pending_target.is_some() }))
        }
        TOOL_GUMP_RESPOND | TOOL_GUMP_CLOSE => {
            let gump = inner.world.read().gumps.first().cloned();
            if let Some(g) = gump {
                let button = if call.name == TOOL_GUMP_CLOSE {
                    0
                } else {
                    args.get("button").and_then(|v| v.as_u64()).unwrap_or(0) as u32
                };
                let switches = if call.name == TOOL_GUMP_CLOSE {
                    Vec::new()
                } else {
                    args.get("switches")
                        .and_then(|v| v.as_array())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|v| v.as_u64().map(|n| n as u32))
                                .collect()
                        })
                        .unwrap_or_default()
                };
                inner.outbound.push_back(encode::gump_response(
                    g.serial, g.gump_id, button, &switches,
                ));
                inner.world.write().close_gump(g.gump_id);
                ToolResult::action(if call.name == TOOL_GUMP_CLOSE {
                    TOOL_GUMP_CLOSE
                } else {
                    TOOL_GUMP_RESPOND
                })
            } else {
                ToolResult::err("no open gump")
            }
        }
        TOOL_SET_GOAL => {
            let raw = args
                .get("goal")
                .and_then(|v| v.as_str())
                .unwrap_or(Goal::Idle.name());
            let mut g = Goal::parse(raw);
            if let Goal::Travel { dest } = &mut g {
                if let (Some(x), Some(y)) = (
                    args.get("x").and_then(|v| v.as_u64()),
                    args.get("y").and_then(|v| v.as_u64()),
                ) {
                    dest.x = x as u16;
                    dest.y = y as u16;
                    dest.z = asked_z(args, inner.world.read().self_state.location.z);
                }
            }
            inner.follow = None;
            inner.goal = g.clone();
            inner.world.write().goal = g.name().into();
            if let Goal::Travel { dest } = g {
                let _ = queue_move(inner, dest);
            }
            ToolResult::ok(json!({ "goal": inner.goal.name() }))
        }
        TOOL_FOLLOW => {
            let serial = arg_serial(args, "serial");
            if !serial.is_valid() {
                return ToolResult::err("follow needs a mobile serial");
            }
            inner.follow = Some(serial);
            inner.follow_state = FollowState::default();
            // The followed mobile leads, so the session holds no travel
            // destination of its own to walk back to.
            inner.goal = Goal::Idle;
            inner.world.write().goal = inner.goal.name().into();
            inner.movement.hold();
            ToolResult::action(TOOL_FOLLOW)
        }
        TOOL_UNEQUIP => {
            let Some(layer) = args.get("layer").and_then(|v| v.as_u64()) else {
                return ToolResult::err("unequip needs layer");
            };
            let layer = layer as u8;
            if layer == 0 || layer == LAYER_BACKPACK || layer > LAYER_BANK {
                return ToolResult::err("invalid layer");
            }
            let (item, pack) = {
                let w = inner.world.read();
                let item = w
                    .self_state
                    .equipment
                    .iter()
                    .find(|e| e.layer == layer)
                    .cloned();
                let pack = w
                    .self_state
                    .equipment
                    .iter()
                    .find(|e| e.layer == LAYER_BACKPACK)
                    .map(|e| e.serial)
                    .unwrap_or(w.self_state.serial);
                (item, pack)
            };
            match item {
                Some(eq) => {
                    let grid = drop_grid(inner);
                    inner
                        .outbound
                        .push_back(encode::drop_into_container(eq.serial, pack, grid));
                    ToolResult::action(TOOL_UNEQUIP)
                }
                None => ToolResult::err("layer empty"),
            }
        }
        TOOL_TRADE_OFFER => {
            let serial = arg_serial(args, "serial");
            inner.outbound.push_back(encode::trade_start(serial));
            ToolResult::action(TOOL_TRADE_OFFER)
        }
        TOOL_SET_PERSONA => match serde_json::from_value::<Persona>(args.clone()) {
            Ok(mut p) => {
                p.clamp_rates();
                inner.persona = p;
                ToolResult::ok(json!({ "persona": inner.persona.name }))
            }
            Err(e) => ToolResult::err(format!("persona parse: {e}")),
        },
        other => ToolResult::err(format!("unknown tool {other}")),
    }
}

/// The radar an agent reads, drawn from the floor the character stands on.
///
/// A building with more than one floor holds one answer for each floor at the
/// same x,y. Asked with no height, such a tile answers the floor above, and the
/// agent then reads the layout of a room it is not standing in.
fn radar_from_floor(world: &World, map: &dyn TileQuery, from_z: i8) -> uoterm_world::Observe {
    world.observe(|x, y| map.tile_from(from_z, x, y).radar_char())
}

fn observe_value(inner: &Inner) -> Value {
    let w = inner.world.read();
    let idx = w.self_state.map;
    let loc = w.self_state.location;
    let mut obs = match inner.maps.get(&idx) {
        Some(mul) => radar_from_floor(&w, mul.as_ref(), loc.z),
        None => radar_from_floor(&w, &inner.map, loc.z),
    };
    drop(w);
    let mut doors: Vec<uoterm_world::NearbyDoor> = Vec::new();
    if let Some(mul) = inner.maps.get(&idx) {
        for d in mul.doors_near(loc.x, loc.y, uoterm_world::OBSERVE_DOOR_RADIUS) {
            doors.push(uoterm_world::NearbyDoor {
                x: d.x,
                y: d.y,
                z: d.z,
                dx: d.dx,
                dy: d.dy,
                dist: d.dx.unsigned_abs() + d.dy.unsigned_abs(),
                source: "map".into(),
                serial: None,
                graphic: Some(d.graphic),
            });
        }
        let w = inner.world.read();
        for item in w.nearby_items(uoterm_world::OBSERVE_DOOR_RADIUS) {
            if !mul.is_door_graphic(item.graphic) {
                continue;
            }
            let dx = i32::from(item.location.x) - i32::from(loc.x);
            let dy = i32::from(item.location.y) - i32::from(loc.y);
            doors.push(uoterm_world::NearbyDoor {
                x: item.location.x,
                y: item.location.y,
                z: item.location.z,
                dx,
                dy,
                dist: loc.chebyshev(item.location),
                source: "item".into(),
                serial: Some(item.serial.to_string()),
                graphic: Some(item.graphic),
            });
        }
    } else {
        let r = i32::from(uoterm_world::OBSERVE_DOOR_RADIUS);
        for dy in -r..=r {
            for dx in -r..=r {
                let x = i32::from(loc.x) + dx;
                let y = i32::from(loc.y) + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let t = inner.map.tile(x as u16, y as u16);
                if !t.door {
                    continue;
                }
                doors.push(uoterm_world::NearbyDoor {
                    x: x as u16,
                    y: y as u16,
                    z: t.z,
                    dx,
                    dy,
                    dist: dx.unsigned_abs() + dy.unsigned_abs(),
                    source: "map".into(),
                    serial: None,
                    graphic: None,
                });
            }
        }
    }
    doors.sort_by_key(|d| d.dist);
    if let Some(d) = doors.first() {
        obs.caption.push_str(&format!(
            ". nearest door {} at {},{} z{} (dx={} dy={})",
            d.source, d.x, d.y, d.z, d.dx, d.dy
        ));
    } else {
        obs.caption
            .push_str(". no door in range (map TILE_DOOR or item with door flag)");
    }
    obs.doors = doors;
    serde_json::to_value(obs).unwrap_or(Value::Null)
}

fn speech_allowed(
    speech: &mut SpeechPolicy,
    persona: &Persona,
    text: &str,
    kind: u8,
) -> std::result::Result<String, &'static str> {
    let t = persona.filter_speech(text).ok_or(SPEECH_REJECTED)?;
    if kind == SPEECH_REGULAR && !speech.allow(persona) {
        return Err(SPEECH_RATE_LIMITED);
    }
    Ok(t)
}

fn queue_speech(inner: &mut Inner, text: &str, kind: u8) -> std::result::Result<(), &'static str> {
    let t = speech_allowed(&mut inner.speech, &inner.persona, text, kind)?;
    let mut rng = rand::thread_rng();
    let t = inner.persona.maybe_typo(&t, &mut rng);
    let unicode = inner.era == Era::Modern;
    let pkt = match kind {
        SPEECH_WHISPER => encode::whisper(&t, unicode),
        SPEECH_EMOTE => encode::emote(&t, unicode),
        _ => encode::say(&t, false),
    };
    inner.outbound.push_back(pkt);
    Ok(())
}

fn speak(inner: &mut Inner, args: &Value, kind: u8) -> ToolResult {
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    match queue_speech(inner, text, kind) {
        Ok(()) => ToolResult::action(TOOL_SAY),
        Err(e) => ToolResult::err(e),
    }
}

/// The floor a map question is about: the height the caller names in `z`, or
/// `standing_z`, the height the character is at. A building with more than one
/// floor gives a different answer for each floor, so a caller that names no
/// height must get its own floor and not the one above.
fn asked_z(args: &Value, standing_z: i8) -> i8 {
    args.get("z")
        .and_then(|v| v.as_i64())
        .map(|n| n as i8)
        .unwrap_or(standing_z)
}

fn arg_u32(args: &Value, key: &str, default: u32) -> u32 {
    let Some(v) = args.get(key) else {
        return default;
    };
    if let Some(n) = v.as_u64() {
        return n as u32;
    }
    if let Some(n) = v.as_i64() {
        return n.max(0) as u32;
    }
    if let Some(s) = v.as_str() {
        let text = s.trim();
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            return u32::from_str_radix(hex, 16).unwrap_or(default);
        }
        return text.parse().unwrap_or(default);
    }
    default
}

fn arg_serial(args: &Value, key: &str) -> Serial {
    Serial(arg_u32(args, key, 0))
}

/// A serial argument, or `None` when the caller named no object at all.
///
/// A filter needs that difference: no serial means "look everywhere", while a
/// serial nothing answers to means "look in that one thing" and finds nothing.
fn arg_serial_opt(args: &Value, key: &str) -> Option<Serial> {
    match args.get(key) {
        None | Some(Value::Null) => None,
        Some(_) => Some(arg_serial(args, key)),
    }
}
