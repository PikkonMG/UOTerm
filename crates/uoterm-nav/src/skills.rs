//! The skill names in the client files, in skill-number order.
//!
//! A "use skill" request names a skill by its number. The client files hold
//! the name of each number, and whether a player can use that skill from its
//! button. A shard with skills of its own ships its own files, so the names
//! are read, never written into the code.

use std::path::Path;

use crate::mul::{first_existing, read_file, slice_at, MapError};

/// Client directories do not agree on the case of these names.
const SKILLS_IDX_NAMES: [&str; 2] = ["Skills.idx", "skills.idx"];
const SKILLS_MUL_NAMES: [&str; 2] = ["skills.mul", "Skills.mul"];

/// One index entry: offset, length, and a field this reader does not use.
const IDX_ENTRY_LEN: usize = 12;
/// An entry with no record behind it has a negative offset.
const NO_RECORD: i32 = -1;
/// A record opens with one byte: 1 when the skill has a use button.
const USABLE_FLAG_LEN: usize = 1;
const USABLE: u8 = 1;

/// One skill: its number is its place in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillEntry {
    pub name: String,
    /// A player can use this skill from its button, as with Hiding. A skill
    /// such as Magery is used by doing something else.
    pub usable: bool,
}

/// Reads every skill the client files name, in number order. The list stops
/// at the first number with no record, because the numbers after it are
/// unused slots.
pub fn read_skills(uopath: impl AsRef<Path>) -> Result<Vec<SkillEntry>, MapError> {
    let dir = uopath.as_ref();
    let idx_path =
        first_existing(dir, &SKILLS_IDX_NAMES).ok_or(MapError::Missing(SKILLS_IDX_NAMES[0]))?;
    let mul_path =
        first_existing(dir, &SKILLS_MUL_NAMES).ok_or(MapError::Missing(SKILLS_MUL_NAMES[0]))?;
    parse_skills(&read_file(&idx_path)?, &read_file(&mul_path)?)
}

fn parse_skills(idx: &[u8], mul: &[u8]) -> Result<Vec<SkillEntry>, MapError> {
    let mut skills = Vec::new();
    for entry in idx.chunks_exact(IDX_ENTRY_LEN) {
        let offset = i32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]);
        let length = i32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
        if offset <= NO_RECORD || length <= USABLE_FLAG_LEN as i32 {
            break;
        }
        let record = slice_at(mul, offset as usize, length as usize).ok_or(MapError::Truncated)?;
        let text = &record[USABLE_FLAG_LEN..];
        let name_end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
        skills.push(SkillEntry {
            name: String::from_utf8_lossy(&text[..name_end]).into_owned(),
            usable: record[0] == USABLE,
        });
    }
    if skills.is_empty() {
        return Err(MapError::Truncated);
    }
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(usable: u8, name: &str) -> Vec<u8> {
        let mut r = vec![usable];
        r.extend_from_slice(name.as_bytes());
        r.push(0);
        r
    }

    fn idx_entry(offset: i32, length: i32) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend_from_slice(&offset.to_le_bytes());
        e.extend_from_slice(&length.to_le_bytes());
        e.extend_from_slice(&0i32.to_le_bytes());
        e
    }

    #[test]
    fn skills_are_read_in_number_order_up_to_the_first_gap() {
        let alchemy = record(0, "Alchemy");
        let anatomy = record(USABLE, "Anatomy");
        let mut mul = alchemy.clone();
        mul.extend_from_slice(&anatomy);
        let mut idx = idx_entry(0, alchemy.len() as i32);
        idx.extend(idx_entry(alchemy.len() as i32, anatomy.len() as i32));
        idx.extend(idx_entry(NO_RECORD, 0));
        idx.extend(idx_entry(0, alchemy.len() as i32));

        let skills = parse_skills(&idx, &mul).expect("two skills");
        assert_eq!(
            skills,
            vec![
                SkillEntry {
                    name: "Alchemy".into(),
                    usable: false
                },
                SkillEntry {
                    name: "Anatomy".into(),
                    usable: true
                },
            ]
        );
    }

    /// The real client files, when a test run points at them.
    #[test]
    fn the_client_files_name_the_standard_skills() {
        const MAGERY: usize = 25;
        const HIDING: usize = 21;
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let skills = read_skills(&dir).expect("client skill files");
        assert_eq!(skills[MAGERY].name, "Magery");
        assert!(!skills[MAGERY].usable);
        assert!(skills[HIDING].usable);
    }

    #[test]
    fn a_record_past_the_end_of_the_file_is_an_error() {
        const PAST_THE_END: i32 = 100;
        let idx = idx_entry(PAST_THE_END, 8);
        assert!(parse_skills(&idx, &record(0, "Alchemy")).is_err());
    }
}
