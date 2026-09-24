//! The paperdoll of a mobile, as the reference client draws it: the body gump with the gump of each worn
//! item in the order the client paints them, in their hues; the slots of
//! worn items at the sides; the title at the foot. The character's own
//! paperdoll has the button column (Help, Options, Log Out, Quests or the
//! Journal of old clients, Skills, Guild, Peace or War), the profile and
//! party scrolls, the minimize corner (the profile keeps it minimized for
//! the character) and, on shards with property lists,
//! the combat book and the book of racial abilities. Every paperdoll has
//! the Status button, the virtue gem and the profile scroll.
//!
//! The player drags a worn item off the doll and drops one on it to put it
//! on, on his own paperdoll and on one the shard lets him dress. With
//! "Show durability bars" on, a slot of the character's own paperdoll shows
//! how worn its item is.

use super::canvas::{ButtonArt, Canvas, ItemLook};
use super::doll_order::{
    backpack_gump, body_gump, equipment_gump, is_covered, is_female_body, is_gargoyle_body,
    own_backpack_gump, paint_order, Worn,
};
use super::health_bar::open_bar_at;
use super::item_control::{
    ask_waiting_name, item_tooltip, pick_up, single_click, started_drag, ClickDelay,
};
use super::manager::GumpManager;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::windows::PAPERDOLL_COMMAND;
use crate::view::{WatchEquip, WatchFrame, WatchLook, WatchPackItem};
use crate::window::control::Act;
use crate::window::desk::Zone;
use crate::window::model::dolls::{self, DollWatch, GUILD_COMMAND, QUESTS_COMMAND};
use crate::window::model::durability::{worn_wear, Wear};
use crate::window::scene::Scene;
use crate::window::settings::Profile;
use eframe::egui::{Color32, Id, Pos2, Rect, Sense, Vec2};
use uoterm_nav::TileFlagSet;
use uoterm_protocol::types::{
    LAYER_ARMS, LAYER_BACKPACK, LAYER_BEARD, LAYER_BRACELET, LAYER_CLOAK, LAYER_EARRINGS,
    LAYER_GLOVES, LAYER_HAIR, LAYER_HELMET, LAYER_NECKLACE, LAYER_ONE_HANDED, LAYER_PANTS,
    LAYER_RING, LAYER_ROBE, LAYER_SHOES, LAYER_TALISMAN, LAYER_TUNIC, LAYER_TWO_HANDED,
};

pub const PAPERDOLL: GumpKind = GumpKind {
    id: well_known::PAPERDOLL,
    rules: GumpRules::DEFAULT,
    open: |serial| Box::new(Paperdoll::restored(serial.unwrap_or_default())),
};

