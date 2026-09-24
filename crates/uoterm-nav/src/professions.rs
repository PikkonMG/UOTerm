//! The professions a new character may start as (`Prof.txt`), and the words
//! about each start town of an older client (`citytext.enu`).
//!
//! `Prof.txt` holds blocks from `Begin` to `End`. Each line of a block is a
//! key and its values: the name, the text numbers of the name and of the
//! words about it, the picture, the starting skills and stats, and for a
//! category the professions under it. The client adds the Advanced choice,
//! where the player sets the skills and the stats himself.

use std::path::Path;

use crate::mul::{first_existing, read_file, MapError};

/// Client directories do not agree on the case of these names.
const PROF_NAMES: [&str; 2] = ["Prof.txt", "prof.txt"];
const CITY_TEXT_NAMES: [&str; 2] = ["citytext.enu", "Citytext.enu"];

const COMMENT_MARKS: [char; 2] = ['#', ';'];
const QUOTE: char = '"';
const KEY_BEGIN: &str = "begin";
const KEY_END: &str = "end";
const KEY_NAME: &str = "name";
const KEY_TRUE_NAME: &str = "truename";
const KEY_DESC: &str = "desc";
const KEY_TOP_LEVEL: &str = "toplevel";
const KEY_GUMP: &str = "gump";
const KEY_TYPE: &str = "type";
const KEY_CHILDREN: &str = "children";
const KEY_SKILL: &str = "skill";
const KEY_STAT: &str = "stat";
const KEY_NAME_ID: &str = "nameid";
const KEY_DESC_ID: &str = "descid";
const WORD_TRUE: &str = "true";
const WORD_CATEGORY: &str = "category";
const STAT_STR: &str = "str";
const STAT_DEX: &str = "dex";
const STAT_INT: &str = "int";

/// The Advanced choice the client adds after the professions of the file.
const ADVANCED_NAME: &str = "Advanced";
const ADVANCED_NAME_ID: u32 = 1_061_176;
const ADVANCED_DESC_ID: u32 = 1_061_226;
const ADVANCED_GUMP: u16 = 5545;
/// The words-number of the Advanced choice: it has no set template.
pub const ADVANCED_DESCRIPTION_INDEX: i32 = -1;

/// The skill names the client also knows each skill by, in skill-number
/// order. A profession file may name
/// a skill so, as "Blacksmith" for Blacksmithy.
const SKILL_CODE_NAMES: [&str; 58] = [
    "Alchemy",
    "Anatomy",
    "AnimalLore",
    "ItemID",
    "ArmsLore",
    "Parrying",
    "Begging",
    "Blacksmith",
    "Bowcraft",
    "Peacemaking",
    "Camping",
    "Carpentry",
    "Cartography",
    "Cooking",
    "DetectHidden",
    "Enticement",
    "EvaluateIntelligence",
    "Healing",
    "Fishing",
    "ForensicEvaluation",
    "Herding",
    "Hiding",
    "Provocation",
    "Inscription",
    "Lockpicking",
    "Magery",
    "ResistingSpells",
    "Tactics",
    "Snooping",
    "Musicianship",
    "Poisoning",
    "Archery",
    "SpiritSpeak",
    "Stealing",
    "Tailoring",
    "AnimalTaming",
    "TasteIdentification",
    "Tinkering",
    "Tracking",
    "Veterinary",
    "Swordsmanship",
    "MaceFighting",
    "Fencing",
    "Wrestling",
    "Lumberjacking",
    "Mining",
    "Meditation",
    "Stealth",
    "RemoveTrap",
    "Necromancy",
    "Focus",
    "Chivalry",
    "Bushido",
    "Ninjitsu",
    "Spellweaving",
    "Mysticism",
    "Imbuing",
    "Throwing",
];

// The words of a start town in `citytext.enu`.
const CITY_MARK: &[u8; 4] = b"END\0";
const CITY_TEXT_START: u8 = b'<';
const CITY_TEXT_END: u8 = b'.';
const PARAGRAPH: &str = "\n\n";

/// Whether a block is a group of professions or one profession.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfessionKind {
    Category,
    Profession,
}

/// One profession, or one category of professions, of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profession {
    pub name: String,
    pub true_name: String,
    /// The text numbers of the name and of the words about it.
    pub name_id: u32,
    pub description_id: u32,
    /// The number the client sends for the profession. Advanced has
    /// [`ADVANCED_DESCRIPTION_INDEX`].
    pub description_index: i32,
    /// The gump picture that shows it.
    pub gump: u16,
    pub top_level: bool,
    pub kind: ProfessionKind,
    /// The true names of the professions of a category.
    pub children: Vec<String>,
    /// Each starting skill as the file names it, and its value.
    pub skills: Vec<(String, u8)>,
    /// The starting strength, dexterity and intelligence.
    pub stats: [u8; 3],
}

impl Profession {
    /// The Advanced choice: the player sets the skills and the stats.
    pub fn is_advanced(&self) -> bool {
        self.description_index == ADVANCED_DESCRIPTION_INDEX
    }

