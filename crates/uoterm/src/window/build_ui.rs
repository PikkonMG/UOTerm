//! The house designer and the chat of the shard.
//!
//! The designer shows the parts of the client catalog. The human picks a
//! part and clicks the map to put it on the house, or picks one off the
//! house with the eyedropper. With a TypeSafe key, a field takes the part
//! in plain words, such as "a stone wall", and Jev picks it from the
//! catalog. A button for each storey turns how it shows while he designs,
//! and the foot counts the components, the fixtures and the cost, as the
//! classic designer does.
//!
//! The chat shows every channel of the shard and the lines that were said.
//! A channel with a password asks for it, Create makes a channel, and the
//! box for the chat name shows when the shard asks for it. Its own field
//! takes a channel in plain words the same way. The rules of the chat are
//! `model::chat`, as the classic chat gumps follow.
//!
//! The player moves and locks both panels.

use super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::control::{Act, Answer, Ask, Asker};
use super::model::chat::{
    chat_door, chat_name_act, joining, lines_back, Asking, ChatDoor, ChatWatch, Joining,
    CHAT_NAME_MAX_CHARS, WORDS_CHOOSE_NAME,
};
use super::model::house_design::{
    design_counts, limits_words, part_words, plot_limits, styles_of, HouseDesign, PlotLimits,
    SharedDesign, StoreyLook, ACTION_EXIT, DESIGNER_FLOORS, DESIGN_COMMANDS, KINDS,
};
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::layout::{self, Spot};
use super::settings::Profile;
use super::theme::{self, text_font};
use crate::view::{WatchChat, WatchFrame};
use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Id, Key, Pos2, Rect, Sense, TextFormat, Vec2,
};
use uoterm_nav::HousePart;

const BUILD_ID: &str = "modern:build";
const PANEL_WIDTH: f32 = 430.0;
/// The designer's foot has five rows: the changes, the backups and the
/// eyedropper, the levels, how each storey shows, and the counts.
const BUTTON_ROWS: usize = 5;
/// The first steps of `DESIGN_COMMANDS` keep the design and bring it back;
/// the rest change it.
const KEEPING_COMMANDS: usize = 3;
const TITLE_ROW: f32 = 32.0;
const FIELD_ROW: f32 = 30.0;
const FOOT_ROW: f32 = 40.0;
const GAP: f32 = 8.0;
const LIST_ROWS: usize = 8;
const LIST_ROW: f32 = 30.0;
const PIECE_SIDE: f32 = 40.0;
const ASK_WIDTH: f32 = 60.0;
const NOTE_SECONDS: f64 = 6.0;

const WORDS_TITLE: &str = "Build";
const WORDS_REMOVE: &str = "Remove";
const WORDS_PICK: &str = "Pick";
const WORDS_STOREY: &str = "Storey";
const WORDS_COMPONENTS: &str = "Components";
const WORDS_FIXTURES: &str = "Fixtures";
const WORDS_COST: &str = "Cost";
const HINT_PICK: &str = "Click a part of the house to build with it.";
const HINT_STOREY: &str = "Click: how this storey shows while you design.";
const WORDS_FLOOR: &str = "Floor";
const WORDS_EXIT: &str = "Leave";
const WORDS_FIND: &str = "Find";
const WORDS_ASKING: &str = "Jev looks at the catalog...";
const WORDS_NO_PARTS: &str = "The catalog needs the client files.";
const HINT_WISH: &str = "Say the part in plain words, for example: a stone wall";
const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
pub struct BuildUi {
    /// The part the human builds with, which the classic gump shares.
    design: SharedDesign,
    first_style: usize,
    wish: String,
    note: Option<(String, bool, f64)>,
}

impl BuildUi {
    /// The designer as the window starts. It shows itself when the shard
    /// opens the designer, so an operator needs no button for it.
    pub fn starting() -> Self {
        Self {
            design: SharedDesign::default(),
            first_style: 0,
            wish: String::new(),
            note: None,
        }
    }
}

impl Default for BuildUi {
    fn default() -> Self {
        Self::starting()
    }
}

impl BuildUi {
    fn say(&mut self, words: &str, failed: bool, time: f64) {
        self.note = Some((words.to_string(), failed, time));
    }

    /// The design the classic gump shares with this panel.
    pub fn design(&self) -> SharedDesign {
        SharedDesign::clone(&self.design)
    }

    /// Picks the part at this place of the catalog.
    fn pick_part(&mut self, frame: &WatchFrame, place: usize) {
        if place >= frame.house_parts.len() {
            return;
        }
        let mut design = self.design.borrow_mut();
        design.pick_part(&frame.house_parts, place);
        self.first_style = design.style.saturating_sub(LIST_ROWS / 2);
        self.wish.clear();
        self.note = None;
    }

