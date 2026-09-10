//! The words a condition reads: `hits`, `poisoned`, `findtype`, `counttype`
//! and the rest. A word that names an object reads the character when the
//! script names none.

use uoterm_assist::mobiles::notorieties;
use uoterm_assist::{buffs, items, name_key};
use uoterm_protocol::types::{FLAG_HIDDEN, FLAG_WAR};

use super::commands::{find_gump, find_wand};
use super::*;

type Read = std::result::Result<ScriptValue, String>;

/// A cursor's flag byte by its kind.
const CURSOR_NEUTRAL: u8 = 0;
const CURSOR_HARMFUL: u8 = 1;
const CURSOR_BENEFICIAL: u8 = 2;
/// Skills are sent in tenths of a point.
const SKILL_TENTHS: f64 = 10.0;
/// The name a shard gives the lines it says itself.
const SYSTEM_NAME: &str = "system";

pub(super) fn read(game: &mut Game, call: &Call, ctx: &mut Ctx) -> Read {
    let name = call.name.as_str();
    if let Some(stat) = self_stat(game, name) {
        return Ok(ScriptValue::Number(stat));
    }
    match name {
        "true" => Ok(ScriptValue::Bool(true)),
        "false" => Ok(ScriptValue::Bool(false)),
        "x" | "y" | "z" => {
            let serial = serial_or_self(game, call.args.first(), ctx)?;
            let at = game
                .world()
                .map_location(serial)
                .ok_or_else(|| format!("{serial} is not in sight"))?;
            Ok(ScriptValue::Number(f64::from(match name {
                "x" => i32::from(at.x),
                "y" => i32::from(at.y),
                _ => i32::from(at.z),
            })))
        }
        "hits" | "maxhits" | "diffhits" => {
            let serial = serial_or_self(game, call.args.first(), ctx)?;
            let (hits, max) = hits_of(game, serial)?;
            Ok(ScriptValue::Number(f64::from(match name {
                "hits" => hits,
                "maxhits" => max,
                _ => max.saturating_sub(hits),
            })))
        }
        "serial" => {
            let serial = serial_or_self(game, call.args.first(), ctx)?;
            Ok(ScriptValue::Number(f64::from(serial.0)))
        }
        "graphic" | "color" | "amount" | "name" | "direction" | "directionname" => {
            let serial = serial_or_self(game, call.args.first(), ctx)?;
            object_detail(game, serial, name)
        }
        "dead" | "flying" | "paralyzed" | "poisoned" | "mounted" | "yellowhits" | "war"
        | "hidden" => {
            let serial = serial_or_self(game, call.args.first(), ctx)?;
            Ok(ScriptValue::Bool(mobile_state(game, serial, name)?))
        }
        "criminal" | "enemy" | "friend" | "gray" | "innocent" | "invulnerable" | "murderer" => {
            let serial = serial_or_self(game, call.args.first(), ctx)?;
            let noto = {
                let w = game.world();
                if serial == w.self_state.serial {
                    w.self_state.notoriety
                } else {
                    w.mobiles
                        .get(&serial)
                        .map(|m| m.notoriety)
                        .ok_or_else(|| format!("{serial} is not in sight"))?
                }
            };
            Ok(ScriptValue::Bool(notorieties(name).contains(&noto)))
        }
        "contents" => {
            let serial = game.serial(need(call, 0, "a container")?, ctx)?;
            Ok(ScriptValue::Number(
                game.world().items_inside(serial, false).len() as f64,
            ))
        }
        "inregion" => Err(
            "inregion needs the shard's region map, which a shard does not send to a client".into(),
        ),
        "skill" | "skillvalue" | "skillbase" | "skillstate" => skill(game, call),
        "findobject" => find_object(game, call, ctx),
        "findtype" => find_type(game, call, ctx),
        "findlayer" => find_layer(game, call, ctx),
        "findwand" => {
            ctx.vars.unset_alias(FOUND);
            let spell = need(call, 0, "a spell name or any")?.text.clone();
            let min = call.args.get(2).and_then(Arg::number).unwrap_or(0);
            Ok(ScriptValue::Bool(match find_wand(game, &spell, min) {
                Some(wand) => {
                    ctx.vars.set_alias(FOUND, wand.0);
                    true
                }
                None => false,
            }))
        }
        "distance" => {
            let serial = game.serial(need(call, 0, "an object")?, ctx)?;
            Ok(ScriptValue::Number(f64::from(distance_to(game, serial)?)))
        }
        "inrange" => {
            let serial = game.serial(need(call, 0, "an object")?, ctx)?;
            let range = need_number(call, 1, "a range")?;
            Ok(ScriptValue::Bool(
                distance_to(game, serial).is_ok_and(|d| i64::from(d) <= range),
            ))
        }
        "buffexists" => {
            let want = name_key(&need(call, 0, "a buff name")?.text);
            Ok(ScriptValue::Bool(
                buff_names(game.inner).iter().any(|b| name_key(b) == want),
            ))
        }
        "property" => {
            let prop = need(call, 0, "a property name")?.text.clone();
            let serial = game.serial(need(call, 1, "an object")?, ctx)?;
            Ok(property_value(game, serial, &prop))
        }
        "durability" => {
            let serial = game.serial(call.args.last().ok_or("durability needs an object")?, ctx)?;
            Ok(property_value(game, serial, "durability"))
        }
        "counttype" => count_type(game, call, ctx),
        "counttypeground" => {
            let graphic = Game::graphic(need(call, 0, "a graphic")?)?;
            let color = Game::color(call.args.get(1))?;
            let range = Game::range(call.args.get(2))?;
            let items = game.find_items(graphic, color, Source::Ground, range);
            let mobiles = game.find_mobiles(graphic, color, range).len() as f64;
            Ok(ScriptValue::Number(
                stack_total(game, &items, call.force) + mobiles,
            ))
        }
        "counter" => {
            let format = &need(call, 0, "a counter name")?.text;
            let supply =
                items::supply(format).ok_or_else(|| format!("no counter named '{format}'"))?;
            let found = game.find_items(supply.graphic, None, Source::Backpack, DEFAULT_RANGE);
            Ok(ScriptValue::Number(stack_total(game, &found, false)))
        }
        "bandage" => {
            let found = game.find_items(GRAPHIC_BANDAGE, None, Source::Backpack, DEFAULT_RANGE);
            Ok(ScriptValue::Number(stack_total(game, &found, false)))
        }
        "inparty" => {
            let serial = game.serial(need(call, 0, "a mobile")?, ctx)?;
            Ok(ScriptValue::Bool(game.world().party.contains(&serial)))
        }
        "findalias" => {
            let name = &need(call, 0, "a name")?.text;
            Ok(ScriptValue::Bool(game.system_alias(name).is_some()))
        }
        "infriendlist" => {
            let serial = game.serial(need(call, 0, "a mobile")?, ctx)?;
            let world = game.world().clone();
            Ok(ScriptValue::Bool(
                game.inner.agents.is_friend(&world, serial),
            ))
        }
        "ingump" => {
            let id = need(call, 0, "a gump id or any")?;
            let text = need(call, 1, "text")?.text.to_lowercase();
            Ok(ScriptValue::Bool(
                find_gump(game, id).is_some_and(|g| gump_text(game, &g).contains(&text)),
            ))
        }
        "gumpexists" => {
            let id = need(call, 0, "a gump id or any")?;
            Ok(ScriptValue::Bool(find_gump(game, id).is_some()))
        }
        "injournal" => {
            let text = &need(call, 0, "text")?.text;
            let author = call.args.get(1).map(|a| a.text.as_str());
            Ok(ScriptValue::Bool(journal_holds(game, text, author)))
        }
        "targetexists" => target_exists(game, call),
        "waitingfortarget" => Ok(ScriptValue::Bool(game.inner.target_intent.is_some())),
        "organizing" => Ok(ScriptValue::Bool(matches!(
            game.inner.agents.job,
            Some(agents::Job::Organize { .. })
        ))),
        "restocking" => Ok(ScriptValue::Bool(matches!(
            game.inner.agents.job,
            Some(agents::Job::Restock { .. })
        ))),
        "dressing" => Ok(ScriptValue::Bool(matches!(
            game.inner.agents.job,
            Some(agents::Job::Dress { .. } | agents::Job::Undress { .. })
        ))),
        // Commands that also answer a condition: true when they found what
        // they look for and did it.
        "useobject" | "usetype" | "moveitem" | "movetype" | "useonce" | "clearhands" => {
            act_as_condition(game, call, ctx)
        }
        other => Err(format!("unknown word '{other}'")),
    }
}

