//! The commands a script line can run.
//!
//! Each one returns a [`Step`]: done, acted (something went to the shard),
//! wait (run the line again next tick), or fail (the script stops and names
//! the line). A command that has to act waits while the character is still
//! paying for the last action, so a script goes at a player's pace.

use uoterm_assist::abilities::{move_for, MoveSlot};
use uoterm_assist::items::{food_graphics, WAND_GRAPHICS};
use uoterm_assist::mobiles::{is_humanoid, is_transformed, notorieties};
use uoterm_assist::name_key;

use super::*;

/// What a wait with no time of its own waits.
const WAIT_DEFAULT: Duration = Duration::from_secs(5);
/// A walk that has not ended after this long is given up.
const WALK_LIMIT: Duration = Duration::from_secs(60);
/// A paperdoll is asked for by double-clicking the mobile with this bit set.
const PAPERDOLL_REQUEST_BIT: u32 = 0x8000_0000;
/// The resources a tool can be aimed at with no cursor, by number.
const RESOURCES: [&str; 5] = ["ore", "sand", "wood", "graves", "red mushrooms"];
/// The virtues a player can invoke, by number.
const VIRTUES: [(&str, u8); 3] = [("honor", 1), ("sacrifice", 2), ("valor", 3)];
const SPEECH_GUILD: u8 = 13;
const SPEECH_ALLIANCE: u8 = 14;
/// Spell numbers the heal commands cast.
const SPELL_HEAL: u16 = 4;
const SPELL_CURE: u16 = 11;
const SPELL_ARCH_CURE: u16 = 25;
const SPELL_GREATER_HEAL: u16 = 29;
const SPELL_CLEANSE_BY_FIRE: u16 = 201;
const SPELL_CLOSE_WOUNDS: u16 = 202;
/// The lowest client text number. A context menu entry number from here up
/// names the entry's text; below it, the entry's place in the menu.
const FIRST_CLILOC_NUMBER: i64 = 500_000;
/// A skill lock by its word.
const LOCKS: [(&str, u8); 3] = [
    ("up", SKILL_LOCK_UP),
    ("down", SKILL_LOCK_DOWN),
    ("locked", SKILL_LOCK_LOCKED),
];
const LAST: &str = "last";
const HAND_LEFT: &str = "left";
const HAND_RIGHT: &str = "right";
const HAND_BOTH: &str = "both";
/// The hands by their place in [`Scripting::hands`] and their layer.
const HANDS: [(&str, u8); 2] = [
    (HAND_LEFT, LAYER_TWO_HANDED),
    (HAND_RIGHT, LAYER_ONE_HANDED),
];
/// Commands that only draw on a game window. A client with no window has
/// nothing to show them on, so they say so and go on.
const WINDOW_ONLY: [&str; 7] = [
    "playsound",
    "snapshot",
    "hotkeys",
    "messagebox",
    "mapuo",
    "clickscreen",
    "info",
];
/// What a command says when it needs a person to pick an object.
const NEEDS_A_PICK: &str =
    "needs a person to pick an object with the mouse; give the serial instead";

pub(super) fn run(game: &mut Game, call: &Call, ctx: &mut Ctx) -> Step {
    step(dispatch(game, call, ctx))
}

fn dispatch(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let name = call.name.as_str();
    if WINDOW_ONLY.contains(&name) {
        Game::note(call, ctx, "shows only in a game window; nothing to do here");
        return Ok(Step::Done);
    }
    match name {
        // Abilities.
        "fly" | "land" => fly(game, name == "fly"),
        "togglefly" => {
            let up = !game.world().self_state.flying;
            fly(game, up)
        }
        "setability" => set_ability(game, call),
        // Actions.
        "attack" => attack(game, call, ctx),
        "warmode" | "war" => war_mode(game, call),
        "clickobject" => click_object(game, call, ctx),
        "bandageself" => bandage(game, call, ctx, None),
        "bandagetarget" => {
            let target = game.serial(need(call, 0, "a mobile")?, ctx)?;
            bandage(game, call, ctx, Some(target))
        }
        "drinkpotion" => drink_potion(game, call, ctx),
        "interrupt" => interrupt(game, call, ctx),
        "opendoor" => {
            let Some(tile) = nearest_door_tile(game.inner) else {
                Game::note(call, ctx, "no door beside the character");
                return Ok(Step::Done);
            };
            let at = game.world().self_state.location;
            want_door_macro(game.inner, at, tile, Instant::now());
            Ok(Step::Acted)
        }
        "togglewar" => {
            let on = !game.world().self_state.war;
            send_war_mode(game.inner, on);
            Ok(Step::Acted)
        }
        "usetype" => use_type(game, call, ctx),
        "useobject" => use_object(game, call, ctx),
        "useonce" => use_once(game, call, ctx),
        "clearuseonce" | "clearusequeue" => {
            game.inner.scripting.used_once.clear();
            Ok(Step::Done)
        }
        "moveitem" => move_item(game, call, ctx, false),
        "moveitemoffset" => move_item(game, call, ctx, true),
        "movetype" => move_type(game, call, ctx, false),
        "movetypeoffset" => move_type(game, call, ctx, true),
        "walk" | "run" => walk(game, call, ctx, name == "run"),
        "turn" => turn(game, call),
        "pathfindto" => path_find_to(game, call, ctx),
        "useskill" => use_skill(game, call),
        "feed" => feed(game, call, ctx),
        "rename" => rename(game, call, ctx),
        "shownames" => show_names(game, call),
        "togglehands" => toggle_hands(game, call),
        "clearhands" => clear_hands(game, call, ctx),
        "equipitem" => equip_item(game, call, ctx),
        "togglemounted" => toggle_mounted(game, ctx),
        "equipwand" => equip_wand(game, call, ctx),
        // Aliases. `setalias` with a serial is the interpreter's own.
        "setalias" => set_alias(game, call, ctx),
        "promptalias" => Err(format!("promptalias {NEEDS_A_PICK}")),
        "addfriend" if call.args.is_empty() => Err(format!("addfriend {NEEDS_A_PICK}")),
        "addfriend" => friend_list(game, call, ctx, true),
        "removefriend" => friend_list(game, call, ctx, false),
        // Gumps.
        "waitforgump" => wait_for_gump(game, call, ctx),
        "replygump" => reply_gump(game, call, ctx),
        "closegump" => close_gump(game, call, ctx),
        // Journal.
        "clearjournal" | "uniquejournal" => {
            let newest = game.world().journal.last_seq();
            game.inner.scripting.journal_floor = newest;
            Ok(Step::Done)
        }
        "waitforjournal" => wait_for_journal_line(game, call, ctx),
        // Main.
        "ping" => {
            game.inner.outbound.push_back(encode::ping(1));
            Ok(Step::Acted)
        }
        "playmacro" => play_macro(game, call),
        "script" => script_control(game, call, ctx),
        "resync" => {
            game.inner.outbound.push_back(encode::resync());
            Ok(Step::Acted)
        }
        "where" => {
            let (at, map) = {
                let w = game.world();
                (w.self_state.location, w.self_state.map)
            };
            ctx.say(format!("{} {} {} on map {map}", at.x, at.y, at.z));
            Ok(Step::Done)
        }
        "location" => location(game, call, ctx),
        // Others.
        "paperdoll" => paperdoll(game, call, ctx),
        "helpbutton" => Err("the help menu needs a person to read it".into()),
        "guildbutton" => button(game, encode::guild_menu),
        "questsbutton" => button(game, encode::quest_menu),
        "logoutbutton" => {
            game.inner.outbound.push_back(encode::logout());
            Ok(Step::Acted)
        }
        "virtue" => virtue(game, call),
        "msg" | "chatmsg" => say(game, call, SPEECH_REGULAR),
        "yellmsg" => say(game, call, SPEECH_YELL),
        "whispermsg" => say(game, call, SPEECH_WHISPER),
        "emotemsg" => say(game, call, SPEECH_EMOTE),
        "guildmsg" => say(game, call, SPEECH_GUILD),
        "allymsg" => say(game, call, SPEECH_ALLIANCE),
        "partymsg" => party_message(game, call, ctx),
        "headmsg" | "sysmsg" => need(call, 0, "text").map(|text| {
            ctx.say(text.text.clone());
            Step::Done
        }),
        "timermsg" => timer_message(game, call),
        "promptmsg" => prompt_message(game, call, ctx),
        "waitforprompt" => {
            let open = game.world().prompt.is_some();
            wait_until(call, ctx, 0, "no prompt came", open)
        }
        "cancelprompt" => cancel_prompt(game),
        "contextmenu" => context_menu(game, call, ctx),
        "waitforcontext" => wait_for_context(game, call, ctx),
        "ignoreobject" => ignore_object(game, call, ctx),
        "clearignorelist" => {
            game.inner.scripting.ignored.clear();
            Ok(Step::Done)
        }
        "setskill" => set_skill(game, call),
        "waitforproperties" => wait_for_properties(game, call, ctx),
        "autocolorpick" => auto_color_pick(game, call),
        "waitforcontents" => wait_for_contents(game, call, ctx),
        // Spells.
        "cast" => cast(game, call, ctx),
        "miniheal" => heal(game, call, ctx, (SPELL_HEAL, SPELL_CURE)),
        "bigheal" => heal(game, call, ctx, (SPELL_GREATER_HEAL, SPELL_ARCH_CURE)),
        "chivalryheal" => heal(game, call, ctx, (SPELL_CLOSE_WOUNDS, SPELL_CLEANSE_BY_FIRE)),
        // Targeting.
        "waitfortarget" => {
            let open = game.world().pending_target.is_some();
            wait_until(call, ctx, 0, "no target cursor came", open)
        }
        "canceltarget" => cancel_target(game),
        "target" => target(game, call, ctx),
        "targettype" => target_type(game, call, ctx, false),
        "targetground" => target_type(game, call, ctx, true),
        "targettile" => target_tile(game, call, ctx),
        "targettileoffset" => target_tile_offset(game, call, ctx, false),
        "targettilerelative" => target_tile_relative(game, call, ctx, false),
        "targetresource" => target_resource(game, call, ctx),
        "cleartargetqueue" | "cancelautotarget" => {
            game.inner.target_intent = None;
            Ok(Step::Done)
        }
        "autotargetobject" => auto_target_object(game, call, ctx),
        "autotargetself" => {
            let me = game.me();
            queue_target(game.inner, me, ctx.now);
            Ok(Step::Done)
        }
        "autotargetlast" => match game.inner.last_target {
            Some(last) => {
                queue_target(game.inner, last, ctx.now);
                Ok(Step::Done)
            }
            None => Err("there is no last target yet".into()),
        },
        "autotargettype" => auto_target_type(game, call, ctx, false),
        "autotargetground" => auto_target_type(game, call, ctx, true),
        "autotargettile" => target_tile(game, call, ctx),
        "autotargettileoffset" => target_tile_offset(game, call, ctx, true),
        "autotargettilerelative" => target_tile_relative(game, call, ctx, true),
        "autotargetghost" => auto_target_ghost(game, call, ctx),
        // Agents.
        "organizer" => agent_job(game, call, ctx, AgentJob::Organize),
        "restock" => agent_job(game, call, ctx, AgentJob::Restock),
        "dress" => start_job(
            game,
            agents::Job::Dress {
                list: list_arg(call).unwrap_or_else(|| agents::TEMP_DRESS_LIST.into()),
            },
        ),
        "undress" => start_job(
            game,
            agents::Job::Undress {
                list: list_arg(call),
            },
        ),
        "dressconfig" => {
            let world = game.world().clone();
            game.inner.agents.dress_from_worn(&world);
            Ok(Step::Done)
        }
        "autoloot" => start_job(game, agents::Job::LootOnce),
        "toggleautoloot" | "togglescavenger" => {
            let agent = if name == "toggleautoloot" {
                "autoloot"
            } else {
                "scavenger"
            };
            let a = &mut game.inner.agents;
            let on = !match agent {
                "autoloot" => a.config.autoloot.enabled,
                _ => a.config.scavenger.enabled,
            };
            a.switch(agent, on)
                .and_then(|()| a.save())
                .map(|()| Step::Done)
        }
        "buy" | "sell" => vendor_agent(game, call, name),
        "clearbuy" | "clearsell" => {
            let a = &mut game.inner.agents;
            a.switch(&name["clear".len()..], false)
                .and_then(|()| a.save())
                .map(|()| Step::Done)
        }
        "clearlasttarget" => {
            game.inner.last_target = None;
            Ok(Step::Done)
        }
        "targetfilter" => {
            let filter = need(call, 0, "a target filter name")?.text.clone();
            match agents::work_pick(game.inner, &filter)? {
                Some(serial) => {
                    game.inner.last_target = Some(serial);
                    ctx.vars.set_alias("enemy", serial.0);
                }
                None => Game::note(call, ctx, "no mobile passes that filter"),
            }
            Ok(Step::Done)
        }
        "textentrymsg" => text_entry(game, call, ctx, true),
        "canceltextentry" => text_entry(game, call, ctx, false),
        "waitfortextentry" => {
            let open = game.world().text_entry.is_some();
            wait_until(call, ctx, 0, "no text dialog came", open)
        }
        "partyinvite" => {
            let member = call.args.first().map(|a| game.serial(a, ctx)).transpose()?;
            game.inner.outbound.push_back(encode::party_invite(member));
            Ok(Step::Acted)
        }
        "partyremove" => {
            let member = game.serial(need(call, 0, "a member")?, ctx)?;
            game.inner.outbound.push_back(encode::party_remove(member));
            Ok(Step::Acted)
        }
        "partyloot" => {
            let allow = on_off(call, 0)?;
            game.inner.outbound.push_back(encode::party_can_loot(allow));
            Ok(Step::Acted)
        }
        "setstatlock" => stat_lock(game, call),
        "emoteaction" => {
            let action = text_arg(call, 0, EMOTE_ACTION_MAX)?;
            game.inner
                .outbound
                .push_back(encode::emote_animation(action));
            Ok(Step::Acted)
        }
        "partyaccept" | "partydecline" => {
            let Some(leader) = game.inner.world.read().party_invite else {
                Game::note(call, ctx, "no party invite is open");
                return Ok(Step::Done);
            };
            if name == "partyaccept" && chat_refuses(game.inner, leader, Plan::Party) {
                return Err(CHAT_SAYS_NO.into());
            }
            game.inner.world.write().party_invite = None;
            let packet = if name == "partyaccept" {
                encode::party_accept(leader)
            } else {
                encode::party_decline(leader)
            };
            game.inner.outbound.push_back(packet);
            Ok(Step::Acted)
        }
        "getenemy" => pick_mobile(game, call, ctx, Pick::Enemy),
        "getfriend" => pick_mobile(game, call, ctx, Pick::Friend),
        other => Err(format!("unknown command '{other}'")),
    }
}

