//! The two ability panels of the Modern style, with what the classic
//! combat book and racial book do. The combat panel shows the primary and
//! the secondary ability of the weapon in hand, red while armed: a click
//! arms one or lets it go, and each pins or drags onto the hotbar. Under
//! them it lists every weapon ability with the weapons that have it. The
//! racial panel shows the abilities of the character's race; the
//! gargoyle's flight flies or lands, and pins onto the hotbar.

use super::super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::super::deck_ui::{Offer, ROW, TAB_GAP};
use super::super::model::abilities::{
    ability_of, armed, icon_of, race_of, slot_hue, AbilitySlot, Race,
};
use super::super::settings::Profile;
use super::super::theme::{self, text_font, title_font};
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use uoterm_view::ui::abilities::{
    abilities_first_place, ability_rows, arm_words, racial_act, racial_first_place, racial_height,
    racial_slot, slot_act, slot_of, ABILITIES_MIN, ABILITY_ICON as ICON_SIDE, HINT_FLIGHT,
    HINT_SLOT, WORDS_ABILITIES, WORDS_ALL, WORDS_ARMED, WORDS_NO_RACE, WORDS_PASSIVE, WORDS_RACIAL,
    WORDS_RACIAL_USE as WORDS_USE, WORDS_WEAPONS,
};
pub use uoterm_view::ui::abilities::{ABILITIES_ID, RACIAL_ID};
use uoterm_view::ui::lists::{ability_slot_words, ability_weapon_names};
use uoterm_view::ui::sheet::WORDS_PIN;

const BUTTON_WIDTH: f32 = 72.0;
/// The buttons under the name of an ability.
const BUTTON_HEIGHT: f32 = ROW - TAB_GAP;

/// A gump icon in a cell.
fn icon(ui: &egui::Ui, tools: &mut Tools<'_>, cell: Rect, gump: u16, hue: u16) {
    ui.painter()
        .rect_filled(cell, CornerRadius::same(CELL_RADIUS), theme::TRACK);
    if let Some((texture, sprite)) = tools.scene.gump_picture(gump, hue) {
        ui.painter().image(
            texture,
            theme::fit(cell, sprite.width, sprite.height),
            bridge::rect(sprite.uv),
            Color32::WHITE,
        );
    }
}

/// The spec of a panel that first opens at `default`.
fn spec<'a>(
    (id, title): (&'a str, &'a str),
    default: egui::Rect,
    min: Option<Vec2>,
) -> PanelSpec<'a> {
    PanelSpec {
        id,
        title,
        default,
        min_size: min,
        closable: true,
    }
}

/// Two buttons side by side under the name beside an icon. Gives the one
/// pressed.
fn button_pair(
    ui: &egui::Ui,
    cell: Rect,
    key: (&'static str, u16),
    words: [&str; 2],
) -> Option<usize> {
    let first = Rect::from_min_size(
        Pos2::new(cell.right() + theme::ROW_GAP, cell.bottom() - BUTTON_HEIGHT),
        Vec2::new(BUTTON_WIDTH, BUTTON_HEIGHT),
    );
    let mut pressed = None;
    for (at, words) in words.into_iter().enumerate() {
        let area = first.translate(Vec2::new(at as f32 * (BUTTON_WIDTH + TAB_GAP), 0.0));
        let color = if at == 0 { theme::GOAL } else { theme::TEXT };
        if theme::segment_keyed(ui, area, Id::new((key, at)), words, color) {
            pressed = Some(at);
        }
    }
    pressed
}

/// Draws the combat panel. Gives its place, whether the player closed it,
/// and what he pinned or dragged toward the hotbar.
pub fn abilities_panel(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
    first_row: &mut usize,
) -> (Rect, bool, Option<Offer>) {
    let spec = spec(
        (ABILITIES_ID, WORDS_ABILITIES),
        bridge::rect(abilities_first_place(bridge::area(rect))),
        Some(bridge::vec2(ABILITIES_MIN)),
    );
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_ABILITIES);
    let mut offer = None;
    for (at, slot) in AbilitySlot::BOTH.into_iter().enumerate() {
        let card = Rect::from_min_size(
            body.left_top() + Vec2::new(0.0, at as f32 * (ICON_SIDE + theme::ROW_GAP)),
            Vec2::new(body.width(), ICON_SIDE),
        );
        offer = offer.or(slot_card(ui, card, slot, frame, tools));
    }
    let title_top = body.top() + (ICON_SIDE + theme::ROW_GAP) * AbilitySlot::BOTH.len() as f32;
    ui.painter().text(
        Pos2::new(body.left(), title_top),
        Align2::LEFT_TOP,
        WORDS_ALL,
        title_font(theme::SIZE_SMALL),
        theme::GOAL,
    );
    let list = Rect::from_min_max(Pos2::new(body.left(), title_top + ROW), body.max);
    every_ability(ui, list, frame, tools, first_row);
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    (panel, closed, offer)
}

