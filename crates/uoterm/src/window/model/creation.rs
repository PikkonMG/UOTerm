//! A new character as the player makes him, with no drawing: the pages of
//! the character creation of the classic client as one model, which the form
//! of the login screens (`creation_ui`) changes.
//!
//! The pages: the look (name, sex, race, hair and beard, colors), the
//! profession, the skills and stats of the Advanced choice, and the start
//! town. The model checks each page as the classic client does and makes
//! the wish the login sends to the shard.
//!
//! The name is checked for its length, its letters and the titles it may
//! not start with. The shard checks its own list of forbidden words and
//! refuses such a name with words the screen shows.

use crate::view::{WatchEquip, WatchLook};
use std::path::Path;
use uoterm_nav::{ClilocData, Profession, ProfessionKind, ProfessionList};
use uoterm_protocol::types::{
    LAYER_BEARD, LAYER_HAIR, LAYER_PANTS, LAYER_ROBE, LAYER_SHIRT, LAYER_SHOES, LAYER_SKIRT,
};
use uoterm_protocol::{ClientVersion, StartTown};
use uoterm_runtime::{CharacterChoices, NewCharacterWish};
pub use uoterm_world::Style;
use uoterm_world::{
    RaceChange, BODY_ELF_FEMALE, BODY_ELF_MALE, BODY_GARGOYLE_FEMALE, BODY_GARGOYLE_MALE,
    BODY_HUMAN_FEMALE, BODY_HUMAN_MALE, PALETTE_ROWS,
};

// The features of the account (`0xB9`) that open races and skills.
const FEATURE_AOS: u32 = 0x10;
const FEATURE_SE: u32 = 0x40;
const FEATURE_ML: u32 = 0x80;
const FEATURE_SA: u32 = 0x1_0000;
// The flags of the character list.
const LIST_ONE_SLOT: u32 = 0x04;
const LIST_SIX_SLOTS: u32 = 0x40;
const LIST_SAMURAI_NINJA: u32 = 0x80;
const LIST_ELVEN_RACE: u32 = 0x100;
const LIST_SEVEN_SLOTS: u32 = 0x1000;
const DEFAULT_SLOTS: usize = 5;
const ONE_SLOT: usize = 1;
const SIX_SLOTS: usize = 6;
const SEVEN_SLOTS: usize = 7;

/// The first client that offers the gargoyle race.
const GARGOYLE_VERSION: ClientVersion = ClientVersion::new(6, 0, 14, 4);

// The skills the creation treats on their own.
const SKILL_ARCHERY: u8 = 31;
const SKILL_STEALTH: u8 = 47;
const SKILL_REMOVE_TRAP: u8 = 48;
const SKILL_NECROMANCY: u8 = 49;
const SKILL_FOCUS: u8 = 50;
const SKILL_CHIVALRY: u8 = 51;
const SKILL_BUSHIDO: u8 = 52;
const SKILL_NINJITSU: u8 = 53;
const SKILL_SPELLWEAVING: u8 = 54;
const SKILL_MYSTICISM: u8 = 55;
const SKILL_IMBUING: u8 = 56;
const SKILL_THROWING: u8 = 57;

/// The skills and the stats a new character may have, and where the
/// Advanced choice starts them.
pub const STAT_RANGE: (i32, i32) = (10, 60);
pub const SKILL_RANGE: (i32, i32) = (0, 50);
const FIRST_STAT: i32 = 60;
const OTHER_STATS_NEW: i32 = 15;
const OTHER_STATS_OLD: i32 = 10;
const SKILL_START_NEW: i32 = 30;
const SKILL_START_OLD: i32 = 50;
const SKILL_SLOTS_NEW: usize = 4;
const SKILL_SLOTS_OLD: usize = 3;
/// Before 7.0.16.0 the third skill starts empty.
const OLD_EMPTY_SKILL: usize = 2;

// The words of the pages that can refuse to go on.
pub const WORDS_NAME_SHORT: (u32, &str) = (3_000_612, "Your Character Name is Too Short");
pub const WORDS_NAME_BAD: (u32, &str) = (3_000_611, "Unacceptable Name");
pub const WORDS_UNIQUE_SKILLS: (u32, &str) =
    (1_080_032, "You must have three unique skills chosen!");
pub const WORDS_NEEDS_SAMURAI_EMPIRE: (u32, &str) = (
    1_063_016,
    "You must upgrade your account to Samurai Empire before you can choose that profession.",
);

// The name rules of the classic client.
const NAME_MIN: usize = 2;
const NAME_MAX: usize = 16;
const NAME_MARKS: [char; 4] = [' ', '-', '.', '\''];
const NAME_MARKS_IN_A_ROW: usize = 1;
const NAME_TITLES: [&str; 6] = ["seer", "counselor", "gm", "admin", "lady", "lord"];

