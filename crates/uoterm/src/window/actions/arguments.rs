//! The arguments an action takes. A macro step keeps its argument as the
//! words the Options screen shows, so a kept profile reads plainly: a
//! spell name, a window name, a number of milliseconds.

use crate::window::settings::{choices, Choice};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uoterm_assist::items::POTIONS;
use uoterm_assist::spells::{School, SpellBook};

/// How long a step waits for a target cursor when it names no time. The
/// official client waits this long too.
pub const DEFAULT_TARGET_WAIT_MS: u64 = 5000;

/// The kind of argument an action takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    None,
    /// Words to say.
    Text,
    /// A whole number, such as a slot or a range.
    Number,
    Milliseconds,
    Direction,
    Gump,
    Skill,
    Spell,
    Virtue,
    Hand,
    Select,
    Potion,
    Usable,
    Zoom,
    Look,
    /// The name of a saved script.
    Script,
    /// One line of the script language.
    Command,
    /// The name of a hotkey of the session.
    Hotkey,
}

choices! {
    /// The eight ways a character walks.
    Direction {
        North => "North",
        NorthEast => "North-east",
        East => "East",
        SouthEast => "South-east",
        South => "South",
        SouthWest => "South-west",
        West => "West",
        NorthWest => "North-west",
    }
}

/// The ways as the session names them, in the order of `Direction`.
const WAY_WORDS: [&str; 8] = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];

impl Direction {
    /// The way as the walk tool names it.
    pub fn way(self) -> &'static str {
        WAY_WORDS[self.index()]
    }
}

choices! {
    /// Every window an action opens or closes. A style that has no such
    /// window says so.
    GumpKind {
        Options => "Options",
        Paperdoll => "Paperdoll",
        Status => "Status",
        Journal => "Journal",
        Skills => "Skills",
        MagerySpellbook => "Magery spellbook",
        NecromancySpellbook => "Necromancy spellbook",
        ChivalrySpellbook => "Chivalry spellbook",
        BushidoSpellbook => "Bushido spellbook",
        NinjitsuSpellbook => "Ninjitsu spellbook",
        SpellweavingSpellbook => "Spellweaving spellbook",
        MysticismSpellbook => "Mysticism spellbook",
        MasterySpellbook => "Bard masteries spellbook",
        Chat => "Chat",
        Backpack => "Backpack",
        Minimap => "Minimap",
        WorldMap => "World map",
        Party => "Party",
        Guild => "Guild",
        QuestLog => "Quest log",
        CombatBook => "Combat book",
        RacialAbilities => "Racial abilities",
        Macros => "Macros",
        Counters => "Counter bar",
        InfoBar => "Info bar",
        Buffs => "Buff window",
    }
}

impl GumpKind {
    /// The school of a spellbook window.
    pub fn school(self) -> Option<School> {
        Some(match self {
            GumpKind::MagerySpellbook => School::Magery,
            GumpKind::NecromancySpellbook => School::Necromancy,
            GumpKind::ChivalrySpellbook => School::Chivalry,
            GumpKind::BushidoSpellbook => School::Bushido,
            GumpKind::NinjitsuSpellbook => School::Ninjitsu,
            GumpKind::SpellweavingSpellbook => School::Spellweaving,
            GumpKind::MysticismSpellbook => School::Mysticism,
            GumpKind::MasterySpellbook => School::Mastery,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        Self::LABELS[self.index()]
    }
}

choices! {
    /// The virtues a player invokes with a key.
    Virtue {
        Honor => "Honor",
        Sacrifice => "Sacrifice",
        Valor => "Valor",
    }
}

choices! {
    /// A hand of the character.
    Hand {
        Left => "Left",
        Right => "Right",
    }
}

choices! {
    /// What a selection picks from.
    SelectKind {
        Hostile => "Hostile",
        Party => "Party",
        Follower => "Follower",
        Object => "Object",
        Mobile => "Mobile",
    }
}

choices! {
    /// The things the official client finds and uses with one key.
    UsableObject {
        BestHealPotion => "Best heal potion",
        BestCurePotion => "Best cure potion",
        BestRefreshPotion => "Best refresh potion",
        BestStrengthPotion => "Best strength potion",
        BestAgilityPotion => "Best agility potion",
        BestExplosionPotion => "Best explosion potion",
        BestConflagrationPotion => "Best conflagration potion",
        EnchantedApple => "Enchanted apple",
        PetalsOfTrinsic => "Petals of Trinsic",
        OrangePetals => "Orange petals",
        TrappedBox => "Trapped box",
        SmokeBomb => "Smoke bomb",
        HealingStone => "Healing stone",
        SpellStone => "Spell stone",
    }
}

choices! {
    /// A step of the zoom.
    ZoomStep {
        Default => "Default zoom",
        In => "Zoom in",
        Out => "Zoom out",
    }
}

choices! {
    /// Which way the camera looks while the key is held.
    Look {
        Forwards => "Toward the mouse",
        Backwards => "Away from the mouse",
    }
}

/// The skills the official client uses from a key, in its order.
const USABLE_SKILLS: [&str; 24] = [
    "Anatomy",
    "Animal Lore",
    "Animal Taming",
    "Arms Lore",
    "Begging",
    "Cartography",
    "Detecting Hidden",
    "Discordance",
    "Evaluating Intelligence",
    "Forensic Evaluation",
    "Hiding",
    "Imbuing",
    "Inscription",
    "Item Identification",
    "Meditation",
    "Peacemaking",
    "Poisoning",
    "Provocation",
    "Remove Trap",
    "Spirit Speak",
    "Stealing",
    "Stealth",
    "Taste Identification",
    "Tracking",
];

const HINT_NONE: &str = "";
const HINT_TEXT: &str = "the words";
const HINT_NUMBER: &str = "a number";
const HINT_MILLISECONDS: &str = "milliseconds";
const HINT_SCRIPT: &str = "the name of a saved script";
const HINT_COMMAND: &str = "a script line, for example: useskill 'hiding'";
const HINT_HOTKEY: &str = "a hotkey of the session, for example: Cast Heal";
const HINT_CHOICE: &str = "pick one";

/// A choice in the words the Options screen shows: "night sight" becomes
/// "Night sight".
fn sentence_case(words: &str) -> String {
    let mut letters = words.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

fn labels_of<T: Choice>() -> Vec<String> {
    T::LABELS.iter().map(|label| label.to_string()).collect()
}

/// The names of the standard spells, in the order of the spell book.
fn spell_names() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        SpellBook::standard()
            .iter()
            .map(|spell| spell.name.clone())
            .collect()
    })
}

