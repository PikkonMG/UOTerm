//! Mobile facts the assistant looks up by name: notoriety and body groups.

use crate::name_key;
use uoterm_protocol::types::{
    NOTO_CRIMINAL, NOTO_ENEMY, NOTO_FRIEND, NOTO_GREY, NOTO_INNOCENT, NOTO_INVULNERABLE,
    NOTO_MURDERER,
};

/// The notoriety a name stands for in a script. Some names stand for more
/// than one: "gray" covers both kinds of grey.
pub fn notorieties(name: &str) -> &'static [u8] {
    match name_key(name).as_str() {
        "innocent" | "blue" => &[NOTO_INNOCENT],
        "friend" | "green" | "ally" => &[NOTO_FRIEND],
        "gray" | "grey" => &[NOTO_GREY, NOTO_CRIMINAL],
        "criminal" => &[NOTO_CRIMINAL],
        "enemy" | "orange" => &[NOTO_ENEMY],
        "murderer" | "red" => &[NOTO_MURDERER],
        "invulnerable" | "yellow" => &[NOTO_INVULNERABLE],
        "any" => &ANY_NOTORIETY,
        _ => &[],
    }
}

const ANY_NOTORIETY: [u8; 7] = [
    NOTO_INNOCENT,
    NOTO_FRIEND,
    NOTO_GREY,
    NOTO_CRIMINAL,
    NOTO_ENEMY,
    NOTO_MURDERER,
    NOTO_INVULNERABLE,
];

pub fn notoriety_name(notoriety: u8) -> &'static str {
    match notoriety {
        NOTO_INNOCENT => "innocent",
        NOTO_FRIEND => "friend",
        NOTO_GREY => "gray",
        NOTO_CRIMINAL => "criminal",
        NOTO_ENEMY => "enemy",
        NOTO_MURDERER => "murderer",
        NOTO_INVULNERABLE => "invulnerable",
        _ => "unknown",
    }
}

/// The bodies of people: humans, elves and gargoyles.
pub const HUMANOID_BODIES: [u16; 6] = [0x0190, 0x0191, 0x025D, 0x025E, 0x029A, 0x029B];

/// The bodies a player takes in a form spell or a disguise.
pub const TRANSFORMATION_BODIES: [u16; 13] = [
    0x02EA, 0x02EC, 0x02EB, 0x02ED, 0x02E8, 0x02E9, 0x00DC, 0x0019, 0x0302, 0x011D, 0x02C1, 0x00B7,
    0x00B8,
];

pub fn is_humanoid(body: u16) -> bool {
    HUMANOID_BODIES.contains(&body)
}

pub fn is_transformed(body: u16) -> bool {
    TRANSFORMATION_BODIES.contains(&body)
}

