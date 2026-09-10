use crate::error::{Result, RuntimeError};
use chrono::Timelike;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};

pub const TYPO_NEAR: &[(char, char)] = &[
    ('a', 's'),
    ('s', 'a'),
    ('e', 'r'),
    ('t', 'y'),
    ('n', 'm'),
    ('o', 'p'),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Persona {
    pub name: String,
    pub class: String,
    pub tier: String,
    #[serde(default)]
    pub active_hours: Vec<String>,
    #[serde(default = "default_risk")]
    pub risk_tolerance: f32,
    #[serde(default = "default_chat")]
    pub chat_rate_per_hour: u32,
    #[serde(default = "default_typo")]
    pub typo_rate: f32,
    #[serde(default)]
    pub allow_emote: bool,
    /// How the character plays along with a player who spoke to it, when
    /// the `play_along` setting is on.
    #[serde(default)]
    pub play_along: PlayAlong,
}

/// A plan a player can ask the character to join in chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// Join the player's party.
    Party,
    /// Follow the player.
    Follow,
    /// Fight what the player fights.
    Fight,
}

/// The persona's rules for playing along. A file without them gets
/// [`PlayAlong::default`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayAlong {
    /// The plans the character may say yes to.
    pub plans: Vec<Plan>,
    /// How long the character follows a player before it goes back to its
    /// own task.
    pub stay_minutes: u32,
    /// The risk tolerance while it plays along. None keeps the persona's.
    pub risk_tolerance: Option<f32>,
    /// A few words on how the character talks, for the agent that writes
    /// its lines.
    pub reply_style: String,
}

/// How long the character plays along when the persona does not say.
pub const PLAY_ALONG_STAY_MINUTES: u32 = 30;

impl Default for PlayAlong {
    fn default() -> Self {
        Self {
            plans: vec![Plan::Party, Plan::Follow],
            stay_minutes: PLAY_ALONG_STAY_MINUTES,
            risk_tolerance: None,
            reply_style: String::new(),
        }
    }
}

impl PlayAlong {
    pub fn allows(&self, plan: Plan) -> bool {
        self.plans.contains(&plan)
    }

    pub fn stay(&self) -> Duration {
        Duration::from_secs(u64::from(self.stay_minutes) * SECONDS_PER_MINUTE)
    }
}

pub const SECONDS_PER_MINUTE: u64 = 60;

fn default_risk() -> f32 {
    0.35
}
fn default_chat() -> u32 {
    8
}
fn default_typo() -> f32 {
    0.02
}

impl Default for Persona {
    fn default() -> Self {
        Self::lumberjack_yew()
    }
}

impl Persona {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).map_err(|e| RuntimeError::Config(e.to_string()))?;
        let mut persona: Self =
            toml::from_str(&text).map_err(|e| RuntimeError::Config(e.to_string()))?;
        persona.clamp_rates();
        Ok(persona)
    }

    pub fn clamp_rates(&mut self) {
        self.typo_rate = self.typo_rate.clamp(0.0, 1.0);
        self.risk_tolerance = self.risk_tolerance.clamp(0.0, 1.0);
        if let Some(risk) = self.play_along.risk_tolerance.as_mut() {
            *risk = risk.clamp(0.0, 1.0);
        }
    }

    pub fn lumberjack_yew() -> Self {
        Self {
            name: "Mara of Yew".into(),
            class: "lumberjack".into(),
            tier: "journeyman".into(),
            active_hours: vec!["14:00-18:00".into(), "21:00-23:30".into()],
            risk_tolerance: 0.35,
            chat_rate_per_hour: 8,
            typo_rate: 0.02,
            allow_emote: false,
            play_along: PlayAlong::default(),
        }
    }

    pub fn is_active_now(&self, hour: u32, minute: u32) -> bool {
        if self.active_hours.is_empty() {
            return true;
        }
        let now = hour * 60 + minute;
        self.active_hours.iter().any(|w| in_window(w, now))
    }

    pub fn hp_flee_ratio(&self) -> f32 {
        self.flee_ratio_at(self.risk_tolerance)
    }

    /// The flee ratio while the character plays along with a player.
    pub fn play_along_flee_ratio(&self) -> f32 {
        self.flee_ratio_at(
            self.play_along
                .risk_tolerance
                .unwrap_or(self.risk_tolerance),
        )
    }

    fn flee_ratio_at(&self, risk_tolerance: f32) -> f32 {
        let base = match self.tier.as_str() {
            "novice" => 0.55,
            "apprentice" => 0.45,
            "journeyman" => 0.35,
            "expert" => 0.28,
            "adept" => 0.22,
            "master" | "grandmaster" => 0.18,
            _ => 0.35,
        };
        base * (1.2 - risk_tolerance).clamp(0.7, 1.3)
    }

    pub fn filter_speech(&self, text: &str) -> Option<String> {
        let t = text.trim();
        if t.is_empty() {
            return None;
        }
        if !self.allow_emote && (t.starts_with('*') && t.ends_with('*')) {
            return None;
        }
        if t.split_whitespace().count() > 18 {
            return Some(shorten(t));
        }
        Some(t.to_string())
    }

    pub fn maybe_typo(&self, text: &str, rng: &mut impl Rng) -> String {
        if self.typo_rate <= 0.0 || !rng.gen_bool(self.typo_rate as f64) {
            return text.to_string();
        }
        let mut chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return text.to_string();
        }
        let i = rng.gen_range(0..chars.len());
        if let Some((_, to)) = TYPO_NEAR.iter().find(|(a, _)| *a == chars[i]) {
            chars[i] = *to;
        }
        chars.into_iter().collect()
    }
}

