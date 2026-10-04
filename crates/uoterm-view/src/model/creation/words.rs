//! The words of the creation screen: the fixed words of its pages, and the
//! words it makes from the model, such as the name of a profession, the
//! rules of a name, or the summary before Create. The window and the web
//! client both show these, so each is written here once.

use super::{at_limit, Creation, CreationFiles, Limit, Paint, Race, Stage, Step};
use super::{NAME_MAX, NAME_MIN, SKILL_RANGE, STAT_RANGE};
use serde::Serialize;
use uoterm_nav::{Profession, ProfessionKind};

pub const WORDS_TITLE: &str = "New character";
pub const WORDS_BACK: &str = "Back";
pub const WORDS_NEXT: &str = "Next";
pub const WORDS_CREATE: &str = "Create";
pub const WORDS_KEYS: &str = "Enter: next     Esc: back";
pub const WORDS_TURN_LEFT: &str = "Turn left";
pub const WORDS_TURN_RIGHT: &str = "Turn right";
pub const WORDS_NO_ART: &str = "The figure needs the client files.";
pub const WORDS_BODY: &str = "Body";
pub const WORDS_MALE: &str = "Male";
pub const WORDS_FEMALE: &str = "Female";
const WORDS_MAN: &str = "man";
const WORDS_WOMAN: &str = "woman";
const WORDS_LOCKED: &str = "locked";
pub const WORDS_HAIR: &str = "Hair";
pub const WORDS_BEARD: &str = "Beard";
pub const WORDS_PROFESSION: &str = "Pick a profession";
pub const WORDS_PROFESSION_HINT: &str = "Or pick Custom to set your own stats and skills.";
const WORDS_CUSTOM: &str = "Custom";
pub const WORDS_CUSTOM_ABOUT: &str = "Set your own stats and skills on the next page.";
const WORDS_NEEDS_SAMURAI: &str = "Needs Samurai Empire";
const WORDS_CATEGORY: &str = "Opens a list of professions.";
pub const WORDS_STATS: &str = "Stats";
pub const WORDS_SKILLS: &str = "Skills";
pub const WORDS_PICK_SKILL: &str = "Pick a skill";
pub const WORDS_SEARCH: &str = "Search skills";
const WORDS_TAKEN: &str = "(taken)";
pub const WORDS_TOWN: &str = "Pick a start town";
pub const WORDS_NAME: &str = "Name your character";
pub const WORDS_NAME_HINT: &str = "Type a name";
pub const WORDS_NAME_GOOD: &str = "The name is good.";
pub const WORDS_SUMMARY: &str = "Your character";
const WORDS_LOOK: &str = "Look";
pub const WORDS_COLORS: &str = "Colors";
const WORDS_TRADE: &str = "Profession";
const WORDS_START: &str = "Town";
/// The web page waits for the creation files of the server with these.
pub const WORDS_LOADING: &str = "Reading the client files...";
/// The stats in the order the Custom page shows them.
pub const STAT_WORDS: [&str; 3] = ["Strength", "Intelligence", "Dexterity"];
const STAT_SHORT: [&str; 3] = ["Str", "Int", "Dex"];
const LIST_JOIN: &str = "  ·  ";
/// The colors of the look in the order the page shows them: the body
/// first, then the clothes.
const COLOR_ORDER: [Paint; 5] = [
    Paint::Skin,
    Paint::Hair,
    Paint::Beard,
    Paint::Shirt,
    Paint::Pants,
];
const WORDS_LEAST: &str = "least";
const WORDS_MOST: &str = "most";

// The words of the client have tags.
const TAG_OPEN: char = '<';
const TAG_CLOSE: char = '>';
const LINE_BREAK_TAG: &str = "br";
const SELF_CLOSING: char = '/';
const SENTENCE_END: &str = ". ";

/// The fixed words of the creation screen, for a page that draws it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CreationWords {
    pub title: &'static str,
    pub back: &'static str,
    pub keys: &'static str,
    pub turn_left: &'static str,
    pub turn_right: &'static str,
    pub no_art: &'static str,
    pub body: &'static str,
    pub male: &'static str,
    pub female: &'static str,
    pub hair: &'static str,
    pub beard: &'static str,
    pub profession: &'static str,
    pub profession_hint: &'static str,
    pub stats: &'static str,
    pub skills: &'static str,
    pub pick_skill: &'static str,
    pub search: &'static str,
    pub town: &'static str,
    pub name: &'static str,
    pub name_hint: &'static str,
    pub summary: &'static str,
    pub colors: &'static str,
    pub loading: &'static str,
}

