//! What the human sees and touches to control the character: the bar that
//! takes and gives back control, the clicks on the map, the chat box, and the
//! line that tells what came of the last act.
//!
//! Nothing here acts while the agent has the character. The human must take
//! control first, so a stray click never takes the character from the agent.

use super::boxes_ui::BoxesUi;
use super::control::{Act, Hand, Report};
use super::hud::Hud;
use super::options_ui::OptionsUi;
use super::scene::{PickKind, Scene};
use super::theme::{self, number_font, text_font};
use crate::view::WatchFrame;
use eframe::egui::text::LayoutJob;
use eframe::egui::TextFormat;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};

/// The top panel has one width in each state, so nothing in it moves when
/// the buttons change.
const STRIP_WIDTH: f32 = 470.0;
const STRIP_PAD: f32 = 10.0;
const STRIP_ROW: f32 = 24.0;
const RULE_GAP: f32 = 8.0;
const SEGMENT_HEIGHT: f32 = 30.0;
const SEGMENT_GAP: f32 = 6.0;
const LOCATION_GAP: f32 = 18.0;
const WORD_GAP: f32 = 6.0;
const FOLD_SIDE: f32 = 22.0;
const FOLD_ARROW_SHARE: f32 = 0.5;
const FOLD_STROKE: f32 = 2.0;
const BUTTON_GAP: f32 = 8.0;
const FIELD_RADIUS: u8 = 6;
const REPORT_SECONDS: f64 = 5.0;
const REPORT_GAP: f32 = 8.0;
const MODE_WIDTH: f32 = 58.0;

const WORDS_TAKE: &str = "Take control";
const WORDS_IN_CONTROL: &str = "You have control. The agent waits.";
const WORDS_GIVE_BACK: &str = "Give back";
const WORDS_STOP: &str = "Stop";
const WORDS_WAR: &str = "War";
const WORDS_PEACE: &str = "Peace";
const WORDS_BAG: &str = "Bag";
const WORDS_OPTIONS: &str = "Options";
const WORDS_TARGET: &str = "Click the target. Press Esc to cancel.";
const WORDS_SAY: &str = "Say";
const WORDS_ORDER: &str = "Order";
const HINT_SAY: &str = "Words to say. Press Enter.";
const HINT_ORDER: &str = "An order, for example: attack the orc. Press Enter.";
const HINT_ORDER_OFF: &str = "Orders are off. Set TYPESAFE_API_KEY.";

/// Where the other parts of the window are, so this part knows what a
/// click is on.
pub struct Places<'a> {
    /// The click sense of the whole map. The window makes it before each
    /// button, so a button that lies on the map wins the click.
    pub map: &'a egui::Response,
    pub chat_row: Option<Rect>,
    /// The places of the container and gump windows.
    pub covered: &'a [Rect],
    pub boxes: &'a mut BoxesUi,
    pub options: &'a mut OptionsUi,
}

/// The click sense of the whole map. Call this before any button is made.
pub fn map_sense(ui: &egui::Ui, rect: Rect) -> egui::Response {
    ui.interact(rect, Id::new("control-map"), Sense::click())
}

/// Where the character is: the numbers in the number face, the words in
/// the text face.
fn location_job(frame: &WatchFrame) -> LayoutJob {
    let mut job = LayoutJob::default();
    let words = TextFormat::simple(text_font(theme::SIZE_BODY), theme::TEXT_DIM);
    let numbers = TextFormat::simple(number_font(theme::SIZE_BODY), theme::TEXT);
    job.append(
        &format!("{}, {}, {}", frame.x, frame.y, frame.z),
        0.0,
        numbers.clone(),
    );
    job.append("map", LOCATION_GAP, words.clone());
    job.append(&frame.map.to_string(), WORD_GAP, numbers);
    if !frame.facing.is_empty() {
        job.append("faces", LOCATION_GAP, words);
        job.append(
            &frame.facing,
            WORD_GAP,
            TextFormat::simple(text_font(theme::SIZE_BODY), theme::TEXT),
        );
    }
    job
}

/// What a button of the bar does.
enum Press {
    Act(Act),
    /// Opens the backpack with this serial, or closes its panel.
    Bag(u32),
    Options,
}