/// The facets, by map number, as the start town page names them.
const FACET_NAMES: [&str; 6] = [
    "Felucca", "Trammel", "Ilshenar", "Malas", "Tokuno", "Ter Mur",
];
/// An older client starts at the fourth town of the list.
const OLD_FIRST_TOWN: usize = 3;

// The bodies and the clothes of the look.
const SHOES: u16 = 0x1710;
const SHOES_HUE: u16 = 0x0384;
const PANTS: u16 = 0x152F;
const SKIRT: u16 = 0x1531;
const SHIRT: u16 = 0x1518;
const GARGOYLE_ROBE: u16 = 0x4001;

// The cloth colors of the shirt and the pants.
const CLOTH_ROWS: usize = 10;
const CLOTH_COLUMNS: usize = 20;
const CLOTH_FIRST_HUE: u16 = 3;
const CLOTH_HUE_STEP: u16 = 5;
/// A new character starts with the second hair style and no beard.
const FIRST_HAIR: usize = 1;

/// The race of a new character, as the shard numbers it from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Race {
    #[default]
    Human,
    Elf,
    Gargoyle,
}

impl Race {
    pub const ALL: [Race; 3] = [Race::Human, Race::Elf, Race::Gargoyle];

    fn number(self) -> u8 {
        match self {
            Race::Human => 0,
            Race::Elf => 1,
            Race::Gargoyle => 2,
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Race::Human => "Human",
            Race::Elf => "Elf",
            Race::Gargoyle => "Gargoyle",
        }
    }

    /// The race as the shared looks of the world know it.
    fn looks(self) -> uoterm_world::Race {
        match self {
            Race::Human => uoterm_world::Race::Human,
            Race::Elf => uoterm_world::Race::Elf,
            Race::Gargoyle => uoterm_world::Race::Gargoyle,
        }
    }
}

/// What a color of the look paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    Skin,
    Shirt,
    Pants,
    Hair,
    Beard,
}

impl Paint {
    pub const ALL: [Paint; 5] = [
        Paint::Skin,
        Paint::Shirt,
        Paint::Pants,
        Paint::Hair,
        Paint::Beard,
    ];

    fn at(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// The hues one color picks from, in rows and columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub rows: usize,
    pub columns: usize,
    /// The hue numbers, row by row.
    pub hues: Vec<u16>,
}

impl Palette {
    /// A palette of the shared looks, whose hues come ready to pick.
    fn of(hues: Vec<u16>) -> Self {
        Self {
            rows: PALETTE_ROWS,
            columns: hues.len() / PALETTE_ROWS,
            hues,
        }
    }

    fn cloth() -> Self {
        let count = CLOTH_ROWS * CLOTH_COLUMNS;
        Self {
            rows: CLOTH_ROWS,
            columns: CLOTH_COLUMNS,
            hues: (0..count as u16)
                .map(|at| CLOTH_FIRST_HUE + at * CLOTH_HUE_STEP)
                .collect(),
        }
    }
}

/// The page of the creation that shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Look,
    /// The professions, or the ones of a category.
    Profession(Option<String>),
    /// The skills and stats of the Advanced choice.
    Trade,
    Town,
}

/// One skill of the Advanced choice: the skill picked, and its points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillPick {
    pub skill: Option<u8>,
    pub value: i32,
}

/// What the creation reads from the client files.
#[derive(Default)]
pub struct CreationFiles {
    pub professions: ProfessionList,
    /// The skill names, by skill number.
    pub skill_names: Vec<String>,
    /// The words about each start town of an older client.
    pub town_texts: Vec<String>,
    /// The text numbers of the client, for the names and the words of the
    /// professions and of the newer start towns.
    words: Option<ClilocData>,
}

impl CreationFiles {
    pub fn read(uopath: Option<&Path>) -> Self {
        let Some(dir) = uopath else {
            return Self::default();
        };
        Self {
            professions: uoterm_nav::read_professions(dir),
            skill_names: uoterm_nav::read_skills(dir)
                .map(|skills| skills.into_iter().map(|skill| skill.name).collect())
                .unwrap_or_default(),
            town_texts: uoterm_nav::read_city_texts(dir).unwrap_or_default(),
            words: ClilocData::open(dir).ok(),
        }
    }

    /// The words of a text number of the client, or `fallback`.
    pub fn words(&self, number: u32, fallback: &str) -> String {
        self.words
            .as_ref()
            .and_then(|words| words.text(number))
            .unwrap_or(fallback)
            .to_string()
    }