/// The alias the find words set.
const FOUND: &str = "found";

/// The character's own numbers that take no object.
fn self_stat(game: &Game, name: &str) -> Option<f64> {
    let w = game.world();
    let s = &w.self_state;
    let x = &s.status;
    Some(match name {
        "str" => f64::from(s.str_),
        "dex" => f64::from(s.dex),
        "int" => f64::from(s.int_),
        "stam" => f64::from(s.stam),
        "maxstam" => f64::from(s.stam_max),
        "mana" => f64::from(s.mana),
        "maxmana" => f64::from(s.mana_max),
        "physical" => f64::from(x.physical_resist),
        "fire" => f64::from(x.fire_resist),
        "cold" => f64::from(x.cold_resist),
        "poison" => f64::from(x.poison_resist),
        "energy" => f64::from(x.energy_resist),
        "followers" => f64::from(x.followers),
        "maxfollowers" => f64::from(x.followers_max),
        "gold" => f64::from(s.gold),
        "luck" => f64::from(x.luck),
        "tithingpoints" => f64::from(x.tithing),
        "weight" => f64::from(s.weight),
        "maxweight" => f64::from(s.weight_max),
        "diffweight" => f64::from(s.weight_max) - f64::from(s.weight),
        _ => return None,
    })
}

fn serial_or_self(
    game: &Game,
    arg: Option<&Arg>,
    ctx: &Ctx,
) -> std::result::Result<Serial, String> {
    match arg {
        Some(a) => game.serial(a, ctx),
        None => Ok(game.me()),
    }
}