/// The fixed words of the creation screen.
pub const CREATION_WORDS: CreationWords = CreationWords {
    title: WORDS_TITLE,
    back: WORDS_BACK,
    keys: WORDS_KEYS,
    turn_left: WORDS_TURN_LEFT,
    turn_right: WORDS_TURN_RIGHT,
    no_art: WORDS_NO_ART,
    body: WORDS_BODY,
    male: WORDS_MALE,
    female: WORDS_FEMALE,
    hair: WORDS_HAIR,
    beard: WORDS_BEARD,
    profession: WORDS_PROFESSION,
    profession_hint: WORDS_PROFESSION_HINT,
    stats: WORDS_STATS,
    skills: WORDS_SKILLS,
    pick_skill: WORDS_PICK_SKILL,
    search: WORDS_SEARCH,
    town: WORDS_TOWN,
    name: WORDS_NAME,
    name_hint: WORDS_NAME_HINT,
    summary: WORDS_SUMMARY,
    colors: WORDS_COLORS,
    loading: WORDS_LOADING,
};

/// Where a part of the creation stands, from the part that shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Progress {
    Done,
    Current,
    Ahead,
}

impl Stage {
    /// Where this part stands when the player is in `now`.
    pub fn progress(self, now: Stage) -> Progress {
        let place = |stage: Stage| Stage::ALL.iter().position(|known| *known == stage);
        match (place(self), place(now)) {
            (Some(at), Some(now)) if at < now => Progress::Done,
            (Some(at), Some(now)) if at == now => Progress::Current,
            _ => Progress::Ahead,
        }
    }
}

/// Words of the client with their tags taken out: a line break tag starts
/// a new line.
pub fn plain_words(html: &str) -> String {
    let mut words = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, TAG_OPEN) => tag = Some(String::new()),
            (Some(name), TAG_CLOSE) => {
                let name = name.trim().trim_end_matches(SELF_CLOSING);
                if name.eq_ignore_ascii_case(LINE_BREAK_TAG) {
                    words.push('\n');
                }
                tag = None;
            }
            (Some(name), c) => name.push(c),
            (None, c) => words.push(c),
        }
    }
    words
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// The first sentence of some words, as the short text of a card.
pub fn first_sentence(words: &str) -> &str {
    let line = words.lines().next().unwrap_or_default();
    line.find(SENTENCE_END)
        .map_or(line, |at| &line[..=at])
        .trim()
}

/// Words in capitals, as the client writes a profession ("SAMURAI"), with
/// only the first letter of each word as a capital.
pub fn name_case(words: &str) -> String {
    if words.chars().any(char::is_lowercase) {
        return words.to_string();
    }
    words
        .split(' ')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or(String::new(), |first| {
                first.to_string() + &letters.as_str().to_lowercase()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The name of a profession as the card shows it: Advanced is Custom.
pub fn profession_name(profession: &Profession, files: &CreationFiles) -> String {
    if profession.is_advanced() {
        WORDS_CUSTOM.to_string()
    } else {
        name_case(&files.words(profession.name_id, &profession.name))
    }
}

/// The words about a profession: what Custom does, what a category does,
/// or the words of the client about it.
pub fn profession_about(profession: &Profession, files: &CreationFiles) -> String {
    if profession.is_advanced() {
        WORDS_CUSTOM_ABOUT.to_string()
    } else if profession.kind == ProfessionKind::Category {
        WORDS_CATEGORY.to_string()
    } else {
        plain_words(&files.words(profession.description_id, ""))
    }
}

/// The short words of a card: why a locked profession cannot be taken, or
/// the first sentence of its words.
pub fn card_short(about: &str, locked: bool) -> &str {
    if locked {
        WORDS_NEEDS_SAMURAI
    } else {
        first_sentence(about)
    }
}

/// The rules of a name, as the page lists them.
pub fn name_rules() -> [String; 3] {
    [
        format!("{NAME_MIN} to {NAME_MAX} characters."),
        "Letters, spaces and the marks - . ' only. Start with a letter, and put a letter between two marks.".into(),
        "No title at the start, such as Lord, Lady, GM or Seer.".into(),
    ]
}

/// What a color of the look is called: a gargoyle wears a robe.
pub fn paint_words(paint: Paint, race: Race) -> &'static str {
    match paint {
        Paint::Skin => "Skin",
        Paint::Shirt if race == Race::Gargoyle => "Robe",
        Paint::Shirt => "Shirt",
        Paint::Pants => "Pants",
        Paint::Hair => "Hair color",
        Paint::Beard => "Beard color",
    }
}

/// The words of a race choice: a race the account may not make says so.
pub fn race_words(race: Race, allowed: bool) -> String {
    if allowed {
        race.words().to_string()
    } else {
        format!("{} ({WORDS_LOCKED})", race.words())
    }
}

/// The skill of a list a row picks from: one another row has is taken.
pub fn skill_choice_words(name: &str, taken: bool) -> String {
    if taken {
        format!("{name} {WORDS_TAKEN}")
    } else {
        name.to_string()
    }
}

/// The sum of a group of values and the total it must make.
pub fn total_words(values: i32, total: i32) -> String {
    format!("Total {values} of {total}")
}

/// The rule of the stats of the Custom page.
pub fn stat_rule() -> String {
    format!(
        "Each stat is {} to {}. Moving one moves the others.",
        STAT_RANGE.0, STAT_RANGE.1
    )
}

/// The rule of the skills of the Custom page.
pub fn skill_rule() -> String {
    format!(
        "Each skill is {} to {}. Pick a different skill in each row. Moving one moves the others.",
        SKILL_RANGE.0, SKILL_RANGE.1
    )
}

/// The note of a value at one end of its range.
fn limit_note(name: &str, limit: Limit, value: i32) -> String {
    let end = match limit {
        Limit::Least => WORDS_LEAST,
        Limit::Most => WORDS_MOST,
    };
    format!("{name} is at its {end}: {value}.")
}

/// One skill of the list a row picks from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkillChoice {
    pub skill: u8,
    pub words: String,
    /// Another row has it: it cannot be picked.
    pub taken: bool,
    /// The row has it.
    pub picked: bool,
}

/// One row of the summary before Create: its label and its words. The
/// colors row has no words: the page shows a box of each color.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SummaryRow {
    pub label: &'static str,
    pub value: String,
}