/// The primary or the secondary ability: its icon, red while armed, its
/// name, and Arm and Pin.
fn slot_card(
    ui: &egui::Ui,
    card: Rect,
    slot: AbilitySlot,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
) -> Option<Offer> {
    let live = frame.human_control;
    let cell = Rect::from_min_size(card.min, Vec2::splat(ICON_SIDE));
    icon(
        ui,
        tools,
        cell,
        icon_of(ability_of(frame, slot)),
        slot_hue(frame, slot),
    );
    let is_armed = armed(frame, slot);
    let words = ability_slot_words(frame, slot);
    let beside = cell.right() + theme::ROW_GAP;
    ui.painter().text(
        Pos2::new(beside, cell.top()),
        Align2::LEFT_TOP,
        &words,
        text_font(theme::SIZE_BODY),
        theme::TEXT,
    );
    if is_armed {
        ui.painter().text(
            card.right_top(),
            Align2::RIGHT_TOP,
            WORDS_ARMED,
            text_font(theme::SIZE_SMALL),
            theme::ALARM,
        );
    }
    if !live {
        return None;
    }
    let toggle = || tools.hand.act(slot_act(frame, slot));
    let response = ui.interact(
        cell,
        Id::new(("ability-slot", slot.serial())),
        Sense::click_and_drag(),
    );
    if response.hovered() && !response.dragged() {
        super::super::tips::label(ui, &words, HINT_SLOT);
    }
    let pressed = button_pair(
        ui,
        cell,
        ("ability", slot.serial() as u16),
        [arm_words(is_armed), WORDS_PIN],
    );
    if pressed == Some(0) || response.clicked() {
        toggle();
    }
    if response.drag_started() {
        return Some(Offer::Drag(slot_of(slot)));
    }
    (pressed == Some(1)).then_some(Offer::Pin(slot_of(slot)))
}

/// Every weapon ability, with its icon; the ones of the weapon in hand are
/// marked. The weapons that have one show under the mouse.
fn every_ability(
    ui: &egui::Ui,
    list: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    first_row: &mut usize,
) {
    let rows = ((list.height() / ROW).floor() as usize).max(1);
    let every = ability_rows(frame);
    let last_first = every.len().saturating_sub(rows);
    *first_row = scrolled(ui, list, (*first_row).min(last_first), last_first);
    for (at, ability) in every.iter().skip(*first_row).take(rows).enumerate() {
        let row = Rect::from_min_size(
            list.left_top() + Vec2::new(0.0, at as f32 * ROW),
            Vec2::new(list.width(), ROW - TAB_GAP / 2.0),
        );
        let cell = Rect::from_min_size(row.min, Vec2::splat(row.height()));
        icon(ui, tools, cell, ability.icon, 0);
        let color = if ability.in_hand.is_some() {
            theme::GOAL
        } else {
            theme::TEXT
        };
        ui.painter().text(
            Pos2::new(cell.right() + theme::ROW_GAP, row.center().y),
            Align2::LEFT_CENTER,
            ability.name,
            text_font(theme::SIZE_SMALL),
            color,
        );
        if let Some(slot) = ability.in_hand {
            ui.painter().text(
                row.right_center(),
                Align2::RIGHT_CENTER,
                slot.title(),
                text_font(theme::SIZE_SMALL),
                theme::GOAL,
            );
        }
        let response = ui.interact(
            row,
            Id::new(("every-ability", ability.ability)),
            Sense::hover(),
        );
        if response.hovered() {
            let weapons = format!(
                "{WORDS_WEAPONS}{}",
                ability_weapon_names(ability.ability, |graphic| {
                    tools.scene.item_tile(graphic).map(|tile| tile.name.clone())
                })
            );
            super::super::tips::label(ui, ability.name, &weapons);
        }
    }
}

