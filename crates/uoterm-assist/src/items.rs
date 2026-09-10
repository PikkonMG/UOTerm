//! Item facts the assistant looks up by name: potions, reagents, foods.

use crate::name_key;
use uoterm_protocol::types::{
    GRAPHIC_BANDAGE, GRAPHIC_POTION_CURE, GRAPHIC_POTION_HEAL, GRAPHIC_POTION_REFRESH,
};

/// The hue of an ordinary item that was never dyed.
pub const PLAIN_HUE: u16 = 0;

pub const GRAPHIC_POTION_AGILITY: u16 = 0x0F08;
pub const GRAPHIC_POTION_STRENGTH: u16 = 0x0F09;
pub const GRAPHIC_POTION_POISON: u16 = 0x0F0A;
pub const GRAPHIC_POTION_EXPLOSION: u16 = 0x0F0D;
pub const GRAPHIC_POTION_NIGHT_SIGHT: u16 = 0x0F06;

/// A potion kind: the item graphic and hue that make it that potion. Several
/// potions share a bottle and differ only by hue, so both must match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Potion {
    pub name: &'static str,
    pub graphic: u16,
    pub hue: u16,
}

#[rustfmt::skip]
pub const POTIONS: &[Potion] = &[
    Potion { name: "heal", graphic: GRAPHIC_POTION_HEAL, hue: PLAIN_HUE },
    Potion { name: "cure", graphic: GRAPHIC_POTION_CURE, hue: PLAIN_HUE },
    Potion { name: "refresh", graphic: GRAPHIC_POTION_REFRESH, hue: PLAIN_HUE },
    Potion { name: "agility", graphic: GRAPHIC_POTION_AGILITY, hue: PLAIN_HUE },
    Potion { name: "strength", graphic: GRAPHIC_POTION_STRENGTH, hue: PLAIN_HUE },
    Potion { name: "poison", graphic: GRAPHIC_POTION_POISON, hue: PLAIN_HUE },
    Potion { name: "explosion", graphic: GRAPHIC_POTION_EXPLOSION, hue: PLAIN_HUE },
    Potion { name: "night sight", graphic: GRAPHIC_POTION_NIGHT_SIGHT, hue: PLAIN_HUE },
    Potion { name: "shatter", graphic: GRAPHIC_POTION_EXPLOSION, hue: 0x003C },
    Potion { name: "exploding tar", graphic: GRAPHIC_POTION_EXPLOSION, hue: 0x0455 },
    Potion { name: "fear essence", graphic: GRAPHIC_POTION_EXPLOSION, hue: 0x0005 },
    Potion { name: "parasitic", graphic: GRAPHIC_POTION_POISON, hue: 0x017C },
    Potion { name: "darkglow poison", graphic: GRAPHIC_POTION_POISON, hue: 0x0096 },
    Potion { name: "supernova", graphic: GRAPHIC_POTION_STRENGTH, hue: 0x000D },
    Potion { name: "confusion blast", graphic: GRAPHIC_POTION_NIGHT_SIGHT, hue: 0x048D },
    Potion { name: "conflagration", graphic: GRAPHIC_POTION_NIGHT_SIGHT, hue: 0x0489 },
    Potion { name: "invisibility", graphic: GRAPHIC_POTION_NIGHT_SIGHT, hue: 0x0132 },
];

/// A potion by name. "heal", "Heal Potion" and "greater heal potion" all find
/// the heal potion: the word "potion" and a strength word are dropped.
pub fn potion(name: &str) -> Option<&'static Potion> {
    let mut key = name_key(name);
    for word in POTION_WORDS_IGNORED {
        key = key.replace(word, "");
    }
    POTIONS.iter().find(|p| name_key(p.name) == key)
}

/// Words a player may add to a potion name that do not change its kind.
const POTION_WORDS_IGNORED: [&str; 4] = ["potion", "greater", "lesser", "total"];

/// A counted supply: a short name a script uses, and the item it counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Supply {
    pub short: &'static str,
    pub name: &'static str,
    pub graphic: u16,
}

#[rustfmt::skip]
pub const SUPPLIES: &[Supply] = &[
    Supply { short: "band", name: "Bandages", graphic: GRAPHIC_BANDAGE },
    Supply { short: "bp", name: "Black Pearl", graphic: 0x0F7A },
    Supply { short: "bm", name: "Blood Moss", graphic: 0x0F7B },
    Supply { short: "gl", name: "Garlic", graphic: 0x0F84 },
    Supply { short: "gs", name: "Ginseng", graphic: 0x0F85 },
    Supply { short: "mr", name: "Mandrake Root", graphic: 0x0F86 },
    Supply { short: "ns", name: "Nightshade", graphic: 0x0F88 },
    Supply { short: "sa", name: "Sulfurous Ash", graphic: 0x0F8C },
    Supply { short: "ss", name: "Spider's Silk", graphic: 0x0F8D },
    Supply { short: "bw", name: "Bat Wing", graphic: 0x0F78 },
    Supply { short: "db", name: "Daemon Blood", graphic: 0x0F7D },
    Supply { short: "gd", name: "Grave Dust", graphic: 0x0F8F },
    Supply { short: "nc", name: "Nox Crystal", graphic: 0x0F8E },
    Supply { short: "pi", name: "Pig Iron", graphic: 0x0F8A },
];

/// A supply by its short name or its full name.
pub fn supply(name: &str) -> Option<&'static Supply> {
    let key = name_key(name);
    SUPPLIES
        .iter()
        .find(|s| s.short == key || name_key(s.name) == key)
}

