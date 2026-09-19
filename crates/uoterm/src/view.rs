//! A compact watch picture of one session: map, vitals, activity, journal.
//!
//! The window and the text loop both draw this. Tests check the picture,
//! not the OS window.

use serde_json::Value;

pub const WATCH_POLL_MS: u64 = 250;
pub const WATCH_RADAR_SIZE: u16 = 31;
/// The window asks for the largest radar, so the map without client files
/// covers as much of the window as it can.
pub const WINDOW_RADAR_SIZE: u16 = 41;
/// With client files the window draws the map itself, so it asks for the
/// smallest radar. The session then has less work for each picture.
pub const WINDOW_RADAR_SIZE_WITH_ART: u16 = 5;
/// The window asks more often than the terminal, so that a walk looks smooth.
pub const WINDOW_POLL_MS: u64 = 100;
pub const WINDOW_TITLE: &str = "UOTerm watch";
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 800.0;
pub const JOURNAL_LINES: usize = 12;
pub const MOBILE_LINES: usize = 12;
/// The layer on which a character wears his backpack.
pub const LAYER_BACKPACK: u8 = 0x15;
const PERCENT: u32 = 100;
/// Below this share of his hits the character is in serious danger.
const CRITICAL_HITS_PERCENT: u32 = 35;
/// An enemy or a murderer this near is a danger before he attacks.
const HOSTILE_NEAR_TILES: u16 = 10;
const NOTORIETY_ENEMY: u8 = 5;
const NOTORIETY_MURDERER: u8 = 6;

pub const SYM_SELF: char = '@';
pub const SYM_BLOCK: char = '#';
pub const SYM_WALK: char = '.';
pub const SYM_WATER: char = '~';
pub const SYM_DOOR: char = '+';
pub const SYM_DEST: char = 'X';

/// How much the operator must worry, least first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Danger {
    Calm,
    Fight,
    Critical,
    Dead,
}

/// One worn item, as the animation files need it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WatchEquip {
    pub serial: u32,
    pub graphic: u16,
    pub layer: u8,
    pub hue: u16,
}

/// How a mobile looks: what the animation files need to draw him.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct WatchLook {
    pub body: u16,
    pub hue: u16,
    /// The way he faces, 0 for north and then clockwise to 7.
    pub direction: u8,
    pub equipment: Vec<WatchEquip>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchMobile {
    pub serial: u32,
    pub name: String,
    pub title: String,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub notoriety: u8,
    /// The share of his hits the mobile has left, when the shard told it.
    pub hits_percent: Option<u8>,
    pub dist: u16,
    pub look: WatchLook,
}

/// One item that lies on the ground.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchItem {
    pub serial: u32,
    pub name: String,
    pub graphic: u16,
    pub hue: u16,
    pub x: u16,
    pub y: u16,
    pub z: i8,
}

/// One sound effect the shard asked for. The window plays each `seq` once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WatchSound {
    pub seq: u64,
    pub sound: u16,
    pub x: u16,
    pub y: u16,
}

/// One item inside an open container.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchPackItem {
    pub serial: u32,
    pub graphic: u16,
    pub hue: u16,
    pub amount: u16,
    pub name: String,
}

/// A container the character has opened: his pack, a chest, a corpse.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchContainer {
    pub serial: u32,
    pub name: String,
    /// How many items it holds. `items` may list fewer.
    pub total: usize,
    pub items: Vec<WatchPackItem>,
}

/// A button of a gump. `id` answers the gump. `to_page` only turns the page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchGumpButton {
    pub id: Option<u32>,
    pub to_page: Option<u32>,
    pub page: u32,
    pub label: String,
}

/// A box of a gump that the player ticks before he answers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchGumpChoice {
    pub switch: u32,
    /// One of a group. A tick on it clears the others of its page.
    pub radio: bool,
    pub on: bool,
    pub page: u32,
    pub label: String,
}

