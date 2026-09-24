//! The looks a character may pick when the shard lets him change race: the
//! hair and beard styles and the skin and hair hues of each race, as the
//! reference client offers them in its race change window.

use serde::{Deserialize, Serialize};
use uoterm_protocol::NewLooks;

/// The race numbers of the shard, counted from one.
const RACE_HUMAN: u8 = 1;
const RACE_ELF: u8 = 2;
const RACE_GARGOYLE: u8 = 3;

/// A hue of a palette is one below the hue the window shows and the answer
/// carries, as the reference client reads its palettes.
const PALETTE_HUE_STEP: u16 = 1;
/// A palette shows as a grid of this many rows. The hues past the last
/// whole column are not offered.
pub const PALETTE_ROWS: usize = 8;

/// A race a character may change to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Race {
    Human,
    Elf,
    Gargoyle,
}

impl Race {
    /// The race of a shard race number, or None for a number past them.
    pub fn from_number(number: u8) -> Option<Self> {
        match number {
            RACE_HUMAN => Some(Self::Human),
            RACE_ELF => Some(Self::Elf),
            RACE_GARGOYLE => Some(Self::Gargoyle),
            _ => None,
        }
    }

    /// The shard race number.
    pub fn number(self) -> u8 {
        match self {
            Self::Human => RACE_HUMAN,
            Self::Elf => RACE_ELF,
            Self::Gargoyle => RACE_GARGOYLE,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Elf => "elf",
            Self::Gargoyle => "gargoyle",
        }
    }
}

/// One style of hair or beard: the text number of its name, its English
/// name and its item graphic. A graphic of zero is none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub name: u32,
    pub words: &'static str,
    pub graphic: u16,
}

const fn style(name: u32, words: &'static str, graphic: u16) -> Style {
    Style {
        name,
        words,
        graphic,
    }
}

/// The first style of each list.
const NO_STYLE: Style = style(3_000_340, "None", 0);

const HUMAN_HAIR: [Style; 10] = [
    NO_STYLE,
    style(3_000_341, "Short", 0x203B),
    style(3_000_342, "Long", 0x203C),
    style(3_000_343, "Ponytail", 0x203D),
    style(3_000_344, "Mohawk", 0x2044),
    style(3_000_345, "Pageboy", 0x2045),
    style(3_000_346, "Topknot", 0x204A),
    style(3_000_347, "Curly", 0x2047),
    style(3_000_348, "Receding", 0x2048),
    style(3_000_349, "Pigtails", 0x2049),
];
const HUMAN_FEMALE_HAIR: [Style; 10] = [
    NO_STYLE,
    style(3_000_341, "Short", 0x203B),
    style(3_000_342, "Long", 0x203C),
    style(3_000_343, "Ponytail", 0x203D),
    style(3_000_344, "Mohawk", 0x2044),
    style(3_000_345, "Pageboy", 0x2045),
    style(3_000_346, "Topknot", 0x204A),
    style(3_000_347, "Curly", 0x2047),
    style(3_000_349, "Pigtails", 0x2049),
    style(3_000_350, "Buns", 0x2046),
];
const HUMAN_BEARD: [Style; 8] = [
    NO_STYLE,
    style(3_000_351, "Goatee", 0x2040),
    style(3_000_352, "Long Beard", 0x203E),
    style(3_000_353, "Short Beard", 0x203F),
    style(3_000_354, "Mustache", 0x2041),
    style(1_011_060, "Short beard", 0x204B),
    style(1_011_061, "Long beard", 0x204C),
    style(3_000_357, "Vandyke", 0x204D),
];
const ELF_HAIR: [Style; 9] = [
    NO_STYLE,
    style(1_074_385, "Mid Long", 0x2FBF),
    style(1_074_386, "Long Feather", 0x2FC0),
    style(1_074_387, "Short", 0x2FC1),
    style(1_074_388, "Mullet", 0x2FC2),
    style(1_074_390, "Long", 0x2FCD),
    style(1_074_391, "Topknot", 0x2FCE),
    style(1_074_392, "Long Braid", 0x2FCF),
    style(1_074_394, "Spiked", 0x2FD1),
];
const ELF_FEMALE_HAIR: [Style; 9] = [
    NO_STYLE,
    style(1_074_386, "Long Feather", 0x2FC0),
    style(1_074_387, "Short", 0x2FC1),
    style(1_074_388, "Mullet", 0x2FC2),
    style(1_074_389, "Flower", 0x2FCC),
    style(1_074_391, "Topknot", 0x2FCE),
    style(1_074_392, "Long Braid", 0x2FCF),
    style(1_074_393, "Buns", 0x2FD0),
    style(1_074_394, "Spiked", 0x2FD1),
];
const GARGOYLE_HAIR: [Style; 9] = [
    NO_STYLE,
    style(1_112_310, "Horn Style 1", 0x4258),
    style(1_112_311, "Horn Style 2", 0x4259),
    style(1_112_312, "Horn Style 3", 0x425A),
    style(1_112_313, "Horn Style 4", 0x425B),
    style(1_112_314, "Horn Style 5", 0x425C),
    style(1_112_315, "Horn Style 6", 0x425D),
    style(1_112_316, "Horn Style 7", 0x425E),
    style(1_112_317, "Horn Style 8", 0x425F),
];
const GARGOYLE_FEMALE_HAIR: [Style; 9] = [
    NO_STYLE,
    style(1_112_310, "Horn Style 1", 0x4261),
    style(1_112_311, "Horn Style 2", 0x4262),
    style(1_112_312, "Horn Style 3", 0x4273),
    style(1_112_313, "Horn Style 4", 0x4274),
    style(1_112_314, "Horn Style 5", 0x4275),
    style(1_112_315, "Horn Style 6", 0x42AA),
    style(1_112_316, "Horn Style 7", 0x42AB),
    style(1_112_317, "Horn Style 8", 0x42B1),
];
const GARGOYLE_BEARD: [Style; 5] = [
    NO_STYLE,
    style(1_112_310, "Horn Style 1", 0x42AD),
    style(1_112_311, "Horn Style 2", 0x42AE),
    style(1_112_312, "Horn Style 3", 0x42AF),
    style(1_112_313, "Horn Style 4", 0x42B0),
];