// ---------- pacing and small helpers ----------

/// Waits while the last action is still being paid for.
fn ready(game: &Game) -> bool {
    action_ready(game.inner)
}

/// The time argument at `i` in milliseconds, or [`WAIT_DEFAULT`].
fn wait_time(call: &Call, i: usize) -> Duration {
    call.args
        .get(i)
        .and_then(Arg::number)
        .map_or(WAIT_DEFAULT, |ms| Duration::from_millis(ms.max(0) as u64))
}

/// A wait for something to be true, with the time argument at `time_arg`. It
/// goes on when the time runs out, as scripts expect, and says so.
fn wait_until(
    call: &Call,
    ctx: &mut Ctx,
    time_arg: usize,
    gave_up: &str,
    holds: bool,
) -> std::result::Result<Step, String> {
    if holds {
        return Ok(Step::Done);
    }
    if ctx.waited() >= wait_time(call, time_arg) {
        Game::note(call, ctx, gave_up);
        return Ok(Step::Done);
    }
    Ok(Step::Wait)
}

/// True the first time a line runs: its wait clock has not moved.
fn first_run(ctx: &Ctx) -> bool {
    ctx.waited().is_zero()
}

/// Double-clicks, or waits for the double-click to be allowed.
fn double_click(game: &mut Game, serial: Serial) -> Step {
    if dismount_blocked(game.inner, serial) {
        return Step::Fail(DISMOUNT_BLOCKED.into());
    }
    if send_double_click(game.inner, serial) {
        game.inner.last_object = Some(serial);
        Step::Acted
    } else {
        Step::Wait
    }
}

fn button(game: &mut Game, packet: fn(Serial) -> Vec<u8>) -> std::result::Result<Step, String> {
    let me = game.me();
    game.inner.outbound.push_back(packet(me));
    Ok(Step::Acted)
}

fn on_off(call: &Call, i: usize) -> std::result::Result<bool, String> {
    let arg = need(call, i, "on or off")?;
    match arg.text.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(format!("'{other}' is not on or off")),
    }
}

// ---------- abilities ----------

fn fly(game: &mut Game, up: bool) -> std::result::Result<Step, String> {
    if game.world().self_state.flying == up {
        return Ok(Step::Done);
    }
    game.inner.outbound.push_back(encode::toggle_flying());
    Ok(Step::Acted)
}

fn set_ability(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let which = need(call, 0, "primary, secondary, stun or disarm")?;
    let on = match call.args.get(1) {
        Some(_) => on_off(call, 1)?,
        None => true,
    };
    let me = game.me();
    let packet = if !on {
        encode::set_ability(me, encode::NO_ABILITY)
    } else if which.is("primary") || which.is("secondary") {
        let slot = if which.is("primary") {
            MoveSlot::Primary
        } else {
            MoveSlot::Secondary
        };
        let weapon = game.world().equipped_weapon_graphic();
        encode::set_ability(me, move_for(weapon, slot))
    } else if which.is("stun") {
        encode::stun_request()
    } else if which.is("disarm") {
        encode::disarm_request()
    } else {
        return Err(format!("'{}' is not an ability", which.text));
    };
    game.inner.outbound.push_back(packet);
    Ok(Step::Acted)
}

// ---------- actions ----------

