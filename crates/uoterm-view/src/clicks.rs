//! What the clicks of the human on the map do, and the bar that takes and
//! gives back control, apart from how they draw. The clicks follow the
//! General page, as in the official client: a double click on the ground
//! walks there when pathfinding is on (with Shift held, when the page asks
//! for Shift), and a single click only when the page asks for it.

use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point};
use crate::guard::{aim_words, LocalAim};
use crate::input::Mods;
use crate::keys::chat::{parse_line, Said};
use crate::model::asked::AskedCommands;
use crate::settings::{GeneralOptions, SpeechOptions};

const HINT_MAP: &str = "Double-click: use.  Right-click: more.";
const HINT_MAP_WAR: &str = "Double-click: attack.  Right-click: more.";
const HINT_MAP_ITEM: &str = "Double-click: use.  Drag: move.  Right-click: more.";
const HINT_MAP_TARGET: &str = "Click: target.";
const WORDS_TAKE: &str = "Take control";
const WORDS_IN_CONTROL: &str = "You have control. The agent waits.";
const WORDS_TARGET: &str = "Click the target. Press Esc to cancel.";
const WORDS_GIVE_BACK: &str = "Give back";
const WORDS_STOP: &str = "Stop";
const WORDS_WAR: &str = "War";
const WORDS_PEACE: &str = "Peace";
const WORDS_BAG: &str = "Bag";
const WORDS_SHEET: &str = "Sheet";
const WORDS_MAP: &str = "Map";
const WORDS_MACROS: &str = "Macros";
const WORDS_PROFILE: &str = "Profile";
const WORDS_CHAT: &str = "Chat";
const WORDS_HELP: &str = "Help";
const WORDS_QUIT: &str = "Quit";
const WORDS_OPTIONS: &str = "Options";
const WORDS_SAY: &str = "Say";
const WORDS_ORDER: &str = "Order";
const WORDS_COMMAND: &str = "Do";
const WORDS_ANSWER: &str = "Answer";
const HINT_SAY: &str = "Press Enter, then the words to say.";
const HINT_COMMAND: &str = "A command, for example: useskill 'hiding'. Press Enter.";
const HINT_ANSWER: &str = "The shard asks for words. Type them and press Enter.";
const HINT_ORDER: &str = "An order, for example: attack the orc. Press Enter.";
const HINT_ORDER_OFF: &str = "Orders are off. Set TYPESAFE_API_KEY.";
const HINT_CLOSED: &str = "Press Enter to chat.";

/// The words of the last act show this long beside the bar.
pub const REPORT_SECONDS: f64 = 5.0;

/// True while the words of an act that came at `since` still show at
/// `time`.
pub fn report_shows(since: f64, time: f64) -> bool {
    time - since <= REPORT_SECONDS
}

/// What kind of thing the mouse is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Mobile,
    Item,
    Corpse,
}

/// What the chat box does with a line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChatMode {
    #[default]
    Say,
    /// Words for Jev to turn into an act.
    Order,
    /// One line of the script language.
    Command,
}

impl ChatMode {
    pub fn next(self) -> Self {
        match self {
            Self::Say => Self::Order,
            Self::Order => Self::Command,
            Self::Command => Self::Say,
        }
    }