fn hits_of(game: &Game, serial: Serial) -> std::result::Result<(u16, u16), String> {
    let w = game.world();
    if serial == w.self_state.serial {
        return Ok((w.self_state.hits, w.self_state.hits_max));
    }
    let m = w
        .mobiles
        .get(&serial)
        .ok_or_else(|| format!("{serial} is not in sight"))?;
    Ok((m.hits.unwrap_or(0), m.hits_max.unwrap_or(0)))
}

fn object_detail(game: &Game, serial: Serial, what: &str) -> Read {
    let w = game.world();
    let me = &w.self_state;
    let text = |t: String| Ok(ScriptValue::Text(t));
    let number = |n: u32| Ok(ScriptValue::Number(f64::from(n)));
    if serial == me.serial {
        return match what {
            "graphic" => number(u32::from(me.body)),
            "color" => number(u32::from(me.hue)),
            "name" => text(me.name.clone()),
            "direction" => number(u32::from(me.direction & DIRECTION_MASK)),
            "directionname" => text(Direction::from_byte(me.direction).name().into()),
            _ => number(1),
        };
    }
    if let Some(m) = w.mobiles.get(&serial) {
        return match what {
            "graphic" => number(u32::from(m.body)),
            "color" => number(u32::from(m.hue)),
            "name" => text(m.name.clone()),
            "direction" => number(u32::from(m.direction)),
            "directionname" => text(Direction::from_byte(m.direction).name().into()),
            _ => number(1),
        };
    }
    let item = w
        .items
        .get(&serial)
        .ok_or_else(|| format!("{serial} is not in sight"))?;
    match what {
        "graphic" => number(u32::from(item.graphic)),
        "color" => number(u32::from(item.hue)),
        "amount" => number(u32::from(item.amount)),
        "name" => text(item.name.clone()),
        other => Err(format!("an item has no {other}")),
    }
}