fn attack(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a mobile")?, ctx)?;
    send_attack(game.inner, serial);
    game.inner.last_target = Some(serial);
    Ok(Step::Acted)
}

fn war_mode(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let on = on_off(call, 0)?;
    send_war_mode(game.inner, on);
    Ok(Step::Acted)
}

fn click_object(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a serial")?, ctx)?;
    game.inner.outbound.push_back(encode::single_click(serial));
    Ok(Step::Acted)
}

/// Uses a bandage on the character, or on `target`, with no cursor.
fn bandage(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    target: Option<Serial>,
) -> std::result::Result<Step, String> {
    let bandage = {
        let w = game.world();
        backpack_serial(&w).and_then(|pack| {
            w.items
                .values()
                .filter(|i| i.graphic == GRAPHIC_BANDAGE && w.is_inside(i.serial, pack))
                .map(|i| i.serial)
                .min_by_key(|s| s.0)
        })
    };
    let Some(bandage) = bandage else {
        Game::note(call, ctx, "no bandages");
        return Ok(Step::Done);
    };
    if !ready(game) {
        return Ok(Step::Wait);
    }
    let (me, dex) = {
        let w = game.world();
        (w.self_state.serial, w.self_state.dex)
    };
    let on = target.unwrap_or(me);
    game.inner
        .outbound
        .push_back(encode::bandage_target(bandage, on));
    mark_action(game.inner);
    game.inner.next_bandage_at = Instant::now() + Duration::from_millis(bandage_self_ms(dex));
    Ok(Step::Acted)
}

/// Drinks a potion by its name: heal, cure, refresh, and the rest. The
/// potions that share a bottle are told apart by colour.
fn drink_potion(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    if !shard_allows(game.inner, AssistFeature::PotionHotkeys) {
        return Err(forbidden(AssistFeature::PotionHotkeys));
    }
    let name = &need(call, 0, "a potion name")?.text;
    let potion =
        uoterm_assist::items::potion(name).ok_or_else(|| format!("no potion named '{name}'"))?;
    let found = game
        .find_items(
            potion.graphic,
            Some(potion.hue),
            Source::Backpack,
            DEFAULT_RANGE,
        )
        .first()
        .copied();
    if found.is_some() && free_a_hand(game, ctx) {
        return Ok(Step::Wait);
    }
    match found {
        Some(serial) => Ok(double_click(game, serial)),
        None => {
            Game::note(call, ctx, format!("no {} potions left", potion.name));
            Ok(Step::Done)
        }
    }
}

/// Puts the left-hand item away so a hand is free to drink, when the
/// option says to, and has the agents take it out again after. True while
/// the hand is being freed, so the drink waits a tick.
fn free_a_hand(game: &mut Game, ctx: &Ctx) -> bool {
    let options = &game.inner.agents.config.options;
    if !options.free_hand_for_potions || !shard_allows(game.inner, AssistFeature::AutoPotionEquip) {
        return false;
    }
    if ctx.waited() >= HAND_FREE_WAIT {
        // The shard kept the item in the hand: drink with it there.
        return false;
    }
    let (free, left) = {
        let w = game.world();
        (
            reflex::has_free_hand(&w),
            w.worn(LAYER_TWO_HANDED).map(|e| e.serial),
        )
    };
    let Some(left) = left.filter(|_| !free) else {
        return false;
    };
    if game.inner.agents.rearm_pending(left) {
        // Put away already: wait for the shard to say the hand is empty.
        return true;
    }
    if !ready(game) {
        return true;
    }
    let me = game.me();
    let pack = backpack_serial(&game.world()).unwrap_or(me);
    lift_and_drop(game.inner, left, ONE_WORN_ITEM, DropAt::Into(pack));
    game.inner.agents.rearm_later(left, LAYER_TWO_HANDED);
    true
}

/// How long a line waits for a hand to come free.
const HAND_FREE_WAIT: Duration = Duration::from_secs(3);
const HAND_STAYS_FULL: &str = "the shard kept the item in the hand";

/// The most characters a line of speech or party chat may have.
const TEXT_MAX: usize = 512;
/// The most characters a shard takes in a prompt answer.
const PROMPT_TEXT_MAX: usize = 128;
/// The most characters of an emote animation's name.
const EMOTE_ACTION_MAX: usize = 32;
/// The stats a lock is set on, by the number the shard reads.
const STATS: [(&str, u8); 3] = [("str", 0), ("dex", 1), ("int", 2)];

/// The text argument at `i`, refused when it is longer than `max`.
fn text_arg(call: &Call, i: usize, max: usize) -> std::result::Result<&str, String> {
    let text = &need(call, i, "text")?.text;
    if text.chars().count() > max {
        return Err(format!(
            "{}: the text is longer than {max} characters",
            call.name
        ));
    }
    Ok(text)
}

/// Answers the open one-field dialog: OK with the text, or cancel.
fn text_entry(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    accept: bool,
) -> std::result::Result<Step, String> {
    let Some(dialog) = game.world().text_entry.clone() else {
        Game::note(call, ctx, "no text dialog is open");
        return Ok(Step::Done);
    };
    let text = if accept {
        let max = usize::try_from(dialog.max_len)
            .unwrap_or(usize::MAX)
            .min(TEXT_MAX);
        text_arg(call, 0, max)?.to_string()
    } else {
        String::new()
    };
    game.inner
        .outbound
        .push_back(encode::text_entry_response(&dialog, &text, accept));
    game.inner.world.write().text_entry = None;
    Ok(Step::Acted)
}

/// `setstatlock 'str' 'locked'`: sets the lock of a stat.
fn stat_lock(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let stat = need(call, 0, "str, dex or int")?;
    let id = STATS
        .iter()
        .find(|(n, _)| stat.is(n))
        .map(|&(_, id)| id)
        .ok_or_else(|| format!("'{}' is not str, dex or int", stat.text))?;
    let word = need(call, 1, "up, down or locked")?;
    let lock = LOCKS
        .iter()
        .find(|(w, _)| word.is(w))
        .map(|&(_, l)| l)
        .ok_or_else(|| format!("'{}' is not up, down or locked", word.text))?;
    game.inner.outbound.push_back(encode::stat_lock(id, lock));
    Ok(Step::Acted)
}

/// Breaks the spell being cast the way a player does: lifts an item from
/// the pack and puts it straight back.
fn interrupt(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let (pack, item) = {
        let w = game.world();
        let pack = backpack_serial(&w);
        let item = pack.and_then(|p| w.items_inside(p, false).first().map(|i| i.serial));
        (pack, item)
    };
    let (Some(pack), Some(item)) = (pack, item) else {
        Game::note(call, ctx, "the pack is empty; nothing to lift");
        return Ok(Step::Done);
    };
    if !ready(game) {
        return Ok(Step::Wait);
    }
    lift_and_drop(game.inner, item, ONE_WORN_ITEM, DropAt::Into(pack));
    Ok(Step::Acted)
}

/// The item `usetype graphic [color] [source] [range]` uses.
pub(super) fn use_type_pick(
    game: &Game,
    call: &Call,
    ctx: &Ctx,
) -> std::result::Result<Option<Serial>, String> {
    let graphic = Game::graphic(need(call, 0, "an item graphic")?)?;
    let color = Game::color(call.args.get(1))?;
    let source = game.source(call.args.get(2), ctx)?;
    let range = Game::range(call.args.get(3))?;
    Ok(game
        .find_items(graphic, color, source, range)
        .first()
        .copied())
}

fn use_type(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    match use_type_pick(game, call, ctx)? {
        Some(serial) => Ok(double_click(game, serial)),
        None => {
            Game::note(call, ctx, "none found");
            Ok(Step::Done)
        }
    }
}

fn use_object(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a serial")?, ctx)?;
    Ok(double_click(game, serial))
}

/// The last matching item in the pack that `useonce` has not used.
pub(super) fn use_once_pick(
    game: &Game,
    call: &Call,
) -> std::result::Result<Option<Serial>, String> {
    let graphic = Game::graphic(need(call, 0, "an item graphic")?)?;
    let color = Game::color(call.args.get(1))?;
    let w = game.world();
    let used = &game.inner.scripting.used_once;
    Ok(backpack_serial(&w).and_then(|pack| {
        w.items_inside(pack, false)
            .into_iter()
            .filter(|i| i.graphic == graphic && color.map_or(true, |c| i.hue == c))
            .filter(|i| !used.contains(&i.serial))
            .map(|i| i.serial)
            .next_back()
    }))
}

/// Uses the last matching item in the pack that `useonce` has not used.
fn use_once(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    if !shard_allows(game.inner, AssistFeature::UseOnceAgent) {
        return Err(forbidden(AssistFeature::UseOnceAgent));
    }
    let Some(serial) = use_once_pick(game, call)? else {
        Game::note(call, ctx, "none left that was not used");
        return Ok(Step::Done);
    };
    let step = double_click(game, serial);
    if step == Step::Acted {
        game.inner.scripting.used_once.insert(serial);
    }
    Ok(step)
}

