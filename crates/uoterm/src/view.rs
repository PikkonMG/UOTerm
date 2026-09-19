//! A compact watch picture of one session: radar, vitals, journal.
//!
//! The window and the text loop both draw this. Tests check the picture,
//! not the OS window.

use serde_json::Value;

pub const WATCH_POLL_MS: u64 = 250;
pub const WATCH_RADAR_SIZE: u16 = 31;
pub const WINDOW_TITLE: &str = "UOTerm watch";
pub const WINDOW_WIDTH: f32 = 980.0;
pub const WINDOW_HEIGHT: f32 = 640.0;
pub const CELL_PX: f32 = 12.0;
pub const JOURNAL_LINES: usize = 12;
pub const MOBILE_LINES: usize = 12;
const MARK_NAME_CHARS: usize = 12;

const SYM_SELF: char = '@';
const SYM_MOBILE: char = 'm';
const SYM_ITEM: char = 'i';
const SYM_BLOCK: char = '#';
const SYM_WALK: char = '.';
const SYM_WATER: char = '~';
const SYM_DOOR: char = '+';
const SYM_DEST: char = 'X';

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RadarMark {
    pub name: String,
    pub dx: i32,
    pub dy: i32,
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
    pub war: bool,
    pub dead: bool,
    pub goal: String,
    pub job: String,
    pub dest_x: Option<u16>,
    pub dest_y: Option<u16>,
    pub radar: Vec<String>,
    pub journal: Vec<String>,
    pub mobiles: Vec<String>,
    pub marks: Vec<RadarMark>,
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
        let job = doing
            .and_then(|d| d.get("job"))
            .and_then(|j| j.get("name"))
            .and_then(Value::as_str)
            .or_else(|| doing.and_then(|d| d.get("job")).and_then(Value::as_str))
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
            z: pick_u16(num_opt(loc, "z"), num_opt_at(value, "z")) as i8,
            map: pick_u16(num_opt(me, "map"), num_opt_at(value, "map")) as u8,
            war: bool_field(me, "war") || bool_at(value, "war"),
            dead: bool_field(me, "dead") || bool_at(value, "dead"),
            goal: goal.to_string(),
            job: job.to_string(),
            dest_x,
            dest_y,
            radar,
            journal: journal_lines(value.get("journal")),
            mobiles: mobile_lines(mobiles, x, y),
            marks: radar_marks(mobiles, x, y),
            error: String::new(),
        }
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
            for line in self.mobiles.iter().take(MOBILE_LINES) {
                out.push_str(line);
                out.push('\n');
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

pub fn radar_color(ch: char) -> (u8, u8, u8) {
    match ch {
        SYM_SELF => (240, 220, 80),
        SYM_MOBILE => (200, 60, 50),
        SYM_ITEM => (210, 160, 60),
        SYM_BLOCK => (70, 70, 80),
        SYM_WALK => (40, 90, 45),
        SYM_WATER => (40, 80, 140),
        SYM_DOOR => (160, 110, 50),
        SYM_DEST => (80, 200, 220),
        _ => (30, 30, 35),
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

fn radar_marks(value: Option<&Value>, origin_x: u16, origin_y: u16) -> Vec<RadarMark> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|m| {
            let name = m.get("name").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                return None;
            }
            let loc = m.get("location");
            let mx = num_field(loc, "x");
            let my = num_field(loc, "y");
            let short: String = name.chars().take(MARK_NAME_CHARS).collect();
            Some(RadarMark {
                name: short,
                dx: i32::from(mx) - i32::from(origin_x),
                dy: i32::from(my) - i32::from(origin_y),
            })
        })
        .collect()
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

fn mobile_lines(value: Option<&Value>, origin_x: u16, origin_y: u16) -> Vec<String> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .map(|m| {
            let name = m.get("name").and_then(Value::as_str).unwrap_or("?");
            let dist = m.get("dist").and_then(Value::as_u64).unwrap_or_else(|| {
                let loc = m.get("location");
                let mx = num_field(loc, "x");
                let my = num_field(loc, "y");
                u64::from(mx.abs_diff(origin_x).max(my.abs_diff(origin_y)))
            });
            format!("{name} d={dist}")
        })
        .collect()
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
        assert_eq!(radar_color('@'), (240, 220, 80));
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
        assert_eq!(frame.marks.len(), 1);
        assert_eq!(frame.marks[0].name, "a zombie");
        assert_eq!(frame.marks[0].dx, 0);
        assert_eq!(frame.marks[0].dy, -1);
        assert_eq!(radar_color('X'), (80, 200, 220));
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
