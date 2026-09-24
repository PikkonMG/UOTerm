//! The race change the shard asks for (`0xBF` `0x2A`), apart from how it
//! draws: the hair and beard styles and the skin, hair and beard hues the
//! player picks, as places in their lists, the words that name them, the
//! body the preview shows, and the new looks they send. The classic race
//! change gump and the Modern race change panel pick through these.

use uoterm_protocol::NewLooks;
use uoterm_world::{
    Race, RaceChange, Style, BODY_ELF_FEMALE, BODY_ELF_MALE, BODY_HUMAN_FEMALE, BODY_HUMAN_MALE,
    PALETTE_ROWS,
};

const NO_HUE: u16 = 0;
const NO_STYLE: u16 = 0;

// The words of the labels, with the gargoyle words where they differ: the
// cliloc number and the words without the client files.
pub const WORDS_HAIR_STYLE: (u32, &str) = (3_000_121, "Hair Style");
pub const WORDS_HORN_STYLE: (u32, &str) = (1_112_309, "Horn Style");
pub const WORDS_BEARD_STYLE: (u32, &str) = (3_000_122, "Facial Hair Style");
pub const WORDS_FACIAL_HORN_STYLE: (u32, &str) = (1_112_511, "Facial Horn Style");
pub const WORDS_SKIN: (u32, &str) = (3_000_183, "Skin Tone");
pub const WORDS_HAIR_COLOR: (u32, &str) = (3_000_184, "Hair Color");
pub const WORDS_HORN_COLOR: (u32, &str) = (1_112_322, "Horn Color");
pub const WORDS_BEARD_COLOR: (u32, &str) = (3_000_446, "Facial Hair Color");
pub const WORDS_FACIAL_HORN_COLOR: (u32, &str) = (1_112_512, "Facial Horn Color");

/// What a color of the window paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    Skin,
    Hair,
    Beard,
}

const PAINT_COUNT: usize = 3;

/// What a style list of the window picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StylePart {
    Hair,
    Beard,
}

/// The body the preview shows. The reference client shows a gargoyle on
/// the human body.
pub fn doll_body(change: RaceChange) -> u16 {
    match (change.race, change.female) {
        (Race::Elf, false) => BODY_ELF_MALE,
        (Race::Elf, true) => BODY_ELF_FEMALE,
        (_, false) => BODY_HUMAN_MALE,
        (_, true) => BODY_HUMAN_FEMALE,
    }
}