const HUMAN_SKIN: [u16; 64] = [
    0x03E9, 0x03F1, 0x03F9, 0x0401, 0x0409, 0x0411, 0x0419, 0x0421, 0x03EA, 0x03F2, 0x03FA, 0x0402,
    0x040A, 0x0412, 0x041A, 0x0421, 0x03EB, 0x03F3, 0x03FB, 0x0403, 0x040B, 0x0413, 0x041B, 0x0421,
    0x03EC, 0x03F4, 0x03FC, 0x0404, 0x040C, 0x0414, 0x041C, 0x0421, 0x03ED, 0x03F5, 0x03FD, 0x0405,
    0x040D, 0x0415, 0x041D, 0x0421, 0x03EE, 0x03F6, 0x03FE, 0x0406, 0x040E, 0x0416, 0x041E, 0x0421,
    0x03EF, 0x03F7, 0x03FF, 0x0407, 0x040F, 0x0417, 0x041F, 0x0421, 0x03F0, 0x03F8, 0x0400, 0x0408,
    0x0410, 0x0418, 0x0420, 0x0421,
];
const ELF_SKIN: [u16; 32] = [
    0x04DD, 0x076B, 0x0834, 0x042F, 0x024C, 0x024D, 0x024E, 0x00BE, 0x04A6, 0x0360, 0x0374, 0x0366,
    0x03E7, 0x03DD, 0x0352, 0x0902, 0x076C, 0x0383, 0x0578, 0x03E8, 0x0373, 0x0388, 0x0384, 0x0375,
    0x053E, 0x0380, 0x0381, 0x0382, 0x076A, 0x03E4, 0x051C, 0x03E5,
];
const GARGOYLE_SKIN: [u16; 28] = [
    0x06DA, 0x06DB, 0x06DC, 0x06DD, 0x06DE, 0x06DF, 0x06E0, 0x06E1, 0x06E2, 0x06E3, 0x06E4, 0x06E5,
    0x06E6, 0x06E7, 0x06E8, 0x06E9, 0x06EA, 0x06EB, 0x06EC, 0x06ED, 0x06EE, 0x06EF, 0x06F0, 0x06F1,
    0x06F2, 0x06DA, 0x06DB, 0x06DC,
];
const HUMAN_HAIR_HUES: [u16; 48] = [
    0x044D, 0x0455, 0x045D, 0x0465, 0x046D, 0x0475, 0x044E, 0x0456, 0x045E, 0x0466, 0x046E, 0x0476,
    0x044F, 0x0457, 0x045F, 0x0467, 0x046F, 0x0477, 0x0450, 0x0458, 0x0460, 0x0468, 0x0470, 0x0478,
    0x0451, 0x0459, 0x0461, 0x0469, 0x0471, 0x0479, 0x0452, 0x045A, 0x0462, 0x046A, 0x0472, 0x047A,
    0x0453, 0x045B, 0x0463, 0x046B, 0x0473, 0x047B, 0x0454, 0x045C, 0x0464, 0x046C, 0x0474, 0x047C,
];
const ELF_HAIR_HUES: [u16; 54] = [
    0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0100, 0x06B7, 0x0206, 0x0210, 0x026B, 0x02C2,
    0x02C8, 0x01E3, 0x0238, 0x0368, 0x059C, 0x0852, 0x008D, 0x008E, 0x008F, 0x0090, 0x0091, 0x0158,
    0x0159, 0x015A, 0x015B, 0x015C, 0x015D, 0x01BC, 0x0724, 0x0057, 0x0127, 0x012E, 0x01F2, 0x0250,
    0x031C, 0x031D, 0x031E, 0x031F, 0x0320, 0x0321, 0x0322, 0x0323, 0x0324, 0x0325, 0x0385, 0x0386,
    0x0387, 0x0388, 0x0389, 0x0385, 0x0386, 0x0387,
];
const GARGOYLE_HAIR_HUES: [u16; 18] = [
    0x0708, 0x070A, 0x070C, 0x070E, 0x0710, 0x0762, 0x0764, 0x0767, 0x076A, 0x06F2, 0x06F0, 0x06EE,
    0x06E3, 0x06E1, 0x06DF, 0x0708, 0x070A, 0x070C,
];

