//! What a shard lets a player gather, and with what.
//!
//! The lists are the ones both server families harvest from: the static trees
//! an axe chops, the land and the static rock a pickaxe or a shovel mines, and
//! the tools that do each. A shard answers a swing at anything else with "you
//! can't use that on this".

use serde::{Deserialize, Serialize};

/// Something a player gathers from the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Harvest {
    /// Logs, from a tree, with an axe.
    Lumber,
    /// Ore, from the mountain side or the floor of a cave, with a pickaxe or a
    /// shovel.
    Ore,
}

/// How near the spot a player must stand: two tiles, for both.
pub const HARVEST_REACH: u16 = 2;

/// The static trees an axe chops.
pub const TREE_STATICS: &[u16] = &[
    0x0CCA, 0x0CCB, 0x0CCC, 0x0CCD, 0x0CD0, 0x0CD3, 0x0CD6, 0x0CD8, 0x0CDA, 0x0CDD, 0x0CE0, 0x0CE3,
    0x0CE6, 0x0CF8, 0x0CFB, 0x0CFE, 0x0D01, 0x0D41, 0x0D42, 0x0D43, 0x0D44, 0x0D57, 0x0D58, 0x0D59,
    0x0D5A, 0x0D5B, 0x0D6E, 0x0D6F, 0x0D70, 0x0D71, 0x0D72, 0x0D84, 0x0D85, 0x0D86, 0x12B5, 0x12B6,
    0x12B7, 0x12B8, 0x12B9, 0x12BA, 0x12BB, 0x12BC, 0x12BD, 0x0CCE, 0x0CCF, 0x0CD1, 0x0CD2, 0x0CD4,
    0x0CD5, 0x0CD7, 0x0CD9, 0x0CDB, 0x0CDC, 0x0CDE, 0x0CDF, 0x0CE1, 0x0CE2, 0x0CE4, 0x0CE5, 0x0CE7,
    0x0CE8, 0x0CF9, 0x0CFA, 0x0CFC, 0x0CFD, 0x0CFF, 0x0D00, 0x0D02, 0x0D03, 0x0D45, 0x0D46, 0x0D47,
    0x0D48, 0x0D49, 0x0D4A, 0x0D4B, 0x0D4C, 0x0D4D, 0x0D4E, 0x0D4F, 0x0D50, 0x0D51, 0x0D52, 0x0D53,
    0x0D5C, 0x0D5D, 0x0D5E, 0x0D5F, 0x0D60, 0x0D61, 0x0D62, 0x0D63, 0x0D64, 0x0D65, 0x0D66, 0x0D67,
    0x0D68, 0x0D69, 0x0D73, 0x0D74, 0x0D75, 0x0D76, 0x0D77, 0x0D78, 0x0D79, 0x0D7A, 0x0D7B, 0x0D7C,
    0x0D7D, 0x0D7E, 0x0D7F, 0x0D87, 0x0D88, 0x0D89, 0x0D8A, 0x0D8B, 0x0D8C, 0x0D8D, 0x0D8E, 0x0D8F,
    0x0D90, 0x0D95, 0x0D96, 0x0D97, 0x0D99, 0x0D9A, 0x0D9B, 0x0D9D, 0x0D9E, 0x0D9F, 0x0DA1, 0x0DA2,
    0x0DA3, 0x0DA5, 0x0DA6, 0x0DA7, 0x0DA9, 0x0DAA, 0x0DAB, 0x12BE, 0x12BF, 0x12C0, 0x12C1, 0x12C2,
    0x12C3, 0x12C4, 0x12C5, 0x12C6, 0x12C7,
];

/// The land of the mountain side and of cave floors, which a pickaxe mines.
pub const MINE_LAND: &[u16] = &[
    220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 236, 237, 238, 239, 240, 241, 242,
    243, 244, 245, 246, 247, 252, 253, 254, 255, 256, 257, 258, 259, 260, 261, 262, 263, 268, 269,
    270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 286, 287, 288, 289, 290, 291, 292, 293, 294,
    296, 297, 321, 322, 323, 324, 467, 468, 469, 470, 471, 472, 473, 474, 476, 477, 478, 479, 480,
    481, 482, 483, 484, 485, 486, 487, 492, 493, 494, 495, 543, 544, 545, 546, 547, 548, 549, 550,
    551, 552, 553, 554, 555, 556, 557, 558, 559, 560, 561, 562, 563, 564, 565, 566, 567, 568, 569,
    570, 571, 572, 573, 574, 575, 576, 577, 578, 579, 581, 582, 583, 584, 585, 586, 587, 588, 589,
    590, 591, 592, 593, 594, 595, 596, 597, 598, 599, 600, 601, 610, 611, 612, 613, 1010, 1741,
    1742, 1743, 1744, 1745, 1746, 1747, 1748, 1749, 1750, 1751, 1752, 1753, 1754, 1755, 1756, 1757,
    1771, 1772, 1773, 1774, 1775, 1776, 1777, 1778, 1779, 1780, 1781, 1782, 1783, 1784, 1785, 1786,
    1787, 1788, 1789, 1790, 1801, 1802, 1803, 1804, 1805, 1806, 1807, 1808, 1809, 1811, 1812, 1813,
    1814, 1815, 1816, 1817, 1818, 1819, 1820, 1821, 1822, 1823, 1824, 1831, 1832, 1833, 1834, 1835,
    1836, 1837, 1838, 1839, 1840, 1841, 1842, 1843, 1844, 1845, 1846, 1847, 1848, 1849, 1850, 1851,
    1852, 1853, 1854, 1861, 1862, 1863, 1864, 1865, 1866, 1867, 1868, 1869, 1870, 1871, 1872, 1873,
    1874, 1875, 1876, 1877, 1878, 1879, 1880, 1881, 1882, 1883, 1884, 1981, 1982, 1983, 1984, 1985,
    1986, 1987, 1988, 1989, 1990, 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001,
    2002, 2003, 2004, 2028, 2029, 2030, 2031, 2032, 2033, 2100, 2101, 2102, 2103, 2104, 2105,
];

