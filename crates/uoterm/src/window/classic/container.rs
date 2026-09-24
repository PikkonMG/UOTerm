//! The container gumps of the classic client: each open container is its own gump, drawn from
//! the picture the shard opened it with, with each item at the place the
//! shard gave it, in its art and hue; a pile shows two pictures. The player
//! drags items in, out and between them, onto the ground, the paperdoll or
//! a health bar; a pile asks how much to take. A double click uses an item,
//! or loots it with "Double click to loot items inside containers"; a
//! single click asks its name. The gump folds into its small picture, and
//! a corpse blinks its eye.
//!
//! The Containers page: the backpack style, the container scale and "scale
//! items", large containers, relative drag and drop, the mark of the
//! container under the mouse, container gumps in the hue of their item, and
//! where a new container opens. A corpse opens as this gump, as the grid
//! loot gump, or both, by the grid loot option of the General page.

use super::canvas::Canvas;
use super::container_data::{
    cascade, cascade_start, drop_spot, is_game_board, near_parent_gump, near_thing,
    overridden_place, shown_gump, Bounds, ContainerData, ContainerTable, BOARD_PIECE_OFFSET,
    CHESSBOARD_GUMP, CHESSBOARD_LIFT, MINIMIZER_SIDE,
};
use super::grid_loot::GRID_LOOT;
use super::item_control::{
    self, ask_waiting_name, item_look, item_tooltip, pick_up, single_click, started_drag,
    ClickDelay, HIGHLIGHT_HUE,
};
use super::manager::GumpManager;
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::spellbook::SPELLBOOK;
use crate::view::{WatchContainer, WatchFrame, WatchPackItem};
use crate::window::actions::modern::CORPSE_GUMP;
use crate::window::control::{Act, DropTo};
use crate::window::model::loot::{corpse_look, shows_corpse};
use crate::window::scene::Scene;
use crate::window::settings::{ContainerPlace, Profile};
use eframe::egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;
use std::sync::OnceLock;
use uoterm_protocol::types::{LAYER_BEARD, LAYER_FACE, LAYER_HAIR};

pub const CONTAINER: GumpKind = GumpKind {
    id: well_known::CONTAINER,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(ContainerGump::new(serial.unwrap_or_default())),
};

/// The gump the shard opens a spellbook with.
pub const SPELLBOOK_GUMP: u16 = 0xFFFF;
/// The gump the shard opens the boxes of a shopkeeper with; the shop gump
/// shows them.
pub const SHOP_GUMP: u16 = 0x0030;
const CORPSE_EYE: u16 = 0x0045;
const CORPSE_EYE_AT: (i32, i32) = (45, 30);
/// The eye of a corpse blinks this often, in seconds.
const CORPSE_EYE_SECONDS: f64 = 0.75;
const PERCENT: f32 = 100.0;
/// The layers of items a corpse holds in its gump; an item on another layer
/// is worn by the corpse and shows on its body, as the reference client
/// has them.
const CONTAINER_LAYERS: [bool; 30] = [
    false, true, true, true, true, true, true, true, true, true, true, false, true, true, true,
    false, false, true, true, true, true, false, true, true, true, false, false, false, false,
    false,
];

/// The table of container gumps, read once.
fn table() -> &'static ContainerTable {
    static TABLE: OnceLock<ContainerTable> = OnceLock::new();
    TABLE.get_or_init(ContainerTable::load)
}

/// The container scale of the Containers page, as a share.
fn container_scale(profile: &Profile, gump: u16) -> f32 {
    if is_game_board(gump) {
        1.0
    } else {
        f32::from(profile.containers.scale) / PERCENT
    }
}

/// The picture a container gump shows.
fn picture_of(
    scene: &mut Scene,
    frame: &WatchFrame,
    profile: &Profile,
    container: &WatchContainer,
) -> u16 {
    let options = &profile.containers;
    let backpack = (frame.backpack() == Some(container.serial)).then_some(options.backpack_style);
    shown_gump(container.gump, backpack, options.large_gumps, |picture| {
        scene.gump_picture(picture, 0).is_some()
    })
}

