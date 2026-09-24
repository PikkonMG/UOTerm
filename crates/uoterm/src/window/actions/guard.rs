//! What the hand checks before an act goes to the session: the criminal
//! action question of the official client, and a click the window waits
//! for itself (grab an item, pick the grab bag).
//!
//! The hand keeps one guard. Every part of the window acts through the
//! hand, so a click on the map, a ring menu, a key and a macro are all
//! checked the same way.

use super::LocalAim;
use crate::view::WatchFrame;
use crate::window::control::{Act, DropTo, WHOLE_PILE};
use crate::window::kept;
use crate::window::settings::CombatOptions;
use std::collections::{BTreeMap, HashMap};
use uoterm_protocol::types::{NOTO_CRIMINAL, NOTO_FRIEND, NOTO_GREY, NOTO_INNOCENT, NOTO_MURDERER};

/// The flag of a target cursor whose spell or skill hurts.
pub const TARGET_FLAG_HARMFUL: u8 = 1;
/// The flag of a target cursor whose spell or skill helps.
pub const TARGET_FLAG_BENEFICIAL: u8 = 2;
const GRAB_BAGS_FILE: &str = "watch-grab-bags.toml";

pub const QUESTION_CRIMINAL: &str = "This may flag you criminal!";
pub const AIM_GRAB: &str = "Target an item to grab it.";
pub const AIM_GRAB_BAG: &str = "Target the container to grab items into.";
const AIM_PICK_THING: &str = "Target a thing to take its color.";
const AIM_IGNORE_PLAYER: &str = "Target a player to ignore.";
const NOTE_THING_PICKED: &str = "The color is taken.";
const NOTE_PLAYER_PICKED: &str = "Picked.";
const NOTE_GRAB_BAG_SET: &str = "The grab bag is set.";
const NOTE_NO_GRAB_BAG: &str = "There is no grab bag and no backpack.";

/// The words that ask the player for the click the window waits for.
pub fn aim_words(aim: LocalAim) -> &'static str {
    match aim {
        LocalAim::Grab => AIM_GRAB,
        LocalAim::SetGrabBag => AIM_GRAB_BAG,
        LocalAim::PickThing => AIM_PICK_THING,
        LocalAim::IgnorePlayer => AIM_IGNORE_PLAYER,
    }
}

/// The grab bag of each character, by his name.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct KeptGrabBags {
    #[serde(default)]
    characters: BTreeMap<String, u32>,
}

/// What the guard knows of the world, from the newest picture.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Seen {
    player: u32,
    name: String,
    player_notoriety: u8,
    notorieties: HashMap<u32, u8>,
    target_flags: u8,
    backpack: Option<u32>,
    query_attack: bool,
    query_beneficial: bool,
}

/// What became of an act the hand was given.
#[derive(Clone, Debug, PartialEq)]
pub enum Checked {
    /// Send these acts.
    Send(Vec<Act>),
    /// The act waits for the player to answer the question.
    Asked,
    /// The act was the click the window waited for. The words tell what
    /// came of it.
    Aimed(&'static str),
}

#[derive(Default)]
pub struct Guard {
    seen: Seen,
    aim: Option<LocalAim>,
    /// The thing the last pick took, and the aim it answered, until the
    /// window reads it.
    picked: Option<(LocalAim, u32)>,
    question: Option<Act>,
    grab_bags: Option<KeptGrabBags>,
}

/// An innocent (or an ally) who attacks an innocent becomes a criminal.
fn attack_flags(player: u8, target: u8) -> bool {
    matches!(player, NOTO_INNOCENT | NOTO_FRIEND) && target == NOTO_INNOCENT
}

/// An innocent who helps a criminal, a murderer or a gray one becomes gray.
fn help_flags(player: u8, target: u8) -> bool {
    matches!(player, NOTO_INNOCENT | NOTO_FRIEND)
        && matches!(target, NOTO_CRIMINAL | NOTO_MURDERER | NOTO_GREY)
}

impl Guard {
    /// Takes what the guard needs from the newest picture and options.
    pub fn watch_over(&mut self, frame: &WatchFrame, combat: &CombatOptions) {
        self.seen = Seen {
            player: frame.serial,
            name: frame.name.clone(),
            player_notoriety: frame.notoriety,
            notorieties: frame
                .mobiles
                .iter()
                .map(|mobile| (mobile.serial, mobile.notoriety))
                .collect(),
            target_flags: frame.target_flags,
            backpack: frame.backpack(),
            query_attack: combat.query_before_attack,
            query_beneficial: combat.query_beneficial_acts,
        };
    }