/// A dialog the shard opened, in words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchGump {
    pub gump: u32,
    /// Each line of words, with the page it is on. Page 0 is on each page.
    pub texts: Vec<(u32, String)>,
    pub buttons: Vec<WatchGumpButton>,
    pub choices: Vec<WatchGumpChoice>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchFrame {
    pub name: String,
    pub hits: u16,
    pub hits_max: u16,
    pub mana: u16,
    pub mana_max: u16,
    pub stam: u16,
    pub stam_max: u16,
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub map: u8,
    pub facing: String,
    pub look: WatchLook,
    pub notoriety: u8,
    pub war: bool,
    pub dead: bool,
    /// A human has the character, and the agent waits.
    pub human_control: bool,
    /// The shard waits for the character to point at something.
    pub target_cursor: bool,
    pub poisoned: bool,
    pub hidden: bool,
    pub paralyzed: bool,
    pub gold: u32,
    pub weight: u16,
    pub weight_max: u16,
    pub goal: String,
    pub job: String,
    pub phase: String,
    pub script: String,
    pub following: String,
    pub combatant: String,
    pub dest_x: Option<u16>,
    pub dest_y: Option<u16>,
    pub radar: Vec<String>,
    pub journal: Vec<String>,
    /// The lines spoken to the character that wait for an answer.
    pub unanswered: usize,
    pub mobiles: Vec<WatchMobile>,
    pub items: Vec<WatchItem>,
    pub buffs: Vec<String>,
    pub party: Vec<String>,
    pub containers: Vec<WatchContainer>,
    pub gumps: Vec<WatchGump>,
    pub sounds: Vec<WatchSound>,
    /// The music the shard asked for. None for silence.
    pub music: Option<u16>,
    pub error: String,
}