/// The facing bits of a direction byte; the bit above them is "running".
const DIRECTION_MASK: u8 = 0x07;

fn mobile_state(game: &Game, serial: Serial, what: &str) -> std::result::Result<bool, String> {
    let w = game.world();
    if serial == w.self_state.serial {
        let s = &w.self_state;
        return Ok(match what {
            "dead" => s.dead,
            "flying" => s.flying,
            "paralyzed" => s.paralyzed,
            "poisoned" => s.poisoned,
            "mounted" => w.worn(LAYER_MOUNT).is_some(),
            "yellowhits" => s.yellow_bar,
            "war" => s.war,
            _ => s.hidden,
        });
    }
    let m = w
        .mobiles
        .get(&serial)
        .ok_or_else(|| format!("{serial} is not in sight"))?;
    Ok(match what {
        "dead" => uoterm_world::is_ghost_body(m.body),
        "flying" => w.flags_mean_flying && m.flags & FLAG_POISONED != 0,
        "paralyzed" => m.flags & FLAG_FROZEN != 0,
        "poisoned" => w.is_poisoned(serial),
        "mounted" => m.equipment.iter().any(|e| e.layer == LAYER_MOUNT),
        "yellowhits" => w.has_yellow_bar(serial),
        "war" => m.flags & FLAG_WAR != 0,
        _ => m.flags & FLAG_HIDDEN != 0,
    })
}

fn distance_to(game: &Game, serial: Serial) -> std::result::Result<u32, String> {
    let w = game.world();
    let at = w
        .map_location(serial)
        .ok_or_else(|| format!("{serial} is not in sight"))?;
    Ok(w.self_state.location.chebyshev(at))
}

fn skill(game: &Game, call: &Call) -> Read {
    let skill_name = &need(call, 0, "a skill name")?.text;
    let id = game
        .inner
        .scripting
        .skills
        .find(skill_name)
        .map(|s| s.id)
        .ok_or_else(|| format!("no skill named '{skill_name}'"))?;
    let w = game.world();
    let value = w
        .self_state
        .skills
        .get(&id)
        .ok_or_else(|| format!("the shard has not sent the {skill_name} skill yet"))?;
    Ok(match call.name.as_str() {
        "skillbase" => ScriptValue::Number(f64::from(value.base) / SKILL_TENTHS),
        "skillstate" => ScriptValue::Text(
            match value.lock {
                SKILL_LOCK_UP => "up",
                SKILL_LOCK_DOWN => "down",
                _ => "locked",
            }
            .into(),
        ),
        _ => ScriptValue::Number(f64::from(value.value) / SKILL_TENTHS),
    })
}

/// `findobject serial [color] [source] [amount] [range]`.
fn find_object(game: &Game, call: &Call, ctx: &mut Ctx) -> Read {
    ctx.vars.unset_alias(FOUND);
    let serial = game.serial(need(call, 0, "an object")?, ctx)?;
    let color = Game::color(call.args.get(1))?;
    let source = match call.args.get(2) {
        Some(a) if a.is(ANY) || a.is(SOURCE_GROUND) => None,
        Some(a) => Some(game.source(Some(a), ctx)?),
        None => None,
    };
    let amount = call.args.get(3).and_then(Arg::number).unwrap_or(0);
    let range = call.args.get(4).map(|a| Game::range(Some(a))).transpose()?;
    let found = {
        let w = game.world();
        let here = w.self_state.location;
        let in_range = |at: Point3| range.map_or(true, |r| here.chebyshev(at) <= r);
        if let Some(m) = w.mobiles.get(&serial) {
            color.map_or(true, |c| m.hue == c) && in_range(m.location)
        } else if let Some(i) = w.items.get(&serial) {
            let inside = match source {
                Some(Source::Container(c)) => w.is_inside(serial, c),
                Some(Source::Backpack) => {
                    backpack_serial(&w).is_some_and(|p| w.is_inside(serial, p))
                }
                _ => true,
            };
            color.map_or(true, |c| i.hue == c)
                && inside
                && i64::from(i.amount) >= amount
                && w.map_location(serial).is_some_and(in_range)
        } else {
            serial == w.self_state.serial
        }
    };
    if found {
        ctx.vars.set_alias(FOUND, serial.0);
    }
    Ok(ScriptValue::Bool(found))
}

