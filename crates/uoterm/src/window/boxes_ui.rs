//! The windows a player knows from the game, as glass panels: the open
//! containers as grids of their items, and the gumps with their words,
//! boxes and buttons. Each one shows at all times, so the operator sees
//! what the agent sees. The clicks work only while the human has control.

use super::control::{Act, Hand};
use super::desk::Desk;
use super::model::clicks::ClickDelay;
use super::model::compare::ItemLayers;
use super::model::places;
use super::model::reads::Readings;
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::layout::{self, Spot};
use super::modern::GridUi;
use super::ring_ui::RingUi;
use super::scene::Scene;
use super::settings::{Profile, ProfileHome};
use super::theme::{self, text_font};
use super::tips::Tips;
use crate::view::{WatchFrame, WatchGump};
use eframe::egui::{self, Align2, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use std::collections::{HashMap, HashSet};

pub(super) const CELL: f32 = 46.0;
pub(super) const CELL_GAP: f32 = 4.0;
pub(super) const CELL_RADIUS: u8 = 5;

const GUMP_PLACE_ID: &str = "modern:gump:";
const GUMP_WIDTH: f32 = 300.0;
const GUMP_MAX_TEXTS: usize = 8;
const GUMP_MAX_ROWS: usize = 10;
const GUMP_ROW: f32 = 26.0;
const BOX_SIDE: f32 = 14.0;
const BOX_RADIUS: u8 = 3;
pub(super) const FIRST_PAGE: u32 = 1;
const EVERY_PAGE: u32 = 0;

const WORDS_GUMP: &str = "Shard window";

/// The parts of the window a panel works with.
pub struct Tools<'a> {
    pub scene: &'a mut Scene,
    pub hand: &'a Hand,
    pub desk: &'a mut Desk,
    pub tips: &'a mut Tips,
    pub ring: &'a mut RingUi,
    pub time: f64,
    /// Where the profile is kept.
    pub profile_home: &'a ProfileHome,
    /// The session read for the panels: agents, the meter, properties.
    pub readings: &'a mut Readings,
    /// The layer each wearable graphic is worn on.
    pub layers: &'a ItemLayers,
}

impl Tools<'_> {
    /// Keeps the profile in its file, after a panel changed it.
    pub fn keep_profile(&self, profile: &Profile) {
        self.profile_home.save(&places::for_saving(profile));
    }
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
    grids: GridUi,
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

/// A click on an item of a panel: under a target cursor a click targets
/// the item, and else a single click asks its name once the double click
/// time is over. True on a double click, whose act is the panel's.
pub(super) fn single_or_double(
    response: &egui::Response,
    clicks: &mut ClickDelay,
    frame: &WatchFrame,
    hand: &Hand,
    serial: u32,
    time: f64,
) -> bool {
    if response.double_clicked() {
        clicks.double_clicked();
        return true;
    }
    if response.clicked() {
        if let Some(act) = clicks.single_click(frame, serial, time) {
            hand.act(act);
        }
    }
    false
}

/// Asks the name of the item whose single click waited long enough, and
/// keeps the window drawing while one waits.
pub(super) fn ask_waiting_name(ui: &egui::Ui, clicks: &mut ClickDelay, hand: &Hand, time: f64) {
    if let Some(act) = clicks.due_look(time) {
        hand.act(act);
    }
    if clicks.is_waiting() {
        ui.ctx().request_repaint();
    }
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

    /// Draws the containers as grids and, without the gump art of the
    /// client, each open gump of the shard as a list, first at the right
    /// side between the roster and the journal. The player moves and locks
    /// each one. Gives the places it covered, so the map does not take
    /// their clicks.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        gumps_as_lists: bool,
    ) -> Vec<Rect> {
        self.closed
            .retain(|serial| frame.containers.iter().any(|c| c.serial == *serial));
        let mut covered = self
            .grids
            .draw(ui, rect, frame, tools, profile, &mut self.closed);
        self.gumps
            .retain(|id, _| frame.gumps.iter().any(|g| g.gump == *id));
        // With the gump art of the client, the gumps show in their own
        // layout, not as lists.
        if gumps_as_lists {
            for (index, gump) in frame.gumps.iter().enumerate() {
                covered.push(self.gump(ui, rect, index, gump, frame, tools, profile));
            }
        }
        covered
    }

    /// One gump of the shard as a list: its words, its boxes, its fields
    /// and its buttons, on the page that shows.
    #[allow(clippy::too_many_arguments)]
    fn gump(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        index: usize,
        gump: &WatchGump,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let hand = tools.hand;
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
        let rows = texts.len() + choices.len() + entries.len() + buttons.len();
        let live = frame.human_control;
        let id = format!("{GUMP_PLACE_ID}{:08X}", gump.gump);
        let spec = PanelSpec {
            id: &id,
            title: WORDS_GUMP,
            default: layout::first_place(
                rect,
                Spot::RightColumn(index),
                Vec2::new(
                    GUMP_WIDTH,
                    frame::TITLE_ROW + rows as f32 * GUMP_ROW + theme::PANEL_PAD * 2.0,
                ),
            ),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let painter = ui.painter().clone();
        let painter = &painter;
        let inner = frame::draw(painter, panel, WORDS_GUMP);
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
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) && live {
            hand.act(Act::GumpClose(gump.gump));
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchContainer, WatchGumpChoice};

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
    fn every_open_gump_shows_as_a_list_without_the_gump_art() {
        use crate::window::modern::testing::draw_frames;
        let gump = |gump| WatchGump {
            gump,
            ..WatchGump::default()
        };
        let frame = WatchFrame {
            human_control: true,
            gumps: vec![gump(1), gump(2), gump(3)],
            ..WatchFrame::default()
        };
        let mut boxes = BoxesUi::default();
        let mut profile = Profile::default();
        for (as_lists, shown) in [(true, 3), (false, 0)] {
            let mut covered = Vec::new();
            draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
                covered = boxes.draw(ui, rect, &frame, tools, profile, as_lists);
            });
            assert_eq!(covered.len(), shown);
        }
    }

    #[test]
    fn page_zero_shows_on_each_page() {
        assert!(on_page(EVERY_PAGE, 3));
        assert!(on_page(2, 2));
        assert!(!on_page(2, FIRST_PAGE));
    }
}