impl WatchFrame {
    pub fn from_observe(value: &Value) -> Self {
        let me = value
            .get("self_state")
            .or_else(|| value.get("me"))
            .or_else(|| value.get("self"));
        let loc = me
            .and_then(|m| m.get("location"))
            .or_else(|| value.get("location"));
        let x = pick_u16(num_opt(loc, "x"), num_opt_at(value, "x"));
        let y = pick_u16(num_opt(loc, "y"), num_opt_at(value, "y"));
        let doing = value.get("doing");
        let job_value = doing.and_then(|d| d.get("job"));
        let job = job_value
            .and_then(|j| j.get("name"))
            .and_then(Value::as_str)
            .or_else(|| job_value.and_then(Value::as_str))
            .unwrap_or("-");
        let goal = doing
            .and_then(|d| d.get("goal"))
            .and_then(Value::as_str)
            .or_else(|| value.get("goal").and_then(Value::as_str))
            .unwrap_or("-");
        let walking = doing.and_then(|d| d.get("walking_to"));
        let dest_x = walking.and_then(|w| num_opt(Some(w), "x"));
        let dest_y = walking.and_then(|w| num_opt(Some(w), "y"));
        let mobiles = value.get("mobiles").or_else(|| value.get("nearby_mobiles"));
        let self_serial = me.and_then(|m| m.get("serial")).and_then(Value::as_u64);
        let mut radar = radar_rows(value.get("radar").and_then(Value::as_str).unwrap_or(""));
        overlay_dest(&mut radar, x, y, dest_x, dest_y);
        Self {
            name: pick_string(string_field(me, "name"), string_at(value, "name")),
            hits: pick_u16(num_opt(me, "hits"), num_opt_at(value, "hits")),
            hits_max: pick_u16(num_opt(me, "hits_max"), num_opt_at(value, "hits_max")),
            mana: pick_u16(num_opt(me, "mana"), num_opt_at(value, "mana")),
            mana_max: pick_u16(num_opt(me, "mana_max"), num_opt_at(value, "mana_max")),
            stam: pick_u16(num_opt(me, "stam"), num_opt_at(value, "stam")),
            stam_max: pick_u16(num_opt(me, "stam_max"), num_opt_at(value, "stam_max")),
            x,
            y,
            z: if loc.is_some() {
                signed_field(loc, "z")
            } else {
                signed_field(Some(value), "z")
            },
            map: pick_u16(num_opt(me, "map"), num_opt_at(value, "map")) as u8,
            facing: string_at(value, "facing"),
            look: watch_look(me),
            notoriety: num_field(me, "notoriety") as u8,
            war: bool_field(me, "war") || bool_at(value, "war"),
            dead: bool_field(me, "dead") || bool_at(value, "dead"),
            human_control: bool_at(value, "human_control"),
            target_cursor: bool_at(value, "pending_target"),
            poisoned: bool_field(me, "poisoned"),
            hidden: bool_field(me, "hidden"),
            paralyzed: bool_field(me, "paralyzed"),
            gold: me
                .and_then(|m| m.get("gold"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            weight: num_field(me, "weight"),
            weight_max: num_field(me, "weight_max"),
            goal: goal.to_string(),
            job: job.to_string(),
            phase: string_field(job_value, "phase"),
            script: string_field(doing, "script"),
            following: string_field(doing, "following"),
            combatant: string_at(value, "combatant"),
            dest_x,
            dest_y,
            radar,
            journal: journal_lines(value.get("journal")),
            unanswered: value
                .get("spoken_to")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            mobiles: watch_mobiles(mobiles, self_serial, x, y),
            items: watch_items(value.get("items")),
            buffs: string_list(value.get("buffs")),
            party: string_list(value.get("party")),
            containers: list_of(value.get("containers"), watch_container),
            gumps: list_of(value.get("gumps"), watch_gump),
            sounds: list_of(value.get("sounds"), |cue| WatchSound {
                seq: cue.get("seq").and_then(Value::as_u64).unwrap_or(0),
                sound: num_field(Some(cue), "sound"),
                x: num_field(Some(cue), "x"),
                y: num_field(Some(cue), "y"),
            }),
            music: num_opt_at(value, "music"),
            error: String::new(),
        }
    }

    /// The backpack the character wears, when the shard told of it.
    pub fn backpack(&self) -> Option<u32> {
        self.look
            .equipment
            .iter()
            .find(|item| item.layer == LAYER_BACKPACK)
            .map(|item| item.serial)
    }

    pub fn danger(&self) -> Danger {
        if self.dead {
            Danger::Dead
        } else if u32::from(self.hits) * PERCENT < u32::from(self.hits_max) * CRITICAL_HITS_PERCENT
        {
            Danger::Critical
        } else if self.war || !self.combatant.is_empty() || self.hostile_near() {
            Danger::Fight
        } else {
            Danger::Calm
        }
    }

    fn hostile_near(&self) -> bool {
        self.mobiles.iter().any(|m| {
            m.dist <= HOSTILE_NEAR_TILES
                && matches!(m.notoriety, NOTORIETY_ENEMY | NOTORIETY_MURDERER)
        })
    }

    pub fn error_frame(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            ..Self::default()
        }
    }

    pub fn text(&self) -> String {
        let mut out = String::new();
        if !self.error.is_empty() {
            out.push_str(&self.error);
            out.push('\n');
            return out;
        }
        out.push_str(&format!(
            "{}  hp {}/{}  mana {}/{}  stam {}/{}  at {},{},{} map {}  war={} dead={}\n",
            self.name,
            self.hits,
            self.hits_max,
            self.mana,
            self.mana_max,
            self.stam,
            self.stam_max,
            self.x,
            self.y,
            self.z,
            self.map,
            self.war,
            self.dead
        ));
        out.push_str(&format!("goal {}  job {}\n", self.goal, self.job));
        if let (Some(dx), Some(dy)) = (self.dest_x, self.dest_y) {
            out.push_str(&format!("dest {dx},{dy}\n"));
        }
        for row in &self.radar {
            out.push_str(row);
            out.push('\n');
        }
        if !self.mobiles.is_empty() {
            out.push_str("near:\n");
            for mobile in self.mobiles.iter().take(MOBILE_LINES) {
                out.push_str(&format!("{} d={}\n", mobile.name, mobile.dist));
            }
        }
        if !self.journal.is_empty() {
            out.push_str("journal:\n");
            for line in self.journal.iter().rev().take(JOURNAL_LINES).rev() {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }
}

fn overlay_dest(
    rows: &mut [String],
    origin_x: u16,
    origin_y: u16,
    dest_x: Option<u16>,
    dest_y: Option<u16>,
) {
    let (Some(dest_x), Some(dest_y)) = (dest_x, dest_y) else {
        return;
    };
    if rows.is_empty() {
        return;
    }
    let size = rows.len() as i32;
    let half = size / 2;
    let col = half + i32::from(dest_x) - i32::from(origin_x);
    let row = half + i32::from(dest_y) - i32::from(origin_y);
    if row < 0 || col < 0 || row >= size {
        return;
    }
    let row = row as usize;
    let col = col as usize;
    let Some(line) = rows.get_mut(row) else {
        return;
    };
    let mut chars: Vec<char> = line.chars().collect();
    if col >= chars.len() {
        return;
    }
    if chars[col] != SYM_SELF {
        chars[col] = SYM_DEST;
        *line = chars.into_iter().collect();
    }
}

fn radar_rows(radar: &str) -> Vec<String> {
    radar
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn journal_lines(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    if let Some(text) = value.as_str() {
        return text.lines().map(str::to_string).collect();
    }
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|line| {
            line.get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| line.as_str().map(str::to_string))
        })
        .collect()
}

fn watch_mobiles(
    value: Option<&Value>,
    self_serial: Option<u64>,
    origin_x: u16,
    origin_y: u16,
) -> Vec<WatchMobile> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out: Vec<WatchMobile> = arr
        .iter()
        .filter(|m| {
            let serial = m.get("serial").and_then(Value::as_u64);
            serial.is_none() || serial != self_serial
        })
        .map(|m| {
            let loc = m.get("location");
            let x = num_field(loc, "x");
            let y = num_field(loc, "y");
            let hits = num_opt(Some(m), "hits");
            let hits_max = num_opt(Some(m), "hits_max").filter(|max| *max > 0);
            WatchMobile {
                serial: m.get("serial").and_then(Value::as_u64).unwrap_or(0) as u32,
                name: m
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|n| !n.is_empty())
                    .unwrap_or("?")
                    .to_string(),
                title: string_field(Some(m), "title"),
                x,
                y,
                z: signed_field(loc, "z"),
                notoriety: num_field(Some(m), "notoriety") as u8,
                hits_percent: hits
                    .zip(hits_max)
                    .map(|(now, max)| (u32::from(now.min(max)) * PERCENT / u32::from(max)) as u8),
                dist: x.abs_diff(origin_x).max(y.abs_diff(origin_y)),
                look: watch_look(Some(m)),
            }
        })
        .collect();
    out.sort_by_key(|m| m.dist);
    out
}