impl ArgumentKind {
    /// The words the player picks from. Empty for a kind he types.
    pub fn choices(self) -> Vec<String> {
        match self {
            ArgumentKind::Direction => labels_of::<Direction>(),
            ArgumentKind::Gump => labels_of::<GumpKind>(),
            ArgumentKind::Virtue => labels_of::<Virtue>(),
            ArgumentKind::Hand => labels_of::<Hand>(),
            ArgumentKind::Select => labels_of::<SelectKind>(),
            ArgumentKind::Usable => labels_of::<UsableObject>(),
            ArgumentKind::Zoom => labels_of::<ZoomStep>(),
            ArgumentKind::Look => labels_of::<Look>(),
            ArgumentKind::Skill => USABLE_SKILLS.iter().map(|s| s.to_string()).collect(),
            ArgumentKind::Spell => spell_names().to_vec(),
            ArgumentKind::Potion => POTIONS.iter().map(|p| sentence_case(p.name)).collect(),
            ArgumentKind::Hotkey => uoterm_runtime::session::fixed_hotkey_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
            ArgumentKind::None
            | ArgumentKind::Text
            | ArgumentKind::Number
            | ArgumentKind::Milliseconds
            | ArgumentKind::Script
            | ArgumentKind::Command => Vec::new(),
        }
    }

    /// The player types the argument. A hotkey also offers its choices,
    /// because a session has more hotkeys than every character shares.
    pub fn is_typed(self) -> bool {
        matches!(
            self,
            ArgumentKind::Text
                | ArgumentKind::Number
                | ArgumentKind::Milliseconds
                | ArgumentKind::Script
                | ArgumentKind::Command
                | ArgumentKind::Hotkey
        )
    }

    /// The argument a new step starts with: the first choice, or nothing
    /// for a kind the player types.
    pub fn default_argument(self) -> String {
        if self.is_typed() {
            return String::new();
        }
        self.choices().into_iter().next().unwrap_or_default()
    }

    /// Words in an empty argument field.
    pub fn hint(self) -> &'static str {
        match self {
            ArgumentKind::None => HINT_NONE,
            ArgumentKind::Text => HINT_TEXT,
            ArgumentKind::Number => HINT_NUMBER,
            ArgumentKind::Milliseconds => HINT_MILLISECONDS,
            ArgumentKind::Script => HINT_SCRIPT,
            ArgumentKind::Command => HINT_COMMAND,
            ArgumentKind::Hotkey => HINT_HOTKEY,
            _ => HINT_CHOICE,
        }
    }
}

/// The choice of a list whose words the argument holds, in any case.
pub fn chosen<T: Choice>(argument: &str) -> Option<T> {
    let wanted = argument.trim();
    T::LABELS
        .iter()
        .position(|label| label.eq_ignore_ascii_case(wanted))
        .map(T::from_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_is_found_by_its_words_in_any_case() {
        assert_eq!(
            chosen::<Direction>("north-west"),
            Some(Direction::NorthWest)
        );
        assert_eq!(chosen::<GumpKind>(" World map "), Some(GumpKind::WorldMap));
        assert_eq!(chosen::<Virtue>("courage"), None);
        assert_eq!(Direction::SouthEast.way(), "se");
    }

    #[test]
    fn every_kind_with_choices_starts_with_its_first_choice() {
        assert_eq!(ArgumentKind::Gump.default_argument(), "Options");
        assert_eq!(ArgumentKind::Skill.default_argument(), "Anatomy");
        assert!(ArgumentKind::Spell
            .choices()
            .contains(&"Greater Heal".to_string()));
        assert!(ArgumentKind::Potion
            .choices()
            .contains(&"Night sight".to_string()));
        assert!(ArgumentKind::Hotkey
            .choices()
            .contains(&"Resync".to_string()));
        assert_eq!(ArgumentKind::Hotkey.default_argument(), "");
        assert!(ArgumentKind::Text.choices().is_empty());
    }

    #[test]
    fn a_spellbook_window_knows_its_school() {
        assert_eq!(GumpKind::ChivalrySpellbook.school(), Some(School::Chivalry));
        assert_eq!(GumpKind::Paperdoll.school(), None);
    }
}
