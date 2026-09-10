//! Weapon special moves: which two moves each weapon has.
//!
//! A player arms "the primary move" or "the secondary move". The server only
//! takes a move by its number, so the client has to know the two moves of the
//! weapon in hand. Bare hands have a pair of their own.

use crate::name_key;

/// Every special move by number, the number the server takes.
#[rustfmt::skip]
pub const ABILITIES: &[(u8, &str)] = &[
    (1, "Armor Ignore"),
    (2, "Bleed Attack"),
    (3, "Concussion Blow"),
    (4, "Crushing Blow"),
    (5, "Disarm"),
    (6, "Dismount"),
    (7, "Double Strike"),
    (8, "Infectious Strike"),
    (9, "Mortal Strike"),
    (10, "Moving Shot"),
    (11, "Paralyzing Blow"),
    (12, "Shadow Strike"),
    (13, "Whirlwind Attack"),
    (14, "Riding Swipe"),
    (15, "Frenzied Whirlwind"),
    (16, "Block"),
    (17, "Defense Mastery"),
    (18, "Nerve Strike"),
    (19, "Talon Strike"),
    (20, "Feint"),
    (21, "Dual Wield"),
    (22, "Double Shot"),
    (23, "Armor Pierce"),
    (24, "Bladeweave"),
    (25, "Force Arrow"),
    (26, "Lightning Arrow"),
    (27, "Psychic Attack"),
    (28, "Serpent Arrow"),
    (29, "Force of Nature"),
    (30, "Infused Throw"),
    (31, "Mystic Arc"),
];

const PARALYZING_BLOW: u8 = 11;
const DISARM: u8 = 5;
/// The two moves of bare hands.
pub const BARE_HANDS: (u8, u8) = (PARALYZING_BLOW, DISARM);

