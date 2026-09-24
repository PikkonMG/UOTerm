//! The kinds of classic gumps. A kind has an id, the rules the gump manager
//! keeps for its gumps, and a function that makes a new gump of it. Each
//! gump is a [`GumpBody`]: it draws itself on a [`Canvas`] each frame and
//! acts through the [`GumpContext`].
//!
//! To add a kind, write its body and a `pub const KIND: GumpKind` in its own
//! module under `classic/`, and put the kind in [`KINDS`].

use super::canvas::Canvas;
use super::race_change;
use super::{book, bulletin_board, bulletin_post};
use super::{buff_gump, combat_book, health_bar, paperdoll, party, profile, racial};
use super::{chat, ignore_list, map_item, tip_notice};
use super::{container, grid_loot, skill_button, skills, spell_button, spellbook, split_menu};
use super::{counter_bar, macro_button, shop, trade};
use super::{debug, hue_picker, options, status, top_bar};
use super::{house, macro_gump, popup_menu};
use super::{info_bar, journal, map_markers, minimap, net_stats, world_map};
use super::{message_box, old_menu, prompt, text_entry};
use crate::view::WatchFrame;
use crate::window::control::{Act, Hand};
use crate::window::desk::Desk;
use crate::window::gump_ui;
use crate::window::model::journal::JournalLog;
use crate::window::model::reads::Readings;
use crate::window::settings::{MacroStep, Profile};
use crate::window::tips::Tips;
use eframe::egui::{Pos2, Vec2};
use std::collections::HashMap;

/// Every kind of classic gump.
pub const KINDS: &[GumpKind] = &[
    status::STATUS,
    status::SELF_BAR,
    container::CONTAINER,
    split_menu::SPLIT_MENU,
    grid_loot::GRID_LOOT,
    spellbook::SPELLBOOK,
    spell_button::SPELL_BUTTON,
    skills::SKILLS,
    skills::RESET_GROUPS,
    skill_button::SKILL_BUTTON,
    shop::SHOP,
    trade::TRADE,
    counter_bar::COUNTER_BAR,
    macro_button::MACRO_BUTTON,
    top_bar::TOP_BAR,
    options::OPTIONS,
    hue_picker::HUE_PICKER,
    hue_picker::DYE_PICKER,
    debug::DEBUG,
    gump_ui::SHARD_GUMP,
    book::BOOK,
    bulletin_board::BULLETIN_BOARD,
    bulletin_post::BULLETIN_POST,
    old_menu::OLD_MENU,
    text_entry::TEXT_ENTRY,
    prompt::PROMPT,
    message_box::CRIMINAL_QUESTION,
    message_box::QUIT_QUESTION,
    message_box::QUESTION,
    chat::CHAT,
    chat::CHAT_NAME,
    map_item::MAP_ITEM,
    tip_notice::TIP_NOTICE,
    race_change::RACE_CHANGE,
    ignore_list::IGNORE_LIST,
    popup_menu::POPUP_MENU,
    house::HOUSE,
    macro_gump::MACRO,
    journal::JOURNAL,
    journal::RESIZABLE_JOURNAL,
    world_map::WORLD_MAP,
    map_markers::MARKERS_MANAGER,
    map_markers::USER_MARKER,
    map_markers::LOCATION_GO,
    minimap::MINIMAP,
    info_bar::INFO_BAR,
    net_stats::NET_STATS,
    paperdoll::PAPERDOLL,
    health_bar::HEALTH_BAR,
    health_bar::TARGET_BAR,
    buff_gump::BUFFS,
    combat_book::COMBAT_BOOK,
    combat_book::ABILITY_BUTTON,
    racial::RACIAL_ABILITIES,
    racial::RACIAL_BUTTON,
    party::PARTY,
    party::PARTY_INVITE,
    party::PARTY_TELL,
    profile::PROFILE,
];