const BACKGROUND_OWN: u16 = 0x07D0;
const BACKGROUND_OTHER: u16 = 0x07D1;
const MINIMIZED: u16 = 0x07EE;
const BUTTON_X: i32 = 185;
const BUTTON_TOP: i32 = 44;
const BUTTON_STEP: i32 = 27;
const HELP: ButtonArt = ButtonArt::new(0x07EF, 0x07F0, 0x07F1);
const OPTIONS: ButtonArt = ButtonArt::new(0x07D6, 0x07D7, 0x07D8);
const LOG_OUT: ButtonArt = ButtonArt::new(0x07D9, 0x07DA, 0x07DB);
const JOURNAL: ButtonArt = ButtonArt::new(0x07DC, 0x07DD, 0x07DE);
const QUESTS: ButtonArt = ButtonArt::new(0x57B5, 0x57B7, 0x57B6);
const SKILLS: ButtonArt = ButtonArt::new(0x07DF, 0x07E0, 0x07E1);
const GUILD: ButtonArt = ButtonArt::new(0x57B2, 0x57B4, 0x57B3);
const PEACE: ButtonArt = ButtonArt::new(0x07E5, 0x07E6, 0x07E7);
const WAR: ButtonArt = ButtonArt::new(0x07E8, 0x07E9, 0x07EA);
const STATUS: ButtonArt = ButtonArt::new(0x07EB, 0x07EC, 0x07ED);
const STATUS_ROW: i32 = 7;
/// The Status button opens the status gump this far up and left of the
/// mouse.
const STATUS_FROM_MOUSE: Vec2 = Vec2::new(100.0, 25.0);
const SCROLL: u16 = 0x07D2;
const SCROLL_Y: i32 = 196;
const SCROLL_FIRST_X: i32 = 25;
const SCROLL_STEP: i32 = 14;
const VIRTUE_GEM: u16 = 0x0071;
const VIRTUE_GEM_AT: (i32, i32) = (80, 4);
const MINIMIZE_BOX: (i32, i32, i32, i32) = (228, 260, 16, 16);
const COMBAT_BOOK: u16 = 0x2B34;
const COMBAT_BOOK_AT: (i32, i32) = (156, 200);
const RACIAL_BOOK: u16 = 0x2B28;
const RACIAL_BOOK_AT: (i32, i32) = (23, 200);
const DOLL_AT: (i32, i32) = (8, 19);
/// The backpack moves left when the books show.
const BACKPACK_SHIFT_WITH_BOOKS: i32 = 6;
const TITLE_AT: (i32, i32) = (39, 262);
const TITLE_FONT: u8 = 1;
const TITLE_HUE: u16 = 0x0386;
const TITLE_WIDTH: u32 = 185;
/// Worn item hues keep their low bits; the top ones are flags.
const HUE_MASK: u16 = 0x3FFF;
/// A worn item the player holds over the doll shows at this opacity.
const HELD_ALPHA: f32 = 0.5;
// The slots of worn items at the sides.
const SLOT_BACK: u16 = 0x243A;
const SLOT_FRAME: u16 = 0x2344;
const SLOT_SIZE: (i32, i32) = (19, 20);
const SLOT_ITEM_SIDE: f32 = 18.0;
const SLOTS_TOP: i32 = 70;
const SLOT_STEP: i32 = 21;
const LEFT_SLOTS_X: i32 = 2;
const RIGHT_SLOTS_X: i32 = 162;
const LEFT_SLOTS: [(u8, &str); 9] = [
    (LAYER_HELMET, "Helmet"),
    (LAYER_EARRINGS, "Earrings"),
    (LAYER_NECKLACE, "Necklace"),
    (LAYER_RING, "Ring"),
    (LAYER_BRACELET, "Bracelet"),
    (LAYER_TUNIC, "Tunic"),
    (LAYER_ONE_HANDED, "OneHanded"),
    (LAYER_TWO_HANDED, "TwoHanded"),
    (LAYER_TALISMAN, "Talisman"),
];
const RIGHT_SLOTS: [(u8, &str); 6] = [
    (LAYER_ROBE, "Robe"),
    (LAYER_GLOVES, "Gloves"),
    (LAYER_PANTS, "Pants"),
    (LAYER_ARMS, "Arms"),
    (LAYER_CLOAK, "Cloak"),
    (LAYER_SHOES, "Shoes"),
];
const SLOT_WORDS: &str = "slot";
// The durability bar under a slot.
const DURABILITY_HEIGHT: i32 = 2;
const DURABILITY_GOOD: Color32 = Color32::from_rgb(0, 200, 0);
const DURABILITY_WARNING: Color32 = Color32::from_rgb(220, 0, 0);
const DURABILITY_BACK: Color32 = Color32::from_rgb(40, 40, 40);

/// What a double click on a picture of the paperdoll does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Picture {
    Profile,
    PartyManifest,
    VirtueGem,
    CombatBook,
    RacialBook,
}

/// A button of the column of the character's own paperdoll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Help,
    Options,
    LogOut,
    Journal,
    Quests,
    Skills,
    Guild,
    PeaceWar,
}