    /// The starting skills by number, as the skill list of the client
    /// names them. A skill the list does not know is left out.
    pub fn skill_numbers(&self, skill_names: &[String]) -> Vec<(u8, u8)> {
        self.skills
            .iter()
            .filter_map(|(name, value)| Some((skill_number(name, skill_names)?, *value)))
            .collect()
    }

    /// A block of the file before its lines are read.
    fn empty() -> Self {
        Self {
            name: String::new(),
            true_name: String::new(),
            name_id: 0,
            description_id: 0,
            description_index: 0,
            gump: 0,
            top_level: false,
            kind: ProfessionKind::Profession,
            children: Vec::new(),
            skills: Vec::new(),
            stats: [0; 3],
        }
    }

    fn advanced() -> Self {
        Self {
            name: ADVANCED_NAME.to_string(),
            true_name: ADVANCED_NAME.to_ascii_lowercase(),
            name_id: ADVANCED_NAME_ID,
            description_id: ADVANCED_DESC_ID,
            description_index: ADVANCED_DESCRIPTION_INDEX,
            gump: ADVANCED_GUMP,
            top_level: true,
            ..Self::empty()
        }
    }
}

/// The number of a skill a profession file names: the name of the client's
/// list, or the name the client also knows it by, with or without spaces.
fn skill_number(name: &str, skill_names: &[String]) -> Option<u8> {
    let bare = |words: &str| -> String {
        words
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase()
    };
    let wanted = bare(name);
    skill_names
        .iter()
        .position(|known| bare(known) == wanted)
        .or_else(|| {
            SKILL_CODE_NAMES
                .iter()
                .position(|code| code.eq_ignore_ascii_case(&wanted))
                .filter(|number| *number < skill_names.len())
        })
        .and_then(|number| u8::try_from(number).ok())
}

/// The professions of the client: the top level in file order with Advanced
/// last, and the professions of each category.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionList {
    all: Vec<Profession>,
}

impl ProfessionList {
    /// The choices of the first screen.
    pub fn top(&self) -> Vec<&Profession> {
        self.all.iter().filter(|p| p.top_level).collect()
    }

    /// The professions of a category, in file order.
    pub fn children(&self, category: &Profession) -> Vec<&Profession> {
        self.all
            .iter()
            .filter(|p| !p.top_level && category.children.contains(&p.true_name))
            .collect()
    }
}

