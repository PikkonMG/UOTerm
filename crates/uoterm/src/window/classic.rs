//! The Classic look: floating gumps drawn from the gump art of the client
//! with its fonts, as the official client and the reference client show them. The
//! Modern style is the default style of the play window, and this one a
//! choice of the Interface page. In the Modern style only the gumps of the
//! shard float here; the rest of that style is the command deck.
//!
//! The parts:
//! - `registry`: the kinds of gumps ([`registry::KINDS`]), their rules,
//!   the [`registry::GumpBody`] trait and the [`registry::GumpContext`] a
//!   gump acts through.
//! - `manager`: the gump manager: order, drags, right clicks, locks, the
//!   places kept in the profile, anchoring, modal gumps and tooltips.
//! - `canvas`: the controls a gump draws with, from gump art: pictures,
//!   frames, buttons, check and radio boxes, sliders, scroll bars and
//!   areas, words and HTML, text boxes, drop-down lists, color boxes.
//! - `text`, `html`, `text_field`: UO fonts, the HTML of gumps, and the
//!   words of a text box.
//! - `layout`, `anchor`: the arithmetic, with no drawing.
//! - The gumps of this module: the status gump and the health bar of the
//!   character, the top bar, the Options gump, the color picker and the
//!   debug gump. The gumps of the shard live in `window/gump_ui.rs`.
//! - `windows`: the window commands of actions and keys, for this style.
//! - `shard_windows`: opens the gump of each window the shard opens (books,
//!   boards, map items, menus, text entry, prompts, chat, tips, questions).
//! - `context_menu`: the right-click menu of a gump, as the world map and
//!   the resizable journal have.
//! - The journals, the world map with its markers manager, marker box and
//!   go-to box, the minimap, the info bar, the network statistics, and the
//!   quest arrow over the game view. The world maps of both styles draw
//!   with `window/map_view.rs`.
//!
//! # Writing a new classic gump
//!
//! 1. Make a module under `classic/`, with a struct for what the gump keeps
//!    between frames, and `impl GumpBody for It`. `draw` gets a
//!    [`canvas::Canvas`] in the gump's own pixels from its top left corner
//!    (the manager scales and fades it), and the [`registry::GumpContext`]:
//!    `cx.frame` is the `WatchFrame`, `cx.act(Act::...)` acts when the
//!    human has control, `cx.open(id)` / `cx.close(id)` / `cx.toggle(id)`
//!    / `cx.open_at(id, place)` work on other gumps, `cx.profile` is the
//!    profile, `cx.me` is this gump.
//! 2. Draw with the canvas, as the reference client's controls: `g.pic`, `g.pic_tiled`,
//!    `g.pic_width`, `g.frame` (a framed box of 9 parts), `g.item`, `g.label`
//!    with a [`text::TextLook`] (`TextLook::ascii(font, hue)` or
//!    `TextLook::unicode(font, hue)`, then `.wrap(w)`, `.cropped(w)`,
//!    `.aligned(..)`, `.bordered()`), `g.html`, `g.button` with a
//!    [`canvas::ButtonArt`], `g.caption_button`, `g.nice_button`,
//!    `g.checkbox`, `g.radio`, `g.slider`, `g.scroll_bar`, `g.scroll_area`,
//!    `g.expandable_scroll`, `g.resize_grip`, `g.text_box` with a
//!    [`text_field::TextField`], `g.combobox`, `g.color_box`, `g.shade`,
//!    `g.fill`, `g.checker_trans`, `g.hit_box`, `g.tooltip`. Each control
//!    with input takes a key that is unique in the gump. Pictures and words
//!    drag the gump; controls take their own clicks. `g.body_click()`,
//!    `g.body_double_click()` and `g.right_click()` tell of clicks on the
//!    gump itself.
//! 3. Add `pub const KIND: GumpKind = GumpKind { id, rules, open }` with
//!    the id from [`registry::well_known`] when one names the gump, rules
//!    from [`registry::GumpRules::DEFAULT`] with the fields that differ
//!    (`right_click_closes`, `anchor`, `modal`, `both_styles`, `kept`,
//!    `first_place`), and `open: |serial| Box::new(It::new(serial))`. A
//!    kind with one gump for each thing (a container, a health bar) opens
//!    with `GumpId::of(kind, serial)`; its place is kept under
//!    `kind:SERIAL`. `GumpBody::alive` closes the gump when its thing is
//!    gone; `first_place`, `locks` and `close` change the rest.
//! 4. Put the kind in [`registry::KINDS`] and declare the module here. The
//!    top bar and the window commands find it by its id.

