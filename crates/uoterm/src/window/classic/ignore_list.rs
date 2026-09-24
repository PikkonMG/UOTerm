//! The ignore list, as the reference client draws it: the names
//! of the players whose speech the journal and the heads do not show, each
//! with a button that takes it off, and a button that asks the player to
//! target the one to add. The list is the Ignore List of the profile.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::WatchFrame;
use crate::window::actions::LocalAim;
use eframe::egui::Color32;
use uoterm_nav::TextAlign;
use uoterm_protocol::types::NOTO_INVULNERABLE;

pub const IGNORE_LIST: GumpKind = GumpKind {
    id: well_known::IGNORE_LIST,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(IgnoreList),
};

const WIDTH: i32 = 300;
const HEIGHT: i32 = 400;
const BACKGROUND_HUE: u16 = 999;
const BACKGROUND_OPACITY: f32 = 0.95;
const EDGE: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
const EDGE_WIDTH: i32 = 1;
const FONT: u8 = 1;
const WHITE: u16 = 0xFFFF;
const LEGEND_Y: i32 = 10;
const NAME_X: i32 = 10;
const REMOVE_X: i32 = 210;
const LEGEND_RULE_Y: i32 = 30;
const LIST: (i32, i32, i32, i32) = (10, 40, 280, 320);
const ROW: i32 = 25;
const REMOVE_BUTTON_X: i32 = 220;
const REMOVE_BUTTON: ButtonArt = ButtonArt::new(0x0FAB, 0x0FAC, 0);
const ADD_BUTTON: (i32, i32, i32, i32) = (20, 370, 260, 25);

const WORDS_NAME: &str = "Character name";
const WORDS_REMOVE: &str = "Remove";
const WORDS_ADD: &str = "Add person to ignore list";
const NOTE_NOT_A_PLAYER: &str = "This is not a player.";

/// The gump keeps nothing; the profile holds the list.
pub struct IgnoreList;

/// The name to ignore of the thing the player targeted, or why there is
/// none: the character himself and the invulnerable are not players.
fn ignored_name(frame: &WatchFrame, serial: u32) -> Result<String, &'static str> {
    frame
        .mobiles
        .iter()
        .find(|mobile| mobile.serial == serial)
        .filter(|mobile| {
            mobile.serial != frame.serial
                && mobile.notoriety != NOTO_INVULNERABLE
                && !mobile.name.is_empty()
        })
        .map(|mobile| mobile.name.clone())
        .ok_or(NOTE_NOT_A_PLAYER)
}

fn added_words(name: &str) -> String {
    format!("Added {name} to ignore list.")
}

fn already_words(name: &str) -> String {
    format!("{name} is on the ignore list already.")
}

impl IgnoreList {
    /// Puts the player targeted for the list on it.
    fn take_pick(&self, cx: &mut GumpContext<'_>) {
        let Some(serial) = cx.hand.take_picked(LocalAim::IgnorePlayer) else {
            return;
        };
        match ignored_name(cx.frame, serial) {
            Ok(name) if cx.profile.ignore.add(&name) => {
                cx.hand.report(&added_words(&name));
                cx.profile_changed();
            }
            Ok(name) => cx.hand.report(&already_words(&name)),
            Err(words) => cx.hand.report(words),
        }
    }
}

