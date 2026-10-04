//! What the human sees and touches to control the character: the bar that
//! takes and gives back control, the clicks on the map, the chat box, and the
//! line that tells what came of the last act.
//!
//! Nothing here acts while the agent has the character. The human must take
//! control first, so a stray click never takes the character from the agent.
//!
//! The clicks follow the General page, as in the official client: a double
//! click on the ground walks there when pathfinding is on (with Shift held,
//! when the page asks for Shift), and a single click only when the page
//! asks for it. The chat line follows the Speech page (see `keys::chat`).

use super::actions::guard::aim_words;
use super::actions::WindowCommand;
use super::boxes_ui::{BoxesUi, Tools};
use super::bridge;
use super::build_ui::{BuildUi, ChatUi};
use super::control::{Act, Hand, Report};
use super::deck_ui::DeckUi;
use super::desk;
use super::hud::Hud;
use super::keys::chat::{self, ChatKey, ChatLine, ChatOut};
use super::macros_ui::MacrosUi;
use super::map_ui::MapUi;
use super::mapitem_ui::ProfileUi;
use super::model::asked::asked_commands;
use super::model::health_bars::MapDrag;
use super::modern::layout::{self, Spot};
use super::modern::{ModernUi, WORDS_LAUNCHER};
use super::options_ui::OptionsUi;
use super::ring_ui::{opens_menu, Subject};
use super::scene::{PickKind, Scene};
use super::settings::{Profile, SpeechOptions};
use super::steer::{self, Movement, Steer};
use super::theme::{self, number_font, text_font};
use super::tips;
use crate::view::{WatchFrame, WatchPackItem};
use eframe::egui::text::LayoutJob;
use eframe::egui::TextFormat;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_view::clicks::{
    act_for_click, bar_buttons, bar_status, beside_bar, escape_on_map, grabbed, hint_for,
    report_shows, ChatMode, EscapeOnMap, GroundClicks, Press, WordsEdge,
};
use uoterm_view::ui::control_bar::{BAR_WIDTH, RULE_GAP, SEGMENT_HEIGHT, STRIP_PAD, STRIP_ROW};

/// The button that opens the panel launcher of the Modern style, at the
/// left of the place row.
const LAUNCHER_BUTTON_WIDTH: f32 = 72.0;
const SEGMENT_GAP: f32 = 6.0;
const LOCATION_GAP: f32 = 18.0;
const WORD_GAP: f32 = 6.0;
const FOLD_SIDE: f32 = 22.0;
const FOLD_ARROW_SHARE: f32 = 0.5;
const FOLD_STROKE: f32 = 2.0;
const BUTTON_GAP: f32 = 8.0;
const FIELD_RADIUS: u8 = 6;
const REPORT_GAP: f32 = 8.0;
const MODE_WIDTH: f32 = 58.0;

const WORDS_PIN: &str = "Pin";
const PIN_WIDTH: f32 = 48.0;
const REPORT_BAR_FULL: &str = "The hotbar is full. Right-click a slot to clear it.";
const WORDS_YES: &str = "Yes";
const WORDS_NO: &str = "No";
const QUESTION_SIZE: Vec2 = Vec2::new(300.0, 110.0);
/// The id of the chat line. The keys know it by this id.
const CHAT_BOX: &str = "chat-box";

/// The chat line's id, so the keys know when it has them.
pub fn chat_id() -> Id {
    Id::new(CHAT_BOX)
}

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
    pub deck: &'a mut DeckUi,
    pub world_map: &'a mut MapUi,
    pub macros: &'a mut MacrosUi,
    pub profiles: &'a mut ProfileUi,
    pub chat: &'a mut ChatUi,
    pub build: &'a BuildUi,
    /// The Modern panels, whose launcher the bar opens.
    pub modern: &'a mut ModernUi,
    pub profile: &'a Profile,
    pub movement: &'a Movement,
    /// The Classic style draws its own questions as gumps.
    pub classic: bool,
    /// Where the world is drawn: the whole window, or the game window of
    /// the Classic style.
    pub view: Rect,
}

