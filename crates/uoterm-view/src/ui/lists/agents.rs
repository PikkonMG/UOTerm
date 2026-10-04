//! The rows of the agent windows, as data: the list of the windows with
//! the job that runs, the switches, numbers, flags and lists of each
//! agent, and the ignore list of the profile. Each window draws the rows
//! in order; a press on a button, a picture or a field does what its
//! `AgentPress` says. Acts go to the character only while the human has
//! control.

use super::super::deck::layer_words;
use super::super::launch::IGNORE_ID;
use super::super::theme::{ALARM, GOAL, TEXT, TEXT_DIM, TEXT_FAINT};
use crate::act::Act;
use crate::frame::{WatchFrame, WatchPackItem};
use crate::geom::Rgba;
use crate::model::agents::{
    set_field, AgentPanel, AgentsView, Lists, RuleRow, AUTOLOOT, BANDAGE, BANDAGE_WHOM,
    BONE_CUTTER, CARVER, DRESS, FRIENDS, HEAL_SPELLS, KEY_ACTIVE, KEY_BLADE, KEY_DESTINATION,
    KEY_HEAL_SPELL, KEY_ITEMS, KEY_LAYER, KEY_MOUNT, KEY_SERIAL, KEY_SOURCE, KEY_WHOM, ORGANIZER,
    REMOUNT, SELF_HEAL, UNDRESS,
};
use crate::model::counters;
use crate::model::durability::is_worn_layer;
use crate::model::places;
use crate::settings::Profile;
use serde_json::{json, Value};

/// The width of a button at the right of a row, and of a small one.
pub const AGENT_BUTTON_WIDTH: f32 = 64.0;
pub const AGENT_SMALL_BUTTON_WIDTH: f32 = 30.0;
/// The field of a new list is wider than a button by this share.
const NEW_LIST_WIDTH_SHARE: f32 = 1.5;
/// A mount is picked from the mobiles this near, a friend from farther.
const MOUNT_TILES: u16 = 3;
const FRIEND_TILES: u16 = 12;

pub const WORDS_IGNORE: &str = "Ignore list";
const WORDS_OPEN: &str = "Open";
const WORDS_CLOSE: &str = "Close";
const WORDS_ON: &str = "on";
const WORDS_OFF: &str = "off";
const WORDS_TURN_ON: &str = "Turn on";
const WORDS_TURN_OFF: &str = "Turn off";
const WORDS_LESS: &str = "-";
const WORDS_MORE: &str = "+";
const WORDS_REMOVE: &str = "x";
const WORDS_ADD: &str = "Add";
const WORDS_PICK: &str = "Use";
const WORDS_JOB: &str = "Job running:";
const WORDS_NO_JOB: &str = "No job runs.";
const WORDS_STOP: &str = "Stop";
const WORDS_RUN: &str = "Run";
const WORDS_DRESS: &str = "Dress";
const WORDS_UNDRESS: &str = "Undress";
const WORDS_LOOT_NOW: &str = "Loot corpses in reach";
const WORDS_LISTS: &str = "Lists (the one in use is bright):";
const WORDS_ADD_FROM_BAG: &str = "Add from your bag:";
const WORDS_NEW_LIST: &str = "Add list";
const WORDS_SAVE_WORN: &str = "Save worn";
const HINT_LIST_NAME: &str = "new list name";
const HINT_IGNORE_NAME: &str = "a name to ignore";
const WORDS_HEAL_WHOM: &str = "Heal:";
const WORDS_SPELL: &str = "Spell:";
const WORDS_MOUNT: &str = "Mount:";
const WORDS_BLADE: &str = "Blade:";
const WORDS_NONE_SET: &str = "none";
const WORDS_TO: &str = "Into:";
const WORDS_FROM: &str = "From:";
const WORDS_THE_PACK: &str = "the pack";
const WORDS_NEAR: &str = "Near you:";

/// What a press on a part of a row does.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentPress {
    /// Acts on the character's side, while the human has control.
    Acts(Vec<Act>),
    /// Opens or closes the panel of this place id.
    Open(String, bool),
    /// Shows the job list of this name in the window.
    ChooseList(String),
    /// Takes the name at this place off the ignore list.
    Unignore(usize),
    /// Puts a name on the ignore list.
    Ignore(String),
}

