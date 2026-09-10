//! Every spell a client can ask for, by number and by name.
//!
//! A cast goes to the server as a spell number. Players and scripts name the
//! spell instead, so the book maps each name to its number. A shard with
//! spells of its own adds them with [`SpellBook::add`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::name_key;

/// The school a spell belongs to. Each school has its own range of numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum School {
    Magery,
    Necromancy,
    Chivalry,
    Bushido,
    Ninjitsu,
    Spellweaving,
    Mysticism,
    Mastery,
    /// A spell a shard added. It has no standard number range.
    Custom,
}

/// What a spell does to the one it lands on. A heal is beneficial, a bolt is
/// harmful, and a recall is neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellFlag {
    Harmful,
    Beneficial,
    Neutral,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spell {
    pub id: u16,
    pub school: School,
    pub name: String,
    /// The words of power the caster says. Empty for the schools that say
    /// none.
    pub words: String,
    pub flag: SpellFlag,
}

impl Spell {
    /// The circle of a Magery spell, 1 to 8. Other schools have no circles.
    pub fn circle(&self) -> Option<u8> {
        (self.school == School::Magery)
            .then(|| ((self.id - FIRST_MAGERY) / MAGERY_SPELLS_PER_CIRCLE + 1) as u8)
    }
}

const FIRST_MAGERY: u16 = 1;
const MAGERY_SPELLS_PER_CIRCLE: u16 = 8;

use School::*;
use SpellFlag::{Beneficial as B, Harmful as H, Neutral as N};

/// One row of the standard table: number, school, name, words, flag.
type Row = (u16, School, &'static str, &'static str, SpellFlag);