impl Creation {
    /// The words of the race and the sex, as "Human man".
    pub fn body_words(&self) -> String {
        let sex = if self.female { WORDS_WOMAN } else { WORDS_MAN };
        format!("{} {sex}", self.race.words())
    }

    /// The colors of the look in the order the page shows them.
    pub fn shown_paints(&self) -> Vec<Paint> {
        let paints = self.paints();
        COLOR_ORDER
            .into_iter()
            .filter(|paint| paints.contains(paint))
            .collect()
    }

    /// The stats and the skills the character has, in words.
    pub fn trade_words(&self, files: &CreationFiles) -> (String, String) {
        let stats = STAT_SHORT
            .iter()
            .zip(self.stats)
            .map(|(words, value)| format!("{words} {value}"))
            .collect::<Vec<_>>()
            .join(LIST_JOIN);
        let skills = self
            .skills
            .iter()
            .filter_map(|pick| {
                let name = files.skill_names.get(usize::from(pick.skill?))?;
                Some(format!("{name} {}", pick.value))
            })
            .collect::<Vec<_>>()
            .join(LIST_JOIN);
        (stats, skills)
    }

    /// The two lines under the figure: the name, or the race and the sex
    /// when there is none yet, and what else is picked.
    pub fn caption(&self, files: &CreationFiles) -> (String, String) {
        let name = self.name.trim();
        let profession = self
            .profession
            .as_ref()
            .map(|profession| profession_name(profession, files));
        match (name.is_empty(), profession) {
            (true, profession) => (self.body_words(), profession.unwrap_or_default()),
            (false, Some(profession)) => (
                name.to_string(),
                format!("{}, {profession}", self.body_words()),
            ),
            (false, None) => (name.to_string(), self.body_words()),
        }
    }

    /// What the profession picked gives, in two lines, or None with none
    /// picked.
    pub fn profession_details(&self, files: &CreationFiles) -> Option<[String; 2]> {
        let profession = self.profession.as_ref()?;
        Some(if profession.is_advanced() {
            [WORDS_CUSTOM_ABOUT.to_string(), String::new()]
        } else {
            let (stats, skills) = self.trade_words(files);
            [
                format!("{WORDS_STATS}: {stats}"),
                format!("{WORDS_SKILLS}: {skills}"),
            ]
        })
    }

    /// The words at the bottom: why Next cannot go on, or the keys.
    pub fn footer_words(&self) -> String {
        self.blocker()
            .map_or_else(|| WORDS_KEYS.to_string(), |blocker| blocker.words())
    }