impl GumpBody for IgnoreList {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        self.take_pick(cx);
        g.shade(0, 0, WIDTH, HEIGHT, BACKGROUND_HUE, BACKGROUND_OPACITY);
        g.fill(0, 0, WIDTH, EDGE_WIDTH, EDGE);
        g.fill(0, 0, EDGE_WIDTH, HEIGHT, EDGE);
        g.fill(0, HEIGHT, WIDTH, EDGE_WIDTH, EDGE);
        g.fill(WIDTH, 0, EDGE_WIDTH, HEIGHT, EDGE);
        let legend = TextLook::unicode(FONT, WHITE).bordered();
        g.label(NAME_X, LEGEND_Y, WORDS_NAME, &legend);
        g.label(REMOVE_X, LEGEND_Y, WORDS_REMOVE, &legend);
        g.fill(0, LEGEND_RULE_Y, WIDTH, EDGE_WIDTH, EDGE);
        let names = cx.profile.ignore.names.clone();
        let look = TextLook::unicode(FONT, WHITE).cropped((REMOVE_BUTTON_X - NAME_X) as u32);
        let mut removed = None;
        let (x, y, w, h) = LIST;
        g.scroll_area("names", x, y, w, h, |g| {
            for (at, name) in names.iter().enumerate() {
                let row_y = at as i32 * ROW;
                g.label(NAME_X, row_y, name, &look);
                if g.button(("remove", at), REMOVE_BUTTON_X, row_y, REMOVE_BUTTON) {
                    removed = Some(name.clone());
                }
            }
            names.len() as i32 * ROW
        });
        if let Some(name) = removed {
            if cx.profile.ignore.remove(&name) {
                cx.profile_changed();
            }
        }
        let (x, y, w, h) = ADD_BUTTON;
        let caption = TextLook::unicode(FONT, WHITE).aligned(TextAlign::Center);
        if g.nice_button("add", x, y, w, h, WORDS_ADD, &caption, false) {
            cx.hand.aim(LocalAim::IgnorePlayer);
        }
    }

    /// Closing stops waiting for the player to target someone.
    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        if cx.hand.aiming() == Some(LocalAim::IgnorePlayer) {
            cx.hand.cancel_aim();
        }
        Closing::Now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchMobile;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::{IgnoreOptions, Profile};
    use uoterm_protocol::types::NOTO_INNOCENT;

    const ME: u32 = 1;
    const ANN: u32 = 2;
    const GUARD: u32 = 3;

    fn mobile(serial: u32, name: &str, notoriety: u8) -> WatchMobile {
        WatchMobile {
            serial,
            name: name.into(),
            notoriety,
            ..WatchMobile::default()
        }
    }

    fn frame() -> WatchFrame {
        WatchFrame {
            serial: ME,
            mobiles: vec![
                mobile(ME, "Mara", NOTO_INNOCENT),
                mobile(ANN, "Ann", NOTO_INNOCENT),
                mobile(GUARD, "a guard", NOTO_INVULNERABLE),
            ],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn only_another_player_goes_on_the_list() {
        let frame = frame();
        assert_eq!(ignored_name(&frame, ANN), Ok("Ann".into()));
        assert_eq!(ignored_name(&frame, ME), Err(NOTE_NOT_A_PLAYER));
        assert_eq!(ignored_name(&frame, GUARD), Err(NOTE_NOT_A_PLAYER));
        assert_eq!(ignored_name(&frame, 0x4000_0001), Err(NOTE_NOT_A_PLAYER));
    }

    #[test]
    fn a_name_goes_on_the_list_once_and_comes_off() {
        let mut ignore = IgnoreOptions::default();
        assert!(ignore.add(" Ann "));
        assert!(!ignore.add("Ann"), "once");
        assert!(!ignore.add("  "), "no empty names");
        assert_eq!(ignore.names, vec!["Ann".to_string()]);
        assert!(ignore.remove("Ann"));
        assert!(!ignore.remove("Ann"));
        assert!(ignore.names.is_empty());
        assert_eq!(added_words("Ann"), "Added Ann to ignore list.");
        assert_eq!(already_words("Ann"), "Ann is on the ignore list already.");
    }

    #[test]
    fn the_list_draws_with_the_client_files_and_changes_nothing_by_itself() {
        let mut profile = Profile::default();
        profile.ignore.add("Ann");
        let mut manager = GumpManager::default();
        let id = GumpId::one(well_known::IGNORE_LIST);
        manager.open(id, &mut profile);
        if !draw_frames(&mut manager, &mut profile, &frame()) {
            return;
        }
        assert!(manager.is_open(&id));
        assert_eq!(profile.ignore.names, vec!["Ann".to_string()]);
    }
}