/// `moveitem serial dest [amount]` or `moveitem serial dest x y z [amount]`.
/// With `offset`, the place is counted from the character.
fn move_item(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    offset: bool,
) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "an item")?, ctx)?;
    let dest = need(call, 1, "a destination")?;
    let (place, amount) = place_and_amount(call, 2)?;
    move_one(game, serial, dest, place, amount, offset, ctx)
}

/// What `movetype graphic source dest [x y z] [color] [amount] [range]`
/// moves: the item, where to, the place and the amount.
pub(super) struct MoveTypePick {
    pub(super) item: Option<Serial>,
    dest: Arg,
    place: Option<Place>,
    amount: Option<u16>,
}

pub(super) fn move_type_pick(
    game: &Game,
    call: &Call,
    ctx: &Ctx,
) -> std::result::Result<MoveTypePick, String> {
    let graphic = Game::graphic(need(call, 0, "an item graphic")?)?;
    let source = game.source(Some(need(call, 1, "a source")?), ctx)?;
    let dest = need(call, 2, "a destination")?.clone();
    let rest = &call.args[3..];
    // With a place the next three are x y z; without one, colour comes next.
    let (place, rest) = match rest {
        [x, y, z, more @ ..]
            if x.number().is_some() && y.number().is_some() && z.number().is_some() =>
        {
            (Some(xyz(x, y, z)?), more)
        }
        _ => (None, rest),
    };
    let color = Game::color(rest.first())?;
    let amount = match rest.get(1) {
        Some(a) => Some(
            a.number()
                .and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| format!("'{}' is not an amount", a.text))?,
        ),
        None => None,
    };
    let range = Game::range(rest.get(2))?;
    Ok(MoveTypePick {
        item: game
            .find_items(graphic, color, source, range)
            .first()
            .copied(),
        dest,
        place,
        amount,
    })
}

fn move_type(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    offset: bool,
) -> std::result::Result<Step, String> {
    let pick = move_type_pick(game, call, ctx)?;
    let Some(serial) = pick.item else {
        Game::note(call, ctx, "none found");
        return Ok(Step::Done);
    };
    move_one(
        game,
        serial,
        &pick.dest,
        pick.place,
        pick.amount,
        offset,
        ctx,
    )
}

fn xyz(x: &Arg, y: &Arg, z: &Arg) -> std::result::Result<Place, String> {
    let n = |a: &Arg| {
        a.number()
            .ok_or_else(|| format!("'{}' is not a number", a.text))
    };
    Ok((n(x)?, n(y)?, n(z)?))
}

/// A place as `x y z`: absolute, or counted from the character.
type Place = (i64, i64, i64);

/// The optional `x y z` and amount that follow a move's destination.
fn place_and_amount(
    call: &Call,
    from: usize,
) -> std::result::Result<(Option<Place>, Option<u16>), String> {
    let rest = call.args.get(from..).unwrap_or(&[]);
    let amount = |a: &Arg| {
        a.number()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| format!("'{}' is not an amount", a.text))
    };
    match rest {
        [] => Ok((None, None)),
        [a] => Ok((None, Some(amount(a)?))),
        [x, y, z] => Ok((Some(xyz(x, y, z)?), None)),
        [x, y, z, a] => Ok((Some(xyz(x, y, z)?), Some(amount(a)?))),
        _ => Err(format!("{} takes a place (x y z) and an amount", call.name)),
    }
}

/// Lifts an item and drops it at a destination.
fn move_one(
    game: &mut Game,
    serial: Serial,
    dest: &Arg,
    place: Option<Place>,
    amount: Option<u16>,
    offset: bool,
    ctx: &Ctx,
) -> std::result::Result<Step, String> {
    let (stack, here) = {
        let w = game.world();
        let stack = w
            .items
            .get(&serial)
            .map(|i| i.amount)
            .ok_or_else(|| format!("no item {serial}"))?;
        (stack, w.self_state.location)
    };
    let destination = if dest.is(SOURCE_GROUND) {
        let at = match (place, offset) {
            (Some((x, y, z)), true) => offset_point(here, x, y, z)?,
            (Some((x, y, z)), false) => map_point(x, y, z)?,
            (None, _) => here,
        };
        DropAt::Ground(at)
    } else {
        DropAt::Into(game.serial(dest, ctx)?)
    };
    if !ready(game) {
        return Ok(Step::Wait);
    }
    let amount = amount.filter(|&a| a > 0).unwrap_or(stack).min(stack);
    lift_and_drop(game.inner, serial, amount, destination);
    Ok(Step::Acted)
}

/// Walks one or more steps: `walk 'North'` or `walk "North, East, East"`.
/// The line waits until the steps are walked.
fn walk(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    running: bool,
) -> std::result::Result<Step, String> {
    if !first_run(ctx) {
        return Ok(walk_on(game, call, ctx));
    }
    let text = need(call, 0, "a direction")?.text.clone();
    let mut dirs = Vec::new();
    for word in text.split(',') {
        dirs.push(
            Direction::from_name(word)
                .ok_or_else(|| format!("'{}' is not a direction", word.trim()))?,
        );
    }
    let from = game
        .inner
        .movement
        .stepping_from(game.world().self_state.location);
    game.inner.ensure_facet();
    let mut points = Vec::new();
    let mut at = from;
    for dir in dirs {
        let step = hold_path(game.inner.tiles(), at, dir, 1);
        let Some(&next) = step.first() else {
            break;
        };
        points.push(next);
        at = next;
    }
    let Some(&dest) = points.last() else {
        Game::note(call, ctx, "cannot step that way");
        return Ok(Step::Done);
    };
    game.inner.movement.run_override = Some(running);
    apply_path(game.inner, points, dest);
    Ok(Step::Wait)
}

/// Waits for a walk the line started, up to [`WALK_LIMIT`].
fn walk_on(game: &mut Game, call: &Call, ctx: &mut Ctx) -> Step {
    if !game.inner.movement.walking() {
        game.inner.movement.run_override = None;
        return Step::Done;
    }
    if ctx.waited() >= WALK_LIMIT {
        game.inner.movement.clear();
        Game::note(call, ctx, "the walk took too long and was stopped");
        return Step::Done;
    }
    Step::Wait
}

fn turn(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let text = &need(call, 0, "a direction")?.text;
    let dir = Direction::from_name(text).ok_or_else(|| format!("'{text}' is not a direction"))?;
    let (at, reported) = {
        let w = game.world();
        (
            w.self_state.location,
            Direction::from_byte(w.self_state.direction),
        )
    };
    let facing = game.inner.movement.facing_after(reported);
    if facing == dir {
        return Ok(Step::Done);
    }
    if !game.inner.movement.in_flight.is_empty() {
        return Ok(Step::Wait);
    }
    match game
        .inner
        .movement
        .build_turn(facing, dir, at, Instant::now())
    {
        Some(packet) => {
            game.inner.outbound.push_back(packet);
            Ok(Step::Acted)
        }
        None => Ok(Step::Done),
    }
}

fn path_find_to(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    if !first_run(ctx) {
        return Ok(walk_on(game, call, ctx));
    }
    let x = need_number(call, 0, "an x")?;
    let y = need_number(call, 1, "a y")?;
    let z = match call.args.get(2) {
        Some(a) => a
            .number()
            .ok_or_else(|| format!("'{}' is not a z", a.text))?,
        None => i64::from(game.world().self_state.location.z),
    };
    let dest = map_point(x, y, z)?;
    if !queue_move(game.inner, dest) {
        Game::note(call, ctx, "no path there");
        return Ok(Step::Done);
    }
    Ok(Step::Wait)
}

fn use_skill(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let arg = need(call, 0, "a skill name")?;
    let id = if arg.is(LAST) {
        game.inner
            .scripting
            .last_skill
            .ok_or("no skill was used yet")?
    } else {
        match arg.number() {
            Some(n) => u16::try_from(n).map_err(|_| format!("'{}' is not a skill", arg.text))?,
            None => game
                .inner
                .scripting
                .skills
                .by_name(&arg.text)
                .map(|s| s.id)
                .ok_or_else(|| format!("no skill named '{}'", arg.text))?,
        }
    };
    if !ready(game) {
        return Ok(Step::Wait);
    }
    game.inner.outbound.push_back(encode::use_skill(id));
    game.inner.scripting.last_skill = Some(id);
    mark_action(game.inner);
    Ok(Step::Acted)
}

/// Gives food from the pack to a mobile: a food name, a food group, `any`
/// food, or a graphic.
fn feed(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let mobile = game.serial(need(call, 0, "a mobile")?, ctx)?;
    let food = need(call, 1, "a food")?;
    let graphics: Vec<u16> = match food.number() {
        Some(g) => vec![u16::try_from(g).map_err(|_| format!("'{}' is not a graphic", food.text))?],
        None if food.is(ANY) => uoterm_assist::items::FOODS.iter().map(|f| f.2).collect(),
        None => food_graphics(&food.text),
    };
    if graphics.is_empty() {
        return Err(format!("no food named '{}'", food.text));
    }
    let color = Game::color(call.args.get(2))?;
    let amount = call.args.get(3).and_then(Arg::number).unwrap_or(1).max(1);
    let amount = u16::try_from(amount).map_err(|_| format!("{amount} is too many to feed"))?;
    let item = graphics.iter().find_map(|&g| {
        game.find_items(g, color, Source::Backpack, DEFAULT_RANGE)
            .first()
            .copied()
    });
    let Some(item) = item else {
        Game::note(call, ctx, "no such food in the pack");
        return Ok(Step::Done);
    };
    move_one(
        game,
        item,
        &Arg::word(mobile.0.to_string()),
        None,
        Some(amount),
        false,
        ctx,
    )
}

