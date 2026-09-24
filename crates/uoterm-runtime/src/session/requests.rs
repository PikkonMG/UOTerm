//! The small requests a Classic Client sends at one click: open a spellbook,
//! turn a tip, click the quest arrow, steer a boat, ask where the party or
//! the guild stands, show what stands inside public houses, and answer the
//! race change of the shard. Each goes
//! out only when a tool asks for it, so the session says nothing a player
//! did not.

use super::*;

const ARG_KIND: &str = "kind";
const ARG_NEXT: &str = "next";
const ARG_RIGHT: &str = "right";
const ARG_DIRECTION: &str = "direction";
const ARG_SPEED: &str = "speed";
const ARG_WHO: &str = "who";
const ARG_SHOW: &str = "show";
const ARG_CANCEL: &str = "cancel";
const ARG_SKIN_HUE: &str = "skin_hue";
const ARG_HAIR: &str = "hair";
const ARG_HAIR_HUE: &str = "hair_hue";
const ARG_BEARD: &str = "beard";
const ARG_BEARD_HUE: &str = "beard_hue";
const WHO_PARTY: &str = "party";
const WHO_GUILD: &str = "guild";
/// The boat speeds a tool names, and the speed each one asks for.
const BOAT_SPEEDS: [(&str, u8); 3] = [
    ("stop", BOAT_SPEED_STOP),
    ("slow", BOAT_SPEED_SLOW),
    ("fast", BOAT_SPEED_FAST),
];
const BAD_BOOK_KIND: &str =
    "kind must be magery, necromancy, chivalry, bushido, ninjitsu, spellweaving or mysticism";
const NO_TIP_SHOWN: &str = "no tip of the day was shown";
const NO_QUEST_ARROW: &str = "no quest arrow is shown";
const BAD_DIRECTION: &str = "direction must be n, ne, e, se, s, sw, w or nw";
const BAD_SPEED: &str = "speed must be stop, slow or fast";
const BAD_WHO: &str = "who must be party or guild";
const NEEDS_SHOW: &str = "needs show: true or false";
const NO_RACE_CHANGE: &str = "the shard asks for no race change";
const BAD_LOOK: &str =
    "skin_hue, hair, hair_hue, beard and beard_hue must be numbers from 0 to 65535";

/// `open_spellbook`: opens the book of one school of spells.
pub(super) fn open_spellbook(inner: &mut Inner, args: &Value) -> ToolResult {
    let kind = args.get(ARG_KIND).and_then(Value::as_str).unwrap_or("");
    let Some(book) = uoterm_world::spell_school_named(kind).and_then(|school| school.book_kind)
    else {
        return ToolResult::err(BAD_BOOK_KIND);
    };
    inner.outbound.push_back(encode::open_spellbook(book));
    ToolResult::action(TOOL_OPEN_SPELLBOOK)
}

/// `tip`: asks for the tip after, or before, the one shown.
pub(super) fn tip(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(shown) = inner.world.read().tip else {
        return ToolResult::err(NO_TIP_SHOWN);
    };
    let next = args.get(ARG_NEXT).and_then(Value::as_bool).unwrap_or(true);
    // The request carries the number in a word, as the reference client
    // writes it.
    inner
        .outbound
        .push_back(encode::tip_request(shown as u16, next));
    ToolResult::action(TOOL_TIP)
}

/// `quest_arrow`: clicks the arrow the shard shows.
pub(super) fn quest_arrow(inner: &mut Inner, args: &Value) -> ToolResult {
    if inner.world.read().quest_arrow.is_none() {
        return ToolResult::err(NO_QUEST_ARROW);
    }
    let right = args
        .get(ARG_RIGHT)
        .and_then(Value::as_bool)
        .unwrap_or(false);
    inner.outbound.push_back(encode::quest_arrow_click(right));
    ToolResult::action(TOOL_QUEST_ARROW)
}

/// `boat_move`: steers the boat the character pilots. The shard reads the
/// pilot, so the request names the character.
pub(super) fn boat_move(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(direction) = args
        .get(ARG_DIRECTION)
        .and_then(Value::as_str)
        .and_then(Direction::from_name)
    else {
        return ToolResult::err(BAD_DIRECTION);
    };
    let speed = match args.get(ARG_SPEED).and_then(Value::as_str) {
        None => BOAT_SPEED_FAST,
        Some(name) => match BOAT_SPEEDS
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name.trim()))
        {
            Some((_, speed)) => *speed,
            None => return ToolResult::err(BAD_SPEED),
        },
    };
    let pilot = inner.world.read().self_state.serial;
    inner
        .outbound
        .push_back(encode::boat_move(pilot, direction, speed));
    ToolResult::action(TOOL_BOAT_MOVE)
}

/// `track_members`: asks where the party or the guild members out of sight
/// stand.
pub(super) fn track_members(inner: &mut Inner, args: &Value) -> ToolResult {
    let packet = match args.get(ARG_WHO).and_then(Value::as_str).map(str::trim) {
        Some(who) if who.eq_ignore_ascii_case(WHO_PARTY) => encode::query_party_positions(),
        Some(who) if who.eq_ignore_ascii_case(WHO_GUILD) => encode::query_guild_positions(),
        _ => return ToolResult::err(BAD_WHO),
    };
    inner.outbound.push_back(packet);
    ToolResult::action(TOOL_TRACK_MEMBERS)
}

/// `house_content`: whether the shard shows what stands inside public
/// houses.
pub(super) fn house_content(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(show) = args.get(ARG_SHOW).and_then(Value::as_bool) else {
        return ToolResult::err(NEEDS_SHOW);
    };
    inner.outbound.push_back(encode::public_house_content(show));
    ToolResult::action(TOOL_HOUSE_CONTENT)
}

