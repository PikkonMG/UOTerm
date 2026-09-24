//! The kinds of map tile an agent looks for by a word: water, trees, ore,
//! a forge, an anvil, a loom, an oven, a mill. Each kind is read from the
//! tiledata of the tile: its name, its flags, and the harvest tables a shard
//! gathers from. The tiledata flags are named here too, so a caller can ask
//! for every wet or impassable tile by the flag's name.

use uoterm_assist::harvest::Harvest;
use uoterm_nav::TileFlagSet;

/// A kind of map tile, by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileGroup {
    /// Land or statics that are wet: the sea, lakes, rivers.
    Water,
    /// The trees an axe chops.
    Trees,
    /// The mountain side, the cave floor and the cave rock a pickaxe mines.
    Ore,
    Forge,
    Anvil,
    Loom,
    Oven,
    Mill,
}

/// The words a caller names a kind by, each with its kind.
const GROUP_NAMES: [(&str, TileGroup); 11] = [
    ("water", TileGroup::Water),
    ("trees", TileGroup::Trees),
    ("tree", TileGroup::Trees),
    ("ore", TileGroup::Ore),
    ("rock", TileGroup::Ore),
    ("cave", TileGroup::Ore),
    ("forge", TileGroup::Forge),
    ("anvil", TileGroup::Anvil),
    ("loom", TileGroup::Loom),
    ("oven", TileGroup::Oven),
    ("mill", TileGroup::Mill),
];

/// The word in the tiledata name of water, of a tree, and of each crafting
/// station.
const WATER_WORD: &str = "water";
const TREE_WORD: &str = "tree";
const FORGE_WORD: &str = "forge";
const ANVIL_WORD: &str = "anvil";
const LOOM_WORD: &str = "loom";
const OVEN_WORD: &str = "oven";
const MILL_WORD: &str = "mill";

/// Every tiledata flag by the name a caller writes.
pub const FLAG_NAMES: [(&str, TileFlagSet); 37] = [
    ("background", TileFlagSet::BACKGROUND),
    ("weapon", TileFlagSet::WEAPON),
    ("transparent", TileFlagSet::TRANSPARENT),
    ("translucent", TileFlagSet::TRANSLUCENT),
    ("wall", TileFlagSet::WALL),
    ("damaging", TileFlagSet::DAMAGING),
    ("impassable", TileFlagSet::IMPASSABLE),
    ("wet", TileFlagSet::WET),
    ("surface", TileFlagSet::SURFACE),
    ("bridge", TileFlagSet::BRIDGE),
    ("stackable", TileFlagSet::STACKABLE),
    ("window", TileFlagSet::WINDOW),
    ("no_shoot", TileFlagSet::NO_SHOOT),
    ("article_a", TileFlagSet::ARTICLE_A),
    ("article_an", TileFlagSet::ARTICLE_AN),
    ("internal", TileFlagSet::INTERNAL),
    ("foliage", TileFlagSet::FOLIAGE),
    ("partial_hue", TileFlagSet::PARTIAL_HUE),
    ("no_house", TileFlagSet::NO_HOUSE),
    ("map", TileFlagSet::MAP),
    ("container", TileFlagSet::CONTAINER),
    ("wearable", TileFlagSet::WEARABLE),
    ("light_source", TileFlagSet::LIGHT_SOURCE),
    ("animation", TileFlagSet::ANIMATION),
    ("no_diagonal", TileFlagSet::NO_DIAGONAL),
    ("armor", TileFlagSet::ARMOR),
    ("roof", TileFlagSet::ROOF),
    ("door", TileFlagSet::DOOR),
    ("stair_back", TileFlagSet::STAIR_BACK),
    ("stair_right", TileFlagSet::STAIR_RIGHT),
    ("alpha_blend", TileFlagSet::ALPHA_BLEND),
    ("use_new_art", TileFlagSet::USE_NEW_ART),
    ("art_used", TileFlagSet::ART_USED),
    ("no_shadow", TileFlagSet::NO_SHADOW),
    ("pixel_bleed", TileFlagSet::PIXEL_BLEED),
    ("play_anim_once", TileFlagSet::PLAY_ANIM_ONCE),
    ("multi_movable", TileFlagSet::MULTI_MOVABLE),
];

/// One tile of the map as the kinds read it: a land tile or a static.
#[derive(Clone, Copy, Debug)]
pub struct TileFacts<'a> {
    /// The land id, or the graphic of a static.
    pub id: u16,
    pub land: bool,
    /// The tiledata name.
    pub name: &'a str,
    pub flags: TileFlagSet,
}

impl TileFacts<'_> {
    /// True when the tiledata name holds `word`, in any case.
    pub fn named(&self, word: &str) -> bool {
        self.name.to_ascii_lowercase().contains(word)
    }
}

