//! What an item in a classic gump does under the mouse, as the reference
//! client does: a single click asks its name once the double click time
//! is over, or targets it while the shard waits for a target; a drag picks
//! it up, or opens the split menu for a pile; the mouse on it shows its
//! tooltip. The container gump, the grid loot and the trade gump share it.

use super::canvas::{Canvas, ItemLook};
use super::registry::{GumpContext, GumpId};
use super::split_menu::{SplitMenu, SPLIT_MENU};
use crate::view::WatchPackItem;
use crate::window::model::clicks::asks_amount;
pub use crate::window::model::clicks::ClickDelay;
use crate::window::scene::Scene;
use eframe::egui::{Pos2, Response, Vec2};
use uoterm_nav::TileFlagSet;

/// The hue of the item under the mouse, and of the container whose gump
/// is under the mouse.
pub const HIGHLIGHT_HUE: u16 = 0x0035;
/// A pile shows its second picture this far down and right.
pub const PILE_OFFSET: i32 = 5;
/// Coins show one picture for a pile: their graphic shows the amount.
const COIN_GRAPHICS: [u16; 3] = [0x0EEA, 0x0EED, 0x0EF0];

/// True when items of this graphic stack into one pile.
pub fn is_stackable(scene: &Scene, graphic: u16) -> bool {
    scene
        .item_tile(graphic)
        .is_some_and(|tile| tile.flags.contains(TileFlagSet::STACKABLE))
}

/// True when items of this graphic hold other items.
pub fn is_container(scene: &Scene, graphic: u16) -> bool {
    scene
        .item_tile(graphic)
        .is_some_and(|tile| tile.flags.contains(TileFlagSet::CONTAINER))
}

/// True when an item shows as a pile of two pictures.
pub fn shows_pile(scene: &Scene, item: &WatchPackItem) -> bool {
    item.amount > 1 && !COIN_GRAPHICS.contains(&item.graphic) && is_stackable(scene, item.graphic)
}

/// How an item of a gump looks: in its hue, or marked under the mouse, as
/// a pile when it is one.
pub fn item_look(scene: &Scene, item: &WatchPackItem, marked: bool, scale: f32) -> ItemLook {
    ItemLook {
        graphic: item.graphic,
        hue: if marked { HIGHLIGHT_HUE } else { item.hue },
        whole_hue: marked,
        scale,
        pile_offset: shows_pile(scene, item).then_some(PILE_OFFSET),
    }
}

/// The time of the frame, in seconds.
pub fn now(g: &Canvas<'_>) -> f64 {
    g.ctx().input(|i| i.time)
}

/// Asks the name of the thing whose single click waited long enough, and
/// keeps the window drawing while one waits.
pub fn ask_waiting_name(g: &Canvas<'_>, cx: &GumpContext<'_>, clicks: &mut ClickDelay) {
    if let Some(act) = clicks.due_look(now(g)) {
        cx.act(act);
    }
    if clicks.is_waiting() {
        g.ctx().request_repaint();
    }
}

/// A single click on an item: it targets the item while the shard waits
/// for a target, and else asks its name when no double click follows.
pub fn single_click(g: &Canvas<'_>, cx: &GumpContext<'_>, clicks: &mut ClickDelay, serial: u32) {
    if let Some(act) = clicks.single_click(cx.frame, serial, now(g)) {
        cx.act(act);
    }
}

/// Shows the tooltip of the item drawn last: the words of the shard, or
/// its name until they come.
pub fn item_tooltip(g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, item: &WatchPackItem) {
    if !g.last_hovered() {
        return;
    }
    let words = cx
        .tips
        .lines_of(cx.hand, item.serial)
        .filter(|lines| !lines.is_empty())
        .map(|lines| lines.join("\n"))
        .unwrap_or_else(|| item.name.clone());
    g.tooltip(&words);
}

/// Picks up an item the player started to drag. A pile opens the split
/// menu, by the "Shift to split stacks" option. `center` is the middle of
/// the item picture in window points: with relative drag and drop the
/// item keeps its place against the mouse.
pub fn pick_up(g: &Canvas<'_>, cx: &mut GumpContext<'_>, item: &WatchPackItem, center: Pos2) {
    let (mouse, shift) = g
        .ui()
        .input(|i| (i.pointer.press_origin(), i.modifiers.shift));
    let Some(mouse) = mouse else {
        return;
    };
    let grab = if cx.profile.containers.relative_drag_and_drop {
        center - mouse
    } else {
        Vec2::ZERO
    };
    let splits = asks_amount(
        item.amount,
        is_stackable(g.scene, item.graphic),
        cx.profile.general.shift_to_split_stacks,
        shift,
    );
    if splits {
        cx.open_with(
            GumpId::of(SPLIT_MENU.id, item.serial),
            Box::new(SplitMenu::new(item.clone(), grab, mouse)),
        );
    } else {
        cx.desk.pick_up_at(item, grab);
    }
}

/// True when the player started to drag a control with the left button.
pub fn started_drag(response: &Response) -> bool {
    response.drag_started_by(eframe::egui::PointerButton::Primary)
}