/// Weapon graphic, primary move, secondary move.
#[rustfmt::skip]
const WEAPONS: &[(u16, u8, u8)] = &[
    (0x0481, 13, 4),
    (0x08FD, 7, 8),
    (0x08FE, 2, 11),
    (0x08FF, 31, 3),
    (0x0900, 1, 11),
    (0x0901, 10, 30),
    (0x0902, 8, 12),
    (0x0903, 1, 5),
    (0x0904, 7, 5),
    (0x0905, 7, 9),
    (0x0906, 4, 6),
    (0x0908, 13, 6),
    (0x090A, 1, 9),
    (0x090B, 4, 3),
    (0x090C, 2, 9),
    (0x0DF0, 13, 11),
    (0x0DF1, 13, 11),
    (0x0DF2, 6, 5),
    (0x0DF3, 6, 5),
    (0x0DF4, 6, 5),
    (0x0DF5, 6, 5),
    (0x0E81, 4, 5),
    (0x0E82, 4, 5),
    (0x0E85, 7, 5),
    (0x0E86, 7, 5),
    (0x0E87, 2, 6),
    (0x0E88, 2, 6),
    (0x0E89, 7, 3),
    (0x0E8A, 7, 3),
    (0x0EC2, 2, 8),
    (0x0EC3, 2, 8),
    (0x0EC4, 12, 2),
    (0x0EC5, 12, 2),
    (0x0F43, 1, 5),
    (0x0F44, 1, 5),
    (0x0F45, 2, 9),
    (0x0F46, 2, 9),
    (0x0F47, 2, 3),
    (0x0F48, 2, 3),
    (0x0F49, 4, 6),
    (0x0F4A, 4, 6),
    (0x0F4B, 7, 13),
    (0x0F4C, 7, 13),
    (0x0F4D, 11, 6),
    (0x0F4E, 11, 6),
    (0x0F4F, 3, 9),
    (0x0F50, 3, 9),
    (0x0F51, 8, 12),
    (0x0F52, 8, 12),
    (0x0F5C, 3, 5),
    (0x0F5D, 3, 5),
    (0x0F5E, 4, 1),
    (0x0F5F, 4, 1),
    (0x0F60, 1, 3),
    (0x0F61, 1, 3),
    (0x0F62, 1, 11),
    (0x0F63, 1, 11),
    (0x0FB5, 4, 12),
    (0x13AF, 1, 2),
    (0x13B0, 1, 2),
    (0x13B1, 11, 9),
    (0x13B2, 11, 9),
    (0x13B3, 12, 6),
    (0x13B4, 12, 6),
    (0x13B6, 7, 11),
    (0x13B7, 7, 11),
    (0x13B8, 7, 11),
    (0x13B9, 11, 4),
    (0x13BA, 11, 4),
    (0x13E3, 4, 12),
    (0x13F6, 8, 5),
    (0x13F8, 3, 29),
    (0x13FB, 13, 2),
    (0x13FD, 10, 6),
    (0x13FF, 7, 1),
    (0x1401, 1, 8),
    (0x1402, 12, 9),
    (0x1403, 12, 9),
    (0x1404, 2, 5),
    (0x1405, 2, 5),
    (0x1406, 4, 9),
    (0x1407, 4, 9),
    (0x1438, 13, 4),
    (0x1439, 13, 4),
    (0x143A, 7, 3),
    (0x143B, 7, 3),
    (0x143C, 1, 9),
    (0x143D, 1, 9),
    (0x143E, 13, 3),
    (0x143F, 13, 3),
    (0x1440, 2, 12),
    (0x1441, 2, 12),
    (0x1442, 7, 12),
    (0x1443, 7, 12),
    (0x26BA, 2, 11),
    (0x26BB, 11, 9),
    (0x26BC, 4, 9),
    (0x26BD, 1, 6),
    (0x26BE, 11, 8),
    (0x26BF, 7, 8),
    (0x26C0, 6, 3),
    (0x26C1, 7, 9),
    (0x26C2, 1, 10),
    (0x26C3, 7, 10),
    (0x26C4, 2, 11),
    (0x26C5, 11, 9),
    (0x26C6, 4, 9),
    (0x26C7, 1, 6),
    (0x26C8, 11, 8),
    (0x26C9, 7, 8),
    (0x26CA, 6, 3),
    (0x26CB, 7, 9),
    (0x26CC, 1, 10),
    (0x26CD, 7, 10),
    (0x26CE, 13, 5),
    (0x26CF, 13, 5),
    (0x27A2, 4, 14),
    (0x27A3, 20, 16),
    (0x27A4, 15, 7),
    (0x27A5, 23, 22),
    (0x27A6, 15, 4),
    (0x27A7, 17, 15),
    (0x27A8, 20, 18),
    (0x27A9, 20, 7),
    (0x27AA, 5, 11),
    (0x27AB, 21, 19),
    (0x27AD, 13, 17),
    (0x27AE, 16, 20),
    (0x27AF, 16, 23),
    (0x27ED, 4, 14),
    (0x27EE, 20, 16),
    (0x27EF, 15, 7),
    (0x27F0, 23, 22),
    (0x27F1, 15, 4),
    (0x27F2, 17, 15),
    (0x27F3, 20, 18),
    (0x27F4, 20, 7),
    (0x27F5, 5, 11),
    (0x27F6, 21, 19),
    (0x27F8, 13, 17),
    (0x27F9, 16, 20),
    (0x27FA, 16, 23),
    (0x2D1E, 25, 28),
    (0x2D1F, 26, 27),
    (0x2D20, 27, 2),
    (0x2D21, 8, 12),
    (0x2D22, 20, 1),
    (0x2D23, 5, 24),
    (0x2D24, 3, 4),
    (0x2D25, 16, 29),
    (0x2D26, 5, 24),
    (0x2D27, 13, 24),
    (0x2D28, 5, 4),
    (0x2D29, 17, 24),
    (0x2D2A, 25, 28),
    (0x2D2B, 26, 27),
    (0x2D2C, 27, 2),
    (0x2D2D, 8, 12),
    (0x2D2E, 20, 1),
    (0x2D2F, 5, 24),
    (0x2D30, 3, 4),
    (0x2D31, 16, 29),
    (0x2D32, 5, 24),
    (0x2D33, 13, 24),
    (0x2D34, 5, 4),
    (0x2D35, 17, 24),
    (0x4067, 31, 3),
    (0x4068, 7, 8),
    (0x406B, 1, 9),
    (0x406C, 10, 30),
    (0x406D, 7, 5),
    (0x406E, 1, 5),
    (0x4071, 1, 11),
    (0x4072, 2, 11),
    (0x4074, 4, 3),
    (0x4075, 13, 6),
    (0x4076, 1, 9),
    (0x48AE, 2, 8),
    (0x48B0, 2, 3),
    (0x48B2, 4, 6),
    (0x48B3, 4, 6),
    (0x48B4, 11, 6),
    (0x48B5, 11, 6),
    (0x48B6, 8, 5),
    (0x48B7, 8, 5),
    (0x48B8, 3, 11),
    (0x48B9, 3, 11),
    (0x48BA, 7, 1),
    (0x48BB, 7, 1),
    (0x48BC, 1, 8),
    (0x48BD, 1, 8),
    (0x48BE, 2, 5),
    (0x48BF, 2, 5),
    (0x48C0, 13, 4),
    (0x48C2, 7, 3),
    (0x48C3, 7, 3),
    (0x48C4, 2, 11),
    (0x48C5, 2, 11),
    (0x48C6, 11, 9),
    (0x48C7, 11, 9),
    (0x48C8, 11, 8),
    (0x48C9, 11, 8),
    (0x48CA, 6, 3),
    (0x48CB, 6, 3),
    (0x48CC, 20, 16),
    (0x48CD, 20, 16),
    (0x48CE, 21, 19),
    (0x48CF, 21, 19),
    (0x48D0, 20, 7),
    (0x48D1, 20, 7),
    (0xA289, 3, 13),
    (0xA28A, 23, 13),
    (0xA28B, 2, 13),
    (0xA291, 3, 13),
    (0xA292, 23, 13),
    (0xA293, 2, 13),
    (0xAEA4, 7, 13),
    (0xAEA5, 7, 1),
    (0xAEB3, 7, 13),
    (0xAEB4, 7, 1),
    (0xAEC2, 7, 13),
    (0xAEC3, 7, 1),
    (0xAED1, 7, 13),
    (0xAED2, 7, 1),
];