/// One button of a row: its words, its color and its press.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentButton {
    pub words: String,
    pub color: Rgba,
    pub press: AgentPress,
}

/// What a field of a row makes of the words typed in it.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldUse {
    /// A new list of this runtime agent, made with these settings.
    NewList {
        agent: &'static str,
        settings: Value,
    },
    /// A name for the ignore list.
    IgnoreName,
}

/// One row of an agent window. `part` and `at` tell the rows apart.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentRow {
    /// Words on a row of their own.
    Words { words: String, color: Rgba },
    /// Words on the left, and buttons of `width` on the right.
    Labeled {
        part: &'static str,
        at: usize,
        words: String,
        buttons: Vec<AgentButton>,
        width: f32,
    },
    /// A row of buttons of equal width.
    Buttons {
        part: &'static str,
        buttons: Vec<AgentButton>,
    },
    /// The pictures of the items of `agent_bag_items`; a click on the one
    /// at a place does the press at that place.
    Pictures {
        part: &'static str,
        presses: Vec<AgentPress>,
    },
    /// A field of words and its button of `width`.
    Field {
        part: &'static str,
        hint: &'static str,
        button: &'static str,
        width: f32,
        field_use: FieldUse,
    },
}

fn button(words: &str, color: Rgba, press: AgentPress) -> AgentButton {
    AgentButton {
        words: words.to_string(),
        color,
        press,
    }
}

fn acts(acts: impl IntoIterator<Item = Act>) -> AgentPress {
    AgentPress::Acts(acts.into_iter().collect())
}

fn words(words: impl Into<String>, color: Rgba) -> AgentRow {
    AgentRow::Words {
        words: words.into(),
        color,
    }
}

fn labeled(
    part: &'static str,
    at: usize,
    words: impl Into<String>,
    buttons: Vec<AgentButton>,
    width: f32,
) -> AgentRow {
    AgentRow::Labeled {
        part,
        at,
        words: words.into(),
        buttons,
        width,
    }
}

/// The items of the backpack and its open bags, one of each graphic and
/// hue.
pub fn agent_bag_items(frame: &WatchFrame) -> Vec<WatchPackItem> {
    let mut seen: Vec<WatchPackItem> = Vec::new();
    for item in counters::backpack_containers(frame)
        .into_iter()
        .flat_map(|bag| bag.items.iter())
    {
        if !seen
            .iter()
            .any(|kept| kept.graphic == item.graphic && kept.hue == item.hue)
        {
            seen.push(item.clone());
        }
    }
    seen
}

/// The name of a thing by its serial: a mobile or an item the frame knows,
/// else its number.
pub fn agent_name_of(frame: &WatchFrame, serial: u32) -> String {
    frame
        .mobiles
        .iter()
        .find(|m| m.serial == serial)
        .map(|m| m.name.clone())
        .or_else(|| {
            frame
                .containers
                .iter()
                .flat_map(|c| c.items.iter())
                .find(|i| i.serial == serial)
                .map(|i| i.name.clone())
        })
        .or_else(|| {
            frame
                .containers
                .iter()
                .find(|c| c.serial == serial)
                .map(|c| c.name.clone())
        })
        .unwrap_or_else(|| format!("0x{serial:08X}"))
}

/// A dress list of what the character wears now.
pub fn agent_dress_of_worn(frame: &WatchFrame) -> Value {
    let items: Vec<Value> = frame
        .look
        .equipment
        .iter()
        .filter(|item| is_worn_layer(item.layer))
        .map(|item| json!({ KEY_LAYER: item.layer, KEY_SERIAL: item.serial }))
        .collect();
    json!({ KEY_ITEMS: items })
}

fn on_words(on: bool) -> &'static str {
    if on {
        WORDS_ON
    } else {
        WORDS_OFF
    }
}

fn chosen_color(chosen: bool) -> Rgba {
    if chosen {
        GOAL
    } else {
        TEXT_DIM
    }
}

fn open_words(open: bool) -> &'static str {
    if open {
        WORDS_CLOSE
    } else {
        WORDS_OPEN
    }
}