/// The running bit of a facing byte. The other bits are the direction.
const DIRECTION_MASK: u16 = 0x07;

fn watch_look(mobile: Option<&Value>) -> WatchLook {
    let equipment = mobile
        .and_then(|m| m.get("equipment"))
        .and_then(Value::as_array)
        .map(|worn| {
            worn.iter()
                .map(|item| WatchEquip {
                    serial: serial_field(item, "serial"),
                    graphic: num_field(Some(item), "graphic"),
                    layer: num_field(Some(item), "layer") as u8,
                    hue: num_field(Some(item), "hue"),
                })
                .collect()
        })
        .unwrap_or_default();
    WatchLook {
        body: num_field(mobile, "body"),
        hue: num_field(mobile, "hue"),
        direction: (num_field(mobile, "direction") & DIRECTION_MASK) as u8,
        equipment,
    }
}

/// The items on the ground. An item inside a container has a parent.
fn watch_items(value: Option<&Value>) -> Vec<WatchItem> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter(|i| i.get("parent").is_none_or(Value::is_null))
        .map(|i| {
            let loc = i.get("location");
            WatchItem {
                serial: i.get("serial").and_then(Value::as_u64).unwrap_or(0) as u32,
                name: string_field(Some(i), "name"),
                graphic: num_field(Some(i), "graphic"),
                hue: num_field(Some(i), "hue"),
                x: num_field(loc, "x"),
                y: num_field(loc, "y"),
                z: signed_field(loc, "z"),
            }
        })
        .collect()
}