/// One picture on the doll: the body, a worn item, the backpack or a
/// worn item the player holds over it.
#[derive(Clone, Debug, PartialEq)]
struct Piece {
    gump: u16,
    /// Where it is drawn, from the left of the gump.
    x: i32,
    hue: u16,
    partial: bool,
    /// The worn item, and whether the player may lift it off.
    item: Option<(WatchEquip, bool)>,
    held: bool,
}

/// Opens the paperdoll the shard sends. In the Modern style the paperdoll
/// page shows it instead.
pub fn sync(
    manager: &mut GumpManager,
    frame: &WatchFrame,
    profile: &mut Profile,
    dolls: &mut DollWatch,
    classic: bool,
) {
    let Some(doll) = dolls.take(frame) else {
        return;
    };
    if classic {
        let id = GumpId::of(well_known::PAPERDOLL, doll.serial);
        manager.open_body(id, Box::new(Paperdoll::new(doll.serial)), profile);
    }
}

/// The look of a mobile, when the window knows it.
fn look_of(frame: &WatchFrame, serial: u32) -> Option<&WatchLook> {
    if serial == frame.serial {
        return Some(&frame.look);
    }
    frame
        .mobiles
        .iter()
        .find(|mobile| mobile.serial == serial)
        .map(|mobile| &mobile.look)
}

/// The animation number a worn item of this graphic shows. Zero for none.
fn item_anim(scene: &Scene, graphic: u16) -> u16 {
    scene.item_tile(graphic).map_or(0, |tile| tile.anim_id)
}

/// A worn item as the gumps hand items around.
fn pack_item(item: &WatchEquip) -> WatchPackItem {
    WatchPackItem {
        serial: item.serial,
        graphic: item.graphic,
        hue: item.hue,
        amount: 1,
        ..WatchPackItem::default()
    }
}

/// The paperdoll of one mobile.
pub struct Paperdoll {
    serial: u32,
    /// Opened from the places the profile kept, not by the shard.
    restored: bool,
    title: String,
    can_lift: bool,
    clicks: ClickDelay,
    /// The worn item the left button went down on, while it is down.
    pressed_on: Option<u32>,
}

impl Paperdoll {
    pub fn new(serial: u32) -> Self {
        Self {
            serial,
            restored: false,
            title: String::new(),
            can_lift: false,
            clicks: ClickDelay::default(),
            pressed_on: None,
        }
    }

    /// A paperdoll the profile kept open. The character's own asks the
    /// shard again; another's closes, as in the classic client.
    pub fn restored(serial: u32) -> Self {
        Self {
            restored: true,
            ..Self::new(serial)
        }
    }

    fn own(&self, frame: &WatchFrame) -> bool {
        self.serial == frame.serial
    }

    /// The player may take items off and put them on.
    fn dresses(&self, frame: &WatchFrame) -> bool {
        dolls::dresses(frame, self.serial, self.can_lift)
    }

