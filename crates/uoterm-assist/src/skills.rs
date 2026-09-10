//! Skills by name. The names come from the client files; this adds the short
//! names players use for them.

use std::collections::HashMap;

use crate::name_key;

/// Short names players type, and the full skill name each one means.
const ALIASES: &[(&str, &str)] = &[
    ("Detect Hidden", "Detecting Hidden"),
    ("Item ID", "Item Identification"),
    ("Eval Int", "Evaluating Intelligence"),
    ("Taste ID", "Taste Identification"),
    ("Forensics", "Forensic Evaluation"),
    ("Resist", "Resisting Spells"),
    ("Bowcraft", "Bowcraft/Fletching"),
    ("Fletching", "Bowcraft/Fletching"),
    ("Lockpick", "Lockpicking"),
    ("Mace", "Mace Fighting"),
    ("Swords", "Swordsmanship"),
    ("Vet", "Veterinary"),
    ("Taming", "Animal Taming"),
    ("Peace", "Peacemaking"),
    ("Provo", "Provocation"),
    ("Discord", "Discordance"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub id: u16,
    pub name: String,
    /// A player can use it from its button.
    pub usable: bool,
}

/// Every skill the client files name, found by number or by name.
#[derive(Clone, Debug, Default)]
pub struct SkillBook {
    skills: Vec<Skill>,
    by_name: HashMap<String, usize>,
}

impl SkillBook {
    /// Builds the book from the client's skill list, in number order.
    pub fn new(entries: impl IntoIterator<Item = (String, bool)>) -> Self {
        let skills: Vec<Skill> = entries
            .into_iter()
            .enumerate()
            .map(|(id, (name, usable))| Skill {
                id: id as u16,
                name,
                usable,
            })
            .collect();
        let mut by_name: HashMap<String, usize> = skills
            .iter()
            .enumerate()
            .map(|(i, s)| (name_key(&s.name), i))
            .collect();
        for (alias, full) in ALIASES {
            if let Some(&i) = by_name.get(&name_key(full)) {
                by_name.entry(name_key(alias)).or_insert(i);
            }
        }
        Self { skills, by_name }
    }

    pub fn by_name(&self, name: &str) -> Option<&Skill> {
        self.by_name.get(&name_key(name)).map(|&i| &self.skills[i])
    }

    pub fn by_id(&self, id: u16) -> Option<&Skill> {
        self.skills.get(usize::from(id))
    }

    /// A skill named by a number or by a name, as a script writes it.
    pub fn find(&self, number_or_name: &str) -> Option<&Skill> {
        let text = number_or_name.trim();
        match text.parse::<u16>() {
            Ok(id) => self.by_id(id),
            Err(_) => self.by_name(text),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book() -> SkillBook {
        SkillBook::new(
            [
                ("Alchemy", false),
                ("Anatomy", true),
                ("Detecting Hidden", true),
            ]
            .into_iter()
            .map(|(n, u)| (n.to_string(), u)),
        )
    }

    #[test]
    fn a_skill_is_found_by_name_short_name_or_number() {
        let book = book();
        assert_eq!(book.by_name("anatomy").map(|s| s.id), Some(1));
        assert_eq!(book.find("Detect Hidden").map(|s| s.id), Some(2));
        assert_eq!(book.find("0").map(|s| s.name.as_str()), Some("Alchemy"));
        assert!(book.find("Eval Int").is_none(), "not in these files");
    }

    #[test]
    fn a_book_knows_which_skills_have_a_button() {
        let book = book();
        assert!(!book.by_id(0).expect("alchemy").usable);
        assert!(book.by_id(1).expect("anatomy").usable);
    }
}