fn list_of<T>(value: Option<&Value>, read: fn(&Value) -> T) -> Vec<T> {
    value
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(read).collect())
        .unwrap_or_default()
}

/// A serial as a number, or as the `0x` text that a container view holds.
fn serial_field(obj: &Value, key: &str) -> u32 {
    let Some(value) = obj.get(key) else {
        return 0;
    };
    let from_text = || {
        let text = value.as_str()?.trim();
        match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => u32::from_str_radix(hex, 16).ok(),
            None => text.parse().ok(),
        }
    };
    value
        .as_u64()
        .map(|n| n as u32)
        .or_else(from_text)
        .unwrap_or(0)
}

fn watch_container(value: &Value) -> WatchContainer {
    WatchContainer {
        serial: serial_field(value, "serial"),
        name: string_field(Some(value), "name"),
        total: value.get("total").and_then(Value::as_u64).unwrap_or(0) as usize,
        items: list_of(value.get("contents"), |item| WatchPackItem {
            serial: serial_field(item, "serial"),
            graphic: num_field(Some(item), "graphic"),
            hue: num_field(Some(item), "hue"),
            amount: num_field(Some(item), "amount"),
            name: string_field(Some(item), "name"),
        }),
    }
}

fn page_of(value: &Value) -> u32 {
    value.get("page").and_then(Value::as_u64).unwrap_or(0) as u32
}

