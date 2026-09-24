//! The window commands of the Classic style: an action that opens, closes,
//! folds or unfolds a window works on the classic gump of that window. A
//! window the shard opens, as the paperdoll and the backpack are, is asked
//! of the shard. A window with no classic gump kind yet answers false, and
//! the window tells the player.

use super::chat::chat_button;
use super::grid_loot::GRID_LOOT;
use super::journal::journal_kind;
use super::manager::GumpManager;
use super::options::Options;
use super::registry::{kind, well_known, GumpId};
use super::spellbook::SPELLBOOK;
use crate::view::WatchFrame;
use crate::window::actions::modern::{wanted, CORPSE_GUMP};
use crate::window::actions::{GumpKind, GumpOp, StyleWindows, WindowCommand};
use crate::window::control::{Act, Hand};
use crate::window::control_ui::ControlUi;
use crate::window::model::counters;
use crate::window::model::spell_data::book_of;
use crate::window::settings::{Page, Profile};
use uoterm_assist::spells::School;

/// The script command that asks the shard for the character's paperdoll.
pub const PAPERDOLL_COMMAND: &str = "paperdoll";

/// The Classic windows, borrowed for one command.
pub struct ClassicWindows<'a> {
    pub frame: &'a WatchFrame,
    pub hand: &'a Hand,
    pub manager: &'a mut GumpManager,
    pub profile: &'a mut Profile,
    pub control: &'a mut ControlUi,
}

/// The classic gump kind that shows a window, when it has one.
fn classic_kind(kind: GumpKind) -> Option<&'static str> {
    Some(match kind {
        GumpKind::Options | GumpKind::Macros => well_known::OPTIONS,
        GumpKind::Status => well_known::STATUS,
        GumpKind::Minimap => well_known::MINIMAP,
        GumpKind::WorldMap => well_known::WORLD_MAP,
        GumpKind::Buffs => well_known::BUFFS,
        GumpKind::Skills => well_known::SKILLS,
        GumpKind::Party => well_known::PARTY,
        GumpKind::CombatBook => well_known::COMBAT_BOOK,
        GumpKind::RacialAbilities => well_known::RACIAL_ABILITIES,
        _ => return None,
    })
}

impl ClassicWindows<'_> {
    fn act(&self, act: Act) {
        if self.frame.human_control {
            self.hand.act(act);
        }
    }

    /// Opens or closes a gump the window draws by itself.
    fn switch(&mut self, op: GumpOp, id: GumpId) -> bool {
        if kind(id.kind).is_none() {
            return false;
        }
        let open = self.manager.is_open(&id);
        match (wanted(op, open), open) {
            (true, false) => {
                self.manager.open(id, self.profile);
            }
            (false, true) => self.manager.close(&id, self.profile),
            _ => {}
        }
        true
    }

    /// Opens or closes a gump the shard shows, such as a paperdoll or a
    /// container. Opening asks the shard with `ask`.
    fn shard_window(&mut self, op: GumpOp, id: GumpId, ask: Act) -> bool {
        let open = self.manager.is_open(&id);
        match (wanted(op, open), open) {
            (true, false) => self.act(ask),
            (false, true) => self.manager.close(&id, self.profile),
            _ => {}
        }
        true
    }

    /// Opens or closes the spellbook of a school: the gump of an open book
    /// of the school closes, and else the shard is asked to open the book.
    /// A book of masteries has no open command: the book the character
    /// carries is used.
    fn spellbook(&mut self, op: GumpOp, window: GumpKind) -> bool {
        let Some(book) = window.school().and_then(book_of) else {
            return false;
        };
        let open = self
            .frame
            .spellbooks
            .iter()
            .filter(|contents| contents.school.eq_ignore_ascii_case(book.name))
            .map(|contents| GumpId::of(SPELLBOOK.id, contents.serial))
            .find(|id| self.manager.is_open(id));
        match (wanted(op, open.is_some()), open) {
            (false, Some(id)) => self.manager.close(&id, self.profile),
            (true, None) => {
                let carried = self
                    .frame
                    .spellbooks
                    .iter()
                    .find(|contents| contents.school.eq_ignore_ascii_case(book.name));
                match (book.school, carried) {
                    (School::Mastery, Some(contents)) => self.act(Act::Use(contents.serial)),
                    (School::Mastery, None) => return false,
                    _ => self.act(Act::OpenSpellbook(book.name)),
                }
            }
            _ => {}
        }
        true
    }

    fn gump(&mut self, op: GumpOp, window: GumpKind) -> bool {
        match window {
            GumpKind::Macros
                if wanted(op, self.manager.is_open(&GumpId::one(well_known::OPTIONS))) =>
            {
                let id = GumpId::one(well_known::OPTIONS);
                self.manager.close(&id, self.profile);
                self.manager
                    .open_body(id, Box::new(Options::at_page(Page::Macros)), self.profile);
                true
            }
            GumpKind::Paperdoll => {
                let id = GumpId::of(well_known::PAPERDOLL, self.frame.serial);
                self.shard_window(op, id, Act::Command(PAPERDOLL_COMMAND.into()))
            }
            GumpKind::Backpack => match self.frame.backpack() {
                Some(bag) => {
                    self.shard_window(op, GumpId::of(well_known::CONTAINER, bag), Act::Use(bag))
                }
                None => false,
            },
            GumpKind::Journal => self.switch(op, GumpId::one(journal_kind(self.profile))),
            // The info bar shows while the Info Bar page has it on.
            GumpKind::InfoBar => {
                self.profile.info_bar.enabled = wanted(op, self.profile.info_bar.enabled);
                true
            }
            // The counter bar shows while the Counters page has it on.
            GumpKind::Counters => {
                self.profile.counters.enabled = wanted(op, self.profile.counters.enabled);
                true
            }
            window if window.school().is_some() => self.spellbook(op, window),
            GumpKind::Chat => match chat_button(self.frame) {
                Ok(id) => self.switch(op, id),
                Err(ask) => {
                    if wanted(op, false) {
                        self.act(ask);
                    }
                    true
                }
            },
            window => match classic_kind(window) {
                Some(id) => self.switch(op, GumpId::one(id)),
                None => false,
            },
        }
    }
}