    /// Waits for the next click on a thing, in place of its usual act.
    pub fn aim(&mut self, aim: LocalAim) {
        self.aim = Some(aim);
    }

    pub fn aiming(&self) -> Option<LocalAim> {
        self.aim
    }

    pub fn cancel_aim(&mut self) {
        self.aim = None;
    }

    /// The thing a click took for this aim, once.
    pub fn take_picked(&mut self, aim: LocalAim) -> Option<u32> {
        let (picked_for, serial) = self.picked?;
        (picked_for == aim).then(|| {
            self.picked = None;
            serial
        })
    }

    /// The question that waits for an answer, in words.
    pub fn question(&self) -> Option<&'static str> {
        self.question.as_ref().map(|_| QUESTION_CRIMINAL)
    }

    /// The answer to the question: yes gives the act that waited.
    pub fn answer(&mut self, yes: bool) -> Option<Act> {
        self.question.take().filter(|_| yes)
    }

    fn notoriety_of(&self, serial: u32) -> Option<u8> {
        (serial != self.seen.player)
            .then(|| self.seen.notorieties.get(&serial).copied())
            .flatten()
    }

    /// True when the act may flag the character and the options ask first.
    fn needs_question(&self, act: &Act) -> bool {
        let player = self.seen.player_notoriety;
        match act {
            Act::Attack(serial) => {
                self.seen.query_attack
                    && self
                        .notoriety_of(*serial)
                        .is_some_and(|target| attack_flags(player, target))
            }
            Act::Target(serial) => {
                self.notoriety_of(*serial)
                    .is_some_and(|target| match self.seen.target_flags {
                        TARGET_FLAG_HARMFUL => {
                            self.seen.query_attack && attack_flags(player, target)
                        }
                        TARGET_FLAG_BENEFICIAL => {
                            self.seen.query_beneficial && help_flags(player, target)
                        }
                        _ => false,
                    })
            }
            _ => false,
        }
    }

    /// The bag grabbed items go into: the one the player set, or the
    /// backpack.
    pub fn grab_bag(&mut self) -> Option<u32> {
        let name = self.seen.name.clone();
        let bags = self
            .grab_bags
            .get_or_insert_with(|| kept::load(GRAB_BAGS_FILE));
        bags.characters.get(&name).copied().or(self.seen.backpack)
    }

    fn set_grab_bag(&mut self, bag: u32) {
        let name = self.seen.name.clone();
        let bags = self
            .grab_bags
            .get_or_insert_with(|| kept::load(GRAB_BAGS_FILE));
        bags.characters.insert(name, bag);
        kept::save(GRAB_BAGS_FILE, bags);
    }

    /// The click the window waited for, when the act is a click on a thing.
    fn take_aim(&mut self, act: &Act) -> Option<Checked> {
        let serial = match act {
            Act::Look(serial) | Act::Use(serial) | Act::Target(serial) => *serial,
            _ => return None,
        };
        let aim = self.aim.take()?;
        Some(match aim {
            LocalAim::Grab => match self.grab_bag() {
                Some(bag) => Checked::Send(vec![Act::Move {
                    item: serial,
                    amount: WHOLE_PILE,
                    to: DropTo::Into(bag),
                }]),
                None => Checked::Aimed(NOTE_NO_GRAB_BAG),
            },
            LocalAim::SetGrabBag => {
                self.set_grab_bag(serial);
                Checked::Aimed(NOTE_GRAB_BAG_SET)
            }
            LocalAim::PickThing => {
                self.picked = Some((aim, serial));
                Checked::Aimed(NOTE_THING_PICKED)
            }
            LocalAim::IgnorePlayer => {
                self.picked = Some((aim, serial));
                Checked::Aimed(NOTE_PLAYER_PICKED)
            }
        })
    }