    /// The words of the main button: Create on the last page.
    pub fn next_words(&self) -> &'static str {
        if self.step == Step::Name {
            WORDS_CREATE
        } else {
            WORDS_NEXT
        }
    }

    /// The words under the name field: why the name is not taken, or that
    /// it is good.
    pub fn name_verdict(&self) -> String {
        self.blocker()
            .map_or_else(|| WORDS_NAME_GOOD.to_string(), |blocker| blocker.words())
    }

    /// The note of a stat at one end of its range.
    pub fn stat_limit_note(&self) -> Option<String> {
        at_limit(&self.stats, STAT_RANGE)
            .map(|(at, limit)| limit_note(STAT_WORDS[at], limit, self.stats[at]))
    }

    /// The note of a skill at one end of its range. A row with no skill is
    /// named by its place.
    pub fn skill_limit_note(&self, files: &CreationFiles) -> Option<String> {
        let values: Vec<i32> = self.skills.iter().map(|pick| pick.value).collect();
        at_limit(&values, SKILL_RANGE).map(|(at, limit)| {
            let name = self.skills[at]
                .skill
                .and_then(|skill| files.skill_names.get(usize::from(skill)).cloned())
                .unwrap_or_else(|| format!("Skill {}", at + 1));
            limit_note(&name, limit, values[at])
        })
    }

    /// The skills of `menu` a row picks from, whose names hold `search`.
    pub fn skill_choices(
        &self,
        menu: &[(u8, String)],
        row: usize,
        search: &str,
    ) -> Vec<SkillChoice> {
        let picked = self.skills.get(row).and_then(|pick| pick.skill);
        super::find_skills(menu, search)
            .into_iter()
            .map(|(skill, name)| {
                let taken = self.skill_taken(*skill, row);
                SkillChoice {
                    skill: *skill,
                    words: skill_choice_words(name, taken),
                    taken,
                    picked: picked == Some(*skill),
                }
            })
            .collect()
    }

    /// The name a skill row shows: its skill, or None with none picked.
    pub fn skill_row_name<'a>(&self, menu: &'a [(u8, String)], row: usize) -> Option<&'a str> {
        let skill = self.skills.get(row)?.skill?;
        menu.iter()
            .find(|(known, _)| *known == skill)
            .map(|(_, name)| name.as_str())
    }

    /// The words about the start town picked, with no tags.
    pub fn town_about(&self, files: &CreationFiles) -> String {
        self.towns()
            .get(self.town)
            .map(|town| plain_words(&files.town_words(town, self.town)))
            .unwrap_or_default()
    }

    /// Every choice of the character, row by row, as the last page shows
    /// them before Create.
    pub fn summary_rows(&self, files: &CreationFiles) -> [SummaryRow; 6] {
        let hair = self
            .hair_styles()
            .get(self.hair)
            .map_or(String::new(), |style| {
                format!(", {WORDS_HAIR}: {}", style.words)
            });
        let beard = self
            .beard_styles()
            .and_then(|styles| styles.get(self.beard))
            .map_or(String::new(), |style| {
                format!(", {WORDS_BEARD}: {}", style.words)
            });
        let profession = self
            .profession
            .as_ref()
            .map_or(String::new(), |profession| {
                profession_name(profession, files)
            });
        let (stats, skills) = self.trade_words(files);
        let town = self
            .towns()
            .get(self.town)
            .map_or(String::new(), |town| match town.place {
                Some(place) => format!(
                    "{}, {} ({})",
                    town.name,
                    town.building,
                    super::facet_name(place.map)
                ),
                None => format!("{}, {}", town.name, town.building),
            });
        let row = |label, value| SummaryRow { label, value };
        [
            row(WORDS_LOOK, format!("{}{hair}{beard}", self.body_words())),
            row(WORDS_COLORS, String::new()),
            row(WORDS_TRADE, profession),
            row(WORDS_STATS, stats),
            row(WORDS_SKILLS, skills),
            row(WORDS_START, town),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::creation::{sample_choices, Blocker, NameFault};
    use uoterm_protocol::ClientVersion;

    fn sample() -> (Creation, CreationFiles) {
        (
            Creation::new(ClientVersion::MODERN, sample_choices()),
            CreationFiles::sample(),
        )
    }

    #[test]
    fn the_words_of_the_client_lose_their_tags_and_a_card_keeps_one_sentence() {
        assert_eq!(
            plain_words("<b>Yew</b> is a town.<BR>In the woods."),
            "Yew is a town.\nIn the woods."
        );
        assert_eq!(plain_words("<br/>plain"), "plain");
        assert_eq!(first_sentence("A mage. Casts spells."), "A mage.");
        assert_eq!(first_sentence("One line\nTwo"), "One line");
        assert_eq!(card_short("A mage. Casts spells.", false), "A mage.");
        assert_eq!(card_short("A mage.", true), WORDS_NEEDS_SAMURAI);
    }

    #[test]
    fn a_profession_in_capitals_is_named_as_words_and_advanced_is_custom() {
        assert_eq!(name_case("SAMURAI"), "Samurai");
        assert_eq!(name_case("ANIMAL TAMER"), "Animal Tamer");
        assert_eq!(name_case("Warrior"), "Warrior");
        let (_, files) = sample();
        let top = files.professions.top();
        assert_eq!(profession_name(top[0], &files), "Warrior");
        let custom = top.last().copied().expect("Advanced");
        assert_eq!(profession_name(custom, &files), WORDS_CUSTOM);
        assert_eq!(profession_about(custom, &files), WORDS_CUSTOM_ABOUT);
    }

    #[test]
    fn each_part_of_the_creation_stands_done_current_or_ahead() {
        assert_eq!(Stage::Look.progress(Stage::Town), Progress::Done);
        assert_eq!(Stage::Town.progress(Stage::Town), Progress::Current);
        assert_eq!(Stage::Name.progress(Stage::Town), Progress::Ahead);
    }

    #[test]
    fn the_caption_names_the_character_once_he_has_a_name() {
        let (mut creation, files) = sample();
        assert_eq!(
            creation.caption(&files),
            ("Human man".to_string(), String::new())
        );
        creation.pick_profession(files.professions.top()[0], &files);
        creation.name = "Mara".into();
        creation.set_female(true);
        assert_eq!(
            creation.caption(&files),
            ("Mara".to_string(), "Human woman, Warrior".to_string())
        );
        let [stats, skills] = creation.profession_details(&files).unwrap();
        assert_eq!(stats, "Stats: Str 45  ·  Int 10  ·  Dex 35");
        assert!(skills.starts_with("Skills: Tactics 30"), "{skills}");
    }

    #[test]
    fn the_footer_tells_why_next_waits_and_the_last_page_creates() {
        let (mut creation, _) = sample();
        assert_eq!(creation.footer_words(), WORDS_KEYS);
        assert_eq!(creation.next_words(), WORDS_NEXT);
        creation.step = Step::Name;
        assert_eq!(
            creation.footer_words(),
            Blocker::Name(NameFault::Empty).words()
        );
        assert_eq!(creation.name_verdict(), NameFault::Empty.words());
        assert_eq!(creation.next_words(), WORDS_CREATE);
        creation.name = "Mara".into();
        assert_eq!(creation.name_verdict(), WORDS_NAME_GOOD);
    }

    #[test]
    fn a_value_at_an_end_of_its_range_is_noted_by_its_name() {
        let (mut creation, files) = sample();
        assert_eq!(
            creation.stat_limit_note().as_deref(),
            Some("Strength is at its most: 60.")
        );
        creation.stats = [30, 30, 30];
        assert_eq!(creation.stat_limit_note(), None);
        creation.skills[0].value = 0;
        assert_eq!(
            creation.skill_limit_note(&files).as_deref(),
            Some("Skill 1 is at its least: 0.")
        );
        creation.set_skill(0, 27);
        assert_eq!(
            creation.skill_limit_note(&files).as_deref(),
            Some("Tactics is at its least: 0.")
        );
        assert_eq!(total_words(90, 90), "Total 90 of 90");
    }

    #[test]
    fn a_skill_of_another_row_shows_taken() {
        let (mut creation, files) = sample();
        let menu = creation.skill_menu(&files);
        creation.set_skill(0, 27);
        let choices = creation.skill_choices(&menu, 1, "tac");
        assert_eq!(choices.len(), 1);
        assert!(choices[0].taken && !choices[0].picked);
        assert_eq!(choices[0].words, "Tactics (taken)");
        let own = creation.skill_choices(&menu, 0, "tac");
        assert!(own[0].picked && !own[0].taken);
        assert_eq!(creation.skill_row_name(&menu, 0), Some("Tactics"));
        assert_eq!(creation.skill_row_name(&menu, 1), None);
    }

    #[test]
    fn the_summary_holds_every_choice_and_a_gargoyle_wears_a_robe() {
        let (mut creation, files) = sample();
        creation.set_town(2);
        let rows = creation.summary_rows(&files);
        assert_eq!(rows[0].value, "Human man, Hair: Short, Beard: None");
        assert_eq!(rows[5].value, "Britain, Sweet Dreams Inn (Trammel)");
        assert_eq!(
            creation.shown_paints(),
            [
                Paint::Skin,
                Paint::Hair,
                Paint::Beard,
                Paint::Shirt,
                Paint::Pants
            ]
        );
        assert_eq!(paint_words(Paint::Shirt, Race::Gargoyle), "Robe");
        assert_eq!(race_words(Race::Elf, false), "Elf (locked)");
        assert_eq!(creation.town_about(&files), "Sweet Dreams Inn");
    }
}