    /// The pictures of the doll, first drawn first.
    fn pieces(&self, g: &mut Canvas<'_>, cx: &GumpContext<'_>, look: &WatchLook) -> Vec<Piece> {
        let frame = cx.frame;
        let female = is_female_body(look.body) || (self.own(frame) && frame.female);
        let body = body_gump(look.body, female);
        let mut pieces = vec![Piece {
            gump: body.gump,
            x: DOLL_AT.0,
            hue: body.hue.unwrap_or(look.hue),
            partial: true,
            item: None,
            held: false,
        }];
        if let Some(overlay) = body.overlay {
            pieces.push(Piece {
                gump: overlay,
                x: DOLL_AT.0,
                hue: look.hue,
                partial: true,
                item: None,
                held: false,
            });
        }
        let held = self.held_item(g, cx, look);
        let mut worn: Vec<WatchEquip> = look.equipment.clone();
        if let Some(held) = &held {
            worn.push(held.clone());
        }
        let gargoyle = is_gargoyle_body(look.body);
        let (order, covering) = {
            let scene = &*g.scene;
            let anim_of = |graphic: u16| item_anim(scene, graphic);
            (
                paint_order(&Worn::new(&worn, anim_of), female || gargoyle),
                Worn::new(&look.equipment, anim_of),
            )
        };
        let dresses = self.dresses(frame);
        for layer in order {
            let Some(item) = worn.iter().find(|item| item.layer == layer) else {
                continue;
            };
            let is_held = held.as_ref().is_some_and(|h| h.layer == layer);
            if !is_held && is_covered(&covering, layer, gargoyle) {
                continue;
            }
            let anim = item_anim(g.scene, item.graphic);
            let partial = g
                .scene
                .item_tile(item.graphic)
                .is_some_and(|tile| tile.flags.contains(TileFlagSet::PARTIAL_HUE));
            let converted = g.scene.equip_conv(look.body, anim).map(|conv| conv.gump);
            let exists = |gump: u16| g.gump_size(gump).is_some();
            let Some(gump) = equipment_gump(look.body, anim, female, converted, exists) else {
                continue;
            };
            let liftable = dresses && layer != LAYER_HAIR && layer != LAYER_BEARD;
            pieces.push(Piece {
                gump,
                x: DOLL_AT.0,
                hue: item.hue & HUE_MASK,
                partial,
                item: (!is_held).then(|| (item.clone(), liftable)),
                held: is_held,
            });
        }
        if let Some(pack) = look
            .equipment
            .iter()
            .find(|item| item.layer == LAYER_BACKPACK)
        {
            let anim = item_anim(g.scene, pack.graphic);
            let own_style = self
                .own(frame)
                .then(|| own_backpack_gump(cx.profile.containers.backpack_style))
                .filter(|gump| g.gump_size(*gump).is_some());
            if let Some(gump) =
                own_style.or_else(|| (anim != 0).then(|| backpack_gump(anim)).flatten())
            {
                let shift = if self.books_shown(g, frame) {
                    BACKPACK_SHIFT_WITH_BOOKS
                } else {
                    0
                };
                pieces.push(Piece {
                    gump,
                    x: DOLL_AT.0 - shift,
                    hue: pack.hue & HUE_MASK,
                    partial: false,
                    item: Some((pack.clone(), false)),
                    held: false,
                });
            }
        }
        pieces
    }

    /// The worn item the player holds over the doll, whose layer is free,
    /// shown faded where it would go.
    fn held_item(
        &self,
        g: &mut Canvas<'_>,
        cx: &GumpContext<'_>,
        look: &WatchLook,
    ) -> Option<WatchEquip> {
        if !self.dresses(cx.frame) {
            return None;
        }
        let background = g.gump_size(self.background(cx.frame)).unwrap_or(Vec2::ZERO);
        if !g.hovered(0, 0, background.x as i32, background.y as i32) {
            return None;
        }
        let carried = cx.desk.carried()?;
        let tile = g.scene.item_tile(carried.graphic)?;
        let layer = tile.quality;
        let free = !look.equipment.iter().any(|item| item.layer == layer);
        (tile.anim_id != 0 && layer != 0 && layer != LAYER_BACKPACK && free).then_some(WatchEquip {
            serial: carried.serial,
            graphic: carried.graphic,
            layer,
            hue: carried.hue,
        })
    }

    fn background(&self, frame: &WatchFrame) -> u16 {
        if self.own(frame) {
            BACKGROUND_OWN
        } else {
            BACKGROUND_OTHER
        }
    }