/// True when a container gump lists the item: a corpse leaves out what it
/// wears, and no gump lists hair, a beard or a face.
fn listed(scene: &Scene, item: &WatchPackItem, corpse: bool) -> bool {
    if item.amount == 0 {
        return false;
    }
    let tile_layer = scene.item_tile(item.graphic).map_or(0, |tile| tile.quality);
    let worn_on_head = matches!(tile_layer, LAYER_HAIR | LAYER_BEARD | LAYER_FACE);
    let worn_by_corpse = corpse
        && item.layer > 0
        && !CONTAINER_LAYERS
            .get(usize::from(tile_layer))
            .copied()
            .unwrap_or(false);
    !worn_on_head && !worn_by_corpse
}

/// The container that holds a container at the top: the thing on the
/// ground, on a mobile, or the character.
fn root_of(frame: &WatchFrame, serial: u32) -> u32 {
    let mut root = serial;
    let mut seen = 0;
    while let Some(parent) = frame
        .containers
        .iter()
        .find(|container| container.serial == root)
        .and_then(|container| container.parent)
    {
        root = parent;
        seen += 1;
        if seen > frame.containers.len() {
            break;
        }
    }
    root
}

/// Opens the gump of each container the shard opened, as the classic
/// client does when the open container packet comes, at the place the
/// Containers page asks for. A gump the player closed stays closed until
/// the shard opens its container again.
#[derive(Default)]
pub struct ContainerSync {
    /// The open count of each container when its gump was opened.
    shown: HashMap<u32, u64>,
    /// The last place of the cascade, in window points.
    cascade: Option<(f32, f32)>,
}

impl ContainerSync {
    pub fn sync(
        &mut self,
        manager: &mut GumpManager,
        frame: &WatchFrame,
        profile: &mut Profile,
        scene: &mut Scene,
        screen: Rect,
    ) {
        self.shown
            .retain(|serial, _| frame.containers.iter().any(|c| c.serial == *serial));
        for container in &frame.containers {
            if self.shown.get(&container.serial) == Some(&container.opened) {
                continue;
            }
            match container.gump {
                SHOP_GUMP => continue,
                SPELLBOOK_GUMP => {
                    manager.open(GumpId::of(SPELLBOOK.id, container.serial), profile);
                }
                CORPSE_GUMP => {
                    if !shows_corpse(&profile.general, container.items.len()) {
                        continue;
                    }
                    let look = corpse_look(profile.general.grid_loot);
                    if look.grid {
                        manager.open(GumpId::of(GRID_LOOT.id, container.serial), profile);
                    }
                    if look.plain {
                        self.open(manager, frame, profile, scene, screen, container);
                    }
                }
                _ => self.open(manager, frame, profile, scene, screen, container),
            }
            self.shown.insert(container.serial, container.opened);
        }
    }

    fn open(
        &mut self,
        manager: &mut GumpManager,
        frame: &WatchFrame,
        profile: &mut Profile,
        scene: &mut Scene,
        screen: Rect,
        container: &WatchContainer,
    ) {
        let id = GumpId::of(CONTAINER.id, container.serial);
        let options = profile.containers.clone();
        let remembers = options.override_place && options.place == ContainerPlace::RememberEach;
        if manager.is_open(&id) || (remembers && manager.remembered_place(&id, profile).is_some()) {
            manager.open(id, profile);
            return;
        }
        let picture = picture_of(scene, frame, profile, container);
        let zoom = profile.video.ui_scale;
        let size = scene
            .gump_picture(picture, 0)
            .map_or(Vec2::ZERO, |(_, sprite)| {
                Vec2::new(sprite.width, sprite.height)
            })
            * container_scale(profile, picture)
            * zoom;
        let size = (size.x, size.y);
        let room = (screen.width(), screen.height());
        let place = if options.override_place {
            let near = self.near(manager, frame, scene, screen, container, size, zoom);
            overridden_place(options.place, size, room, near, options.last_dragged)
                .unwrap_or_else(|| self.next_cascade(size, room, zoom))
        } else {
            self.next_cascade(size, room, zoom)
        };
        manager.open_at(id, screen.min + Vec2::new(place.0, place.1), profile);
    }