/// `findtype graphic [color] [source] [amount] [range]`. Items first; on the
/// ground a mobile with that body counts too.
fn find_type(game: &Game, call: &Call, ctx: &mut Ctx) -> Read {
    ctx.vars.unset_alias(FOUND);
    let graphic = Game::graphic(need(call, 0, "a graphic")?)?;
    let color = Game::color(call.args.get(1))?;
    let source = match call.args.get(2) {
        Some(a) if a.is(ANY) => Source::World,
        other => game.source(other, ctx)?,
    };
    let amount = call.args.get(3).and_then(Arg::number).unwrap_or(0);
    let range = Game::range(call.args.get(4))?;
    let item = {
        let found = game.find_items(graphic, color, source, range);
        let w = game.world();
        found.into_iter().find(|s| {
            w.items
                .get(s)
                .is_some_and(|i| i64::from(i.amount) >= amount)
        })
    };
    let found = item.or_else(|| {
        matches!(source, Source::Ground | Source::World)
            .then(|| game.find_mobiles(graphic, color, range).first().copied())
            .flatten()
    });
    if let Some(serial) = found {
        ctx.vars.set_alias(FOUND, serial.0);
    }
    Ok(ScriptValue::Bool(found.is_some()))
}

/// `findlayer mobile layer`: the item a mobile wears on a layer.
fn find_layer(game: &Game, call: &Call, ctx: &mut Ctx) -> Read {
    ctx.vars.unset_alias(FOUND);
    let serial = game.serial(need(call, 0, "a mobile")?, ctx)?;
    let layer = need_number(call, 1, "a layer")?;
    let worn = {
        let w = game.world();
        let equipment = if serial == w.self_state.serial {
            w.self_state.equipment.clone()
        } else {
            w.mobiles
                .get(&serial)
                .map(|m| m.equipment.clone())
                .unwrap_or_default()
        };
        equipment
            .iter()
            .find(|e| i64::from(e.layer) == layer)
            .map(|e| e.serial)
    };
    if let Some(item) = worn {
        ctx.vars.set_alias(FOUND, item.0);
    }
    Ok(ScriptValue::Bool(worn.is_some()))
}

/// `counttype graphic color source`: the items of a kind, by stack amount,
/// or one each with `!`.
fn count_type(game: &Game, call: &Call, ctx: &Ctx) -> Read {
    let graphic = Game::graphic(need(call, 0, "a graphic")?)?;
    let color = Game::color(call.args.get(1))?;
    let source = game.source(call.args.get(2), ctx)?;
    let found = game.find_items(graphic, color, source, DEFAULT_RANGE);
    Ok(ScriptValue::Number(stack_total(game, &found, call.force)))
}

/// The amounts of these stacks added up, or the number of stacks.
fn stack_total(game: &Game, serials: &[Serial], count_stacks: bool) -> f64 {
    if count_stacks {
        return serials.len() as f64;
    }
    let w = game.world();
    serials
        .iter()
        .filter_map(|s| w.items.get(s))
        .map(|i| f64::from(i.amount.max(1)))
        .sum()
}

fn target_exists(game: &Game, call: &Call) -> Read {
    let Some(cursor) = game.world().pending_target.clone() else {
        return Ok(ScriptValue::Bool(false));
    };
    let kind = call.args.first().map(|a| a.text.to_ascii_lowercase());
    Ok(ScriptValue::Bool(match kind.as_deref() {
        None | Some("any") | Some("server") => true,
        Some("harmful") => cursor.flags == CURSOR_HARMFUL,
        Some("beneficial") => cursor.flags == CURSOR_BENEFICIAL,
        Some("neutral") => cursor.flags == CURSOR_NEUTRAL,
        // A cursor of the assistant's own: this client opens none.
        Some("system") => false,
        Some(other) => return Err(format!("'{other}' is not a cursor kind")),
    }))
}

