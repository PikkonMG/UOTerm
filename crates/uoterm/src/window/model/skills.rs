//! The skills of the character, apart from how a window draws them: their
//! values in points, the lock after the one a skill has, the groups of the
//! skills gump and how the player changes them, the order of the advanced
//! skills gump, the sum of the values, and the messages that tell of a
//! skill that changed, by the General options.

use crate::view::{WatchFrame, WatchSkill};
use crate::window::settings::{GeneralOptions, SkillGroupSet};
use std::cmp::Ordering;
use std::collections::HashMap;
use uoterm_nav::FIRST_SKILL_GROUP;

const SKILL_TENTHS: u16 = 10;
const TENTHS_PER_POINT: f32 = 10.0;
/// The words of each lock, as the skill lock command takes them: up, down,
/// locked.
pub const LOCK_WORDS: [&str; 3] = ["up", "down", "locked"];
/// What the player is told when he would take away the first group.
pub const WORDS_CANNOT_DELETE: &str = "Cannot delete this group.";
/// The name of a group the player makes.
pub const NEW_GROUP: &str = "New Group";
/// The name a group takes when the player leaves it with none.
pub const NO_NAME: &str = "No Name";

/// A skill value in points, from the tenths the shard sends.
pub fn points(tenths: u16) -> String {
    format!("{}.{}", tenths / SKILL_TENTHS, tenths % SKILL_TENTHS)
}

/// A skill value in points with no tenth when it is whole, as the
/// advanced skills gump writes it: 50 and 50.5.
pub fn short_points(tenths: u16) -> String {
    if tenths.is_multiple_of(SKILL_TENTHS) {
        (tenths / SKILL_TENTHS).to_string()
    } else {
        points(tenths)
    }
}

/// The sum of the values of the skills in points: the real ones, or the
/// values with the items and the spells on.
pub fn total(skills: &[WatchSkill], real: bool) -> String {
    let tenths: u32 = skills
        .iter()
        .map(|skill| u32::from(if real { skill.base } else { skill.value }))
        .sum();
    format!("{:.1}", tenths as f32 / TENTHS_PER_POINT)
}

/// The script line that sets the lock after the one a skill has now.
pub fn next_lock_command(skill: &WatchSkill) -> String {
    let next = (usize::from(skill.lock) + 1) % LOCK_WORDS.len();
    format!("setskill '{}' {}", skill.name, LOCK_WORDS[next])
}

/// The groups of the client files: each group with the skills the frame
/// lists under it, in the order of the files. Skills with no group go in
/// the first.
pub fn file_groups(skills: &[WatchSkill]) -> Vec<SkillGroupSet> {
    let mut groups: Vec<(u16, SkillGroupSet)> = Vec::new();
    let mut sorted: Vec<&WatchSkill> = skills.iter().collect();
    sorted.sort_by_key(|skill| skill.id);
    for skill in sorted {
        let (index, name) = if skill.group.is_empty() {
            (0, FIRST_SKILL_GROUP.to_string())
        } else {
            (skill.group_index, skill.group.clone())
        };
        match groups.iter_mut().find(|(at, _)| *at == index) {
            Some((_, group)) => group.skills.push(skill.id),
            None => groups.push((
                index,
                SkillGroupSet {
                    name,
                    skills: vec![skill.id],
                    open: false,
                },
            )),
        }
    }
    groups.sort_by_key(|(index, _)| *index);
    groups.into_iter().map(|(_, group)| group).collect()
}

/// The groups the skills gump shows: the player's, with any skill none of
/// them holds in the first; or those of the client files.
pub fn shown_groups(kept: &[SkillGroupSet], skills: &[WatchSkill]) -> Vec<SkillGroupSet> {
    if kept.is_empty() {
        return file_groups(skills);
    }
    let mut groups = kept.to_vec();
    let loose: Vec<u16> = skills
        .iter()
        .map(|skill| skill.id)
        .filter(|id| !groups.iter().any(|group| group.skills.contains(id)))
        .collect();
    if let Some(first) = groups.first_mut() {
        first.skills.extend(loose);
        first.skills.sort_unstable();
    }
    groups
}

