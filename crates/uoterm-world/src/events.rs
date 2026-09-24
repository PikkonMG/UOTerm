use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use uoterm_protocol::Serial;

pub const EVENT_LOG_CAP: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    LoggedIn,
    Speech,
    Damaged,
    Died,
    Resurrected,
    ItemAdded,
    ContainerOpened,
    TargetRequested,
    GumpOpened,
    /// The shard closed a gump, named by its type number.
    GumpClosed,
    /// A shopkeeper's list of goods came: what he sells, or what he buys.
    /// `observe` holds the goods and their prices.
    ShopOpened,
    /// The lines of a context menu came. `observe` holds them.
    ContextMenuOpened,
    /// An old-style menu opened: a question with a list of answers.
    MenuOpened,
    /// A buff or a debuff came on the character or went off him, named by
    /// its icon.
    BuffChanged,
    /// The shard told the character something in its own voice, such as
    /// "that is too far away".
    SystemMessage,
    Arrived,
    PathFailed,
    PkFlag,
    PartyInvite,
    Disconnected,
    CombatantChanged,
    LiftRejected,
    /// Another character said this one's name.
    SpokenTo,
    /// The character stopped playing along with a player, and why.
    PlayAlongEnded,
    /// The character's health fell under the low mark.
    LowHealth,
    /// A mobile the character may fight came near.
    EnemyNear,
    /// The shard waits for a line of text: a prompt or a text dialog.
    PromptOpened,
    /// Another player opened a trade window with the character.
    TradeOpened,
    /// A loot or bank job gave up, and why.
    JobFailed,
    /// A session job ended on purpose, and why.
    JobEnded,
    /// The character went to another map (facet), named by its number.
    MapChanged,
    /// A human took the character. The agent may look but not act.
    ControlTaken,
    /// The agent has the character again, and why.
    ControlReleased,
    /// The shard played a sound: its number and where.
    Sound,
    /// A picture flew, flashed or stayed for a moment: a fireball, a heal.
    Effect,
    /// A mobile played an action: a swing, a cast, a bow.
    Animation,
    /// An item left the world: used up, taken away or out of sight.
    ItemDeleted,
    /// The shard put up a quest arrow, or took it down.
    QuestArrow,
    /// A map item opened: a treasure map or a city map.
    MapOpened,
    /// A skill went up or down, with the change.
    SkillChanged,
    /// Strength, dexterity or intelligence went up or down, with the change.
    StatChanged,
    /// The shard told where the party or the guild members out of sight
    /// stand.
    MemberPositions,
    /// The weapon move was spent or cleared, or a spell or stance that stays
    /// on came on or went off.
    AbilityChanged,
    /// The shard asks the character to pick new looks for another race.
    /// `observe` holds the race and the looks it may pick.
    RaceChangeOpened,
}

/// The kinds that come many times a second in a busy place. `next_event`
/// gives them only when the caller asks for them, and a full log drops them
/// before the others.
pub const AMBIENT_EVENT_KINDS: [EventKind; 5] = [
    EventKind::Sound,
    EventKind::Effect,
    EventKind::Animation,
    EventKind::ItemDeleted,
    EventKind::MemberPositions,
];

impl EventKind {
    /// True for one of the [`AMBIENT_EVENT_KINDS`].
    pub fn is_ambient(self) -> bool {
        AMBIENT_EVENT_KINDS.contains(&self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub kind: EventKind,
    pub unix_ms: u64,
    pub serial: Option<Serial>,
    pub text: String,
}

/// Milliseconds since 1970, or 0 when the clock is before it.
pub fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Event {
    pub fn new(kind: EventKind, serial: Option<Serial>, text: impl Into<String>) -> Self {
        let unix_ms = unix_now_ms();
        Self {
            seq: 0,
            kind,
            unix_ms,
            serial,
            text: text.into(),
        }
    }

    pub fn fact_line(&self) -> String {
        match self.kind {
            EventKind::Speech => self.text.clone(),
            EventKind::Died => format!("{} died", self.text),
            EventKind::PkFlag => format!("pk flag: {}", self.text),
            _ => format!("{:?}: {}", self.kind, self.text),
        }
    }
}
