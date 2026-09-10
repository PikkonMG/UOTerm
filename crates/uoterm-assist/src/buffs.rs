//! Buff icons by number, for when the client text files are not at hand.
//! The shard names each buff with a client text number; this table is the
//! fallback name for its icon.

use crate::name_key;

#[rustfmt::skip]
const ICONS: &[(u16, &str)] = &[
    (1001, "Dismount"),
    (1002, "Disarm"),
    (1005, "Night Sight"),
    (1006, "Death Strike"),
    (1007, "Evil Omen"),
    (1009, "Regeneration"),
    (1010, "Divine Fury"),
    (1011, "Enemy Of One"),
    (1012, "Hiding"),
    (1013, "Meditation"),
    (1014, "Blood Oath Caster"),
    (1015, "Blood Oath"),
    (1016, "Corpse Skin"),
    (1017, "Mind Rot"),
    (1018, "Pain Spike"),
    (1019, "Strangle"),
    (1020, "Gift of Renewal"),
    (1021, "Attune Weapon"),
    (1022, "Thunderstorm"),
    (1023, "Essence of Wind"),
    (1024, "Ethereal Voyage"),
    (1025, "Gift Of Life"),
    (1026, "Arcane Empowerment"),
    (1027, "Mortal Strike"),
    (1028, "Reactive Armor"),
    (1029, "Protection"),
    (1030, "Arch Protection"),
    (1031, "Magic Reflection"),
    (1032, "Incognito"),
    (1033, "Disguised"),
    (1034, "Animal Form"),
    (1035, "Polymorph"),
    (1036, "Invisibility"),
    (1037, "Paralyze"),
    (1038, "Poison"),
    (1039, "Bleed"),
    (1040, "Clumsy"),
    (1041, "Feeblemind"),
    (1042, "Weaken"),
    (1043, "Curse"),
    (1044, "Mass Curse"),
    (1045, "Agility"),
    (1046, "Cunning"),
    (1047, "Strength"),
    (1048, "Bless"),
    (1049, "Sleep"),
    (1051, "Spell Plague"),
];

/// The name of a buff icon.
pub fn icon_name(icon: u16) -> Option<&'static str> {
    ICONS.iter().find(|(i, _)| *i == icon).map(|&(_, n)| n)
}

/// The icon of a buff by its name, however the player types it.
pub fn icon_by_name(name: &str) -> Option<u16> {
    let key = name_key(name);
    ICONS
        .iter()
        .find(|(_, n)| name_key(n) == key)
        .map(|&(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_icon_has_a_name_both_ways() {
        const BLESS: u16 = 1048;
        assert_eq!(icon_name(BLESS), Some("Bless"));
        assert_eq!(icon_by_name("divine fury"), Some(1010));
        assert_eq!(icon_name(1), None);
    }
}