/// The rows of the list of the agent windows: the job that runs, each
/// window the Agents page offers, and the ignore list.
pub fn agent_chooser_rows(view: Option<&AgentsView>, profile: &Profile) -> Vec<AgentRow> {
    let mut rows = vec![match view.and_then(AgentsView::job) {
        Some(job) => labeled(
            "job",
            0,
            format!("{WORDS_JOB} {job}"),
            vec![button(WORDS_STOP, ALARM, acts([Act::AgentStop]))],
            AGENT_BUTTON_WIDTH,
        ),
        None => words(WORDS_NO_JOB, TEXT_FAINT),
    }];
    let offered = AgentPanel::ALL
        .into_iter()
        .filter(|panel| panel.offered(&profile.agents));
    for (at, agent) in offered.enumerate() {
        let id = agent.place_id();
        let open = places::is_open(profile, &id);
        let on = view.is_some_and(|view| agent.switches().iter().any(|name| view.is_on(name)));
        let title = if agent.switches().is_empty() {
            agent.title().to_string()
        } else {
            format!("{} ({})", agent.title(), on_words(on))
        };
        let press = AgentPress::Open(id, !open);
        rows.push(labeled(
            "agent",
            at,
            title,
            vec![button(open_words(open), TEXT, press)],
            AGENT_BUTTON_WIDTH,
        ));
    }
    let open = places::is_open(profile, IGNORE_ID);
    let press = AgentPress::Open(IGNORE_ID.to_string(), !open);
    rows.push(labeled(
        "ignore",
        0,
        WORDS_IGNORE,
        vec![button(open_words(open), TEXT, press)],
        AGENT_BUTTON_WIDTH,
    ));
    rows
}