    /// The words about the start town at `at` of the list: its text number
    /// for a newer town, the town file of the client for an older one, or
    /// its building.
    pub fn town_words(&self, town: &StartTown, at: usize) -> String {
        match town.place {
            Some(place) => self.words(place.description, &town.building),
            None => self
                .town_texts
                .get(at)
                .cloned()
                .unwrap_or_else(|| town.building.clone()),
        }
    }
}

/// How many characters the account may have, by the flags of the list.
pub fn character_slots(list_flags: u32) -> usize {
    if list_flags & LIST_ONE_SLOT != 0 {
        ONE_SLOT
    } else if list_flags & LIST_SEVEN_SLOTS != 0 {
        SEVEN_SLOTS
    } else if list_flags & LIST_SIX_SLOTS != 0 {
        SIX_SLOTS
    } else {
        DEFAULT_SLOTS
    }
}

/// True when the account has room for one more character.
pub fn can_make(names: &[String], list_flags: u32) -> bool {
    names.iter().filter(|name| !name.is_empty()).count() < character_slots(list_flags)
}

/// The facet a start town lies on, by name.
pub fn facet_name(map: u32) -> &'static str {
    FACET_NAMES[(map as usize).min(FACET_NAMES.len() - 1)]
}

/// The text number of the words about a name that the classic client does
/// not take, or None when it takes it.
pub fn check_name(name: &str) -> Option<(u32, &'static str)> {
    let count = name.chars().count();
    if count < NAME_MIN {
        return Some(WORDS_NAME_SHORT);
    }
    if count > NAME_MAX || name.trim() != name {
        return Some(WORDS_NAME_BAD);
    }
    let lower = name.to_lowercase();
    let mut marks_in_a_row = 0;
    for (at, c) in lower.chars().enumerate() {
        if c.is_ascii_lowercase() {
            marks_in_a_row = 0;
        } else if !NAME_MARKS.contains(&c) || at == 0 || marks_in_a_row == NAME_MARKS_IN_A_ROW {
            return Some(WORDS_NAME_BAD);
        } else {
            marks_in_a_row += 1;
        }
    }
    NAME_TITLES
        .iter()
        .any(|title| lower.starts_with(title))
        .then_some(WORDS_NAME_BAD)
}

/// Moves one value of a group whose sum stays the same, as the paired
/// sliders of the classic client do: the moved value goes where the player
/// put it, and the others give back or take the difference one point at a
/// time in turn, as far as their limits let them. The first to give is at
/// the new value modulo the number of the others.
pub fn move_paired(values: &mut [i32], moved: usize, to: i32, (min, max): (i32, i32)) {
    let Some(old) = values.get(moved).copied() else {
        return;
    };
    let new = to.clamp(min, max);
    values[moved] = new;
    let others: Vec<usize> = (0..values.len()).filter(|at| *at != moved).collect();
    if others.is_empty() {
        return;
    }
    let step = if new > old { -1 } else { 1 };
    let mut points = (new - old).abs();
    let mut turn = new.rem_euclid(others.len() as i32) as usize;
    let mut moved_this_round = true;
    while points > 0 {
        let at = others[turn];
        let room = if step > 0 {
            values[at] < max
        } else {
            values[at] > min
        };
        if room {
            values[at] += step;
            points -= 1;
            moved_this_round = true;
        }
        turn += 1;
        if turn == others.len() {
            if !moved_this_round {
                return;
            }
            moved_this_round = false;
            turn = 0;
        }
    }
}

/// A new character as the player makes him.
#[derive(Clone, Debug)]
pub struct Creation {
    pub version: ClientVersion,
    pub choices: CharacterChoices,
    pub step: Step,
    pub name: String,
    pub female: bool,
    pub race: Race,
    /// The hair and the beard styles, by place in their lists.
    pub hair: usize,
    pub beard: usize,
    /// The place of each color in its palette, in the order of
    /// [`Paint::ALL`].
    colors: [usize; 5],
    /// The profession picked, or Advanced.
    pub profession: Option<Profession>,
    /// Strength, intelligence and dexterity, in the order the Advanced page
    /// shows them.
    pub stats: [i32; 3],
    pub skills: Vec<SkillPick>,
    /// The start town, by place in the list of the shard.
    pub town: usize,
    /// Words that stop the player going on, until he closes them.
    pub message: Option<(u32, &'static str)>,
}

impl Creation {
    pub fn new(version: ClientVersion, choices: CharacterChoices) -> Self {
        let town = if version.has_placed_start_towns() || choices.towns.len() <= OLD_FIRST_TOWN {
            0
        } else {
            OLD_FIRST_TOWN
        };
        let mut creation = Self {
            version,
            choices,
            step: Step::Look,
            name: String::new(),
            female: false,
            race: Race::Human,
            hair: FIRST_HAIR,
            beard: 0,
            colors: [0; 5],
            profession: None,
            stats: [0; 3],
            skills: Vec::new(),
            town,
            message: None,
        };
        creation.start_trade();
        creation
    }

