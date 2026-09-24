//! The two ability panels of the Modern style, with what the classic
//! combat book and racial book do. The combat panel shows the primary and
//! the secondary ability of the weapon in hand, red while armed: a click
//! arms one or lets it go, and each pins or drags onto the hotbar. Under
//! them it lists every weapon ability with the weapons that have it. The
//! racial panel shows the abilities of the character's race; the
//! gargoyle's flight flies or lands, and pins onto the hotbar.

use super::super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::deck_ui::{Offer, Slot, ROW, TAB_GAP};
use super::super::model::abilities::{
    ability_of, armed, icon_of, race_of, racial_command, slot_hue, toggle_command, AbilitySlot,
    Race,
};
use super::super::settings::Profile;
use super::super::theme::{self, text_font, title_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use uoterm_assist::abilities::{ability_name, weapons_with, ABILITIES};

pub const ABILITIES_ID: &str = "modern:abilities";
pub const RACIAL_ID: &str = "modern:racial";
const ABILITIES_SIZE: Vec2 = Vec2::new(300.0, 390.0);
const ABILITIES_MIN: Vec2 = Vec2::new(260.0, 260.0);
const RACIAL_WIDTH: f32 = 250.0;
/// Where the panels first open: the combat panel in the left column a
/// step from the box of a party invite, the racial panel in the right
/// column, beside the sheet in the middle.
const ABILITIES_SPOT: Spot = Spot::LeftColumn(1);
const RACIAL_SPOT: Spot = Spot::RightColumn(0);
const ICON_SIDE: f32 = 44.0;
const BUTTON_WIDTH: f32 = 72.0;
/// The buttons under the name of an ability.
const BUTTON_HEIGHT: f32 = ROW - TAB_GAP;
const WORDS_ABILITIES: &str = "Combat abilities";
const WORDS_RACIAL: &str = "Racial abilities";
const WORDS_ALL: &str = "Every weapon ability";
const WORDS_ARM: &str = "Arm";
const WORDS_LET_GO: &str = "Let go";
const WORDS_ARMED: &str = "Armed";
const WORDS_PIN: &str = "Pin";
const WORDS_USE: &str = "Use";
const WORDS_PASSIVE: &str = "Passive";
const WORDS_NO_RACE: &str = "The shard names no race for the character.";
const WORDS_WEAPONS: &str = "Weapons: ";
const HINT_SLOT: &str = "Click: arm or let go.  Drag: onto the hotbar.";
const HINT_FLIGHT: &str = "Click: fly or land.  Drag: onto the hotbar.";

/// The name of an ability, or its number when the table has none.
fn name_of(ability: u8) -> String {
    ability_name(ability).map_or_else(|| ability.to_string(), str::to_string)
}

/// The words of a slot: which it is, and the ability of the weapon in hand.
pub fn slot_words(frame: &WatchFrame, slot: AbilitySlot) -> String {
    format!("{}: {}", slot.title(), name_of(ability_of(frame, slot)))
}

/// A gump icon in a cell.
fn icon(ui: &egui::Ui, tools: &mut Tools<'_>, cell: Rect, gump: u16, hue: u16) {
    ui.painter()
        .rect_filled(cell, CornerRadius::same(CELL_RADIUS), theme::TRACK);
    if let Some((texture, sprite)) = tools.scene.gump_picture(gump, hue) {
        ui.painter().image(
            texture,
            theme::fit(cell, sprite.width, sprite.height),
            sprite.uv,
            Color32::WHITE,
        );
    }
}

/// The spec of a panel that first opens at `spot` of the plan.
fn spec<'a>(
    rect: Rect,
    (id, title): (&'a str, &'a str),
    spot: Spot,
    size: Vec2,
    min: Option<Vec2>,
) -> PanelSpec<'a> {
    PanelSpec {
        id,
        title,
        default: layout::first_place(rect, spot, size),
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
        rect,
        (ABILITIES_ID, WORDS_ABILITIES),
        ABILITIES_SPOT,
        ABILITIES_SIZE,
        Some(ABILITIES_MIN),
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
    let words = slot_words(frame, slot);
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
    let toggle = || tools.hand.act(Act::Command(toggle_command(frame, slot)));
    let response = ui.interact(
        cell,
        Id::new(("ability-slot", slot.serial())),
        Sense::click_and_drag(),
    );
    if response.hovered() && !response.dragged() {
        super::super::tips::label(ui, &words, HINT_SLOT);
    }
    let arm_words = if is_armed { WORDS_LET_GO } else { WORDS_ARM };
    let pressed = button_pair(
        ui,
        cell,
        ("ability", slot.serial() as u16),
        [arm_words, WORDS_PIN],
    );
    if pressed == Some(0) || response.clicked() {
        toggle();
    }
    if response.drag_started() {
        return Some(Offer::Drag(Slot::Ability { slot }));
    }
    (pressed == Some(1)).then_some(Offer::Pin(Slot::Ability { slot }))
}