/// Draws the racial panel. Gives its place, whether the player closed it,
/// and what he pinned or dragged toward the hotbar.
pub fn racial_panel(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> (Rect, bool, Option<Offer>) {
    let race = race_of(frame);
    let height = racial_height(race);
    let spec = spec(
        (RACIAL_ID, WORDS_RACIAL),
        bridge::rect(racial_first_place(bridge::area(rect), race)),
        None,
    );
    let placed = frame::place(rect, &spec, profile);
    // The panel keeps its place and takes the height of the race.
    let panel = Rect::from_min_size(placed.min, Vec2::new(placed.width(), height));
    let body = frame::draw(ui.painter(), panel, WORDS_RACIAL);
    let offer = match race {
        Some(race) => race_rows(ui, body, race, frame, tools),
        None => {
            ui.painter().text(
                body.left_top(),
                Align2::LEFT_TOP,
                WORDS_NO_RACE,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            None
        }
    };
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    (panel, closed, offer)
}

/// Each ability of the race: its icon and name, and Passive, or Use and
/// Pin for the flight.
fn race_rows(
    ui: &egui::Ui,
    body: Rect,
    race: &Race,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
) -> Option<Offer> {
    let mut offer = None;
    for (index, name) in race.names.iter().enumerate() {
        let row = Rect::from_min_size(
            body.left_top() + Vec2::new(0.0, index as f32 * (ICON_SIDE + theme::ROW_GAP)),
            Vec2::new(body.width(), ICON_SIDE),
        );
        let cell = Rect::from_min_size(row.min, Vec2::splat(ICON_SIDE));
        let gump = race.icon(index);
        icon(ui, tools, cell, gump, 0);
        let beside = cell.right() + theme::ROW_GAP;
        ui.painter().text(
            Pos2::new(beside, row.top()),
            Align2::LEFT_TOP,
            *name,
            text_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        if race.passive(index) {
            ui.painter().text(
                Pos2::new(beside, row.bottom()),
                Align2::LEFT_BOTTOM,
                WORDS_PASSIVE,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            continue;
        }
        let (Some(act), Some(slot)) = (
            racial_act(frame, race, index),
            racial_slot(frame, race, index),
        ) else {
            continue;
        };
        let response = ui.interact(cell, Id::new(("racial", gump)), Sense::click_and_drag());
        if response.hovered() && !response.dragged() {
            super::super::tips::label(ui, name, HINT_FLIGHT);
        }
        let pressed = button_pair(ui, cell, ("racial", gump), [WORDS_USE, WORDS_PIN]);
        if pressed == Some(0) || response.clicked() {
            tools.hand.act(act);
        }
        if response.drag_started() {
            offer = Some(Offer::Drag(slot));
        } else if pressed == Some(1) {
            offer = Some(Offer::Pin(slot));
        }
    }
    offer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_panels_draw_and_the_racial_one_takes_the_height_of_the_race() {
        use super::super::testing::draw_frames;
        const RACE_ELF: u8 = 2;
        let mut frame = WatchFrame {
            human_control: true,
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        let mut first_row = 0;
        let mut drawn = Vec::new();
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            let (panel, closed, offer) =
                abilities_panel(ui, rect, &frame, tools, profile, &mut first_row);
            assert!(!closed && offer.is_none());
            drawn.push(panel);
            drawn.push(racial_panel(ui, rect, &frame, tools, profile).0);
        });
        frame.status.race = RACE_ELF;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            drawn.push(racial_panel(ui, rect, &frame, tools, profile).0);
        });
        assert_eq!(
            drawn[0].size(),
            bridge::vec2(uoterm_view::ui::abilities::ABILITIES_SIZE)
        );
        assert!(
            drawn[2].height() > drawn[1].height(),
            "six abilities need more room"
        );
    }
}