    fn next_cascade(&mut self, size: (f32, f32), room: (f32, f32), zoom: f32) -> (f32, f32) {
        let last = self.cascade.unwrap_or_else(|| cascade_start(zoom));
        let place = cascade(last, size, room, zoom);
        self.cascade = Some(place);
        place
    }

    /// The place near the thing that holds a container: the thing on the
    /// ground or the mobile that carries it, or the gump of the container
    /// it lies in.
    #[allow(clippy::too_many_arguments)]
    fn near(
        &self,
        manager: &GumpManager,
        frame: &WatchFrame,
        scene: &Scene,
        screen: Rect,
        container: &WatchContainer,
        size: (f32, f32),
        zoom: f32,
    ) -> Option<(f32, f32)> {
        let on_screen = |serial: u32| {
            scene
                .place_of(frame, serial)
                .map(|place| scene.screen_of(screen, place) - screen.min)
                .map(|at| (at.x, at.y))
        };
        let holder = match container.parent {
            None => container.serial,
            Some(parent) => {
                let parent_gump = GumpId::of(CONTAINER.id, parent);
                if let Some(gump) = manager.place_of(&parent_gump) {
                    let at = gump - screen.min;
                    return Some(near_parent_gump((at.x, at.y), size.0));
                }
                parent
            }
        };
        on_screen(holder).map(|at| near_thing(at, size.1, zoom))
    }
}

/// One container gump.
pub struct ContainerGump {
    serial: u32,
    minimized: bool,
    /// The open sound played.
    sounded: bool,
    clicks: ClickDelay,
    /// Where the gump was in the last frame, in window points.
    last_origin: Option<Pos2>,
}

impl ContainerGump {
    pub fn new(serial: u32) -> Self {
        Self {
            serial,
            minimized: false,
            sounded: false,
            clicks: ClickDelay::default(),
            last_origin: None,
        }
    }

