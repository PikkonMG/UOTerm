//! The window commands of the Modern style: the deck, the map, the panels
//! of the control bar and of the launcher, the journal and the radar, the
//! health bars, the counter bar and the containers. A quit asks first, as
//! the classic client does.

use super::{GumpKind, GumpOp, StyleWindows, WindowCommand};
use crate::view::WatchFrame;
use crate::window::boxes_ui::BoxesUi;
use crate::window::build_ui::ChatUi;
use crate::window::control::{Act, Hand};
use crate::window::control_ui::ControlUi;
use crate::window::deck_ui::{AbilityPanel, CharacterView, DeckUi, Tab};
use crate::window::macros_ui::MacrosUi;
use crate::window::map_ui::MapUi;
use crate::window::mapitem_ui::ProfileUi;
use crate::window::model::spell_data::book_of;
use crate::window::model::{counters, places};
use crate::window::modern::{ModernUi, JOURNAL_ID, RADAR_ID};
use crate::window::options_ui::OptionsUi;
use crate::window::pages_ui::PagesUi;
use crate::window::settings::{Profile, ProfileHome};
use uoterm_assist::spells::School;

/// The gump a shard opens a corpse with.
pub(crate) const CORPSE_GUMP: u16 = 0x0009;

/// The Modern windows, borrowed for one command.
pub struct ModernWindows<'a> {
    pub frame: &'a WatchFrame,
    pub hand: &'a Hand,
    pub profile: &'a mut Profile,
    /// Where the profile is kept, after a command changed it.
    pub home: &'a ProfileHome,
    pub modern: &'a mut ModernUi,
    pub options: &'a mut OptionsUi,
    pub deck: &'a mut DeckUi,
    pub world_map: &'a mut MapUi,
    pub macros: &'a mut MacrosUi,
    pub chat: &'a mut ChatUi,
    pub profiles: &'a mut ProfileUi,
    pub boxes: &'a mut BoxesUi,
    pub control: &'a mut ControlUi,
    /// The paperdoll, the book and the other windows the shard opens.
    pub pages: &'a mut PagesUi,
}

/// Whether a window that is `open` now should be open after `op`.
/// Minimize closes a Modern panel and maximize opens it.
pub(crate) fn wanted(op: GumpOp, open: bool) -> bool {
    match op {
        GumpOp::Open | GumpOp::Maximize => true,
        GumpOp::Close | GumpOp::Minimize => false,
        GumpOp::Toggle => !open,
    }
}

/// Opens, shuts, folds or unfolds a panel that shows until the player
/// shuts it, as the journal and the radar do: minimize folds it to its
/// title, and maximize unfolds it.
fn shown_panel(op: GumpOp, id: &str, profile: &mut Profile) {
    let shows = !places::is_shut(profile, id);
    match op {
        GumpOp::Open => places::set_shut(profile, id, false),
        GumpOp::Close => places::set_shut(profile, id, true),
        GumpOp::Toggle => places::set_shut(profile, id, shows),
        GumpOp::Minimize | GumpOp::Maximize => {
            places::set_shut(profile, id, false);
            places::set_folded(profile, id, op == GumpOp::Minimize);
        }
    }
}

/// Opens or closes a panel that has only a switch.
fn switch_panel(op: GumpOp, open: bool, toggle: impl FnOnce()) {
    if wanted(op, open) != open {
        toggle();
    }
}

/// The tab of the deck that shows a window.
fn deck_tab(kind: GumpKind) -> Option<Tab> {
    Some(match kind {
        GumpKind::Skills => Tab::Skills,
        GumpKind::Party => Tab::Party,
        _ => return None,
    })
}

/// The view of the character tab that shows a window.
fn character_view(kind: GumpKind) -> Option<CharacterView> {
    Some(match kind {
        GumpKind::Paperdoll => CharacterView::Worn,
        GumpKind::Status => CharacterView::Status,
        _ => return None,
    })
}