/// The species of the creature each body shows, for the bodies of the
/// classic creatures. A named creature is still of its species: "Gruuk" on the
/// body of an orc is an orc. A body not listed here is known by its name.
pub const SPECIES_BY_BODY: &[(u16, &str)] = &[
    (0x0001, "ogre"),
    (0x0002, "sea serpent"),
    (0x0003, "zombie"),
    (0x0004, "gargoyle"),
    (0x0005, "eagle"),
    (0x0006, "bird"),
    (0x0007, "orc"),
    (0x0008, "corpser"),
    (0x0009, "daemon"),
    (0x000A, "daemon"),
    (0x000B, "dread spider"),
    (0x000C, "dragon"),
    (0x000D, "air elemental"),
    (0x000E, "earth elemental"),
    (0x000F, "fire elemental"),
    (0x0010, "water elemental"),
    (0x0011, "orc"),
    (0x0012, "ettin"),
    (0x0014, "frost spider"),
    (0x0015, "giant serpent"),
    (0x0016, "gazer"),
    (0x0017, "dire wolf"),
    (0x0018, "lich"),
    (0x0019, "grey wolf"),
    (0x001A, "shade"),
    (0x001B, "grey wolf"),
    (0x001C, "giant spider"),
    (0x001D, "gorilla"),
    (0x001E, "harpy"),
    (0x001F, "headless one"),
    (0x0022, "white wolf"),
    (0x0023, "lizardman"),
    (0x0024, "lizardman"),
    (0x0025, "white wolf"),
    (0x0027, "mongbat"),
    (0x0028, "balron"),
    (0x002A, "ratman"),
    (0x002B, "ice fiend"),
    (0x002E, "ancient wyrm"),
    (0x002F, "reaper"),
    (0x0030, "scorpion"),
    (0x0032, "skeleton"),
    (0x0033, "slime"),
    (0x0034, "snake"),
    (0x0035, "troll"),
    (0x0036, "troll"),
    (0x0037, "frost troll"),
    (0x0038, "skeleton"),
    (0x0039, "bone knight"),
    (0x003A, "wisp"),
    (0x003B, "dragon"),
    (0x003D, "cold drake"),
    (0x003E, "wyvern"),
    (0x003F, "cougar"),
    (0x0040, "snow leopard"),
    (0x0041, "snow leopard"),
    (0x0042, "swamp tentacle"),
    (0x0043, "stone gargoyle"),
    (0x0046, "terathan warrior"),
    (0x0047, "terathan drone"),
    (0x0048, "terathan matriarch"),
    (0x0049, "stone harpy"),
    (0x004A, "imp"),
    (0x004B, "cyclops"),
    (0x004C, "titan"),
    (0x004D, "kraken"),
    (0x004E, "ancient lich"),
    (0x004F, "lich lord"),
    (0x0050, "giant toad"),
    (0x0051, "bull frog"),
    (0x0053, "ogre lord"),
    (0x0055, "ophidian mage"),
    (0x0056, "ophidian warrior"),
    (0x0057, "ophidian matriarch"),
    (0x0058, "mountain goat"),
    (0x0059, "ice serpent"),
    (0x005A, "lava serpent"),
    (0x005C, "silver serpent"),
    (0x005E, "frost ooze"),
    (0x005F, "turkey"),
    (0x0062, "hell hound"),
    (0x0063, "dark wolf"),
    (0x0065, "centaur"),
    (0x0067, "serpentine dragon"),
    (0x0068, "skeletal dragon"),
    (0x006A, "shadow wyrm"),
    (0x0074, "nightmare"),
    (0x007A, "unicorn"),
    (0x007C, "evil mage"),
    (0x007D, "evil mage lord"),
    (0x007F, "hell cat"),
    (0x0080, "pixie"),
    (0x0082, "fire gargoyle"),
    (0x0083, "efreet"),
    (0x0084, "kirin"),
    (0x0087, "arctic ogre lord"),
    (0x008A, "orcish lord"),
    (0x008C, "orcish mage"),
    (0x008E, "ratman archer"),
    (0x008F, "ratman mage"),
    (0x0090, "sea horse"),
    (0x0092, "harrower"),
    (0x0093, "skeletal knight"),
    (0x0094, "bone magi"),
    (0x0095, "succubus"),
    (0x0096, "sea serpent"),
    (0x0097, "dolphin"),
    (0x0098, "terathan avenger"),
    (0x0099, "ghoul"),
    (0x009A, "mummy"),
    (0x009B, "rotting corpse"),
    (0x009D, "giant black widow"),
    (0x00A4, "energy vortex"),
    (0x00A5, "wisp"),
    (0x00A7, "brown bear"),
    (0x00B5, "orc scout"),
    (0x00B6, "orc bomber"),
    (0x00B7, "savage"),
    (0x00B8, "savage"),
    (0x00B9, "savage rider"),
    (0x00BA, "savage shaman"),
    (0x00BB, "ridgeback"),
    (0x00BC, "savage ridgeback"),
    (0x00BD, "orc brute"),
    (0x00C8, "horse"),
    (0x00C9, "cat"),
    (0x00CA, "alligator"),
    (0x00CB, "pig"),
    (0x00CC, "horse"),
    (0x00CD, "rabbit"),
    (0x00CE, "lava lizard"),
    (0x00CF, "sheep"),
    (0x00D0, "chicken"),
    (0x00D1, "goat"),
    (0x00D2, "desert ostard"),
    (0x00D3, "black bear"),
    (0x00D4, "grizzly bear"),
    (0x00D5, "polar bear"),
    (0x00D6, "panther"),
    (0x00D7, "giant rat"),
    (0x00D8, "cow"),
    (0x00D9, "dog"),
    (0x00DA, "frenzied ostard"),
    (0x00DB, "forest ostard"),
    (0x00DC, "llama"),
    (0x00DD, "walrus"),
    (0x00E1, "timber wolf"),
    (0x00E2, "horse"),
    (0x00E4, "horse"),
    (0x00E7, "cow"),
    (0x00E8, "bull"),
    (0x00E9, "bull"),
    (0x00EA, "great hart"),
    (0x00ED, "hind"),
    (0x00EE, "rat"),
    (0x0122, "boar"),
    (0x0123, "pack horse"),
    (0x0124, "pack llama"),
];

/// The species the body of a creature shows, when the table knows it.
pub fn species_of_body(body: u16) -> Option<&'static str> {
    SPECIES_BY_BODY
        .iter()
        .find(|(listed, _)| *listed == body)
        .map(|(_, species)| *species)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_name_means_its_notoriety() {
        assert_eq!(notorieties("red"), &[NOTO_MURDERER]);
        assert_eq!(notorieties("Gray"), &[NOTO_GREY, NOTO_CRIMINAL]);
        assert_eq!(notorieties("any").len(), 7);
        assert!(notorieties("purple").is_empty());
    }

    #[test]
    fn every_notoriety_has_a_name() {
        for noto in ANY_NOTORIETY {
            assert!(!notorieties(notoriety_name(noto)).is_empty());
        }
    }

    #[test]
    fn bodies_fall_in_their_groups() {
        const HUMAN_MALE: u16 = 0x0190;
        const WOLF: u16 = 0x0019;
        assert!(is_humanoid(HUMAN_MALE) && !is_transformed(HUMAN_MALE));
        assert!(is_transformed(WOLF));
    }

    #[test]
    fn a_named_creature_is_still_of_its_body_species() {
        const ORC: u16 = 0x0011;
        const HUMAN_MALE: u16 = 0x0190;
        assert_eq!(species_of_body(ORC), Some("orc"));
        assert_eq!(species_of_body(HUMAN_MALE), None, "a person is no species");
    }
}