pub mod anchor;
pub mod book;
pub mod book_pages;
pub mod buff_gump;
pub mod bulletin_board;
pub mod bulletin_post;
pub mod canvas;
pub mod chat;
pub mod chat_line;
pub mod combat_book;
pub mod container;
pub mod container_data;
pub mod context_menu;
pub mod counter_bar;
pub mod debug;
pub mod doll_order;
pub mod grid_loot;
pub mod health_bar;
pub mod house;
pub mod html;
pub mod hue_picker;
pub mod ignore_list;
pub mod info_bar;
pub mod item_control;
pub mod journal;
pub mod layout;
pub mod macro_button;
pub mod macro_gump;
pub mod macro_steps;
pub mod manager;
pub mod map_item;
pub mod map_markers;
pub mod message_box;
pub mod minimap;
pub mod net_stats;
pub mod old_menu;
pub mod options;
pub mod paperdoll;
pub mod party;
pub mod popup_menu;
pub mod profile;
pub mod prompt;
pub mod quest_arrow;
pub mod race_change;
pub mod racial;
pub mod registry;
pub mod shard_windows;
pub mod shop;
pub mod skill_button;
pub mod skills;
pub mod spell_button;
pub mod spellbook;
pub mod split_menu;
pub mod status;
pub mod text;
pub mod text_entry;
pub mod text_field;
pub mod tip_notice;
pub mod top_bar;
pub mod trade;
pub mod viewport;
pub mod windows;
pub mod world_map;

#[cfg(test)]
pub mod testing;

pub use manager::GumpManager;

use super::control::Hand;
use super::desk::Desk;
use super::gump_ui;
use super::keys::chat::ChatLine;
use super::model::dolls::DollWatch;
use super::model::journal::{stamp_now, JournalLog};
use super::model::reads::Readings;
use super::scene::Scene;
use super::settings::{Profile, VideoOptions};
use super::tips::Tips;
use crate::view::WatchFrame;
use canvas::{Canvas, CanvasInput};
use eframe::egui::{self, Id, Pos2, Rect, Vec2};
use manager::{ManagerInputs, ManagerOutcome};
use registry::{well_known, GumpId};
use std::path::Path;
use text::{TextKit, UoFonts};
use uoterm_nav::GUMP_UOP_NAME;

/// The id of the canvas of the game window frame.
const VIEWPORT_ID: &str = "classic-viewport";
/// The id of the canvas of the chat line in the game window.
const CHAT_LINE_ID: &str = "classic-chat-line";

/// The gumps that open the first time a profile is used.
const FIRST_GUMPS: [GumpId; 1] = [GumpId::one(well_known::STATUS)];

/// What the classic layer draws with in one frame.
pub struct ClassicInputs<'a> {
    pub frame: &'a WatchFrame,
    pub scene: &'a mut Scene,
    pub hand: &'a Hand,
    pub tips: &'a mut Tips,
    pub profile: &'a mut Profile,
    pub desk: &'a mut Desk,
    pub readings: &'a mut Readings,
    pub time: f64,
    pub sound_note: &'a str,
    /// The Classic style shows every gump; the Modern one only the gumps
    /// of the shard.
    pub classic: bool,
}

/// Opens the one gump of a kind while `shown`, and closes it when not.
fn show_while(manager: &mut GumpManager, kind: &'static str, shown: bool, profile: &mut Profile) {
    let id = GumpId::one(kind);
    match (shown, manager.is_open(&id)) {
        (true, false) => {
            manager.open(id, profile);
        }
        (false, true) => manager.close(&id, profile),
        _ => {}
    }
}

/// The classic layer of the play window.
pub struct ClassicUi {
    /// None when the client files hold no fonts, and the classic look
    /// cannot show.
    kit: Option<TextKit>,
    manager: GumpManager,
    /// The gumps the profile keeps are open.
    started: bool,
    /// Every journal line since the window opened, for the journal gumps.
    journal: JournalLog,
    shard_windows: shard_windows::ShardWindows,
    /// Opens the container gumps as the shard opens containers.
    containers: container::ContainerSync,
    /// Opens the paperdolls the shard sends.
    dolls: DollWatch,
    /// The health bars the map opens: pulled off a mobile, drag-selected,
    /// and the bar of the last target.
    bars: health_bar::WorldBars,
    /// Opens the box of a party invite.
    invites: party::InviteWatch,
    /// The chat line at the foot of the game window.
    chat: chat_line::ClassicChat,
}

impl ClassicUi {
    pub fn new(uopath: Option<&Path>) -> Self {
        Self {
            kit: uopath.and_then(|dir| match UoFonts::open(dir) {
                Ok(fonts) => Some(TextKit::new(fonts, dir.join(GUMP_UOP_NAME).exists())),
                Err(e) => {
                    tracing::warn!(error = %e, "no fonts for the classic look");
                    None
                }
            }),
            manager: GumpManager::default(),
            started: false,
            journal: JournalLog::default(),
            shard_windows: shard_windows::ShardWindows::default(),
            containers: container::ContainerSync::default(),
            dolls: DollWatch::default(),
            bars: health_bar::WorldBars::default(),
            invites: party::InviteWatch::default(),
            chat: chat_line::ClassicChat::default(),
        }
    }

    /// True when the client files hold the fonts and the gump art the
    /// classic look draws with.
    pub fn ready(&self, scene: &Scene) -> bool {
        self.kit.is_some() && scene.has_gump_art()
    }