impl ModernWindows<'_> {
    /// Sends an act of the character, when the human has control.
    fn act(&self, act: Act) {
        if self.frame.human_control {
            self.hand.act(act);
        }
    }

    /// Closes every window that closes, as the classic client's "close all
    /// gumps": the panels, the containers, the paperdoll, and the book, the
    /// board and the map items the shard opened.
    fn close_all(&mut self) {
        for kind in [GumpKind::Options, GumpKind::Macros, GumpKind::Chat] {
            self.gump(GumpOp::Close, kind);
        }
        self.world_map.close();
        if self.deck.is_open() {
            self.deck.toggle();
        }
        for panel in [AbilityPanel::Combat, AbilityPanel::Racial] {
            self.deck.switch_panel(panel, GumpOp::Close);
        }
        self.profiles.close();
        self.pages.close_doll();
        self.modern.close_all(self.frame, self.profile);
        for container in &self.frame.containers {
            self.boxes.close(container.serial);
        }
        if self.frame.book.is_some() {
            self.act(Act::BookClose);
        }
        if self.frame.board.is_some() {
            self.act(Act::BoardClose);
        }
        for map in &self.frame.maps {
            self.act(Act::MapClose(map.serial));
        }
    }

    /// Does one command. False when this style has no such window.
    fn run(&mut self, command: &WindowCommand) -> bool {
        match command {
            WindowCommand::Gump(op, kind) => return self.gump(*op, *kind),
            WindowCommand::CloseAllGumps => self.close_all(),
            WindowCommand::CloseCorpses => {
                let corpses: Vec<u32> = self
                    .frame
                    .containers
                    .iter()
                    .filter(|container| container.gump == CORPSE_GUMP)
                    .map(|container| container.serial)
                    .collect();
                for corpse in corpses {
                    self.boxes.close(corpse);
                }
            }
            WindowCommand::CloseHealthBars { inactive_only } => {
                self.modern
                    .close_health_bars(self.frame, *inactive_only, self.profile);
            }
            WindowCommand::UseCounterSlot(slot) => {
                if let Some(item) = counters::slot_item(self.frame, &self.profile.counters, *slot) {
                    self.act(Act::Use(item));
                }
            }
            WindowCommand::ToggleChat => self.control.toggle_chat(),
            WindowCommand::PasteToChat => self.control.paste(),
            // The Modern style asks before the game quits, as the classic
            // client does.
            WindowCommand::QuitGame => self.modern.ask_quit(),
            _ => return false,
        }
        true
    }

    /// Opens or closes the spells tab on a book of a school. Without a
    /// book of it the shard is asked to open one, and the tab turns to it
    /// when it comes. A book of masteries has no open command.
    fn spellbook(&mut self, op: GumpOp, school: School) -> bool {
        let Some(book) = book_of(school) else {
            return false;
        };
        let shows = self.deck.shows_school(self.frame, school);
        if !wanted(op, shows) {
            if shows {
                self.deck.toggle();
            }
            return true;
        }
        if !shows && !self.deck.choose_school(self.frame, school) {
            if school == School::Mastery {
                return false;
            }
            self.hand.act(Act::OpenSpellbook(book.name));
        }
        self.deck.show(Tab::Spells);
        true
    }

    fn gump(&mut self, op: GumpOp, kind: GumpKind) -> bool {
        if let Some(school) = kind.school() {
            return self.spellbook(op, school);
        }
        if let Some(view) = character_view(kind) {
            let shows = self.deck.shows_view(view);
            if wanted(op, shows) {
                self.deck.show_view(view);
            } else if shows {
                self.deck.toggle();
            }
            return true;
        }
        if let Some(tab) = deck_tab(kind) {
            let shows = self.deck.shows(tab);
            if wanted(op, shows) {
                self.deck.show(tab);
            } else if shows {
                self.deck.toggle();
            }
            return true;
        }
        match kind {
            GumpKind::Options => switch_panel(op, self.options.is_open(), || self.options.toggle()),
            GumpKind::WorldMap => {
                switch_panel(op, self.world_map.is_open(), || self.world_map.toggle())
            }
            GumpKind::Macros => switch_panel(op, self.macros.is_open(), || self.macros.toggle()),
            GumpKind::Chat => switch_panel(op, self.chat.is_open(), || self.chat.toggle()),
            GumpKind::CombatBook => self.deck.switch_panel(AbilityPanel::Combat, op),
            GumpKind::RacialAbilities => self.deck.switch_panel(AbilityPanel::Racial, op),
            GumpKind::Backpack => {
                let Some(bag) = self.frame.backpack() else {
                    return false;
                };
                let shows = self.boxes.shows(self.frame, bag);
                match (wanted(op, shows), op) {
                    // The act of an open came with the command.
                    (true, GumpOp::Open) => self.boxes.used(bag),
                    (true, _) if !shows => {
                        self.boxes.used(bag);
                        self.hand.act(Act::Use(bag));
                    }
                    (false, _) if shows => self.boxes.close(bag),
                    _ => {}
                }
            }
            GumpKind::Journal => shown_panel(op, JOURNAL_ID, self.profile),
            GumpKind::Minimap => shown_panel(op, RADAR_ID, self.profile),
            // The counter bar, the info bar and the buff bar show while
            // their pages have them on.
            GumpKind::Counters => {
                self.profile.counters.enabled = wanted(op, self.profile.counters.enabled);
            }
            GumpKind::InfoBar => {
                self.profile.info_bar.enabled = wanted(op, self.profile.info_bar.enabled);
            }
            GumpKind::Buffs => {
                let combat = &mut self.profile.combat;
                combat.improved_buff_bar = wanted(op, combat.improved_buff_bar);
            }
            _ => return false,
        }
        true
    }
}

