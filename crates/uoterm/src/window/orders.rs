//! Plain orders from the human, such as "attack the orc" or "go to the
//! banker". TypeSafe's Jev model reads the order and answers two closed
//! questions: which act, and which thing near. This code then makes the one
//! tool call. Jev makes no text and runs nothing.
//!
//! The order and the names of the things near go to the TypeSafe service.
//! That happens only when the operator gave a key. Without a key the order
//! box is off, and the rest of the window works the same.

use super::control::Act;
use crate::view::WatchFrame;
use serde_json::{json, Map, Value};
use uoterm_protocol::{NOTO_ENEMY, NOTO_FRIEND, NOTO_INNOCENT, NOTO_INVULNERABLE, NOTO_MURDERER};

pub const KEY_ENV: &str = "TYPESAFE_API_KEY";
const ENV_FILE: &str = ".env";
const API_URL: &str = "https://api.typesafe.ai/v1/systemone";
const MODEL: &str = "jev-latest";
/// Under this, the window asks the human again and does nothing.
const SURE_ENOUGH: f64 = 0.6;
/// More things than this make the question long and the answer worse.
const MAX_THINGS: usize = 16;
const NO_THING: &str = "none";
const THING_PREFIX: &str = "thing_";

const NOT_UNDERSTOOD: &str = "I did not understand the order. Say it in different words.";
const WHICH_ONE: &str = "I do not know which one you mean. Say its name.";

const Q_ACT: &str = "act";
const Q_THING: &str = "thing";

/// Each act the order box knows, and the words that tell Jev what it is.
const ACTS: [(&str, &str); 10] = [
    ("go_to", "Walk to a person, a creature or an item that is near."),
    ("attack", "Fight a person or a creature that is near."),
    ("use", "Use, open or double-click an item or a person that is near, for example a door, a chest or a vendor."),
    ("follow", "Walk behind a person or a creature and stay with it."),
    ("loot", "Take the items from a corpse that is near."),
    ("stop", "Stop walking, stop following, stand still."),
    ("war_on", "Go into war mode, draw the weapon, get ready to fight."),
    ("war_off", "Go into peace mode, put the weapon away."),
    ("deposit", "Put the items of the backpack in the bank box."),
    (NO_THING, "The order asks for something that is not in this list."),
];
const ACTS_WITH_A_THING: [&str; 5] = ["go_to", "attack", "use", "follow", "loot"];

/// One thing near that an order can name.
#[derive(Clone, Debug, PartialEq)]
struct Thing {
    serial: u32,
    words: String,
    x: u16,
    y: u16,
}

/// Players name others by the color of their name, so the color is here too.
fn notoriety_word(notoriety: u8) -> &'static str {
    match notoriety {
        NOTO_INNOCENT => "innocent, blue name",
        NOTO_FRIEND => "friend, green name",
        NOTO_ENEMY => "enemy, orange name",
        NOTO_MURDERER => "murderer, red name",
        NOTO_INVULNERABLE => "cannot be hurt, yellow name",
        _ => "grey name",
    }
}

/// The mobiles and then the ground items, nearest first in each group.
fn things(frame: &WatchFrame) -> Vec<Thing> {
    let mobiles = frame.mobiles.iter().map(|m| Thing {
        serial: m.serial,
        words: format!(
            "{} {}, {}, {} tiles away",
            m.name,
            m.title,
            notoriety_word(m.notoriety),
            m.dist
        ),
        x: m.x,
        y: m.y,
    });
    let items = frame
        .items
        .iter()
        .filter(|item| !item.name.is_empty())
        .map(|item| Thing {
            serial: item.serial,
            words: format!(
                "{}, an item on the ground, {} tiles away",
                item.name,
                item.x.abs_diff(frame.x).max(item.y.abs_diff(frame.y))
            ),
            x: item.x,
            y: item.y,
        });
    mobiles.chain(items).take(MAX_THINGS).collect()
}

/// The request for one order: the state Jev reads, and the two questions.
pub fn request(order: &str, frame: &WatchFrame) -> Value {
    let things = things(frame);
    let mut thing_options = Map::new();
    for (i, thing) in things.iter().enumerate() {
        thing_options.insert(format!("{THING_PREFIX}{i}"), json!(thing.words));
    }
    thing_options.insert(
        NO_THING.into(),
        json!("The order names no thing, or it names a thing that is not in this list."),
    );
    let acts: Map<String, Value> = ACTS
        .iter()
        .map(|(act, words)| ((*act).to_string(), json!(words)))
        .collect();
    json!({
        "model": MODEL,
        "state": {
            "order": order,
            "character": { "name": frame.name, "in_war_mode": frame.war },
            "things_near": things.iter().map(|t| &t.words).collect::<Vec<_>>(),
        },
        "questions": {
            Q_ACT: {
                "type": "choice",
                "instructions": "A player of an online role-playing game gives `order` to the character. Which one act does the order ask for?",
                "criteria": acts,
            },
            Q_THING: {
                "type": "choice",
                "instructions": "Which one of the things near does `order` tell the character to act on?",
                "criteria": thing_options,
            },
        }
    })
}

fn sure_choice<'a>(answers: &'a Value, question: &str) -> Option<&'a str> {
    let answer = answers.get(question)?;
    let sure = answer.get("confidence").and_then(Value::as_f64)? >= SURE_ENOUGH;
    answer
        .get("choice")
        .and_then(Value::as_str)
        .filter(|_| sure)
}