    /// The words of the mode button. A question of the shard takes the
    /// next line, whatever the mode is.
    pub fn words(self, answering: bool) -> &'static str {
        match (answering, self) {
            (true, _) => WORDS_ANSWER,
            (false, ChatMode::Say) => WORDS_SAY,
            (false, ChatMode::Order) => WORDS_ORDER,
            (false, ChatMode::Command) => WORDS_COMMAND,
        }
    }

    /// The words in the empty chat box: the question of the shard, or
    /// what the mode takes. `line_open` is false while the line waits for
    /// Enter; `orders_on` is false with no TypeSafe key.
    pub fn hint(
        self,
        frame: &WatchFrame,
        answering: bool,
        line_open: bool,
        orders_on: bool,
    ) -> &str {
        match (answering, self, orders_on) {
            (true, ..) => frame
                .text_entry
                .as_ref()
                .map_or(HINT_ANSWER, |entry| entry.words()),
            (false, ChatMode::Say, _) if !line_open => HINT_CLOSED,
            (false, ChatMode::Say, _) => HINT_SAY,
            (false, ChatMode::Order, true) => HINT_ORDER,
            (false, ChatMode::Order, false) => HINT_ORDER_OFF,
            (false, ChatMode::Command, _) => HINT_COMMAND,
        }
    }

    /// What a line of the chat box comes to: the answer to what the shard
    /// asks, or the line of the mode. None for a line with no words.
    pub fn said(
        self,
        words: &str,
        asked: Option<AskedCommands>,
        frame: &WatchFrame,
        speech: &SpeechOptions,
    ) -> Option<Said> {
        let act = match (asked, self) {
            (Some(asked), _) => asked.answer_act(words),
            (None, ChatMode::Say) => return parse_line(words).said(frame, speech),
            (None, ChatMode::Order) => Act::Order(words.to_string(), Box::new(frame.clone())),
            (None, ChatMode::Command) => Act::Command(words.to_string()),
        };
        Some(Said::Act(act))
    }
}

/// Which clicks on the ground run or walk there, from the General page and
/// the keys held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroundClicks {
    /// Any click runs: "Click on the ground runs there" is on, or the run
    /// key is held.
    pub run: bool,
    /// A double click walks, by the pathfinding options.
    pub double: bool,
}

impl GroundClicks {
    pub fn of(general: &GeneralOptions, mods: Mods) -> Self {
        Self {
            run: general.click_to_run || general.run_click_key.is_held(mods),
            double: general.pathfinding && (mods.shift || !general.shift_pathfinding),
        }
    }
}

/// The act for a click on the map.
pub fn act_for_click(
    frame: &WatchFrame,
    thing: Option<(u32, PickKind)>,
    tile: (u16, u16, i8),
    double: bool,
    ground: GroundClicks,
) -> Option<Act> {
    let (x, y, z) = tile;
    Some(match (thing, frame.target_cursor, double) {
        (Some((serial, _)), true, _) => Act::Target(serial),
        (None, true, _) => Act::TargetGround { x, y, z },
        (Some((serial, PickKind::Mobile)), false, true) if frame.war => Act::Attack(serial),
        (Some((serial, _)), false, true) => Act::Use(serial),
        (Some((serial, _)), false, false) => Act::Look(serial),
        (None, false, _) if ground.run => Act::RunTo { x, y },
        (None, false, true) if ground.double => Act::WalkTo { x, y },
        (None, false, _) => return None,
    })
}

/// What Escape does on the map.
#[derive(Clone, Debug, PartialEq)]
pub enum EscapeOnMap {
    /// The click the window waits for is not wanted any more.
    CancelAim,
    Act(Act),
}

/// What Escape does on the map: it lets the aim of the window go first,
/// then the target cursor of the shard.
pub fn escape_on_map(aiming: bool, target_cursor: bool) -> Option<EscapeOnMap> {
    if aiming {
        Some(EscapeOnMap::CancelAim)
    } else if target_cursor {
        Some(EscapeOnMap::Act(Act::CancelTarget))
    } else {
        None
    }
}

/// What a drag on the map takes: what the button went down on. With
/// Sallos easy grab, a drag that began on the ground takes what the mouse
/// is over now, as in the reference client.
pub fn grabbed<T>(pressed_on: Option<T>, hovered: Option<T>, easy_grab: bool) -> Option<T> {
    pressed_on.or(hovered.filter(|_| easy_grab))
}

/// What a click on a thing of the map does, for the line beside the mouse.
pub fn hint_for(frame: &WatchFrame, kind: PickKind) -> &'static str {
    match (frame.target_cursor, kind) {
        (true, _) => HINT_MAP_TARGET,
        (false, PickKind::Mobile) if frame.war => HINT_MAP_WAR,
        (false, PickKind::Item) => HINT_MAP_ITEM,
        (false, _) => HINT_MAP,
    }
}

