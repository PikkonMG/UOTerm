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
}