/// The names of the weapons that have an ability, from the client files.
fn weapon_names(tools: &Tools<'_>, ability: u8) -> String {
    let mut names: Vec<String> = Vec::new();
    for graphic in weapons_with(ability) {
        let Some(tile) = tools.scene.item_tile(graphic) else {
            continue;
        };
        let name = tile.name.trim().to_string();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    names.join(", ")
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
    let last_first = ABILITIES.len().saturating_sub(rows);
    *first_row = scrolled(ui, list, (*first_row).min(last_first), last_first);
    let in_hand: Vec<(u8, AbilitySlot)> = AbilitySlot::BOTH
        .into_iter()
        .map(|slot| (ability_of(frame, slot), slot))
        .collect();
    for (at, (ability, name)) in ABILITIES.iter().skip(*first_row).take(rows).enumerate() {
        let row = Rect::from_min_size(
            list.left_top() + Vec2::new(0.0, at as f32 * ROW),
            Vec2::new(list.width(), ROW - TAB_GAP / 2.0),
        );
        let cell = Rect::from_min_size(row.min, Vec2::splat(row.height()));
        icon(ui, tools, cell, icon_of(*ability), 0);
        let slot = in_hand.iter().find(|(number, _)| number == ability);
        let color = if slot.is_some() {
            theme::GOAL
        } else {
            theme::TEXT
        };
        ui.painter().text(
            Pos2::new(cell.right() + theme::ROW_GAP, row.center().y),
            Align2::LEFT_CENTER,
            *name,
            text_font(theme::SIZE_SMALL),
            color,
        );
        if let Some((_, slot)) = slot {
            ui.painter().text(
                row.right_center(),
                Align2::RIGHT_CENTER,
                slot.title(),
                text_font(theme::SIZE_SMALL),
                theme::GOAL,
            );
        }
        let response = ui.interact(row, Id::new(("every-ability", *ability)), Sense::hover());
        if response.hovered() {
            let weapons = format!("{WORDS_WEAPONS}{}", weapon_names(tools, *ability));
            super::super::tips::label(ui, name, &weapons);
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
    let rows = race.map_or(1, |race| race.names.len()) as f32;
    let height = frame::TITLE_ROW + rows * (ICON_SIDE + theme::ROW_GAP) + theme::PANEL_PAD * 2.0;
    let spec = spec(
        rect,
        (RACIAL_ID, WORDS_RACIAL),
        RACIAL_SPOT,
        Vec2::new(RACIAL_WIDTH, height),
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
        let Some(command) = racial_command(frame, gump).filter(|_| frame.human_control) else {
            continue;
        };
        let slot = || Slot::Racial {
            icon: gump,
            name: (*name).to_string(),
        };
        let response = ui.interact(cell, Id::new(("racial", gump)), Sense::click_and_drag());
        if response.hovered() && !response.dragged() {
            super::super::tips::label(ui, name, HINT_FLIGHT);
        }
        let pressed = button_pair(ui, cell, ("racial", gump), [WORDS_USE, WORDS_PIN]);
        if pressed == Some(0) || response.clicked() {
            tools.hand.act(Act::Command(command.into()));
        }
        if response.drag_started() {
            offer = Some(Offer::Drag(slot()));
        } else if pressed == Some(1) {
            offer = Some(Offer::Pin(slot()));
        }
    }
    offer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_names_the_ability_of_the_weapon_in_hand() {
        let frame = WatchFrame::default();
        assert_eq!(
            slot_words(&frame, AbilitySlot::Primary),
            "Primary: Paralyzing Blow"
        );
        assert_eq!(
            slot_words(&frame, AbilitySlot::Secondary),
            "Secondary: Disarm"
        );
        assert_eq!(name_of(200), "200");
    }

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
        assert_eq!(drawn[0].size(), ABILITIES_SIZE);
        assert!(
            drawn[2].height() > drawn[1].height(),
            "six abilities need more room"
        );
    }
}