#[rustfmt::skip]
const STANDARD: &[Row] = &[
    // Magery, circle 1 to 8.
    (1, Magery, "Clumsy", "Uus Jux", H),
    (2, Magery, "Create Food", "In Mani Ylem", N),
    (3, Magery, "Feeblemind", "Rel Wis", H),
    (4, Magery, "Heal", "In Mani", B),
    (5, Magery, "Magic Arrow", "In Por Ylem", H),
    (6, Magery, "Night Sight", "In Lor", B),
    (7, Magery, "Reactive Armor", "Flam Sanct", B),
    (8, Magery, "Weaken", "Des Mani", H),
    (9, Magery, "Agility", "Ex Uus", B),
    (10, Magery, "Cunning", "Uus Wis", B),
    (11, Magery, "Cure", "An Nox", B),
    (12, Magery, "Harm", "An Mani", H),
    (13, Magery, "Magic Trap", "In Jux", N),
    (14, Magery, "Magic Untrap", "An Jux", N),
    (15, Magery, "Protection", "Uus Sanct", B),
    (16, Magery, "Strength", "Uus Mani", B),
    (17, Magery, "Bless", "Rel Sanct", B),
    (18, Magery, "Fireball", "Vas Flam", H),
    (19, Magery, "Magic Lock", "An Por", N),
    (20, Magery, "Poison", "In Nox", H),
    (21, Magery, "Telekinesis", "Ort Por Ylem", N),
    (22, Magery, "Teleport", "Rel Por", N),
    (23, Magery, "Unlock", "Ex Por", N),
    (24, Magery, "Wall of Stone", "In Sanct Ylem", N),
    (25, Magery, "Arch Cure", "Vas An Nox", B),
    (26, Magery, "Arch Protection", "Vas Uus Sanct", B),
    (27, Magery, "Curse", "Des Sanct", H),
    (28, Magery, "Fire Field", "In Flam Grav", N),
    (29, Magery, "Greater Heal", "In Vas Mani", B),
    (30, Magery, "Lightning", "Por Ort Grav", H),
    (31, Magery, "Mana Drain", "Ort Rel", H),
    (32, Magery, "Recall", "Kal Ort Por", N),
    (33, Magery, "Blade Spirits", "In Jux Hur Ylem", N),
    (34, Magery, "Dispel Field", "An Grav", N),
    (35, Magery, "Incognito", "Kal In Ex", N),
    (36, Magery, "Magic Reflection", "In Jux Sanct", B),
    (37, Magery, "Mind Blast", "Por Corp Wis", H),
    (38, Magery, "Paralyze", "An Ex Por", H),
    (39, Magery, "Poison Field", "In Nox Grav", N),
    (40, Magery, "Summon Creature", "Kal Xen", N),
    (41, Magery, "Dispel", "An Ort", H),
    (42, Magery, "Energy Bolt", "Corp Por", H),
    (43, Magery, "Explosion", "Vas Ort Flam", H),
    (44, Magery, "Invisibility", "An Lor Xen", B),
    (45, Magery, "Mark", "Kal Por Ylem", N),
    (46, Magery, "Mass Curse", "Vas Des Sanct", H),
    (47, Magery, "Paralyze Field", "In Ex Grav", N),
    (48, Magery, "Reveal", "Wis Quas", N),
    (49, Magery, "Chain Lightning", "Vas Ort Grav", H),
    (50, Magery, "Energy Field", "In Sanct Grav", N),
    (51, Magery, "Flame Strike", "Kal Vas Flam", H),
    (52, Magery, "Gate Travel", "Vas Rel Por", N),
    (53, Magery, "Mana Vampire", "Ort Sanct", H),
    (54, Magery, "Mass Dispel", "Vas An Ort", N),
    (55, Magery, "Meteor Swarm", "Flam Kal Des Ylem", H),
    (56, Magery, "Polymorph", "Vas Ylem Rel", N),
    (57, Magery, "Earthquake", "In Vas Por", H),
    (58, Magery, "Energy Vortex", "Vas Corp Por", N),
    (59, Magery, "Resurrection", "An Corp", B),
    (60, Magery, "Summon Air Elemental", "Kal Vas Xen Hur", N),
    (61, Magery, "Summon Daemon", "Kal Vas Xen Corp", N),
    (62, Magery, "Summon Earth Elemental", "Kal Vas Xen Ylem", N),
    (63, Magery, "Summon Fire Elemental", "Kal Vas Xen Flam", N),
    (64, Magery, "Summon Water Elemental", "Kal Vas Xen An Flam", N),
    // Necromancy.
    (101, Necromancy, "Animate Dead", "Uus Corp", N),
    (102, Necromancy, "Blood Oath", "In Jux Mani Xen", H),
    (103, Necromancy, "Corpse Skin", "In Agle Corp Ylem", H),
    (104, Necromancy, "Curse Weapon", "An Sanct Gra Char", N),
    (105, Necromancy, "Evil Omen", "Pas Tym An Sanct", H),
    (106, Necromancy, "Horrific Beast", "Rel Xen Vas Bal", N),
    (107, Necromancy, "Lich Form", "Rel Xen Corp Ort", N),
    (108, Necromancy, "Mind Rot", "Wis An Ben", H),
    (109, Necromancy, "Pain Spike", "In Sar", H),
    (110, Necromancy, "Poison Strike", "In Vas Nox", H),
    (111, Necromancy, "Strangle", "In Bal Nox", H),
    (112, Necromancy, "Summon Familiar", "Kal Xen Bal", N),
    (113, Necromancy, "Vampiric Embrace", "Rel Xen An Sanct", N),
    (114, Necromancy, "Vengeful Spirit", "Kal Xen Bal Beh", H),
    (115, Necromancy, "Wither", "Kal Vas An Flam", H),
    (116, Necromancy, "Wraith Form", "Rel Xen Um", N),
    (117, Necromancy, "Exorcism", "Ort Corp Grav", N),
    // Chivalry.
    (201, Chivalry, "Cleanse by Fire", "Expor Flamus", B),
    (202, Chivalry, "Close Wounds", "Obsu Vulni", B),
    (203, Chivalry, "Consecrate Weapon", "Consecrus Arma", N),
    (204, Chivalry, "Dispel Evil", "Dispiro Malas", N),
    (205, Chivalry, "Divine Fury", "Divinum Furis", N),
    (206, Chivalry, "Enemy of One", "Forul Solum", N),
    (207, Chivalry, "Holy Light", "Augus Luminos", H),
    (208, Chivalry, "Noble Sacrifice", "Dium Prostra", B),
    (209, Chivalry, "Remove Curse", "Extermo Vomica", B),
    (210, Chivalry, "Sacred Journey", "Sanctum Viatas", N),
    // Bushido.
    (401, Bushido, "Honorable Execution", "", H),
    (402, Bushido, "Confidence", "", B),
    (403, Bushido, "Evasion", "", B),
    (404, Bushido, "Counter Attack", "", H),
    (405, Bushido, "Lightning Strike", "", H),
    (406, Bushido, "Momentum Strike", "", H),
    // Ninjitsu.
    (501, Ninjitsu, "Focus Attack", "", H),
    (502, Ninjitsu, "Death Strike", "", H),
    (503, Ninjitsu, "Animal Form", "", B),
    (504, Ninjitsu, "Ki Attack", "", H),
    (505, Ninjitsu, "Surprise Attack", "", H),
    (506, Ninjitsu, "Backstab", "", H),
    (507, Ninjitsu, "Shadowjump", "", N),
    (508, Ninjitsu, "Mirror Image", "", N),
    // Spellweaving.
    (601, Spellweaving, "Arcane Circle", "Myrshalee", N),
    (602, Spellweaving, "Gift of Renewal", "Olorisstra", B),
    (603, Spellweaving, "Immolating Weapon", "Thalshara", N),
    (604, Spellweaving, "Attunement", "Haeldril", B),
    (605, Spellweaving, "Thunderstorm", "Erelonia", H),
    (606, Spellweaving, "Nature's Fury", "Rauvvrae", N),
    (607, Spellweaving, "Summon Fey", "Alalithra", N),
    (608, Spellweaving, "Summon Fiend", "Nylisstra", N),
    (609, Spellweaving, "Reaper Form", "Tarisstree", N),
    (610, Spellweaving, "Wildfire", "Haelyn", H),
    (611, Spellweaving, "Essence of Wind", "Anathrae", H),
    (612, Spellweaving, "Dryad Allure", "Rathril", N),
    (613, Spellweaving, "Ethereal Voyage", "Orlavdra", N),
    (614, Spellweaving, "Word of Death", "Nyraxle", H),
    (615, Spellweaving, "Gift of Life", "Illorae", B),
    (616, Spellweaving, "Arcane Empowerment", "Aslavdra", B),
    // Mysticism.
    (678, Mysticism, "Nether Bolt", "In Corp Ylem", H),
    (679, Mysticism, "Healing Stone", "Kal In Mani", N),
    (680, Mysticism, "Purge Magic", "An Ort Sanct", B),
    (681, Mysticism, "Enchant", "In Ort Ylem", N),
    (682, Mysticism, "Sleep", "In Zu", H),
    (683, Mysticism, "Eagle Strike", "Kal Por Xen", H),
    (684, Mysticism, "Animated Weapon", "In Jux Por Ylem", N),
    (685, Mysticism, "Stone Form", "In Rel Ylem", N),
    (686, Mysticism, "Spell Trigger", "In Vas Ort Ex", N),
    (687, Mysticism, "Mass Sleep", "Vas Zu", H),
    (688, Mysticism, "Cleansing Winds", "In Vas Mani Hur", B),
    (689, Mysticism, "Bombard", "Corp Por Ylem", H),
    (690, Mysticism, "Spell Plague", "Vas Rel Jux Ort", H),
    (691, Mysticism, "Hail Storm", "Kal Des Ylem", H),
    (692, Mysticism, "Nether Cyclone", "Grav Hur", H),
    (693, Mysticism, "Rising Colossus", "Kal Vas Xen Corp Ylem", N),
    // Masteries.
    (701, Mastery, "Inspire", "Uus Por", B),
    (702, Mastery, "Invigorate", "An Zu", B),
    (703, Mastery, "Resilience", "Kal Mani Tym", B),
    (704, Mastery, "Perseverance", "Uus Jux Sanct", B),
    (705, Mastery, "Tribulation", "In Jux Hur Rel", H),
    (706, Mastery, "Despair", "Kal Des Mani Tym", H),
    (707, Mastery, "Death Ray", "In Grav Corp", H),
    (708, Mastery, "Ethereal Blast", "Uus Ort Grav", H),
    (709, Mastery, "Nether Blast", "In Vas Xen Por", H),
    (710, Mastery, "Mystic Weapon", "Vas Ylem Wis", B),
    (711, Mastery, "Command Undead", "In Corp Xen Por", N),
    (712, Mastery, "Conduit", "Uus Corp Grav", H),
    (713, Mastery, "Mana Shield", "Faerkulggen", B),
    (714, Mastery, "Summon Reaper", "Lartarisstree", N),
    (715, Mastery, "Enchanted Summoning", "", N),
    (716, Mastery, "Anticipate Hit", "", B),
    (717, Mastery, "Warcry", "", B),
    (718, Mastery, "Intuition", "", N),
    (719, Mastery, "Rejuvenate", "", B),
    (720, Mastery, "Holy Fist", "", H),
    (721, Mastery, "Shadow", "", B),
    (722, Mastery, "White Tiger Form", "", B),
    (723, Mastery, "Flaming Shot", "", H),
    (724, Mastery, "Playing The Odds", "", B),
    (725, Mastery, "Thrust", "", B),
    (726, Mastery, "Pierce", "", H),
    (727, Mastery, "Stagger", "", B),
    (728, Mastery, "Toughness", "", B),
    (729, Mastery, "Onslaught", "", H),
    (730, Mastery, "Focused Eye", "", B),
    (731, Mastery, "Elemental Fury", "", B),
    (732, Mastery, "Called Shot", "", B),
    (733, Mastery, "Saving Throw", "", B),
    (734, Mastery, "Shield Bash", "", H),
    (735, Mastery, "Bodyguard", "", B),
    (736, Mastery, "Heighten Senses", "", B),
    (737, Mastery, "Tolerance", "", B),
    (738, Mastery, "Injected Strike", "", B),
    (739, Mastery, "Potency", "", B),
    (740, Mastery, "Rampage", "", B),
    (741, Mastery, "Fists Of Fury", "", H),
    (742, Mastery, "Knockout", "", H),
    (743, Mastery, "Whispering", "", B),
    (744, Mastery, "Combat Training", "", B),
    (745, Mastery, "Boarding", "", B),
];