    /// Takes the answer of Jev about a part of the catalog.
    fn take_answers(&mut self, frame: &WatchFrame, tools: &Tools<'_>, time: f64) {
        for answer in tools.hand.new_answers(Asker::Designer) {
            match answer {
                Answer::Picked(Ok(place)) => self.pick_part(frame, place),
                Answer::Picked(Err(words)) => self.say(&words, true, time),
                // The designer asks only for a pick.
                _ => {}
            }
        }
    }

    /// Draws the designer while the shard has it open. Gives its place.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let time = tools.time;
        self.take_answers(frame, tools, time);
        let designing = frame.designing?;
        let kind = self.design.borrow().kind;
        let styles = styles_of(&frame.house_parts, kind);
        {
            let mut design = self.design.borrow_mut();
            design.style = design.style.min(styles.len().saturating_sub(1));
        }
        let live = frame.human_control;
        let spec = PanelSpec {
            id: BUILD_ID,
            title: WORDS_TITLE,
            default: layout::first_place(
                rect,
                Spot::RightColumn(0),
                Vec2::new(
                    PANEL_WIDTH,
                    frame::TITLE_ROW
                        + TITLE_ROW
                        + LIST_ROWS as f32 * LIST_ROW
                        + PIECE_SIDE
                        + FIELD_ROW
                        + FOOT_ROW * BUTTON_ROWS as f32
                        + GAP * 4.0
                        + theme::PANEL_PAD * 2.0,
                ),
            ),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        ui.painter()
            .rect_filled(panel, CornerRadius::same(theme::PANEL_RADIUS), theme::VOID);
        let inner = frame::draw(ui.painter(), panel, WORDS_TITLE);
        let mut y = inner.top();
        self.kind_row(
            ui,
            Rect::from_min_size(
                Pos2::new(inner.left(), y),
                Vec2::new(inner.width(), TITLE_ROW),
            ),
        );
        y += TITLE_ROW + GAP;
        let list = Rect::from_min_size(
            Pos2::new(inner.left(), y),
            Vec2::new(inner.width(), LIST_ROWS as f32 * LIST_ROW),
        );
        self.style_list(ui, panel, list, &styles);
        y = list.bottom() + GAP;
        let pieces = Rect::from_min_size(
            Pos2::new(inner.left(), y),
            Vec2::new(inner.width(), PIECE_SIDE),
        );
        self.piece_row(ui, pieces, &styles, frame, tools);
        y = pieces.bottom() + GAP;
        let wish_row = Rect::from_min_size(
            Pos2::new(inner.left(), y),
            Vec2::new(inner.width(), FIELD_ROW),
        );
        self.ask_field(ui, wish_row, frame, tools);
        y = wish_row.bottom() + GAP;
        let limits = designing
            .plot
            .map(|(width, depth)| plot_limits(width, depth));
        if live {
            let storeys = limits.map_or(DESIGNER_FLOORS, |limits| limits.storeys);
            self.buttons(
                ui,
                Pos2::new(inner.left(), y),
                designing.floor,
                storeys,
                tools,
            );
        }
        if let Some(limits) = limits {
            let counts_at = Pos2::new(inner.left(), y + FOOT_ROW * (BUTTON_ROWS - 1) as f32);
            counts_row(ui, counts_at, frame, limits);
        }
        self.show_note(ui, panel, time);
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) && live {
            tools.hand.act(Act::HouseCommand(ACTION_EXIT));
        }
        Some(panel)
    }

    fn kind_row(&mut self, ui: &egui::Ui, row: Rect) {
        let width = (row.width() - theme::ROW_GAP * (KINDS.len() - 1) as f32) / KINDS.len() as f32;
        let picked = self.design.borrow().kind;
        for (i, (kind, _)) in KINDS.into_iter().enumerate() {
            let area = Rect::from_min_size(
                row.left_top() + Vec2::new(i as f32 * (width + theme::ROW_GAP), 0.0),
                Vec2::new(width, row.height()),
            );
            let color = if kind == picked {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            if theme::segment(ui, area, kind.word(), color) {
                self.design.borrow_mut().set_kind(kind);
                self.first_style = 0;
            }
        }
    }

    fn style_list(&mut self, ui: &egui::Ui, panel: Rect, list: Rect, styles: &[&HousePart]) {
        if styles.is_empty() {
            ui.painter().text(
                list.left_top(),
                Align2::LEFT_TOP,
                WORDS_NO_PARTS,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
            return;
        }
        let last_first = styles.len().saturating_sub(LIST_ROWS);
        self.first_style = scrolled(ui, panel, self.first_style, last_first);
        let picked = self.design.borrow().style;
        for (i, part) in styles
            .iter()
            .skip(self.first_style)
            .take(LIST_ROWS)
            .enumerate()
        {
            let at = self.first_style + i;
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, i as f32 * LIST_ROW),
                Vec2::new(list.width(), LIST_ROW - theme::ROW_GAP / 2.0),
            );
            let response = ui.interact(row, Id::new(("house-style", at)), Sense::click());
            let fill = if at == picked || response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::BUTTON
            };
            let painter = ui.painter().with_clip_rect(row);
            painter.rect_filled(row, CornerRadius::same(CELL_RADIUS), fill);
            painter.text(
                row.left_center() + Vec2::new(theme::ROW_GAP, 0.0),
                Align2::LEFT_CENTER,
                &part.name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if response.clicked() {
                self.design.borrow_mut().set_style(at);
            }
        }
    }

    /// The pieces of the style: the south wall, the east wall, the corner.
    fn piece_row(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        styles: &[&HousePart],
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        let HouseDesign { style, piece, .. } = *self.design.borrow();
        let Some(part) = styles.get(style) else {
            return;
        };
        for (i, graphic) in part.pieces.iter().enumerate() {
            let cell = Rect::from_min_size(
                row.left_top() + Vec2::new(i as f32 * (PIECE_SIDE + theme::ROW_GAP), 0.0),
                Vec2::splat(PIECE_SIDE),
            );
            if cell.right() > row.right() {
                break;
            }
            let response = ui.interact(cell, Id::new(("house-piece", i)), Sense::click());
            let fill = if i == piece || response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::TRACK
            };
            ui.painter()
                .rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
            if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, *graphic, 0) {
                let area = theme::fit(cell, sprite.width, sprite.height);
                ui.painter().image(texture, area, sprite.uv, Color32::WHITE);
            }
            if response.clicked() {
                self.design.borrow_mut().piece = i;
            }
        }
    }

    fn ask_field(&mut self, ui: &mut egui::Ui, row: Rect, frame: &WatchFrame, tools: &Tools<'_>) {
        let on = tools.hand.orders_on && !frame.house_parts.is_empty();
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - ASK_WIDTH - GAP, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.wish)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .hint_text(if tools.hand.orders_on {
                    HINT_WISH
                } else {
                    HINT_WISH_OFF
                })
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        let (_, find) = theme::button(
            ui,
            Pos2::new(field.right() + GAP, row.top()),
            WORDS_FIND,
            if on { theme::GOAL } else { theme::TEXT_FAINT },
        );
        let asked = find || (typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
        if asked && on && !self.wish.trim().is_empty() {
            tools.hand.ask(
                Asker::Designer,
                Ask::HousePart {
                    wish: self.wish.trim().to_string(),
                    options: frame.house_parts.iter().map(part_words).collect(),
                },
            );
            let time = tools.time;
            self.say(WORDS_ASKING, false, time);
        }
    }

    /// The buttons of the designer: Remove and the steps that change the
    /// design, then the steps that keep and bring it back, then the levels.
    fn buttons(
        &mut self,
        ui: &egui::Ui,
        left_top: Pos2,
        floor: u8,
        storeys: u8,
        tools: &Tools<'_>,
    ) {
        let (removing, picking) = {
            let design = self.design.borrow();
            (design.removing, design.picking)
        };
        let remove_color = if removing { theme::ALARM } else { theme::TEXT };
        let (remove, pressed_remove) = theme::button(ui, left_top, WORDS_REMOVE, remove_color);
        if pressed_remove {
            self.design.borrow_mut().toggle_removing();
        }
        let (kept, changes) = DESIGN_COMMANDS.split_at(KEEPING_COMMANDS);
        let exit = (WORDS_EXIT, ACTION_EXIT);
        let rows: [(Pos2, Vec<&(&str, &'static str)>); 2] = [
            (
                Pos2::new(remove.right() + theme::ROW_GAP, left_top.y),
                changes.iter().chain([&exit]).collect(),
            ),
            (
                Pos2::new(left_top.x, left_top.y + FOOT_ROW),
                kept.iter().collect(),
            ),
        ];
        let mut kept_end = left_top.x;
        for (start, steps) in rows {
            let mut at = start;
            for &(words, action) in steps {
                let (area, pressed) = theme::button(ui, at, words, theme::TEXT);
                at = Pos2::new(area.right() + theme::ROW_GAP, start.y);
                if pressed {
                    tools.hand.act(Act::HouseCommand(action));
                }
            }
            kept_end = at.x;
        }
        let pick_color = if picking { theme::GOAL } else { theme::TEXT };
        let pick_at = Pos2::new(kept_end, left_top.y + FOOT_ROW);
        let (pick, pressed_pick) = theme::button(ui, pick_at, WORDS_PICK, pick_color);
        if pressed_pick {
            self.design.borrow_mut().toggle_picking();
        }
        if ui.rect_contains_pointer(pick) {
            super::tips::label(ui, WORDS_PICK, HINT_PICK);
        }
        let floors = Pos2::new(left_top.x, left_top.y + FOOT_ROW * 2.0);
        let label = ui.painter().text(
            Pos2::new(floors.x, floors.y + FOOT_ROW / 2.0 - theme::ROW_GAP / 2.0),
            Align2::LEFT_CENTER,
            WORDS_FLOOR,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let mut at = Pos2::new(label.right() + theme::ROW_GAP, floors.y);
        for level in 1..=storeys {
            let color = if level == floor {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            let (area, pressed) = theme::button(ui, at, &level.to_string(), color);
            at = Pos2::new(area.right() + theme::ROW_GAP, floors.y);
            if pressed {
                let act = self.design.borrow_mut().go_to_floor(level);
                tools.hand.act(act);
            }
        }
        let looks_y = floors.y + FOOT_ROW;
        let mut at = Pos2::new(left_top.x, looks_y);
        for storey in 0..usize::from(storeys) {
            let look = self.design.borrow().storeys[storey];
            let words = format!("{WORDS_STOREY} {}", storey + 1);
            let (area, pressed) = theme::button(ui, at, &words, storey_color(look));
            at = Pos2::new(area.right() + theme::ROW_GAP, looks_y);
            if pressed {
                self.design.borrow_mut().turn_storey(storey);
            }
            if ui.rect_contains_pointer(area) {
                super::tips::label(ui, look.words(), HINT_STOREY);
            }
        }
    }

    fn show_note(&mut self, ui: &egui::Ui, panel: Rect, time: f64) {
        let Some((words, failed, since)) = &self.note else {
            return;
        };
        if time - since > NOTE_SECONDS {
            self.note = None;
            return;
        }
        let color = if *failed {
            theme::ALARM
        } else {
            theme::WAITING
        };
        theme::shadowed_text(
            ui.painter(),
            Pos2::new(panel.center().x, panel.bottom() + theme::ROW_GAP),
            Align2::CENTER_TOP,
            words,
            text_font(theme::SIZE_SMALL),
            color,
        );
    }

    /// What a click on the house does: build the picked part, or take one
    /// off. None when the designer is shut.
    pub fn click_on_house(&self, frame: &WatchFrame, x: i32, y: i32, z: i32) -> Option<Act> {
        self.design.borrow_mut().click_on_house(frame, x, y, z)
    }

    /// The words beside the mouse while the designer is open.
    pub fn hint(&self) -> &'static str {
        self.design.borrow().hint()
    }
}

/// The color of a storey's button by how it shows: all shown, some
/// see-through, some hidden.
fn storey_color(look: StoreyLook) -> Color32 {
    [theme::TEXT, theme::WAITING, theme::ALARM][look.button_look()]
}

/// The components and fixtures of the design against the most the plot
/// takes, and what it costs; a count at its most in the alarm color.
fn counts_row(ui: &egui::Ui, left_top: Pos2, frame: &WatchFrame, limits: PlotLimits) {
    let counts = design_counts(frame);
    let font = text_font(theme::SIZE_BODY);
    let color = |count: u32, most: u32| {
        if count >= most {
            theme::ALARM
        } else {
            theme::TEXT
        }
    };
    let mut job = LayoutJob::default();
    let parts = [
        (
            format!(
                "{WORDS_COMPONENTS} {}/{}   ",
                counts.components, limits.components
            ),
            color(counts.components, limits.components),
        ),
        (
            format!(
                "{WORDS_FIXTURES} {}/{}   ",
                counts.fixtures, limits.fixtures
            ),
            color(counts.fixtures, limits.fixtures),
        ),
        (format!("{WORDS_COST} {}", counts.cost()), theme::TEXT),
    ];
    for (words, color) in parts {
        job.append(&words, 0.0, TextFormat::simple(font.clone(), color));
    }
    let galley = ui.painter().layout_job(job);
    let area = Rect::from_min_size(left_top, galley.size());
    ui.painter().galley(left_top, galley, theme::TEXT);
    if ui.rect_contains_pointer(area) {
        super::tips::label(ui, &limits_words(limits), "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::HousePartKind;

    fn designing() -> WatchFrame {
        WatchFrame {
            designing: Some(crate::view::WatchDesigning {
                serial: 70,
                floor: 1,
                ..crate::view::WatchDesigning::default()
            }),
            house_parts: vec![
                HousePart {
                    kind: HousePartKind::Wall,
                    name: "Dark Wood".into(),
                    pieces: vec![10, 7, 12],
                },
                HousePart {
                    kind: HousePartKind::Wall,
                    name: "Stone".into(),
                    pieces: vec![20, 22],
                },
                HousePart {
                    kind: HousePartKind::Roof,
                    name: "Tile Roof".into(),
                    pieces: vec![11314],
                },
            ],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_answer_of_jev_picks_the_style_and_its_kind_in_the_shared_design() {
        let mut build = BuildUi::default();
        let shared = build.design();
        let frame = designing();
        build.pick_part(&frame, 2);
        assert_eq!(shared.borrow().kind, HousePartKind::Roof);
        build.pick_part(&frame, 1);
        assert_eq!(
            (shared.borrow().kind, shared.borrow().style),
            (HousePartKind::Wall, 1)
        );
        build.pick_part(&frame, 99);
        assert_eq!(
            shared.borrow().style,
            1,
            "a place the catalog has not changes nothing"
        );
        shared.borrow_mut().piece = 1;
        assert!(matches!(
            build.click_on_house(&frame, 3, 4, 0),
            Some(Act::HouseEdit { graphic: 22, .. })
        ));
    }
}

const CHAT_ID: &str = "modern:chat";
const CHAT_SIZE: Vec2 = Vec2::new(420.0, 600.0);
const CHAT_LEAST_SIZE: Vec2 = Vec2::new(360.0, 460.0);
const CHANNEL_ROW: f32 = 24.0;
const CHANNEL_ROWS: usize = 4;
const LABEL_WIDTH: f32 = 80.0;

const WORDS_CHAT: &str = "Chat";
const WORDS_JOIN: &str = "Join";
const WORDS_LEAVE: &str = "Leave";
const WORDS_CREATE: &str = "Create";
const WORDS_OKAY: &str = "Okay";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_TURN_ON: &str = "Turn the chat on";
const WORDS_NAME: &str = "Name:";
const WORDS_PASSWORD: &str = "Password:";
const WORDS_LOCKED: &str = "needs a password";
const WORDS_LOCK_MARK: &str = "locked";
const HINT_CHANNEL_ROW: &str = "Click: pick.  Double-click: join.";
const HINT_SAY: &str = "Words for the channel. Press Enter.";
const HINT_CHANNEL: &str = "Say the channel in plain words, for example: the trade one";

/// The chat of the shard: its channels, its lines, and a box to talk in.
/// It opens by itself when the shard opens the chat or asks for the chat
/// name.
#[derive(Default)]
pub struct ChatUi {
    open: bool,
    words: String,
    wish: String,
    /// The channel the player picked in the list.
    picked: Option<String>,
    first_channel: usize,
    /// The newest lines hidden under the view, and how many lines there
    /// were when the panel last looked.
    back: usize,
    seen: usize,
    /// The small box that asks for a channel name or a password, with the
    /// words typed in it.
    asking: Option<(Asking, String)>,
    /// The chat name typed for the shard.
    name: String,
    watch: ChatWatch,
    note: Option<(String, bool, f64)>,
}

impl ChatUi {
    /// The chat as the window starts.
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            ..Self::default()
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Draws the chat when it is open. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let time = tools.time;
        self.take_answers(frame, tools, time);
        if self.watch.opened(frame) {
            self.open = true;
        }
        if !self.open {
            return None;
        }
        let title = match frame.chat.as_ref() {
            Some(chat) if !chat.in_channel.is_empty() => {
                format!("{WORDS_CHAT}: {}", chat.in_channel)
            }
            _ => WORDS_CHAT.to_string(),
        };
        let spec = PanelSpec {
            id: CHAT_ID,
            title: &title,
            default: layout::first_place(rect, Spot::LeftColumn(0), CHAT_SIZE),
            min_size: Some(CHAT_LEAST_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        ui.painter()
            .rect_filled(panel, CornerRadius::same(theme::PANEL_RADIUS), theme::VOID);
        let body = frame::draw(ui.painter(), panel, &title);
        let live = frame.human_control;
        match (frame.chat.as_ref(), chat_door(frame)) {
            (Some(chat), _) => self.chat(ui, body, chat, tools, live),
            (None, ChatDoor::AskName) => self.name_box(ui, body, tools, live),
            (None, door) => {
                if let (ChatDoor::TurnOn(act), true) = (door, live) {
                    let (_, pressed) =
                        theme::button(ui, body.left_top(), WORDS_TURN_ON, theme::GOAL);
                    if pressed {
                        tools.hand.act(act);
                    }
                }
            }
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.open = false;
        }
        self.show_note(ui, panel, time);
        Some(panel)
    }

    /// The channels, their buttons, the lines and the fields to talk in.
    fn chat(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        chat: &WatchChat,
        tools: &Tools<'_>,
        live: bool,
    ) {
        let channels = Rect::from_min_size(
            body.min,
            Vec2::new(body.width(), CHANNEL_ROWS as f32 * CHANNEL_ROW),
        );
        self.channels(ui, channels, chat, tools, live);
        let mut top = channels.bottom() + GAP;
        if live {
            self.channel_buttons(ui, Pos2::new(body.left(), top), chat, tools);
            top += FOOT_ROW;
            if self.asking.is_some() {
                let row = Rect::from_min_size(
                    Pos2::new(body.left(), top),
                    Vec2::new(body.width(), FIELD_ROW),
                );
                self.small_box(ui, row, tools);
                top = row.bottom() + GAP;
            }
        }
        let fields = if live {
            FIELD_ROW * 2.0 + GAP * 2.0
        } else {
            0.0
        };
        let lines = Rect::from_min_max(
            Pos2::new(body.left(), top),
            Pos2::new(body.right(), body.bottom() - fields),
        );
        self.lines(ui, lines, chat);
        if !live {
            return;
        }
        let say_row = Rect::from_min_size(
            Pos2::new(body.left(), lines.bottom() + GAP),
            Vec2::new(body.width(), FIELD_ROW),
        );
        self.say_field(ui, say_row, tools);
        let wish_row = Rect::from_min_size(
            Pos2::new(body.left(), say_row.bottom() + GAP),
            Vec2::new(body.width(), FIELD_ROW),
        );
        self.channel_field(ui, wish_row, chat, tools);
    }

    fn take_answers(&mut self, frame: &WatchFrame, tools: &Tools<'_>, time: f64) {
        for answer in tools.hand.new_answers(Asker::Chat) {
            let Some(chat) = frame.chat.as_ref() else {
                continue;
            };
            match answer {
                Answer::Picked(Ok(place)) => {
                    if let Some((name, _)) = chat.channels.get(place) {
                        self.join(chat, name.clone(), tools);
                        self.wish.clear();
                        self.note = None;
                    }
                }
                Answer::Picked(Err(words)) => self.note = Some((words, true, time)),
                _ => {}
            }
        }
    }

    /// Joins a channel: at once, or through the box that asks for its
    /// password.
    fn join(&mut self, chat: &WatchChat, channel: String, tools: &Tools<'_>) {
        match joining(chat, Some(&channel)) {
            Joining::Send(act) => tools.hand.act(act),
            Joining::AskPassword(channel) => {
                self.asking = Some((Asking::Password(channel), String::new()));
            }
            Joining::Nothing => {}
        }
        self.picked = Some(channel);
    }

    /// The list of every channel. A click picks one and a double click
    /// joins it.
    fn channels(
        &mut self,
        ui: &egui::Ui,
        area: Rect,
        chat: &WatchChat,
        tools: &Tools<'_>,
        live: bool,
    ) {
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let last_first = chat.channels.len().saturating_sub(CHANNEL_ROWS);
        self.first_channel = scrolled(ui, area, self.first_channel, last_first);
        let shown = chat
            .channels
            .iter()
            .enumerate()
            .skip(self.first_channel)
            .take(CHANNEL_ROWS);
        for (row_at, (at, (name, locked))) in shown.enumerate() {
            let row = Rect::from_min_size(
                area.left_top() + Vec2::new(0.0, row_at as f32 * CHANNEL_ROW),
                Vec2::new(area.width(), CHANNEL_ROW),
            );
            let response = ui.interact(row, Id::new(("chat-channel", at)), Sense::click());
            let picked = self.picked.as_deref() == Some(name.as_str());
            if picked || (live && response.hovered()) {
                ui.painter()
                    .rect_filled(row, CornerRadius::same(CELL_RADIUS), theme::BUTTON_HOVER);
            }
            let color = if *name == chat.in_channel {
                theme::GOAL
            } else {
                theme::TEXT
            };
            ui.painter().text(
                row.left_center() + Vec2::new(theme::ROW_GAP, 0.0),
                Align2::LEFT_CENTER,
                name,
                text_font(theme::SIZE_BODY),
                color,
            );
            if *locked {
                ui.painter().text(
                    row.right_center() - Vec2::new(theme::ROW_GAP, 0.0),
                    Align2::RIGHT_CENTER,
                    WORDS_LOCK_MARK,
                    text_font(theme::SIZE_SMALL),
                    theme::WAITING,
                );
            }
            if !live {
                continue;
            }
            if response.hovered() {
                super::tips::label(ui, HINT_CHANNEL_ROW, "");
            }
            if response.double_clicked() {
                self.join(chat, name.clone(), tools);
            } else if response.clicked() {
                self.picked = Some(name.clone());
            }
        }
    }

    /// Join, Leave and Create.
    fn channel_buttons(
        &mut self,
        ui: &egui::Ui,
        left_top: Pos2,
        chat: &WatchChat,
        tools: &Tools<'_>,
    ) {
        let (join, joined) = theme::button(ui, left_top, WORDS_JOIN, theme::GOAL);
        let (leave, left) = theme::button(
            ui,
            Pos2::new(join.right() + theme::ROW_GAP, left_top.y),
            WORDS_LEAVE,
            theme::TEXT,
        );
        let (_, create) = theme::button(
            ui,
            Pos2::new(leave.right() + theme::ROW_GAP, left_top.y),
            WORDS_CREATE,
            theme::TEXT,
        );
        if let Some(channel) = self.picked.clone().filter(|_| joined) {
            self.join(chat, channel, tools);
        } else if left {
            tools.hand.act(Act::ChatLeave);
        } else if create {
            self.asking = Some((Asking::NewChannel, String::new()));
        }
    }

    /// The small box that asks for the name of a new channel or the
    /// password of one.
    fn small_box(&mut self, ui: &mut egui::Ui, row: Rect, tools: &Tools<'_>) {
        let Some((asking, words)) = self.asking.as_mut() else {
            return;
        };
        let label = match asking {
            Asking::NewChannel => WORDS_NAME,
            Asking::Password(_) => WORDS_PASSWORD,
        };
        ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            label,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let okay_width = ASK_WIDTH * 2.0 + GAP * 2.0;
        let field = Rect::from_min_max(
            Pos2::new(row.left() + LABEL_WIDTH, row.top()),
            Pos2::new(row.right() - okay_width, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(words)
                .id(Id::new("chat-small-box"))
                .password(asking.hides_words())
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let (okay_area, okay) = theme::button(
            ui,
            Pos2::new(field.right() + GAP, row.top()),
            WORDS_OKAY,
            theme::GOAL,
        );
        let (_, cancel) = theme::button(
            ui,
            Pos2::new(okay_area.right() + GAP, row.top()),
            WORDS_CANCEL,
            theme::TEXT_DIM,
        );
        let entered = typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if okay || entered {
            if let Some(act) = asking.act(words) {
                tools.hand.act(act);
            }
        }
        if okay || entered || cancel {
            self.asking = None;
        }
    }

    /// The box that asks for the chat name before the chat opens.
    fn name_box(&mut self, ui: &mut egui::Ui, body: Rect, tools: &Tools<'_>, live: bool) {
        let words = ui.painter().layout(
            WORDS_CHOOSE_NAME.to_string(),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
            body.width(),
        );
        let words_height = words.size().y;
        ui.painter().galley(body.left_top(), words, theme::TEXT_DIM);
        if !live {
            return;
        }
        let row = Rect::from_min_size(
            body.left_top() + Vec2::new(0.0, words_height + GAP),
            Vec2::new(body.width(), FIELD_ROW),
        );
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - ASK_WIDTH - GAP, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.name)
                .id(Id::new("chat-name"))
                .char_limit(CHAT_NAME_MAX_CHARS)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let (_, okay) = theme::button(
            ui,
            Pos2::new(field.right() + GAP, row.top()),
            WORDS_OKAY,
            theme::GOAL,
        );
        let entered = typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if okay || entered {
            if let Some(act) = chat_name_act(&self.name) {
                tools.hand.act(act);
            }
        }
    }

    /// The lines said in the channel, the newest at the bottom. The wheel
    /// looks back, and the view follows the newest line when it is there.
    fn lines(&mut self, ui: &egui::Ui, area: Rect, chat: &WatchChat) {
        let count = chat.lines.len();
        self.back = lines_back(self.back, self.seen, count);
        self.seen = count;
        let most_back = count.saturating_sub(1);
        let first = scrolled(ui, area, most_back - self.back.min(most_back), most_back);
        self.back = most_back - first;
        let painter = ui.painter().with_clip_rect(area);
        let mut bottom = area.bottom();
        for (who, words) in chat.lines.iter().rev().skip(self.back) {
            let mut job = LayoutJob::default();
            job.append(
                &format!("{who} "),
                0.0,
                TextFormat::simple(text_font(theme::SIZE_BODY), theme::GOAL),
            );
            job.append(
                words,
                0.0,
                TextFormat::simple(text_font(theme::SIZE_BODY), theme::TEXT),
            );
            job.wrap.max_width = area.width();
            let galley = painter.layout_job(job);
            let top = bottom - galley.size().y;
            if top < area.top() {
                break;
            }
            painter.galley(Pos2::new(area.left(), top), galley, theme::TEXT);
            bottom = top;
        }
    }

    fn say_field(&mut self, ui: &mut egui::Ui, row: Rect, tools: &Tools<'_>) {
        ui.painter()
            .rect_filled(row, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            row,
            egui::TextEdit::singleline(&mut self.words)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .hint_text(HINT_SAY)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let sent = typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let words = self.words.trim().to_string();
        if sent && !words.is_empty() {
            tools.hand.act(Act::ChatSay(words));
            self.words.clear();
            typed.request_focus();
        }
    }

    fn channel_field(&mut self, ui: &mut egui::Ui, row: Rect, chat: &WatchChat, tools: &Tools<'_>) {
        let on = tools.hand.orders_on && !chat.channels.is_empty();
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - ASK_WIDTH - GAP, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.wish)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .hint_text(if tools.hand.orders_on {
                    HINT_CHANNEL
                } else {
                    HINT_WISH_OFF
                })
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        let (_, join) = theme::button(
            ui,
            Pos2::new(field.right() + GAP, row.top()),
            WORDS_FIND,
            if on { theme::GOAL } else { theme::TEXT_FAINT },
        );
        let asked = join || (typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
        if asked && on && !self.wish.trim().is_empty() {
            tools.hand.ask(
                Asker::Chat,
                Ask::Channel {
                    wish: self.wish.trim().to_string(),
                    options: chat
                        .channels
                        .iter()
                        .map(|(name, locked)| {
                            if *locked {
                                format!("{name} ({WORDS_LOCKED})")
                            } else {
                                name.clone()
                            }
                        })
                        .collect(),
                },
            );
            self.note = Some((WORDS_ASKING.into(), false, tools.time));
        }
    }

    fn show_note(&mut self, ui: &egui::Ui, panel: Rect, time: f64) {
        let Some((words, failed, since)) = &self.note else {
            return;
        };
        if time - since > NOTE_SECONDS {
            self.note = None;
            return;
        }
        let color = if *failed {
            theme::ALARM
        } else {
            theme::WAITING
        };
        theme::shadowed_text(
            ui.painter(),
            Pos2::new(panel.center().x, panel.bottom() + theme::ROW_GAP),
            Align2::CENTER_TOP,
            words,
            text_font(theme::SIZE_SMALL),
            color,
        );
    }
}

#[cfg(test)]
mod chat_tests {
    use super::*;
    use crate::window::modern::testing::draw_frames;

    fn chat() -> WatchChat {
        WatchChat {
            name: "Mara".into(),
            channels: (0..12)
                .map(|at| (format!("Channel {at}"), at == 1))
                .collect(),
            in_channel: "Channel 0".into(),
            lines: vec![("Ann".into(), "hail".into())],
        }
    }

    fn draw(ui_chat: &mut ChatUi, frame: &WatchFrame) -> Option<Rect> {
        let mut profile = Profile::default();
        let mut shown = None;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = ui_chat.draw(ui, rect, frame, tools, profile);
        });
        shown
    }

    #[test]
    fn the_chat_opens_itself_when_the_shard_opens_it_and_stays_closed_after() {
        let mut ui_chat = ChatUi::default();
        assert!(draw(&mut ui_chat, &WatchFrame::default()).is_none());
        let frame = WatchFrame {
            human_control: true,
            chat: Some(chat()),
            ..WatchFrame::default()
        };
        assert!(draw(&mut ui_chat, &frame).is_some(), "the shard opened it");
        ui_chat.toggle();
        assert!(draw(&mut ui_chat, &frame).is_none(), "the player closed it");
    }

    #[test]
    fn a_locked_channel_asks_its_password_and_an_open_one_joins() {
        let mut ui_chat = ChatUi::default();
        let frame = WatchFrame {
            human_control: true,
            chat: Some(chat()),
            ..WatchFrame::default()
        };
        draw(&mut ui_chat, &frame);
        let mut profile = Profile::default();
        draw_frames(&mut profile, &[Vec::new()], |_, _, tools, _| {
            let chat = chat();
            ui_chat.join(&chat, "Channel 1".into(), tools);
            assert_eq!(
                ui_chat.asking.as_ref().map(|(asking, _)| asking.clone()),
                Some(Asking::Password("Channel 1".into()))
            );
            ui_chat.asking = None;
            ui_chat.join(&chat, "Channel 2".into(), tools);
            assert!(ui_chat.asking.is_none());
        });
        assert_eq!(ui_chat.picked.as_deref(), Some("Channel 2"));
    }

    #[test]
    fn the_shard_asking_for_the_chat_name_opens_the_name_box() {
        let mut ui_chat = ChatUi::default();
        let frame = WatchFrame {
            human_control: true,
            chat_asks_for_name: true,
            ..WatchFrame::default()
        };
        assert!(draw(&mut ui_chat, &frame).is_some());
        assert!(ui_chat.is_open());
    }
}