/// The switches, numbers, flags and the rows of each agent's own. `bag`
/// is `agent_bag_items`.
pub fn agent_settings_rows(
    agent: AgentPanel,
    view: &AgentsView,
    frame: &WatchFrame,
    bag: &[WatchPackItem],
) -> Vec<AgentRow> {
    let mut rows = Vec::new();
    for (at, name) in agent.switches().iter().enumerate() {
        let on = view.is_on(name);
        let turn = if on { WORDS_TURN_OFF } else { WORDS_TURN_ON };
        let press = acts([Act::AgentOn {
            agent: (*name).to_string(),
            on: !on,
        }]);
        rows.push(labeled(
            "switch",
            at,
            format!("{name}: {}", on_words(on)),
            vec![button(turn, chosen_color(!on), press)],
            AGENT_BUTTON_WIDTH,
        ));
    }
    if let Some(name) = agent.settings_agent() {
        for (at, (key, words, step)) in agent.numbers().iter().enumerate() {
            let value = view.number(name, key).unwrap_or_default();
            let less = acts([set_field(
                view,
                name,
                key,
                json!(value.saturating_sub(*step)),
            )]);
            let more = acts([set_field(view, name, key, json!(value + step))]);
            rows.push(labeled(
                "number",
                at,
                format!("{words}: {value}"),
                vec![
                    button(WORDS_LESS, TEXT, less),
                    button(WORDS_MORE, TEXT, more),
                ],
                AGENT_SMALL_BUTTON_WIDTH,
            ));
        }
        for (at, (key, words)) in agent.flags().iter().enumerate() {
            let on = view.flag(name, key);
            let press = acts([set_field(view, name, key, json!(!on))]);
            rows.push(labeled(
                "flag",
                at,
                *words,
                vec![button(on_words(on), chosen_color(on), press)],
                AGENT_BUTTON_WIDTH,
            ));
        }
    }
    match agent {
        AgentPanel::Bandage => {
            rows.push(words(WORDS_HEAL_WHOM, TEXT_DIM));
            let whom = view
                .settings(BANDAGE)
                .and_then(|s| s.get(KEY_WHOM))
                .and_then(Value::as_str);
            let buttons = BANDAGE_WHOM
                .iter()
                .map(|(word, words)| {
                    let press = acts([set_field(view, BANDAGE, KEY_WHOM, json!(word))]);
                    button(words, chosen_color(whom == Some(*word)), press)
                })
                .collect();
            rows.push(AgentRow::Buttons {
                part: "whom",
                buttons,
            });
        }
        AgentPanel::SelfHeal => {
            rows.push(words(WORDS_SPELL, TEXT_DIM));
            let spell = view.number(SELF_HEAL, KEY_HEAL_SPELL);
            let buttons = HEAL_SPELLS
                .iter()
                .map(|(id, words)| {
                    let press = acts([set_field(view, SELF_HEAL, KEY_HEAL_SPELL, json!(id))]);
                    button(words, chosen_color(spell == Some(u64::from(*id))), press)
                })
                .collect();
            rows.push(AgentRow::Buttons {
                part: "spell",
                buttons,
            });
        }
        AgentPanel::Loot => {
            let press = acts([Act::AgentRun {
                agent: AUTOLOOT.to_string(),
                list: None,
            }]);
            rows.push(AgentRow::Buttons {
                part: "loot-now",
                buttons: vec![button(WORDS_LOOT_NOW, GOAL, press)],
            });
        }
        AgentPanel::Remount => {
            let mount = view
                .number(REMOUNT, KEY_MOUNT)
                .map(|serial| agent_name_of(frame, serial as u32));
            rows.push(words(
                format!(
                    "{WORDS_MOUNT} {}",
                    mount.as_deref().unwrap_or(WORDS_NONE_SET)
                ),
                TEXT,
            ));
            rows.push(words(WORDS_NEAR, TEXT_DIM));
            let near = frame.mobiles.iter().filter(|m| m.dist <= MOUNT_TILES);
            for (at, mobile) in near.enumerate() {
                let press = acts([set_field(view, REMOUNT, KEY_MOUNT, json!(mobile.serial))]);
                rows.push(labeled(
                    "mount",
                    at,
                    mobile.name.clone(),
                    vec![button(WORDS_PICK, TEXT, press)],
                    AGENT_BUTTON_WIDTH,
                ));
            }
            rows.push(words(WORDS_ADD_FROM_BAG, TEXT_DIM));
            let presses = bag
                .iter()
                .map(|item| acts([set_field(view, REMOUNT, KEY_MOUNT, json!(item.serial))]))
                .collect();
            rows.push(AgentRow::Pictures {
                part: "mount-item",
                presses,
            });
        }
        AgentPanel::Skinning => {
            let blade = view
                .number(CARVER, KEY_BLADE)
                .map(|serial| agent_name_of(frame, serial as u32));
            rows.push(words(
                format!(
                    "{WORDS_BLADE} {}",
                    blade.as_deref().unwrap_or(WORDS_NONE_SET)
                ),
                TEXT,
            ));
            rows.push(words(WORDS_ADD_FROM_BAG, TEXT_DIM));
            let presses = bag
                .iter()
                .map(|item| {
                    acts(
                        [CARVER, BONE_CUTTER]
                            .map(|name| set_field(view, name, KEY_BLADE, json!(item.serial))),
                    )
                })
                .collect();
            rows.push(AgentRow::Pictures {
                part: "blade",
                presses,
            });
        }
        AgentPanel::Friends => {
            let friends = view.friends();
            for (at, friend) in friends.iter().enumerate() {
                let press = acts([Act::AgentSet {
                    agent: FRIENDS.to_string(),
                    list: None,
                    settings: view.friends_toggled(*friend),
                }]);
                rows.push(labeled(
                    "friend",
                    at,
                    agent_name_of(frame, *friend),
                    vec![button(WORDS_REMOVE, ALARM, press)],
                    AGENT_SMALL_BUTTON_WIDTH,
                ));
            }
            rows.push(words(WORDS_NEAR, TEXT_DIM));
            let near = frame
                .mobiles
                .iter()
                .filter(|m| m.dist <= FRIEND_TILES && !friends.contains(&m.serial));
            for (at, mobile) in near.enumerate() {
                let press = acts([Act::AgentSet {
                    agent: FRIENDS.to_string(),
                    list: None,
                    settings: view.friends_toggled(mobile.serial),
                }]);
                rows.push(labeled(
                    "near",
                    at,
                    mobile.name.clone(),
                    vec![button(WORDS_ADD, TEXT, press)],
                    AGENT_BUTTON_WIDTH,
                ));
            }
        }
        _ => {}
    }
    rows
}