/// One number of the looks, or `first` when the call leaves it out. None
/// when it is not a number that fits.
fn look_arg(args: &Value, key: &str, first: u16) -> Option<u16> {
    match args.get(key) {
        None | Some(Value::Null) => Some(first),
        Some(value) => value.as_u64().and_then(|n| u16::try_from(n).ok()),
    }
}

/// `race_change`: answers the race change of the shard with new looks, or
/// says no. A character that grows no beard sends none, as the reference
/// client does.
pub(super) fn race_change(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(change) = inner.world.read().race_change else {
        return ToolResult::err(NO_RACE_CHANGE);
    };
    let cancel = args
        .get(ARG_CANCEL)
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let looks = if cancel {
        None
    } else {
        let first = change.first_looks();
        let picked = (
            look_arg(args, ARG_SKIN_HUE, first.skin_hue),
            look_arg(args, ARG_HAIR, first.hair),
            look_arg(args, ARG_HAIR_HUE, first.hair_hue),
            look_arg(args, ARG_BEARD, first.beard),
            look_arg(args, ARG_BEARD_HUE, first.beard_hue),
        );
        let (Some(skin_hue), Some(hair), Some(hair_hue), Some(beard), Some(beard_hue)) = picked
        else {
            return ToolResult::err(BAD_LOOK);
        };
        let bearded = change.has_beard();
        Some(encode::NewLooks {
            skin_hue,
            hair,
            hair_hue,
            beard: if bearded { beard } else { first.beard },
            beard_hue: if bearded { beard_hue } else { first.beard_hue },
        })
    };
    inner.world.write().race_change = None;
    inner.outbound.push_back(encode::race_change_answer(looks));
    ToolResult::action(TOOL_RACE_CHANGE)
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;
    use uoterm_world::{Race, RaceChange};

    const ME: Serial = Serial(0x0000_0042);

    fn sent_by(tool: fn(&mut Inner, &Value) -> ToolResult, args: Value) -> Option<Vec<u8>> {
        let mut inner = test_session();
        inner.world.write().self_state.serial = ME;
        inner.world.write().tip = Some(3);
        inner.world.write().quest_arrow = Some((10, 20));
        let result = tool(&mut inner, &args);
        assert!(result.ok || inner.outbound.is_empty(), "{result:?}");
        inner.outbound.pop_front()
    }

    #[test]
    fn each_request_goes_out_as_the_tool_names_it() {
        assert_eq!(
            sent_by(open_spellbook, json!({ "kind": "Necromancy" })),
            Some(encode::open_spellbook(SPELLBOOK_NECROMANCY))
        );
        assert_eq!(sent_by(open_spellbook, json!({ "kind": "mastery" })), None);
        assert_eq!(
            sent_by(tip, json!({ "next": false })),
            Some(encode::tip_request(3, false))
        );
        assert_eq!(
            sent_by(quest_arrow, json!({ "right": true })),
            Some(encode::quest_arrow_click(true))
        );
        assert_eq!(
            sent_by(boat_move, json!({ "direction": "ne", "speed": "slow" })),
            Some(encode::boat_move(ME, Direction::Northeast, BOAT_SPEED_SLOW))
        );
        assert_eq!(
            sent_by(boat_move, json!({ "direction": "w" })),
            Some(encode::boat_move(ME, Direction::West, BOAT_SPEED_FAST))
        );
        assert_eq!(
            sent_by(boat_move, json!({ "direction": "w", "speed": "warp" })),
            None
        );
        assert_eq!(
            sent_by(track_members, json!({ "who": "guild" })),
            Some(encode::query_guild_positions())
        );
        assert_eq!(
            sent_by(track_members, json!({ "who": "party" })),
            Some(encode::query_party_positions())
        );
        assert_eq!(
            sent_by(house_content, json!({ "show": false })),
            Some(encode::public_house_content(false))
        );
        assert_eq!(sent_by(house_content, json!({})), None);
    }

    /// The answer carries the picked looks, the first choices for the ones
    /// left out, and no beard for an elf; a refusal carries nothing. Either
    /// ends the race change.
    #[test]
    fn a_race_change_is_answered_once_with_looks_or_a_refusal() {
        const HAIR: u16 = 0x2FC0;
        const HAIR_HUE: u16 = 0x0035;
        let elf = RaceChange {
            race: Race::Elf,
            female: false,
        };
        let mut inner = test_session();
        inner.world.write().race_change = Some(elf);
        let args = json!({ "hair": HAIR, "hair_hue": HAIR_HUE, "beard": 0x2040 });
        assert!(race_change(&mut inner, &args).ok);
        let first = elf.first_looks();
        assert_eq!(
            inner.outbound.pop_front(),
            Some(encode::race_change_answer(Some(encode::NewLooks {
                skin_hue: first.skin_hue,
                hair: HAIR,
                hair_hue: HAIR_HUE,
                beard: 0,
                beard_hue: 0,
            })))
        );
        assert_eq!(inner.world.read().race_change, None);
        assert!(!race_change(&mut inner, &json!({})).ok);

        inner.world.write().race_change = Some(elf);
        assert!(!race_change(&mut inner, &json!({ "hair": "long" })).ok);
        assert!(race_change(&mut inner, &json!({ "cancel": true })).ok);
        assert_eq!(
            inner.outbound.pop_front(),
            Some(encode::race_change_answer(None))
        );
        assert!(inner.outbound.is_empty());
    }

    #[test]
    fn a_tip_and_an_arrow_are_clicked_only_while_shown() {
        let mut inner = test_session();
        assert!(!tip(&mut inner, &json!({})).ok);
        assert!(!quest_arrow(&mut inner, &json!({})).ok);
        assert!(inner.outbound.is_empty());
    }
}
