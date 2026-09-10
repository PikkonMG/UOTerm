use serde::{Deserialize, Serialize};
use uoterm_protocol::Direction;

use crate::addressed::SpokenTo;
use crate::events::unix_now_ms;
use crate::radar::legend;
use crate::state::{Item, Mobile, SelfState, World};

pub const OBSERVE_MOBILE_CAP: usize = 24;
pub const OBSERVE_ITEM_CAP: usize = 24;
pub const OBSERVE_FACT_CAP: usize = 8;
pub const OBSERVE_DOOR_RADIUS: u16 = 12;

/// How many containers one observation describes, nearest first.
///
/// A character works out of his own pack and out of one thing he has just
/// opened, so a short list holds everything he is really using. Every other
/// container he has open stays in the world model and a search still reaches
/// it.
pub const OBSERVE_CONTAINER_CAP: usize = 4;

/// How many items one described container lists.
///
/// A bank box holds far more than an agent can read in one look, so a long
/// container is cut here and [`OpenContainer::total`] says how many it really
/// holds. The ground scene is cut at [`OBSERVE_ITEM_CAP`] for the same reason,
/// and one container is worth as much of the view as the ground is.
pub const OBSERVE_CONTAINER_ITEM_CAP: usize = OBSERVE_ITEM_CAP;

/// Where a container sits when its own item record has not arrived yet. Such a
/// container sorts behind every container whose distance is known.
const CONTAINER_DIST_UNKNOWN: u32 = u32::MAX;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NearbyDoor {
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub dx: i32,
    pub dy: i32,
    pub dist: u32,
    pub source: String,
    pub serial: Option<String>,
    pub graphic: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NearbyItem {
    pub serial: String,
    pub graphic: u16,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub dx: i32,
    pub dy: i32,
    pub dist: u32,
    pub amount: u16,
    pub name: String,
}

/// One item inside a container.
///
/// It has no place on the map, so it carries no tile: what an agent needs to
/// act on it is its serial, what it looks like, how many there are, and what
/// it is called.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainedItem {
    pub serial: String,
    pub graphic: u16,
    pub amount: u16,
    pub hue: u16,
    pub name: String,
}

impl From<&Item> for ContainedItem {
    fn from(item: &Item) -> Self {
        Self {
            serial: item.serial.to_string(),
            graphic: item.graphic,
            amount: item.amount,
            hue: item.hue,
            name: item.name.clone(),
        }
    }
}

/// A container the character has opened, and what it holds.
///
/// The contents hang under the container instead of joining the ground scene,
/// because a corpse and a bank box would otherwise push every tree and every
/// person out of a view that is already cut short.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenContainer {
    pub serial: String,
    pub name: String,
    /// What the container itself looks like, once its own item record has
    /// arrived.
    pub graphic: Option<u16>,
    /// How far the container is, once its own item record has arrived. A pack
    /// the character wears is at his own tile, so it reads zero.
    pub dist: Option<u32>,
    /// How many items the container holds. This counts them all, even the ones
    /// past [`OBSERVE_CONTAINER_ITEM_CAP`] that `contents` does not list.
    pub total: usize,
    /// The first [`OBSERVE_CONTAINER_ITEM_CAP`] items in it.
    pub contents: Vec<ContainedItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observe {
    pub self_state: SelfState,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub facing: String,
    pub radar: String,
    pub legend: String,
    pub caption: String,
    pub journal: Vec<String>,
    pub mobiles: Vec<Mobile>,
    pub items: Vec<Item>,
    pub nearby_items: Vec<NearbyItem>,
    /// What the character can see inside the containers he has opened, his own
    /// pack among them. `items` and `nearby_items` are the ground scene and
    /// hold none of this.
    pub containers: Vec<OpenContainer>,
    pub doors: Vec<NearbyDoor>,
    pub pending_target: bool,
    pub open_gumps: usize,
    pub holding: Option<String>,
    pub combatant: Option<String>,
    pub goal: String,
    pub facts: Vec<String>,
    /// The assistant features the shard forbids and the character obeys. It
    /// does not use them by itself. Empty when the user set the character to
    /// ignore the shard's list.
    pub forbidden: Vec<String>,
    /// The names of the buffs and debuffs on the character. The runtime
    /// fills this in, because the names live in the client text files.
    pub buffs: Vec<String>,
    /// The party members by name. Empty outside a party.
    pub party: Vec<String>,
    /// Who asked the character to join a party, while the ask is open.
    pub party_invite: Option<String>,
    /// The shard waits for a line of text.
    pub prompt: bool,
    /// The question of an open one-field text dialog.
    pub text_entry: Option<String>,
    /// Lines other characters said to this one by name in the last minute,
    /// oldest first. Empty when the `answer_when_named` switch is off.
    pub spoken_to: Vec<SpokenTo>,
    /// `basic` or `play_along`: what the agent may agree to in chat. None
    /// when the `answer_when_named` switch is off.
    pub chat_mode: Option<String>,
    /// How the character talks, from the persona, for the agent that writes
    /// its lines. The runtime fills this in.
    pub reply_style: Option<String>,
    /// The player the character plays along with now. The runtime fills
    /// this in.
    pub playing_along: Option<PlayingAlong>,
}