impl StyleWindows for ModernWindows<'_> {
    fn command(&mut self, command: &WindowCommand) -> bool {
        let before = self.profile.clone();
        let done = self.run(command);
        if *self.profile != before {
            self.home.save(&places::for_saving(self.profile));
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_journal_and_the_radar_shut_show_fold_and_unfold() {
        let mut profile = Profile::default();
        shown_panel(GumpOp::Toggle, JOURNAL_ID, &mut profile);
        assert!(places::is_shut(&profile, JOURNAL_ID));
        shown_panel(GumpOp::Minimize, JOURNAL_ID, &mut profile);
        assert!(!places::is_shut(&profile, JOURNAL_ID));
        assert!(places::is_folded(&profile, JOURNAL_ID));
        shown_panel(GumpOp::Maximize, JOURNAL_ID, &mut profile);
        assert!(!places::is_folded(&profile, JOURNAL_ID));
        shown_panel(GumpOp::Close, RADAR_ID, &mut profile);
        assert!(places::is_shut(&profile, RADAR_ID));
        shown_panel(GumpOp::Open, RADAR_ID, &mut profile);
        assert!(!places::is_shut(&profile, RADAR_ID));
    }

    #[test]
    fn a_window_op_opens_or_closes_as_the_official_client_does() {
        assert!(wanted(GumpOp::Toggle, false));
        assert!(!wanted(GumpOp::Toggle, true));
        assert!(wanted(GumpOp::Maximize, false));
        assert!(!wanted(GumpOp::Minimize, true));
    }

    #[test]
    fn the_deck_shows_the_character_windows() {
        assert_eq!(deck_tab(GumpKind::Skills), Some(Tab::Skills));
        assert_eq!(deck_tab(GumpKind::Party), Some(Tab::Party));
        assert_eq!(deck_tab(GumpKind::WorldMap), None);
        assert_eq!(
            character_view(GumpKind::Paperdoll),
            Some(CharacterView::Worn)
        );
        assert_eq!(
            character_view(GumpKind::Status),
            Some(CharacterView::Status)
        );
        assert_eq!(character_view(GumpKind::Skills), None);
    }
}