/// The colors the window offers, top down, with the words of each label.
pub fn paints(change: RaceChange) -> Vec<(Paint, (u32, &'static str))> {
    let gargoyle = change.race == Race::Gargoyle;
    let mut paints = vec![
        (Paint::Skin, WORDS_SKIN),
        (
            Paint::Hair,
            if gargoyle {
                WORDS_HORN_COLOR
            } else {
                WORDS_HAIR_COLOR
            },
        ),
    ];
    if change.has_beard() {
        let words = if gargoyle {
            WORDS_FACIAL_HORN_COLOR
        } else {
            WORDS_BEARD_COLOR
        };
        paints.push((Paint::Beard, words));
    }
    paints
}

/// The style lists the window offers, top down, with the words of each
/// label and the styles.
pub fn style_lists(change: RaceChange) -> Vec<(StylePart, (u32, &'static str), &'static [Style])> {
    let gargoyle = change.race == Race::Gargoyle;
    let (hair_words, beard_words) = if gargoyle {
        (WORDS_HORN_STYLE, WORDS_FACIAL_HORN_STYLE)
    } else {
        (WORDS_HAIR_STYLE, WORDS_BEARD_STYLE)
    };
    let mut lists = vec![(StylePart::Hair, hair_words, change.hair_styles())];
    if change.has_beard() {
        lists.push((StylePart::Beard, beard_words, change.beard_styles()));
    }
    lists
}

/// The columns of a palette of `hue_count` hues, laid in its rows.
pub fn palette_columns(hue_count: usize) -> usize {
    (hue_count / PALETTE_ROWS).max(1)
}

/// The hues one color picks from.
pub fn palette(change: RaceChange, paint: Paint) -> Vec<u16> {
    match paint {
        Paint::Skin => change.skin_hues(),
        Paint::Hair | Paint::Beard => change.hair_hues(),
    }
}

/// The style and the hue picked for each part, as places in their lists,
/// for one request of the shard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RacePicks {
    /// The request the picks were made for. Another request starts over.
    pub change: Option<RaceChange>,
    pub hair: usize,
    pub beard: usize,
    /// The place of the hue in its palette, for each [`Paint`].
    pub hues: [usize; PAINT_COUNT],
}

impl RacePicks {
    /// Follows the request of the shard: another one starts over.
    pub fn follow(&mut self, change: RaceChange) {
        if self.change != Some(change) {
            *self = Self {
                change: Some(change),
                ..Self::default()
            };
        }
    }

    /// The place picked in the palette of a paint.
    pub fn hue_place(&mut self, paint: Paint) -> &mut usize {
        &mut self.hues[paint as usize]
    }

    /// The place picked in a style list.
    pub fn style_place(&mut self, part: StylePart) -> &mut usize {
        match part {
            StylePart::Hair => &mut self.hair,
            StylePart::Beard => &mut self.beard,
        }
    }

    pub fn hue(&self, change: RaceChange, paint: Paint) -> u16 {
        palette(change, paint)
            .get(self.hues[paint as usize])
            .copied()
            .unwrap_or(NO_HUE)
    }

    fn style(styles: &[Style], place: usize) -> u16 {
        styles.get(place).map_or(NO_STYLE, |style| style.graphic)
    }

    /// The looks the player sends. A character with no beard sends none.
    pub fn looks(&self, change: RaceChange) -> NewLooks {
        let bearded = change.has_beard();
        NewLooks {
            skin_hue: self.hue(change, Paint::Skin),
            hair: Self::style(change.hair_styles(), self.hair),
            hair_hue: self.hue(change, Paint::Hair),
            beard: Self::style(change.beard_styles(), self.beard),
            beard_hue: if bearded {
                self.hue(change, Paint::Beard)
            } else {
                NO_HUE
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUMAN_MAN: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };
    const ELF_WOMAN: RaceChange = RaceChange {
        race: Race::Elf,
        female: true,
    };
    const GARGOYLE_MAN: RaceChange = RaceChange {
        race: Race::Gargoyle,
        female: false,
    };

    /// The picks start as the tool does: no hair, no beard, and the first
    /// hue of each palette.
    #[test]
    fn the_first_looks_are_the_ones_the_tool_sends_by_default() {
        for change in [HUMAN_MAN, ELF_WOMAN, GARGOYLE_MAN] {
            assert_eq!(RacePicks::default().looks(change), change.first_looks());
        }
    }

    #[test]
    fn the_picks_become_the_looks_and_a_woman_sends_no_beard() {
        let picks = RacePicks {
            hair: 1,
            beard: 2,
            hues: [3, 4, 5],
            ..RacePicks::default()
        };
        let looks = picks.looks(HUMAN_MAN);
        assert_eq!(looks.hair, HUMAN_MAN.hair_styles()[1].graphic);
        assert_eq!(looks.beard, HUMAN_MAN.beard_styles()[2].graphic);
        assert_eq!(looks.skin_hue, HUMAN_MAN.skin_hues()[3]);
        assert_eq!(looks.hair_hue, HUMAN_MAN.hair_hues()[4]);
        assert_eq!(looks.beard_hue, HUMAN_MAN.hair_hues()[5]);
        let looks = picks.looks(ELF_WOMAN);
        assert_eq!((looks.beard, looks.beard_hue), (0, 0));
    }

    #[test]
    fn another_request_starts_the_picks_over() {
        let mut picks = RacePicks::default();
        picks.follow(HUMAN_MAN);
        *picks.style_place(StylePart::Hair) = 2;
        *picks.hue_place(Paint::Skin) = 3;
        picks.follow(HUMAN_MAN);
        assert_eq!((picks.hair, picks.hues[0]), (2, 3), "the same request");
        picks.follow(ELF_WOMAN);
        assert_eq!((picks.hair, picks.hues[0]), (0, 0));
    }

    /// The gargoyle words name horns; only a bearded character has a
    /// third color and a second style list.
    #[test]
    fn the_words_and_the_lists_follow_the_race_and_the_sex() {
        let gargoyle: Vec<_> = paints(GARGOYLE_MAN).into_iter().map(|p| p.1).collect();
        assert_eq!(
            gargoyle,
            vec![WORDS_SKIN, WORDS_HORN_COLOR, WORDS_FACIAL_HORN_COLOR]
        );
        assert_eq!(paints(ELF_WOMAN).len(), 2);
        assert_eq!(style_lists(ELF_WOMAN).len(), 1);
        assert_eq!(style_lists(GARGOYLE_MAN)[0].1, WORDS_HORN_STYLE);
        assert_eq!(doll_body(ELF_WOMAN), BODY_ELF_FEMALE);
        assert_eq!(doll_body(GARGOYLE_MAN), BODY_HUMAN_MALE);
        assert_eq!(palette_columns(HUMAN_MAN.skin_hues().len()), 8);
        assert_eq!(palette_columns(0), 1);
    }
}
