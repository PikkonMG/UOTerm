//! Hotkeys: named actions a person or an agent calls by name.
//!
//! A client with no keyboard has no keys to bind, so a hotkey is its name.
//! Most hotkeys run one or two script lines at once, which keeps them and
//! scripts the same. The list grows with the character: a hotkey for every
//! spell, every usable skill, every agent list, target filter and script.

use uoterm_assist::items::POTIONS;
use uoterm_assist::name_key;

use super::*;

/// What a hotkey does.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    /// Script lines, run at once.
    Lines(String),
    /// Switches an agent on or off.
    ToggleAgent(&'static str),
    /// Starts a script, or stops it when it runs.
    ToggleScript(String),
    StopScripts,
    DamageMeter(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Hotkey {
    name: String,
    group: &'static str,
    action: Action,
}

fn lines(group: &'static str, name: impl Into<String>, text: impl Into<String>) -> Hotkey {
    Hotkey {
        name: name.into(),
        group,
        action: Action::Lines(text.into()),
    }
}

const GENERAL: &str = "general";
const ACTIONS: &str = "actions";
const PETS: &str = "pets";
const AGENTS: &str = "agents";
const COMBAT: &str = "combat";
const POTION_GROUP: &str = "potions";
const ITEMS: &str = "items";
const WANDS: &str = "wands";
const SKILLS: &str = "skills";
const SPELLS: &str = "spells";
const VIRTUES: &str = "virtues";
const TARGETS: &str = "targets";
const SCRIPTS: &str = "scripts";

#[rustfmt::skip]
const FIXED: &[(&str, &str, &str)] = &[
    (GENERAL, "Resync", "resync"),
    (GENERAL, "Ping Server", "ping"),
    (GENERAL, "Accept Party", "partyaccept"),
    (GENERAL, "Decline Party", "partydecline"),
    (GENERAL, "Where Am I", "where"),
    (ACTIONS, "Fly On/Off", "togglefly"),
    (ACTIONS, "Use Last Item", "useobject 'lastobject'"),
    (ACTIONS, "Use Left Hand", "useobject 'lefthand'"),
    (ACTIONS, "Use Right Hand", "useobject 'righthand'"),
    (ACTIONS, "Show Names Mobiles", "shownames 'mobiles'"),
    (ACTIONS, "Show Names Corpses", "shownames 'corpses'"),
    (ACTIONS, "Mount / Dismount", "togglemounted"),
    (PETS, "All Come", "msg 'all come'"),
    (PETS, "All Follow Me", "msg 'all follow me'"),
    (PETS, "All Follow", "msg 'all follow'"),
    (PETS, "All Guard Me", "msg 'all guard me'"),
    (PETS, "All Guard", "msg 'all guard'"),
    (PETS, "All Kill", "msg 'all kill'"),
    (PETS, "All Stay", "msg 'all stay'"),
    (PETS, "All Stop", "msg 'all stop'"),
    (AGENTS, "Autoloot Once", "autoloot"),
    (AGENTS, "Dress", "dress"),
    (AGENTS, "Undress", "undress"),
    (AGENTS, "Save Dress", "dressconfig"),
    (AGENTS, "Buy On", "buy"),
    (AGENTS, "Buy Off", "clearbuy"),
    (AGENTS, "Sell On", "sell"),
    (AGENTS, "Sell Off", "clearsell"),
    (COMBAT, "Primary Ability", "setability 'primary' 'on'"),
    (COMBAT, "Secondary Ability", "setability 'secondary' 'on'"),
    (COMBAT, "Stun", "setability 'stun'"),
    (COMBAT, "Disarm", "setability 'disarm'"),
    (COMBAT, "Cancel Ability", "setability 'primary' 'off'"),
    (COMBAT, "Attack Last Target", "attack 'last'"),
    (COMBAT, "Attack Nearest Enemy", "getenemy 'enemy' 'criminal' 'gray' 'murderer' 'closest'\nif findalias 'enemy'\nattack 'enemy'\nendif"),
    (COMBAT, "War Mode On/Off", "togglewar"),
    (COMBAT, "Bandage Self", "bandageself"),
    (COMBAT, "Bandage Last", "bandagetarget 'last'"),
    (COMBAT, "Use Bandage", "usetype 0x0E21"),
    (COMBAT, "Clear Left Hand", "clearhands 'left'"),
    (COMBAT, "Clear Right Hand", "clearhands 'right'"),
    (COMBAT, "Toggle Left Hand", "togglehands 'left'"),
    (COMBAT, "Toggle Right Hand", "togglehands 'right'"),
    (ITEMS, "Enchanted Apple", "usetype 0x2FD8 1160"),
    (ITEMS, "Orange Petals", "usetype 0x1021 0x002B"),
    (ITEMS, "Wrath Grapes", "usetype 0x2FD7 0x0482"),
    (ITEMS, "Rose Of Trinsic", "usetype 0x234B 0"),
    (ITEMS, "Smoke Bomb", "usetype 0x2808"),
    (ITEMS, "Spell Stone", "usetype 0x4079"),
    (ITEMS, "Healing Stone", "usetype 0x4078"),
    (SPELLS, "Mini Heal", "miniheal"),
    (SPELLS, "Big Heal", "bigheal"),
    (SPELLS, "Chivalry Heal", "chivalryheal"),
    (SPELLS, "Interrupt", "interrupt"),
    (SPELLS, "Last Spell", "cast 'last'"),
    (SPELLS, "Last Spell On Last Target", "cast 'last' 'last'"),
    (SKILLS, "Last Skill", "useskill 'last'"),
    (VIRTUES, "Honor", "virtue 'honor'"),
    (VIRTUES, "Sacrifice", "virtue 'sacrifice'"),
    (VIRTUES, "Valor", "virtue 'valor'"),
    (TARGETS, "Target Self", "target 'self'"),
    (TARGETS, "Target Last", "target 'last'"),
    (TARGETS, "Target Self Queued", "autotargetself"),
    (TARGETS, "Target Last Queued", "autotargetlast"),
    (TARGETS, "Cancel Target", "canceltarget"),
    (TARGETS, "Clear Target Queue", "cleartargetqueue"),
    (TARGETS, "Clear Last Target", "clearlasttarget"),
    (TARGETS, "Clear Last And Queue", "clearlasttarget\ncleartargetqueue"),
];

/// The wand spells a wand hotkey equips a wand for.
const WAND_SPELLS: [&str; 11] = [
    "Clumsy",
    "Identification",
    "Heal",
    "Feeblemind",
    "Weaken",
    "Magic Arrow",
    "Harm",
    "Fireball",
    "Greater Heal",
    "Lightning",
    "Mana Drain",
];

/// The agents a hotkey switches on and off, by hotkey name.
const AGENT_TOGGLES: [(&str, &str); 7] = [
    ("Autoloot On/Off", "autoloot"),
    ("Scavenger On/Off", "scavenger"),
    ("Bandage Heal On/Off", "bandage"),
    ("Auto Remount On/Off", "remount"),
    ("Bone Cutter On/Off", "bone_cutter"),
    ("Auto Carver On/Off", "carver"),
    ("Open Corpses On/Off", "open_corpses"),
];

const METER: [(&str, &str); 4] = [
    ("Damage Meter Start", "start"),
    ("Damage Meter Pause", "pause"),
    ("Damage Meter Resume", "resume"),
    ("Damage Meter Stop", "stop"),
];

/// Every hotkey the character has now.
fn all(inner: &Inner) -> Vec<Hotkey> {
    let mut keys: Vec<Hotkey> = FIXED
        .iter()
        .map(|&(group, name, text)| lines(group, name, text))
        .collect();
    keys.extend(POTIONS.iter().map(|p| {
        lines(
            POTION_GROUP,
            format!("Potion {}", title(p.name)),
            format!("drinkpotion {}", quoted(p.name)),
        )
    }));
    keys.extend(WAND_SPELLS.iter().map(|spell| {
        lines(
            WANDS,
            format!("Wand {spell}"),
            format!("equipwand {}", quoted(spell)),
        )
    }));
    keys.extend(inner.scripting.spells.iter().map(|s| {
        lines(
            SPELLS,
            format!("Cast {}", s.name),
            format!("cast {}", quoted(&s.name)),
        )
    }));
    keys.extend(inner.scripting.skills.iter().filter(|s| s.usable).map(|s| {
        lines(
            SKILLS,
            format!("Use {}", s.name),
            format!("useskill {}", quoted(&s.name)),
        )
    }));
    let c = &inner.agents.config;
    keys.extend(c.organizer.keys().map(|l| {
        lines(
            AGENTS,
            format!("Organizer {l}"),
            format!("organizer {}", quoted(l)),
        )
    }));
    keys.extend(c.restock.keys().map(|l| {
        lines(
            AGENTS,
            format!("Restock {l}"),
            format!("restock {}", quoted(l)),
        )
    }));
    keys.extend(
        c.dress
            .keys()
            .map(|l| lines(AGENTS, format!("Dress {l}"), format!("dress {}", quoted(l)))),
    );
    keys.extend(c.dress.keys().map(|l| {
        lines(
            AGENTS,
            format!("Undress {l}"),
            format!("undress {}", quoted(l)),
        )
    }));
    keys.extend(c.targets.keys().map(|t| {
        lines(
            TARGETS,
            format!("Target Filter {t}"),
            format!("targetfilter {}", quoted(t)),
        )
    }));
    keys.extend(AGENT_TOGGLES.iter().map(|&(name, agent)| Hotkey {
        name: name.into(),
        group: AGENTS,
        action: Action::ToggleAgent(agent),
    }));
    keys.extend(METER.iter().map(|&(name, action)| Hotkey {
        name: name.into(),
        group: GENERAL,
        action: Action::DamageMeter(action),
    }));
    keys.extend(scripting::script_names().into_iter().map(|name| Hotkey {
        name: format!("Script {name}"),
        group: SCRIPTS,
        action: Action::ToggleScript(name),
    }));
    keys.push(Hotkey {
        name: "Stop All Scripts".into(),
        group: SCRIPTS,
        action: Action::StopScripts,
    });
    keys
}

/// A name in the quotes a script line needs: single quotes, or double quotes
/// for a name with an apostrophe in it, such as "Nature's Fury".
fn quoted(name: &str) -> String {
    if name.contains('\'') {
        format!("\"{name}\"")
    } else {
        format!("'{name}'")
    }
}

/// A potion name as a title: "night sight" becomes "Night Sight".
fn title(name: &str) -> String {
    name.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const ARG_GROUP: &str = "group";

/// The hotkeys, by group. A group name limits the list to that group.
pub(super) fn list(inner: &Inner, args: &Value) -> ToolResult {
    let only = args
        .get(ARG_GROUP)
        .and_then(|v| v.as_str())
        .map(str::to_ascii_lowercase);
    let mut groups: std::collections::BTreeMap<&str, Vec<String>> = Default::default();
    for key in all(inner) {
        if only.as_deref().is_some_and(|g| g != key.group) {
            continue;
        }
        groups.entry(key.group).or_default().push(key.name);
    }
    ToolResult::ok(json!({ "hotkeys": groups }))
}

/// Runs a hotkey by name, in any case and spacing.
pub(super) fn press(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(name) = args.get("name").and_then(|v| v.as_str()) else {
        return ToolResult::err("hotkey needs name");
    };
    let want = name_key(name);
    let Some(key) = all(inner).into_iter().find(|k| name_key(&k.name) == want) else {
        return ToolResult::err(format!(
            "no hotkey named '{name}'; the hotkeys tool lists them"
        ));
    };
    match key.action {
        Action::Lines(text) => {
            let mut pressed = scripting::run_now(inner, &key.name, &text);
            if pressed.ok {
                // The lines it ran, so a recording can write them down.
                pressed.result["lines"] = json!(text);
            }
            pressed
        }
        Action::ToggleAgent(agent) => {
            let on = !inner.agents.enabled(agent);
            let a = &mut inner.agents;
            match a.switch(agent, on).and_then(|()| a.save()) {
                Ok(()) => ToolResult::ok(json!({ "agent": agent, "on": on })),
                Err(e) => ToolResult::err(e),
            }
        }
        Action::ToggleScript(script) => {
            if scripting::running_name(inner).is_some_and(|n| n.eq_ignore_ascii_case(&script)) {
                scripting::stop_script(inner)
            } else {
                scripting::run_script(inner, &json!({ "name": script }))
            }
        }
        Action::StopScripts => match scripting::running_name(inner) {
            Some(_) => scripting::stop_script(inner),
            None => ToolResult::ok(json!({ "stopped": false })),
        },
        Action::DamageMeter(action) => agents::damage_meter(inner, &json!({ "action": action })),
    }
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::{armed_session, ready_to_act, PACK};
    use super::*;

    const ME: Serial = Serial(0x0000_0001);

    fn player() -> Inner {
        let inner = armed_session();
        {
            let mut w = inner.world.write();
            w.logged_in = true;
            w.self_state.serial = ME;
        }
        inner
    }

    #[test]
    fn every_hotkey_name_is_used_once() {
        let inner = player();
        let keys = all(&inner);
        let mut names: Vec<String> = keys.iter().map(|k| name_key(&k.name)).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "two hotkeys share a name");
    }

    #[test]
    fn every_hotkey_line_reads_as_a_script() {
        let inner = player();
        for key in all(&inner) {
            if let Action::Lines(text) = &key.action {
                assert!(
                    uoterm_script::Program::parse(text).is_ok(),
                    "{} does not read: {text}",
                    key.name
                );
            }
        }
    }

    #[test]
    fn a_hotkey_casts_a_spell_by_its_name_in_any_case() {
        const GREATER_HEAL: u16 = 29;
        let mut inner = player();
        ready_to_act(&mut inner);
        let result = press(&mut inner, &json!({ "name": "cast greater heal" }));
        assert!(result.ok, "{:?}", result.error);
        assert!(inner.outbound.contains(&encode::cast_spell(GREATER_HEAL)));
    }

    #[test]
    fn a_potion_hotkey_drinks_the_potion_of_its_colour() {
        const HEAL_POTION: Serial = Serial(0x4000_0F01);
        let mut inner = player();
        inner.world.write().items.insert(
            HEAL_POTION,
            uoterm_world::Item {
                serial: HEAL_POTION,
                graphic: GRAPHIC_POTION_HEAL,
                amount: 1,
                hue: 0,
                location: Point3::new(0, 0, 0),
                parent: Some(PACK),
                layer: None,
                grid: 0,
                name: String::new(),
            },
        );
        ready_to_act(&mut inner);
        assert!(press(&mut inner, &json!({ "name": "Potion Heal" })).ok);
        assert!(inner.outbound.contains(&encode::double_click(HEAL_POTION)));
    }

    #[test]
    fn a_hotkey_says_when_the_character_must_wait() {
        let mut inner = player();
        mark_action(&mut inner);
        let result = press(&mut inner, &json!({ "name": "Cast Heal" }));
        assert!(!result.ok);
    }

    #[test]
    fn an_agent_hotkey_switches_the_agent() {
        let mut inner = player();
        assert!(press(&mut inner, &json!({ "name": "Autoloot On/Off" })).ok);
        assert!(inner.agents.config.autoloot.enabled);
        assert!(press(&mut inner, &json!({ "name": "autoloot on/off" })).ok);
        assert!(!inner.agents.config.autoloot.enabled);
    }

    #[test]
    fn an_unknown_hotkey_is_refused() {
        let mut inner = player();
        assert!(!press(&mut inner, &json!({ "name": "Summon Dragon Army" })).ok);
    }
}