/// The ids of the kinds that other gumps open by name. A kind with one of
/// these ids must be the gump the name says.
pub mod well_known {
    pub const STATUS: &str = "status";
    /// The health bar of the character.
    pub const SELF_BAR: &str = "self_bar";
    /// The anchor group of every health bar.
    pub const HEALTH_BAR_GROUP: &str = "health_bar";
    /// The health bar of a mobile, by its serial.
    pub const HEALTH_BAR: &str = "health_bar";
    /// The health bar that follows the last target.
    pub const TARGET_BAR: &str = "target_bar";
    /// The profile of a character, by its serial.
    pub const PROFILE: &str = "profile";
    pub const TOP_BAR: &str = "top_bar";
    pub const OPTIONS: &str = "options";
    pub const HUE_PICKER: &str = "hue_picker";
    pub const DEBUG: &str = "debug";
    /// A gump the shard sent, by its gump id.
    pub const SHARD: &str = "shard";
    pub const BUFFS: &str = "buffs";
    pub const MINIMAP: &str = "minimap";
    pub const JOURNAL: &str = "journal";
    pub const WORLD_MAP: &str = "world_map";
    pub const SKILLS: &str = "skills";
    pub const PARTY: &str = "party";
    pub const COUNTERS: &str = "counters";
    pub const INFO_BAR: &str = "info_bar";
    pub const COMBAT_BOOK: &str = "combat_book";
    pub const RACIAL_ABILITIES: &str = "racial_abilities";
    /// The paperdoll of a mobile, by its serial.
    pub const PAPERDOLL: &str = "paperdoll";
    /// A container, by its serial.
    pub const CONTAINER: &str = "container";
    /// The journal with tabs the player sizes, in place of the scroll.
    pub const RESIZABLE_JOURNAL: &str = "resizable_journal";
    pub const NET_STATS: &str = "net_stats";
    pub const MARKERS_MANAGER: &str = "markers_manager";
    /// The box that adds a marker of the player to the world map.
    pub const USER_MARKER: &str = "user_marker";
    /// The box that moves the world map to typed coordinates.
    pub const LOCATION_GO: &str = "location_go";
    /// The chat of the shard, and the box that asks for a chat name.
    pub const CHAT: &str = "chat";
    pub const CHAT_NAME: &str = "chat_name";
    /// A book, by its serial.
    pub const BOOK: &str = "book";
    /// A bulletin board, by its serial.
    pub const BULLETIN_BOARD: &str = "bulletin_board";
    /// One message of a bulletin board, read or written, by its serial
    /// (zero for a new message).
    pub const BULLETIN_POST: &str = "bulletin_post";
    /// A map item such as a treasure map, by its serial.
    pub const MAP_ITEM: &str = "map_item";
    /// The old menu of the shard (0x7C), with pictures or gray.
    pub const OLD_MENU: &str = "old_menu";
    /// The text entry dialog of the shard (0xAB).
    pub const TEXT_ENTRY: &str = "text_entry";
    /// The words the shard waits for (0x9A and 0xC2).
    pub const PROMPT: &str = "prompt";
    /// The question of the criminal action.
    pub const CRIMINAL_QUESTION: &str = "criminal_question";
    /// The question before the game quits.
    pub const QUIT_QUESTION: &str = "quit_question";
    /// A tip or a notice of the shard (0xA6).
    pub const TIP_NOTICE: &str = "tip_notice";
    pub const IGNORE_LIST: &str = "ignore_list";
    /// The race change of the shard (0xBF 0x2A).
    pub const RACE_CHANGE: &str = "race_change";
    /// A question another gump asks, opened with its own words and answer.
    pub const QUESTION: &str = "question";
}

/// The kind with this id.
pub fn kind(id: &str) -> Option<&'static GumpKind> {
    KINDS.iter().find(|kind| kind.id == id)
}

/// Names one gump: its kind and, for a kind with one gump for each thing,
/// the serial of the thing, such as a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GumpId {
    pub kind: &'static str,
    pub serial: Option<u32>,
}

impl GumpId {
    /// The one gump of a kind.
    pub const fn one(kind: &'static str) -> Self {
        Self { kind, serial: None }
    }

    /// The gump of a kind for one thing.
    pub const fn of(kind: &'static str, serial: u32) -> Self {
        Self {
            kind,
            serial: Some(serial),
        }
    }

    /// The name its place is kept under, such as `status` or
    /// `container:3C`.
    pub fn place_key(&self) -> String {
        match self.serial {
            Some(serial) => format!("{}:{serial:X}", self.kind),
            None => self.kind.to_string(),
        }
    }
}

/// The rules the gump manager keeps for the gumps of one kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GumpRules {
    /// A right click closes the gump, as it does most gumps.
    pub right_click_closes: bool,
    /// The player can drag the gump.
    pub movable: bool,
    /// Gumps of one anchor group snap to each other's sides and move
    /// together, as health bars do.
    pub anchor: Option<&'static str>,
    /// The gump takes every click while it is open.
    pub modal: bool,
    /// The gump also shows in the Modern style.
    pub both_styles: bool,
    /// The place of the gump is kept in the profile. A gump that is not
    /// kept remembers its place until the window closes.
    pub kept: bool,
    /// Where the gump first opens, in window points.
    pub first_place: Pos2,
    /// The player resizes the gump with the grip at its corner: its first
    /// size and its least size, in its own pixels. The body reads the size
    /// with `Canvas::size`.
    pub resizable: Option<(Vec2, Vec2)>,
}