/// The static rock of a cave, which a pickaxe mines as it mines the land.
pub const MINE_STATICS: &[u16] = &[
    0x053B, 0x053C, 0x053D, 0x053E, 0x053F, 0x0540, 0x0541, 0x0542, 0x0543, 0x0544, 0x0545, 0x0546,
    0x0547, 0x0548, 0x0549, 0x054A, 0x054B, 0x054C, 0x054D, 0x054E, 0x054F,
];

/// Every axe, both ways each one lies. Any axe chops.
pub const AXES: &[u16] = &[
    0x0F43, 0x0F44, 0x0F45, 0x0F46, 0x0F47, 0x0F48, 0x0F49, 0x0F4A, 0x0F4B, 0x0F4C, 0x13AF, 0x13B0,
    0x13FA, 0x13FB, 0x1442, 0x1443, 0x2D28, 0x2D34, 0x48B0, 0x48B1, 0x48B2, 0x48B3,
];

/// The pickaxes and the shovels, both ways each one lies.
pub const MINING_TOOLS: &[u16] = &[0x0E85, 0x0E86, 0x0F39, 0x0F3A];

/// How long one swing takes before the next is worth making, in
/// milliseconds. A shard plays each swing as effects 1.6 seconds apart: an
/// axe makes up to three of them on an older shard, a pickaxe one.
const LUMBER_SWING_MS: u64 = 5_000;
const ORE_SWING_MS: u64 = 2_000;

/// The line numbers a shard answers a swing at a spot with nothing left.
pub const CLILOC_NO_WOOD: u32 = 500_493;
pub const CLILOC_NO_METAL: u32 = 503_040;

impl Harvest {
    /// The goal name a persona and an agent give it.
    pub fn from_goal(name: &str) -> Option<Self> {
        match name {
            "gather" | "chop" | "lumber" => Some(Self::Lumber),
            "mine" => Some(Self::Ore),
            _ => None,
        }
    }

    /// The goal name it is shown under.
    pub fn goal_name(self) -> &'static str {
        match self {
            Self::Lumber => "gather",
            Self::Ore => "mine",
        }
    }

    /// The graphics of every tool that gathers it.
    pub fn tools(self) -> &'static [u16] {
        match self {
            Self::Lumber => AXES,
            Self::Ore => MINING_TOOLS,
        }
    }

    /// True when a static of this graphic is a spot to gather from.
    pub fn from_static(self, graphic: u16) -> bool {
        match self {
            Self::Lumber => TREE_STATICS.contains(&graphic),
            Self::Ore => MINE_STATICS.contains(&graphic),
        }
    }

    /// True when land of this id is a spot to gather from.
    pub fn from_land(self, land: u16) -> bool {
        match self {
            Self::Lumber => false,
            Self::Ore => MINE_LAND.contains(&land),
        }
    }

    pub fn swing_ms(self) -> u64 {
        match self {
            Self::Lumber => LUMBER_SWING_MS,
            Self::Ore => ORE_SWING_MS,
        }
    }

    /// The line number a shard says when a spot has nothing left of it.
    pub fn exhausted_cliloc(self) -> u32 {
        match self {
            Self::Lumber => CLILOC_NO_WOOD,
            Self::Ore => CLILOC_NO_METAL,
        }
    }

    /// Words that say the character carries nothing to gather it with.
    pub fn no_tool(self) -> &'static str {
        match self {
            Self::Lumber => "no axe to chop with",
            Self::Ore => "no pickaxe or shovel to mine with",
        }
    }

    /// Words that say nothing to gather is in reach.
    pub fn nothing_near(self) -> &'static str {
        match self {
            Self::Lumber => "no tree near to chop",
            Self::Ore => "no rock near to mine",
        }
    }

    /// Words that say the character cannot gather it the way he is.
    pub fn refused_while_mounted(self) -> Option<&'static str> {
        match self {
            Self::Lumber => None,
            Self::Ore => Some("a character cannot dig while riding"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_its_own_spots_and_tools() {
        const A_TREE: u16 = 0x0CCA;
        const CAVE_FLOOR: u16 = 1741;
        const CAVE_ROCK: u16 = 0x053B;
        const HATCHET: u16 = 0x0F43;
        const PICKAXE: u16 = 0x0E86;
        assert!(Harvest::Lumber.from_static(A_TREE));
        assert!(!Harvest::Ore.from_static(A_TREE));
        assert!(Harvest::Ore.from_land(CAVE_FLOOR));
        assert!(Harvest::Ore.from_static(CAVE_ROCK));
        assert!(!Harvest::Lumber.from_land(CAVE_FLOOR));
        assert!(Harvest::Lumber.tools().contains(&HATCHET));
        assert!(Harvest::Ore.tools().contains(&PICKAXE));
        assert_eq!(Harvest::from_goal("mine"), Some(Harvest::Ore));
        assert_eq!(Harvest::from_goal("chop"), Some(Harvest::Lumber));
        assert_eq!(Harvest::from_goal("gather"), Some(Harvest::Lumber));
        assert_eq!(Harvest::from_goal("hunt"), None);
    }
}