/// The lists of an agent that keeps them: the names, the items of the
/// list in use or of `chosen`, the job list the window shows, and the
/// field of a new list. `bag` is `agent_bag_items`.
pub fn agent_list_rows(
    agent: AgentPanel,
    view: &AgentsView,
    frame: &WatchFrame,
    chosen: Option<&str>,
    bag: &[WatchPackItem],
) -> Vec<AgentRow> {
    let lists = agent.lists();
    let names = view.list_names(lists);
    let (name, active) = match lists {
        Lists::None => return Vec::new(),
        Lists::Items(name) => (name, view.active_list(name)),
        Lists::Jobs(name) => {
            let shown = chosen
                .filter(|chosen| names.iter().any(|name| name == chosen))
                .map(str::to_string)
                .or_else(|| names.first().cloned())
                .unwrap_or_default();
            (name, shown)
        }
    };
    let mut rows = vec![words(WORDS_LISTS, TEXT_DIM)];
    let buttons = names
        .iter()
        .map(|list| {
            let press = match lists {
                Lists::Items(_) => acts([set_field(view, name, KEY_ACTIVE, json!(list))]),
                _ => AgentPress::ChooseList(list.clone()),
            };
            button(list, chosen_color(*list == active), press)
        })
        .collect();
    rows.push(AgentRow::Buttons {
        part: "lists",
        buttons,
    });
    if !active.is_empty() {
        rows.extend(chosen_rows(agent, view, frame, &active, bag));
    }
    let (words, settings) = match agent {
        AgentPanel::Dress => (WORDS_SAVE_WORN, agent_dress_of_worn(frame)),
        AgentPanel::Organizer => (WORDS_NEW_LIST, json!({ KEY_ITEMS: [] })),
        _ => (WORDS_NEW_LIST, json!([])),
    };
    rows.push(AgentRow::Field {
        part: "new-list",
        hint: HINT_LIST_NAME,
        button: words,
        width: AGENT_BUTTON_WIDTH * NEW_LIST_WIDTH_SHARE,
        field_use: FieldUse::NewList {
            agent: name,
            settings,
        },
    });
    rows
}