    /// Another profile came, for the character: the gumps it keeps open
    /// in its places.
    pub fn profile_replaced(&mut self) {
        self.manager.close_kept();
        self.started = false;
    }

    /// Opens a gump, or closes it when it is open.
    pub fn toggle(&mut self, id: GumpId, profile: &mut Profile) {
        self.manager.toggle(id, profile);
    }

    /// A gump waits for a key press, so keys do not run macros.
    pub fn wants_keys(&self) -> bool {
        self.manager.wants_keys()
    }

    /// The gump manager, for the window commands of this style.
    pub fn manager(&mut self) -> &mut GumpManager {
        &mut self.manager
    }

    /// Opens the gump of the shard's context menu for a thing at a window
    /// place, in the place of the last one.
    pub fn open_popup(&mut self, place: Pos2, serial: u32, profile: &mut Profile) {
        let id = popup_menu::POPUP_MENU_GUMP;
        self.manager.close(&id, profile);
        self.manager
            .open_body(id, Box::new(popup_menu::PopupMenu::new(serial)), profile);
        self.manager.open_at(id, place, profile);
    }

    /// Draws the frame of the game window round `view`, the world of the
    /// Classic style. True when the player moved or resized it, so the
    /// profile is kept.
    pub fn frame_view(
        &mut self,
        ui: &mut egui::Ui,
        view: Rect,
        screen: Rect,
        scene: &mut Scene,
        video: &mut VideoOptions,
    ) -> bool {
        let Some(text) = self.kit.as_mut() else {
            return false;
        };
        let input = CanvasInput {
            id: Id::new(VIEWPORT_ID),
            origin: view.min - Vec2::splat(viewport::BORDER),
            scale: 1.0,
            alpha: 1.0,
            pointer: ui.ctx().pointer_hover_pos(),
            body_click: None,
            body_double_click: false,
            right_click: false,
            size: None,
            map: 0,
        };
        let mut g = Canvas::new(ui, scene, text, input);
        viewport::frame(&mut g, view, screen, video)
    }

    /// Draws the chat line at the foot of the game window `view`. True
    /// while lines show over it, so the window draws again to let them go.
    pub fn chat_line(
        &mut self,
        ui: &mut egui::Ui,
        view: Rect,
        scene: &mut Scene,
        line: &mut ChatLine,
        inputs: chat_line::ChatInputs<'_>,
    ) -> bool {
        let Some(text) = self.kit.as_mut() else {
            return false;
        };
        let canvas_input = CanvasInput {
            id: Id::new(CHAT_LINE_ID),
            origin: view.min,
            scale: 1.0,
            alpha: 1.0,
            pointer: ui.ctx().pointer_hover_pos(),
            body_click: None,
            body_double_click: false,
            right_click: false,
            size: None,
            map: 0,
        };
        let size = (view.width() as i32, view.height() as i32);
        let mut g = Canvas::new(ui, scene, text, canvas_input);
        self.chat.draw(&mut g, size, line, inputs)
    }

    /// Draws the gumps. Gives where they are and whether the profile
    /// changed.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        inputs: ClassicInputs<'_>,
    ) -> ManagerOutcome {
        let Some(text) = self.kit.as_mut() else {
            return ManagerOutcome::default();
        };
        let ClassicInputs {
            frame,
            scene,
            hand,
            tips,
            profile,
            desk,
            readings,
            time,
            sound_note,
            classic,
        } = inputs;
        self.journal
            .take(frame, &stamp_now(), usize::from(profile.journal.max_lines));
        if classic {
            if !self.started {
                self.manager.open_kept(profile, &FIRST_GUMPS);
                self.started = true;
            }
            // The menu bar and the info bar show as the profile says.
            let top_bar = !profile.general.hide_menu_bar;
            show_while(&mut self.manager, well_known::TOP_BAR, top_bar, profile);
            let info_bar = profile.info_bar.enabled;
            show_while(&mut self.manager, well_known::INFO_BAR, info_bar, profile);
            self.shard_windows
                .sync(&mut self.manager, frame, hand, profile);
            self.containers
                .sync(&mut self.manager, frame, profile, scene, rect);
            shop::sync(&mut self.manager, frame, profile);
            counter_bar::sync(&mut self.manager, profile);
            trade::sync(&mut self.manager, frame, profile);
        }
        gump_ui::sync(&mut self.manager, frame, profile);
        hue_picker::sync_dye(&mut self.manager, frame, profile);
        paperdoll::sync(&mut self.manager, frame, profile, &mut self.dolls, classic);
        self.invites
            .follow(&mut self.manager, frame, profile, classic);
        self.bars
            .follow(ui, rect, &mut self.manager, scene, frame, profile, classic);
        self.manager.draw(
            ui,
            rect,
            ManagerInputs {
                frame,
                scene,
                text,
                hand,
                tips,
                profile,
                desk,
                journal: &self.journal,
                readings,
                time,
                sound_note,
                shared_only: !classic,
            },
        )
    }
}