/// Moves a skill into a group, in its place by number.
pub fn move_skill(groups: &mut [SkillGroupSet], skill: u16, to: usize) {
    if to >= groups.len() || groups[to].skills.contains(&skill) {
        return;
    }
    for group in groups.iter_mut() {
        group.skills.retain(|id| *id != skill);
    }
    let target = &mut groups[to].skills;
    let at = target.partition_point(|id| *id < skill);
    target.insert(at, skill);
}

/// Takes a group away and gives its skills to the first group. The first
/// group stays: false when asked to take it.
pub fn remove_group(groups: &mut Vec<SkillGroupSet>, at: usize) -> bool {
    if at == 0 || at >= groups.len() {
        return false;
    }
    let removed = groups.remove(at);
    let first = &mut groups[0].skills;
    first.extend(removed.skills);
    first.sort_unstable();
    true
}

/// The name a group keeps after the player typed it.
pub fn group_name(typed: &str) -> String {
    let typed = typed.trim();
    if typed.is_empty() {
        NO_NAME.to_string()
    } else {
        typed.to_string()
    }
}

/// The columns the advanced skills gump sorts by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSort {
    Name,
    /// The value with no items and spells.
    Real,
    /// The value with the items and the spells on.
    Value,
    Cap,
}

impl SkillSort {
    /// The columns of a skill table, in their order.
    pub const COLUMNS: [SkillSort; 4] = [
        SkillSort::Name,
        SkillSort::Real,
        SkillSort::Value,
        SkillSort::Cap,
    ];

    /// The words of a column, as the advanced skills gump heads it: its
    /// "Base" is the value with the items and the spells on.
    pub const fn label(self) -> &'static str {
        match self {
            SkillSort::Name => "Name",
            SkillSort::Real => "Real",
            SkillSort::Value => "Base",
            SkillSort::Cap => "Cap",
        }
    }

    /// The tenths a column shows of a skill. The name column shows none.
    pub fn tenths(self, skill: &WatchSkill) -> Option<u16> {
        match self {
            SkillSort::Name => None,
            SkillSort::Real => Some(skill.base),
            SkillSort::Value => Some(skill.value),
            SkillSort::Cap => Some(skill.cap),
        }
    }
}

/// The skills in the order of a column, from the least; `descending`
/// turns it round.
pub fn sorted(skills: &[WatchSkill], by: SkillSort, descending: bool) -> Vec<&WatchSkill> {
    let mut list: Vec<&WatchSkill> = skills.iter().collect();
    let order = |a: &&WatchSkill, b: &&WatchSkill| -> Ordering {
        match by {
            SkillSort::Name => a.name.cmp(&b.name),
            SkillSort::Real => a.base.cmp(&b.base),
            SkillSort::Value => a.value.cmp(&b.value),
            SkillSort::Cap => a.cap.cmp(&b.cap),
        }
    };
    list.sort_by(order);
    if descending {
        list.reverse();
    }
    list
}

/// Tells of the skills that changed since the last frame, as the classic
/// client writes it in the journal, when the General page asks for it and
/// the change is at least as large as it asks.
#[derive(Default)]
pub struct SkillChanges {
    last: HashMap<u16, u16>,
}

impl SkillChanges {
    pub fn observe(&mut self, frame: &WatchFrame, options: &GeneralOptions) -> Vec<String> {
        let mut said = Vec::new();
        for skill in &frame.skills {
            let before = self.last.insert(skill.id, skill.value);
            let Some(before) = before.filter(|before| *before != skill.value) else {
                continue;
            };
            let change = i32::from(skill.value) - i32::from(before);
            if !options.skill_change_messages
                || change.unsigned_abs() < u32::from(options.skill_change_tenths)
            {
                continue;
            }
            let way = if change < 0 { "decreased" } else { "increased" };
            said.push(format!(
                "Your skill in {} has {way} by {:.1}.  It is now {}.",
                skill.name,
                change.unsigned_abs() as f32 / TENTHS_PER_POINT,
                points(skill.value)
            ));
        }
        said
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: u16, name: &str, group: &str, group_index: u16, value: u16) -> WatchSkill {
        WatchSkill {
            id,
            name: name.into(),
            group: group.into(),
            group_index,
            value,
            base: value,
            cap: 1000,
            ..WatchSkill::default()
        }
    }

    fn hiding(lock: u8) -> WatchSkill {
        WatchSkill {
            id: 21,
            name: "Hiding".into(),
            lock,
            ..WatchSkill::default()
        }
    }

