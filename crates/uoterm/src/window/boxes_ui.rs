//! The windows a player knows from the game, as glass panels: the open
//! containers with their items, and the gumps with their words, boxes and
//! buttons. Each one shows at all times, so the operator sees what the agent
//! sees. The clicks work only while the human has control.

use super::control::{Act, Hand};
use super::desk::{Desk, Zone};
use super::ring_ui::{RingUi, Subject};
use super::scene::Scene;
use super::theme::{self, number_font, text_font, title_font};
use super::tips::Tips;
use crate::view::{WatchContainer, WatchFrame, WatchGump, WatchPackItem};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use std::collections::{HashMap, HashSet};

const LEFT_COLUMN_TOP: f32 = 150.0;
pub(super) const CELL: f32 = 46.0;
pub(super) const CELL_GAP: f32 = 4.0;
pub(super) const CELL_RADIUS: u8 = 5;
const COLUMNS: usize = 6;
const MAX_ROWS: usize = 3;
const MAX_CONTAINERS: usize = 2;
const TITLE_ROW: f32 = 28.0;
const CLOSE_SIDE: f32 = 20.0;
const CLOSE_STROKE: f32 = 1.5;
const PANEL_GAP: f32 = 12.0;
/// The vitals panel is at the bottom of the left side. The containers stop
/// above it.
const VITALS_ROOM: f32 = 230.0;

const GUMP_WIDTH: f32 = 300.0;
const GUMP_TOP: f32 = 330.0;
const GUMP_MAX_TEXTS: usize = 8;
const GUMP_MAX_ROWS: usize = 10;
const GUMP_ROW: f32 = 26.0;
const BOX_SIDE: f32 = 14.0;
const BOX_RADIUS: u8 = 3;
pub(super) const FIRST_PAGE: u32 = 1;
const EVERY_PAGE: u32 = 0;

const WORDS_CLOSE: &str = "Close";
const HINT_USE: &str = "Double-click: use.  Drag: move.  Right-click: more.";

/// The parts of the window a panel works with.
pub struct Tools<'a> {
    pub scene: &'a mut Scene,
    pub hand: &'a Hand,
    pub desk: &'a mut Desk,
    pub tips: &'a mut Tips,
    pub ring: &'a mut RingUi,
    pub time: f64,
}

/// What the human did to one gump before he answers it.
#[derive(Default)]
struct GumpState {
    page: Option<u32>,
    /// The words the human typed, by the id of the field.
    typed: HashMap<u16, String>,
    /// The boxes whose tick the human changed from what the gump came with.
    flipped: HashSet<u32>,
}

#[derive(Default)]
pub struct BoxesUi {
    gumps: HashMap<u32, GumpState>,
    /// The containers the human closed. The game keeps no "closed" state for
    /// a container, so the window keeps it. A use of the container shows it
    /// again.
    closed: HashSet<u32>,
    /// The first row each container shows. The mouse wheel turns it.
    first_row: HashMap<u32, usize>,
}

fn panel_size(columns: usize, rows: usize) -> Vec2 {
    Vec2::new(
        columns as f32 * (CELL + CELL_GAP) - CELL_GAP,
        rows as f32 * (CELL + CELL_GAP) - CELL_GAP,
    ) + Vec2::splat(theme::PANEL_PAD * 2.0)
        + Vec2::new(0.0, TITLE_ROW)
}

/// The boxes that are ticked now: the ticks the gump came with, with the
/// human's changes on top. A tick on a radio box clears the others of its page.
fn ticked(gump: &WatchGump, flipped: &HashSet<u32>) -> Vec<u32> {
    gump.choices
        .iter()
        .filter(|c| c.on != flipped.contains(&c.switch))
        .map(|c| c.switch)
        .collect()
}

