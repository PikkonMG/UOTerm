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
    Arrived,
    PathFailed,
    PkFlag,
    PartyInvite,
    Disconnected,
    CombatantChanged,
    LiftRejected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    pub kind: EventKind,
    pub unix_ms: u64,
    pub serial: Option<Serial>,
    pub text: String,
}

impl Event {
    pub fn new(kind: EventKind, serial: Option<Serial>, text: impl Into<String>) -> Self {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
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
