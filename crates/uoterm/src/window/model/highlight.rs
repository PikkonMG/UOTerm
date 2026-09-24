//! The highlight rules of the grid containers: which items a rule marks,
//! by the words and the numbers of their properties. A rarity, a slayer and
//! a resist are each a rule of this kind.

use super::properties::{self, plain_words};
use crate::window::settings::{HighlightRule, PropertyNeed};

/// The hue a new rule takes: a bright yellow of the UO hue table.
const NEW_RULE_HUE: u16 = 0x0035;
/// A resist rule marks items with at least this much of one resist.
const RESIST_RULE_MIN: f32 = 10.0;

const RARITY_WORDS: [&str; 5] = [
    "legendary artifact",
    "major artifact",
    "greater artifact",
    "lesser artifact",
    "minor artifact",
];
const SLAYER_WORDS: &str = "slayer";
const RESIST_WORDS: [&str; 5] = [
    "physical resist",
    "fire resist",
    "cold resist",
    "poison resist",
    "energy resist",
];

/// True when the item's property lines meet one need: a line holds its
/// words, with a number at least its least when it names one.
fn meets(need: &PropertyNeed, lines: &[String]) -> bool {
    let wanted = plain_words(&need.words);
    !wanted.is_empty()
        && lines
            .iter()
            .map(|line| properties::parse(line))
            .filter(|property| property.words.contains(&wanted))
            .any(|property| match need.min {
                Some(min) => property.value.is_some_and(|value| value >= min),
                None => true,
            })
}

/// True when the item passes the rule. A rule with no needs marks nothing.
pub fn passes(rule: &HighlightRule, lines: &[String], in_corpse: bool) -> bool {
    if rule.needs.is_empty() || (rule.corpses_only && !in_corpse) {
        return false;
    }
    if rule.need_all {
        rule.needs.iter().all(|need| meets(need, lines))
    } else {
        rule.needs.iter().any(|need| meets(need, lines))
    }
}

/// The first rule the item passes.
pub fn first_passed<'a>(
    rules: &'a [HighlightRule],
    lines: &[String],
    in_corpse: bool,
) -> Option<&'a HighlightRule> {
    rules.iter().find(|rule| passes(rule, lines, in_corpse))
}

/// True when a line holds any of the plain property words of the
/// Containers page.
pub fn has_any_words(words: &[String], lines: &[String]) -> bool {
    words.iter().any(|words| {
        let wanted = plain_words(words);
        !wanted.is_empty() && lines.iter().any(|line| plain_words(line).contains(&wanted))
    })
}

fn rule(name: &str, words: &[&str], min: Option<f32>) -> HighlightRule {
    HighlightRule {
        name: name.to_string(),
        hue: NEW_RULE_HUE,
        needs: words
            .iter()
            .map(|words| PropertyNeed {
                words: (*words).to_string(),
                min,
            })
            .collect(),
        need_all: false,
        corpses_only: false,
    }
}

/// The rules a player most often wants, by the words of their button.
pub fn presets() -> Vec<HighlightRule> {
    vec![
        rule("Artifacts", &RARITY_WORDS, None),
        rule("Slayers", &[SLAYER_WORDS], None),
        rule("Resists", &RESIST_WORDS, Some(RESIST_RULE_MIN)),
    ]
}

/// A rule of one property, for the player to fill in.
pub fn blank() -> HighlightRule {
    rule("", &[""], None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn a_rule_marks_items_by_words_and_least_number() {
        let [artifacts, slayers, resists] = presets().try_into().unwrap();
        let sword = lines(&["a longsword", "Undead Slayer", "Fire Resist 5%"]);
        assert!(passes(&slayers, &sword, false));
        assert!(!passes(&artifacts, &sword, false));
        assert!(!passes(&resists, &sword, false), "5% is under the least");
        let robe = lines(&["a robe", "Legendary Artifact", "Energy Resist 15%"]);
        assert!(passes(&artifacts, &robe, false));
        assert!(passes(&resists, &robe, false));
        let rules = vec![artifacts.clone(), slayers.clone()];
        assert_eq!(first_passed(&rules, &sword, false), Some(&slayers));
    }

    #[test]
    fn a_rule_for_every_need_or_for_corpses_only_asks_more() {
        let mut both = rule("both", &["slayer", "fire resist"], None);
        both.need_all = true;
        let sword = lines(&["Undead Slayer"]);
        assert!(!passes(&both, &sword, false));
        assert!(passes(
            &both,
            &lines(&["Undead Slayer", "Fire Resist 1%"]),
            false
        ));
        let mut corpse = rule("corpse", &["slayer"], None);
        corpse.corpses_only = true;
        assert!(!passes(&corpse, &sword, false));
        assert!(passes(&corpse, &sword, true));
        assert!(
            !passes(&blank(), &sword, false),
            "an empty need marks nothing"
        );
    }

    #[test]
    fn plain_words_of_the_page_match_any_line() {
        let words = lines(&["Faster Casting"]);
        assert!(has_any_words(&words, &lines(&["faster casting 1"])));
        assert!(!has_any_words(&words, &lines(&["a ring"])));
        assert!(!has_any_words(&lines(&[" "]), &lines(&["a ring"])));
    }
}