/// The hues a palette offers: the whole columns of its grid, each one
/// step up.
fn offered(palette: &[u16]) -> Vec<u16> {
    let whole = palette.len() / PALETTE_ROWS * PALETTE_ROWS;
    palette[..whole]
        .iter()
        .map(|hue| hue + PALETTE_HUE_STEP)
        .collect()
}

/// The race change the shard asks for: the race and the sex of the
/// character.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceChange {
    pub race: Race,
    pub female: bool,
}

impl RaceChange {
    /// The race and the sex in words, such as `elf, female`.
    pub fn words(self) -> String {
        let sex = if self.female { "female" } else { "male" };
        format!("{}, {sex}", self.race.name())
    }

    pub fn hair_styles(self) -> &'static [Style] {
        match (self.race, self.female) {
            (Race::Human, false) => &HUMAN_HAIR,
            (Race::Human, true) => &HUMAN_FEMALE_HAIR,
            (Race::Elf, false) => &ELF_HAIR,
            (Race::Elf, true) => &ELF_FEMALE_HAIR,
            (Race::Gargoyle, false) => &GARGOYLE_HAIR,
            (Race::Gargoyle, true) => &GARGOYLE_FEMALE_HAIR,
        }
    }

    /// A woman and an elf grow no beard.
    pub fn has_beard(self) -> bool {
        !self.female && self.race != Race::Elf
    }

    /// The beard styles, empty when the character grows no beard.
    pub fn beard_styles(self) -> &'static [Style] {
        if !self.has_beard() {
            return &[];
        }
        match self.race {
            Race::Gargoyle => &GARGOYLE_BEARD,
            Race::Human | Race::Elf => &HUMAN_BEARD,
        }
    }

    pub fn skin_hues(self) -> Vec<u16> {
        offered(match self.race {
            Race::Human => &HUMAN_SKIN,
            Race::Elf => &ELF_SKIN,
            Race::Gargoyle => &GARGOYLE_SKIN,
        })
    }

    /// The hues of the hair, and of the beard too.
    pub fn hair_hues(self) -> Vec<u16> {
        offered(match self.race {
            Race::Human => &HUMAN_HAIR_HUES,
            Race::Elf => &ELF_HAIR_HUES,
            Race::Gargoyle => &GARGOYLE_HAIR_HUES,
        })
    }

    /// The looks the window starts with: no hair, no beard, and the first
    /// hue of each palette. A character with no beard sends none.
    pub fn first_looks(self) -> NewLooks {
        let hair_hue = self.hair_hues()[0];
        NewLooks {
            skin_hue: self.skin_hues()[0],
            hair: NO_STYLE.graphic,
            hair_hue,
            beard: NO_STYLE.graphic,
            beard_hue: if self.has_beard() { hair_hue } else { 0 },
        }
    }
}