fn watch_gump(value: &Value) -> WatchGump {
    let number = |v: &Value, key: &str| v.get(key).and_then(Value::as_u64).map(|n| n as u32);
    WatchGump {
        gump: number(value, "gump").unwrap_or(0),
        texts: list_of(value.get("texts"), |t| {
            (page_of(t), string_field(Some(t), "words"))
        }),
        buttons: value
            .get("buttons")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .map(|b| WatchGumpButton {
                        id: number(b, "id"),
                        to_page: number(b, "to_page"),
                        page: page_of(b),
                        label: string_field(Some(b), "label"),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        choices: list_of(value.get("choices"), |c| WatchGumpChoice {
            switch: c.get("switch").and_then(Value::as_u64).unwrap_or(0) as u32,
            radio: c.get("kind").and_then(Value::as_str) == Some("radio"),
            on: bool_field(Some(c), "on"),
            page: page_of(c),
            label: string_field(Some(c), "label"),
        }),
    }
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn signed_field(obj: Option<&Value>, key: &str) -> i8 {
    obj.and_then(|v| v.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i8
}

fn pick_string(a: String, b: String) -> String {
    if a.is_empty() {
        b
    } else {
        a
    }
}

fn pick_u16(a: Option<u16>, b: Option<u16>) -> u16 {
    a.or(b).unwrap_or(0)
}

fn num_opt(obj: Option<&Value>, key: &str) -> Option<u16> {
    obj.and_then(|v| v.get(key))
        .and_then(Value::as_u64)
        .map(|n| n as u16)
}

fn num_opt_at(value: &Value, key: &str) -> Option<u16> {
    value.get(key).and_then(Value::as_u64).map(|n| n as u16)
}

fn string_field(obj: Option<&Value>, key: &str) -> String {
    obj.and_then(|v| v.get(key))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn string_at(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn num_field(obj: Option<&Value>, key: &str) -> u16 {
    obj.and_then(|v| v.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u16
}

fn bool_field(obj: Option<&Value>, key: &str) -> bool {
    obj.and_then(|v| v.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn bool_at(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn watch_frame_reads_observe_json() {
        let value = json!({
            "self_state": {
                "name": "Mara",
                "hits": 80,
                "hits_max": 100,
                "mana": 10,
                "mana_max": 20,
                "stam": 30,
                "stam_max": 40,
                "map": 0,
                "war": true,
                "dead": false,
                "location": { "x": 1425, "y": 1695, "z": 0 }
            },
            "x": 1425,
            "y": 1695,
            "z": 0,
            "goal": "hunt",
            "doing": { "goal": "hunt", "job": { "name": "hunt", "phase": "kill" } },
            "radar": "...\n.@.\n...",
            "journal": ["a zombie is attacking you"],
            "mobiles": [{ "name": "a zombie", "location": { "x": 1427, "y": 1695, "z": 0 } }]
        });
        let frame = WatchFrame::from_observe(&value);
        assert_eq!(frame.name, "Mara");
        assert_eq!(frame.hits, 80);
        assert_eq!(frame.x, 1425);
        assert_eq!(frame.job, "hunt");
        assert_eq!(frame.radar.len(), 3);
        assert!(frame.text().contains('@'));
        assert!(frame.text().contains("a zombie"));
        assert_eq!(frame.phase, "kill");
        assert_eq!(frame.danger(), Danger::Fight);
        assert_eq!(frame.mobiles[0].dist, 2);
    }

    #[test]
    fn watch_frame_overlays_dest_and_names() {
        let value = json!({
            "self_state": {
                "name": "Mara",
                "hits": 80,
                "hits_max": 100,
                "location": { "x": 10, "y": 10, "z": 0 }
            },
            "doing": {
                "goal": "travel",
                "walking_to": { "x": 11, "y": 10, "z": 0 }
            },
            "radar": "...\n.@.\n...",
            "mobiles": [{ "name": "a zombie", "location": { "x": 10, "y": 9, "z": 0 } }]
        });
        let frame = WatchFrame::from_observe(&value);
        assert_eq!(frame.dest_x, Some(11));
        assert_eq!(frame.dest_y, Some(10));
        assert_eq!(frame.radar[1], ".@X");
        assert!(frame.text().contains("dest 11,10"));
        assert_eq!(frame.mobiles.len(), 1);
        assert_eq!(frame.mobiles[0].name, "a zombie");
        assert_eq!((frame.mobiles[0].x, frame.mobiles[0].y), (10, 9));
    }

    #[test]
    fn watch_frame_reads_danger_people_and_ground_items() {
        const SELF_SERIAL: u32 = 7;
        const PACK_SERIAL: u32 = 0x4000_0001;
        let value = json!({
            "self_state": {
                "serial": SELF_SERIAL,
                "name": "Mara",
                "hits": 20,
                "hits_max": 100,
                "equipment": [{ "serial": PACK_SERIAL, "graphic": 0x0E75, "layer": 21, "hue": 0 }],
                "location": { "x": 10, "y": 10, "z": -5 }
            },
            "mobiles": [
                { "serial": SELF_SERIAL, "name": "Mara", "location": { "x": 10, "y": 10, "z": -5 } },
                { "serial": 9, "name": "an orc", "notoriety": 6, "hits": 5, "hits_max": 20,
                  "body": 17, "hue": 0, "direction": 0x83,
                  "equipment": [{ "serial": 3, "graphic": 0x13B2, "layer": 1, "hue": 5 }],
                  "location": { "x": 13, "y": 10, "z": -5 } }
            ],
            "items": [
                { "serial": 1, "graphic": 0x0EED, "hue": 0, "parent": null,
                  "location": { "x": 11, "y": 10, "z": -5 } },
                { "serial": 2, "graphic": 0x0F0E, "hue": 0, "parent": PACK_SERIAL,
                  "location": { "x": 40, "y": 60, "z": 0 } }
            ],
            "buffs": ["Bless"],
            "sounds": [{ "seq": 3, "sound": 0x023B, "x": 12, "y": 10 }],
            "music": 9,
            "containers": [{ "serial": "0x40000001", "name": "backpack", "total": 1,
                "contents": [{ "serial": "0x40000009", "graphic": 3821, "amount": 54, "hue": 0, "name": "gold" }] }],
            "gumps": [{ "gump": 77, "texts": [{ "words": "Go where?" }],
                "buttons": [{ "id": 1, "label": "Okay" }, { "to_page": 2, "label": "Next" }],
                "choices": [{ "switch": 5, "kind": "radio", "on": true, "page": 1, "label": "Britain" }] }],
            "human_control": true,
            "pending_target": true,
            "spoken_to": [{ "name": "Ann", "text": "hail" }]
        });
        let frame = WatchFrame::from_observe(&value);
        assert_eq!(frame.z, -5);
        assert_eq!(frame.backpack(), Some(PACK_SERIAL));
        assert_eq!(frame.danger(), Danger::Critical);
        assert_eq!(frame.mobiles.len(), 1);
        assert_eq!(frame.mobiles[0].hits_percent, Some(25));
        assert_eq!(frame.mobiles[0].dist, 3);
        assert_eq!(frame.mobiles[0].look.body, 17);
        assert_eq!(frame.mobiles[0].look.direction, 3);
        assert_eq!(frame.mobiles[0].look.equipment[0].graphic, 0x13B2);
        assert_eq!(frame.items.len(), 1);
        assert_eq!(frame.buffs, vec!["Bless"]);
        assert_eq!(frame.unanswered, 1);
        assert!(frame.human_control && frame.target_cursor);
        assert_eq!(frame.sounds[0].seq, 3);
        assert_eq!(frame.sounds[0].sound, 0x023B);
        assert_eq!(frame.music, Some(9));
        assert_eq!(frame.containers[0].serial, PACK_SERIAL);
        assert_eq!(frame.containers[0].items[0].serial, 0x4000_0009);
        assert_eq!(frame.containers[0].items[0].amount, 54);
        let gump = &frame.gumps[0];
        assert_eq!(gump.gump, 77);
        assert_eq!(gump.texts, vec![(0, "Go where?".to_string())]);
        assert_eq!(gump.buttons[0].id, Some(1));
        assert_eq!(gump.buttons[1].to_page, Some(2));
        assert!(gump.choices[0].radio && gump.choices[0].on);
    }

    #[test]
    fn a_murderer_near_is_a_danger_and_a_murderer_far_is_not() {
        let murderer = |dist: u16| WatchFrame {
            hits: 50,
            hits_max: 50,
            mobiles: vec![WatchMobile {
                notoriety: NOTORIETY_MURDERER,
                dist,
                ..WatchMobile::default()
            }],
            ..WatchFrame::default()
        };
        assert_eq!(murderer(HOSTILE_NEAR_TILES).danger(), Danger::Fight);
        assert_eq!(murderer(HOSTILE_NEAR_TILES + 1).danger(), Danger::Calm);
    }

    #[test]
    fn watch_frame_keeps_self_over_dest() {
        let value = json!({
            "self_state": {
                "name": "Mara",
                "location": { "x": 10, "y": 10, "z": 0 }
            },
            "doing": { "walking_to": { "x": 10, "y": 10, "z": 0 } },
            "radar": "@"
        });
        let frame = WatchFrame::from_observe(&value);
        assert_eq!(frame.radar[0], "@");
    }
}