    /// Checks one act before it goes to the session.
    pub fn check(&mut self, act: Act) -> Checked {
        if let Some(aimed) = self.take_aim(&act) {
            return aimed;
        }
        if self.needs_question(&act) {
            self.question = Some(act);
            return Checked::Asked;
        }
        Checked::Send(vec![act])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchMobile;

    const ME: u32 = 1;
    const ANN: u32 = 2;
    const THIEF: u32 = 3;
    const BAG: u32 = 0x4000_0010;
    const COIN: u32 = 0x4000_0020;

    fn guard(target_flags: u8, beneficial: bool) -> Guard {
        let mobile = |serial, notoriety| WatchMobile {
            serial,
            notoriety,
            ..WatchMobile::default()
        };
        let frame = WatchFrame {
            serial: ME,
            notoriety: NOTO_INNOCENT,
            target_flags,
            mobiles: vec![mobile(ANN, NOTO_INNOCENT), mobile(THIEF, NOTO_CRIMINAL)],
            ..WatchFrame::default()
        };
        let combat = CombatOptions {
            query_before_attack: true,
            query_beneficial_acts: beneficial,
            ..CombatOptions::default()
        };
        let mut guard = Guard::default();
        guard.watch_over(&frame, &combat);
        guard
    }

    #[test]
    fn an_attack_on_an_innocent_waits_for_a_yes() {
        let mut guard = guard(0, false);
        assert_eq!(guard.check(Act::Attack(ANN)), Checked::Asked);
        assert_eq!(guard.question(), Some(QUESTION_CRIMINAL));
        assert_eq!(guard.answer(true), Some(Act::Attack(ANN)));
        assert_eq!(guard.question(), None);
        assert_eq!(guard.check(Act::Attack(ANN)), Checked::Asked);
        assert_eq!(guard.answer(false), None);
        assert_eq!(
            guard.check(Act::Attack(THIEF)),
            Checked::Send(vec![Act::Attack(THIEF)])
        );
    }

    #[test]
    fn a_harmful_cursor_asks_on_an_innocent_and_a_helpful_one_on_a_criminal() {
        let mut harmful = guard(TARGET_FLAG_HARMFUL, false);
        assert_eq!(harmful.check(Act::Target(ANN)), Checked::Asked);
        let mut helpful = guard(TARGET_FLAG_BENEFICIAL, true);
        assert_eq!(helpful.check(Act::Target(THIEF)), Checked::Asked);
        assert_eq!(
            helpful.check(Act::Target(ANN)),
            Checked::Send(vec![Act::Target(ANN)])
        );
        let mut off = guard(TARGET_FLAG_BENEFICIAL, false);
        assert!(matches!(off.check(Act::Target(THIEF)), Checked::Send(_)));
    }

    #[test]
    fn the_picture_tells_the_flags_of_the_cursor_and_the_last_target() {
        let frame = WatchFrame::from_observe(&serde_json::json!({
            "target_cursor": { "kind": 0, "id": 5, "flags": TARGET_FLAG_BENEFICIAL },
            "last_target": ANN,
        }));
        assert!(frame.target_cursor);
        assert_eq!(frame.target_flags, TARGET_FLAG_BENEFICIAL);
        assert_eq!(frame.last_target, Some(ANN));
        let quiet = WatchFrame::from_observe(&serde_json::json!({ "target_cursor": null }));
        assert_eq!((quiet.target_flags, quiet.last_target), (0, None));
    }

    #[test]
    fn with_the_question_off_an_attack_goes_at_once() {
        let mut guard = guard(0, false);
        guard.seen.query_attack = false;
        assert!(matches!(guard.check(Act::Attack(ANN)), Checked::Send(_)));
    }

    #[test]
    fn a_pick_keeps_the_clicked_thing_once_and_sends_nothing() {
        let mut guard = guard(0, false);
        guard.aim(LocalAim::PickThing);
        assert_eq!(
            guard.check(Act::Look(COIN)),
            Checked::Aimed(NOTE_THING_PICKED)
        );
        assert_eq!(guard.take_picked(LocalAim::IgnorePlayer), None);
        assert_eq!(guard.take_picked(LocalAim::PickThing), Some(COIN));
        assert_eq!(guard.take_picked(LocalAim::PickThing), None);
        assert_eq!(aim_words(LocalAim::PickThing), AIM_PICK_THING);
        guard.aim(LocalAim::IgnorePlayer);
        assert_eq!(
            guard.check(Act::Look(ANN)),
            Checked::Aimed(NOTE_PLAYER_PICKED)
        );
        assert_eq!(guard.take_picked(LocalAim::IgnorePlayer), Some(ANN));
        assert_eq!(aim_words(LocalAim::IgnorePlayer), AIM_IGNORE_PLAYER);
    }

    #[test]
    fn a_grab_moves_the_clicked_item_into_the_grab_bag() {
        let mut guard = guard(0, false);
        guard.grab_bags = Some(KeptGrabBags::default());
        guard.seen.backpack = Some(BAG);
        guard.aim(LocalAim::Grab);
        assert_eq!(guard.aiming(), Some(LocalAim::Grab));
        assert_eq!(
            guard.check(Act::Look(COIN)),
            Checked::Send(vec![Act::Move {
                item: COIN,
                amount: WHOLE_PILE,
                to: DropTo::Into(BAG),
            }])
        );
        assert_eq!(guard.aiming(), None);
        assert!(
            matches!(guard.check(Act::Look(COIN)), Checked::Send(acts) if acts == vec![Act::Look(COIN)])
        );
    }
}