/// The items of the list in use or chosen, and its buttons.
fn chosen_rows(
    agent: AgentPanel,
    view: &AgentsView,
    frame: &WatchFrame,
    list: &str,
    bag: &[WatchPackItem],
) -> Vec<AgentRow> {
    let lists = agent.lists();
    let (Lists::Items(name) | Lists::Jobs(name)) = lists else {
        return Vec::new();
    };
    let set_list = |settings: Value| Act::AgentSet {
        agent: name.to_string(),
        list: Some(list.to_string()),
        settings,
    };
    let run = |agent: &str| Act::AgentRun {
        agent: agent.to_string(),
        list: Some(list.to_string()),
    };
    let mut rows = Vec::new();
    if agent == AgentPanel::Dress {
        let items = view
            .settings(DRESS)
            .and_then(|s| s.get(list))
            .and_then(|l| l.get(KEY_ITEMS))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (at, item) in items.iter().enumerate() {
            let layer = item
                .get(KEY_LAYER)
                .and_then(Value::as_u64)
                .unwrap_or_default() as u8;
            let serial = item
                .get(KEY_SERIAL)
                .and_then(Value::as_u64)
                .unwrap_or_default() as u32;
            let press = acts([set_list(view.list_without(lists, list, at))]);
            rows.push(labeled(
                "dress-item",
                at,
                format!("{}: {}", layer_words(layer), agent_name_of(frame, serial)),
                vec![button(WORDS_REMOVE, ALARM, press)],
                AGENT_SMALL_BUTTON_WIDTH,
            ));
        }
        rows.push(AgentRow::Buttons {
            part: "dress-run",
            buttons: vec![
                button(WORDS_DRESS, GOAL, acts([run(DRESS)])),
                button(WORDS_UNDRESS, TEXT, acts([run(UNDRESS)])),
                button(WORDS_STOP, ALARM, acts([Act::AgentStop])),
            ],
        });
        return rows;
    }
    for (at, rule) in view.rules(lists, list).iter().enumerate() {
        let words = match (rule.name.is_empty(), rule.graphic, rule.amount) {
            (false, _, Some(amount)) => format!("{} x{amount}", rule.name),
            (false, _, None) => rule.name.clone(),
            (true, Some(graphic), _) => format!("0x{graphic:04X}"),
            (true, None, _) => WORDS_NONE_SET.to_string(),
        };
        let press = acts([set_list(view.list_without(lists, list, at))]);
        rows.push(labeled(
            "rule",
            at,
            words,
            vec![button(WORDS_REMOVE, ALARM, press)],
            AGENT_SMALL_BUTTON_WIDTH,
        ));
    }
    rows.push(words(WORDS_ADD_FROM_BAG, TEXT_DIM));
    let presses = bag
        .iter()
        .map(|item| {
            let rule = RuleRow {
                name: item.name.clone(),
                graphic: Some(item.graphic),
                color: (item.hue != 0).then_some(item.hue),
                amount: None,
            };
            acts([set_list(view.list_with(lists, list, rule.to_value()))])
        })
        .collect();
    rows.push(AgentRow::Pictures {
        part: "rule-add",
        presses,
    });
    if agent == AgentPanel::Organizer {
        for (part, key, words_now) in [
            ("from", KEY_SOURCE, WORDS_FROM),
            ("to", KEY_DESTINATION, WORDS_TO),
        ] {
            let bag_now = view.job_list_number(ORGANIZER, list, key).map_or_else(
                || WORDS_THE_PACK.to_string(),
                |serial| agent_name_of(frame, serial as u32),
            );
            rows.push(words(format!("{words_now} {bag_now}"), TEXT));
            let buttons = frame
                .containers
                .iter()
                .map(|container| {
                    let settings =
                        view.job_list_with(ORGANIZER, list, key, json!(container.serial));
                    button(&container.name, TEXT_DIM, acts([set_list(settings)]))
                })
                .collect();
            rows.push(AgentRow::Buttons { part, buttons });
        }
        rows.push(AgentRow::Buttons {
            part: "organize",
            buttons: vec![
                button(WORDS_RUN, GOAL, acts([run(ORGANIZER)])),
                button(WORDS_STOP, ALARM, acts([Act::AgentStop])),
            ],
        });
    }
    rows
}

/// The rows of the ignore list: each name with its remove button, the
/// field of a new name, and the people near.
pub fn ignore_rows(names: &[String], frame: &WatchFrame) -> Vec<AgentRow> {
    let mut rows: Vec<AgentRow> = names
        .iter()
        .enumerate()
        .map(|(at, name)| {
            let press = AgentPress::Unignore(at);
            labeled(
                "name",
                at,
                name.clone(),
                vec![button(WORDS_REMOVE, ALARM, press)],
                AGENT_SMALL_BUTTON_WIDTH,
            )
        })
        .collect();
    rows.push(AgentRow::Field {
        part: "typed",
        hint: HINT_IGNORE_NAME,
        button: WORDS_ADD,
        width: AGENT_BUTTON_WIDTH,
        field_use: FieldUse::IgnoreName,
    });
    rows.push(words(WORDS_NEAR, TEXT_DIM));
    let near = frame
        .mobiles
        .iter()
        .filter(|m| m.dist <= FRIEND_TILES && !names.contains(&m.name));
    for (at, mobile) in near.enumerate() {
        let press = AgentPress::Ignore(mobile.name.clone());
        rows.push(labeled(
            "near",
            at,
            mobile.name.clone(),
            vec![button(WORDS_ADD, TEXT, press)],
            AGENT_BUTTON_WIDTH,
        ));
    }
    rows
}