/// Which of the weapon's two moves a player asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveSlot {
    Primary,
    Secondary,
}

/// The primary and secondary move of the weapon with this graphic, or of
/// bare hands when there is no weapon. A graphic the table does not hold
/// gets the bare-hand pair.
pub fn weapon_moves(weapon_graphic: Option<u16>) -> (u8, u8) {
    weapon_graphic
        .and_then(|graphic| {
            WEAPONS
                .binary_search_by_key(&graphic, |&(g, _, _)| g)
                .ok()
                .map(|i| (WEAPONS[i].1, WEAPONS[i].2))
        })
        .unwrap_or(BARE_HANDS)
}

/// The move number for one slot of the weapon in hand.
pub fn move_for(weapon_graphic: Option<u16>, slot: MoveSlot) -> u8 {
    let (primary, secondary) = weapon_moves(weapon_graphic);
    match slot {
        MoveSlot::Primary => primary,
        MoveSlot::Secondary => secondary,
    }
}

/// A move by its name, however the player types it.
pub fn ability_by_name(name: &str) -> Option<u8> {
    let key = name_key(name);
    ABILITIES
        .iter()
        .find(|(_, n)| name_key(n) == key)
        .map(|&(id, _)| id)
}

pub fn ability_name(id: u8) -> Option<&'static str> {
    ABILITIES.iter().find(|(i, _)| *i == id).map(|&(_, n)| n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KATANA: u16 = 0x13FF;
    const DOUBLE_STRIKE: u8 = 7;
    const ARMOR_IGNORE: u8 = 1;

    #[test]
    fn a_katana_has_double_strike_and_armor_ignore() {
        assert_eq!(weapon_moves(Some(KATANA)), (DOUBLE_STRIKE, ARMOR_IGNORE));
        assert_eq!(move_for(Some(KATANA), MoveSlot::Secondary), ARMOR_IGNORE);
    }

    #[test]
    fn bare_hands_and_unknown_weapons_use_the_bare_hand_pair() {
        const NOT_A_WEAPON: u16 = 0x0001;
        assert_eq!(weapon_moves(None), BARE_HANDS);
        assert_eq!(weapon_moves(Some(NOT_A_WEAPON)), BARE_HANDS);
    }

    #[test]
    fn the_weapon_table_is_sorted_for_search() {
        assert!(WEAPONS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn every_weapon_move_has_a_name() {
        assert!(WEAPONS
            .iter()
            .all(|&(_, a, b)| ability_name(a).is_some() && ability_name(b).is_some()));
        assert_eq!(ability_by_name("mortal strike"), Some(9));
    }
}