/// The arrow that folds the bar away and brings it back: it points up when
/// the bar is open, and down when it is folded. True when it was clicked.
fn fold_arrow(ui: &egui::Ui, area: Rect, folded: bool) -> bool {
    let response = ui.interact(area, Id::new("control-fold"), Sense::click());
    let color = if response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let half = FOLD_SIDE * FOLD_ARROW_SHARE / 2.0;
    let tip = if folded { half / 2.0 } else { -half / 2.0 };
    let center = area.center();
    let points = [
        center + Vec2::new(-half, -tip),
        center + Vec2::new(0.0, tip),
        center + Vec2::new(half, -tip),
    ];
    let stroke = egui::Stroke::new(FOLD_STROKE, color);
    ui.painter().line_segment([points[0], points[1]], stroke);
    ui.painter().line_segment([points[1], points[2]], stroke);
    response.clicked()
}

#[derive(Default)]
pub struct ControlUi {
    /// The operator folded the bar away.
    folded: bool,
    text: String,
    order_mode: bool,
    /// The last report, and when it came.
    report: Option<(Report, f64)>,
}

/// The act for a click on the map.
fn act_for_click(
    frame: &WatchFrame,
    thing: Option<(u32, PickKind)>,
    tile: (u16, u16, i8),
    double: bool,
) -> Option<Act> {
    let (x, y, z) = tile;
    Some(match (thing, frame.target_cursor, double) {
        (Some((serial, _)), true, _) => Act::Target(serial),
        (None, true, _) => Act::TargetGround { x, y, z },
        (Some((serial, PickKind::Mobile)), false, true) if frame.war => Act::Attack(serial),
        (Some((serial, _)), false, true) => Act::Use(serial),
        (Some((serial, _)), false, false) => Act::Look(serial),
        (None, false, false) => Act::WalkTo { x, y },
        (None, false, true) => return None,
    })
}

fn hint_for(frame: &WatchFrame, name: &str, kind: PickKind) -> String {
    let act = match (frame.target_cursor, kind) {
        (true, _) => "Click: target",
        (false, PickKind::Mobile) if frame.war => "Double-click: attack",
        (false, PickKind::Corpse) => "Double-click: open",
        (false, _) => "Double-click: use",
    };
    if name.is_empty() {
        act.to_string()
    } else {
        format!("{name}.  {act}")
    }
}

impl ControlUi {
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        hud: &Hud,
        hand: &Hand,
        mut places: Places<'_>,
        time: f64,
    ) {
        if let Some(report) = hand.newest_report() {
            self.report = Some((report, time));
        }
        let mut on_controls = places.covered.to_vec();
        let map = places.map;
        let bar = self.bar(ui, rect, frame, scene, hand, &mut places);
        on_controls.push(bar);
        if let Some(row) = places.chat_row {
            self.chat(ui, row, frame, hand);
            on_controls.push(row);
        }
        self.show_report(ui, bar, time);
        if frame.human_control {
            act_on_map(
                ui,
                rect,
                frame,
                scene,
                hud,
                hand,
                map,
                &on_controls,
                places.boxes,
            );
        }
    }
}

/// The clicks of the human on the map, and the hint beside the mouse.
#[allow(clippy::too_many_arguments)]
fn act_on_map(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    scene: &mut Scene,
    hud: &Hud,
    hand: &Hand,
    map: &egui::Response,
    on_controls: &[Rect],
    boxes: &mut BoxesUi,
) {
    {
        if frame.target_cursor && ui.input(|i| i.key_pressed(Key::Escape)) {
            hand.act(Act::CancelTarget);
        }
        let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        if hud.covers(mouse) || on_controls.iter().any(|r| r.contains(mouse)) {
            return;
        }
        let thing = scene.thing_at(mouse).cloned();
        if let Some(thing) = &thing {
            theme::hint(
                ui.painter(),
                mouse,
                &hint_for(frame, &thing.name, thing.kind),
            );
        }
        if map.clicked() || map.double_clicked() {
            let tile = scene.tile_at(rect, frame, mouse);
            let picked = thing.map(|t| (t.serial, t.kind));
            if let Some(act) = act_for_click(frame, picked, tile, map.double_clicked()) {
                if let Act::Use(thing) = act {
                    boxes.used(thing);
                }
                hand.act(act);
            }
        }
    }
}