    fn container<'f>(&self, frame: &'f WatchFrame) -> Option<&'f WatchContainer> {
        frame.containers.iter().find(|c| c.serial == self.serial)
    }

    /// Keeps the middle of the gump when the player dragged it, for the
    /// "Last dragged position" place.
    fn follow_drag(&mut self, g: &Canvas<'_>, cx: &mut GumpContext<'_>, size: Vec2) {
        let origin = g.at(0, 0);
        let moved = self.last_origin.is_some_and(|last| last != origin);
        self.last_origin = Some(origin);
        let options = &cx.profile.containers;
        let follows = options.override_place
            && matches!(
                options.place,
                ContainerPlace::LastDragged | ContainerPlace::RememberEach
            );
        let held = g.ui().input(|i| i.pointer.primary_down());
        if moved && follows && !held {
            let middle = origin + size * cx.profile.video.ui_scale / 2.0;
            cx.profile.containers.last_dragged = Some((middle.x, middle.y));
            cx.profile_changed();
        }
    }

    /// Lands the item on the mouse when the button comes up over the gump:
    /// into a container under the mouse, onto a pile of its kind, or at the
    /// place of the mouse.
    #[allow(clippy::too_many_arguments)]
    fn land(
        &self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        data: &ContainerData,
        scale: f32,
        board: bool,
        under: Option<&WatchPackItem>,
    ) {
        let Some((carried, grab)) = cx.desk.land(g.ui()) else {
            return;
        };
        let to = match under.filter(|item| item.serial != carried.serial) {
            Some(item) if item_control::is_container(g.scene, item.graphic) => {
                DropTo::Into(item.serial)
            }
            Some(item)
                if item.graphic == carried.graphic
                    && item_control::is_stackable(g.scene, item.graphic) =>
            {
                DropTo::IntoAt {
                    container: item.serial,
                    x: item.x,
                    y: item.y,
                }
            }
            _ => {
                let Some(mouse) = g.ui().input(|i| i.pointer.interact_pos()) else {
                    return;
                };
                let zoom = cx.profile.video.ui_scale;
                let at = (mouse + grab - g.at(0, 0)) / zoom;
                let lift = if board && self.is_chessboard(cx.frame, g) {
                    CHESSBOARD_LIFT
                } else {
                    0
                };
                let picture = if board {
                    g.gump_size(carried.graphic.wrapping_sub(BOARD_PIECE_OFFSET))
                } else {
                    Some(g.item_size(carried.graphic))
                }
                .unwrap_or(Vec2::ZERO)
                    * if cx.profile.containers.scale_items && !board {
                        scale
                    } else {
                        1.0
                    };
                let bounds = Bounds {
                    bottom: data.bounds.bottom + lift,
                    ..data.bounds
                };
                let (x, y) = drop_spot(
                    bounds,
                    scale,
                    (at.x as i32, at.y as i32 + lift),
                    (picture.x as i32, picture.y as i32),
                );
                DropTo::IntoAt {
                    container: self.serial,
                    x,
                    y,
                }
            }
        };
        cx.act(Act::Move {
            item: carried.serial,
            amount: carried.amount.max(1),
            to,
        });
    }

    fn is_chessboard(&self, frame: &WatchFrame, g: &mut Canvas<'_>) -> bool {
        self.container(frame)
            .is_some_and(|container| container.gump == CHESSBOARD_GUMP)
            && g.gump_size(CHESSBOARD_GUMP).is_some()
    }

    /// A double click on an item: it loots an item of a container that is
    /// not the backpack, with the option on and Ctrl up, and uses it else.
    fn double_click(&self, g: &Canvas<'_>, cx: &GumpContext<'_>, item: &WatchPackItem) {
        if cx.frame.target_cursor {
            return;
        }
        let ctrl = g.ui().input(|i| i.modifiers.ctrl);
        let root = root_of(cx.frame, self.serial);
        let in_own_pack = cx.frame.backpack() == Some(root) || root == cx.frame.serial;
        let empty = !cx.frame.containers.iter().any(|c| c.serial == item.serial);
        let loots = !ctrl
            && cx.profile.containers.double_click_loots
            && !item_control::is_container(g.scene, item.graphic)
            && empty
            && !in_own_pack;
        match cx.hand.grab_bag().filter(|_| loots) {
            Some(bag) => cx.act(Act::Move {
                item: item.serial,
                amount: item.amount.max(1),
                to: DropTo::Into(bag),
            }),
            None => cx.act(Act::Use(item.serial)),
        }
    }
}