/// Reads `Prof.txt`. A client with no such file offers only Advanced.
pub fn read_professions(uopath: impl AsRef<Path>) -> ProfessionList {
    let text = first_existing(uopath.as_ref(), &PROF_NAMES)
        .and_then(|path| read_file(&path).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    parse_professions(&text)
}

/// The words of a line, with comments cut off and quotes taken away. Words
/// in quotes stay one word.
fn tokens(line: &str) -> Vec<String> {
    let line = line
        .find(|c| COMMENT_MARKS.contains(&c))
        .map_or(line, |at| &line[..at]);
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for c in line.chars() {
        if c == QUOTE {
            quoted = !quoted;
        } else if (c.is_whitespace() || c == ',') && !quoted {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// The professions of the words of a `Prof.txt`, and Advanced.
pub fn parse_professions(text: &str) -> ProfessionList {
    let mut all = Vec::new();
    let mut block: Option<Profession> = None;
    let mut kind = None;
    for line in text.lines() {
        let words = tokens(line);
        let Some(key) = words.first().map(|w| w.to_ascii_lowercase()) else {
            continue;
        };
        let value = words.get(1).cloned().unwrap_or_default();
        if key == KEY_BEGIN {
            block = Some(Profession::empty());
            kind = None;
            continue;
        }
        let Some(profession) = block.as_mut() else {
            continue;
        };
        match key.as_str() {
            KEY_END => {
                if let (Some(kind), Some(mut done)) = (kind, block.take()) {
                    done.kind = kind;
                    all.push(done);
                }
            }
            KEY_NAME => profession.name = value,
            KEY_TRUE_NAME => profession.true_name = value,
            KEY_DESC => profession.description_index = value.parse().unwrap_or(0),
            KEY_TOP_LEVEL => profession.top_level = value.eq_ignore_ascii_case(WORD_TRUE),
            KEY_GUMP => profession.gump = value.parse().unwrap_or(0),
            KEY_TYPE => {
                kind = Some(if value.eq_ignore_ascii_case(WORD_CATEGORY) {
                    ProfessionKind::Category
                } else {
                    ProfessionKind::Profession
                });
            }
            KEY_CHILDREN => profession.children = words[1..].to_vec(),
            KEY_SKILL if words.len() > 2 => {
                let points = words[words.len() - 1].parse().unwrap_or(0);
                let name = words[1..words.len() - 1].join(" ");
                profession.skills.push((name, points));
            }
            KEY_STAT if words.len() > 2 => {
                let points = words[2].parse().unwrap_or(0);
                let at = match value.to_ascii_lowercase().as_str() {
                    STAT_STR => 0,
                    STAT_DEX => 1,
                    STAT_INT => 2,
                    _ => continue,
                };
                profession.stats[at] = points;
            }
            KEY_NAME_ID => profession.name_id = value.parse().unwrap_or(0),
            KEY_DESC_ID => profession.description_id = value.parse().unwrap_or(0),
            _ => {}
        }
    }
    all.push(Profession::advanced());
    ProfessionList { all }
}

/// Reads the words about each start town from `citytext.enu`, in the order
/// of the towns. A client with no such file has none.
pub fn read_city_texts(uopath: impl AsRef<Path>) -> Result<Vec<String>, MapError> {
    let path = first_existing(uopath.as_ref(), &CITY_TEXT_NAMES)
        .ok_or(MapError::Missing(CITY_TEXT_NAMES[0]))?;
    Ok(parse_city_texts(&read_file(&path)?))
}

/// Each town starts with `END\0`, then its name up to the first `<`, then
/// pieces of words that each end with a zero byte, up to a `.` or the next
/// `END\0`.
fn parse_city_texts(data: &[u8]) -> Vec<String> {
    let latin = |bytes: &[u8]| bytes.iter().map(|b| char::from(*b)).collect::<String>();
    let mut texts = Vec::new();
    let mut at = 0;
    while at + CITY_MARK.len() <= data.len() {
        if &data[at..at + CITY_MARK.len()] != CITY_MARK {
            at += 1;
            continue;
        }
        at += CITY_MARK.len();
        while at < data.len() && data[at] != CITY_TEXT_START {
            at += 1;
        }
        let mut text = String::new();
        while at < data.len() {
            let end = data[at..]
                .iter()
                .position(|b| *b == 0)
                .map_or(data.len(), |n| at + n);
            let piece = latin(&data[at..end]);
            at = (end + 1).min(data.len());
            if !piece.is_empty() {
                text.push_str(&piece);
                text.push_str(PARAGRAPH);
            }
            let next_is_mark = data
                .get(at..at + CITY_MARK.len())
                .is_some_and(|next| next == CITY_MARK);
            if data.get(at) == Some(&CITY_TEXT_END) || next_is_mark {
                break;
            }
        }
        texts.push(text);
    }
    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
# A comment line
Begin
	Name			Warrior
	TrueName		"Warrior"
	NameId			1061180
	DescId			1061230
	Desc			1
	TopLevel		true
	Gump			5577
	Type			Profession
	Skill			Tactics			30
	Skill			Evaluate Intelligence	30
	Skill			Blacksmith		30
	Skill			Unknown Skill		30
	Stat			Str			45
	Stat			Dex			35
	Stat			Int			10
End

Begin
	Name			Crafters
	TrueName		"crafters"
	TopLevel		true
	Type			Category
	Children		smith
End

Begin
	Name			Smith
	TrueName		"smith"
	Desc			3
	Type			Profession
End
"#;

    fn skill_names() -> Vec<String> {
        let mut names: Vec<String> = SKILL_CODE_NAMES.iter().map(|s| s.to_string()).collect();
        names[7] = "Blacksmithy".into();
        names[16] = "Evaluating Intelligence".into();
        names[27] = "Tactics".into();
        names
    }

    #[test]
    fn professions_are_read_with_their_skills_stats_and_advanced_last() {
        let list = parse_professions(FILE);
        let top = list.top();
        assert_eq!(
            top.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            vec!["Warrior", "Crafters", ADVANCED_NAME]
        );
        let warrior = top[0];
        assert_eq!(
            (warrior.name_id, warrior.description_id, warrior.gump),
            (1_061_180, 1_061_230, 5577)
        );
        assert_eq!(warrior.description_index, 1);
        assert_eq!(warrior.stats, [45, 35, 10]);
        // "Evaluate Intelligence" is a skill of two words; "Blacksmith" is
        // the other name of Blacksmithy.
        assert_eq!(
            warrior.skill_numbers(&skill_names()),
            vec![(27, 30), (16, 30), (7, 30)]
        );
        assert_eq!(top[1].kind, ProfessionKind::Category);
        let children = list.children(top[1]);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "Smith");
        assert!(top[2].is_advanced());
        assert!(parse_professions("").top()[0].is_advanced());
    }

    #[test]
    fn city_texts_follow_each_mark_up_to_a_period_or_the_next_mark() {
        let mut data = Vec::new();
        data.extend_from_slice(CITY_MARK);
        data.extend_from_slice(b"Yew<i>A town</i>\0in the woods\0");
        data.extend_from_slice(CITY_MARK);
        data.extend_from_slice(b"Minoc<b>Mines</b>\0.");
        let texts = parse_city_texts(&data);
        assert_eq!(
            texts,
            vec![
                "<i>A town</i>\n\nin the woods\n\n".to_string(),
                "<b>Mines</b>\n\n".to_string()
            ]
        );
    }

    #[test]
    fn the_client_files_list_the_professions() {
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let list = read_professions(&dir);
        assert!(list.top().len() > 1);
        assert!(list.top().last().is_some_and(|p| p.is_advanced()));
    }
}