/// The food groups a pet or a player is fed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodGroup {
    Fish,
    FruitsAndVegetables,
    Meat,
}

impl FoodGroup {
    pub fn by_name(name: &str) -> Option<Self> {
        match name_key(name).as_str() {
            "fish" => Some(Self::Fish),
            "fruitsandvegetables" | "fruit" | "fruits" | "vegetables" => {
                Some(Self::FruitsAndVegetables)
            }
            "meat" => Some(Self::Meat),
            _ => None,
        }
    }
}

#[rustfmt::skip]
pub const FOODS: &[(FoodGroup, &str, u16)] = &[
    (FoodGroup::Fish, "Fish Steak", 0x097B),
    (FoodGroup::Fish, "Raw Fish Steak", 0x097A),
    (FoodGroup::FruitsAndVegetables, "Honeydew Melon", 0x0C74),
    (FoodGroup::FruitsAndVegetables, "Yellow Gourd", 0x0C64),
    (FoodGroup::FruitsAndVegetables, "Green Gourd", 0x0C66),
    (FoodGroup::FruitsAndVegetables, "Banana", 0x171F),
    (FoodGroup::FruitsAndVegetables, "Lemon", 0x1728),
    (FoodGroup::FruitsAndVegetables, "Lime", 0x172A),
    (FoodGroup::FruitsAndVegetables, "Grape", 0x09D1),
    (FoodGroup::FruitsAndVegetables, "Peach", 0x09D2),
    (FoodGroup::FruitsAndVegetables, "Pear", 0x0994),
    (FoodGroup::FruitsAndVegetables, "Apple", 0x09D0),
    (FoodGroup::FruitsAndVegetables, "Watermelon", 0x0C5C),
    (FoodGroup::FruitsAndVegetables, "Squash", 0x0C72),
    (FoodGroup::FruitsAndVegetables, "Cantaloupe", 0x0C79),
    (FoodGroup::FruitsAndVegetables, "Carrot", 0x0C78),
    (FoodGroup::FruitsAndVegetables, "Cabbage", 0x0C7B),
    (FoodGroup::FruitsAndVegetables, "Onion", 0x0C6D),
    (FoodGroup::FruitsAndVegetables, "Lettuce", 0x0C70),
    (FoodGroup::FruitsAndVegetables, "Pumpkin", 0x0C6A),
    (FoodGroup::Meat, "Bacon", 0x0979),
    (FoodGroup::Meat, "Cooked Bird", 0x09B7),
    (FoodGroup::Meat, "Sausage", 0x09C0),
    (FoodGroup::Meat, "Ham", 0x09C9),
    (FoodGroup::Meat, "Ribs", 0x09F2),
    (FoodGroup::Meat, "Lamb Leg", 0x160A),
    (FoodGroup::Meat, "Chicken Leg", 0x1608),
    (FoodGroup::Meat, "Raw Bird", 0x09B9),
    (FoodGroup::Meat, "Raw Ribs", 0x09F1),
    (FoodGroup::Meat, "Raw Lamb Leg", 0x1609),
    (FoodGroup::Meat, "Raw Chicken Leg", 0x1607),
];

/// The food graphics a feed request may use: one named food, or every food
/// of a group.
pub fn food_graphics(name_or_group: &str) -> Vec<u16> {
    if let Some(group) = FoodGroup::by_name(name_or_group) {
        return FOODS
            .iter()
            .filter(|(g, _, _)| *g == group)
            .map(|&(_, _, graphic)| graphic)
            .collect();
    }
    let key = name_key(name_or_group);
    FOODS
        .iter()
        .filter(|(_, n, _)| name_key(n) == key)
        .map(|&(_, _, graphic)| graphic)
        .collect()
}

/// The item graphics that are magic wands.
pub const WAND_GRAPHICS: [u16; 4] = [0x0DF2, 0x0DF3, 0x0DF4, 0x0DF5];

/// The first and last graphic of a pile of bones a blade can cut.
pub const BONE_PILE_FIRST: u16 = 0x0ECA;
pub const BONE_PILE_LAST: u16 = 0x0ED2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_potion_is_found_whatever_the_player_calls_it() {
        assert_eq!(potion("heal").map(|p| p.graphic), Some(GRAPHIC_POTION_HEAL));
        assert_eq!(potion("Greater Heal Potion"), potion("heal"));
        assert_eq!(potion("night sight").map(|p| p.hue), Some(PLAIN_HUE));
        assert!(potion("elixir of life").is_none());
    }

    /// Night sight and invisibility share a bottle. Only the hue tells them
    /// apart, and drinking the wrong one is not a small mistake.
    #[test]
    fn potions_in_the_same_bottle_differ_by_hue() {
        let night = potion("night sight").expect("night sight");
        let invisible = potion("invisibility").expect("invisibility");
        assert_eq!(night.graphic, invisible.graphic);
        assert_ne!(night.hue, invisible.hue);
    }

    #[test]
    fn a_supply_is_found_by_short_or_full_name() {
        assert_eq!(supply("bp"), supply("Black Pearl"));
        assert_eq!(supply("band").map(|s| s.graphic), Some(GRAPHIC_BANDAGE));
    }

    #[test]
    fn a_food_group_names_all_its_foods() {
        assert_eq!(food_graphics("Fish"), vec![0x097B, 0x097A]);
        assert_eq!(food_graphics("Fruits and Vegetables").len(), 18);
        assert_eq!(food_graphics("apple"), vec![0x09D0]);
        assert!(food_graphics("rocks").is_empty());
    }
}