fn rename(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a pet")?, ctx)?;
    let name = &need(call, 1, "a name")?.text;
    game.inner.outbound.push_back(encode::rename(serial, name));
    Ok(Step::Acted)
}

/// Single-clicks every mobile, or every corpse, in sight so its name shows in
/// the journal.
fn show_names(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    const CORPSE_GRAPHIC: u16 = 0x2006;
    let corpses = call.args.first().is_some_and(|a| a.is("corpses"));
    let serials: Vec<Serial> = {
        let w = game.world();
        if corpses {
            w.nearby_items(DEFAULT_RANGE as u16)
                .into_iter()
                .filter(|i| i.graphic == CORPSE_GRAPHIC)
                .map(|i| i.serial)
                .collect()
        } else {
            w.nearby_mobiles(DEFAULT_RANGE as u16)
                .into_iter()
                .map(|m| m.serial)
                .collect()
        }
    };
    for serial in &serials {
        game.inner.outbound.push_back(encode::single_click(*serial));
    }
    Ok(if serials.is_empty() {
        Step::Done
    } else {
        Step::Acted
    })
}

/// The hands a word names: left, right, or both.
fn hands(word: Option<&Arg>) -> std::result::Result<Vec<usize>, String> {
    match word {
        None => Ok(vec![1, 0]),
        Some(w) if w.is(HAND_BOTH) => Ok(vec![1, 0]),
        Some(w) if w.is(HAND_RIGHT) => Ok(vec![1]),
        Some(w) if w.is(HAND_LEFT) => Ok(vec![0]),
        Some(w) => Err(format!("'{}' is not left, right or both", w.text)),
    }
}

/// Puts the item in one hand into the pack and remembers it.
fn put_away(game: &mut Game, hand: usize, item: Serial) -> Step {
    if !ready(game) {
        return Step::Wait;
    }
    let me = game.me();
    let pack = backpack_serial(&game.world()).unwrap_or(me);
    lift_and_drop(game.inner, item, ONE_WORN_ITEM, DropAt::Into(pack));
    game.inner.scripting.hands[hand] = Some(item);
    game.inner.last_weapon = Some(item);
    Step::Acted
}

/// Wears an item on a layer.
fn wear(game: &mut Game, item: Serial, layer: u8) -> Step {
    if !ready(game) {
        return Step::Wait;
    }
    lift_and_wear(game.inner, item, layer);
    Step::Acted
}

fn toggle_hands(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let [hand] = hands(Some(need(call, 0, "left or right")?))?[..] else {
        return Err("togglehands takes left or right".into());
    };
    let layer = HANDS[hand].1;
    let worn = game.world().worn(layer).map(|e| e.serial);
    Ok(match (worn, game.inner.scripting.hands[hand]) {
        (Some(item), _) => put_away(game, hand, item),
        (None, Some(saved)) => wear(game, saved, layer),
        (None, None) => Step::Done,
    })
}

/// Puts the hands' items away, one hand a tick.
/// Puts the named hands' items into the pack, both in one move.
fn clear_hands(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let full: Vec<(usize, Serial)> = {
        let w = game.world();
        hands(call.args.first())?
            .into_iter()
            .filter_map(|hand| w.worn(HANDS[hand].1).map(|e| (hand, e.serial)))
            .collect()
    };
    if full.is_empty() {
        return Ok(Step::Done);
    }
    if ctx.waited() >= HAND_FREE_WAIT {
        return Err(HAND_STAYS_FULL.into());
    }
    if !ready(game) {
        return Ok(Step::Wait);
    }
    // The shard takes one lift per action: one hand now, the other at the
    // next action.
    let me = game.me();
    let pack = backpack_serial(&game.world()).unwrap_or(me);
    let (hand, item) = full[0];
    lift_and_drop(game.inner, item, ONE_WORN_ITEM, DropAt::Into(pack));
    game.inner.scripting.hands[hand] = Some(item);
    game.inner.last_weapon = Some(item);
    Ok(if full.len() == 1 {
        Step::Acted
    } else {
        Step::Wait
    })
}

fn equip_item(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let item = game.serial(need(call, 0, "an item")?, ctx)?;
    let layer = match call.args.get(1) {
        Some(l) => l
            .number()
            .and_then(|n| u8::try_from(n).ok())
            .ok_or_else(|| format!("'{}' is not a layer", l.text))?,
        None => return Err("equipitem needs the layer to wear the item on".into()),
    };
    Ok(wear(game, item, layer))
}

/// Mounts or dismounts. A mount is ridden by double-clicking it, so the
/// `mount` alias must name it.
fn toggle_mounted(game: &mut Game, ctx: &Ctx) -> std::result::Result<Step, String> {
    let mounted = game.world().worn(LAYER_MOUNT).is_some();
    let target = if mounted {
        game.me()
    } else {
        ctx.vars
            .alias("mount")
            .map(Serial)
            .ok_or("set the mount alias to the mount first: setalias 'mount' serial")?
    };
    Ok(double_click(game, target))
}

/// Wears a wand from the pack: one of any spell, or one whose properties
/// name the spell. `[minimum charges]` is read from the properties.
fn equip_wand(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let spell = need(call, 0, "a spell name or any")?.text.clone();
    let min_charges = call.args.get(1).and_then(Arg::number).unwrap_or(0);
    let wand = find_wand(game, &spell, min_charges);
    match wand {
        Some(wand) => Ok(wear(game, wand, LAYER_ONE_HANDED)),
        None => {
            Game::note(call, ctx, "no such wand in the pack");
            Ok(Step::Done)
        }
    }
}

/// A wand in the pack for a spell (or `any`) with at least `min_charges`.
pub(super) fn find_wand(game: &Game, spell: &str, min_charges: i64) -> Option<Serial> {
    let wands: Vec<Serial> = WAND_GRAPHICS
        .iter()
        .flat_map(|&g| game.find_items(g, None, Source::Backpack, DEFAULT_RANGE))
        .collect();
    let want = name_key(spell);
    wands.into_iter().find(|&wand| {
        let lines = property_lines(game.inner, wand);
        let names_spell = want == ANY || lines.iter().any(|l| name_key(l).contains(&want));
        let charges = lines
            .iter()
            .find(|l| l.to_lowercase().contains("charges"))
            .and_then(|l| first_number(l))
            .unwrap_or(0.0);
        names_spell && charges >= min_charges as f64
    })
}

// ---------- agents ----------

fn list_arg(call: &Call) -> Option<String> {
    call.args.first().map(|a| a.text.clone())
}

/// `buy ['list']` and `sell ['list']`: switch the agent on, with the list
/// named.
fn vendor_agent(game: &mut Game, call: &Call, agent: &str) -> std::result::Result<Step, String> {
    let a = &mut game.inner.agents;
    if let Some(list) = list_arg(call) {
        agents::use_list(&mut a.config, agent, &list)?;
    }
    a.switch(agent, true)?;
    a.save()?;
    Ok(Step::Done)
}

fn start_job(game: &mut Game, job: agents::Job) -> std::result::Result<Step, String> {
    game.inner.agents.start(job)?;
    Ok(Step::Done)
}

#[derive(Clone, Copy)]
enum AgentJob {
    Organize,
    Restock,
}

/// A list's own bag or delay is kept when a script writes this in its place.
const KEEP_LIST_VALUE: i64 = -1;

/// `organizer 'list' [source] [destination] [delay]`, and restock the same.
/// A source, destination or delay given here (not -1) changes the list for
/// this run and the runs after it, until the settings are read again.
fn agent_job(
    game: &mut Game,
    call: &Call,
    ctx: &Ctx,
    kind: AgentJob,
) -> std::result::Result<Step, String> {
    let name = need(call, 0, "a list name")?.text.clone();
    let bag = |i: usize| -> std::result::Result<Option<Serial>, String> {
        match call.args.get(i) {
            Some(a) if a.number() == Some(KEEP_LIST_VALUE) => Ok(None),
            Some(a) => game.serial(a, ctx).map(Some),
            None => Ok(None),
        }
    };
    let source = bag(1)?;
    let destination = bag(2)?;
    let delay = call
        .args
        .get(3)
        .and_then(Arg::number)
        .filter(|&n| n != KEEP_LIST_VALUE)
        .map(|n| n.max(0) as u64);
    let lists = match kind {
        AgentJob::Organize => &mut game.inner.agents.config.organizer,
        AgentJob::Restock => &mut game.inner.agents.config.restock,
    };
    let list: &mut agents::AgentMoveList = lists
        .get_mut(&name)
        .ok_or_else(|| format!("{}: no list named '{name}'", call.name))?;
    if let Some(s) = source {
        list.source = Some(s);
    }
    if let Some(d) = destination {
        list.destination = Some(d);
    }
    if let Some(ms) = delay {
        list.delay_ms = ms;
    }
    start_job(
        game,
        match kind {
            AgentJob::Organize => agents::Job::Organize { list: name },
            AgentJob::Restock => agents::Job::Restock { list: name },
        },
    )
}