impl GumpRules {
    /// The rules of most gumps.
    pub const DEFAULT: Self = Self {
        right_click_closes: true,
        movable: true,
        anchor: None,
        modal: false,
        both_styles: false,
        kept: true,
        first_place: Pos2::new(100.0, 100.0),
        resizable: None,
    };
}

/// One kind of classic gump.
pub struct GumpKind {
    pub id: &'static str,
    pub rules: GumpRules,
    /// Makes a new gump of the kind, for the thing with `serial` when the
    /// kind has one gump for each thing.
    pub open: fn(serial: Option<u32>) -> Box<dyn GumpBody>,
}

/// What a gump of the shard says it may not do now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GumpLocks {
    pub no_move: bool,
    pub no_close: bool,
}

/// What a gump does when the player closes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closing {
    /// It goes at once.
    Now,
    /// It stays until [`GumpBody::alive`] says it is gone, as a gump of the
    /// shard does until the shard takes it away.
    Wait,
}

/// One classic gump.
pub trait GumpBody {
    /// Draws the gump in its own pixels from its top left corner.
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>);

    /// Where the gump first opens, when it knows better than its kind.
    fn first_place(&self, _frame: &WatchFrame) -> Option<Pos2> {
        None
    }

    fn locks(&self, _frame: &WatchFrame) -> GumpLocks {
        GumpLocks::default()
    }

    /// The player closes the gump by a right click.
    fn close(&mut self, _cx: &mut GumpContext<'_>) -> Closing {
        Closing::Now
    }

    /// The gump waits for the player to press a key, so keys do not run
    /// macros meanwhile.
    fn wants_keys(&self) -> bool {
        false
    }

    /// False when the thing the gump shows is gone, and the gump closes.
    fn alive(&self, _frame: &WatchFrame) -> bool {
        true
    }

    /// The gump stays over every other gump, as a world map the player
    /// keeps on top.
    fn on_top(&self, _profile: &Profile) -> bool {
        false
    }
}

/// What a gump asks the manager to do after the frame.
pub enum GumpCommand {
    Open(GumpId),
    /// Opens a gump at a window place, or moves it there when it is open.
    OpenAt(GumpId, Pos2),
    /// Opens a gump with a body the asker made, such as a color picker for
    /// one option.
    OpenWith(GumpId, Box<dyn GumpBody>),
    Close(GumpId),
    Toggle(GumpId),
    /// Joins a gump to another of its anchor group, as a drop on it does.
    Join(GumpId, GumpId),
    /// The mouse, whose left button is down, drags a gump just opened, as
    /// a button dragged out of a book follows it.
    DragWithPointer(GumpId),
}

/// What a gump reads and acts through while it draws.
pub struct GumpContext<'a> {
    pub frame: &'a WatchFrame,
    pub hand: &'a Hand,
    pub tips: &'a mut Tips,
    pub profile: &'a mut Profile,
    /// Why the sound is off, when it is.
    pub sound_note: &'a str,
    /// The gump that draws now.
    pub me: GumpId,
    /// This gump is joined to others of its anchor group.
    pub anchored: bool,
    /// The item on the mouse, and the places that take a dropped item.
    pub desk: &'a mut Desk,
    /// Every journal line the window kept, for the journal gumps.
    pub journal: &'a JournalLog,
    /// The session read for the gumps, such as the named places of a map.
    pub readings: &'a mut Readings,
    pub(super) commands: &'a mut Vec<GumpCommand>,
    pub(super) macros: &'a mut Vec<Vec<MacroStep>>,
    pub(super) sounds: &'a mut Vec<u16>,
    pub(super) answers: &'a mut HashMap<String, u16>,
    pub(super) profile_changed: &'a mut bool,
    pub(super) save_default: &'a mut bool,
}

/// Folds or unfolds the gump kept under `key`. True when the profile
/// changed.
fn keep_folded(profile: &mut Profile, key: &str, folded: bool) -> bool {
    if folded {
        profile.folded.insert(key.to_string())
    } else {
        profile.folded.remove(key)
    }
}

/// The look kept for the gump under `key`: 0 when none is.
fn kept_look(profile: &Profile, key: &str) -> usize {
    profile.looks.get(key).map_or(0, |look| usize::from(*look))
}

/// Keeps the look of the gump under `key`; the first look is kept as none.
/// True when the profile changed.
fn keep_look(profile: &mut Profile, key: &str, look: usize) -> bool {
    let look = u8::try_from(look).unwrap_or(u8::MAX);
    let before = profile.looks.get(key).copied();
    if look == 0 {
        profile.looks.remove(key);
    } else {
        profile.looks.insert(key.to_string(), look);
    }
    profile.looks.get(key).copied() != before
}