/// Where words beside the bar go, `gap` from it, as egui places them.
fn beside(bar: Rect, low: bool, gap: f32) -> (Pos2, Align2) {
    let (at, edge) = beside_bar(bridge::area(bar), low, gap);
    let side = match edge {
        WordsEdge::Top => Align2::CENTER_TOP,
        WordsEdge::Bottom => Align2::CENTER_BOTTOM,
    };
    (bridge::pos2(at), side)
}

/// The click sense of the whole map. Call this before any button is made.
pub fn map_sense(ui: &egui::Ui, rect: Rect) -> egui::Response {
    ui.interact(rect, Id::new("control-map"), Sense::click_and_drag())
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
    /// The chat line of both styles: the Modern chat box draws it here,
    /// the Classic style at the foot of its game window.
    chat_line: ChatLine,
    mode: ChatMode,
    steer: Steer,
    /// The last report, and when it came.
    report: Option<(Report, f64)>,
    /// Window commands of the buttons for the style, for the next frame.
    window_commands: Vec<WindowCommand>,
}

impl ControlUi {
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        hud: &Hud,
        tools: &mut Tools<'_>,
        mut places: Places<'_>,
    ) {
        let (hand, time) = (tools.hand, tools.time);
        if let Some(report) = hand.newest_report() {
            self.report = Some((report, time));
        }
        let mut on_controls = places.covered.to_vec();
        let map = places.map;
        let bar = self.bar(ui, rect, frame, tools.scene, hand, &mut places);
        on_controls.push(bar);
        if let Some(row) = places.chat_row.filter(|_| !self.chat_line.is_hidden()) {
            self.chat(
                ui,
                row,
                frame,
                hand,
                places.deck,
                time,
                &places.profile.speech,
            );
            on_controls.push(row);
        }
        self.show_report(ui, bar, places.classic, time);
        // The Classic style asks the question with its own gump.
        let asked = if places.classic {
            None
        } else {
            question(ui, rect, hand)
        };
        if let Some(asked) = asked {
            on_controls.push(asked);
        }
        if frame.human_control {
            act_on_map(
                ui,
                places.view,
                frame,
                hud,
                tools,
                &mut self.steer,
                map,
                &on_controls,
                places.boxes,
                places.build,
                places.profile,
                places.movement,
                places.classic,
            );
        }
    }

    /// The chat line holds no words.
    pub fn chat_empty(&self) -> bool {
        self.chat_line.text.is_empty()
    }

    /// Hides the chat line, or shows it again.
    pub fn toggle_chat(&mut self) {
        self.chat_line.toggle_hidden();
    }

    /// Pastes the clipboard into the chat line in the next frame.
    pub fn paste(&mut self) {
        self.chat_line.paste();
    }

    /// The chat line, for the Classic style to draw.
    pub fn chat_line(&mut self) -> &mut ChatLine {
        &mut self.chat_line
    }

    /// The window commands the buttons gave since the last frame, for the
    /// style to do.
    pub fn take_window_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut self.window_commands)
    }

    /// Ctrl+Q (older) or Ctrl+W in the chat line.
    pub fn history(&mut self, older: bool) {
        if older {
            self.chat_line.older();
        } else {
            self.chat_line.newer();
        }
    }
}