// ---------- aliases and friends ----------

/// `setalias 'name' alias`, where the second word is a system alias such as
/// `self` or `found`. A plain serial is the interpreter's own.
fn set_alias(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let name = need(call, 0, "a name")?.text.clone();
    let Some(value) = call.args.get(1) else {
        return Err(format!("setalias with no serial {NEEDS_A_PICK}"));
    };
    let serial = game.serial(value, ctx)?;
    ctx.vars.set_alias(&name, serial.0);
    Ok(Step::Done)
}

fn ignore_object(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a serial")?, ctx)?;
    game.inner.scripting.ignored.insert(serial);
    Ok(Step::Done)
}

fn auto_target_object(
    game: &mut Game,
    call: &Call,
    ctx: &Ctx,
) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a serial")?, ctx)?;
    queue_target(game.inner, serial, ctx.now);
    Ok(Step::Done)
}

fn friend_list(
    game: &mut Game,
    call: &Call,
    ctx: &Ctx,
    add: bool,
) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a mobile")?, ctx)?;
    let friends = &mut game.inner.agents.config.friends.friends;
    friends.retain(|&f| f != serial);
    if add {
        friends.push(serial);
    }
    game.inner.agents.save()?;
    Ok(Step::Done)
}

// ---------- gumps ----------

/// The open gump with this id, or any gump for `any` or 0.
pub(super) fn find_gump(game: &Game, id: &Arg) -> Option<uoterm_protocol::OpenGump> {
    let want = if id.is(ANY) {
        None
    } else {
        id.number().filter(|&n| n != 0)
    };
    game.world()
        .gumps
        .iter()
        .rev()
        .find(|g| want.map_or(true, |w| i64::from(g.gump_id) == w))
        .cloned()
}

fn wait_for_gump(game: &Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let open = find_gump(game, need(call, 0, "a gump id or any")?).is_some();
    wait_until(call, ctx, 1, "no gump came", open)
}

fn reply_gump(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let id = need(call, 0, "a gump id or any")?;
    let button = need_number(call, 1, "a button")?;
    let button = u32::try_from(button).map_err(|_| format!("{button} is not a button"))?;
    let switches: Vec<u32> = call.args[2..]
        .iter()
        .map(|a| {
            a.number()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("'{}' is not a switch", a.text))
        })
        .collect::<std::result::Result<_, _>>()?;
    let Some(gump) = find_gump(game, id) else {
        Game::note(call, ctx, "no such gump is open");
        return Ok(Step::Done);
    };
    game.inner.outbound.push_back(encode::gump_response(
        gump.serial,
        gump.gump_id,
        button,
        &switches,
    ));
    game.inner.world.write().close_gump(gump.gump_id);
    Ok(Step::Acted)
}

/// Closes a container on the client side. A client with no window shows no
/// other kind of gump, so only containers are closed.
fn close_gump(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let kind = need(call, 0, "a gump kind")?;
    if !kind.is("container") {
        return Err(format!(
            "only container gumps close here, not '{}'",
            kind.text
        ));
    }
    let serial = game.serial(need(call, 1, "a container")?, ctx)?;
    game.inner.world.write().containers.remove(&serial);
    Ok(Step::Done)
}

// ---------- journal ----------

fn wait_for_journal_line(
    game: &Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    let text = &need(call, 0, "text")?.text;
    let author = call.args.get(2).map(|a| a.text.as_str());
    let heard = super::values::journal_holds(game, text, author);
    wait_until(call, ctx, 1, "the words did not come", heard)
}

// ---------- scripts ----------

/// Runs another script by name in place of this one.
fn play_macro(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let name = call
        .args
        .iter()
        .map(|a| a.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        return Err("playmacro needs a script name".into());
    }
    game.inner.scripting.next = Some(Next::Run(name));
    Ok(Step::Wait)
}

/// `script 'run'|'stop'|'isrunning' [name] [alias]`. One script runs at a
/// time, so "suspend" and "resume" have nothing to hold.
fn script_control(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    let action = need(call, 0, "run, stop or isrunning")?
        .text
        .to_ascii_lowercase();
    let name = call.args.get(1).map(|a| a.text.clone());
    match action.as_str() {
        "run" => {
            let name = name.ok_or("script run needs a script name")?;
            game.inner.scripting.next = Some(Next::Run(name));
            Ok(Step::Wait)
        }
        "stop" => {
            game.inner.scripting.next = Some(Next::Stop);
            Ok(Step::Wait)
        }
        "isrunning" | "issuspended" => {
            let this = game.inner.scripting.current.clone().unwrap_or_default();
            let asked = name.clone().unwrap_or_else(|| this.clone());
            let running = action == "isrunning" && asked.eq_ignore_ascii_case(&this);
            let alias = call
                .args
                .get(2)
                .map(|a| a.text.clone())
                .unwrap_or_else(|| format!("{asked}_{}", &action[2..]));
            ctx.vars.set_alias(&alias, u32::from(running));
            Ok(Step::Done)
        }
        "suspend" | "resume" => {
            Err("one script runs at a time; there is nothing to suspend".into())
        }
        other => Err(format!("'{other}' is not run, stop or isrunning")),
    }
}

fn location(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let serial = match call.args.first() {
        Some(a) => game.serial(a, ctx)?,
        None => game.me(),
    };
    let at = game
        .world()
        .map_location(serial)
        .ok_or_else(|| format!("{serial} is not in sight"))?;
    ctx.say(format!("{serial}: {} {} {}", at.x, at.y, at.z));
    Ok(Step::Done)
}

// ---------- others ----------

fn paperdoll(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = match call.args.first() {
        Some(a) => game.serial(a, ctx)?,
        None => game.me(),
    };
    Ok(double_click(game, Serial(serial.0 | PAPERDOLL_REQUEST_BIT)))
}

fn virtue(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let word = need(call, 0, "a virtue")?;
    let id = VIRTUES
        .iter()
        .find(|(name, _)| word.is(name))
        .map(|&(_, id)| id)
        .ok_or_else(|| format!("'{}' is not honor, sacrifice or valor", word.text))?;
    game.inner.outbound.push_back(encode::invoke_virtue(id));
    Ok(Step::Acted)
}

/// Says a line. A script's words are the player's own commands, so they go
/// as written: with the keyword numbers a shard listens for, and without the
/// chat budget or the no-repeat rule that guards an agent's small talk.
fn say(game: &mut Game, call: &Call, kind: u8) -> std::result::Result<Step, String> {
    let text = text_arg(call, 0, TEXT_MAX)?;
    let keywords = if kind == SPEECH_REGULAR {
        game.inner
            .speech_data
            .as_ref()
            .map(|table| table.keywords(game.inner.version, text))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let text = game
        .inner
        .persona
        .filter_speech(text)
        .ok_or(SPEECH_REJECTED)?;
    game.inner.outbound.push_back(encode::keyword_speech(
        kind,
        DEFAULT_SPEECH_HUE,
        &keywords,
        &text,
    ));
    Ok(Step::Acted)
}

fn party_message(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let text = text_arg(call, 0, TEXT_MAX)?;
    // `partymsg text [color] [serial]`: a serial makes it a private line
    // to that member. A lone small number after the text is a colour,
    // which party lines do not carry.
    let to = match (call.args.get(1), call.args.get(2)) {
        (_, Some(a)) => Some(game.serial(a, ctx)?),
        (Some(a), None) if a.number().map_or(true, |n| n > i64::from(u16::MAX)) => {
            Some(game.serial(a, ctx)?)
        }
        _ => None,
    };
    game.inner
        .outbound
        .push_back(encode::party_message(to, text));
    game.inner.world.write().spoken_to.mark_answered();
    Ok(Step::Acted)
}

/// `timermsg delay text`: the text shows after the delay, and the script
/// goes on at once.
fn timer_message(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let delay = need_number(call, 0, "a delay in milliseconds")?;
    let text = need(call, 1, "text")?.text.clone();
    game.inner.scripting.later.push((
        Instant::now() + Duration::from_millis(delay.max(0) as u64),
        text,
    ));
    Ok(Step::Done)
}

fn prompt_message(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    // A shard drops a longer answer and keeps its prompt open, so it is
    // refused here, before the prompt is marked answered.
    let text = text_arg(call, 0, PROMPT_TEXT_MAX)?.to_string();
    let Some(prompt) = game.inner.world.write().prompt.take() else {
        Game::note(call, ctx, "no prompt is open");
        return Ok(Step::Done);
    };
    game.inner
        .outbound
        .push_back(encode::prompt_response(prompt, &text, true));
    Ok(Step::Acted)
}

fn cancel_prompt(game: &mut Game) -> std::result::Result<Step, String> {
    let Some(prompt) = game.inner.world.write().prompt.take() else {
        return Ok(Step::Done);
    };
    game.inner
        .outbound
        .push_back(encode::prompt_response(prompt, "", false));
    Ok(Step::Acted)
}

/// The menu choice a script names: a client text number, a place in the
/// menu, or words. A number from [`FIRST_CLILOC_NUMBER`] up is a client text
/// number; a smaller one is a place.
fn menu_choice(arg: &Arg) -> MenuChoice {
    let number = arg
        .number()
        .filter(|_| !arg.quoted || arg.text.chars().all(|c| c.is_ascii_digit()));
    if let Some(cliloc) = number
        .filter(|&n| n >= FIRST_CLILOC_NUMBER)
        .and_then(|n| u32::try_from(n).ok())
    {
        return MenuChoice::Cliloc(cliloc);
    }
    match number.and_then(|n| u16::try_from(n).ok()) {
        Some(index) => MenuChoice::Index(index),
        None => MenuChoice::Text(arg.text.clone()),
    }
}

fn context_menu(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "an object")?, ctx)?;
    let choice = menu_choice(need(call, 1, "a menu entry")?);
    game.inner.pending_context_menu = Some((serial, choice));
    game.inner
        .outbound
        .push_back(encode::context_menu_request(serial));
    Ok(Step::Acted)
}

