//! The rows of the grouped skills list, and the sort of the table.

use crate::frame::WatchSkill;
use crate::model::skills::SkillSort;
use crate::settings::SkillGroupSet;

/// One row of the grouped list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillEntry {
    Group(usize),
    Skill(u16),
}

/// The rows of the grouped list: each group, and the skills of an open one.
pub fn skill_entries(groups: &[SkillGroupSet], skills: &[WatchSkill]) -> Vec<SkillEntry> {
    let mut rows = Vec::new();
    for (at, group) in groups.iter().enumerate() {
        rows.push(SkillEntry::Group(at));
        if group.open {
            rows.extend(
                group
                    .skills
                    .iter()
                    .filter(|id| skills.iter().any(|skill| skill.id == **id))
                    .map(|id| SkillEntry::Skill(*id)),
            );
        }
    }
    rows
}

/// The sort after a click on a column: the same column turns round, a new
/// one sorts from the least.
pub fn next_sort(now: (SkillSort, bool), clicked: SkillSort) -> (SkillSort, bool) {
    if now.0 == clicked {
        (clicked, !now.1)
    } else {
        (clicked, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: u16) -> WatchSkill {
        WatchSkill {
            id,
            ..WatchSkill::default()
        }
    }

    #[test]
    fn an_open_group_lists_its_known_skills_and_a_folded_one_none() {
        let groups = vec![
            SkillGroupSet {
                name: "Open".into(),
                skills: vec![1, 2, 99],
                open: true,
            },
            SkillGroupSet {
                name: "Folded".into(),
                skills: vec![3],
                open: false,
            },
        ];
        let skills = [skill(1), skill(2), skill(3)];
        assert_eq!(
            skill_entries(&groups, &skills),
            vec![
                SkillEntry::Group(0),
                SkillEntry::Skill(1),
                SkillEntry::Skill(2),
                SkillEntry::Group(1)
            ]
        );
    }

    #[test]
    fn a_column_sorts_from_the_least_and_turns_round_on_a_second_click() {
        assert_eq!(
            next_sort((SkillSort::Name, false), SkillSort::Cap),
            (SkillSort::Cap, false)
        );
        assert_eq!(
            next_sort((SkillSort::Cap, false), SkillSort::Cap),
            (SkillSort::Cap, true)
        );
    }
}