    #[test]
    fn a_click_on_a_lock_sets_the_next_lock() {
        assert_eq!(next_lock_command(&hiding(0)), "setskill 'Hiding' down");
        assert_eq!(next_lock_command(&hiding(1)), "setskill 'Hiding' locked");
        assert_eq!(next_lock_command(&hiding(2)), "setskill 'Hiding' up");
    }

    #[test]
    fn a_skill_value_shows_in_points_and_the_total_sums_them() {
        assert_eq!(points(702), "70.2");
        assert_eq!(points(5), "0.5");
        assert_eq!(short_points(500), "50");
        assert_eq!(short_points(505), "50.5");
        let skills = [skill(0, "A", "", 0, 702), skill(1, "B", "", 0, 5)];
        assert_eq!(total(&skills, true), "70.7");
    }

    #[test]
    fn the_file_groups_follow_the_files_and_the_players_groups_take_new_skills() {
        let skills = [
            skill(0, "Alchemy", "Trade Skills", 2, 0),
            skill(1, "Anatomy", "Combat", 1, 0),
            skill(2, "Animal Lore", "", 0, 0),
        ];
        let groups = file_groups(&skills);
        let names: Vec<&str> = groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec![FIRST_SKILL_GROUP, "Combat", "Trade Skills"]);
        let kept = vec![SkillGroupSet {
            name: "Mine".into(),
            skills: vec![1],
            open: true,
        }];
        assert_eq!(shown_groups(&kept, &skills)[0].skills, vec![0, 1, 2]);
    }

    #[test]
    fn skills_move_between_groups_and_a_removed_group_gives_them_to_the_first() {
        let mut groups = vec![
            SkillGroupSet {
                name: "First".into(),
                skills: vec![1, 5],
                ..SkillGroupSet::default()
            },
            SkillGroupSet {
                name: "Second".into(),
                skills: vec![2, 9],
                ..SkillGroupSet::default()
            },
        ];
        move_skill(&mut groups, 5, 1);
        assert_eq!(groups[1].skills, vec![2, 5, 9]);
        assert_eq!(groups[0].skills, vec![1]);
        assert!(!remove_group(&mut groups, 0));
        assert!(remove_group(&mut groups, 1));
        assert_eq!(groups[0].skills, vec![1, 2, 5, 9]);
        assert_eq!(group_name("  "), NO_NAME);
    }

    #[test]
    fn sorting_follows_the_column_and_can_turn_round() {
        let skills = [skill(0, "B", "", 0, 10), skill(1, "A", "", 0, 30)];
        let by_name: Vec<u16> = sorted(&skills, SkillSort::Name, false)
            .iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(by_name, vec![1, 0]);
        let by_value: Vec<u16> = sorted(&skills, SkillSort::Value, true)
            .iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(by_value, vec![1, 0]);
    }

    #[test]
    fn each_column_has_its_words_and_its_value() {
        let skill = WatchSkill {
            value: 505,
            base: 500,
            cap: 1000,
            ..WatchSkill::default()
        };
        let words: Vec<&str> = SkillSort::COLUMNS.iter().map(|c| c.label()).collect();
        assert_eq!(words, vec!["Name", "Real", "Base", "Cap"]);
        assert_eq!(SkillSort::Name.tenths(&skill), None);
        assert_eq!(SkillSort::Real.tenths(&skill), Some(500));
        assert_eq!(SkillSort::Value.tenths(&skill), Some(505));
        assert_eq!(SkillSort::Cap.tenths(&skill), Some(1000));
    }

    #[test]
    fn a_change_is_told_once_it_is_large_enough_and_the_option_is_on() {
        let mut changes = SkillChanges::default();
        let mut options = GeneralOptions::default();
        let frame = |value| WatchFrame {
            skills: vec![skill(21, "Hiding", "", 0, value)],
            ..WatchFrame::default()
        };
        assert!(changes.observe(&frame(500), &options).is_empty());
        assert_eq!(
            changes.observe(&frame(501), &options),
            vec!["Your skill in Hiding has increased by 0.1.  It is now 50.1.".to_string()]
        );
        options.skill_change_tenths = 5;
        assert!(changes.observe(&frame(502), &options).is_empty());
        options.skill_change_messages = false;
        assert!(changes.observe(&frame(400), &options).is_empty());
    }
}