/// Asks for the menu once, then waits until it has been answered.
fn wait_for_context(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    if first_run(ctx) {
        context_menu(game, call, ctx)?;
        return Ok(Step::Wait);
    }
    let answered = game.inner.pending_context_menu.is_none();
    wait_until(call, ctx, 2, "the menu did not come", answered)
}

fn set_skill(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let skill = need(call, 0, "a skill name")?;
    let id = game
        .inner
        .scripting
        .skills
        .find(&skill.text)
        .map(|s| s.id)
        .ok_or_else(|| format!("no skill named '{}'", skill.text))?;
    let word = need(call, 1, "up, down or locked")?;
    let lock = LOCKS
        .iter()
        .find(|(w, _)| word.is(w))
        .map(|&(_, l)| l)
        .ok_or_else(|| format!("'{}' is not up, down or locked", word.text))?;
    game.inner.outbound.push_back(encode::skill_lock(id, lock));
    Ok(Step::Acted)
}

/// Asks for an object's property list once, then waits for it.
fn wait_for_properties(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "an object")?, ctx)?;
    if first_run(ctx) {
        game.inner
            .outbound
            .push_back(encode::batch_query_properties(&[serial]));
        return Ok(Step::Wait);
    }
    let came = game.world().properties.contains_key(&serial);
    wait_until(call, ctx, 1, "no properties came", came)
}

/// Answers the next dye tub request with this colour.
fn auto_color_pick(game: &mut Game, call: &Call) -> std::result::Result<Step, String> {
    let hue = need_number(call, 0, "a colour")?;
    let hue = u16::try_from(hue).map_err(|_| format!("{hue} is not a colour"))?;
    game.inner.scripting.dye_hue = Some(hue);
    Ok(Step::Done)
}

/// A map tile from script numbers, refused when a number is off the map.
fn map_point(x: i64, y: i64, z: i64) -> std::result::Result<Point3, String> {
    match (u16::try_from(x), u16::try_from(y), i8::try_from(z)) {
        (Ok(x), Ok(y), Ok(z)) => Ok(Point3::new(x, y, z)),
        _ => Err(format!("{x} {y} {z} is not a tile on the map")),
    }
}

/// The tile these steps away from a place.
fn offset_point(from: Point3, dx: i64, dy: i64, dz: i64) -> std::result::Result<Point3, String> {
    let add = |a: i64, b: i64| {
        a.checked_add(b)
            .ok_or_else(|| "the offset is too far".to_string())
    };
    map_point(
        add(i64::from(from.x), dx)?,
        add(i64::from(from.y), dy)?,
        add(i64::from(from.z), dz)?,
    )
}

/// Opens a container once, then waits for what is in it.
fn wait_for_contents(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a container")?, ctx)?;
    let open = game.world().containers.contains_key(&serial);
    // The double-click goes again until it is sent: pacing can hold the
    // first one back.
    if !open && game.inner.last_object != Some(serial) {
        return Ok(match double_click(game, serial) {
            Step::Fail(e) => Step::Fail(e),
            _ => Step::Wait,
        });
    }
    wait_until(call, ctx, 1, "the container did not open", open)
}

// ---------- spells ----------

/// Casts a spell by name or number, or `last`. With a target after it, the
/// cursor the spell opens is answered with that target.
fn cast(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let spell = need(call, 0, "a spell")?;
    let id = if spell.is(LAST) {
        game.inner
            .scripting
            .last_spell
            .ok_or("no spell was cast yet")?
    } else {
        game.inner
            .scripting
            .spells
            .find(&spell.text)
            .map(|s| s.id)
            .ok_or_else(|| format!("no spell named '{}'", spell.text))?
    };
    let target = match call.args.get(1) {
        Some(a) => Some(game.serial(a, ctx)?),
        None => None,
    };
    Ok(send_cast(game, id, target, ctx.now))
}

fn send_cast(game: &mut Game, spell: u16, target: Option<Serial>, now: Instant) -> Step {
    if !ready(game) {
        return Step::Wait;
    }
    if !clear_hands_for_cast(game.inner, spell) {
        // The other hand goes at the next action; the cast waits for it.
        return Step::Wait;
    }
    game.inner.outbound.push_back(encode::cast_spell(spell));
    note_cast(game.inner, spell);
    game.inner.scripting.last_spell = Some(spell);
    if let Some(target) = target {
        queue_target(game.inner, target, now);
    }
    mark_action(game.inner);
    Step::Acted
}

/// A heal spell on a target, or its cure when the target is poisoned:
/// `(heal, cure)`.
fn heal(
    game: &mut Game,
    call: &Call,
    ctx: &Ctx,
    (heal, cure): (u16, u16),
) -> std::result::Result<Step, String> {
    let target = match call.args.first() {
        Some(a) => game.serial(a, ctx)?,
        None => game.me(),
    };
    let spell = if game.world().is_poisoned(target) {
        cure
    } else {
        heal
    };
    Ok(send_cast(game, spell, Some(target), ctx.now))
}

// ---------- targeting ----------

fn cancel_target(game: &mut Game) -> std::result::Result<Step, String> {
    game.inner.target_intent = None;
    let Some(cursor) = game.world().pending_target.clone() else {
        return Ok(Step::Done);
    };
    game.inner
        .outbound
        .push_back(encode::cancel_target(cursor.id));
    game.inner.world.write().clear_target();
    Ok(Step::Acted)
}

/// Answers the open cursor, or queues the answer for the next one. With `!`
/// nothing is queued: no cursor, no target.
fn aim_at(game: &mut Game, call: &Call, ctx: &mut Ctx, aim: Aim, lifetime: Duration) -> Step {
    let cursor = game.world().pending_target.clone();
    match cursor {
        Some(cursor) => {
            if let Aim::Ground { at, .. } = aim {
                game.inner.scripting.last_ground = Some(at);
            }
            answer_cursor(game.inner, cursor.id, aim);
            Step::Acted
        }
        None if call.force => {
            Game::note(call, ctx, "no target cursor is open");
            Step::Done
        }
        None => {
            queue_aim(game.inner, aim, lifetime, ctx.now);
            Step::Done
        }
    }
}

fn target(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let arg = need(call, 0, "a target")?;
    let serial = game.serial(arg, ctx)?;
    if arg.is(LAST) || arg.is("lasttarget") {
        check_last_target_range(game.inner, serial)?;
    }
    let lifetime = call
        .args
        .get(1)
        .map_or(TARGET_QUEUE_LIFETIME, |_| wait_time(call, 1));
    Ok(aim_at(game, call, ctx, Aim::Object(serial), lifetime))
}

/// The nearest object of a graphic: an item in the pack, then the ground,
/// then a mobile with that body. `ground_only` looks on the ground only.
fn nearest_of_type(
    game: &Game,
    call: &Call,
    ground_only: bool,
) -> std::result::Result<Option<Serial>, String> {
    let graphic = Game::graphic(need(call, 0, "a graphic")?)?;
    let color = Game::color(call.args.get(1))?;
    let range = Game::range(call.args.get(2))?;
    if !ground_only {
        if let Some(&s) = game
            .find_items(graphic, color, Source::Backpack, range)
            .first()
        {
            return Ok(Some(s));
        }
    }
    if let Some(&s) = game
        .find_items(graphic, color, Source::Ground, range)
        .first()
    {
        return Ok(Some(s));
    }
    Ok(if ground_only {
        None
    } else {
        game.find_mobiles(graphic, color, range).first().copied()
    })
}

fn target_type(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    ground_only: bool,
) -> std::result::Result<Step, String> {
    match nearest_of_type(game, call, ground_only)? {
        Some(serial) => Ok(aim_at(
            game,
            call,
            ctx,
            Aim::Object(serial),
            TARGET_QUEUE_LIFETIME,
        )),
        None => {
            Game::note(call, ctx, "none found");
            Ok(Step::Done)
        }
    }
}