fn in_window(window: &str, now: u32) -> bool {
    let Some((a, b)) = window.split_once('-') else {
        return true;
    };
    let parse = |s: &str| {
        let mut p = s.split(':');
        let h: u32 = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let m: u32 = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        h * 60 + m
    };
    let start = parse(a);
    let end = parse(b);
    if start <= end {
        now >= start && now <= end
    } else {
        now >= start || now <= end
    }
}

fn shorten(t: &str) -> String {
    t.split_whitespace().take(8).collect::<Vec<_>>().join(" ")
}

/// How many of a character's own chat lines are remembered, so that none of
/// them is said again while it is still fresh.
pub const RECENT_LINES_KEPT: usize = 32;
/// The shortest gap between two replies to a player who spoke to the
/// character. A person needs a moment to read and type.
pub const REPLY_MIN_GAP: Duration = Duration::from_secs(REPLY_MIN_GAP_SECS);
const REPLY_MIN_GAP_SECS: u64 = 3;

#[derive(Clone, Debug, Default)]
pub struct SpeechPolicy {
    pub last_chat: Option<Instant>,
    pub chats_this_hour: u32,
    pub hour_stamp: u32,
    /// When the character last answered a player who spoke to it.
    pub last_reply: Option<Instant>,
    /// The character's latest chat lines, oldest first, in the form
    /// [`line_key`] gives them.
    recent: VecDeque<String>,
}

impl SpeechPolicy {
    pub fn allow(&mut self, persona: &Persona) -> bool {
        let hour = chrono::Local::now().hour();
        if hour != self.hour_stamp {
            self.hour_stamp = hour;
            self.chats_this_hour = 0;
        }
        if self.chats_this_hour >= persona.chat_rate_per_hour {
            return false;
        }
        if let Some(last) = self.last_chat {
            let min_gap = Duration::from_secs(3600 / persona.chat_rate_per_hour.max(1) as u64);
            if last.elapsed() < min_gap / 2 {
                return false;
            }
        }
        self.chats_this_hour += 1;
        self.last_chat = Some(Instant::now());
        true
    }

    /// True when the character may answer a player who spoke to it now.
    /// A reply is not small talk, so the persona's chat budget does not
    /// hold it; only [`REPLY_MIN_GAP`] does.
    pub fn allow_reply(&mut self) -> bool {
        if self
            .last_reply
            .is_some_and(|at| at.elapsed() < REPLY_MIN_GAP)
        {
            return false;
        }
        self.last_reply = Some(Instant::now());
        true
    }

    /// True when the character said this line lately. A player does not say
    /// the same sentence over and over; a line said again word for word is
    /// the plainest sign of a bot. Case, spacing and punctuation do not make
    /// a line new.
    pub fn said_lately(&self, line: &str) -> bool {
        let key = line_key(line);
        self.recent.iter().any(|said| *said == key)
    }