impl TileGroup {
    /// The kind a word names, in any case.
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        GROUP_NAMES
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, group)| *group)
    }

    /// The names of the kinds, one for each.
    pub fn names() -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        for (name, group) in GROUP_NAMES {
            if !GROUP_NAMES
                .iter()
                .take_while(|(earlier, _)| *earlier != name)
                .any(|(_, seen)| *seen == group)
            {
                names.push(name);
            }
        }
        names
    }

    /// True when the tile belongs to this kind.
    pub fn holds(self, tile: &TileFacts<'_>) -> bool {
        match self {
            Self::Water => tile.flags.contains(TileFlagSet::WET) || tile.named(WATER_WORD),
            Self::Trees => {
                !tile.land && (Harvest::Lumber.from_static(tile.id) || tile.named(TREE_WORD))
            }
            Self::Ore => {
                if tile.land {
                    Harvest::Ore.from_land(tile.id)
                } else {
                    Harvest::Ore.from_static(tile.id)
                }
            }
            Self::Forge => !tile.land && tile.named(FORGE_WORD),
            Self::Anvil => !tile.land && tile.named(ANVIL_WORD),
            Self::Loom => !tile.land && tile.named(LOOM_WORD),
            Self::Oven => !tile.land && tile.named(OVEN_WORD),
            Self::Mill => !tile.land && tile.named(MILL_WORD),
        }
    }
}

/// The flag a name names, in any case.
pub fn flag_from_name(name: &str) -> Option<TileFlagSet> {
    let name = name.trim().to_ascii_lowercase();
    FLAG_NAMES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, flag)| *flag)
}

/// The names of every flag set in `flags`.
pub fn flag_names(flags: TileFlagSet) -> Vec<&'static str> {
    FLAG_NAMES
        .iter()
        .filter(|(_, flag)| flags.contains(*flag))
        .map(|(name, _)| *name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_TREE: u16 = 0x0CCA;
    const CAVE_FLOOR: u16 = 1741;
    const AN_ANVIL: u16 = 0x0FAF;

    fn item<'a>(id: u16, name: &'a str, flags: TileFlagSet) -> TileFacts<'a> {
        TileFacts {
            id,
            land: false,
            name,
            flags,
        }
    }

    fn land<'a>(id: u16, name: &'a str, flags: TileFlagSet) -> TileFacts<'a> {
        TileFacts {
            id,
            land: true,
            name,
            flags,
        }
    }

    #[test]
    fn each_kind_knows_its_tiles() {
        assert!(TileGroup::Water.holds(&land(0x00A8, "water", TileFlagSet::WET)));
        assert!(TileGroup::Water.holds(&land(0x00A8, "", TileFlagSet::WET)));
        assert!(!TileGroup::Water.holds(&land(0x0003, "grass", TileFlagSet::NONE)));
        assert!(TileGroup::Trees.holds(&item(A_TREE, "", TileFlagSet::NONE)));
        assert!(TileGroup::Trees.holds(&item(0x0001, "cedar tree", TileFlagSet::NONE)));
        assert!(!TileGroup::Trees.holds(&land(0x0001, "tree", TileFlagSet::NONE)));
        assert!(TileGroup::Ore.holds(&land(CAVE_FLOOR, "cave floor", TileFlagSet::NONE)));
        assert!(!TileGroup::Ore.holds(&item(CAVE_FLOOR, "", TileFlagSet::NONE)));
        assert!(TileGroup::Anvil.holds(&item(AN_ANVIL, "anvil", TileFlagSet::NONE)));
        assert!(TileGroup::Forge.holds(&item(0x0FB1, "small Forge", TileFlagSet::NONE)));
        assert!(!TileGroup::Loom.holds(&item(AN_ANVIL, "anvil", TileFlagSet::NONE)));
    }

    #[test]
    fn kinds_and_flags_are_read_by_name() {
        assert_eq!(TileGroup::from_name("Rock"), Some(TileGroup::Ore));
        assert_eq!(TileGroup::from_name("tree"), Some(TileGroup::Trees));
        assert_eq!(TileGroup::from_name("castle"), None);
        let names = TileGroup::names();
        assert_eq!(
            names,
            vec!["water", "trees", "ore", "forge", "anvil", "loom", "oven", "mill"]
        );
        assert_eq!(flag_from_name("Wet"), Some(TileFlagSet::WET));
        assert_eq!(flag_from_name("no_shoot"), Some(TileFlagSet::NO_SHOOT));
        assert_eq!(flag_from_name("sticky"), None);
        assert_eq!(
            flag_names(TileFlagSet::WET | TileFlagSet::IMPASSABLE),
            vec!["impassable", "wet"]
        );
    }
}