fn auto_target_type(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    ground_only: bool,
) -> std::result::Result<Step, String> {
    match nearest_of_type(game, call, ground_only)? {
        Some(serial) => {
            queue_target(game.inner, serial, ctx.now);
            Ok(Step::Done)
        }
        None => {
            Game::note(call, ctx, "none found");
            Ok(Step::Done)
        }
    }
}

/// The height of the ground at a tile, from the map.
fn ground_z(game: &mut Game, x: u16, y: u16) -> i8 {
    let from = game.world().self_state.location.z;
    game.inner.ensure_facet();
    game.inner.tiles().tile_from(from, x, y).z
}

fn static_graphic(call: &Call, i: usize) -> std::result::Result<u16, String> {
    match call.args.get(i) {
        Some(a) => Game::graphic(a),
        None => Ok(BARE_LAND_GRAPHIC),
    }
}

/// `targettile 'last'|'current'|x y z [graphic]`.
fn target_tile(game: &mut Game, call: &Call, ctx: &mut Ctx) -> std::result::Result<Step, String> {
    let first = need(call, 0, "last, current, or x y z")?;
    let (at, graphic_at) = if first.is(LAST) {
        let at = game
            .inner
            .scripting
            .last_ground
            .ok_or("no tile was targeted yet")?;
        (at, 1)
    } else if first.is("current") {
        (game.world().self_state.location, 1)
    } else {
        let at = map_point(
            need_number(call, 0, "an x")?,
            need_number(call, 1, "a y")?,
            0,
        )?;
        let z = match call.args.get(2).and_then(Arg::number) {
            Some(z) => map_point(i64::from(at.x), i64::from(at.y), z)?.z,
            None => ground_z(game, at.x, at.y),
        };
        (Point3::new(at.x, at.y, z), 3)
    };
    let graphic = static_graphic(call, graphic_at)?;
    Ok(aim_at(
        game,
        call,
        ctx,
        Aim::Ground { at, graphic },
        TARGET_QUEUE_LIFETIME,
    ))
}

/// `targettileoffset x y z [graphic]`: a tile counted from the character.
fn target_tile_offset(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    queue_only: bool,
) -> std::result::Result<Step, String> {
    let (dx, dy, dz) = xyz(
        need(call, 0, "an x offset")?,
        need(call, 1, "a y offset")?,
        need(call, 2, "a z offset")?,
    )?;
    let here = game.world().self_state.location;
    let at = offset_point(here, dx, dy, dz)?;
    let aim = Aim::Ground {
        at,
        graphic: static_graphic(call, 3)?,
    };
    Ok(if queue_only {
        queue_aim(game.inner, aim, TARGET_QUEUE_LIFETIME, ctx.now);
        Step::Done
    } else {
        aim_at(game, call, ctx, aim, TARGET_QUEUE_LIFETIME)
    })
}

/// `targettilerelative serial range [reverse] [graphic]`: the tile `range`
/// steps in front of a mobile, or behind it.
fn target_tile_relative(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    queue_only: bool,
) -> std::result::Result<Step, String> {
    let serial = game.serial(need(call, 0, "a mobile")?, ctx)?;
    let range = need_number(call, 1, "a range")?;
    let reverse = call.args.get(2).is_some_and(|a| a.is("true"));
    let graphic = static_graphic(call, 3)?;
    let (from, facing) = {
        let w = game.world();
        if serial == w.self_state.serial {
            (w.self_state.location, w.self_state.direction)
        } else {
            let m = w
                .mobiles
                .get(&serial)
                .ok_or_else(|| format!("{serial} is not in sight"))?;
            (m.location, m.direction)
        }
    };
    let (dx, dy) = Direction::from_byte(facing).delta();
    let steps = if reverse {
        range.checked_neg()
    } else {
        Some(range)
    };
    let at = steps
        .and_then(|n| Some((i64::from(dx).checked_mul(n)?, i64::from(dy).checked_mul(n)?)))
        .ok_or_else(|| format!("{range} is not a range"))
        .and_then(|(x, y)| offset_point(from, x, y, 0))?;
    let at = Point3::new(at.x, at.y, ground_z(game, at.x, at.y));
    let aim = Aim::Ground { at, graphic };
    Ok(if queue_only {
        queue_aim(game.inner, aim, TARGET_QUEUE_LIFETIME, ctx.now);
        Step::Done
    } else {
        aim_at(game, call, ctx, aim, TARGET_QUEUE_LIFETIME)
    })
}

/// `targetresource tool resource`: uses a tool on ore, sand, wood, graves
/// or red mushrooms (or a shard's own number) with no cursor.
fn target_resource(game: &mut Game, call: &Call, ctx: &Ctx) -> std::result::Result<Step, String> {
    let tool = game.serial(need(call, 0, "a tool")?, ctx)?;
    let what = need(call, 1, "a resource")?;
    let resource = match what.number() {
        Some(n) => u16::try_from(n).map_err(|_| format!("{n} is not a resource"))?,
        None => RESOURCES
            .iter()
            .position(|r| name_key(r) == name_key(&what.text))
            .map(|i| i as u16)
            .ok_or_else(|| {
                format!(
                    "'{}' is not ore, sand, wood, graves or red mushrooms",
                    what.text
                )
            })?,
    };
    if !ready(game) {
        return Ok(Step::Wait);
    }
    game.inner
        .outbound
        .push_back(encode::resource_target(tool, resource));
    mark_action(game.inner);
    Ok(Step::Acted)
}

/// Queues the nearest ghost in range for the next cursor.
fn auto_target_ghost(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
) -> std::result::Result<Step, String> {
    let range = Game::range(call.args.first())?;
    let ghost = {
        let w = game.world();
        let here = w.self_state.location;
        w.mobiles
            .values()
            .filter(|m| uoterm_world::is_ghost_body(m.body) && here.chebyshev(m.location) <= range)
            .min_by_key(|m| (here.chebyshev(m.location), m.serial.0))
            .map(|m| m.serial)
    };
    match ghost {
        Some(serial) => {
            queue_target(game.inner, serial, ctx.now);
            Ok(Step::Done)
        }
        None => {
            Game::note(call, ctx, "no ghost in range");
            Ok(Step::Done)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pick {
    Enemy,
    Friend,
}

/// `getenemy` and `getfriend`: pick a mobile by notoriety words and filter
/// words, and set the `enemy` or `friend` alias. `closest` takes the nearest;
/// `nearest` takes turns between the two nearest; with neither, each call
/// moves on to the next match, nearest first.
fn pick_mobile(
    game: &mut Game,
    call: &Call,
    ctx: &mut Ctx,
    pick: Pick,
) -> std::result::Result<Step, String> {
    let alias = match pick {
        Pick::Enemy => "enemy",
        Pick::Friend => "friend",
    };
    let mut wanted: Vec<u8> = Vec::new();
    let (mut humanoid, mut transformed, mut closest, mut nearest) = (false, false, false, false);
    for word in &call.args {
        match word.text.to_ascii_lowercase().as_str() {
            "humanoid" => humanoid = true,
            "transformation" => transformed = true,
            "closest" => closest = true,
            "nearest" => nearest = true,
            "invulnerable" if pick == Pick::Enemy => {}
            other => wanted.extend_from_slice(notorieties(other)),
        }
    }
    let candidates: Vec<Serial> = {
        let w = game.world().clone();
        let here = w.self_state.location;
        let s = &game.inner.scripting;
        let agents = &game.inner.agents;
        let mut list: Vec<&uoterm_world::Mobile> = w
            .mobiles
            .values()
            .filter(|m| m.serial != w.self_state.serial && !s.ignored.contains(&m.serial))
            .filter(|m| wanted.is_empty() || wanted.contains(&m.notoriety))
            .filter(|m| !humanoid || is_humanoid(m.body))
            .filter(|m| !transformed || is_transformed(m.body))
            .filter(|m| pick == Pick::Friend || !agents.is_friend(&w, m.serial))
            .collect();
        list.sort_by_key(|m| (here.chebyshev(m.location), m.serial.0));
        list.into_iter().map(|m| m.serial).collect()
    };
    if (closest || nearest) && !shard_allows(game.inner, AssistFeature::ClosestTargets) {
        return Err(forbidden(AssistFeature::ClosestTargets));
    }
    let current = ctx.vars.alias(alias).map(Serial);
    let chosen = if closest {
        candidates.first().copied()
    } else if nearest {
        match candidates.as_slice() {
            [a, b, ..] if current == Some(*a) => Some(*b),
            [a, ..] => Some(*a),
            [] => None,
        }
    } else {
        let at = current.and_then(|c| candidates.iter().position(|&s| s == c));
        match at {
            Some(i) => candidates.get((i + 1) % candidates.len()).copied(),
            None => candidates.first().copied(),
        }
    };
    match chosen {
        Some(serial) => {
            ctx.vars.set_alias(alias, serial.0);
            game.inner.last_target = Some(serial);
            if !call.quiet {
                let name = game.world().name_of(serial);
                ctx.say(format!("[{alias}] {name}"));
            }
        }
        None => {
            ctx.vars.unset_alias(alias);
            Game::note(call, ctx, format!("no {alias} found"));
        }
    }
    Ok(Step::Done)
}