/// A style as an agent reads it: its item graphic and its name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleChoice {
    pub graphic: u16,
    pub name: String,
}

/// A race change as an agent reads it: the race and the looks it may pick.
/// The hues are the ones the answer carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceChangeView {
    pub race: Race,
    pub female: bool,
    pub hair_styles: Vec<StyleChoice>,
    /// Empty when the character grows no beard.
    pub beard_styles: Vec<StyleChoice>,
    pub skin_hues: Vec<u16>,
    /// The hues of the hair, and of the beard too.
    pub hair_hues: Vec<u16>,
}

impl From<RaceChange> for RaceChangeView {
    fn from(change: RaceChange) -> Self {
        let choices = |styles: &[Style]| {
            styles
                .iter()
                .map(|style| StyleChoice {
                    graphic: style.graphic,
                    name: style.words.to_string(),
                })
                .collect()
        };
        Self {
            race: change.race,
            female: change.female,
            hair_styles: choices(change.hair_styles()),
            beard_styles: choices(change.beard_styles()),
            skin_hues: change.skin_hues(),
            hair_hues: change.hair_hues(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUMAN_MALE: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };
    const ELF_MALE: RaceChange = RaceChange {
        race: Race::Elf,
        female: false,
    };
    const GARGOYLE_FEMALE: RaceChange = RaceChange {
        race: Race::Gargoyle,
        female: true,
    };

    #[test]
    fn races_are_named_by_the_shard_number_from_one() {
        for race in [Race::Human, Race::Elf, Race::Gargoyle] {
            assert_eq!(Race::from_number(race.number()), Some(race));
        }
        assert_eq!(Race::from_number(0), None);
        assert_eq!(Race::from_number(u8::MAX), None);
        assert_eq!(GARGOYLE_FEMALE.words(), "gargoyle, female");
    }

    /// Only a man of the humans or the gargoyles grows a beard, and every
    /// list starts with none.
    #[test]
    fn each_race_and_sex_has_its_styles() {
        assert!(HUMAN_MALE.has_beard());
        assert!(!ELF_MALE.has_beard());
        assert!(!GARGOYLE_FEMALE.has_beard());
        assert!(ELF_MALE.beard_styles().is_empty());
        assert_eq!(HUMAN_MALE.beard_styles().len(), HUMAN_BEARD.len());
        assert_eq!(GARGOYLE_FEMALE.hair_styles()[1].graphic, 0x4261);
        assert_eq!(ELF_MALE.hair_styles()[0], NO_STYLE);
    }

    /// The palettes offer their whole columns only, each hue one step up,
    /// as the reference client's picker shows them.
    #[test]
    fn a_palette_offers_whole_columns_one_hue_up() {
        assert_eq!(HUMAN_MALE.skin_hues().len(), HUMAN_SKIN.len());
        assert_eq!(HUMAN_MALE.skin_hues()[0], 0x03EA);
        assert_eq!(ELF_MALE.hair_hues().len(), 48);
        assert_eq!(GARGOYLE_FEMALE.skin_hues().len(), 24);
        assert_eq!(GARGOYLE_FEMALE.hair_hues().len(), 16);
    }

    #[test]
    fn the_first_looks_have_no_hair_and_the_first_hues() {
        let looks = HUMAN_MALE.first_looks();
        assert_eq!(
            looks,
            NewLooks {
                skin_hue: 0x03EA,
                hair: 0,
                hair_hue: 0x044E,
                beard: 0,
                beard_hue: 0x044E,
            }
        );
        assert_eq!(ELF_MALE.first_looks().beard_hue, 0);
        let view = RaceChangeView::from(ELF_MALE);
        assert!(view.beard_styles.is_empty());
        assert_eq!(
            view.hair_styles[1],
            StyleChoice {
                graphic: 0x2FBF,
                name: "Mid Long".into()
            }
        );
    }
}
