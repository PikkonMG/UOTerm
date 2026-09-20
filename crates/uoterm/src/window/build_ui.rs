//! The house designer and the chat of the shard.
//!
//! The designer shows the parts of the client catalog. The human picks a
//! part and clicks the map to put it on the house. With a TypeSafe key, a
//! field takes the part in plain words, such as "a stone wall", and Jev
//! picks it from the catalog.
//!
//! The chat shows the channels of the shard and the lines that were said.
//! Its own field takes a channel in plain words the same way.

use super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::control::{Act, Answer, Ask};
use super::theme::{self, number_font, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_nav::{HousePart, HousePartKind};

const PANEL_WIDTH: f32 = 430.0;
/// The panels hang below the bar at the top of the window.
const PANEL_TOP: f32 = 150.0;
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
const WORDS_FLOOR: &str = "Floor";
const WORDS_COMMIT: &str = "Save";
const WORDS_REVERT: &str = "Undo all";
const WORDS_CLEAR: &str = "Clear";
const WORDS_EXIT: &str = "Leave";
const WORDS_FIND: &str = "Find";
const WORDS_ASKING: &str = "Jev looks at the catalog...";
const WORDS_NO_PARTS: &str = "The catalog needs the client files.";
const HINT_WISH: &str = "Say the part in plain words, for example: a stone wall";
const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
const HINT_BUILD: &str = "Click the house to build here.";
const HINT_REMOVE: &str = "Click a part of the house to take it off.";

/// The kinds of part, and the designer action that puts one on the house.
const KINDS: [(HousePartKind, &str); 7] = [
    (HousePartKind::Wall, "add"),
    (HousePartKind::Floor, "add"),
    (HousePartKind::Door, "add"),
    (HousePartKind::Stair, "stair"),
    (HousePartKind::Roof, "roof"),
    (HousePartKind::Misc, "add"),
    (HousePartKind::Teleporter, "add"),
];

pub struct BuildUi {
    kind: HousePartKind,
    /// The style of the kind that is picked, and which of its pieces.
    style: usize,
    piece: usize,
    first_style: usize,
    /// The human takes parts off the house instead of putting them on.
    removing: bool,
    wish: String,
    note: Option<(String, bool, f64)>,
}

impl BuildUi {
    /// The designer as the window starts. It shows itself when the shard
    /// opens the designer, so an operator needs no button for it.
    pub fn starting() -> Self {
        Self {
            kind: HousePartKind::Wall,
            style: 0,
            piece: 0,
            first_style: 0,
            removing: false,
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

/// The action that puts a part of this kind on the house.
fn action_for(kind: HousePartKind) -> &'static str {
    KINDS
        .iter()
        .find(|(known, _)| *known == kind)
        .map_or("add", |(_, action)| *action)
}

/// The action that takes a part of this kind off the house.
fn remove_action_for(kind: HousePartKind) -> &'static str {
    if kind == HousePartKind::Roof {
        "remove_roof"
    } else {
        "remove"
    }
}

/// The styles of one kind, in the order the catalog lists them.
fn styles_of(parts: &[HousePart], kind: HousePartKind) -> Vec<&HousePart> {
    parts.iter().filter(|part| part.kind == kind).collect()
}

/// What Jev reads about each part of the catalog.
fn part_words(part: &HousePart) -> String {
    format!("{}: {}", part.kind.word(), part.name)
}

impl BuildUi {
    fn say(&mut self, words: &str, failed: bool, time: f64) {
        self.note = Some((words.to_string(), failed, time));
    }

    /// The part the human builds with now, when the designer is open.
    fn picked<'a>(&self, parts: &'a [HousePart]) -> Option<(&'a HousePart, u16)> {
        let styles = styles_of(parts, self.kind);
        let part = *styles.get(self.style)?;
        let piece = *part.pieces.get(self.piece)?;
        Some((part, piece))
    }

    /// Picks the part at this place of the catalog.
    fn pick_part(&mut self, frame: &WatchFrame, place: usize) {
        let Some(part) = frame.house_parts.get(place) else {
            return;
        };
        self.kind = part.kind;
        self.style = styles_of(&frame.house_parts, part.kind)
            .iter()
            .position(|known| known.name == part.name)
            .unwrap_or(0);
        self.piece = 0;
        self.first_style = self.style.saturating_sub(LIST_ROWS / 2);
        self.wish.clear();
        self.note = None;
    }

    /// Takes the answer of Jev about a part of the catalog.
    fn take_answers(&mut self, frame: &WatchFrame, tools: &Tools<'_>, time: f64) {
        for answer in tools.hand.new_answers() {
            match answer {
                Answer::Picked(Ok(place)) => self.pick_part(frame, place),
                Answer::Picked(Err(words)) => self.say(&words, true, time),
                // The macro editor and the map item take the rest.
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
    ) -> Option<Rect> {
        let time = tools.time;
        self.take_answers(frame, tools, time);
        let designing = frame.designing?;
        let styles = styles_of(&frame.house_parts, self.kind);
        self.style = self.style.min(styles.len().saturating_sub(1));
        let panel = Rect::from_min_size(
            Pos2::new(
                rect.right() - theme::SCREEN_MARGIN - PANEL_WIDTH,
                rect.top() + PANEL_TOP,
            ),
            Vec2::new(
                PANEL_WIDTH,
                TITLE_ROW * 2.0
                    + LIST_ROWS as f32 * LIST_ROW
                    + PIECE_SIDE
                    + FIELD_ROW
                    + FOOT_ROW * 2.0
                    + GAP * 4.0
                    + theme::PANEL_PAD * 2.0,
            ),
        );
        ui.painter()
            .rect_filled(panel, CornerRadius::same(theme::PANEL_RADIUS), theme::VOID);
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        ui.painter().text(
            inner.right_top() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::RIGHT_TOP,
            format!("{WORDS_FLOOR} {}", designing.floor),
            number_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let mut y = inner.top() + TITLE_ROW;
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
        self.buttons(ui, Pos2::new(inner.left(), y), tools);
        self.show_note(ui, panel, time);
        Some(panel)
    }

    fn kind_row(&mut self, ui: &egui::Ui, row: Rect) {
        let width = (row.width() - theme::ROW_GAP * (KINDS.len() - 1) as f32) / KINDS.len() as f32;
        for (i, (kind, _)) in KINDS.into_iter().enumerate() {
            let area = Rect::from_min_size(
                row.left_top() + Vec2::new(i as f32 * (width + theme::ROW_GAP), 0.0),
                Vec2::new(width, row.height()),
            );
            let color = if kind == self.kind {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            if theme::segment(ui, area, kind.word(), color) {
                self.kind = kind;
                self.style = 0;
                self.piece = 0;
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
            let fill = if at == self.style || response.hovered() {
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
                self.style = at;
                self.piece = 0;
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
        let Some(part) = styles.get(self.style) else {
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
            let fill = if i == self.piece || response.hovered() {
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
                self.piece = i;
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
            tools.hand.ask(Ask::HousePart {
                wish: self.wish.trim().to_string(),
                options: frame.house_parts.iter().map(part_words).collect(),
            });
            let time = tools.time;
            self.say(WORDS_ASKING, false, time);
        }
    }

    fn buttons(&mut self, ui: &egui::Ui, left_top: Pos2, tools: &Tools<'_>) {
        let remove_color = if self.removing {
            theme::ALARM
        } else {
            theme::TEXT
        };
        let (remove, pressed_remove) = theme::button(ui, left_top, WORDS_REMOVE, remove_color);
        if pressed_remove {
            self.removing = !self.removing;
        }
        let mut at = Pos2::new(remove.right() + theme::ROW_GAP, left_top.y);
        for (words, action) in [
            (WORDS_COMMIT, "commit"),
            (WORDS_REVERT, "revert"),
            (WORDS_CLEAR, "clear"),
            (WORDS_EXIT, "exit"),
        ] {
            let color = match words {
                WORDS_COMMIT => theme::GOAL,
                WORDS_CLEAR => theme::ALARM,
                _ => theme::TEXT,
            };
            let (area, pressed) = theme::button(ui, at, words, color);
            at = Pos2::new(area.right() + theme::ROW_GAP, left_top.y);
            if pressed {
                tools.hand.act(Act::HouseCommand(action));
            }
        }
        let floors = Pos2::new(left_top.x, left_top.y + FOOT_ROW);
        let mut at = floors;
        for level in 1..=DESIGNER_FLOORS {
            let (area, pressed) = theme::button(ui, at, &level.to_string(), theme::TEXT_DIM);
            at = Pos2::new(area.right() + theme::ROW_GAP, floors.y);
            if pressed {
                tools.hand.act(Act::HouseFloor(level));
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
        frame.designing?;
        let (part, graphic) = self.picked(&frame.house_parts)?;
        let action = if self.removing {
            remove_action_for(part.kind)
        } else {
            action_for(part.kind)
        };
        Some(Act::HouseEdit {
            action,
            graphic,
            x,
            y,
            z,
        })
    }

    /// The words beside the mouse while the designer is open.
    pub fn hint(&self) -> &'static str {
        if self.removing {
            HINT_REMOVE
        } else {
            HINT_BUILD
        }
    }
}

/// How many levels a house can have.
const DESIGNER_FLOORS: u8 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Vec<HousePart> {
        vec![
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
        ]
    }

    fn designing() -> WatchFrame {
        WatchFrame {
            designing: Some(crate::view::WatchDesigning {
                serial: 70,
                floor: 1,
            }),
            house_parts: catalog(),
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_click_builds_the_picked_piece_of_the_picked_style() {
        let mut build = BuildUi {
            style: 1,
            piece: 1,
            ..BuildUi::default()
        };
        let frame = designing();
        assert_eq!(
            build.click_on_house(&frame, 3, 4, 0),
            Some(Act::HouseEdit {
                action: "add",
                graphic: 22,
                x: 3,
                y: 4,
                z: 0
            })
        );
        build.removing = true;
        assert!(matches!(
            build.click_on_house(&frame, 3, 4, 7),
            Some(Act::HouseEdit {
                action: "remove",
                z: 7,
                ..
            })
        ));
        // A roof goes on and comes off with its own commands.
        build.kind = HousePartKind::Roof;
        build.style = 0;
        build.piece = 0;
        assert!(matches!(
            build.click_on_house(&frame, 0, 0, 0),
            Some(Act::HouseEdit {
                action: "remove_roof",
                ..
            })
        ));
        build.removing = false;
        assert!(matches!(
            build.click_on_house(&frame, 0, 0, 0),
            Some(Act::HouseEdit { action: "roof", .. })
        ));
    }

    #[test]
    fn a_click_does_nothing_while_the_designer_is_shut() {
        let build = BuildUi::default();
        assert_eq!(build.click_on_house(&WatchFrame::default(), 1, 1, 0), None);
        let empty = WatchFrame {
            designing: Some(crate::view::WatchDesigning::default()),
            ..WatchFrame::default()
        };
        assert_eq!(build.click_on_house(&empty, 1, 1, 0), None);
    }

    #[test]
    fn jev_reads_the_kind_and_the_name_of_each_part() {
        assert_eq!(part_words(&catalog()[0]), "wall: Dark Wood");
        assert_eq!(part_words(&catalog()[2]), "roof: Tile Roof");
    }

    #[test]
    fn the_answer_of_jev_picks_the_style_and_its_kind() {
        let mut build = BuildUi::default();
        let frame = designing();
        build.pick_part(&frame, 2);
        assert_eq!(build.kind, HousePartKind::Roof);
        assert_eq!((build.style, build.piece), (0, 0));
        build.pick_part(&frame, 1);
        assert_eq!((build.kind, build.style), (HousePartKind::Wall, 1));
        build.pick_part(&frame, 99);
        assert_eq!(
            build.style, 1,
            "a place the catalog has not changes nothing"
        );
    }
}

const CHAT_WIDTH: f32 = 400.0;
const CHAT_ROWS: usize = 10;
const CHAT_LINE: f32 = 20.0;
const CHANNEL_ROW: f32 = 26.0;
const CHANNELS_SHOWN: usize = 4;

const WORDS_CHAT: &str = "Chat";
const WORDS_JOIN: &str = "Join";
const WORDS_LEAVE: &str = "Leave";
const WORDS_CLOSE: &str = "Close";
const WORDS_TURN_ON: &str = "Turn the chat on";
const WORDS_LOCKED: &str = "needs a password";
const HINT_SAY: &str = "Words for the channel. Press Enter.";
const HINT_CHANNEL: &str = "Say the channel in plain words, for example: the trade one";

/// The chat of the shard: its channels, its lines, and a box to talk in.
#[derive(Default)]
pub struct ChatUi {
    open: bool,
    words: String,
    wish: String,
    first_line: usize,
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

    /// Draws the chat when it is open. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Option<Rect> {
        let time = tools.time;
        self.take_answers(frame, tools, time);
        if !self.open {
            return None;
        }
        let chat = frame.chat.as_ref();
        let panel = Rect::from_min_size(
            Pos2::new(rect.left() + theme::SCREEN_MARGIN, rect.top() + PANEL_TOP),
            Vec2::new(
                CHAT_WIDTH,
                TITLE_ROW
                    + CHANNEL_ROW
                    + CHAT_ROWS as f32 * CHAT_LINE
                    + FIELD_ROW * 2.0
                    + FOOT_ROW
                    + GAP * 4.0
                    + theme::PANEL_PAD * 2.0,
            ),
        );
        ui.painter()
            .rect_filled(panel, CornerRadius::same(theme::PANEL_RADIUS), theme::VOID);
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let title = match chat {
            Some(chat) if !chat.in_channel.is_empty() => {
                format!("{WORDS_CHAT}: {}", chat.in_channel)
            }
            _ => WORDS_CHAT.to_string(),
        };
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            title,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let mut y = inner.top() + TITLE_ROW;
        let Some(chat) = chat else {
            self.turn_on(
                ui,
                Rect::from_min_size(
                    Pos2::new(inner.left(), y),
                    Vec2::new(inner.width(), FOOT_ROW),
                ),
                frame,
                tools,
            );
            return Some(panel);
        };
        self.channels(
            ui,
            Rect::from_min_size(
                Pos2::new(inner.left(), y),
                Vec2::new(inner.width(), CHANNEL_ROW),
            ),
            chat,
            tools,
        );
        y += CHANNEL_ROW + GAP;
        let lines = Rect::from_min_size(
            Pos2::new(inner.left(), y),
            Vec2::new(inner.width(), CHAT_ROWS as f32 * CHAT_LINE),
        );
        self.lines(ui, panel, lines, chat);
        y = lines.bottom() + GAP;
        let say_row = Rect::from_min_size(
            Pos2::new(inner.left(), y),
            Vec2::new(inner.width(), FIELD_ROW),
        );
        self.say_field(ui, say_row, frame, tools);
        y = say_row.bottom() + GAP;
        let wish_row = Rect::from_min_size(
            Pos2::new(inner.left(), y),
            Vec2::new(inner.width(), FIELD_ROW),
        );
        self.channel_field(ui, wish_row, chat, tools);
        y = wish_row.bottom() + GAP;
        let (leave, left) = theme::button(ui, Pos2::new(inner.left(), y), WORDS_LEAVE, theme::TEXT);
        let (_, closed) = theme::button(
            ui,
            Pos2::new(leave.right() + theme::ROW_GAP, y),
            WORDS_CLOSE,
            theme::TEXT_DIM,
        );
        if left {
            tools.hand.act(Act::ChatLeave);
        } else if closed {
            self.open = false;
        }
        self.show_note(ui, panel, time);
        Some(panel)
    }

    fn take_answers(&mut self, frame: &WatchFrame, tools: &Tools<'_>, time: f64) {
        for answer in tools.hand.new_answers() {
            let Some(chat) = frame.chat.as_ref() else {
                continue;
            };
            match answer {
                Answer::Picked(Ok(place)) => {
                    if let Some((name, _)) = chat.channels.get(place) {
                        tools.hand.act(Act::ChatJoin(name.clone()));
                        self.wish.clear();
                        self.note = None;
                    }
                }
                Answer::Picked(Err(words)) => self.note = Some((words, true, time)),
                _ => {}
            }
        }
    }

    fn turn_on(&mut self, ui: &egui::Ui, row: Rect, frame: &WatchFrame, tools: &Tools<'_>) {
        let (_, pressed) = theme::button(ui, row.left_top(), WORDS_TURN_ON, theme::GOAL);
        if pressed {
            tools.hand.act(Act::ChatOpen(frame.name.clone()));
        }
        let (_, closed) = theme::button(
            ui,
            Pos2::new(row.left() + CHAT_WIDTH / 2.0, row.top()),
            WORDS_CLOSE,
            theme::TEXT_DIM,
        );
        if closed {
            self.open = false;
        }
    }

    fn channels(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        chat: &crate::view::WatchChat,
        tools: &Tools<'_>,
    ) {
        let shown = chat.channels.len().clamp(1, CHANNELS_SHOWN);
        let width = (row.width() - theme::ROW_GAP * (shown - 1) as f32) / shown as f32;
        for (i, (name, locked)) in chat.channels.iter().take(CHANNELS_SHOWN).enumerate() {
            let area = Rect::from_min_size(
                row.left_top() + Vec2::new(i as f32 * (width + theme::ROW_GAP), 0.0),
                Vec2::new(width, row.height()),
            );
            let color = if *name == chat.in_channel {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            if theme::segment(ui, area, name, color) {
                tools.hand.act(Act::ChatJoin(name.clone()));
            }
            if *locked {
                ui.painter().text(
                    area.right_bottom(),
                    Align2::RIGHT_BOTTOM,
                    "*",
                    text_font(theme::SIZE_SMALL),
                    theme::WAITING,
                );
            }
        }
    }

    fn lines(&mut self, ui: &egui::Ui, panel: Rect, area: Rect, chat: &crate::view::WatchChat) {
        let last_first = chat.lines.len().saturating_sub(CHAT_ROWS);
        // A new line brings the list to its end by itself.
        if self.first_line >= last_first.saturating_sub(1) {
            self.first_line = last_first;
        }
        self.first_line = scrolled(ui, panel, self.first_line, last_first);
        let painter = ui.painter().with_clip_rect(area);
        for (i, (who, words)) in chat
            .lines
            .iter()
            .skip(self.first_line)
            .take(CHAT_ROWS)
            .enumerate()
        {
            let at = area.left_top() + Vec2::new(0.0, i as f32 * CHAT_LINE);
            let name = painter.text(
                at,
                Align2::LEFT_TOP,
                who,
                text_font(theme::SIZE_BODY),
                theme::GOAL,
            );
            painter.text(
                Pos2::new(name.right() + theme::ROW_GAP, at.y),
                Align2::LEFT_TOP,
                words,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
        }
    }

    fn say_field(&mut self, ui: &mut egui::Ui, row: Rect, frame: &WatchFrame, tools: &Tools<'_>) {
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
        if sent && frame.human_control && !words.is_empty() {
            tools.hand.act(Act::ChatSay(words));
            self.words.clear();
            typed.request_focus();
        }
    }

    fn channel_field(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        chat: &crate::view::WatchChat,
        tools: &Tools<'_>,
    ) {
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
            WORDS_JOIN,
            if on { theme::GOAL } else { theme::TEXT_FAINT },
        );
        let asked = join || (typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
        if asked && on && !self.wish.trim().is_empty() {
            tools.hand.ask(Ask::Channel {
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
            });
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
