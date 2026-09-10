//! What each agent does on its tick. Each function returns true when it
//! sent something, so the tick stops there and the next agent waits.

use uoterm_assist::mobiles::{is_humanoid, notorieties};

use super::config::{HealWhom, ItemRule, PropertyRule, Selector};
use super::*;

/// A corpse's item graphic.
const CORPSE_GRAPHIC: u16 = 0x2006;
/// Free weight, in stones, under which the loot and scavenge agents stop.
const MIN_FREE_WEIGHT: u16 = 5;
/// The most times a job moves one item. The shard may refuse a move, for
/// example when it comes too soon after another action.
pub(super) const MAX_JOB_MOVES: u8 = 3;
/// How far a blade reaches: corpses and bones next to the character.
const BLADE_REACH: u32 = 1;
/// How far the bandage agent looks for a friend when its range is unset.
const HEAL_SIGHT: u32 = 18;
/// The health share the bandage agent measures against.
const PERCENT: u32 = 100;

fn delay(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

/// The character's backpack, or the character when the pack is unknown.
fn pack_or_self(inner: &Inner) -> Serial {
    let w = inner.world.read();
    backpack_serial(&w).unwrap_or(w.self_state.serial)
}

/// True when the pack can take a little more weight.
fn room_for_more(inner: &Inner) -> bool {
    let s = &inner.world.read().self_state;
    s.weight_max == 0 || s.weight_max.saturating_sub(s.weight) >= MIN_FREE_WEIGHT
}

fn hidden(inner: &Inner) -> bool {
    inner.world.read().self_state.hidden
}

/// Moves an item and charges the agent's delay.
fn move_now(
    inner: &mut Inner,
    item: Serial,
    amount: u16,
    into: Serial,
    wait: Duration,
    now: Instant,
) {
    lift_and_drop(inner, item, amount, DropAt::Into(into));
    inner.agents.next_move_at = now + wait;
}

/// True when an item passes a rule's property ranges. When the item's
/// property list is not known yet it is asked for, and the item waits.
fn passes_properties(inner: &mut Inner, item: Serial, rules: &[PropertyRule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    if !inner.world.read().properties.contains_key(&item) {
        if inner.agents.asked_properties.insert(item) {
            inner
                .outbound
                .push_back(encode::batch_query_properties(&[item]));
        }
        return false;
    }
    let lines = property_lines(inner, item);
    rules.iter().all(|rule| {
        let want = rule.name.to_lowercase();
        lines
            .iter()
            .find(|l| l.to_lowercase().contains(&want))
            .is_some_and(|line| {
                let value = first_number(line).unwrap_or(1.0);
                rule.min.map_or(true, |m| value >= m) && rule.max.map_or(true, |m| value <= m)
            })
    })
}

/// The first rule an item matches, if any.
fn rule_for(rules: &[ItemRule], graphic: u16, color: u16) -> Option<&ItemRule> {
    rules.iter().find(|r| r.matches(graphic, color))
}

// ---------- options ----------

/// Wears again the item a potion took out of the hand, once its time comes.
pub(super) fn rearm(inner: &mut Inner, now: Instant) -> bool {
    let Some((item, layer, due)) = inner.agents.rearm else {
        return false;
    };
    if now < due {
        return false;
    }
    inner.agents.rearm = None;
    let (worn, exists) = {
        let w = inner.world.read();
        (w.worn(layer).is_some(), w.items.contains_key(&item))
    };
    if worn || !exists {
        return false;
    }
    lift_and_wear(inner, item, layer);
    true
}

/// Graphics of the ore, logs and fish the stack option drops at the feet.
const STACKED_AT_FEET: [std::ops::RangeInclusive<u16>; 3] =
    [0x19B7..=0x19BA, 0x1BDD..=0x1BE2, 0x09CC..=0x09CF];

/// Drops ore, logs and fish from the pack at the character's feet, where
/// they stack, when the option says to.
pub(super) fn stack_at_feet(inner: &mut Inner, now: Instant) -> bool {
    if !inner.agents.config.options.stack_at_feet {
        return false;
    }
    let (item, at) = {
        let w = inner.world.read();
        let item = backpack_serial(&w).and_then(|pack| {
            w.items_inside(pack, false)
                .into_iter()
                .find(|i| STACKED_AT_FEET.iter().any(|r| r.contains(&i.graphic)))
                .map(|i| (i.serial, i.amount))
        });
        (item, w.self_state.location)
    };
    let Some((item, amount)) = item else {
        return false;
    };
    lift_and_drop(inner, item, amount.max(1), DropAt::Ground(at));
    inner.agents.next_move_at = now + delay(config::DEFAULT_DELAY_MS);
    true
}

// ---------- remount ----------

pub(super) fn remount(inner: &mut Inner, now: Instant) -> bool {
    if !shard_allows(inner, AssistFeature::AutoRemount) {
        return false;
    }
    let r = inner.agents.config.remount.clone();
    let Some(mount) = r.mount.filter(|_| r.enabled) else {
        return false;
    };
    // A mount not in sight cannot be clicked; asking again each second only
    // spams the shard.
    let seen = {
        let w = inner.world.read();
        w.mobiles.contains_key(&mount) || w.items.contains_key(&mount)
    };
    if !seen {
        return false;
    }
    let (mounted, body) = {
        let w = inner.world.read();
        (w.worn(LAYER_MOUNT).is_some(), w.self_state.body)
    };
    if mounted || !is_humanoid(body) {
        inner.agents.unmounted_since = None;
        return false;
    }
    let since = *inner.agents.unmounted_since.get_or_insert(now);
    if now.saturating_duration_since(since) < delay(r.delay_ms) {
        return false;
    }
    if send_double_click(inner, mount) {
        inner.agents.unmounted_since = None;
        return true;
    }
    false
}

// ---------- bandage ----------

pub(super) fn bandage(inner: &mut Inner, now: Instant) -> bool {
    if !shard_allows(inner, AssistFeature::AutoBandage) {
        return false;
    }
    let b = inner.agents.config.bandage.clone();
    if !b.enabled || now < inner.next_bandage_at || (b.skip_when_hidden && hidden(inner)) {
        return false;
    }
    let world = inner.world.read().clone();
    let hurt = |hits: u16, max: u16| {
        max > 0 && u32::from(hits) * PERCENT < u32::from(max) * u32::from(b.hp_pct)
    };
    let me = world.self_state.serial;
    let self_hurt = hurt(world.self_state.hits, world.self_state.hits_max);
    let range = if b.range == 0 { HEAL_SIGHT } else { b.range };
    let friend = || {
        world
            .mobiles
            .values()
            .filter(|m| inner.agents.is_friend(&world, m.serial))
            .filter(|m| world.self_state.location.chebyshev(m.location) <= range)
            .filter_map(|m| Some((m.serial, m.hits?, m.hits_max?)))
            .filter(|&(_, hits, max)| hurt(hits, max))
            .min_by_key(|&(_, hits, max)| u32::from(hits) * PERCENT / u32::from(max.max(1)))
            .map(|(serial, _, _)| serial)
    };
    let target = match b.whom {
        HealWhom::SelfOnly => self_hurt.then_some(me),
        HealWhom::Friend => friend(),
        HealWhom::FriendOrSelf => friend().or(self_hurt.then_some(me)),
        HealWhom::Target(serial) => world
            .mobiles
            .get(&serial)
            .and_then(|m| Some((m.hits?, m.hits_max?)))
            .filter(|&(hits, max)| hurt(hits, max))
            .map(|_| serial),
    };
    let Some(target) = target else {
        return false;
    };
    if b.skip_poisoned && world.is_poisoned(target) {
        return false;
    }
    let graphic = b.bandage_graphic.unwrap_or(GRAPHIC_BANDAGE);
    let pack = backpack_serial(&world);
    let Some(bandage) = world
        .items
        .values()
        .filter(|i| i.graphic == graphic && b.bandage_color.map_or(true, |c| i.hue == c))
        .filter(|i| pack.is_some_and(|p| world.is_inside(i.serial, p)))
        .map(|i| i.serial)
        .min_by_key(|s| s.0)
    else {
        return false;
    };
    inner
        .outbound
        .push_back(encode::bandage_target(bandage, target));
    mark_action(inner);
    let wait = b
        .delay_ms
        .map_or_else(|| delay(bandage_self_ms(world.self_state.dex)), delay);
    inner.next_bandage_at = now + wait;
    true
}

// ---------- jobs ----------

pub(super) fn job(inner: &mut Inner, now: Instant) -> bool {
    let Some(job) = inner.agents.job.clone() else {
        return false;
    };
    let step = match &job {
        Job::Organize { list } => organize(inner, list, now),
        Job::Restock { list } => restock(inner, list, now),
        Job::Dress { list } => dress(inner, list, now),
        Job::Undress { list } => undress(inner, list.as_deref(), now),
        Job::LootOnce => {
            if autoloot(inner, now, true) {
                JobStep::Acted
            } else if !action_ready(inner) || now < inner.next_double_click_at {
                // Pacing held a double-click back: the job is not done.
                JobStep::Waiting
            } else {
                JobStep::Finished
            }
        }
    };
    match step {
        JobStep::Acted => true,
        JobStep::Waiting => false,
        JobStep::Moved(serial) => {
            *inner.agents.job_moves.entry(serial).or_default() += 1;
            true
        }
        JobStep::Finished => {
            tracing::info!(job = job.name(), "agent job finished");
            inner.agents.job = None;
            false
        }
        JobStep::Failed(why) => {
            tracing::warn!(job = job.name(), why = %why, "agent job stopped");
            inner.agents.job = None;
            false
        }
    }
}

enum JobStep {
    Acted,
    /// Nothing sent now; the job goes on at a later tick.
    Waiting,
    /// Moved this item; the job counts the move.
    Moved(Serial),
    Finished,
    Failed(String),
}

/// True while a job may still move this item. An item the shard refuses
/// to move stays put; after a few tries the job leaves it.
fn may_move(inner: &Inner, item: Serial) -> bool {
    inner.agents.job_moves.get(&item).map_or(true, |&n| n < MAX_JOB_MOVES)
}

/// Moves each listed item from the source bag to the destination, split to
/// its amount when the rule names one.
fn organize(inner: &mut Inner, name: &str, now: Instant) -> JobStep {
    let Some(list) = inner.agents.config.organizer.get(name).cloned() else {
        return JobStep::Failed(format!("no organizer list '{name}'"));
    };
    let source = list.source.unwrap_or_else(|| pack_or_self(inner));
    let Some(dest) = list.destination else {
        return JobStep::Failed("the organizer list names no destination bag".into());
    };
    let next = {
        let w = inner.world.read();
        w.items_inside(source, false)
            .into_iter()
            .filter(|i| may_move(inner, i.serial))
            .find_map(|i| {
                let rule = rule_for(&list.items, i.graphic, i.hue)?;
                let amount = rule
                    .amount
                    .map_or(i.amount, |a| (a.min(u32::from(i.amount))) as u16);
                Some((i.serial, amount))
            })
    };
    match next {
        Some((item, amount)) => {
            move_now(inner, item, amount.max(1), dest, delay(list.delay_ms), now);
            JobStep::Moved(item)
        }
        None => JobStep::Finished,
    }
}

/// Tops each listed item in the destination up to its amount, from the
/// source bag.
fn restock(inner: &mut Inner, name: &str, now: Instant) -> JobStep {
    if !shard_allows(inner, AssistFeature::RestockAgent) {
        return JobStep::Failed(forbidden(AssistFeature::RestockAgent));
    }
    let Some(list) = inner.agents.config.restock.get(name).cloned() else {
        return JobStep::Failed(format!("no restock list '{name}'"));
    };
    let (source, dest) = {
        let w = inner.world.read();
        let source = list.source.or_else(|| w.bank_box());
        let dest = list.destination.or_else(|| backpack_serial(&w));
        (source, dest)
    };
    let (Some(source), Some(dest)) = (source, dest) else {
        return JobStep::Failed("restock needs a source bag (or an open bank) and a pack".into());
    };
    let next = {
        let w = inner.world.read();
        list.items.iter().filter(|r| !r.disabled).find_map(|rule| {
            let limit = rule.amount?;
            let have: u32 = w
                .items_inside(dest, true)
                .into_iter()
                .filter(|i| rule.matches(i.graphic, i.hue))
                .map(|i| u32::from(i.amount.max(1)))
                .sum();
            let short = limit.checked_sub(have).filter(|&s| s > 0)?;
            w.items_inside(source, true)
                .into_iter()
                .filter(|i| may_move(inner, i.serial))
                .find(|i| rule.matches(i.graphic, i.hue))
                .map(|i| (i.serial, short.min(u32::from(i.amount)) as u16))
        })
    };
    match next {
        Some((item, amount)) => {
            move_now(inner, item, amount.max(1), dest, delay(list.delay_ms), now);
            JobStep::Moved(item)
        }
        None => JobStep::Finished,
    }
}

/// Puts on each item of a dress list, taking off what is in its way.
fn dress(inner: &mut Inner, name: &str, now: Instant) -> JobStep {
    let Some(list) = inner.agents.config.dress.get(name).cloned() else {
        return JobStep::Failed(format!("no dress list '{name}'"));
    };
    let undress_bag = list.undress_bag.unwrap_or_else(|| pack_or_self(inner));
    for entry in &list.items {
        let (worn, exists) = {
            let w = inner.world.read();
            (
                w.worn(entry.layer).map(|e| e.serial),
                w.items.contains_key(&entry.serial),
            )
        };
        match worn {
            Some(on) if on == entry.serial => continue,
            Some(other) if list.replace_worn && may_move(inner, other) => {
                move_now(
                    inner,
                    other,
                    ONE_WORN_ITEM,
                    undress_bag,
                    delay(list.delay_ms),
                    now,
                );
                return JobStep::Moved(other);
            }
            Some(_) => continue,
            None if exists && may_move(inner, entry.serial) => {
                lift_and_wear(inner, entry.serial, entry.layer);
                inner.agents.next_move_at = now + delay(list.delay_ms);
                return JobStep::Moved(entry.serial);
            }
            None => continue,
        }
    }
    JobStep::Finished
}

/// Takes off the items of a dress list, or everything worn but the pack.
fn undress(inner: &mut Inner, name: Option<&str>, now: Instant) -> JobStep {
    let list = match name {
        Some(n) => match inner.agents.config.dress.get(n).cloned() {
            Some(l) => l,
            None => return JobStep::Failed(format!("no dress list '{n}'")),
        },
        None => DressList::default(),
    };
    let bag = list.undress_bag.unwrap_or_else(|| pack_or_self(inner));
    let next = {
        let w = inner.world.read();
        w.self_state
            .equipment
            .iter()
            .filter(|e| !NOT_DRESSED.contains(&e.layer))
            .filter(|e| may_move(inner, e.serial))
            .find(|e| name.is_none() || list.items.iter().any(|d| d.serial == e.serial))
            .map(|e| e.serial)
    };
    match next {
        Some(item) => {
            move_now(inner, item, ONE_WORN_ITEM, bag, delay(list.delay_ms), now);
            JobStep::Moved(item)
        }
        None => JobStep::Finished,
    }
}

// ---------- loot, scavenge ----------

/// Opens corpses in range and moves the items the active list wants. With
/// `once`, it runs even while the agent is switched off.
pub(super) fn autoloot(inner: &mut Inner, now: Instant, once: bool) -> bool {
    if !shard_allows(inner, AssistFeature::AutolootAgent) {
        return false;
    }
    let a = inner.agents.config.autoloot.clone();
    if (!a.enabled && !once) || (hidden(inner) && !a.while_hidden) || !room_for_more(inner) {
        return false;
    }
    let corpses: Vec<Serial> = {
        let w = inner.world.read();
        let here = w.self_state.location;
        w.items
            .values()
            .filter(|i| i.graphic == CORPSE_GRAPHIC && i.parent.is_none())
            .filter(|i| here.chebyshev(i.location) <= a.range)
            .map(|i| i.serial)
            .collect()
    };
    for corpse in corpses {
        if !a.no_open_corpse && !inner.agents.opened.contains(&corpse) {
            if send_double_click(inner, corpse) {
                inner.agents.opened.insert(corpse);
                return true;
            }
            return false;
        }
        let candidates: Vec<(Serial, u16, u16, u16)> = {
            let w = inner.world.read();
            w.items_inside(corpse, true)
                .into_iter()
                .filter(|i| !inner.agents.tried.contains(&i.serial))
                .map(|i| (i.serial, i.graphic, i.hue, i.amount))
                .collect()
        };
        for (item, graphic, hue, amount) in candidates {
            let Some(rule) = rule_for(a.items.rules(), graphic, hue).cloned() else {
                continue;
            };
            if !passes_properties(inner, item, &rule.properties) {
                continue;
            }
            let bag = rule.bag.or(a.bag).unwrap_or_else(|| pack_or_self(inner));
            inner.agents.tried.insert(item);
            move_now(inner, item, amount.max(1), bag, delay(a.delay_ms), now);
            return true;
        }
    }
    false
}

/// Picks up the ground items the active list wants.
pub(super) fn scavenge(inner: &mut Inner, now: Instant) -> bool {
    let s = inner.agents.config.scavenger.clone();
    if !s.enabled || (hidden(inner) && !s.while_hidden) || !room_for_more(inner) {
        return false;
    }
    let candidates: Vec<(Serial, u16, u16, u16)> = {
        let w = inner.world.read();
        let here = w.self_state.location;
        let mut found: Vec<&uoterm_world::Item> = w
            .items
            .values()
            .filter(|i| i.parent.is_none() && i.graphic != CORPSE_GRAPHIC)
            .filter(|i| here.chebyshev(i.location) <= s.range)
            .filter(|i| !inner.agents.tried.contains(&i.serial))
            .collect();
        found.sort_by_key(|i| (here.chebyshev(i.location), i.serial.0));
        found
            .into_iter()
            .map(|i| (i.serial, i.graphic, i.hue, i.amount))
            .collect()
    };
    for (item, graphic, hue, amount) in candidates {
        let Some(rule) = rule_for(s.items.rules(), graphic, hue).cloned() else {
            continue;
        };
        if !passes_properties(inner, item, &rule.properties) {
            continue;
        }
        let bag = rule.bag.or(s.bag).unwrap_or_else(|| pack_or_self(inner));
        inner.agents.tried.insert(item);
        move_now(inner, item, amount.max(1), bag, delay(s.delay_ms), now);
        return true;
    }
    false
}

// ---------- blades and corpses ----------

/// Uses the blade on the nearest new corpse next to the character.
pub(super) fn carve(inner: &mut Inner, now: Instant) -> bool {
    let c = inner.agents.config.carver.clone();
    let Some(blade) = c.blade.filter(|_| c.enabled) else {
        return false;
    };
    let corpse = nearest_ground(
        inner,
        |i| i.graphic == CORPSE_GRAPHIC,
        BLADE_REACH,
        &inner.agents.carved,
    );
    let Some(corpse) = corpse else {
        return false;
    };
    if !send_double_click(inner, blade) {
        return false;
    }
    queue_target(inner, corpse, now);
    inner.agents.carved.insert(corpse);
    true
}

/// Uses the blade on the nearest bone pile next to the character.
pub(super) fn cut_bones(inner: &mut Inner, now: Instant) -> bool {
    if !shard_allows(inner, AssistFeature::BoneCutterAgent) {
        return false;
    }
    let c = inner.agents.config.bone_cutter.clone();
    let Some(blade) = c.blade.filter(|_| c.enabled) else {
        return false;
    };
    let bones = nearest_ground(
        inner,
        |i| {
            (uoterm_assist::items::BONE_PILE_FIRST..=uoterm_assist::items::BONE_PILE_LAST)
                .contains(&i.graphic)
        },
        BLADE_REACH,
        &inner.agents.carved,
    );
    let Some(bones) = bones else {
        return false;
    };
    if !send_double_click(inner, blade) {
        return false;
    }
    queue_target(inner, bones, now);
    inner.agents.carved.insert(bones);
    true
}

/// Opens each new corpse in range.
pub(super) fn open_corpses(inner: &mut Inner, _now: Instant) -> bool {
    let o = inner.agents.config.open_corpses.clone();
    if !o.enabled || hidden(inner) {
        return false;
    }
    let corpse = nearest_ground(
        inner,
        |i| i.graphic == CORPSE_GRAPHIC,
        o.range,
        &inner.agents.opened,
    );
    match corpse {
        Some(corpse) if send_double_click(inner, corpse) => {
            inner.agents.opened.insert(corpse);
            true
        }
        _ => false,
    }
}

/// The nearest ground item that passes `wanted`, within `reach`, that is not
/// in `skip`.
fn nearest_ground(
    inner: &Inner,
    wanted: impl Fn(&uoterm_world::Item) -> bool,
    reach: u32,
    skip: &HashSet<Serial>,
) -> Option<Serial> {
    let w = inner.world.read();
    let here = w.self_state.location;
    w.items
        .values()
        .filter(|i| i.parent.is_none() && wanted(i) && !skip.contains(&i.serial))
        .filter(|i| here.chebyshev(i.location) <= reach)
        .min_by_key(|i| (here.chebyshev(i.location), i.serial.0))
        .map(|i| i.serial)
}

// ---------- vendors ----------

/// Buys the active list's items from a vendor's buy list. The prices come in
/// the order of the shop container's last content list.
pub(super) fn buy(
    inner: &mut Inner,
    container: Serial,
    entries: &[uoterm_protocol::VendorBuyEntry],
) {
    if !shard_allows(inner, AssistFeature::BuyAgent) {
        return;
    }
    let b = inner.agents.config.buy.clone();
    let Some((listed, stock)) = inner.agents.last_contents.clone() else {
        return;
    };
    if listed != container || stock.len() != entries.len() {
        tracing::debug!("the buy list does not line up with the shop's contents");
        return;
    }
    let (vendor, have) = {
        let w = inner.world.read();
        let vendor = w
            .mobiles
            .values()
            .find(|m| m.equipment.iter().any(|e| e.serial == container))
            .map(|m| m.serial)
            .or_else(|| w.items.get(&container).and_then(|i| i.parent));
        let pack = backpack_serial(&w);
        let have = |rule: &ItemRule| -> u32 {
            pack.map_or(0, |p| {
                w.items_inside(p, true)
                    .into_iter()
                    .filter(|i| rule.matches(i.graphic, i.hue))
                    .map(|i| u32::from(i.amount.max(1)))
                    .sum()
            })
        };
        let owned: Vec<u32> = b.items.rules().iter().map(have).collect();
        (vendor, owned)
    };
    let Some(vendor) = vendor else {
        return;
    };
    let mut bought: Vec<(Serial, u16)> = Vec::new();
    for (i, rule) in b.items.rules().iter().enumerate() {
        let want = rule.amount.unwrap_or(1);
        let want = if b.complete_amount {
            want.saturating_sub(have[i])
        } else {
            want
        };
        if want == 0 {
            continue;
        }
        if let Some(item) = stock.iter().find(|s| rule.matches(s.graphic, s.hue)) {
            bought.push((item.serial, want.min(u32::from(item.amount)) as u16));
        }
    }
    if !bought.is_empty() {
        inner
            .outbound
            .push_back(encode::vendor_buy(vendor, &bought));
    }
}

/// Sells the active list's items from a vendor's sell list, up to each
/// rule's amount. Returns true when it sold anything.
pub(super) fn sell(
    inner: &mut Inner,
    vendor: Serial,
    entries: &[uoterm_protocol::VendorSellEntry],
) -> bool {
    if !shard_allows(inner, AssistFeature::SellAgent) {
        return false;
    }
    let s = inner.agents.config.sell.clone();
    let bag = s.bag;
    let mut left: Vec<Option<u32>> = s.items.rules().iter().map(|r| r.amount).collect();
    let mut sold: Vec<(Serial, u16)> = Vec::new();
    {
        let w = inner.world.read();
        for entry in entries {
            if bag.is_some_and(|b| !w.is_inside(entry.serial, b)) {
                continue;
            }
            let Some(i) = s
                .items
                .rules()
                .iter()
                .position(|r| r.matches(entry.graphic, entry.hue))
            else {
                continue;
            };
            let take = match &mut left[i] {
                Some(0) => continue,
                Some(n) => {
                    let t = (*n).min(u32::from(entry.amount));
                    *n -= t;
                    t as u16
                }
                None => entry.amount,
            };
            sold.push((entry.serial, take));
        }
    }
    if sold.is_empty() {
        return false;
    }
    inner.outbound.push_back(encode::vendor_sell(vendor, &sold));
    true
}

// ---------- target filters ----------

/// Picks a mobile with a named filter.
pub(super) fn pick_by_filter(
    inner: &mut Inner,
    name: &str,
) -> std::result::Result<Option<Serial>, String> {
    let filter = inner
        .agents
        .config
        .targets
        .get(name)
        .cloned()
        .ok_or_else(|| format!("no target filter named '{name}'"))?;
    let wanted: Vec<u8> = filter
        .notorieties
        .iter()
        .flat_map(|n| notorieties(n).iter().copied())
        .collect();
    let world = inner.world.read().clone();
    let here = world.self_state.location;
    let mut passing: Vec<&uoterm_world::Mobile> = world
        .mobiles
        .values()
        .filter(|m| m.serial != world.self_state.serial)
        .filter(|m| wanted.is_empty() || wanted.contains(&m.notoriety))
        .filter(|m| filter.bodies.is_empty() || filter.bodies.contains(&m.body))
        .filter(|m| filter.colors.is_empty() || filter.colors.contains(&m.hue))
        .filter(|m| {
            filter
                .name
                .as_ref()
                .map_or(true, |n| m.name.to_lowercase().contains(&n.to_lowercase()))
        })
        .filter(|m| {
            let d = here.chebyshev(m.location);
            filter.range_min.map_or(true, |r| d >= r) && filter.range_max.map_or(true, |r| d <= r)
        })
        .filter(|m| {
            filter
                .poisoned
                .map_or(true, |p| world.is_poisoned(m.serial) == p)
        })
        .filter(|m| filter.human.map_or(true, |h| is_humanoid(m.body) == h))
        .filter(|m| {
            filter
                .ghost
                .map_or(true, |g| uoterm_world::is_ghost_body(m.body) == g)
        })
        .filter(|m| {
            filter.war.map_or(true, |w| {
                (m.flags & uoterm_protocol::types::FLAG_WAR != 0) == w
            })
        })
        .filter(|m| {
            filter
                .friend
                .map_or(true, |f| inner.agents.is_friend(&world, m.serial) == f)
        })
        .filter(|m| {
            filter
                .paralyzed
                .map_or(true, |p| (m.flags & FLAG_FROZEN != 0) == p)
        })
        .collect();
    passing.sort_by_key(|m| (here.chebyshev(m.location), m.serial.0));
    let health = |m: &uoterm_world::Mobile| {
        m.hits_max.filter(|&max| max > 0).map_or(PERCENT, |max| {
            u32::from(m.hits.unwrap_or(0)) * PERCENT / u32::from(max)
        })
    };
    let feature = match filter.selector {
        Selector::Nearest | Selector::Farthest => Some(AssistFeature::ClosestTargets),
        Selector::Random => Some(AssistFeature::RandomTargets),
        _ => None,
    };
    if let Some(f) = feature.filter(|&f| !shard_allows(inner, f)) {
        return Err(forbidden(f));
    }
    let last = inner.agents.last_pick.get(name).copied();
    let place = last.and_then(|l| passing.iter().position(|m| m.serial == l));
    let chosen = match filter.selector {
        Selector::Nearest => passing.first(),
        Selector::Farthest => passing.last(),
        Selector::Weakest => passing.iter().min_by_key(|m| health(m)),
        Selector::Strongest => passing.iter().max_by_key(|m| health(m)),
        Selector::Random => {
            if passing.is_empty() {
                None
            } else {
                passing.get(rand::thread_rng().gen_range(0..passing.len()))
            }
        }
        Selector::Next => match place {
            Some(i) => passing.get((i + 1) % passing.len()),
            None => passing.first(),
        },
        Selector::Previous => match place {
            Some(0) | None => passing.last(),
            Some(i) => passing.get(i - 1),
        },
    }
    .map(|m| m.serial);
    if let Some(serial) = chosen {
        inner.agents.last_pick.insert(name.to_string(), serial);
    }
    Ok(chosen)
}