/// The question of the criminal action, with Yes and No. Gives its place
/// while it waits for the answer.
fn question(ui: &egui::Ui, rect: Rect, hand: &Hand) -> Option<Rect> {
    let words = hand.question()?;
    let panel = Rect::from_center_size(rect.center(), QUESTION_SIZE);
    theme::panel(ui.painter(), panel);
    let inner = panel.shrink(theme::PANEL_PAD);
    ui.painter().text(
        Pos2::new(inner.center().x, inner.top()),
        Align2::CENTER_TOP,
        words,
        text_font(theme::SIZE_BODY),
        theme::ALARM,
    );
    let buttons_top = inner.bottom() - SEGMENT_HEIGHT;
    let half = (inner.width() - BUTTON_GAP) / 2.0;
    for (at, (label, yes)) in [(WORDS_YES, true), (WORDS_NO, false)]
        .into_iter()
        .enumerate()
    {
        let area = Rect::from_min_size(
            Pos2::new(inner.left() + at as f32 * (half + BUTTON_GAP), buttons_top),
            Vec2::new(half, SEGMENT_HEIGHT),
        );
        if theme::segment(ui, area, label, theme::TEXT) {
            hand.answer(yes);
        }
    }
    Some(panel)
}

/// The clicks of the human on the map, and the hint beside the mouse.
#[allow(clippy::too_many_arguments)]
fn act_on_map(
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    hud: &Hud,
    tools: &mut Tools<'_>,
    steer: &mut Steer,
    map: &egui::Response,
    on_controls: &[Rect],
    boxes: &mut BoxesUi,
    builder: &BuildUi,
    profile: &Profile,
    movement: &Movement,
    classic: bool,
) {
    let hand = tools.hand;
    let escape = ui.input(|i| i.key_pressed(Key::Escape));
    match escape_on_map(hand.aiming().is_some(), frame.target_cursor).filter(|_| escape) {
        Some(EscapeOnMap::CancelAim) => hand.cancel_aim(),
        Some(EscapeOnMap::Act(act)) => hand.act(act),
        None => {}
    }
    let mouse = ui.input(|i| i.pointer.hover_pos());
    let on_panel =
        mouse.is_some_and(|at| hud.covers(at) || on_controls.iter().any(|r| r.contains(at)));
    desk::carry_and_land(
        tools.desk,
        ui,
        rect,
        frame,
        tools.scene,
        hand,
        on_panel,
        profile.general.shift_to_split_stacks,
    );
    let mouse_on_map = mouse.filter(|at| rect.contains(*at) && !on_panel && !tools.ring.is_open());
    let character = tools
        .scene
        .place_of(frame, frame.serial)
        .map_or(rect.center(), |place| tools.scene.screen_of(rect, place));
    let steered = steer::run(
        steer,
        ui,
        character,
        mouse_on_map,
        hand,
        tools.time,
        movement,
    );
    let Some(mouse) = mouse_on_map else {
        return;
    };
    if tools.desk.carries() || steered {
        return;
    }
    // While the designer is open, a click on the house builds with the
    // part the human picked.
    if frame.designing.is_some() {
        let (x, y, z) = tools.scene.tile_at(rect, frame, mouse);
        tips::label(ui, builder.hint(), "");
        if map.clicked() {
            if let Some(act) =
                builder.click_on_house(frame, i32::from(x), i32::from(y), i32::from(z))
            {
                hand.act(act);
            }
        }
        return;
    }
    // A building that waits for its place shows where the mouse points.
    if tools.scene.draw_placing(ui.painter(), rect, frame, mouse) {
        if map.clicked() {
            let (x, y, z) = tools.scene.tile_at(rect, frame, mouse);
            hand.act(Act::TargetGround { x, y, z });
        }
        return;
    }
    let thing = tools.scene.thing_at(mouse).cloned();
    if let Some(aim) = hand.aiming() {
        tips::label(ui, aim_words(aim), "");
    } else if let Some(thing) = &thing {
        let footer = hint_for(frame, thing.kind);
        tips::point_at(
            tools.tips,
            ui,
            hand,
            thing.serial,
            &thing.name,
            footer,
            tools.time,
        );
    }
    let shift = ui.input(|i| i.modifiers.shift);
    let shift_needed = classic && profile.general.shift_for_context_menus;
    let asks_menu = opens_menu(map.secondary_clicked(), map.clicked(), shift, shift_needed);
    if let Some(thing) = thing.as_ref().filter(|_| asks_menu) {
        tools.ring.open_at(
            mouse,
            thing.serial,
            &thing.name,
            Subject::OnMap(thing.kind),
            hand,
        );
        // A left click also looks at the thing, as in the classic client.
        if !map.clicked() {
            return;
        }
    }
    let pressed_on = ui
        .input(|i| i.pointer.press_origin())
        .and_then(|at| tools.scene.thing_at(at).cloned());
    let grabbed = grabbed(pressed_on, thing.clone(), profile.general.sallos_easy_grab);
    let dragging = map.drag_started_by(egui::PointerButton::Primary);
    let dragged_item = grabbed
        .as_ref()
        .filter(|t| t.kind == PickKind::Item && dragging)
        .and_then(|t| frame.items.iter().find(|item| item.serial == t.serial));
    if let Some(item) = dragged_item {
        tools.desk.pick_up(&WatchPackItem {
            serial: item.serial,
            graphic: item.graphic,
            hue: item.hue,
            amount: item.amount,
            name: item.name.clone(),
            ..WatchPackItem::default()
        });
        return;
    }
    // A drag from a mobile or from the ground is the gumps' to take: it
    // pulls off a health bar, or selects health bars by a box.
    let drag_from = grabbed.as_ref().map_or(Some(None), |t| {
        (t.kind == PickKind::Mobile).then_some(Some(t.serial))
    });
    if let Some(mobile) = drag_from.filter(|_| dragging) {
        let from = ui.input(|i| i.pointer.press_origin()).unwrap_or(mouse);
        let from = bridge::point(from);
        tools.scene.start_map_drag(MapDrag { from, mobile });
        return;
    }
    if map.clicked() || map.double_clicked() {
        let tile = tools.scene.tile_at(rect, frame, mouse);
        let picked = thing.map(|t| (t.serial, t.kind));
        let ground = GroundClicks::of(&profile.general, bridge::mods(ui.input(|i| i.modifiers)));
        if let Some(act) = act_for_click(frame, picked, tile, map.double_clicked(), ground) {
            if let Act::Use(thing) = act {
                boxes.used(thing);
            }
            hand.act(act);
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
        let buttons = bar_buttons(frame);
        let status = bar_status(frame, hand.aiming());
        let menu_height = if self.folded {
            0.0
        } else {
            RULE_GAP * 2.0 + status.map_or(0.0, |_| STRIP_ROW) + SEGMENT_HEIGHT
        };
        let size = Vec2::new(BAR_WIDTH, STRIP_PAD * 2.0 + STRIP_ROW + menu_height);
        // The Classic style keeps the top for its menu bar and its game
        // window, so there the bar stands at the foot of the window.
        let panel = if places.classic {
            Align2::CENTER_BOTTOM.align_size_within_rect(size, rect.shrink(theme::SCREEN_MARGIN))
        } else {
            layout::first_place(rect, Spot::ControlBar, size)
        };
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
        if !places.classic {
            let launcher =
                Rect::from_min_size(top_row.min, Vec2::new(LAUNCHER_BUTTON_WIDTH, STRIP_ROW));
            let color = if places.modern.launcher_shows() {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            if theme::segment(ui, launcher, WORDS_LAUNCHER, color) {
                places.modern.toggle_launcher();
            }
        }
        if !scene.note().is_empty() {
            let (at, side) = beside(panel, places.classic, theme::ROW_GAP);
            theme::shadowed_text(
                ui.painter(),
                at,
                side,
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
                Press::Sheet => places.deck.toggle(),
                Press::Map => places.world_map.toggle(),
                Press::Macros => places.macros.toggle(),
                Press::Profile if places.profiles.shows(frame.serial) => places.profiles.close(),
                Press::Profile => places.profiles.show(frame.serial, hand),
                Press::Chat => places.chat.toggle(),
                // Each style asks with its own question before the game
                // quits.
                Press::Quit => self.window_commands.push(WindowCommand::QuitGame),
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
    /// say and an order for the character. The Speech page says when the
    /// box takes the keys and what Enter does.
    #[allow(clippy::too_many_arguments)]
    fn chat(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        frame: &WatchFrame,
        hand: &Hand,
        deck: &mut DeckUi,
        time: f64,
        speech: &SpeechOptions,
    ) {
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
        // The entry dialog of the Modern style cancels what the shard asks
        // on Esc. A prompt of the shard takes the next line, whatever the
        // mode is.
        let asked = asked_commands(frame);
        let mode_words = self.mode.words(asked.is_some());
        let (_, switched) = theme::button(ui, row.left_top(), mode_words, theme::GOAL);
        if switched && asked.is_none() {
            self.mode = self.mode.next();
        }
        // A command can go on the hotbar in place of one run.
        let can_pin = self.mode == ChatMode::Command && asked.is_none();
        let pin_room = if can_pin { PIN_WIDTH + BUTTON_GAP } else { 0.0 };
        let field = Rect::from_min_max(
            Pos2::new(row.left() + MODE_WIDTH + BUTTON_GAP * 2.0, row.top()),
            row.right_bottom() - Vec2::new(pin_room, 0.0),
        );
        if can_pin {
            let at = Pos2::new(field.right() + BUTTON_GAP, row.top());
            let (_, pinned) = theme::button(ui, at, WORDS_PIN, theme::TEXT);
            let words = self.chat_line.text.trim();
            if pinned && !words.is_empty() {
                if deck.pin_command(&frame.name, words) {
                    self.chat_line.text.clear();
                } else {
                    self.report = Some((
                        Report {
                            text: REPORT_BAR_FULL.into(),
                            failed: true,
                        },
                        time,
                    ));
                }
            }
        }
        // The box keeps one id of its own, so it does not lose the focus
        // when the buttons beside it come and go.
        let key = chat_id();
        chat::take_keys(&mut self.chat_line, ui.ctx(), key, speech);
        let typing = ui.ctx().memory(|m| m.has_focus(key));
        ui.painter()
            .rect_filled(field, CornerRadius::same(FIELD_RADIUS), theme::TRACK);
        if typing {
            ui.painter().rect_stroke(
                field,
                CornerRadius::same(FIELD_RADIUS),
                egui::Stroke::new(1.0, theme::GOAL),
                egui::StrokeKind::Inside,
            );
        }
        let hint = self.mode.hint(
            frame,
            asked.is_some(),
            self.chat_line.is_open(speech),
            hand.orders_on,
        );
        // Tab is war mode, so it must not move the keys to another field.
        let edit = egui::TextEdit::singleline(&mut self.chat_line.text)
            .id(key)
            .lock_focus(true)
            .frame(false)
            .font(text_font(theme::SIZE_BODY))
            .text_color(theme::TEXT)
            .hint_text(hint)
            .margin(egui::Margin::symmetric(8, 6));
        let response = ui.put(field, edit);
        let sent = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if !sent {
            return;
        }
        let shift = ui.input(|i| i.modifiers.shift);
        let out = self.chat_line.key(ChatKey::Enter { shift }, speech);
        // An open line stays open after a line, so the next one needs no
        // key.
        if self.chat_line.is_open(speech) {
            response.request_focus();
        }
        if let ChatOut::Sent(words) = out {
            chat::tell(self.mode.said(&words, asked, frame, speech), hand);
        }
    }

    /// The words of the last report beside the bar, `low` when the bar
    /// stands at the foot of the window.
    fn show_report(&mut self, ui: &egui::Ui, bar: Rect, low: bool, time: f64) {
        let Some((report, since)) = &self.report else {
            return;
        };
        if !report_shows(*since, time) {
            self.report = None;
            return;
        }
        let color = if report.failed {
            theme::ALARM
        } else {
            theme::TEXT
        };
        let (at, side) = beside(bar, low, REPORT_GAP);
        theme::shadowed_text(
            ui.painter(),
            at,
            side,
            &report.text,
            text_font(theme::SIZE_BODY),
            color,
        );
        ui.ctx().request_repaint();
    }
}