/// Other names players use for a spell, and the number each one means.
const ALIASES: &[(&str, u16)] = &[
    ("Air Elemental", 60),
    ("Earth Elemental", 62),
    ("Fire Elemental", 63),
    ("Water Elemental", 64),
    ("Daemon", 61),
];

/// Every spell the client knows, found by number or by name.
#[derive(Clone, Debug)]
pub struct SpellBook {
    spells: Vec<Spell>,
    by_name: HashMap<String, usize>,
    by_id: HashMap<u16, usize>,
}

impl SpellBook {
    /// The spells of the standard schools.
    pub fn standard() -> Self {
        let mut book = Self {
            spells: Vec::with_capacity(STANDARD.len()),
            by_name: HashMap::with_capacity(STANDARD.len()),
            by_id: HashMap::with_capacity(STANDARD.len()),
        };
        for &(id, school, name, words, flag) in STANDARD {
            book.add(Spell {
                id,
                school,
                name: name.into(),
                words: words.into(),
                flag,
            });
        }
        for &(alias, id) in ALIASES {
            if let Some(&index) = book.by_id.get(&id) {
                book.by_name.insert(name_key(alias), index);
            }
        }
        book
    }

    /// Adds a spell, or replaces the one with the same number or name. A
    /// shard's own spells go in this way.
    pub fn add(&mut self, spell: Spell) {
        let key = name_key(&spell.name);
        let slot = self
            .by_id
            .get(&spell.id)
            .or_else(|| self.by_name.get(&key))
            .copied();
        let index = match slot {
            Some(index) => {
                let old = &self.spells[index];
                self.by_name.remove(&name_key(&old.name));
                self.by_id.remove(&old.id);
                self.spells[index] = spell;
                index
            }
            None => {
                self.spells.push(spell);
                self.spells.len() - 1
            }
        };
        let spell = &self.spells[index];
        self.by_name.insert(key, index);
        self.by_id.insert(spell.id, index);
    }