    /// The worn item on the doll under the mouse: the top one that draws
    /// the pixel there.
    fn piece_under_mouse(&self, g: &Canvas<'_>, pieces: &[Piece]) -> Option<usize> {
        let mouse = g.ui().input(|i| i.pointer.hover_pos())?;
        let scale = g.area(0, 0, Vec2::splat(1.0)).width();
        pieces.iter().enumerate().rev().find_map(|(at, piece)| {
            let local = (mouse - g.at(piece.x, DOLL_AT.1)) / scale;
            let inside = local.x >= 0.0 && local.y >= 0.0;
            let drawn = inside
                && g.scene
                    .gump_drawn_at(piece.gump, local.x as usize, local.y as usize);
            (piece.item.is_some() && drawn).then_some(at)
        })
    }

    /// Draws the doll and follows the mouse on its worn items.
    fn doll(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, look: &WatchLook) {
        let pieces = self.pieces(g, cx, look);
        let (down, pressed) = g
            .ui()
            .input(|i| (i.pointer.primary_down(), i.pointer.primary_pressed()));
        if !down && !pressed {
            self.pressed_on = None;
        }
        let under_mouse = self.piece_under_mouse(g, &pieces);
        for (at, piece) in pieces.iter().enumerate() {
            let x = piece.x;
            let Some((texture, sprite)) =
                g.scene
                    .hued_gump_picture(piece.gump, piece.hue, piece.partial)
            else {
                continue;
            };
            if piece.held {
                g.faded(HELD_ALPHA, |g| g.sprite(x, DOLL_AT.1, texture, sprite));
                continue;
            }
            g.sprite(x, DOLL_AT.1, texture, sprite);
            let Some((item, liftable)) = &piece.item else {
                continue;
            };
            let rect = g.area(x, DOLL_AT.1, Vec2::new(sprite.width, sprite.height));
            if item.layer == LAYER_BACKPACK {
                cx.desk.zone(rect, Zone::Into(item.serial));
            }
            let held_here = self.pressed_on == Some(item.serial) && down;
            if under_mouse != Some(at) && !held_here {
                continue;
            }
            item_tooltip(g, cx, &pack_item(item));
            let response = g.ui().interact(
                rect,
                Id::new(("paperdoll-item", self.serial, item.serial)),
                Sense::click_and_drag(),
            );
            if response.is_pointer_button_down_on() {
                self.pressed_on = Some(item.serial);
            }
            self.item_input(g, cx, &response, item, *liftable, rect.center());
        }
    }

    /// What the mouse does to one worn item: a click names it or targets
    /// it, a double click uses it, a drag lifts it off.
    fn item_input(
        &mut self,
        g: &Canvas<'_>,
        cx: &mut GumpContext<'_>,
        response: &eframe::egui::Response,
        item: &WatchEquip,
        liftable: bool,
        center: Pos2,
    ) {
        if started_drag(response) && liftable && !cx.desk.carries() {
            pick_up(g, cx, &pack_item(item), center);
        } else if response.double_clicked() {
            self.clicks.double_clicked();
            cx.act(Act::Use(item.serial));
        } else if response.clicked() {
            single_click(g, cx, &mut self.clicks, item.serial);
        }
    }

