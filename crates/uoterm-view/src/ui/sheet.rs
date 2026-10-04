//! The sheet of the character, as both clients show it: its place and
//! size, its tabs, and the words of the worn and status views and of the
//! skills, spells and party tabs. What each row does is the rules of
//! `model` and `ui::lists`.

use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::PANEL_PAD;
use crate::actions::windows::{CharacterView, Tab};
use crate::geom::{Area, Vector};
use crate::input::Mods;

/// The id that keeps the place of the sheet in the profile.
pub const SHEET_ID: &str = "modern:sheet";
pub const SHEET_WIDTH: f32 = 430.0;
/// The rows of the sheet as it first opens, and the fewest it takes.
pub const SHEET_ROWS: usize = 14;
pub const SHEET_MIN_ROWS: usize = 10;
/// The height of a row of the sheet.
pub const ROW: f32 = 24.0;
pub const TAB_HEIGHT: f32 = 28.0;
pub const TAB_GAP: f32 = 6.0;

pub const WORDS_SHEET: &str = "Character";
pub const SHEET_TABS: [(Tab, &str); 4] = [
    (Tab::Character, "Character"),
    (Tab::Skills, "Skills"),
    (Tab::Spells, "Spells"),
    (Tab::Party, "Party"),
];
pub const CHARACTER_VIEWS: [(CharacterView, &str); 2] = [
    (CharacterView::Worn, "Worn"),
    (CharacterView::Status, "Status"),
];
pub const WORDS_USE: &str = "Use";
pub const WORDS_PIN: &str = "Pin";

// The worn view.
pub const WORDS_WORN: &str = "Worn";
pub const WORDS_TAKE_OFF: &str = "x";
pub const WORDS_WEAR: &str = "Wear";
pub const WORDS_WEIGHT: &str = "Weight";
pub const WORDS_GOLD: &str = "Gold";
pub const WORDS_NOTHING_WORN: &str = "Nothing worn. Drag an item onto the figure.";
pub const WORDS_LOOKING: &str = "Jev looks in your bag...";
pub const HINT_WORN: &str =
    "Click: name.  Double-click: use.  Drag or x: take off.  Right-click: more.";
pub const HINT_WEAR: &str = "Say what to wear or take off, for example: my viking sword";
pub const HINT_WEAR_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
/// How long the words about what Jev did stay on the sheet, in seconds.
pub const NOTE_SECONDS: f64 = 6.0;
pub const HINT_STAT_LOCK: &str = "Click: up, down or locked.";

// The skills tab.
pub const WORDS_NEW_GROUP: &str = "New group";
pub const WORDS_RESET: &str = "Reset groups";
pub const WORDS_RESET_ASK: &str = "Back to the first groups?";
pub const WORDS_DELETE_GROUP: &str = "x";
pub const WORDS_GROUP_OPEN: &str = "-";
pub const WORDS_GROUP_FOLDED: &str = "+";
pub const HINT_GROUP: &str = "Click: select, again: rename.  Delete: take it away.";
pub const HINT_SKILL: &str = "Drag: to another group, or a skill to use onto the hotbar.";

// The spells tab.
pub const WORDS_NO_BOOK: &str = "No spellbook is known yet. Open one:";
pub const WORDS_EMPTY_BOOK: &str = "This book holds no spells.";
pub const WORDS_PICK_SPELL: &str = "Click a spell to read it.";
pub const WORDS_REAGENTS: &str = "Reagents:";
pub const WORDS_CAST: &str = "Cast";
pub const WORDS_TITHING_COST: &str = "Tithing cost";
pub const WORDS_TITHING_HAVE: &str = "Tithing points";
pub const WORDS_ASSIGN: &str = "+";
pub const HINT_SPELL: &str = "Double-click: cast.  Drag: onto the hotbar.";
pub const HINT_ASSIGN: &str = "Ctrl+Alt+click: make a macro of it.";

// The party tab.
pub const WORDS_INVITE_TITLE: &str = "Party invite";
pub const WORDS_ACCEPT: &str = "Accept";
pub const WORDS_DECLINE: &str = "Decline";
pub const WORDS_LOOT_ON: &str = "Party loots: yes";
pub const WORDS_LOOT_OFF: &str = "Party loots: no";
pub const WORDS_ADD: &str = "Add member";
pub const WORDS_TELL: &str = "Tell";
pub const WORDS_KICK: &str = "Kick";
pub const WORDS_INVITE: &str = "Invite";
pub const WORDS_EMPTY: &str = "Empty";
pub const WORDS_NEAR: &str = "Invite someone near:";
pub const WORDS_SAY: &str = "Say";
pub const HINT_TELL_PARTY: &str = "Tell the party";
pub const HINT_MEMBER: &str = "Click: look, or target while the shard asks for one.";

/// The words that tell of a macro "Fast spell assign" made.
pub fn assigned_words(name: &str) -> String {
    format!("The macro {name} is made. Give it a key on the Macros page of the Options.")
}

/// True when a click on a spell makes a macro of it: "Fast spell assign"
/// is on and Ctrl and Alt are held.
pub fn assigns_spell(fast_spell_assign: bool, mods: Mods) -> bool {
    fast_spell_assign && mods.ctrl && mods.alt
}

/// The words in the field of words to the party: to one member, or to all.
pub fn tell_hint(member: Option<&str>) -> String {
    member.map_or_else(
        || HINT_TELL_PARTY.to_string(),
        |name| format!("{WORDS_TELL} {name}"),
    )
}

/// The height of the sheet with room for `rows` rows.
pub fn sheet_height(rows: usize) -> f32 {
    TITLE_ROW + PANEL_PAD * 2.0 + TAB_HEIGHT + TAB_GAP + rows as f32 * ROW
}

/// Where the sheet first opens: the middle of the window.
pub fn sheet_first_place(window: Area) -> Area {
    first_place(
        window,
        Spot::Middle(0),
        Vector::new(SHEET_WIDTH, sheet_height(SHEET_ROWS)),
    )
}

/// The least size the player may make the sheet.
pub fn sheet_least() -> Vector {
    Vector::new(SHEET_WIDTH, sheet_height(SHEET_MIN_ROWS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    #[test]
    fn the_sheet_opens_in_the_middle_larger_than_its_least() {
        let window = Area::from_min_size(Point::default(), Vector::new(1280.0, 800.0));
        let first = sheet_first_place(window);
        assert!(first.height() > sheet_least().y);
        assert_eq!(first.width(), sheet_least().x);
        assert!(sheet_height(SHEET_MIN_ROWS) < sheet_height(SHEET_ROWS));
        assert_eq!(tell_hint(None), HINT_TELL_PARTY);
        assert_eq!(tell_hint(Some("Bob")), "Tell Bob");
        assert!(assigned_words("Heal").contains("Heal"));
    }

    #[test]
    fn a_spell_click_assigns_only_with_ctrl_and_alt_and_the_option() {
        let both = Mods {
            ctrl: true,
            alt: true,
            ..Mods::default()
        };
        assert!(assigns_spell(true, both));
        assert!(!assigns_spell(false, both));
        assert!(!assigns_spell(true, Mods::default()));
    }
}
