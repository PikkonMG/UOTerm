//! The art a client swaps with the season. In winter the grass shows as
//! snow, and in autumn the green trees show as brown ones. A shard sends
//! only the season; each client swaps the art itself.
//!
//! The table ships with UOTerm. A shard with its own art can replace it:
//! put a `seasons.txt` of the same shape in the UOTerm config folder.

use std::collections::HashMap;
use std::path::Path;

pub const SEASONS_NAME: &str = "seasons.txt";
const BUILT_IN: &str = include_str!("../assets/seasons.txt");
const FIELDS: usize = 4;
const STATIC_WORD: &str = "static";
const COMMENT_MARKS: [&str; 2] = ["#", "//"];

/// The seasons the shard names, in the order of its numbers.
const SEASON_WORDS: [&str; 5] = ["spring", "summer", "fall", "winter", "desolation"];
/// Every season of a shard that is none of the above shows as summer.
const SUMMER: usize = 1;

#[derive(Default)]
struct Swaps {
    land: HashMap<u16, u16>,
    statics: HashMap<u16, u16>,
}

pub struct SeasonArt {
    /// One set of swaps for each season.
    seasons: Vec<Swaps>,
}

fn number(word: &str) -> Option<u16> {
    let word = word.trim();
    match word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")) {
        Some(hex) => u16::from_str_radix(hex, 16).ok(),
        None => word.parse().ok(),
    }
}

impl Default for SeasonArt {
    fn default() -> Self {
        Self::parse(BUILT_IN)
    }
}

impl SeasonArt {
    /// The table of the config folder, or the one that ships with UOTerm.
    pub fn open(config_dir: &Path) -> Self {
        std::fs::read_to_string(config_dir.join(SEASONS_NAME))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    fn parse(text: &str) -> Self {
        let mut seasons: Vec<Swaps> = (0..SEASON_WORDS.len()).map(|_| Swaps::default()).collect();
        for line in text.lines().map(str::trim) {
            if COMMENT_MARKS.iter().any(|mark| line.starts_with(mark)) {
                continue;
            }
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() < FIELDS {
                continue;
            }
            let season = SEASON_WORDS
                .iter()
                .position(|word| fields[0].trim().eq_ignore_ascii_case(word));
            let (Some(season), Some(from), Some(to)) =
                (season, number(fields[2]), number(fields[3]))
            else {
                continue;
            };
            let swaps = &mut seasons[season];
            let kind = if fields[1].trim().eq_ignore_ascii_case(STATIC_WORD) {
                &mut swaps.statics
            } else {
                &mut swaps.land
            };
            kind.insert(from, to);
        }
        Self { seasons }
    }

    fn swaps(&self, season: u8) -> &Swaps {
        let season = usize::from(season);
        &self.seasons[if season < self.seasons.len() {
            season
        } else {
            SUMMER
        }]
    }

    /// The land tile that shows in this season.
    pub fn land(&self, season: u8, land_id: u16) -> u16 {
        self.swaps(season)
            .land
            .get(&land_id)
            .copied()
            .unwrap_or(land_id)
    }

    /// The item or static that shows in this season.
    pub fn item(&self, season: u8, graphic: u16) -> u16 {
        self.swaps(season)
            .statics
            .get(&graphic)
            .copied()
            .unwrap_or(graphic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPRING: u8 = 0;
    const SUMMER_SEASON: u8 = 1;
    const FALL: u8 = 2;
    const WINTER: u8 = 3;

    #[test]
    fn the_table_that_ships_swaps_grass_in_winter_and_leaves_in_autumn() {
        const GRASS: u16 = 196;
        const SNOW: u16 = 282;
        const GREEN_TREE: u16 = 0x0CD1;
        let art = SeasonArt::default();
        assert_eq!(art.land(WINTER, GRASS), SNOW);
        assert_eq!(art.land(SUMMER_SEASON, GRASS), GRASS);
        assert_eq!(art.item(FALL, GREEN_TREE), 0x0CD2);
        assert_eq!(art.item(WINTER, GREEN_TREE), GREEN_TREE);
    }

    #[test]
    fn a_shard_can_give_its_own_table_and_a_bad_line_is_left_out() {
        let art = SeasonArt::parse(
            "# a comment\n\
             winter,landtile,3,0x0FA\n\
             spring,static,0x0C84,900\n\
             winter,landtile\n\
             nosuchseason,static,1,2\n\
             fall,static,notanumber,2\n",
        );
        assert_eq!(art.land(WINTER, 3), 0xFA);
        assert_eq!(art.item(SPRING, 0x0C84), 900);
        assert_eq!(art.item(FALL, 1), 1, "the bad lines changed nothing");
    }

    #[test]
    fn a_season_the_shard_invents_shows_as_summer() {
        const GRASS: u16 = 196;
        let art = SeasonArt::default();
        assert_eq!(art.land(u8::MAX, GRASS), art.land(SUMMER_SEASON, GRASS));
    }
}
