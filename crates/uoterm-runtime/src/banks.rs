//! The banks of the standard maps: where a banker stands in each town.
//!
//! A shard that keeps the standard towns keeps their bankers where these
//! spots say, on every map. The spots are the banker spawners of the
//! standard spawn data, for the maps after the Mondain's Legacy era.
//! A shard with its own towns has other banks, and the agent finds those
//! by their bankers.

use uoterm_protocol::Point3;

const FELUCCA: u8 = 0;
const TRAMMEL: u8 = 1;
const ILSHENAR: u8 = 2;
const MALAS: u8 = 3;
const TOKUNO: u8 = 4;
const TER_MUR: u8 = 5;

/// The farthest the client sends a character to a bank on foot. Past this
/// it is another town, an island or another map, and the one search that
/// finds no way costs seconds.
pub const BANK_WALK_RANGE: u32 = 400;

/// One bank: its map, where its banker stands, and its town. The town is
/// blank where no town region covers the bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bank {
    pub map: u8,
    pub at: Point3,
    pub town: &'static str,
}

/// Every bank of the standard maps.
pub const BANKS: [Bank; 47] = [
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 587,
            y: 2146,
            z: 0,
        },
        town: "Skara Brae",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 652,
            y: 820,
            z: 0,
        },
        town: "Yew",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 1317,
            y: 3773,
            z: 0,
        },
        town: "Jhelom",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 1425,
            y: 1690,
            z: 0,
        },
        town: "Britain",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 1650,
            y: 1608,
            z: 20,
        },
        town: "Britain",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 1813,
            y: 2825,
            z: 0,
        },
        town: "Trinsic",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 1897,
            y: 2684,
            z: 10,
        },
        town: "Trinsic",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 2503,
            y: 552,
            z: 0,
        },
        town: "Minoc",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 2731,
            y: 2192,
            z: 0,
        },
        town: "Buccaneer's Den",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 2880,
            y: 3472,
            z: 15,
        },
        town: "Serpent's Hold",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 2881,
            y: 684,
            z: 0,
        },
        town: "Vesper",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 3695,
            y: 2511,
            z: 0,
        },
        town: "Ocllo",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 3734,
            y: 2149,
            z: 20,
        },
        town: "Magincia",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 3764,
            y: 1317,
            z: 0,
        },
        town: "Nujel'm",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 4471,
            y: 1156,
            z: 0,
        },
        town: "Moonglow",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 4551,
            y: 2323,
            z: -2,
        },
        town: "",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 5275,
            y: 3977,
            z: 37,
        },
        town: "Delucia",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 5347,
            y: 76,
            z: 25,
        },
        town: "Wind",
    },
    Bank {
        map: FELUCCA,
        at: Point3 {
            x: 5669,
            y: 3131,
            z: 14,
        },
        town: "Papua",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 587,
            y: 2146,
            z: 0,
        },
        town: "Skara Brae",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 652,
            y: 820,
            z: 0,
        },
        town: "Yew",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 1317,
            y: 3773,
            z: 0,
        },
        town: "Jhelom",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 1425,
            y: 1690,
            z: 0,
        },
        town: "Britain",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 1650,
            y: 1608,
            z: 20,
        },
        town: "Britain",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 1813,
            y: 2825,
            z: 0,
        },
        town: "Trinsic",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 1897,
            y: 2684,
            z: 10,
        },
        town: "Trinsic",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 2503,
            y: 552,
            z: 0,
        },
        town: "Minoc",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 2731,
            y: 2192,
            z: 0,
        },
        town: "Buccaneer's Den",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 2880,
            y: 3472,
            z: 15,
        },
        town: "Serpent's Hold",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 2881,
            y: 684,
            z: 0,
        },
        town: "Vesper",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 3484,
            y: 2570,
            z: 20,
        },
        town: "New Haven",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 3484,
            y: 2576,
            z: 20,
        },
        town: "New Haven",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 3734,
            y: 2149,
            z: 20,
        },
        town: "Magincia",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 3764,
            y: 1317,
            z: 0,
        },
        town: "Nujel'm",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 4471,
            y: 1156,
            z: 0,
        },
        town: "Moonglow",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 4551,
            y: 2323,
            z: -2,
        },
        town: "",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 5275,
            y: 3977,
            z: 37,
        },
        town: "Delucia",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 5347,
            y: 76,
            z: 25,
        },
        town: "Wind",
    },
    Bank {
        map: TRAMMEL,
        at: Point3 {
            x: 5669,
            y: 3131,
            z: 14,
        },
        town: "Papua",
    },
    Bank {
        map: ILSHENAR,
        at: Point3 {
            x: 854,
            y: 680,
            z: -40,
        },
        town: "Gargoyle City",
    },
    Bank {
        map: ILSHENAR,
        at: Point3 {
            x: 855,
            y: 603,
            z: -40,
        },
        town: "Gargoyle City",
    },
    Bank {
        map: ILSHENAR,
        at: Point3 {
            x: 1610,
            y: 556,
            z: -19,
        },
        town: "",
    },
    Bank {
        map: MALAS,
        at: Point3 {
            x: 989,
            y: 520,
            z: -50,
        },
        town: "Luna",
    },
    Bank {
        map: MALAS,
        at: Point3 {
            x: 2048,
            y: 1343,
            z: -85,
        },
        town: "Umbra",
    },
    Bank {
        map: TOKUNO,
        at: Point3 {
            x: 729,
            y: 1249,
            z: 25,
        },
        town: "Zento",
    },
    Bank {
        map: TER_MUR,
        at: Point3 {
            x: 833,
            y: 3439,
            z: -20,
        },
        town: "Royal City",
    },
    Bank {
        map: TER_MUR,
        at: Point3 {
            x: 843,
            y: 3439,
            z: -20,
        },
        town: "Royal City",
    },
];

/// The nearest bank on `map` that a character at `at` can walk to, when
/// one is in [`BANK_WALK_RANGE`].
pub fn nearest_bank(map: u8, at: Point3) -> Option<&'static Bank> {
    BANKS
        .iter()
        .filter(|bank| bank.map == map)
        .map(|bank| (bank, at.chebyshev(bank.at)))
        .filter(|&(_, dist)| dist <= BANK_WALK_RANGE)
        .min_by_key(|&(_, dist)| dist)
        .map(|(bank, _)| bank)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOONGLOW_GATE: Point3 = Point3 {
        x: 4467,
        y: 1283,
        z: 0,
    };

    #[test]
    fn the_nearest_bank_is_in_the_town_she_stands_in() {
        let bank = nearest_bank(TRAMMEL, MOONGLOW_GATE).expect("a bank in Moonglow");
        assert_eq!(bank.town, "Moonglow");
        let britain = nearest_bank(FELUCCA, Point3::new(1430, 1700, 0)).expect("a bank");
        assert_eq!(britain.town, "Britain");
    }

    #[test]
    fn a_bank_on_another_map_or_too_far_is_none() {
        const OPEN_SEA: Point3 = Point3 {
            x: 2000,
            y: 3900,
            z: 0,
        };
        assert_eq!(nearest_bank(ILSHENAR, MOONGLOW_GATE), None);
        assert_eq!(nearest_bank(FELUCCA, OPEN_SEA), None);
    }

    #[test]
    fn every_map_has_a_bank() {
        for map in [FELUCCA, TRAMMEL, ILSHENAR, MALAS, TOKUNO, TER_MUR] {
            assert!(BANKS.iter().any(|b| b.map == map), "map {map}");
        }
    }
}