    /// The spell with this name, however the player types it.
    pub fn by_name(&self, name: &str) -> Option<&Spell> {
        self.by_name.get(&name_key(name)).map(|&i| &self.spells[i])
    }

    pub fn by_id(&self, id: u16) -> Option<&Spell> {
        self.by_id.get(&id).map(|&i| &self.spells[i])
    }

    /// A spell named by a number or by a name, as a script writes it.
    pub fn find(&self, number_or_name: &str) -> Option<&Spell> {
        let text = number_or_name.trim();
        match text.parse::<u16>() {
            Ok(id) => self.by_id(id),
            Err(_) => self.by_name(text),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Spell> {
        self.spells.iter()
    }
}

impl Default for SpellBook {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREATER_HEAL: u16 = 29;

    #[test]
    fn a_spell_is_found_by_name_or_number() {
        let book = SpellBook::standard();
        assert_eq!(
            book.by_name("greater heal").map(|s| s.id),
            Some(GREATER_HEAL)
        );
        assert_eq!(
            book.find("29").map(|s| s.name.as_str()),
            Some("Greater Heal")
        );
        assert_eq!(book.find("Nature's Fury").map(|s| s.id), Some(606));
        assert!(book.find("no such spell").is_none());
        assert_eq!(book.find("Flamestrike").map(|s| s.id), Some(51));
        assert_eq!(book.find("air elemental").map(|s| s.id), Some(60));
    }

    #[test]
    fn every_number_and_name_is_used_once() {
        let book = SpellBook::standard();
        assert_eq!(book.iter().count(), STANDARD.len());
        assert_eq!(book.by_id.len(), STANDARD.len());
        assert_eq!(book.by_name.len(), STANDARD.len() + ALIASES.len());
    }

    #[test]
    fn a_magery_spell_knows_its_circle() {
        let book = SpellBook::standard();
        assert_eq!(book.by_id(1).and_then(Spell::circle), Some(1));
        assert_eq!(book.by_id(GREATER_HEAL).and_then(Spell::circle), Some(4));
        assert_eq!(book.by_id(64).and_then(Spell::circle), Some(8));
        assert_eq!(book.by_id(202).and_then(Spell::circle), None);
    }

    #[test]
    fn a_shard_spell_is_added_and_can_replace_one() {
        const SHARD_SPELL: u16 = 900;
        let mut book = SpellBook::standard();
        book.add(Spell {
            id: SHARD_SPELL,
            school: School::Custom,
            name: "Holy Nova".into(),
            words: String::new(),
            flag: SpellFlag::Harmful,
        });
        assert_eq!(book.by_name("holy nova").map(|s| s.id), Some(SHARD_SPELL));
        book.add(Spell {
            id: SHARD_SPELL,
            school: School::Custom,
            name: "Holy Burst".into(),
            words: String::new(),
            flag: SpellFlag::Harmful,
        });
        assert!(book.by_name("holy nova").is_none(), "the old name is gone");
        assert_eq!(book.by_name("holy burst").map(|s| s.id), Some(SHARD_SPELL));
    }
}