    /// How many starting skills the version sends.
    pub fn skill_slots(&self) -> usize {
        if self.version.has_three_starting_skills() {
            SKILL_SLOTS_NEW
        } else {
            SKILL_SLOTS_OLD
        }
    }

    /// The races the version shows. The shard may still lock some.
    pub fn races_shown(&self) -> Vec<Race> {
        Race::ALL
            .into_iter()
            .filter(|race| *race != Race::Gargoyle || self.version.at_least(GARGOYLE_VERSION))
            .collect()
    }

    /// True when the account may make a character of this race.
    pub fn race_allowed(&self, race: Race) -> bool {
        let features = self.choices.features;
        match race {
            Race::Human => true,
            Race::Elf => {
                self.choices.list_flags & LIST_ELVEN_RACE != 0 && features & FEATURE_ML != 0
            }
            Race::Gargoyle => features & FEATURE_SA != 0,
        }
    }

    /// A new race starts its colors and styles again.
    pub fn set_race(&mut self, race: Race) {
        if race != self.race {
            self.race = race;
            self.colors = [0; 5];
            self.set_female(self.female);
        }
    }

    /// A new sex starts the hair and the beard again.
    pub fn set_female(&mut self, female: bool) {
        self.female = female;
        self.hair = FIRST_HAIR;
        self.beard = 0;
    }

    /// The race and the sex, as the shared looks of the world read them.
    fn shape(&self) -> RaceChange {
        RaceChange {
            race: self.race.looks(),
            female: self.female,
        }
    }

    pub fn hair_styles(&self) -> &'static [Style] {
        self.shape().hair_styles()
    }