/// The tool call for Jev's answer, or the words to show the human when the
/// answer is not sure enough to act on.
pub fn decide(response: &Value, frame: &WatchFrame) -> Result<Act, String> {
    let answers = response.get("answers").unwrap_or(&Value::Null);
    let act = sure_choice(answers, Q_ACT)
        .filter(|act| *act != NO_THING)
        .ok_or(NOT_UNDERSTOOD)?;
    let thing = || -> Result<Thing, String> {
        let index: usize = sure_choice(answers, Q_THING)
            .and_then(|key| key.strip_prefix(THING_PREFIX))
            .and_then(|n| n.parse().ok())
            .ok_or(WHICH_ONE)?;
        things(frame).get(index).cloned().ok_or(WHICH_ONE.into())
    };
    let thing = if ACTS_WITH_A_THING.contains(&act) {
        Some(thing()?)
    } else {
        None
    };
    Ok(match (act, thing) {
        ("go_to", Some(t)) => Act::WalkTo { x: t.x, y: t.y },
        ("attack", Some(t)) => Act::Attack(t.serial),
        ("use", Some(t)) => Act::Use(t.serial),
        ("follow", Some(t)) => Act::Follow(t.serial),
        ("loot", Some(t)) => Act::Loot(t.serial),
        ("war_on", _) => Act::War(true),
        ("war_off", _) => Act::War(false),
        ("deposit", _) => Act::Deposit,
        _ => Act::Stop,
    })
}

/// The key from the environment, or from a `.env` file in this directory.
pub fn api_key() -> Option<String> {
    let from_file = || {
        let text = std::fs::read_to_string(ENV_FILE).ok()?;
        text.lines().find_map(|line| {
            let value = line
                .trim()
                .strip_prefix(KEY_ENV)?
                .trim()
                .strip_prefix('=')?;
            Some(value.trim().trim_matches(['"', '\'']).to_string())
        })
    };
    std::env::var(KEY_ENV)
        .ok()
        .or_else(from_file)
        .filter(|key| !key.is_empty())
}

/// Asks Jev about one order. The error is words for the human.
pub async fn ask(key: &str, order: &str, frame: &WatchFrame) -> Result<Act, String> {
    let response = reqwest::Client::new()
        .post(API_URL)
        .bearer_auth(key)
        .json(&request(order, frame))
        .send()
        .await
        .map_err(|e| format!("TypeSafe did not answer: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("TypeSafe refused the order: HTTP {status}"));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("TypeSafe gave a bad answer: {e}"))?;
    decide(&body, frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchMobile;

    const ORC: u32 = 9;

    fn frame() -> WatchFrame {
        WatchFrame {
            name: "Mara".into(),
            x: 100,
            y: 100,
            mobiles: vec![
                WatchMobile {
                    serial: 4,
                    name: "Kehinde".into(),
                    title: "the banker".into(),
                    x: 103,
                    y: 100,
                    notoriety: 1,
                    dist: 3,
                    ..WatchMobile::default()
                },
                WatchMobile {
                    serial: ORC,
                    name: "an orc".into(),
                    x: 95,
                    y: 100,
                    notoriety: 6,
                    dist: 5,
                    ..WatchMobile::default()
                },
            ],
            ..WatchFrame::default()
        }
    }

    fn answer(act: &str, act_sure: f64, thing: &str, thing_sure: f64) -> Value {
        json!({ "answers": {
            Q_ACT: { "type": "choice", "choice": act, "confidence": act_sure },
            Q_THING: { "type": "choice", "choice": thing, "confidence": thing_sure },
        }})
    }

    #[test]
    fn the_request_names_each_thing_near_and_a_way_out() {
        let asked = request("kill the orc", &frame());
        let options = &asked["questions"][Q_THING]["criteria"];
        assert!(options["thing_1"].as_str().unwrap().contains("an orc"));
        assert!(options["thing_1"].as_str().unwrap().contains("murderer"));
        assert!(options.get(NO_THING).is_some());
        assert!(asked["questions"][Q_ACT]["criteria"]
            .get("attack")
            .is_some());
        assert_eq!(asked["state"]["order"], "kill the orc");
    }

    #[test]
    fn a_sure_answer_is_one_tool_call() {
        let act = decide(&answer("attack", 0.97, "thing_1", 0.9), &frame());
        assert_eq!(act, Ok(Act::Attack(ORC)));
        let walk = decide(&answer("go_to", 0.9, "thing_0", 0.8), &frame());
        assert_eq!(walk, Ok(Act::WalkTo { x: 103, y: 100 }));
        let stop = decide(&answer("stop", 0.9, NO_THING, 0.3), &frame());
        assert_eq!(stop, Ok(Act::Stop));
    }

    #[test]
    fn an_unsure_answer_does_nothing() {
        let unsure = decide(&answer("attack", 0.4, "thing_1", 0.9), &frame());
        assert_eq!(unsure, Err(NOT_UNDERSTOOD.into()));
        let no_thing = decide(&answer("attack", 0.9, NO_THING, 0.9), &frame());
        assert_eq!(no_thing, Err(WHICH_ONE.into()));
        let other = decide(&answer(NO_THING, 0.9, NO_THING, 0.9), &frame());
        assert_eq!(other, Err(NOT_UNDERSTOOD.into()));
    }
}
