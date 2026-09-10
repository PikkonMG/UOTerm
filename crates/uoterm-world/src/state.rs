use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use uoterm_protocol::{
    weapon_range, BuffEntry, ContainerItem, EquipItem, GroundItem, HealthBarStatus, Inbound,
    MobileView, ObjectProperty, OpenGump, PartyEvent, Point3, PromptRequest, Serial, StatusExtra,
    TargetCursor, TextEntryDialog, DIR_RUNNING, FLAG_BLESSED, FLAG_FROZEN, FLAG_HIDDEN,
    FLAG_POISONED, FLAG_WAR, HEALTH_BAR_POISON, HEALTH_BAR_YELLOW, LAYER_BANK, LAYER_ONE_HANDED,
    LAYER_TWO_HANDED, RANGE_MELEE,
};

use crate::assist::AssistRules;
use crate::events::{Event, EventKind, EVENT_LOG_CAP};
use crate::journal::{Journal, JournalEntry};
use crate::names::{display_name, NameBook};
use crate::observe::Observe;
use crate::radar::{default_tile, render_radar, RadarOptions, TileKind, RADAR_DEFAULT};

pub const BODY_GHOST_MALE: u16 = 0x192;
pub const BODY_GHOST_FEMALE: u16 = 0x193;
pub const BODY_GHOST_ELF_MALE: u16 = 0x25F;
pub const BODY_GHOST_ELF_FEMALE: u16 = 0x260;

/// Anyone may move over anyone else on a facet that carries this rule.
///
/// The server map rules name this bit free movement and give it `0x0002`.
/// Both server families use the same bit with the same meaning.
pub const MAP_RULE_FREE_MOVEMENT: u16 = 0x0002;
/// The Felucca rule set: a facet with none of the rules.
pub const FACET_RULES_FELUCCA: u16 = 0x0000;
/// The Trammel rule set: free movement, beneficial restrictions and harmful
/// restrictions, which is `0x0002 | 0x0004 | 0x0008`.
pub const FACET_RULES_TRAMMEL: u16 = 0x000E;

/// The rules of each facet, by the map index the shard puts the character on.
///
/// Read from the server map definitions: index 0 is the Felucca rule set and
/// indexes 1 to 5 are the Trammel rule set. Both server families ship the same
/// table.
const FACET_RULES: [u16; 6] = [
    FACET_RULES_FELUCCA,
    FACET_RULES_TRAMMEL,
    FACET_RULES_TRAMMEL,
    FACET_RULES_TRAMMEL,
    FACET_RULES_TRAMMEL,
    FACET_RULES_TRAMMEL,
];

/// The rules of the facet a map index names.
///
/// A shard may register facets of its own past the six the client files hold,
/// and a Siege shard registers all six under the Felucca rule set. Neither is
/// on the wire, so an index this table does not hold reads as Felucca: the
/// stricter of the two, which costs a longer route and never a refused step.
pub fn facet_rules(map_index: u8) -> u16 {
    FACET_RULES
        .get(map_index as usize)
        .copied()
        .unwrap_or(FACET_RULES_FELUCCA)
}