/// Runs a command that doubles as a condition. It is true when the command
/// found what it looks for and did it, and false when there was nothing to
/// do or the character must wait to act.
fn act_as_condition(game: &mut Game, call: &Call, ctx: &mut Ctx) -> Read {
    let found = match call.name.as_str() {
        "usetype" => super::commands::use_type_pick(game, call, ctx)?.is_some(),
        "useonce" => super::commands::use_once_pick(game, call)?.is_some(),
        "movetype" => super::commands::move_type_pick(game, call, ctx)?
            .item
            .is_some(),
        "clearhands" => {
            // True while a hand is full: the command then starts to empty it.
            let w = game.world();
            let full = w.worn(LAYER_ONE_HANDED).is_some() || w.worn(LAYER_TWO_HANDED).is_some();
            drop(w);
            if !full {
                return Ok(ScriptValue::Bool(false));
            }
            true
        }
        _ => true,
    };
    if !found {
        return Ok(ScriptValue::Bool(false));
    }
    match super::commands::run(game, call, ctx) {
        Step::Acted => {
            ctx.mark_acted();
            Ok(ScriptValue::Bool(true))
        }
        Step::Done => Ok(ScriptValue::Bool(true)),
        Step::Wait => Ok(ScriptValue::Bool(false)),
        Step::Fail(message) => Err(message),
    }
}

/// A property's number, or true when the property has none, or false when
/// the object does not have it.
fn property_value(game: &Game, serial: Serial, prop: &str) -> ScriptValue {
    let want = prop.to_lowercase();
    let line = property_lines(game.inner, serial)
        .into_iter()
        .find(|l| l.to_lowercase().contains(&want));
    match line {
        Some(l) => first_number(&l).map_or(ScriptValue::Bool(true), ScriptValue::Number),
        None => ScriptValue::Bool(false),
    }
}

/// All the words of a gump, lower case: its text lines and the client text
/// its layout names.
fn gump_text(game: &Game, gump: &uoterm_protocol::OpenGump) -> String {
    let mut all = gump.text.join("\n");
    if let Some(table) = game.inner.cliloc.as_ref() {
        for number in gump
            .layout
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|n| n.parse::<u32>().ok())
            .filter(|&n| n >= CLILOC_FIRST)
        {
            if let Some(text) = table.text(number) {
                all.push('\n');
                all.push_str(text);
            }
        }
    }
    all.to_lowercase()
}

/// The lowest client text number. A layout number below it is a place or a
/// size, not text.
const CLILOC_FIRST: u32 = 500_000;

/// True when a journal line since the last `clearjournal` holds the words,
/// said by `author` or by the shard itself for `system`.
pub(super) fn journal_holds(game: &Game, text: &str, author: Option<&str>) -> bool {
    let needle = text.to_lowercase();
    let w = game.world();
    let floor = game.inner.scripting.journal_floor;
    let heard = w.journal.after(floor).any(|line| {
        let said_by = match author {
            None => true,
            Some(a) if a.eq_ignore_ascii_case(SYSTEM_NAME) => {
                line.name.is_empty() || line.name.eq_ignore_ascii_case(SYSTEM_NAME)
            }
            Some(a) => line.name.eq_ignore_ascii_case(a),
        };
        said_by && line.text.to_lowercase().contains(&needle)
    });
    heard
}

/// The names of the buffs on the character: the client text the shard names
/// each one with, or the icon's own name without the text files.
pub(in crate::session) fn buff_names(inner: &Inner) -> Vec<String> {
    let w = inner.world.read();
    let mut names: Vec<String> = w
        .buffs
        .values()
        .filter_map(|b| {
            inner
                .cliloc
                .as_ref()
                .and_then(|table| table.text(b.title_cliloc))
                .map(|t| t.trim().to_string())
                .or_else(|| buffs::icon_name(b.icon).map(String::from))
        })
        .collect();
    names.sort_unstable();
    names
}