    /// The beard styles, for a man of a race that grows one.
    pub fn beard_styles(&self) -> Option<&'static [Style]> {
        let shape = self.shape();
        shape.has_beard().then(|| shape.beard_styles())
    }

    /// The colors the look shows a picker for.
    pub fn paints(&self) -> Vec<Paint> {
        Paint::ALL
            .into_iter()
            .filter(|paint| match paint {
                Paint::Pants => self.race != Race::Gargoyle,
                Paint::Beard => self.beard_styles().is_some(),
                _ => true,
            })
            .collect()
    }

    pub fn palette(&self, paint: Paint) -> Palette {
        match paint {
            Paint::Skin => Palette::of(self.shape().skin_hues()),
            Paint::Shirt | Paint::Pants => Palette::cloth(),
            Paint::Hair | Paint::Beard => Palette::of(self.shape().hair_hues()),
        }
    }

    /// The place of the color picked in its palette.
    pub fn color_place(&self, paint: Paint) -> usize {
        self.colors[paint.at()]
    }

    pub fn set_color(&mut self, paint: Paint, place: usize) {
        if place < self.palette(paint).hues.len() {
            self.colors[paint.at()] = place;
        }
    }

    pub fn hue(&self, paint: Paint) -> u16 {
        let palette = self.palette(paint);
        palette
            .hues
            .get(self.color_place(paint))
            .or(palette.hues.first())
            .copied()
            .unwrap_or_default()
    }

    fn hair_graphic(&self) -> u16 {
        self.hair_styles()
            .get(self.hair)
            .map_or(0, |style| style.graphic)
    }

    fn beard_graphic(&self) -> u16 {
        self.beard_styles()
            .and_then(|styles| styles.get(self.beard))
            .map_or(0, |style| style.graphic)
    }

    /// The body the preview shows.
    pub fn body(&self) -> u16 {
        match (self.race, self.female) {
            (Race::Human, false) => BODY_HUMAN_MALE,
            (Race::Human, true) => BODY_HUMAN_FEMALE,
            (Race::Elf, false) => BODY_ELF_MALE,
            (Race::Elf, true) => BODY_ELF_FEMALE,
            (Race::Gargoyle, false) => BODY_GARGOYLE_MALE,
            (Race::Gargoyle, true) => BODY_GARGOYLE_FEMALE,
        }
    }

    /// The look of the preview: the body in the skin color, wearing the
    /// clothes a new character starts in, then the hair and the beard.
    pub fn look(&self) -> WatchLook {
        let (shirt, pants) = (self.hue(Paint::Shirt), self.hue(Paint::Pants));
        let mut worn = match (self.race, self.female) {
            (Race::Gargoyle, _) => vec![(LAYER_ROBE, GARGOYLE_ROBE, shirt)],
            (Race::Elf, true) => vec![
                (LAYER_SHOES, SHOES, SHOES_HUE),
                (LAYER_SKIRT, SKIRT, pants),
                (LAYER_SHIRT, SHIRT, shirt),
            ],
            (_, female) => vec![
                (LAYER_SHOES, SHOES, SHOES_HUE),
                (LAYER_PANTS, if female { SKIRT } else { PANTS }, pants),
                (LAYER_SHIRT, SHIRT, shirt),
            ],
        };
        let hair = self.hair_graphic();
        if hair != 0 {
            worn.push((LAYER_HAIR, hair, self.hue(Paint::Hair)));
        }
        let beard = self.beard_graphic();
        if beard != 0 {
            worn.push((LAYER_BEARD, beard, self.hue(Paint::Beard)));
        }
        WatchLook {
            body: self.body(),
            hue: self.hue(Paint::Skin),
            equipment: worn
                .into_iter()
                .map(|(layer, graphic, hue)| WatchEquip {
                    serial: 0,
                    graphic,
                    layer,
                    hue,
                })
                .collect(),
            ..WatchLook::default()
        }
    }

    /// Next on the look page: a good name and a race the account has.
    pub fn look_done(&mut self) {
        if let Some(words) = check_name(&self.name) {
            self.message = Some(words);
        } else if self.race_allowed(self.race) {
            self.step = Step::Profession(None);
        }
    }

    /// The professions of the page: the top ones, or those of a category.
    pub fn professions<'a>(&self, files: &'a CreationFiles) -> Vec<&'a Profession> {
        match &self.step {
            Step::Profession(Some(category)) => files
                .professions
                .top()
                .into_iter()
                .find(|p| &p.true_name == category)
                .map(|p| files.professions.children(p))
                .filter(|children| !children.is_empty())
                .unwrap_or_else(|| files.professions.top()),
            _ => files.professions.top(),
        }
    }

    /// The player picked a profession: a category opens its own list;
    /// Advanced goes to the skills and stats; a profession takes its
    /// template and goes to the towns.
    pub fn pick_profession(&mut self, profession: &Profession, files: &CreationFiles) {
        if profession.kind == ProfessionKind::Category {
            self.step = Step::Profession(Some(profession.true_name.clone()));
            return;
        }
        if profession.is_advanced() {
            self.profession = Some(profession.clone());
            self.start_trade();
            self.step = Step::Trade;
            return;
        }
        let skills = profession.skill_numbers(&files.skill_names);
        let samurai_ninja = self.choices.list_flags & LIST_SAMURAI_NINJA != 0;
        if !samurai_ninja
            && skills
                .iter()
                .any(|(skill, _)| *skill == SKILL_BUSHIDO || *skill == SKILL_NINJITSU)
        {
            self.message = Some(WORDS_NEEDS_SAMURAI_EMPIRE);
            return;
        }
        self.skills = skills
            .into_iter()
            .take(self.skill_slots())
            .map(|(skill, value)| SkillPick {
                skill: Some(skill),
                value: i32::from(value),
            })
            .collect();
        let [strength, dexterity, intelligence] = profession.stats;
        self.stats = [
            i32::from(strength),
            i32::from(intelligence),
            i32::from(dexterity),
        ];
        self.profession = Some(profession.clone());
        self.step = Step::Town;
    }

    /// The skills and the stats the Advanced page starts with.
    fn start_trade(&mut self) {
        let new = self.version.has_three_starting_skills();
        let (skill, other_stats) = if new {
            (SKILL_START_NEW, OTHER_STATS_NEW)
        } else {
            (SKILL_START_OLD, OTHER_STATS_OLD)
        };
        self.stats = [FIRST_STAT, other_stats, other_stats];
        self.skills = (0..self.skill_slots())
            .map(|at| SkillPick {
                skill: None,
                value: if !new && at == OLD_EMPTY_SKILL {
                    0
                } else {
                    skill
                },
            })
            .collect();
    }

    pub fn set_stat(&mut self, at: usize, value: i32) {
        move_paired(&mut self.stats, at, value, STAT_RANGE);
    }

    pub fn set_skill_value(&mut self, at: usize, value: i32) {
        let mut values: Vec<i32> = self.skills.iter().map(|pick| pick.value).collect();
        move_paired(&mut values, at, value, SKILL_RANGE);
        for (pick, value) in self.skills.iter_mut().zip(values) {
            pick.value = value;
        }
    }

    pub fn set_skill(&mut self, at: usize, skill: u8) {
        if let Some(pick) = self.skills.get_mut(at) {
            pick.skill = Some(skill);
        }
    }

    /// The skills the Advanced page offers, by name: the account's
    /// expansions open some; a gargoyle throws and does not shoot.
    pub fn skill_menu(&self, files: &CreationFiles) -> Vec<(u8, String)> {
        let features = self.choices.features;
        let gargoyle = self.race == Race::Gargoyle;
        let mut menu: Vec<(u8, String)> = files
            .skill_names
            .iter()
            .enumerate()
            .filter_map(|(at, name)| Some((u8::try_from(at).ok()?, name.clone())))
            .filter(|(skill, _)| match *skill {
                SKILL_STEALTH | SKILL_REMOVE_TRAP | SKILL_SPELLWEAVING => false,
                SKILL_THROWING => gargoyle,
                SKILL_ARCHERY => !gargoyle,
                SKILL_NECROMANCY | SKILL_FOCUS | SKILL_CHIVALRY => features & FEATURE_AOS != 0,
                SKILL_BUSHIDO | SKILL_NINJITSU => features & FEATURE_SE != 0,
                SKILL_MYSTICISM | SKILL_IMBUING => features & FEATURE_SA != 0,
                _ => true,
            })
            .collect();
        menu.sort_by(|a, b| a.1.cmp(&b.1));
        menu
    }

    /// Next on the Advanced page: each skill picked, and no skill twice.
    pub fn trade_done(&mut self) {
        let picked: Vec<u8> = self.skills.iter().filter_map(|pick| pick.skill).collect();
        let mut unique = picked.clone();
        unique.sort_unstable();
        unique.dedup();
        if picked.len() < self.skills.len() || unique.len() < picked.len() {
            self.message = Some(WORDS_UNIQUE_SKILLS);
        } else {
            self.step = Step::Town;
        }
    }

    /// One page back. True when the player left the creation.
    pub fn back(&mut self) -> bool {
        let advanced = self
            .profession
            .as_ref()
            .is_some_and(Profession::is_advanced);
        self.step = match &self.step {
            Step::Look => return true,
            Step::Profession(Some(_)) | Step::Trade => Step::Profession(None),
            Step::Profession(None) => Step::Look,
            Step::Town if advanced => Step::Trade,
            Step::Town => Step::Profession(None),
        };
        false
    }

    pub fn towns(&self) -> &[StartTown] {
        &self.choices.towns
    }

    pub fn set_town(&mut self, at: usize) {
        if at < self.choices.towns.len() {
            self.town = at;
        }
    }

    /// The wish the login sends: the look, the skills and the stats, the
    /// town, and the first empty slot of the account.
    pub fn wish(&self, names: &[String]) -> NewCharacterWish {
        let mut skills: Vec<(u8, u8)> = self
            .skills
            .iter()
            .filter_map(|pick| Some((pick.skill?, pick.value.clamp(0, i32::from(u8::MAX)) as u8)))
            .collect();
        skills.sort_by_key(|skill| std::cmp::Reverse(skill.1));
        let stat = |at: usize| self.stats[at].clamp(0, i32::from(u8::MAX)) as u8;
        let profession = self
            .profession
            .as_ref()
            .and_then(|p| u8::try_from(p.description_index).ok())
            .unwrap_or(0);
        let slot = names
            .iter()
            .position(String::is_empty)
            .unwrap_or(names.len());
        let beard = self.beard_graphic();
        NewCharacterWish {
            name: self.name.clone(),
            female: self.female,
            race: self.race.number(),
            strength: stat(0),
            intelligence: stat(1),
            dexterity: stat(2),
            skills,
            skin_hue: self.hue(Paint::Skin),
            hair: self.hair_graphic(),
            hair_hue: self.hue(Paint::Hair),
            beard,
            beard_hue: if beard == 0 {
                0
            } else {
                self.hue(Paint::Beard)
            },
            shirt_hue: self.hue(Paint::Shirt),
            pants_hue: self.hue(Paint::Pants),
            profession,
            start_city: self
                .choices
                .towns
                .get(self.town)
                .map_or(0, |town| u16::from(town.index)),
            slot: slot as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEW: ClientVersion = ClientVersion::new(7, 0, 102, 3);
    const OLD: ClientVersion = ClientVersion::new(5, 0, 9, 1);

    fn creation(version: ClientVersion) -> Creation {
        Creation::new(version, CharacterChoices::default())
    }

    fn files() -> CreationFiles {
        let text = "Begin\nName Warrior\nTrueName \"warrior\"\nDesc 1\nTopLevel true\n\
                    Type Profession\nSkill Tactics 30\nSkill Healing 30\nSkill Anatomy 30\n\
                    Skill Swordsmanship 30\nStat Str 45\nStat Dex 35\nStat Int 10\nEnd\n\
                    Begin\nName Ninja\nTrueName \"ninja\"\nDesc 7\nTopLevel true\n\
                    Type Profession\nSkill Ninjitsu 30\nSkill Hiding 30\nEnd\n";
        let professions = uoterm_nav::parse_professions(text);
        let mut skill_names: Vec<String> = (0..58).map(|at| format!("Skill {at}")).collect();
        skill_names[1] = "Anatomy".into();
        skill_names[17] = "Healing".into();
        skill_names[21] = "Hiding".into();
        skill_names[27] = "Tactics".into();
        skill_names[40] = "Swordsmanship".into();
        skill_names[53] = "Ninjitsu".into();
        CreationFiles {
            professions,
            skill_names,
            town_texts: vec!["<b>Yew</b> is a town.".into()],
            words: None,
        }
    }

    #[test]
    fn names_follow_the_classic_rules() {
        assert_eq!(check_name("Mara"), None);
        assert_eq!(check_name("Mara O'Dell"), None);
        assert_eq!(check_name("M"), Some(WORDS_NAME_SHORT));
        assert_eq!(check_name("Mara  Dell"), Some(WORDS_NAME_BAD));
        assert_eq!(check_name(" Mara"), Some(WORDS_NAME_BAD));
        assert_eq!(check_name("-Mara"), Some(WORDS_NAME_BAD));
        assert_eq!(check_name("Mara2"), Some(WORDS_NAME_BAD));
        assert_eq!(check_name("Lord Mara"), Some(WORDS_NAME_BAD));
        assert_eq!(check_name("Abcdefghijklmnopq"), Some(WORDS_NAME_BAD));
    }

    #[test]
    fn paired_values_keep_their_sum_within_their_limits() {
        let mut stats = [60, 15, 15];
        move_paired(&mut stats, 0, 40, STAT_RANGE);
        assert_eq!(stats.iter().sum::<i32>(), 90);
        assert_eq!(stats[0], 40);
        assert_eq!(stats, [40, 25, 25]);
        let mut skills = [50, 50, 0];
        move_paired(&mut skills, 2, 30, SKILL_RANGE);
        assert_eq!(skills, [35, 35, 30]);
        // The others cannot go past their limits: the sum shrinks.
        let mut full = [50, 50];
        move_paired(&mut full, 0, 0, SKILL_RANGE);
        assert_eq!(full, [0, 50]);
    }

    #[test]
    fn the_version_sets_the_skill_slots_the_start_points_and_the_races() {
        let new = creation(NEW);
        assert_eq!(new.skills.len(), 4);
        assert_eq!(new.stats, [60, 15, 15]);
        assert!(new.races_shown().contains(&Race::Gargoyle));
        let old = creation(OLD);
        assert_eq!(
            old.skills.iter().map(|s| s.value).collect::<Vec<_>>(),
            vec![50, 50, 0]
        );
        assert_eq!(old.stats, [60, 10, 10]);
        assert!(!old.races_shown().contains(&Race::Gargoyle));
    }

    #[test]
    fn races_open_by_the_account_and_change_the_look() {
        let mut look = creation(NEW);
        assert!(!look.race_allowed(Race::Elf));
        look.choices.features = FEATURE_ML | FEATURE_SA;
        look.choices.list_flags = LIST_ELVEN_RACE;
        assert!(look.race_allowed(Race::Elf) && look.race_allowed(Race::Gargoyle));
        look.set_color(Paint::Skin, 5);
        look.set_race(Race::Gargoyle);
        assert_eq!(look.color_place(Paint::Skin), 0);
        assert_eq!(look.body(), BODY_GARGOYLE_MALE);
        assert!(!look.paints().contains(&Paint::Pants));
        let robe = &look.look().equipment[0];
        assert_eq!(
            (robe.layer, robe.graphic, robe.hue),
            (LAYER_ROBE, GARGOYLE_ROBE, look.hue(Paint::Shirt))
        );
        look.set_race(Race::Elf);
        assert!(look.beard_styles().is_none());
        look.set_race(Race::Human);
        look.beard = 1;
        assert!(look.look().equipment.iter().any(|w| w.layer == LAYER_BEARD));
        look.set_female(true);
        assert_eq!((look.hair, look.beard), (FIRST_HAIR, 0));
        assert!(look.look().equipment.iter().any(|w| w.graphic == SKIRT));
        assert_eq!(look.look().body, BODY_HUMAN_FEMALE);
        let human = RaceChange {
            race: uoterm_world::Race::Human,
            female: true,
        };
        assert_eq!(look.hue(Paint::Skin), human.skin_hues()[0]);
    }

    #[test]
    fn a_profession_takes_its_template_and_a_samurai_skill_needs_the_flag() {
        let files = files();
        let mut new = creation(NEW);
        new.name = "Mara".into();
        new.look_done();
        assert_eq!(new.step, Step::Profession(None));
        let top = files.professions.top();
        new.pick_profession(top[1], &files);
        assert_eq!(new.message, Some(WORDS_NEEDS_SAMURAI_EMPIRE));
        new.pick_profession(top[0], &files);
        assert_eq!(new.step, Step::Town);
        assert_eq!(new.stats, [45, 10, 35]);
        assert_eq!(new.skills.len(), 4);
        assert!(!new.back());
        assert_eq!(new.step, Step::Profession(None));
        let advanced = top.last().unwrap();
        new.pick_profession(advanced, &files);
        assert_eq!(new.step, Step::Trade);
        new.trade_done();
        assert_eq!(new.message, Some(WORDS_UNIQUE_SKILLS));
        for (at, skill) in [1, 17, 21, 27].into_iter().enumerate() {
            new.set_skill(at, skill);
        }
        new.set_skill(3, 1);
        new.message = None;
        new.trade_done();
        assert_eq!(new.message, Some(WORDS_UNIQUE_SKILLS));
        new.set_skill(3, 27);
        new.trade_done();
        assert_eq!(new.step, Step::Town);
        assert!(!new.back());
        assert_eq!(new.step, Step::Trade);
    }

    #[test]
    fn the_skill_menu_follows_the_expansions_and_the_race() {
        let files = files();
        let mut look = creation(NEW);
        let skills = |look: &Creation| -> Vec<u8> {
            look.skill_menu(&files)
                .into_iter()
                .map(|(n, _)| n)
                .collect()
        };
        let plain = skills(&look);
        assert!(!plain.contains(&SKILL_BUSHIDO) && !plain.contains(&SKILL_STEALTH));
        assert!(plain.contains(&SKILL_ARCHERY) && !plain.contains(&SKILL_THROWING));
        look.choices.features = FEATURE_SE | FEATURE_SA;
        look.race = Race::Gargoyle;
        let wide = skills(&look);
        assert!(wide.contains(&SKILL_BUSHIDO) && wide.contains(&SKILL_THROWING));
        assert!(!wide.contains(&SKILL_ARCHERY));
        let names: Vec<String> = look
            .skill_menu(&files)
            .into_iter()
            .map(|(_, n)| n)
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn the_wish_carries_the_look_the_template_the_town_and_the_free_slot() {
        let files = files();
        let towns = vec![
            StartTown {
                index: 0,
                name: "Yew".into(),
                ..StartTown::default()
            },
            StartTown {
                index: 4,
                name: "Britain".into(),
                ..StartTown::default()
            },
        ];
        let mut new = Creation::new(
            NEW,
            CharacterChoices {
                towns,
                ..CharacterChoices::default()
            },
        );
        assert_eq!(new.town, 0);
        new.name = "Mara".into();
        new.beard = 2;
        new.pick_profession(files.professions.top()[0], &files);
        new.set_town(1);
        let wish = new.wish(&["Old".into(), String::new(), "Other".into()]);
        assert_eq!(wish.slot, 1);
        assert_eq!(wish.start_city, 4);
        assert_eq!(wish.profession, 1);
        assert_eq!(
            (wish.strength, wish.dexterity, wish.intelligence),
            (45, 35, 10)
        );
        let styles = new.shape();
        assert_eq!(wish.beard, styles.beard_styles()[2].graphic);
        assert_eq!(wish.hair, styles.hair_styles()[FIRST_HAIR].graphic);
        assert_eq!(wish.skills.len(), 4);
        assert_eq!(
            files.town_words(&new.towns()[0], 0),
            "<b>Yew</b> is a town."
        );
        assert_eq!(files.town_words(&new.towns()[1], 1), "");
        assert_eq!(files.words(1, "fallback"), "fallback");
        let old = Creation::new(
            OLD,
            CharacterChoices {
                towns: vec![StartTown::default(); 5],
                ..CharacterChoices::default()
            },
        );
        assert_eq!(old.town, OLD_FIRST_TOWN);
    }

    #[test]
    fn the_account_has_room_by_its_slots() {
        let names = vec!["A".to_string(); 5];
        assert!(!can_make(&names, 0));
        assert!(can_make(&names, LIST_SIX_SLOTS));
        assert!(!can_make(&["A".into()], LIST_ONE_SLOT));
        assert_eq!(facet_name(1), "Trammel");
        assert_eq!(facet_name(99), "Ter Mur");
    }
}