/// The edge of words that touches their point; the words are centred on
/// it across.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordsEdge {
    Top,
    Bottom,
}

/// Where words beside the bar go, `gap` from it: under it, or over it when
/// it stands `low`, at the foot of the window. Gives the point and the
/// edge of the words that touches it.
pub fn beside_bar(bar: Area, low: bool, gap: f32) -> (Point, WordsEdge) {
    let middle = bar.center().x;
    if low {
        (Point::new(middle, bar.min.y - gap), WordsEdge::Bottom)
    } else {
        (Point::new(middle, bar.max.y + gap), WordsEdge::Top)
    }
}

/// What a button of the bar does.
#[derive(Clone, Debug, PartialEq)]
pub enum Press {
    Act(Act),
    /// Opens the backpack with this serial, or closes its panel.
    Bag(u32),
    Sheet,
    Map,
    Macros,
    /// Opens the profile of the character, or closes it.
    Profile,
    Chat,
    /// Leaves the world and closes the whole program.
    Quit,
    Options,
}

/// The buttons of the bar, left to right. While the agent has the
/// character only the take button and the windows show. The bag button
/// shows only when the shard told which item the backpack is.
pub fn bar_buttons(frame: &WatchFrame) -> Vec<(&'static str, Press)> {
    if !frame.human_control {
        return vec![
            (WORDS_TAKE, Press::Act(Act::Take)),
            (WORDS_SHEET, Press::Sheet),
            (WORDS_MAP, Press::Map),
            (WORDS_MACROS, Press::Macros),
            (WORDS_OPTIONS, Press::Options),
            (WORDS_QUIT, Press::Quit),
        ];
    }
    let war_words = if frame.war { WORDS_PEACE } else { WORDS_WAR };
    frame
        .backpack()
        .map(|bag| (WORDS_BAG, Press::Bag(bag)))
        .into_iter()
        .chain([
            (WORDS_SHEET, Press::Sheet),
            (WORDS_MAP, Press::Map),
            (WORDS_MACROS, Press::Macros),
            (WORDS_PROFILE, Press::Profile),
            (WORDS_CHAT, Press::Chat),
            (WORDS_HELP, Press::Act(Act::Help)),
            (war_words, Press::Act(Act::War(!frame.war))),
            (WORDS_STOP, Press::Act(Act::Stop)),
            (WORDS_GIVE_BACK, Press::Act(Act::GiveBack)),
            (WORDS_OPTIONS, Press::Options),
            (WORDS_QUIT, Press::Quit),
        ])
        .collect()
}