    /// The slots of worn items at the sides, with the durability of the
    /// character's own items when the Interface page shows it.
    fn slots(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, look: &WatchLook) {
        let own = self.own(cx.frame);
        let wears: Vec<Wear> = if own && cx.profile.interface.durability_bars {
            worn_wear(cx.frame, cx.readings)
        } else {
            Vec::new()
        };
        let warning = cx.profile.interface.durability_warning;
        let liftable = self.dresses(cx.frame);
        let columns = [
            (LEFT_SLOTS_X, &LEFT_SLOTS[..]),
            (RIGHT_SLOTS_X, &RIGHT_SLOTS[..]),
        ];
        for (x, slots) in columns {
            for (row, (layer, name)) in slots.iter().enumerate() {
                let y = SLOTS_TOP + row as i32 * SLOT_STEP;
                let (w, h) = SLOT_SIZE;
                g.pic_tiled(x, y, w, h, SLOT_BACK, 0);
                g.pic(x, y, SLOT_FRAME, 0);
                let Some(item) = look.equipment.iter().find(|item| item.layer == *layer) else {
                    g.tooltip_at(x, y, w, h, &format!("{name} {SLOT_WORDS}"));
                    continue;
                };
                self.slot_item(g, cx, item, (x, y), liftable);
                if let Some(wear) = wears.iter().find(|wear| wear.serial == item.serial) {
                    let color = if wear.warns(warning) {
                        DURABILITY_WARNING
                    } else {
                        DURABILITY_GOOD
                    };
                    let bottom = y + h - DURABILITY_HEIGHT;
                    g.fill(x, bottom, w, DURABILITY_HEIGHT, DURABILITY_BACK);
                    let shown = (wear.share() * w as f32).round() as i32;
                    g.fill(x, bottom, shown, DURABILITY_HEIGHT, color);
                }
            }
        }
    }

    /// The small picture of a worn item in its slot, made to fit.
    fn slot_item(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        item: &WatchEquip,
        (x, y): (i32, i32),
        liftable: bool,
    ) {
        let size = g.item_size(item.graphic);
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let scale = (SLOT_ITEM_SIDE / size.x)
            .min(SLOT_ITEM_SIDE / size.y)
            .min(1.0);
        let shown = size * scale;
        let at_x = x + ((SLOT_ITEM_SIDE - shown.x) / 2.0) as i32;
        let at_y = y + ((SLOT_ITEM_SIDE - shown.y) / 2.0) as i32;
        let look = ItemLook {
            graphic: item.graphic,
            hue: item.hue & HUE_MASK,
            whole_hue: false,
            scale,
            pile_offset: None,
        };
        let Some(response) = g.item_button(("slot", item.serial), at_x, at_y, look) else {
            return;
        };
        item_tooltip(g, cx, &pack_item(item));
        let center = response.rect.center();
        self.item_input(g, cx, &response, item, liftable, center);
    }

    fn books_shown(&self, g: &mut Canvas<'_>, frame: &WatchFrame) -> bool {
        self.own(frame) && frame.property_lists && g.gump_size(COMBAT_BOOK).is_some()
    }

    /// The racial book shows on clients that have its picture, for a race
    /// the shard named.
    fn racial_shown(&self, g: &mut Canvas<'_>, frame: &WatchFrame) -> bool {
        self.books_shown(g, frame) && frame.status.race != 0 && g.gump_size(RACIAL_BOOK).is_some()
    }

    /// The button column of the character's own paperdoll.
    fn buttons(&self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let journal_or_quests = if g.gump_size(QUESTS.normal).is_some() {
            (Press::Quests, QUESTS)
        } else {
            (Press::Journal, JOURNAL)
        };
        let war_art = if cx.frame.war { WAR } else { PEACE };
        let column = [
            (Press::Help, HELP),
            (Press::Options, OPTIONS),
            (Press::LogOut, LOG_OUT),
            journal_or_quests,
            (Press::Skills, SKILLS),
            (Press::Guild, GUILD),
            (Press::PeaceWar, war_art),
        ];
        for (row, (press, art)) in column.into_iter().enumerate() {
            let y = BUTTON_TOP + row as i32 * BUTTON_STEP;
            if g.button(("button", row), BUTTON_X, y, art) {
                pressed(press, cx);
            }
        }
    }

    /// The Status button: the character's status gump in place of his
    /// health bar, or the health bar of another.
    fn status_button(&self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let y = BUTTON_TOP + STATUS_ROW * BUTTON_STEP;
        if !g.button("status", BUTTON_X, y, STATUS) {
            return;
        }
        let mouse = g
            .ui()
            .input(|i| i.pointer.interact_pos())
            .unwrap_or_else(|| g.at(BUTTON_X, y));
        if self.own(cx.frame) {
            cx.close(GumpId::one(well_known::SELF_BAR));
            cx.open_at(GumpId::one(well_known::STATUS), mouse - STATUS_FROM_MOUSE);
        } else {
            open_bar_at(cx, self.serial, mouse);
        }
    }