impl GumpBody for ContainerGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(container) = self.container(cx.frame).cloned() else {
            return;
        };
        let gump = picture_of(g.scene, cx.frame, cx.profile, &container);
        let data = table().get(gump);
        if !self.sounded {
            self.sounded = true;
            if data.open_sound != 0 {
                cx.play_sound(data.open_sound);
            }
        }
        let board = is_game_board(gump);
        let scale = container_scale(cx.profile, gump);
        let options = cx.profile.containers.clone();
        let hue = if options.hue_gumps && gump != CORPSE_GUMP {
            container.hue
        } else {
            0
        };
        let picture = if self.minimized { data.iconized } else { gump };
        let size = g.gump_size(picture).unwrap_or(Vec2::ZERO) * scale;
        self.follow_drag(g, cx, size);
        let mut under = None;
        g.scaled(scale, |g| {
            g.pic(0, 0, picture, hue);
            if self.minimized {
                if g.body_double_click() {
                    self.minimized = false;
                }
                return;
            }
            if gump == CORPSE_GUMP {
                let blink = (item_control::now(g) / CORPSE_EYE_SECONDS) as u64 % 2;
                g.pic(
                    CORPSE_EYE_AT.0,
                    CORPSE_EYE_AT.1,
                    CORPSE_EYE + blink as u16,
                    0,
                );
                g.ctx().request_repaint();
            }
            if let (Some((x, y)), true) = (data.minimizer, data.iconized != 0) {
                if g.hit_box("minimize", x, y, MINIMIZER_SIDE, MINIMIZER_SIDE)
                    .clicked()
                {
                    self.minimized = true;
                }
            }
            let lift = if gump == CHESSBOARD_GUMP {
                CHESSBOARD_LIFT
            } else {
                0
            };
            let item_scale = if options.scale_items {
                1.0
            } else {
                1.0 / scale
            };
            let marked_container = cx.desk.marked_container();
            let corpse = gump == CORPSE_GUMP;
            let items: Vec<&WatchPackItem> = container
                .items
                .iter()
                .filter(|item| listed(g.scene, item, corpse))
                .collect();
            for item in items {
                let (x, y) = (i32::from(item.x as i16), i32::from(item.y as i16) - lift);
                let piece = item.graphic.wrapping_sub(BOARD_PIECE_OFFSET);
                let size = if board {
                    g.gump_size(piece).unwrap_or(Vec2::ZERO)
                } else {
                    g.item_size(item.graphic) * item_scale
                };
                // The pointer, not the hover of egui, which a drag keeps on
                // the item it started on.
                let pointed = g.hovered(x, y, size.x as i32, size.y as i32);
                if pointed {
                    under = Some(item.clone());
                }
                let marked = pointed || marked_container == Some(item.serial);
                let response = if board {
                    let hue = if marked { HIGHLIGHT_HUE } else { item.hue };
                    Some(g.pic_button(("item", item.serial), x, y, piece, hue))
                } else {
                    let look = item_look(g.scene, item, marked, item_scale);
                    g.item_button(("item", item.serial), x, y, look)
                };
                let Some(response) = response else {
                    continue;
                };
                item_tooltip(g, cx, item);
                if started_drag(&response) {
                    pick_up(g, cx, item, response.rect.center());
                } else if response.double_clicked() {
                    self.clicks.double_clicked();
                    self.double_click(g, cx, item);
                } else if response.clicked() {
                    single_click(g, cx, &mut self.clicks, item.serial);
                }
            }
        });
        ask_waiting_name(g, cx, &mut self.clicks);
        let whole = (size.x as i32, size.y as i32);
        if g.hovered(0, 0, whole.0, whole.1) {
            if options.highlight_on_hover {
                cx.desk.hover_container(self.serial);
            }
            if cx.desk.carried().is_some() && !self.minimized {
                self.land(g, cx, &data, scale, board, under.as_ref());
            }
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        if let Some(container) = self.container(cx.frame) {
            let gump = container.gump;
            for item in &container.items {
                cx.close(GumpId::of(CONTAINER.id, item.serial));
            }
            let sound = table().get(gump).close_sound;
            if sound != 0 {
                cx.play_sound(sound);
            }
        }
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.container(frame).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchEquip, WatchLook};
    use crate::window::classic::testing::{draw_frames, draw_with_input};
    use crate::window::desk::Desk;
    use crate::window::settings::GridLoot;
    use eframe::egui::{Event, Modifiers, PointerButton};
    use uoterm_protocol::types::LAYER_BACKPACK;

    const PACK: u32 = 0x4000_0100;
    const CORPSE: u32 = 0x4000_0200;
    const BOOK: u32 = 0x4000_0300;
    const VENDOR_BOX: u32 = 0x4000_0400;
    const GOLD: u32 = 0x4000_0500;
    const GOLD_GRAPHIC: u16 = 0x0EED;
    const GOLD_AT: (u16, u16) = (60, 80);
    const BACKPACK_GUMP: u16 = 0x003C;

    fn container(serial: u32, gump: u16, opened: u64, items: Vec<WatchPackItem>) -> WatchContainer {
        WatchContainer {
            serial,
            gump,
            opened,
            items,
            ..WatchContainer::default()
        }
    }

    fn frame(opened: u64) -> WatchFrame {
        let gold = WatchPackItem {
            serial: GOLD,
            graphic: GOLD_GRAPHIC,
            amount: 1,
            x: GOLD_AT.0,
            y: GOLD_AT.1,
            ..WatchPackItem::default()
        };
        WatchFrame {
            look: WatchLook {
                equipment: vec![WatchEquip {
                    serial: PACK,
                    layer: LAYER_BACKPACK,
                    ..WatchEquip::default()
                }],
                ..WatchLook::default()
            },
            containers: vec![
                container(PACK, BACKPACK_GUMP, opened, vec![gold]),
                container(CORPSE, CORPSE_GUMP, 2, vec![WatchPackItem::default()]),
                container(BOOK, SPELLBOOK_GUMP, 3, Vec::new()),
                container(VENDOR_BOX, SHOP_GUMP, 4, Vec::new()),
            ],
            ..WatchFrame::default()
        }
    }

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))
    }

    #[test]
    fn each_container_opens_its_gump_and_a_closed_one_waits_for_the_shard() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        profile.general.grid_loot = GridLoot::Both;
        let mut scene = Scene::new(None);
        let mut sync = ContainerSync::default();
        sync.sync(&mut manager, &frame(1), &mut profile, &mut scene, screen());
        let pack = GumpId::of(CONTAINER.id, PACK);
        assert!(manager.is_open(&pack));
        assert!(manager.is_open(&GumpId::of(CONTAINER.id, CORPSE)));
        assert!(manager.is_open(&GumpId::of(GRID_LOOT.id, CORPSE)));
        assert!(manager.is_open(&GumpId::of(SPELLBOOK.id, BOOK)));
        assert!(!manager.is_open(&GumpId::of(CONTAINER.id, VENDOR_BOX)));
        // The first gump opens at the first step of the cascade.
        assert_eq!(manager.place_of(&pack), Some(Pos2::new(60.0, 60.0)));
        manager.close(&pack, &mut profile);
        sync.sync(&mut manager, &frame(1), &mut profile, &mut scene, screen());
        assert!(!manager.is_open(&pack));
        sync.sync(&mut manager, &frame(5), &mut profile, &mut scene, screen());
        assert!(manager.is_open(&pack));
    }

    #[test]
    fn grid_loot_only_opens_no_plain_corpse_gump() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        profile.general.grid_loot = GridLoot::GridOnly;
        let mut sync = ContainerSync::default();
        sync.sync(
            &mut manager,
            &frame(1),
            &mut profile,
            &mut Scene::new(None),
            screen(),
        );
        assert!(manager.is_open(&GumpId::of(GRID_LOOT.id, CORPSE)));
        assert!(!manager.is_open(&GumpId::of(CONTAINER.id, CORPSE)));
    }

    #[test]
    fn the_root_of_a_bag_is_what_holds_its_holder() {
        let mut frame = frame(1);
        frame.containers[1].parent = Some(PACK);
        frame.containers[0].parent = Some(0x0000_0010);
        assert_eq!(root_of(&frame, CORPSE), 0x0000_0010);
        assert_eq!(root_of(&frame, 0x4000_0999), 0x4000_0999);
    }

    #[test]
    fn a_container_gump_draws_its_items_and_a_drag_picks_one_up() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let pack = GumpId::of(CONTAINER.id, PACK);
        let place = Pos2::new(100.0, 100.0);
        manager.open_at(pack, place, &mut profile);
        let frame = frame(1);
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&pack));
        let drawn = manager.drawn_of(CONTAINER.id);
        assert!(drawn
            .iter()
            .any(|(id, rect)| *id == pack && rect.width() > 0.0));
        let on_gold = place + Vec2::new(f32::from(GOLD_AT.0) + 6.0, f32::from(GOLD_AT.1) + 6.0);
        let press = |pressed| Event::PointerButton {
            pos: on_gold,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        };
        let mut desk = Desk::default();
        let frames = vec![
            vec![Event::PointerMoved(on_gold)],
            vec![Event::PointerMoved(on_gold)],
            vec![press(true)],
            vec![Event::PointerMoved(on_gold + Vec2::new(20.0, 0.0))],
            vec![Event::PointerMoved(on_gold + Vec2::new(40.0, 0.0))],
        ];
        draw_with_input(&mut manager, &mut profile, &frame, &mut desk, &frames);
        assert_eq!(desk.carried().map(|item| item.serial), Some(GOLD));
    }
}
