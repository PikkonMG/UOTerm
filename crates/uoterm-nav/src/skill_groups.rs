//! The groups the skill list shows its skills under, from `skillgrp.mul`:
//! Combat, Magic, Wilderness and the rest.
//!
//! The file opens with the number of groups. A count of -1 marks names in
//! UTF-16, and the true count follows it. Then come the names of every group
//! but the first, each in a field of fixed width, and then one group number
//! for each skill, in skill-number order. The first group has no name in the
//! file: the reference client calls it Miscellaneous.

use std::path::Path;

use crate::mul::{first_existing, read_file, MapError};

/// Client directories do not agree on the case of the name.
const SKILL_GROUPS_NAMES: [&str; 2] = ["skillgrp.mul", "Skillgrp.mul"];
/// The count that marks UTF-16 names.
const UNICODE_MARK: i32 = -1;
/// A number in the file: the count and each group number.
const NUMBER_LEN: usize = 4;
/// The width of one group name, in characters.
const NAME_CHARS: usize = 17;
/// The bytes of one UTF-16 character.
const UNICODE_CHAR_LEN: usize = 2;
/// The name of the first group, which the file does not hold.
pub const FIRST_SKILL_GROUP: &str = "Miscellaneous";

/// One group of the skill list and the numbers of its skills.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillGroup {
    pub name: String,
    pub skills: Vec<u16>,
}

/// Reads the skill groups of the client files.
pub fn read_skill_groups(uopath: impl AsRef<Path>) -> Result<Vec<SkillGroup>, MapError> {
    let path = first_existing(uopath.as_ref(), &SKILL_GROUPS_NAMES)
        .ok_or(MapError::Missing(SKILL_GROUPS_NAMES[0]))?;
    parse_skill_groups(&read_file(&path)?)
}

fn number_at(data: &[u8], at: usize) -> Result<i32, MapError> {
    let bytes = data.get(at..at + NUMBER_LEN).ok_or(MapError::Truncated)?;
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// The name in one fixed field, up to the first zero character.
fn name_in(field: &[u8], unicode: bool) -> String {
    if unicode {
        let units: Vec<u16> = field
            .chunks_exact(UNICODE_CHAR_LEN)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|&unit| unit != 0)
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
        field[..end].iter().map(|&b| char::from(b)).collect()
    }
}

fn parse_skill_groups(data: &[u8]) -> Result<Vec<SkillGroup>, MapError> {
    let mut count = number_at(data, 0)?;
    let unicode = count == UNICODE_MARK;
    let mut at = NUMBER_LEN;
    if unicode {
        count = number_at(data, at)?;
        at += NUMBER_LEN;
    }
    let count = usize::try_from(count).map_err(|_| MapError::Truncated)?;
    let field = if unicode {
        NAME_CHARS * UNICODE_CHAR_LEN
    } else {
        NAME_CHARS
    };
    let mut groups = vec![SkillGroup {
        name: FIRST_SKILL_GROUP.to_string(),
        skills: Vec::new(),
    }];
    for _ in 1..count {
        let name = data.get(at..at + field).ok_or(MapError::Truncated)?;
        groups.push(SkillGroup {
            name: name_in(name, unicode),
            skills: Vec::new(),
        });
        at += field;
    }
    // A group number past the list is skipped and takes no skill number,
    // as the reference client reads the file.
    let mut skill = 0u16;
    for number in data[at.min(data.len())..].chunks_exact(NUMBER_LEN) {
        let group = i32::from_le_bytes([number[0], number[1], number[2], number[3]]);
        if let Some(group) = usize::try_from(group).ok().and_then(|g| groups.get_mut(g)) {
            group.skills.push(skill);
            skill += 1;
        }
    }
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_data_dir_from_env;

    fn field(name: &str) -> Vec<u8> {
        let mut bytes = name.as_bytes().to_vec();
        bytes.resize(NAME_CHARS, 0);
        bytes
    }

    #[test]
    fn groups_take_their_names_and_their_skills_in_order() {
        let mut data = 3i32.to_le_bytes().to_vec();
        data.extend(field("Combat"));
        data.extend(field("Magic"));
        for group in [0i32, 1, 2, 1] {
            data.extend(group.to_le_bytes());
        }
        let groups = parse_skill_groups(&data).unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, FIRST_SKILL_GROUP);
        assert_eq!(groups[0].skills, vec![0]);
        assert_eq!(groups[1].name, "Combat");
        assert_eq!(groups[1].skills, vec![1, 3]);
        assert_eq!(groups[2].name, "Magic");
    }

    #[test]
    fn unicode_names_follow_the_mark() {
        let mut data = UNICODE_MARK.to_le_bytes().to_vec();
        data.extend(2i32.to_le_bytes());
        let mut name: Vec<u8> = "Bard".encode_utf16().flat_map(u16::to_le_bytes).collect();
        name.resize(NAME_CHARS * UNICODE_CHAR_LEN, 0);
        data.extend(name);
        data.extend(1i32.to_le_bytes());
        let groups = parse_skill_groups(&data).unwrap();
        assert_eq!(groups[1].name, "Bard");
        assert_eq!(groups[1].skills, vec![0]);
    }

    #[test]
    fn a_cut_file_is_refused() {
        let mut data = 3i32.to_le_bytes().to_vec();
        data.extend(field("Combat"));
        assert!(matches!(
            parse_skill_groups(&data),
            Err(MapError::Truncated)
        ));
    }

    #[test]
    fn the_client_files_group_every_skill() {
        let Some(dir) = client_data_dir_from_env() else {
            eprintln!("skipped: UOTERM_TEST_UOPATH is not set");
            return;
        };
        let groups = read_skill_groups(&dir).unwrap();
        let combat = groups.iter().find(|group| group.name == "Combat");
        assert!(combat.is_some_and(|group| !group.skills.is_empty()));
        let mut grouped: Vec<u16> = groups.iter().flat_map(|g| g.skills.clone()).collect();
        grouped.sort_unstable();
        grouped.dedup();
        assert_eq!(grouped.first(), Some(&0));
        assert_eq!(
            grouped.len(),
            usize::from(*grouped.last().unwrap()) + 1,
            "each skill is in one group"
        );
    }
}