    /// The scrolls, the virtue gem and the books, which a double click
    /// opens. Gives their boxes.
    fn pictures(&self, g: &mut Canvas<'_>, frame: &WatchFrame) -> Vec<(Rect, Picture)> {
        let own = self.own(frame);
        let racial = self.racial_shown(g, frame);
        let mut shown = Vec::new();
        let mut put = |g: &mut Canvas<'_>, (x, y): (i32, i32), gump: u16, picture: Picture| {
            let size = g.pic(x, y, gump, 0);
            shown.push((
                Rect::from_min_size(Pos2::new(x as f32, y as f32), size),
                picture,
            ));
        };
        let mut scroll_x = SCROLL_FIRST_X;
        if racial {
            scroll_x += SCROLL_STEP;
        }
        put(g, (scroll_x, SCROLL_Y), SCROLL, Picture::Profile);
        if own {
            put(
                g,
                (scroll_x + SCROLL_STEP, SCROLL_Y),
                SCROLL,
                Picture::PartyManifest,
            );
        }
        put(g, VIRTUE_GEM_AT, VIRTUE_GEM, Picture::VirtueGem);
        if self.books_shown(g, frame) {
            put(g, COMBAT_BOOK_AT, COMBAT_BOOK, Picture::CombatBook);
        }
        if racial {
            put(g, RACIAL_BOOK_AT, RACIAL_BOOK, Picture::RacialBook);
        }
        shown
    }

    fn open_picture(&self, picture: Picture, cx: &mut GumpContext<'_>) {
        match picture {
            Picture::Profile => {
                cx.act(Act::ProfileRead(self.serial));
                cx.open(GumpId::of(well_known::PROFILE, self.serial));
            }
            Picture::PartyManifest => cx.open(GumpId::one(well_known::PARTY)),
            Picture::VirtueGem => cx.act(Act::VirtueGump(self.serial)),
            Picture::CombatBook => cx.open(GumpId::one(well_known::COMBAT_BOOK)),
            Picture::RacialBook => cx.open(GumpId::one(well_known::RACIAL_ABILITIES)),
        }
    }

    /// The first frame of a kept paperdoll: the character's own asks the
    /// shard for it again, another's closes. False when it closed.
    fn restore(&mut self, cx: &mut GumpContext<'_>) -> bool {
        if !std::mem::take(&mut self.restored) {
            return true;
        }
        if self.own(cx.frame) {
            cx.act(Act::Command(PAPERDOLL_COMMAND.into()));
            return true;
        }
        cx.close(cx.me);
        false
    }
}

fn pressed(press: Press, cx: &mut GumpContext<'_>) {
    match press {
        Press::Help => cx.act(Act::Help),
        Press::Options => cx.open(GumpId::one(well_known::OPTIONS)),
        Press::LogOut => cx.open(GumpId::one(well_known::QUIT_QUESTION)),
        Press::Journal => cx.open(GumpId::one(well_known::JOURNAL)),
        Press::Quests => cx.act(Act::Command(QUESTS_COMMAND.into())),
        Press::Skills => cx.open(GumpId::one(well_known::SKILLS)),
        Press::Guild => cx.act(Act::Command(GUILD_COMMAND.into())),
        Press::PeaceWar => cx.act(Act::War(!cx.frame.war)),
    }
}