/// True while a facet lets anyone move over anyone else.
pub fn facet_free_movement(map_index: u8) -> bool {
    facet_rules(map_index) & MAP_RULE_FREE_MOVEMENT != 0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfState {
    pub serial: Serial,
    pub name: String,
    pub body: u16,
    pub hue: u16,
    pub location: Point3,
    pub map: u8,
    pub direction: u8,
    pub war: bool,
    pub hits: u16,
    pub hits_max: u16,
    pub mana: u16,
    pub mana_max: u16,
    pub stam: u16,
    pub stam_max: u16,
    pub str_: u16,
    pub dex: u16,
    pub int_: u16,
    pub notoriety: u8,
    pub weight: u16,
    pub weight_max: u16,
    pub gold: u32,
    pub flags: u8,
    pub hidden: bool,
    pub poisoned: bool,
    pub paralyzed: bool,
    /// A gargoyle in the air.
    pub flying: bool,
    /// The yellow health bar of a blessed or invulnerable mobile.
    pub yellow_bar: bool,
    /// Steps taken since the character hid, for a stealth walk.
    pub stealth_steps: u16,
    pub dead: bool,
    pub skills: HashMap<u16, SkillValue>,
    pub equipment: Vec<EquipItem>,
    /// Resists, luck, followers, tithing and the rest of the status.
    pub status: StatusExtra,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SkillValue {
    pub value: u16,
    pub base: u16,
    pub cap: u16,
    pub lock: u8,
}

impl Default for SelfState {
    fn default() -> Self {
        Self {
            serial: Serial::INVALID,
            name: String::new(),
            body: 0,
            hue: 0,
            location: Point3::new(0, 0, 0),
            map: 0,
            direction: 0,
            war: false,
            hits: 0,
            hits_max: 0,
            mana: 0,
            mana_max: 0,
            stam: 0,
            stam_max: 0,
            str_: 0,
            dex: 0,
            int_: 0,
            notoriety: 0,
            weight: 0,
            weight_max: 0,
            gold: 0,
            flags: 0,
            hidden: false,
            poisoned: false,
            paralyzed: false,
            flying: false,
            yellow_bar: false,
            stealth_steps: 0,
            dead: false,
            skills: HashMap::new(),
            equipment: Vec::new(),
            status: StatusExtra::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mobile {
    pub serial: Serial,
    pub name: String,
    pub body: u16,
    pub hue: u16,
    pub location: Point3,
    pub direction: u8,
    pub running: bool,
    pub notoriety: u8,
    pub flags: u8,
    pub hits: Option<u16>,
    pub hits_max: Option<u16>,
    pub equipment: Vec<EquipItem>,
}

impl From<&MobileView> for Mobile {
    fn from(v: &MobileView) -> Self {
        Self {
            serial: v.serial,
            name: String::new(),
            body: v.body,
            hue: v.hue,
            location: Point3::new(v.x, v.y, v.z),
            direction: v.direction & 0x07,
            running: v.direction & DIR_RUNNING != 0,
            notoriety: v.notoriety,
            flags: v.flags,
            hits: v.hits,
            hits_max: v.hits_max,
            equipment: v.equipment.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub serial: Serial,
    pub graphic: u16,
    pub amount: u16,
    pub hue: u16,
    pub location: Point3,
    pub parent: Option<Serial>,
    pub layer: Option<u8>,
    pub grid: u8,
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Container {
    pub serial: Serial,
    pub gump: u16,
    pub items: Vec<Serial>,
}

/// A door the server sent as an item.
///
/// Only such a door can be opened: it has a serial to click, and the server
/// answers that click by sending the same serial again with the graphic and
/// the tile the leaf now has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorItem {
    pub serial: Serial,
    pub graphic: u16,
    pub location: Point3,
}

/// What an item packet did to the record of one door.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoorUpdate {
    /// A door the world had not seen before.
    Learned,
    /// The same door, in the state the record already held.
    Unchanged,
    /// The leaf swung: a new graphic, a new tile, or both. Which of the two
    /// states the door is now in cannot be read from the graphic, because door
    /// graphics come in open and shut pairs that differ from one door to the
    /// next. That the door answered at all is what this says.
    Swung,
}

/// A player house or a boat the server sent as an item.
///
/// The shape of the building is in the client files and not in this record:
/// the item says which multi it is and where that multi stands, and everything
/// a route needs to know about its walls follows from those two facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiItem {
    pub serial: Serial,
    /// Which shape in the client multi files the building has.
    pub multi_id: u16,
    /// Where the multi item itself stands. Every piece of the building sits at
    /// an offset from this point.
    pub location: Point3,
}

/// Who last harmed the character, and when.
///
/// Only the swing packet names an attacker: the packets that take health away
/// name the one who loses it and nobody else. So the swing is what writes the
/// name here, and every wound after it moves the time on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Harm {
    pub by: Serial,
    pub at: Instant,
}

/// What an item packet did to the record of one building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiUpdate {
    /// A building the world had not seen before.
    Learned,
    /// The same building, in the state the record already held.
    Unchanged,
    /// It stands somewhere else now, or the serial carries another building.
    /// A boat sails, and every route planned around where it was is a route
    /// around a building that is no longer there.
    Moved,
}

/// Journal kinds for party lines. Party chat comes in its own packet, not as
/// speech, so these numbers never come from the wire; they sit above every
/// speech kind a shard sends.
pub const SPEECH_KIND_PARTY: u8 = 0xF0;
pub const SPEECH_KIND_PARTY_PRIVATE: u8 = 0xF1;
/// The hue party lines are filed with. The shard sends none.
const PARTY_HUE: u16 = 0;

/// The deepest a bag inside a bag is followed. A loop in a broken parent
/// chain stops here.
const MAX_NESTING: usize = 16;

/// The bit a shard sets for poison on an old client and for flying on a
/// client from 7.0.0.0 up.
const FLAG_POISONED_OR_FLYING: u8 = FLAG_POISONED;

/// A party list this short is no party.
const PARTY_OF_ONE: usize = 1;

/// The health bar colours of one mobile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarState {
    pub poisoned: bool,
    pub yellow: bool,
}

/// One buff or debuff on the character.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Buff {
    pub icon: u16,
    /// The client text numbers of its name and its description.
    pub title_cliloc: u32,
    pub description_cliloc: u32,
    pub arguments: String,
    /// How long it lasts, as the shard said when it began. Zero when it lasts
    /// until something ends it.
    pub duration_secs: u16,
    #[serde(skip, default = "Instant::now")]
    pub since: Instant,
}

impl Buff {
    fn new(effect: &BuffEntry) -> Self {
        Self {
            icon: effect.icon,
            title_cliloc: effect.title_cliloc,
            description_cliloc: effect.description_cliloc,
            arguments: effect.arguments.clone(),
            duration_secs: effect.duration_secs,
            since: Instant::now(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct World {
    pub self_state: SelfState,
    pub mobiles: HashMap<Serial, Mobile>,
    pub items: HashMap<Serial, Item>,
    pub containers: HashMap<Serial, Container>,
    /// The doors the character can see, by serial. A door moves and changes
    /// graphic while it swings, and the server reports both on the same serial.
    pub doors: HashMap<Serial, DoorItem>,
    /// The player houses and boats the character can see, by serial. A house
    /// stands still and a boat sails, and the server reports both as an
    /// ordinary item on that serial.
    pub multis: HashMap<Serial, MultiItem>,
    pub journal: Journal,
    pub events: Vec<Event>,
    pub event_seq: u64,
    pub pending_target: Option<TargetCursor>,
    pub gumps: Vec<OpenGump>,
    pub holding: Option<Serial>,
    /// The serial the server has us fighting, or none when the fight has ended.
    pub combatant: Option<Serial>,
    /// When the server last sent a swing that named us as the attacker.
    #[serde(skip)]
    pub last_swing: Option<Instant>,
    /// The gap between the last two of those swings.
    #[serde(skip)]
    pub last_swing_gap: Option<Duration>,
    /// Who last harmed the character, and when. See [`Harm`].
    #[serde(skip)]
    pub harmed_by: Option<Harm>,
    pub logged_in: bool,
    pub goal: String,
    pub nav_goal: Option<Point3>,
    pub path_len: usize,
    pub map_width: u16,
    pub map_height: u16,
    pub season: u8,
    pub names: NameBook,
    /// The property list of each object the shard described, as text numbers
    /// and their arguments. The runtime turns them into words.
    pub properties: HashMap<Serial, Vec<ObjectProperty>>,
    /// The buffs and debuffs on the character, by icon.
    pub buffs: HashMap<u16, Buff>,
    /// A text prompt the shard is waiting on.
    pub prompt: Option<PromptRequest>,
    /// A one-field text dialog the shard is waiting on.
    pub text_entry: Option<TextEntryDialog>,
    /// Flag bit 4 means "flying", and poison comes on the health bar packet.
    /// Clients from 7.0.0.0 up read the bits this way; older clients read bit
    /// 4 as poison. The session sets this from the client version.
    pub flags_mean_flying: bool,
    /// The poison and yellow health bars of other mobiles, as the health bar
    /// packet last set them.
    pub bars: HashMap<Serial, BarState>,
    /// The party members, the character among them. Empty outside a party.
    pub party: Vec<Serial>,
    /// The leader of a party the character was asked to join.
    pub party_invite: Option<Serial>,
    /// The assistant features the shard forbids, when the user lets the
    /// shard decide. The session fills it in.
    pub assist: AssistRules,
}

impl World {
    pub fn new() -> Self {
        Self {
            goal: "idle".into(),
            ..Self::default()
        }
    }

    pub fn push_event(&mut self, mut event: Event) {
        self.event_seq = self.event_seq.saturating_add(1);
        event.seq = self.event_seq;
        self.events.push(event);
        if self.events.len() > EVENT_LOG_CAP {
            let extra = self.events.len() - EVENT_LOG_CAP;
            self.events.drain(..extra);
        }
    }

    fn mark_logged_in(&mut self) {
        if self.logged_in {
            return;
        }
        self.logged_in = true;
        self.push_event(Event::new(
            EventKind::LoggedIn,
            Some(self.self_state.serial),
            self.self_state.name.clone(),
        ));
    }

    pub fn apply(&mut self, msg: &Inbound) {
        match msg {
            Inbound::LoginConfirm {
                serial,
                body,
                x,
                y,
                z,
                direction,
                map_width,
                map_height,
            } => {
                self.self_state.serial = *serial;
                self.self_state.body = *body;
                self.self_state.location = Point3::new(*x, *y, *z);
                self.self_state.direction = *direction;
                self.self_state.dead = false;
                self.map_width = *map_width;
                self.map_height = *map_height;
                self.mark_logged_in();
            }
            Inbound::LoginComplete => {
                self.mark_logged_in();
            }
            Inbound::DrawPlayer {
                serial,
                body,
                hue,
                flags,
                x,
                y,
                direction,
                z,
            } => {
                if *serial == self.self_state.serial || self.self_state.serial.0 == 0 {
                    self.self_state.serial = *serial;
                    self.self_state.body = *body;
                    self.self_state.hue = *hue;
                    self.self_state.location = Point3::new(*x, *y, *z);
                    self.self_state.direction = *direction;
                    self.apply_self_flags(*flags);
                    self.refresh_dead_from_body(*body);
                } else {
                    self.apply_mobile_view(&MobileView {
                        serial: *serial,
                        body: *body,
                        x: *x,
                        y: *y,
                        z: *z,
                        direction: *direction,
                        hue: *hue,
                        flags: *flags,
                        notoriety: 0,
                        hits: None,
                        hits_max: None,
                        equipment: Vec::new(),
                    });
                }
            }
            Inbound::MapChange { map } => {
                self.self_state.map = *map;
            }
            Inbound::Damage { serial, amount } => {
                if *serial == self.self_state.serial {
                    self.self_state.hits = self.self_state.hits.saturating_sub(*amount);
                    self.note_harm_now();
                }
                self.push_event(Event::new(
                    EventKind::Damaged,
                    Some(*serial),
                    format!("{amount}"),
                ));
            }
            // The server refused a step and says where the character really
            // stands. That word is final: it snaps him back.
            Inbound::MoveReject {
                x, y, direction, z, ..
            } => {
                self.self_state.location = Point3::new(*x, *y, *z);
                self.self_state.direction = *direction;
            }
            // The server confirmed a step, but this packet carries no tile:
            // only the sequence number of the step it answers. The session
            // holds that step and moves the character, because it alone knows
            // which tile the step was aimed at.
            Inbound::MoveAck { notoriety, .. } => {
                self.self_state.notoriety = *notoriety;
                if self.self_state.hidden {
                    self.self_state.stealth_steps = self.self_state.stealth_steps.saturating_add(1);
                }
            }
            Inbound::Speech(line) => {
                if line.serial == self.self_state.serial && self.self_state.name.is_empty() {
                    self.self_state.name.clone_from(&line.name);
                }
                if let Some(mob) = self.mobiles.get_mut(&line.serial) {
                    if !line.name.is_empty() {
                        mob.name.clone_from(&line.name);
                    }
                }
                self.journal.push(JournalEntry::from(line));
                self.push_event(Event::new(
                    EventKind::Speech,
                    Some(line.serial),
                    format!("{}: {}", line.name, line.text),
                ));
            }
            Inbound::Delete(serial) => {
                self.detach(*serial);
                self.names.forget(*serial);
                self.mobiles.remove(serial);
                self.items.remove(serial);
                self.containers.remove(serial);
                self.doors.remove(serial);
                self.multis.remove(serial);
                self.self_state.equipment.retain(|e| e.serial != *serial);
                self.bars.remove(serial);
                self.properties.remove(serial);
                // The held item is left held. Lifting takes an item off the
                // map and the shard reports that with a delete, so a delete of
                // the item on the cursor is the lift itself. The hold ends when
                // the item lands (an add into a container, or a lift refused).
            }
            Inbound::MobileMoving(view) | Inbound::MobileIncoming(view) => {
                self.apply_mobile_view(view);
            }
            Inbound::WorldItem(item) => {
                self.upsert_ground(item);
            }
            Inbound::AddItem(item) => {
                self.upsert_container_item(item);
                if self.holding == Some(item.serial) {
                    self.holding = None;
                }
                self.push_event(Event::new(
                    EventKind::ItemAdded,
                    Some(item.serial),
                    format!("0x{:04X}", item.graphic),
                ));
            }
            Inbound::OpenContainer { serial, gump } => {
                self.containers.entry(*serial).or_insert(Container {
                    serial: *serial,
                    gump: *gump,
                    items: Vec::new(),
                });
                self.push_event(Event::new(
                    EventKind::ContainerOpened,
                    Some(*serial),
                    format!("gump {gump}"),
                ));
            }
            Inbound::ContainerContents { items } => {
                for item in items {
                    self.upsert_container_item(item);
                }
            }
            Inbound::UpdateHealth {
                serial,
                current,
                max,
            } => {
                self.set_hits(*serial, *current, *max);
            }
            Inbound::UpdateMana {
                serial,
                current,
                max,
            } if *serial == self.self_state.serial => {
                self.self_state.mana = *current;
                self.self_state.mana_max = *max;
            }
            Inbound::UpdateStam {
                serial,
                current,
                max,
            } if *serial == self.self_state.serial => {
                self.self_state.stam = *current;
                self.self_state.stam_max = *max;
            }
            Inbound::Status {
                serial,
                name,
                hits,
                hits_max,
                female: _,
                str_,
                dex,
                int_,
                stam,
                stam_max,
                mana,
                mana_max,
                gold,
                weight,
                weight_max,
                extra,
            } => {
                if *serial == self.self_state.serial || self.self_state.serial.0 == 0 {
                    self.self_state.status.clone_from(extra);
                    self.self_state.serial = *serial;
                    if !name.is_empty() {
                        self.self_state.name.clone_from(name);
                    }
                    self.self_state.hits = *hits;
                    self.self_state.hits_max = *hits_max;
                    self.self_state.str_ = *str_;
                    self.self_state.dex = *dex;
                    self.self_state.int_ = *int_;
                    self.self_state.stam = *stam;
                    self.self_state.stam_max = *stam_max;
                    self.self_state.mana = *mana;
                    self.self_state.mana_max = *mana_max;
                    self.self_state.gold = *gold;
                    self.self_state.weight = *weight;
                    if let Some(max) = weight_max {
                        self.self_state.weight_max = *max;
                    }
                } else if let Some(mob) = self.mobiles.get_mut(serial) {
                    if !name.is_empty() {
                        mob.name.clone_from(name);
                    }
                    mob.hits = Some(*hits);
                    mob.hits_max = Some(*hits_max);
                }
            }
            Inbound::Skills { skills } => {
                for s in skills {
                    self.self_state.skills.insert(
                        s.id,
                        SkillValue {
                            value: s.value,
                            base: s.base,
                            cap: s.cap,
                            lock: s.lock,
                        },
                    );
                }
            }
            Inbound::WarMode(on) => {
                self.self_state.war = *on;
            }
            Inbound::Target(cursor) => {
                self.pending_target = Some(cursor.clone());
                self.push_event(Event::new(
                    EventKind::TargetRequested,
                    None,
                    format!("id {}", cursor.id),
                ));
            }
            Inbound::Gump(gump) => {
                self.gumps.retain(|g| g.gump_id != gump.gump_id);
                self.gumps.push(gump.clone());
                self.push_event(Event::new(
                    EventKind::GumpOpened,
                    Some(gump.serial),
                    gump.layout.clone(),
                ));
            }
            Inbound::Death { serial, corpse } => {
                if *serial == self.self_state.serial {
                    self.self_state.dead = true;
                }
                self.push_event(Event::new(
                    EventKind::Died,
                    Some(*serial),
                    format!("corpse {corpse}"),
                ));
            }
            Inbound::Equipped(eq) => {
                self.wear(eq);
            }
            Inbound::Season { season, .. } => {
                self.season = *season;
            }
            Inbound::Paperdoll {
                serial,
                text,
                flags,
                ..
            } => {
                let name = paperdoll_name(text);
                if *serial == self.self_state.serial {
                    if !name.is_empty() {
                        self.self_state.name = name.to_string();
                    }
                    self.apply_self_flags(*flags);
                } else if let Some(mob) = self.mobiles.get_mut(serial) {
                    if !name.is_empty() {
                        mob.name = name.to_string();
                    }
                    mob.flags = *flags;
                }
            }
            Inbound::Swing { attacker, .. } if *attacker == self.self_state.serial => {
                let now = Instant::now();
                if let Some(prev) = self.last_swing {
                    self.last_swing_gap = Some(now.saturating_duration_since(prev));
                }
                self.last_swing = Some(now);
            }
            // Somebody is swinging at the character. This is the only packet
            // that names the one doing it, so it is the only place the name
            // can be learned.
            Inbound::Swing {
                attacker, defender, ..
            } if *defender == self.self_state.serial => {
                self.harmed_by = Some(Harm {
                    by: *attacker,
                    at: Instant::now(),
                });
            }
            Inbound::CombatantChanged { serial } => {
                if serial.is_valid() {
                    self.combatant = Some(*serial);
                } else {
                    self.combatant = None;
                    self.last_swing = None;
                    self.last_swing_gap = None;
                }
                self.push_event(Event::new(
                    EventKind::CombatantChanged,
                    self.combatant,
                    if serial.is_valid() {
                        format!("{serial}")
                    } else {
                        "ended".into()
                    },
                ));
            }
            Inbound::LiftRejected { reason } => {
                self.holding = None;
                self.push_event(Event::new(
                    EventKind::LiftRejected,
                    None,
                    format!("{reason}"),
                ));
            }
            Inbound::OplInfo { serial, hash } => {
                self.names.note_revision(*serial, *hash);
            }
            Inbound::ObjectPropertyList {
                serial,
                hash,
                properties,
            } => {
                self.accept_properties(*serial, *hash, properties);
            }
            Inbound::BuffDebuff {
                serial,
                icon,
                effects,
            } if *serial == self.self_state.serial => match effects.first() {
                Some(effect) => {
                    self.buffs.insert(*icon, Buff::new(effect));
                }
                None => {
                    self.buffs.remove(icon);
                }
            },
            Inbound::HealthBarUpdate { serial, bars } => self.apply_bars(*serial, bars),
            Inbound::Prompt(prompt) => self.prompt = Some(*prompt),
            Inbound::TextEntry(dialog) => self.text_entry = Some(dialog.clone()),
            Inbound::Party(event) => self.apply_party(event),
            Inbound::Unknown { id, .. } => {
                tracing::debug!(packet = format!("{id:#04x}"), "unhandled inbound packet");
            }
            _ => {}
        }
    }

    fn apply_party(&mut self, event: &PartyEvent) {
        match event {
            // A list of one is the character alone: the party is over.
            PartyEvent::Members(members) | PartyEvent::Removed { members, .. }
                if members.len() <= PARTY_OF_ONE =>
            {
                self.party.clear();
            }
            PartyEvent::Members(members) => {
                self.party.clone_from(members);
                self.party_invite = None;
            }
            PartyEvent::Removed { members, .. } => self.party.clone_from(members),
            PartyEvent::Invite { leader } => {
                self.party_invite = Some(*leader);
                self.push_event(Event::new(
                    EventKind::PartyInvite,
                    Some(*leader),
                    self.name_of(*leader),
                ));
            }
            PartyEvent::Message {
                from,
                text,
                private,
            } => {
                self.journal.push(JournalEntry {
                    serial: *from,
                    name: self.name_of(*from),
                    hue: PARTY_HUE,
                    kind: if *private {
                        SPEECH_KIND_PARTY_PRIVATE
                    } else {
                        SPEECH_KIND_PARTY
                    },
                    text: text.clone(),
                    at: Instant::now(),
                    seq: 0,
                });
            }
        }
    }

    /// The name the world knows a mobile by, or its serial when it has none.
    pub fn name_of(&self, serial: Serial) -> String {
        if serial == self.self_state.serial {
            return self.self_state.name.clone();
        }
        self.mobiles
            .get(&serial)
            .map(|m| m.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| serial.to_string())
    }

    /// Record a property list reply: keep its revision and take the name.
    fn accept_properties(&mut self, serial: Serial, hash: u32, properties: &[ObjectProperty]) {
        self.names.accept(serial, hash);
        self.properties.insert(serial, properties.to_vec());
        if let Some(name) = display_name(properties) {
            self.set_object_name(serial, name);
        }
    }

    fn apply_mobile_view(&mut self, view: &MobileView) {
        if view.serial == self.self_state.serial {
            self.self_state.location = Point3::new(view.x, view.y, view.z);
            self.self_state.direction = view.direction;
            self.self_state.hue = view.hue;
            self.self_state.body = view.body;
            self.self_state.notoriety = view.notoriety;
            self.apply_self_flags(view.flags);
            // Every worn item is an item as well as a row on the paperdoll.
            // Copying the rows alone leaves what the character already wore at
            // login out of the item map altogether, so a lookup by serial or
            // by graphic finds nothing: a dagger on layer one, and no dagger.
            // `wear` writes the row and the item together, which is why the
            // rows are cleared here and put back through it.
            if !view.equipment.is_empty() {
                self.self_state.equipment.clear();
                for worn in &view.equipment {
                    self.wear(worn);
                }
            }
            self.refresh_dead_from_body(view.body);
            return;
        }
        let old_name = self.mobiles.get(&view.serial).map(|m| m.name.clone());
        let mut mob = Mobile::from(view);
        if mob.name.is_empty() {
            if let Some(name) = old_name {
                mob.name = name;
            }
        }
        if mob.name.is_empty() {
            self.names.want(view.serial);
        }
        self.mobiles.insert(view.serial, mob);
    }

    fn apply_self_flags(&mut self, flags: u8) {
        self.self_state.flags = flags;
        self.self_state.war = flags & FLAG_WAR != 0;
        self.self_state.hidden = flags & FLAG_HIDDEN != 0;
        if !self.self_state.hidden {
            self.self_state.stealth_steps = 0;
        }
        self.self_state.paralyzed = flags & FLAG_FROZEN != 0;
        // Every client reads the yellow bar in this bit; a client from 7.0 up
        // also gets it on the health bar packet.
        self.self_state.yellow_bar = flags & FLAG_BLESSED != 0;
        if self.flags_mean_flying {
            self.self_state.flying = flags & FLAG_POISONED_OR_FLYING != 0;
        } else {
            self.self_state.poisoned = flags & FLAG_POISONED_OR_FLYING != 0;
        }
    }

    /// The health bar packet: poison and the yellow bar, for the character
    /// or anyone else.
    fn apply_bars(&mut self, serial: Serial, bars: &[HealthBarStatus]) {
        let mine = serial == self.self_state.serial;
        for bar in bars {
            match bar.kind {
                HEALTH_BAR_POISON if mine => self.self_state.poisoned = bar.enabled,
                HEALTH_BAR_YELLOW if mine => self.self_state.yellow_bar = bar.enabled,
                HEALTH_BAR_POISON => self.bars.entry(serial).or_default().poisoned = bar.enabled,
                HEALTH_BAR_YELLOW => self.bars.entry(serial).or_default().yellow = bar.enabled,
                _ => {}
            }
        }
    }

    /// True when the mobile is poisoned, by whichever sign this client reads.
    pub fn is_poisoned(&self, serial: Serial) -> bool {
        if serial == self.self_state.serial {
            return self.self_state.poisoned;
        }
        let by_bar = self.bars.get(&serial).is_some_and(|b| b.poisoned);
        let by_flag = !self.flags_mean_flying
            && self
                .mobiles
                .get(&serial)
                .is_some_and(|m| m.flags & FLAG_POISONED_OR_FLYING != 0);
        by_bar || by_flag
    }

    /// True when the mobile wears the yellow bar of the blessed.
    pub fn has_yellow_bar(&self, serial: Serial) -> bool {
        if serial == self.self_state.serial {
            return self.self_state.yellow_bar;
        }
        self.bars.get(&serial).is_some_and(|b| b.yellow)
            || self
                .mobiles
                .get(&serial)
                .is_some_and(|m| m.flags & FLAG_BLESSED != 0)
    }

    fn refresh_dead_from_body(&mut self, body: u16) {
        let ghost = is_ghost_body(body);
        if self.self_state.dead && !ghost {
            self.self_state.dead = false;
            self.push_event(Event::new(
                EventKind::Resurrected,
                Some(self.self_state.serial),
                self.self_state.name.clone(),
            ));
        } else if ghost {
            self.self_state.dead = true;
        }
    }

    fn wear(&mut self, eq: &EquipItem) {
        self.self_state.equipment.retain(|e| e.layer != eq.layer);
        self.self_state.equipment.push(eq.clone());
        let name = self.name_or_ask(eq.serial);
        self.items.insert(
            eq.serial,
            Item {
                serial: eq.serial,
                graphic: eq.graphic,
                amount: 1,
                hue: eq.hue,
                location: self.self_state.location,
                parent: Some(self.self_state.serial),
                layer: Some(eq.layer),
                grid: 0,
                name,
            },
        );
    }

    fn detach(&mut self, serial: Serial) {
        if let Some(parent) = self.items.get(&serial).and_then(|i| i.parent) {
            if let Some(container) = self.containers.get_mut(&parent) {
                container.items.retain(|s| *s != serial);
            }
        }
    }

    fn upsert_ground(&mut self, item: &GroundItem) {
        let name = self.name_or_ask(item.serial);
        self.detach(item.serial);
        self.items.insert(
            item.serial,
            Item {
                serial: item.serial,
                graphic: item.graphic,
                amount: item.amount,
                hue: item.hue,
                location: Point3::new(item.x, item.y, item.z),
                parent: None,
                layer: None,
                grid: 0,
                name,
            },
        );
    }

    /// The display name we already learned for an item, and a question to the
    /// server when we have none.
    ///
    /// Every item record is built afresh from the packet that carries it, so a
    /// name learned once is lost the next time the server sends the same item
    /// unless it is carried over here. A shard resends a corpse or a pack
    /// every time it is opened.
    fn name_or_ask(&mut self, serial: Serial) -> String {
        let name = self
            .items
            .get(&serial)
            .map(|i| i.name.clone())
            .unwrap_or_default();
        if name.is_empty() {
            self.names.want(serial);
        }
        name
    }

    /// Put a name learned from a property list on whichever object owns it.
    fn set_object_name(&mut self, serial: Serial, name: String) {
        if serial == self.self_state.serial {
            self.self_state.name = name;
            return;
        }
        if let Some(mob) = self.mobiles.get_mut(&serial) {
            mob.name = name;
            return;
        }
        if let Some(item) = self.items.get_mut(&serial) {
            item.name = name;
        }
    }

    fn upsert_container_item(&mut self, item: &ContainerItem) {
        let name = self.name_or_ask(item.serial);
        self.detach(item.serial);
        self.items.insert(
            item.serial,
            Item {
                serial: item.serial,
                graphic: item.graphic,
                amount: item.amount,
                hue: item.hue,
                location: Point3::new(item.x, item.y, 0),
                parent: Some(item.container),
                layer: None,
                grid: item.grid,
                name,
            },
        );
        let container = self.containers.entry(item.container).or_insert(Container {
            serial: item.container,
            gump: 0,
            items: Vec::new(),
        });
        if !container.items.contains(&item.serial) {
            container.items.push(item.serial);
        }
    }

    fn set_hits(&mut self, serial: Serial, current: u16, max: u16) {
        if serial == self.self_state.serial {
            if current < self.self_state.hits {
                self.push_event(Event::new(
                    EventKind::Damaged,
                    Some(serial),
                    format!("{} -> {}", self.self_state.hits, current),
                ));
                self.note_harm_now();
            }
            self.self_state.hits = current;
            self.self_state.hits_max = max;
        } else if let Some(mob) = self.mobiles.get_mut(&serial) {
            mob.hits = Some(current);
            mob.hits_max = Some(max);
        }
    }

    /// Records a door item the server sent and says what it did to the record.
    ///
    /// The caller decides what is a door, because only the client tile data
    /// knows which graphics carry the door flag.
    pub fn note_door(&mut self, serial: Serial, graphic: u16, location: Point3) -> DoorUpdate {
        let door = DoorItem {
            serial,
            graphic,
            location,
        };
        match self.doors.insert(serial, door) {
            None => DoorUpdate::Learned,
            Some(known) if known == door => DoorUpdate::Unchanged,
            Some(_) => DoorUpdate::Swung,
        }
    }

    /// Forgets a door: the item is gone, or the graphic it now carries is not
    /// a door graphic.
    pub fn forget_door(&mut self, serial: Serial) {
        self.doors.remove(&serial);
    }

    /// Every tile a door item stands on.
    ///
    /// A shut door stands in its own doorway and an open one has swung off it,
    /// so a route that goes around these tiles stops in front of a shut door
    /// and walks straight through an open one. No graphic is read to tell the
    /// two apart.
    pub fn door_tiles(&self) -> Vec<Point3> {
        self.doors.values().map(|door| door.location).collect()
    }

    /// Records a building the server sent and says what it did to the record.
    ///
    /// The caller decides what is a building, because only the item packet the
    /// server wrote says whether the graphic on it names a multi.
    pub fn note_multi(&mut self, serial: Serial, multi_id: u16, location: Point3) -> MultiUpdate {
        let multi = MultiItem {
            serial,
            multi_id,
            location,
        };
        match self.multis.insert(serial, multi) {
            None => MultiUpdate::Learned,
            Some(known) if known == multi => MultiUpdate::Unchanged,
            Some(_) => MultiUpdate::Moved,
        }
    }

    /// Forgets a building: the item is gone, or the graphic it now carries
    /// names no multi.
    pub fn forget_multi(&mut self, serial: Serial) {
        self.multis.remove(&serial);
    }

    /// Every building the character can see, as the route planning reads them.
    pub fn multis_seen(&self) -> Vec<MultiItem> {
        self.multis.values().copied().collect()
    }

    /// True while the character stands on a facet where anyone may move over
    /// anyone else, so no other mobile is in his way.
    pub fn free_movement(&self) -> bool {
        facet_free_movement(self.self_state.map)
    }

    /// Where every other mobile stands that a route has to go around.
    ///
    /// On a facet that carries [`MAP_RULE_FREE_MOVEMENT`] the server lets the
    /// character walk straight through anybody, so nobody is in his way and
    /// this is empty. Planning around them there only makes his routes longer
    /// and has him dodge people he could have walked through.
    pub fn blocking_mobile_tiles(&self) -> Vec<Point3> {
        if self.free_movement() {
            return Vec::new();
        }
        self.mobiles.values().map(|m| m.location).collect()
    }

    pub fn mobile_at(&self, x: u16, y: u16) -> Option<&Mobile> {
        self.mobiles
            .values()
            .find(|m| m.location.x == x && m.location.y == y)
    }

    pub fn item_at(&self, x: u16, y: u16) -> Option<&Item> {
        self.items
            .values()
            .find(|i| i.parent.is_none() && i.location.x == x && i.location.y == y)
    }

    pub fn find_mobiles(
        &self,
        name: Option<&str>,
        graphic: Option<u16>,
        max_dist: Option<u16>,
    ) -> Vec<&Mobile> {
        self.mobiles
            .values()
            .filter(|m| name.map(|n| m.name.eq_ignore_ascii_case(n)).unwrap_or(true))
            .filter(|m| graphic.map(|g| m.body == g).unwrap_or(true))
            .filter(|m| {
                max_dist
                    .map(|d| self.self_state.location.chebyshev(m.location) <= u32::from(d))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// Every item that matches, wherever it is.
    ///
    /// Items inside containers are searched as well as items on the ground:
    /// pass the container's serial to look in one of them, and leave it out to
    /// look everywhere the character can see.
    pub fn find_items(
        &self,
        graphic: Option<u16>,
        container: Option<Serial>,
        name: Option<&str>,
    ) -> Vec<&Item> {
        let word = name.map(|n| n.trim().to_ascii_lowercase());
        self.items
            .values()
            .filter(|i| graphic.map(|g| i.graphic == g).unwrap_or(true))
            .filter(|i| container.map(|c| i.parent == Some(c)).unwrap_or(true))
            .filter(|i| word.as_deref().map(|w| item_named(i, w)).unwrap_or(true))
            .collect()
    }

    /// True when the item sits in `container`, straight in it or in a bag
    /// inside it.
    pub fn is_inside(&self, item: Serial, container: Serial) -> bool {
        let mut holder = self.items.get(&item).and_then(|i| i.parent);
        for _ in 0..MAX_NESTING {
            match holder {
                Some(p) if p == container => return true,
                Some(p) => holder = self.items.get(&p).and_then(|i| i.parent),
                None => return false,
            }
        }
        false
    }

    /// The items in a container, lowest serial first. With `deep`, the items
    /// in the bags inside it count too.
    pub fn items_inside(&self, container: Serial, deep: bool) -> Vec<&Item> {
        let mut found: Vec<&Item> = self
            .items
            .values()
            .filter(|i| {
                if deep {
                    self.is_inside(i.serial, container)
                } else {
                    i.parent == Some(container)
                }
            })
            .collect();
        found.sort_by_key(|i| i.serial.0);
        found
    }

    /// What the character wears on a layer.
    pub fn worn(&self, layer: u8) -> Option<&EquipItem> {
        self.self_state.equipment.iter().find(|e| e.layer == layer)
    }

    /// Where a thing is on the map: its own tile on the ground, or the tile
    /// of the mobile or the ground container that holds it.
    pub fn map_location(&self, serial: Serial) -> Option<Point3> {
        if serial == self.self_state.serial {
            return Some(self.self_state.location);
        }
        if let Some(mobile) = self.mobiles.get(&serial) {
            return Some(mobile.location);
        }
        let mut at = serial;
        for _ in 0..MAX_NESTING {
            let item = self.items.get(&at)?;
            match item.parent {
                None => return Some(item.location),
                Some(holder) if holder == self.self_state.serial => {
                    return Some(self.self_state.location)
                }
                Some(holder) => match self.mobiles.get(&holder) {
                    Some(mobile) => return Some(mobile.location),
                    None => at = holder,
                },
            }
        }
        None
    }

    pub fn find_item_graphic(&self, graphic: u16) -> Option<&Item> {
        self.items.values().find(|i| i.graphic == graphic)
    }

    pub fn nearby_mobiles(&self, dist: u16) -> Vec<&Mobile> {
        self.find_mobiles(None, None, Some(dist))
    }

    pub fn equipped_weapon_graphic(&self) -> Option<u16> {
        self.self_state.equipment.iter().find_map(|eq| {
            if eq.layer == LAYER_ONE_HANDED || eq.layer == LAYER_TWO_HANDED {
                Some(eq.graphic)
            } else {
                None
            }
        })
    }

    pub fn attack_range(&self) -> u16 {
        self.equipped_weapon_graphic()
            .map(weapon_range)
            .unwrap_or(RANGE_MELEE)
    }

    pub fn fighting(&self) -> bool {
        self.combatant.is_some_and(Serial::is_valid)
    }

    /// A wound the character has just taken moves the attacker memory on. It
    /// names nobody: the packets that take health away name the one who loses
    /// it, so a wound only says the one already named is still at work.
    fn note_harm_now(&mut self) {
        if let Some(harm) = self.harmed_by.as_mut() {
            harm.at = Instant::now();
        }
    }

    /// Who harmed the character no longer ago than `within`, if anybody.
    ///
    /// The memory has to end by itself. Nothing on the wire says a fight is
    /// over, so a character who answers his attacker for ever answers a corpse
    /// or a thing that has walked away.
    pub fn recent_attacker(&self, now: Instant, within: Duration) -> Option<Serial> {
        let harm = self.harmed_by?;
        (now.saturating_duration_since(harm.at) <= within).then_some(harm.by)
    }

    /// The bank box the character wears, if the server has sent his equipment.
    /// It rides on [`LAYER_BANK`] like any worn item; the server fills and
    /// opens it when he says "bank" beside a banker, so an agent can address it
    /// by this serial without waiting for the open packet.
    pub fn bank_box(&self) -> Option<Serial> {
        self.self_state
            .equipment
            .iter()
            .find(|item| item.layer == LAYER_BANK)
            .map(|item| item.serial)
    }

    /// True when a swing is overdue given the last gap the server gave us.
    pub fn swing_is_late(&self, now: Instant) -> bool {
        let Some(last) = self.last_swing else {
            return false;
        };
        let Some(gap) = self.last_swing_gap else {
            return false;
        };
        now.saturating_duration_since(last) > gap
    }

    pub fn nearby_items(&self, dist: u16) -> Vec<&Item> {
        self.items
            .values()
            .filter(|i| {
                i.parent.is_none()
                    && self.self_state.location.chebyshev(i.location) <= u32::from(dist)
            })
            .collect()
    }

    pub fn observe(&self, tile: impl Fn(u16, u16) -> char) -> Observe {
        let radar = render_radar(self, RadarOptions::default(), tile);
        Observe::from_world(self, radar)
    }

    pub fn observe_default(&self) -> Observe {
        self.observe(default_tile)
    }

    pub fn radar<F: Fn(u16, u16) -> TileKind + ?Sized>(&self, size: u16, tile: &F) -> String {
        let size = if size == 0 { RADAR_DEFAULT } else { size };
        render_radar(self, RadarOptions { size }, |x, y| tile(x, y).as_char())
    }

    pub fn observe_json(&self, radar: &str) -> serde_json::Value {
        serde_json::to_value(Observe::from_world(self, radar.to_string()))
            .unwrap_or(serde_json::Value::Null)
    }

    pub fn set_nav(&mut self, goal: Option<Point3>, path_len: usize) {
        self.nav_goal = goal;
        self.path_len = path_len;
    }

    pub fn clear_target(&mut self) {
        self.pending_target = None;
    }

    pub fn close_gump(&mut self, gump_id: u32) {
        self.gumps.retain(|g| g.gump_id != gump_id);
    }
}

/// True when a lower-case search word names this item.
///
/// The word is matched inside the display name, because a shard writes "54
/// gold coins" where an agent looking for loot asks for "gold". It is matched
/// whole against the graphic written as hex, because that form is one token
/// and half of it names something else.
fn item_named(item: &Item, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    item.name.to_ascii_lowercase().contains(word)
        || format!("0x{:04X}", item.graphic).eq_ignore_ascii_case(word)
}

pub fn is_ghost_body(body: u16) -> bool {
    matches!(
        body,
        BODY_GHOST_MALE | BODY_GHOST_FEMALE | BODY_GHOST_ELF_MALE | BODY_GHOST_ELF_FEMALE
    )
}

fn paperdoll_name(text: &str) -> &str {
    text.split('(').next().map(str::trim).unwrap_or(text)
}