/// The line of the bar that tells who has control and what a click does.
/// None while the agent has the character.
pub fn bar_status(frame: &WatchFrame, aiming: Option<LocalAim>) -> Option<&'static str> {
    match (frame.human_control, frame.target_cursor, aiming) {
        (false, ..) => None,
        (true, _, Some(aim)) => Some(aim_words(aim)),
        (true, false, None) => Some(WORDS_IN_CONTROL),
        (true, true, None) => Some(WORDS_TARGET),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Vector;
    use crate::model::asked::PROMPT;
    use crate::settings::ModifierKey;

    const ORC: u32 = 9;
    const TILE: (u16, u16, i8) = (10, 20, 5);
    const NONE: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
        command: false,
    };
    const SHIFT: Mods = Mods {
        shift: true,
        ..NONE
    };
    const ALT: Mods = Mods { alt: true, ..NONE };
    const CTRL: Mods = Mods {
        ctrl: true,
        command: true,
        ..NONE
    };

    fn frame(war: bool, target_cursor: bool) -> WatchFrame {
        WatchFrame {
            war,
            target_cursor,
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_words_of_an_act_show_for_a_while() {
        assert!(report_shows(1.0, 1.0 + REPORT_SECONDS));
        assert!(!report_shows(1.0, 1.1 + REPORT_SECONDS));
    }

    #[test]
    fn escape_lets_the_aim_of_the_window_go_before_the_cursor_of_the_shard() {
        assert_eq!(escape_on_map(true, true), Some(EscapeOnMap::CancelAim));
        assert_eq!(
            escape_on_map(false, true),
            Some(EscapeOnMap::Act(Act::CancelTarget))
        );
        assert_eq!(escape_on_map(false, false), None);
    }

    #[test]
    fn words_beside_the_bar_go_under_it_or_over_it_at_the_foot() {
        const GAP: f32 = 4.0;
        let bar = Area::from_min_size(Point::new(100.0, 200.0), Vector::new(880.0, 120.0));
        assert_eq!(
            beside_bar(bar, false, GAP),
            (Point::new(bar.center().x, bar.max.y + GAP), WordsEdge::Top)
        );
        assert_eq!(
            beside_bar(bar, true, GAP),
            (
                Point::new(bar.center().x, bar.min.y - GAP),
                WordsEdge::Bottom
            )
        );
    }

    #[test]
    fn a_drag_takes_what_it_began_on_or_with_easy_grab_what_it_is_over() {
        assert_eq!(grabbed(Some(1), Some(2), false), Some(1));
        assert_eq!(grabbed(Some(1), Some(2), true), Some(1));
        assert_eq!(grabbed(None, Some(2), false), None);
        assert_eq!(grabbed(None, Some(2), true), Some(2));
    }

    #[test]
    fn the_chat_modes_go_round() {
        assert_eq!(ChatMode::Say.next().next().next(), ChatMode::Say);
    }

    #[test]
    fn a_chat_line_goes_by_its_mode_unless_the_shard_asks() {
        let frame = WatchFrame::default();
        let speech = SpeechOptions::default();
        assert_eq!(
            ChatMode::Command.said("useskill 'hiding'", None, &frame, &speech),
            Some(Said::Act(Act::Command("useskill 'hiding'".into())))
        );
        assert_eq!(
            ChatMode::Command.said("Mara", Some(PROMPT), &frame, &speech),
            Some(Said::Act(PROMPT.answer_act("Mara")))
        );
        assert!(matches!(
            ChatMode::Say.said("hail", None, &frame, &speech),
            Some(Said::Act(Act::Say { .. }))
        ));
        assert_eq!(ChatMode::Order.words(true), WORDS_ANSWER);
        assert_eq!(ChatMode::Say.hint(&frame, false, false, true), HINT_CLOSED);
        assert_eq!(
            ChatMode::Order.hint(&frame, false, true, false),
            HINT_ORDER_OFF
        );
        assert_eq!(
            ChatMode::Command.hint(&frame, true, true, true),
            HINT_ANSWER
        );
    }

    #[test]
    fn the_bar_offers_the_take_button_until_the_human_has_control() {
        let agent = WatchFrame::default();
        assert_eq!(bar_buttons(&agent)[0], (WORDS_TAKE, Press::Act(Act::Take)));
        assert_eq!(bar_status(&agent, None), None);
        let human = WatchFrame {
            human_control: true,
            war: true,
            ..WatchFrame::default()
        };
        let buttons = bar_buttons(&human);
        assert!(buttons.contains(&(WORDS_PEACE, Press::Act(Act::War(false)))));
        assert_eq!(bar_status(&human, None), Some(WORDS_IN_CONTROL));
        assert_eq!(
            bar_status(&human, Some(LocalAim::Grab)),
            Some(aim_words(LocalAim::Grab))
        );
    }

    #[test]
    fn a_click_on_the_ground_walks_as_the_general_page_says() {
        let peace = frame(false, false);
        let off = GeneralOptions {
            pathfinding: false,
            ..GeneralOptions::default()
        };
        let still = GroundClicks::of(&off, NONE);
        assert_eq!(act_for_click(&peace, None, TILE, false, still), None);
        assert_eq!(act_for_click(&peace, None, TILE, true, still), None);
        let mut general = GeneralOptions::default();
        assert!(general.pathfinding, "a double click walks by default");
        let walk = Some(Act::WalkTo { x: 10, y: 20 });
        let pathfind = GroundClicks::of(&general, NONE);
        assert_eq!(act_for_click(&peace, None, TILE, true, pathfind), walk);
        assert_eq!(act_for_click(&peace, None, TILE, false, pathfind), None);
        general.shift_pathfinding = true;
        let no_shift = GroundClicks::of(&general, NONE);
        assert_eq!(act_for_click(&peace, None, TILE, true, no_shift), None);
        let shift = GroundClicks::of(&general, SHIFT);
        assert_eq!(act_for_click(&peace, None, TILE, true, shift), walk);
    }

    #[test]
    fn a_click_on_the_ground_runs_with_the_run_key_or_the_click_to_run_option() {
        let peace = frame(false, false);
        let run = Some(Act::RunTo { x: 10, y: 20 });
        let mut general = GeneralOptions::default();
        let plain = GroundClicks::of(&general, NONE);
        assert_eq!(act_for_click(&peace, None, TILE, false, plain), None);
        let alt = GroundClicks::of(&general, ALT);
        assert_eq!(act_for_click(&peace, None, TILE, false, alt), run);
        assert_eq!(act_for_click(&peace, None, TILE, true, alt), run);
        for other in [CTRL, SHIFT] {
            let clicks = GroundClicks::of(&general, other);
            assert_eq!(act_for_click(&peace, None, TILE, false, clicks), None);
        }
        let orc = Some((ORC, PickKind::Mobile));
        assert_eq!(
            act_for_click(&peace, orc, TILE, false, alt),
            Some(Act::Look(ORC)),
            "the run key leaves a click on a thing as it is"
        );
        general.run_click_key = ModifierKey::Ctrl;
        let ctrl = GroundClicks::of(&general, CTRL);
        assert_eq!(act_for_click(&peace, None, TILE, false, ctrl), run);
        let alt = GroundClicks::of(&general, ALT);
        assert_eq!(act_for_click(&peace, None, TILE, false, alt), None);
        general.run_click_key = ModifierKey::None;
        general.click_to_run = true;
        let toggled = GroundClicks::of(&general, NONE);
        assert_eq!(act_for_click(&peace, None, TILE, false, toggled), run);
        assert_eq!(act_for_click(&peace, None, TILE, true, toggled), run);
        let aiming = frame(false, true);
        assert_eq!(
            act_for_click(&aiming, None, TILE, false, toggled),
            Some(Act::TargetGround { x: 10, y: 20, z: 5 }),
            "a target cursor takes the click first"
        );
    }

    #[test]
    fn a_target_cursor_targets_what_is_clicked() {
        let aiming = frame(false, true);
        let ground = GroundClicks::default();
        assert_eq!(
            act_for_click(&aiming, None, TILE, false, ground),
            Some(Act::TargetGround { x: 10, y: 20, z: 5 })
        );
        let orc = Some((ORC, PickKind::Mobile));
        assert_eq!(
            act_for_click(&aiming, orc, TILE, false, ground),
            Some(Act::Target(ORC))
        );
    }

    #[test]
    fn a_double_click_attacks_in_war_and_uses_in_peace() {
        let orc = Some((ORC, PickKind::Mobile));
        let plain = GroundClicks::default();
        assert_eq!(
            act_for_click(&frame(true, false), orc, TILE, true, plain),
            Some(Act::Attack(ORC))
        );
        assert_eq!(
            act_for_click(&frame(false, false), orc, TILE, true, plain),
            Some(Act::Use(ORC))
        );
        assert_eq!(
            act_for_click(&frame(true, false), orc, TILE, false, plain),
            Some(Act::Look(ORC))
        );
        let chest = Some((ORC, PickKind::Item));
        assert_eq!(
            act_for_click(&frame(true, false), chest, TILE, true, plain),
            Some(Act::Use(ORC))
        );
    }
}