impl GumpBody for Paperdoll {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if !self.restore(cx) {
            return;
        }
        let frame = cx.frame;
        if let Some(doll) = frame.paperdoll.as_ref().filter(|d| d.serial == self.serial) {
            self.title.clone_from(&doll.text);
            self.can_lift = doll.can_lift;
        }
        ask_waiting_name(g, cx, &mut self.clicks);
        if cx.folded() {
            g.pic(0, 0, MINIMIZED, 0);
            if g.body_double_click() {
                cx.set_folded(false);
            }
            return;
        }
        let own = self.own(frame);
        let size = g.pic(0, 0, self.background(frame), 0);
        if own {
            cx.desk.zone(g.area(0, 0, size), Zone::Wear);
        } else if self.can_lift {
            cx.desk.zone(g.area(0, 0, size), Zone::Into(self.serial));
        }
        let pictures = self.pictures(g, frame);
        if let Some(look) = look_of(frame, self.serial).cloned() {
            self.slots(g, cx, &look);
            self.doll(g, cx, &look);
        }
        if own {
            self.buttons(g, cx);
            let (x, y, w, h) = MINIMIZE_BOX;
            if g.hit_box("minimize", x, y, w, h).clicked() {
                cx.set_folded(true);
            }
        }
        self.status_button(g, cx);
        let look = TextLook::ascii(TITLE_FONT, TITLE_HUE).wrap(TITLE_WIDTH);
        g.label(TITLE_AT.0, TITLE_AT.1, &self.title, &look);
        if g.body_double_click() {
            let clicked = g.body_click().and_then(|at| {
                pictures
                    .iter()
                    .find(|(rect, _)| rect.contains(at.to_pos2()))
                    .map(|(_, picture)| *picture)
            });
            if let Some(picture) = clicked {
                self.open_picture(picture, cx);
            }
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.restored || look_of(frame, self.serial).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::testing::draw_frames;

    const ME: u32 = 0x0000_0001;
    const SHIRT: u32 = 0x4000_0001;

    fn frame() -> WatchFrame {
        WatchFrame {
            serial: ME,
            name: "Mara".into(),
            human_control: true,
            look: WatchLook {
                body: 0x0190,
                hue: 0x83EA,
                equipment: vec![WatchEquip {
                    serial: SHIRT,
                    graphic: 0x1517,
                    layer: uoterm_protocol::types::LAYER_SHIRT,
                    hue: 0x0021,
                }],
                ..WatchLook::default()
            },
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_paperdoll_the_shard_sends_opens_once_in_the_classic_style() {
        use crate::view::WatchPaperdoll;
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let mut dolls = DollWatch::default();
        let mut frame = frame();
        sync(&mut manager, &frame, &mut profile, &mut dolls, true);
        frame.paperdoll = Some(WatchPaperdoll {
            serial: ME,
            text: "Mara the Brave".into(),
            seq: 1,
            can_lift: false,
        });
        sync(&mut manager, &frame, &mut profile, &mut dolls, false);
        assert!(!manager.is_open(&GumpId::of(well_known::PAPERDOLL, ME)));
        frame.paperdoll.as_mut().unwrap().seq = 2;
        sync(&mut manager, &frame, &mut profile, &mut dolls, true);
        assert!(manager.is_open(&GumpId::of(well_known::PAPERDOLL, ME)));
    }

    #[test]
    fn the_own_paperdoll_draws_with_the_client_files() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::of(well_known::PAPERDOLL, ME);
        manager.open_body(id, Box::new(Paperdoll::new(ME)), &mut profile);
        let frame = frame();
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&id));
        assert!(manager
            .drawn_of(well_known::PAPERDOLL)
            .iter()
            .any(|(open, rect)| *open == id && rect.width() > 0.0));
    }

    #[test]
    fn another_paperdoll_kept_from_before_closes_and_the_own_stays() {
        let frame = frame();
        assert!(Paperdoll::restored(ME).own(&frame));
        let other = Paperdoll::restored(0x0000_0099);
        assert!(!other.own(&frame));
        assert!(other.alive(&frame), "it closes itself in its first frame");
        assert!(!Paperdoll::new(0x0000_0099).alive(&frame));
    }
}