    /// Remembers a chat line the character has just said.
    pub fn remember(&mut self, line: &str) {
        if self.recent.len() == RECENT_LINES_KEPT {
            self.recent.pop_front();
        }
        self.recent.push_back(line_key(line));
    }
}

/// A line reduced to its words: lower case, no punctuation, single spaces.
fn line_key(line: &str) -> String {
    line.split_whitespace()
        .map(|word| {
            word.trim_matches(|letter: char| !letter.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Goal;

    const BASE: &str = r#"
name = "Mara of Yew"
class = "lumberjack"
tier = "journeyman"
"#;

    #[test]
    fn every_shipped_persona_loads() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../personas");
        let files: Vec<_> = std::fs::read_dir(&dir)
            .expect("the personas folder")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        assert!(!files.is_empty());
        for file in files {
            Persona::load(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        }
    }

    #[test]
    fn a_persona_without_play_along_rules_gets_the_defaults() {
        let persona: Persona = toml::from_str(BASE).expect("a persona");
        assert_eq!(persona.play_along, PlayAlong::default());
        assert!(persona.play_along.allows(Plan::Party));
        assert!(!persona.play_along.allows(Plan::Fight));
    }

    #[test]
    fn play_along_rules_are_read_and_their_risk_is_used() {
        const RISKY: f32 = 0.9;
        let text = format!(
            "{BASE}\n[play_along]\nplans = [\"follow\", \"fight\"]\nstay_minutes = 5\nrisk_tolerance = {RISKY}\nreply_style = \"short, jokes a bit\"\n"
        );
        let persona: Persona = toml::from_str(&text).expect("a persona");
        let rules = &persona.play_along;
        assert_eq!(rules.plans, vec![Plan::Follow, Plan::Fight]);
        assert_eq!(rules.stay(), Duration::from_secs(5 * SECONDS_PER_MINUTE));
        assert_eq!(rules.reply_style, "short, jokes a bit");
        assert!(
            persona.play_along_flee_ratio() < persona.hp_flee_ratio(),
            "braver beside a friend"
        );
    }

    #[test]
    fn rejects_asterisk_emotes() {
        let p = Persona::lumberjack_yew();
        assert!(p.filter_speech("*smiles*").is_none());
        assert_eq!(p.filter_speech("ty").unwrap(), "ty");
    }

    #[test]
    fn active_hours_wrap() {
        let mut p = Persona::lumberjack_yew();
        p.active_hours = vec!["22:00-02:00".into()];
        assert!(p.is_active_now(23, 0));
        assert!(p.is_active_now(1, 0));
        assert!(!p.is_active_now(12, 0));
    }

    /// The persona files this repository ships, found from the crate directory
    /// so the check works whatever the test runner's working directory is.
    fn shipped_persona_files() -> Vec<std::path::PathBuf> {
        const CRATE_TO_REPO_ROOT: &str = "../..";
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(CRATE_TO_REPO_ROOT)
            .join("personas");
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        files.sort();
        files
    }

    /// Every shipped persona parses, and no field of it is a dead letter. A
    /// class the runtime does not know falls through to `Idle`, which would
    /// silently give the character nothing to do.
    #[test]
    fn every_shipped_persona_loads_and_names_a_real_goal() {
        let files = shipped_persona_files();
        assert!(!files.is_empty(), "no persona files found");
        for path in files {
            let persona = Persona::load(&path)
                .unwrap_or_else(|e| panic!("{} does not load: {e}", path.display()));
            assert!(
                !persona.name.trim().is_empty(),
                "{} has no name",
                path.display()
            );
            assert_ne!(
                Goal::for_class(&persona.class),
                Goal::Idle,
                "{} has class {:?}, which the runtime does not know",
                path.display(),
                persona.class
            );
            assert!(
                (0.0..=1.0).contains(&persona.typo_rate),
                "{} has an out-of-range typo_rate",
                path.display()
            );
        }
    }

    #[test]
    fn typo_rate_clamps_to_unit_interval() {
        let mut p = Persona::lumberjack_yew();
        p.typo_rate = 4.0;
        p.clamp_rates();
        assert_eq!(p.typo_rate, 1.0);
        p.typo_rate = -0.5;
        p.clamp_rates();
        assert_eq!(p.typo_rate, 0.0);
    }
}