fn flip(gump: &WatchGump, flipped: &mut HashSet<u32>, switch: u32) {
    let Some(clicked) = gump.choices.iter().find(|c| c.switch == switch) else {
        return;
    };
    let now_on = ticked(gump, flipped);
    let toggle = |flipped: &mut HashSet<u32>, switch: u32| {
        if !flipped.remove(&switch) {
            flipped.insert(switch);
        }
    };
    if !clicked.radio {
        toggle(flipped, switch);
        return;
    }
    if now_on.contains(&switch) {
        return;
    }
    let rivals = gump
        .choices
        .iter()
        .filter(|c| c.radio && c.page == clicked.page && now_on.contains(&c.switch));
    for rival in rivals {
        toggle(flipped, rival.switch);
    }
    toggle(flipped, switch);
}

/// The first row after the mouse wheel turned over the panel.
pub(super) fn scrolled(
    ui: &egui::Ui,
    panel: Rect,
    first_row: usize,
    last_first_row: usize,
) -> usize {
    let turned = ui.input(|i| {
        let over = i.pointer.hover_pos().is_some_and(|p| panel.contains(p));
        if over {
            i.raw_scroll_delta.y
        } else {
            0.0
        }
    });
    next_first_row(first_row, turned, last_first_row)
}

fn next_first_row(first_row: usize, turned: f32, last_first_row: usize) -> usize {
    let next = if turned < 0.0 {
        first_row + 1
    } else if turned > 0.0 {
        first_row.saturating_sub(1)
    } else {
        first_row
    };
    next.min(last_first_row)
}

pub(super) fn on_page(item_page: u32, shown: u32) -> bool {
    item_page == EVERY_PAGE || item_page == shown
}

impl BoxesUi {
    /// True when the window shows this container now.
    pub fn shows(&self, frame: &WatchFrame, container: u32) -> bool {
        !self.closed.contains(&container) && frame.containers.iter().any(|c| c.serial == container)
    }

    pub fn close(&mut self, container: u32) {
        self.closed.insert(container);
    }

    /// Call this when the human uses a thing. A closed container shows again.
    pub fn used(&mut self, thing: u32) {
        self.closed.remove(&thing);
    }