/// The player the character plays along with, and how long it has left.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayingAlong {
    pub name: String,
    pub serial: String,
    pub minutes_left: u64,
}

impl Observe {
    pub fn from_world(world: &World, radar: String) -> Self {
        let s = &world.self_state;
        let facing = Direction::from_byte(s.direction).name();
        let caption = format!(
            "{} at {},{},{} facing {} hp {}/{} mana {}/{} stam {}/{} war={} dead={}",
            s.name,
            s.location.x,
            s.location.y,
            s.location.z,
            facing,
            s.hits,
            s.hits_max,
            s.mana,
            s.mana_max,
            s.stam,
            s.stam_max,
            s.war,
            s.dead
        );
        let facts = world
            .events
            .iter()
            .rev()
            .take(OBSERVE_FACT_CAP)
            .map(|e| e.fact_line())
            .collect();
        let mut mobiles: Vec<Mobile> = world.mobiles.values().cloned().collect();
        mobiles.sort_by_key(|m| s.location.chebyshev(m.location));
        mobiles.truncate(OBSERVE_MOBILE_CAP);
        let mut items: Vec<Item> = world
            .items
            .values()
            .filter(|i| i.parent.is_none())
            .cloned()
            .collect();
        items.sort_by_key(|i| s.location.chebyshev(i.location));
        items.truncate(OBSERVE_ITEM_CAP);
        let nearby_items = items
            .iter()
            .map(|i| {
                let dx = i32::from(i.location.x) - i32::from(s.location.x);
                let dy = i32::from(i.location.y) - i32::from(s.location.y);
                NearbyItem {
                    serial: i.serial.to_string(),
                    graphic: i.graphic,
                    x: i.location.x,
                    y: i.location.y,
                    z: i.location.z,
                    dx,
                    dy,
                    dist: s.location.chebyshev(i.location),
                    amount: i.amount,
                    name: i.name.clone(),
                }
            })
            .collect();
        Self {
            self_state: s.clone(),
            x: s.location.x,
            y: s.location.y,
            z: s.location.z,
            facing: facing.into(),
            radar,
            legend: legend(),
            caption,
            journal: world.journal.recent_text(),
            mobiles,
            items,
            nearby_items,
            containers: open_containers(world),
            doors: Vec::new(),
            pending_target: world.pending_target.is_some(),
            open_gumps: world.gumps.len(),
            holding: world.holding.map(|serial| serial.to_string()),
            combatant: world.combatant.map(|serial| serial.to_string()),
            goal: world.goal.clone(),
            facts,
            forbidden: world
                .assist
                .forbidden()
                .into_iter()
                .map(String::from)
                .collect(),
            buffs: Vec::new(),
            party: world.party.iter().map(|&m| world.name_of(m)).collect(),
            party_invite: world.party_invite.map(|leader| world.name_of(leader)),
            prompt: world.prompt.is_some(),
            text_entry: world.text_entry.as_ref().map(|d| d.description.clone()),
            spoken_to: world.spoken_to.fresh(unix_now_ms()),
            chat_mode: world.chat_mode().map(String::from),
            reply_style: None,
            playing_along: None,
        }
    }
}

/// Every container the character has opened, nearest first, cut to
/// [`OBSERVE_CONTAINER_CAP`].
///
/// A container the world knows of but holds no item record for keeps its
/// place in the list: the character opened it, so he can still act on it.
fn open_containers(world: &World) -> Vec<OpenContainer> {
    let here = world.self_state.location;
    let mut open: Vec<OpenContainer> = world
        .containers
        .values()
        .map(|container| {
            let held: Vec<&Item> = container
                .items
                .iter()
                .filter_map(|serial| world.items.get(serial))
                .collect();
            let record = world.items.get(&container.serial);
            OpenContainer {
                serial: container.serial.to_string(),
                name: record.map(|i| i.name.clone()).unwrap_or_default(),
                graphic: record.map(|i| i.graphic),
                // A bag in another bag or on a mobile is where its holder is.
                dist: world
                    .map_location(container.serial)
                    .map(|at| here.chebyshev(at)),
                total: held.len(),
                contents: held
                    .iter()
                    .take(OBSERVE_CONTAINER_ITEM_CAP)
                    .map(|item| ContainedItem::from(*item))
                    .collect(),
            }
        })
        .collect();
    open.sort_by(|a, b| {
        a.dist
            .unwrap_or(CONTAINER_DIST_UNKNOWN)
            .cmp(&b.dist.unwrap_or(CONTAINER_DIST_UNKNOWN))
            .then_with(|| a.serial.cmp(&b.serial))
    });
    open.truncate(OBSERVE_CONTAINER_CAP);
    open
}