impl StyleWindows for ClassicWindows<'_> {
    fn command(&mut self, command: &WindowCommand) -> bool {
        match command {
            WindowCommand::Gump(op, window) => return self.gump(*op, *window),
            WindowCommand::CloseAllGumps => self.manager.close_all(self.profile),
            WindowCommand::CloseCorpses => {
                let corpses: Vec<u32> = self
                    .frame
                    .containers
                    .iter()
                    .filter(|container| container.gump == CORPSE_GUMP)
                    .map(|container| container.serial)
                    .collect();
                for corpse in corpses {
                    self.manager
                        .close(&GumpId::of(well_known::CONTAINER, corpse), self.profile);
                    self.manager
                        .close(&GumpId::of(GRID_LOOT.id, corpse), self.profile);
                }
            }
            WindowCommand::CloseHealthBars { inactive_only } => {
                let in_view: Vec<u32> = self.frame.mobiles.iter().map(|m| m.serial).collect();
                self.manager
                    .close_group(well_known::HEALTH_BAR_GROUP, self.profile, |id| {
                        !inactive_only || id.serial.is_some_and(|serial| !in_view.contains(&serial))
                    });
            }
            WindowCommand::UseCounterSlot(slot) => {
                if let Some(item) = counters::slot_item(self.frame, &self.profile.counters, *slot) {
                    self.act(Act::Use(item));
                }
            }
            WindowCommand::ToggleChat => self.control.toggle_chat(),
            WindowCommand::PasteToChat => self.control.paste(),
            // The classic client asks before the game quits.
            WindowCommand::QuitGame => {
                return self
                    .manager
                    .open(GumpId::one(well_known::QUIT_QUESTION), self.profile);
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_map_to_classic_kinds() {
        assert_eq!(classic_kind(GumpKind::Options), Some(well_known::OPTIONS));
        assert_eq!(classic_kind(GumpKind::Status), Some(well_known::STATUS));
        assert_eq!(classic_kind(GumpKind::Paperdoll), None);
    }

    #[test]
    fn quitting_asks_the_question_first() {
        let hand = crate::window::classic::testing::idle_hand();
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let mut control = ControlUi::default();
        let mut windows = ClassicWindows {
            frame: &WatchFrame::default(),
            hand: &hand,
            manager: &mut manager,
            profile: &mut profile,
            control: &mut control,
        };
        assert!(windows.command(&WindowCommand::QuitGame));
        assert!(manager.is_open(&GumpId::one(well_known::QUIT_QUESTION)));
    }

    #[test]
    fn the_counter_bar_follows_its_option_and_a_mastery_book_needs_a_book() {
        let hand = crate::window::classic::testing::idle_hand();
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let mut control = ControlUi::default();
        let mut windows = ClassicWindows {
            frame: &WatchFrame::default(),
            hand: &hand,
            manager: &mut manager,
            profile: &mut profile,
            control: &mut control,
        };
        assert!(windows.command(&WindowCommand::Gump(GumpOp::Toggle, GumpKind::Counters)));
        assert!(windows.profile.counters.enabled);
        assert!(windows.command(&WindowCommand::Gump(
            GumpOp::Open,
            GumpKind::MagerySpellbook
        )));
        assert!(!windows.command(&WindowCommand::Gump(
            GumpOp::Open,
            GumpKind::MasterySpellbook
        )));
        assert!(windows.command(&WindowCommand::UseCounterSlot(1)));
    }
}