    /// Draws the containers down the left side, and the gump on the right
    /// side between the roster and the journal. The middle stays clear.
    /// Gives the places it covered, so the map does not take their clicks.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        gumps_as_lists: bool,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        let mut top = rect.top() + LEFT_COLUMN_TOP;
        let left = rect.left() + theme::SCREEN_MARGIN;
        let floor = rect.bottom() - VITALS_ROOM;
        self.closed
            .retain(|serial| frame.containers.iter().any(|c| c.serial == *serial));
        let open: Vec<&WatchContainer> = frame
            .containers
            .iter()
            .filter(|c| !self.closed.contains(&c.serial))
            .take(MAX_CONTAINERS)
            .collect();
        for container in open {
            if top + panel_size(COLUMNS, MAX_ROWS).y > floor && !covered.is_empty() {
                break;
            }
            let panel = self.container(ui, Pos2::new(left, top), container, frame, tools);
            top = panel.bottom() + PANEL_GAP;
            covered.push(panel);
        }
        self.gumps
            .retain(|id, _| frame.gumps.iter().any(|g| g.gump == *id));
        // With the gump art of the client, the gumps show in their own
        // layout, not as lists.
        if let Some(gump) = frame.gumps.first().filter(|_| gumps_as_lists) {
            let at = Pos2::new(
                rect.right() - theme::SCREEN_MARGIN - GUMP_WIDTH,
                rect.top() + GUMP_TOP,
            );
            covered.push(self.gump(ui, at, gump, frame, tools.hand));
        }
        covered
    }

    fn container(
        &mut self,
        ui: &egui::Ui,
        left_top: Pos2,
        container: &WatchContainer,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let all_rows = container.items.len().div_ceil(COLUMNS).max(1);
        let rows = all_rows.min(MAX_ROWS);
        let panel = Rect::from_min_size(left_top, panel_size(COLUMNS, rows));
        let first_row = {
            let kept = self.first_row.entry(container.serial).or_default();
            *kept = scrolled(ui, panel, *kept, all_rows - rows);
            *kept
        };
        let shown: Vec<&WatchPackItem> = container
            .items
            .iter()
            .skip(first_row * COLUMNS)
            .take(COLUMNS * MAX_ROWS)
            .collect();
        tools.desk.zone(panel, Zone::Into(container.serial));
        let painter = ui.painter();
        theme::panel(painter, panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        painter.text(
            inner.left_top(),
            Align2::LEFT_TOP,
            &container.name,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let close = Rect::from_min_size(
            Pos2::new(
                inner.right() - CLOSE_SIDE,
                inner.top() + theme::ROW_GAP / 2.0,
            ),
            Vec2::splat(CLOSE_SIDE),
        );
        let response = ui.interact(
            close,
            Id::new(("container-close", container.serial)),
            Sense::click(),
        );
        let cross = if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        let arm = close.shrink(CLOSE_SIDE / 4.0);
        let stroke = egui::Stroke::new(CLOSE_STROKE, cross);
        painter.line_segment([arm.left_top(), arm.right_bottom()], stroke);
        painter.line_segment([arm.right_top(), arm.left_bottom()], stroke);
        if response.clicked() {
            self.close(container.serial);
        }
        painter.text(
            Pos2::new(
                close.left() - theme::ROW_GAP * 2.0,
                inner.top() + theme::ROW_GAP,
            ),
            Align2::RIGHT_TOP,
            container.total.to_string(),
            number_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        for (i, item) in shown.into_iter().enumerate() {
            let cell = Rect::from_min_size(
                inner.left_top()
                    + Vec2::new(
                        (i % COLUMNS) as f32 * (CELL + CELL_GAP),
                        TITLE_ROW + (i / COLUMNS) as f32 * (CELL + CELL_GAP),
                    ),
                Vec2::splat(CELL),
            );
            self.cell(ui, cell, item, frame, tools);
        }
        panel
    }

    fn cell(
        &mut self,
        ui: &egui::Ui,
        cell: Rect,
        item: &WatchPackItem,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        let response = ui.interact(
            cell,
            Id::new(("pack-item", item.serial)),
            Sense::click_and_drag(),
        );
        // An item dropped on a bag goes in, and on a pile of its kind joins.
        tools.desk.zone(cell, Zone::Into(item.serial));
        let painter = ui.painter();
        let fill = if response.hovered() {
            theme::BUTTON_HOVER
        } else {
            theme::TRACK
        };
        painter.rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
        if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, item.graphic, item.hue)
        {
            let area = theme::fit(cell, sprite.width, sprite.height);
            painter.image(texture, area, sprite.uv, Color32::WHITE);
        }
        if item.amount > 1 {
            theme::shadowed_text(
                painter,
                cell.right_bottom() - Vec2::splat(theme::CELL_ART_PAD),
                Align2::RIGHT_BOTTOM,
                &item.amount.to_string(),
                number_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
        }
        if response.hovered() && !tools.desk.carries() && !tools.ring.is_open() {
            let footer = if frame.human_control { HINT_USE } else { "" };
            tools
                .tips
                .point_at(ui, tools.hand, item.serial, &item.name, footer, tools.time);
        }
        if !frame.human_control {
            return;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            tools.desk.pick_up(item);
        } else if response.double_clicked() {
            // The item may be a bag that the human closed before.
            self.used(item.serial);
            tools.hand.act(Act::Use(item.serial));
        } else if response.secondary_clicked() {
            tools.ring.open_at(
                cell.center(),
                item.serial,
                &item.name,
                Subject::Packed,
                tools.hand,
            );
        }
    }

    fn gump(
        &mut self,
        ui: &mut egui::Ui,
        left_top: Pos2,
        gump: &WatchGump,
        frame: &WatchFrame,
        hand: &Hand,
    ) -> Rect {
        let state = self.gumps.entry(gump.gump).or_default();
        let page = state.page.unwrap_or(FIRST_PAGE);
        let texts: Vec<&str> = gump
            .texts
            .iter()
            .filter(|(text_page, _)| on_page(*text_page, page))
            .map(|(_, words)| words.as_str())
            .take(GUMP_MAX_TEXTS)
            .collect();
        let choices: Vec<_> = gump
            .choices
            .iter()
            .filter(|c| on_page(c.page, page))
            .take(GUMP_MAX_ROWS)
            .collect();
        let buttons: Vec<_> = gump
            .buttons
            .iter()
            .filter(|b| on_page(b.page, page))
            .take(GUMP_MAX_ROWS)
            .collect();
        let entries: Vec<_> = gump
            .entries
            .iter()
            .filter(|e| on_page(e.page, page))
            .take(GUMP_MAX_ROWS)
            .collect();
        let rows = texts.len() + choices.len() + entries.len() + buttons.len() + 1;
        let panel = Rect::from_min_size(
            left_top,
            Vec2::new(GUMP_WIDTH, rows as f32 * GUMP_ROW + theme::PANEL_PAD * 2.0),
        );
        let painter = ui.painter().clone();
        let painter = &painter;
        theme::panel(painter, panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let mut y = inner.top();
        let row = |y: &mut f32| {
            let area = Rect::from_min_size(
                Pos2::new(inner.left(), *y),
                Vec2::new(inner.width(), GUMP_ROW),
            );
            *y += GUMP_ROW;
            area
        };
        for words in texts {
            let area = row(&mut y);
            painter.text(
                area.left_center(),
                Align2::LEFT_CENTER,
                words,
                text_font(theme::SIZE_BODY),
                theme::TEXT_DIM,
            );
        }
        let live = frame.human_control;
        let now_on = ticked(gump, &state.flipped);
        for choice in choices {
            let area = row(&mut y);
            let response = ui.interact(
                area,
                Id::new(("gump-box", gump.gump, choice.switch)),
                Sense::click(),
            );
            let tick = Rect::from_center_size(
                Pos2::new(area.left() + BOX_SIDE / 2.0, area.center().y),
                Vec2::splat(BOX_SIDE),
            );
            let radius = if choice.radio {
                CornerRadius::same(BOX_SIDE as u8)
            } else {
                CornerRadius::same(BOX_RADIUS)
            };
            painter.rect_filled(tick, radius, theme::TRACK);
            if now_on.contains(&choice.switch) {
                painter.rect_filled(tick.shrink(BOX_RADIUS as f32), radius, theme::GOAL);
            }
            painter.text(
                Pos2::new(tick.right() + theme::ROW_GAP, area.center().y),
                Align2::LEFT_CENTER,
                &choice.label,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if live && response.clicked() {
                flip(gump, &mut state.flipped, choice.switch);
            }
        }
        for entry in entries {
            let area = row(&mut y).shrink2(Vec2::new(0.0, 2.0));
            let label = painter.text(
                area.left_center(),
                Align2::LEFT_CENTER,
                &entry.label,
                text_font(theme::SIZE_BODY),
                theme::TEXT_DIM,
            );
            let field = Rect::from_min_max(
                Pos2::new(label.right() + theme::ROW_GAP, area.top()),
                area.right_bottom(),
            );
            painter.rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
            let words = state
                .typed
                .entry(entry.id)
                .or_insert_with(|| entry.text.clone());
            let limit = entry.limit.map_or(usize::MAX, |limit| limit as usize);
            ui.add_enabled_ui(live, |ui| {
                ui.put(
                    field,
                    egui::TextEdit::singleline(words)
                        .frame(false)
                        .char_limit(limit)
                        .font(text_font(theme::SIZE_BODY))
                        .text_color(theme::TEXT),
                );
            });
        }
        for button in buttons {
            let area = row(&mut y).shrink2(Vec2::new(0.0, 2.0));
            let response = ui.interact(
                area,
                Id::new((
                    "gump-button",
                    gump.gump,
                    button.id,
                    button.to_page,
                    &button.label,
                )),
                Sense::click(),
            );
            let fill = if live && response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::BUTTON
            };
            painter.rect_filled(area, CornerRadius::same(CELL_RADIUS), fill);
            painter.text(
                area.left_center() + Vec2::new(theme::ROW_GAP, 0.0),
                Align2::LEFT_CENTER,
                &button.label,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if !(live && response.clicked()) {
                continue;
            }
            match (button.id, button.to_page) {
                (Some(id), _) => hand.act(Act::GumpButton {
                    gump: gump.gump,
                    button: id,
                    switches: ticked(gump, &state.flipped),
                    texts: state
                        .typed
                        .iter()
                        .map(|(id, words)| (*id, words.clone()))
                        .collect(),
                }),
                (None, Some(to_page)) => state.page = Some(to_page),
                (None, None) => {}
            }
        }
        let close = row(&mut y).shrink2(Vec2::new(0.0, 2.0));
        let response = ui.interact(close, Id::new(("gump-close", gump.gump)), Sense::click());
        painter.text(
            close.left_center(),
            Align2::LEFT_CENTER,
            WORDS_CLOSE,
            text_font(theme::SIZE_BODY),
            if live {
                theme::WAITING
            } else {
                theme::TEXT_FAINT
            },
        );
        if live && response.clicked() {
            hand.act(Act::GumpClose(gump.gump));
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchGumpChoice;

    #[test]
    fn a_closed_container_shows_again_when_it_is_used() {
        const BAG: u32 = 0x4000_0100;
        let frame = WatchFrame {
            containers: vec![WatchContainer {
                serial: BAG,
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        let mut boxes = BoxesUi::default();
        assert!(boxes.shows(&frame, BAG));
        boxes.close(BAG);
        assert!(!boxes.shows(&frame, BAG));
        boxes.used(BAG);
        assert!(boxes.shows(&frame, BAG));
        assert!(!boxes.shows(&WatchFrame::default(), BAG));
    }

    fn choice(switch: u32, radio: bool, on: bool) -> WatchGumpChoice {
        WatchGumpChoice {
            switch,
            radio,
            on,
            page: FIRST_PAGE,
            label: String::new(),
        }
    }

    #[test]
    fn a_tick_on_a_radio_box_clears_its_rival() {
        let gump = WatchGump {
            choices: vec![
                choice(1, true, true),
                choice(2, true, false),
                choice(3, false, false),
            ],
            ..WatchGump::default()
        };
        let mut flipped = HashSet::new();
        assert_eq!(ticked(&gump, &flipped), vec![1]);
        flip(&gump, &mut flipped, 2);
        assert_eq!(ticked(&gump, &flipped), vec![2]);
        flip(&gump, &mut flipped, 2);
        assert_eq!(ticked(&gump, &flipped), vec![2], "a radio box stays on");
        flip(&gump, &mut flipped, 3);
        assert_eq!(ticked(&gump, &flipped), vec![2, 3]);
        flip(&gump, &mut flipped, 3);
        assert_eq!(ticked(&gump, &flipped), vec![2]);
    }

    #[test]
    fn the_wheel_turns_a_container_one_row_and_stops_at_its_ends() {
        assert_eq!(next_first_row(0, -1.0, 2), 1);
        assert_eq!(next_first_row(2, -1.0, 2), 2);
        assert_eq!(next_first_row(0, 1.0, 2), 0);
        assert_eq!(next_first_row(2, 0.0, 1), 1, "the container lost rows");
    }

    #[test]
    fn page_zero_shows_on_each_page() {
        assert!(on_page(EVERY_PAGE, 3));
        assert!(on_page(2, 2));
        assert!(!on_page(2, FIRST_PAGE));
    }
}