impl ControlUi {
    /// The panel at the top middle of the window. Its first row is the
    /// location, with the arrow that folds the menu in its corner. Under a
    /// thin rule is the menu: who has control, then one row of equal buttons
    /// from edge to edge. Each part has the same left and right edge, so the
    /// panel reads as one piece. Folded, only the location row stays.
    fn bar(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        scene: &Scene,
        hand: &Hand,
        places: &mut Places<'_>,
    ) -> Rect {
        let war_words = if frame.war { WORDS_PEACE } else { WORDS_WAR };
        let buttons: Vec<(&str, Press)> = if frame.human_control {
            // The bag button shows only when the shard told which item the
            // backpack is.
            frame
                .backpack()
                .map(|bag| (WORDS_BAG, Press::Bag(bag)))
                .into_iter()
                .chain([
                    (war_words, Press::Act(Act::War(!frame.war))),
                    (WORDS_STOP, Press::Act(Act::Stop)),
                    (WORDS_GIVE_BACK, Press::Act(Act::GiveBack)),
                    (WORDS_OPTIONS, Press::Options),
                ])
                .collect()
        } else {
            vec![
                (WORDS_TAKE, Press::Act(Act::Take)),
                (WORDS_OPTIONS, Press::Options),
            ]
        };
        let status = match (frame.human_control, frame.target_cursor) {
            (false, _) => None,
            (true, false) => Some(WORDS_IN_CONTROL),
            (true, true) => Some(WORDS_TARGET),
        };
        let menu_height = if self.folded {
            0.0
        } else {
            RULE_GAP * 2.0 + status.map_or(0.0, |_| STRIP_ROW) + SEGMENT_HEIGHT
        };
        let panel = Rect::from_min_size(
            Pos2::new(
                rect.center().x - STRIP_WIDTH / 2.0,
                rect.top() + theme::SCREEN_MARGIN,
            ),
            Vec2::new(STRIP_WIDTH, STRIP_PAD * 2.0 + STRIP_ROW + menu_height),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(STRIP_PAD);
        let top_row = Rect::from_min_size(inner.min, Vec2::new(inner.width(), STRIP_ROW));
        let location = ui.painter().layout_job(location_job(frame));
        ui.painter().galley(
            top_row.center() - location.size() / 2.0,
            location,
            theme::TEXT,
        );
        let fold = Rect::from_center_size(
            Pos2::new(top_row.right() - FOLD_SIDE / 2.0, top_row.center().y),
            Vec2::splat(FOLD_SIDE),
        );
        if fold_arrow(ui, fold, self.folded) {
            self.folded = !self.folded;
        }
        if !scene.note().is_empty() {
            theme::shadowed_text(
                ui.painter(),
                Pos2::new(panel.center().x, panel.bottom() + theme::ROW_GAP),
                Align2::CENTER_TOP,
                scene.note(),
                text_font(theme::SIZE_SMALL),
                theme::WAITING,
            );
        }
        if self.folded {
            return panel;
        }
        let rule_y = top_row.bottom() + RULE_GAP;
        ui.painter().line_segment(
            [
                Pos2::new(inner.left(), rule_y),
                Pos2::new(inner.right(), rule_y),
            ],
            egui::Stroke::new(1.0, theme::GLASS_EDGE),
        );
        let mut y = rule_y + RULE_GAP;
        if let Some(words) = status {
            ui.painter().text(
                Pos2::new(inner.center().x, y + STRIP_ROW / 2.0),
                Align2::CENTER_CENTER,
                words,
                text_font(theme::SIZE_BODY),
                theme::WAITING,
            );
            y += STRIP_ROW;
        }
        let count = buttons.len() as f32;
        let segment_width = (inner.width() - SEGMENT_GAP * (count - 1.0)) / count;
        for (i, (label, press)) in buttons.into_iter().enumerate() {
            let area = Rect::from_min_size(
                Pos2::new(inner.left() + i as f32 * (segment_width + SEGMENT_GAP), y),
                Vec2::new(segment_width, SEGMENT_HEIGHT),
            );
            if !theme::segment(ui, area, label, theme::TEXT) {
                continue;
            }
            match press {
                Press::Act(act) => hand.act(act),
                Press::Options => places.options.toggle(),
                Press::Bag(bag) if places.boxes.shows(frame, bag) => places.boxes.close(bag),
                Press::Bag(bag) => {
                    places.boxes.used(bag);
                    hand.act(Act::Use(bag));
                }
            }
        }
        panel
    }

    /// The chat box under the journal. One button picks between words to
    /// say and an order for the character.
    fn chat(&mut self, ui: &mut egui::Ui, row: Rect, frame: &WatchFrame, hand: &Hand) {
        if !frame.human_control {
            ui.painter().text(
                row.left_center(),
                Align2::LEFT_CENTER,
                "Take control to talk.",
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
            return;
        }
        let mode_words = if self.order_mode {
            WORDS_ORDER
        } else {
            WORDS_SAY
        };
        let (_, switched) = theme::button(ui, row.left_top(), mode_words, theme::GOAL);
        if switched {
            self.order_mode = !self.order_mode;
        }
        let field = Rect::from_min_max(
            Pos2::new(row.left() + MODE_WIDTH + BUTTON_GAP * 2.0, row.top()),
            row.right_bottom(),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(FIELD_RADIUS), theme::TRACK);
        let hint = match (self.order_mode, hand.orders_on) {
            (false, _) => HINT_SAY,
            (true, true) => HINT_ORDER,
            (true, false) => HINT_ORDER_OFF,
        };
        let edit = egui::TextEdit::singleline(&mut self.text)
            .frame(false)
            .font(text_font(theme::SIZE_BODY))
            .text_color(theme::TEXT)
            .hint_text(hint)
            .margin(egui::Margin::symmetric(8, 6));
        let response = ui.put(field, edit);
        let sent = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let words = self.text.trim().to_string();
        if !sent || words.is_empty() {
            return;
        }
        hand.act(if self.order_mode {
            Act::Order(words, Box::new(frame.clone()))
        } else {
            Act::Say(words)
        });
        self.text.clear();
        response.request_focus();
    }

    fn show_report(&mut self, ui: &egui::Ui, bar: Rect, time: f64) {
        let Some((report, since)) = &self.report else {
            return;
        };
        if time - since > REPORT_SECONDS {
            self.report = None;
            return;
        }
        let color = if report.failed {
            theme::ALARM
        } else {
            theme::TEXT
        };
        theme::shadowed_text(
            ui.painter(),
            Pos2::new(bar.center().x, bar.bottom() + REPORT_GAP),
            Align2::CENTER_TOP,
            &report.text,
            text_font(theme::SIZE_BODY),
            color,
        );
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORC: u32 = 9;
    const TILE: (u16, u16, i8) = (10, 20, 5);

    fn frame(war: bool, target_cursor: bool) -> WatchFrame {
        WatchFrame {
            war,
            target_cursor,
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_click_on_the_ground_walks_and_a_target_cursor_targets() {
        let peace = frame(false, false);
        assert_eq!(
            act_for_click(&peace, None, TILE, false),
            Some(Act::WalkTo { x: 10, y: 20 })
        );
        assert_eq!(act_for_click(&peace, None, TILE, true), None);
        let aiming = frame(false, true);
        assert_eq!(
            act_for_click(&aiming, None, TILE, false),
            Some(Act::TargetGround { x: 10, y: 20, z: 5 })
        );
        let orc = Some((ORC, PickKind::Mobile));
        assert_eq!(
            act_for_click(&aiming, orc, TILE, false),
            Some(Act::Target(ORC))
        );
    }

    #[test]
    fn a_double_click_attacks_in_war_and_uses_in_peace() {
        let orc = Some((ORC, PickKind::Mobile));
        assert_eq!(
            act_for_click(&frame(true, false), orc, TILE, true),
            Some(Act::Attack(ORC))
        );
        assert_eq!(
            act_for_click(&frame(false, false), orc, TILE, true),
            Some(Act::Use(ORC))
        );
        assert_eq!(
            act_for_click(&frame(true, false), orc, TILE, false),
            Some(Act::Look(ORC))
        );
        let chest = Some((ORC, PickKind::Item));
        assert_eq!(
            act_for_click(&frame(true, false), chest, TILE, true),
            Some(Act::Use(ORC))
        );
    }
}