impl GumpContext<'_> {
    /// The human has the character, so acts are sent.
    pub fn live(&self) -> bool {
        self.frame.human_control
    }

    /// Sends an act of the character, when the human has control.
    pub fn act(&self, act: Act) {
        if self.live() {
            self.hand.act(act);
        }
    }

    pub fn open(&mut self, id: GumpId) {
        self.commands.push(GumpCommand::Open(id));
    }

    /// Opens a gump with its top left corner at a window place.
    pub fn open_at(&mut self, id: GumpId, place: Pos2) {
        self.commands.push(GumpCommand::OpenAt(id, place));
    }

    pub fn open_with(&mut self, id: GumpId, body: Box<dyn GumpBody>) {
        self.commands.push(GumpCommand::OpenWith(id, body));
    }

    pub fn close(&mut self, id: GumpId) {
        self.commands.push(GumpCommand::Close(id));
    }

    pub fn toggle(&mut self, id: GumpId) {
        self.commands.push(GumpCommand::Toggle(id));
    }

    /// Joins this gump to `host`, at the side of it where it stands.
    pub fn join(&mut self, host: GumpId) {
        self.commands.push(GumpCommand::Join(self.me, host));
    }

    /// Lets the mouse drag a gump this frame opens, while its button is
    /// down.
    pub fn drag_with_pointer(&mut self, id: GumpId) {
        self.commands.push(GumpCommand::DragWithPointer(id));
    }

    /// True when a gump of this kind is registered, so a button can open it.
    pub fn has_kind(&self, id: &str) -> bool {
        kind(id).is_some()
    }

    /// True when this gump is folded small, as the profile keeps it.
    pub fn folded(&self) -> bool {
        self.profile.folded.contains(&self.me.place_key())
    }

    /// Folds or unfolds this gump, and keeps that in the profile.
    pub fn set_folded(&mut self, folded: bool) {
        if keep_folded(self.profile, &self.me.place_key(), folded) {
            self.profile_changed();
        }
    }

    /// The look this gump's button turned to, as the profile keeps it: 0
    /// for the first.
    pub fn look(&self) -> usize {
        kept_look(self.profile, &self.me.place_key())
    }

    /// Keeps the look this gump's button turned to in the profile.
    pub fn set_look(&mut self, look: usize) {
        if keep_look(self.profile, &self.me.place_key(), look) {
            self.profile_changed();
        }
    }

    /// The profile changed: the window keeps it and follows it at once.
    pub fn profile_changed(&mut self) {
        *self.profile_changed = true;
    }

    /// The window keeps the profile as the global default too, the start of
    /// each new character.
    pub fn save_as_default(&mut self) {
        *self.save_default = true;
    }

    /// Hands a picked value, such as a hue, to the gump that asked under
    /// `key`.
    pub fn answer(&mut self, key: &str, value: u16) {
        self.answers.insert(key.to_string(), value);
    }

    /// The value picked for `key`, once.
    pub fn take_answer(&mut self, key: &str) -> Option<u16> {
        self.answers.remove(key)
    }

    /// Runs the steps of a macro of the profile, as its key would.
    pub fn run_macro(&mut self, steps: Vec<MacroStep>) {
        self.macros.push(steps);
    }

    /// Plays a sound effect of the client, as a container does when it
    /// opens.
    pub fn play_sound(&mut self, sound: u16) {
        self.sounds.push(sound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gump_fold_and_look_are_kept_in_the_profile() {
        let key = GumpId::one(well_known::TOP_BAR).place_key();
        let mut profile = Profile::default();
        assert!(keep_folded(&mut profile, &key, true));
        assert!(!keep_folded(&mut profile, &key, true), "already folded");
        assert!(profile.folded.contains(&key));
        assert!(keep_folded(&mut profile, &key, false));
        assert!(profile.folded.is_empty());
        let buffs = GumpId::one(well_known::BUFFS).place_key();
        assert_eq!(kept_look(&profile, &buffs), 0);
        assert!(keep_look(&mut profile, &buffs, 3));
        assert!(!keep_look(&mut profile, &buffs, 3), "the same look");
        assert_eq!(kept_look(&profile, &buffs), 3);
        assert!(keep_look(&mut profile, &buffs, 0));
        assert!(profile.looks.is_empty(), "the first look is kept as none");
    }

    #[test]
    fn each_kind_has_its_own_id_and_places_have_their_names() {
        for (at, kind) in KINDS.iter().enumerate() {
            assert!(
                KINDS[at + 1..].iter().all(|other| other.id != kind.id),
                "{}",
                kind.id
            );
        }
        assert_eq!(GumpId::one("status").place_key(), "status");
        assert_eq!(GumpId::of("container", 0x3C).place_key(), "container:3C");
        assert!(kind(status::STATUS.id).is_some());
        assert!(kind("no such gump").is_none());
    }
}