/// What the words typed in a field do, once its button or Enter was
/// pressed. Nothing for blank words.
pub fn field_presses(field_use: &FieldUse, typed: &str) -> Vec<AgentPress> {
    let name = typed.trim();
    if name.is_empty() {
        return Vec::new();
    }
    match field_use {
        FieldUse::NewList { agent, settings } => vec![
            acts([Act::AgentSet {
                agent: (*agent).to_string(),
                list: Some(name.to_string()),
                settings: settings.clone(),
            }]),
            AgentPress::ChooseList(name.to_string()),
        ],
        FieldUse::IgnoreName => vec![AgentPress::Ignore(name.to_string())],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchContainer, WatchEquip, WatchLook, WatchMobile};
    use uoterm_protocol::types::LAYER_BACKPACK;

    const BAG: u32 = 0x4000_0100;

    fn buttons_of(row: &AgentRow) -> &[AgentButton] {
        match row {
            AgentRow::Labeled { buttons, .. } | AgentRow::Buttons { buttons, .. } => buttons,
            _ => &[],
        }
    }

    #[test]
    fn a_switch_turns_its_agent_the_other_way_and_a_number_steps() {
        let view = AgentsView::default();
        let bag = Vec::new();
        let rows = agent_settings_rows(AgentPanel::Loot, &view, &WatchFrame::default(), &bag);
        let switch = &buttons_of(&rows[0])[0];
        assert_eq!(
            switch.press,
            AgentPress::Acts(vec![Act::AgentOn {
                agent: AUTOLOOT.to_string(),
                on: true,
            }])
        );
        let number = buttons_of(&rows[1]);
        assert_eq!(number.len(), 2, "less and more");
        assert!(rows.iter().any(|row| matches!(
            row,
            AgentRow::Buttons {
                part: "loot-now",
                ..
            }
        )));
    }

    #[test]
    fn the_chooser_opens_a_window_and_tells_that_no_job_runs() {
        let profile = Profile::default();
        let rows = agent_chooser_rows(None, &profile);
        assert_eq!(rows[0], words(WORDS_NO_JOB, TEXT_FAINT));
        let last = buttons_of(rows.last().unwrap());
        assert_eq!(last[0].press, AgentPress::Open(IGNORE_ID.to_string(), true));
    }

    #[test]
    fn a_new_list_needs_a_name_and_shows_once_made() {
        let field = FieldUse::NewList {
            agent: DRESS,
            settings: json!([]),
        };
        assert!(field_presses(&field, "  ").is_empty());
        let presses = field_presses(&field, " Night ");
        assert_eq!(presses[1], AgentPress::ChooseList("Night".to_string()));
        assert_eq!(
            field_presses(&FieldUse::IgnoreName, "Bob"),
            vec![AgentPress::Ignore("Bob".to_string())]
        );
    }

    #[test]
    fn the_ignore_list_offers_the_people_near_who_are_not_on_it() {
        let frame = WatchFrame {
            mobiles: vec![
                WatchMobile {
                    name: "Bob".into(),
                    dist: 2,
                    ..WatchMobile::default()
                },
                WatchMobile {
                    name: "Ann".into(),
                    dist: 2,
                    ..WatchMobile::default()
                },
            ],
            ..WatchFrame::default()
        };
        let rows = ignore_rows(&["Bob".to_string()], &frame);
        let offered: Vec<&AgentPress> = rows
            .iter()
            .filter(|row| matches!(row, AgentRow::Labeled { part: "near", .. }))
            .map(|row| &buttons_of(row)[0].press)
            .collect();
        assert_eq!(offered, vec![&AgentPress::Ignore("Ann".to_string())]);
    }

    #[test]
    fn the_bag_offers_each_kind_once_and_the_dress_list_is_what_is_worn() {
        let item = |serial| WatchPackItem {
            serial,
            graphic: 0x0F3F,
            ..WatchPackItem::default()
        };
        let frame = WatchFrame {
            look: WatchLook {
                equipment: vec![
                    WatchEquip {
                        serial: BAG,
                        layer: LAYER_BACKPACK,
                        ..WatchEquip::default()
                    },
                    WatchEquip {
                        serial: 9,
                        layer: 1,
                        ..WatchEquip::default()
                    },
                ],
                ..WatchLook::default()
            },
            containers: vec![WatchContainer {
                serial: BAG,
                items: vec![item(1), item(2)],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        assert_eq!(agent_bag_items(&frame).len(), 1);
        assert_eq!(agent_name_of(&frame, 0x0BAD), "0x00000BAD");
        assert_eq!(
            agent_dress_of_worn(&frame),
            json!({ KEY_ITEMS: [{ KEY_LAYER: 1, KEY_SERIAL: 9 }] })
        );
    }
}
