//! The macro editor. A macro is a script of the scripts folder. The human
//! picks one from the list, changes its lines, runs it, saves it, or puts
//! it on the hotbar. He can also record what he does as a new macro. What
//! each button does and what the editor asks are
//! `uoterm_view::ui::macros`'; this file draws it.

use super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::bridge;
use super::control::Asker;
use super::deck_ui::DeckUi;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_view::ui::macros::{
    add_line_color, note_color, wish_hint, MacroEditorPanel, HINT_LINES, HINT_NAME,
    MACROS_ADD_WIDTH as ADD_WIDTH, MACROS_FIELD_ROW as FIELD_ROW, MACROS_GAP as GAP,
    MACROS_LIST_ROW as LIST_ROW, MACROS_LIST_WIDTH as LIST_WIDTH, MACROS_SIZE,
    MACROS_TITLE_ROW as TITLE_ROW, STATUS_EVERY, WORDS_ADD_LINE, WORDS_MACROS,
};

pub struct MacrosUi {
    panel: MacroEditorPanel,
    first_name: usize,
}

impl MacrosUi {
    /// The editor as the window starts. An open one asks for the list of
    /// macros with its first picture.
    pub fn starting(open: bool) -> Self {
        Self {
            panel: MacroEditorPanel::starting(open),
            first_name: 0,
        }
    }

    /// Opens or closes the editor. When it opens it asks for the list again.
    pub fn toggle(&mut self) {
        self.panel.toggle();
    }

    pub fn is_open(&self) -> bool {
        self.panel.open
    }

    /// Draws the editor when it is open. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        deck: &mut DeckUi,
    ) -> Option<Rect> {
        let time = tools.time;
        self.panel
            .take_answers(tools.hand.new_answers(Asker::Macros), time);
        if !self.panel.open {
            return None;
        }
        for ask in self.panel.asks_due(time) {
            tools.hand.ask(Asker::Macros, ask);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(STATUS_EVERY));
        let panel = Rect::from_center_size(rect.center(), bridge::vec2(MACROS_SIZE));
        // The editor is large and lies on other panels. A solid back keeps
        // their words from showing through its glass.
        ui.painter()
            .rect_filled(panel, CornerRadius::same(theme::PANEL_RADIUS), theme::VOID);
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_MACROS,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        ui.painter().text(
            inner.right_top() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::RIGHT_TOP,
            &self.panel.status,
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let live = frame.human_control;
        let body = Rect::from_min_max(inner.left_top() + Vec2::new(0.0, TITLE_ROW), inner.max);
        self.list(ui, panel, body, tools);
        let right = Rect::from_min_max(
            Pos2::new(body.left() + LIST_WIDTH + GAP, body.top()),
            body.max,
        );
        let name_row = Rect::from_min_size(right.min, Vec2::new(right.width(), FIELD_ROW));
        let buttons_top = right.bottom() - FIELD_ROW;
        let wish_row = Rect::from_min_size(
            Pos2::new(right.left(), buttons_top - GAP - FIELD_ROW),
            Vec2::new(right.width(), FIELD_ROW),
        );
        let lines_box = Rect::from_min_max(
            Pos2::new(right.left(), name_row.bottom() + GAP),
            Pos2::new(right.right(), wish_row.top() - GAP),
        );
        let orders_on = tools.hand.orders_on;
        let (_, add) = theme::button(
            ui,
            Pos2::new(wish_row.right() - ADD_WIDTH, wish_row.top()),
            WORDS_ADD_LINE,
            bridge::color(add_line_color(orders_on)),
        );
        let wish_field = Rect::from_min_max(
            wish_row.min,
            Pos2::new(wish_row.right() - ADD_WIDTH - GAP, wish_row.bottom()),
        );
        for field in [name_row, lines_box, wish_field] {
            ui.painter()
                .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        }
        let margin = egui::Margin::symmetric(8, 6);
        ui.put(
            name_row,
            egui::TextEdit::singleline(&mut self.panel.name)
                .frame(false)
                .margin(margin)
                .hint_text(HINT_NAME)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        ui.put(
            lines_box,
            egui::TextEdit::multiline(&mut self.panel.lines)
                .frame(false)
                .margin(margin)
                .hint_text(HINT_LINES)
                .font(number_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let wish = ui.put(
            wish_field,
            egui::TextEdit::singleline(&mut self.panel.wish)
                .frame(false)
                .margin(margin)
                .hint_text(wish_hint(orders_on))
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let wished = add || (wish.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
        if let Some(ask) = wished
            .then(|| self.panel.wish_ask(orders_on, time))
            .flatten()
        {
            tools.hand.ask(Asker::Macros, ask);
        }
        self.buttons(
            ui,
            Pos2::new(right.left(), buttons_top),
            frame,
            tools,
            deck,
            live,
        );
        self.show_note(ui, panel, time);
        Some(panel)
    }

    fn list(&mut self, ui: &egui::Ui, panel: Rect, body: Rect, tools: &Tools<'_>) {
        let rows = (body.height() / LIST_ROW) as usize;
        let last_first = self.panel.names.len().saturating_sub(rows);
        self.first_name = scrolled(ui, panel, self.first_name, last_first);
        let shown = self.panel.names.iter().skip(self.first_name).take(rows);
        let mut picked = None;
        for (i, name) in shown.enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, i as f32 * LIST_ROW),
                Vec2::new(LIST_WIDTH, LIST_ROW - theme::ROW_GAP / 2.0),
            );
            let response = ui.interact(row, Id::new(("macro-name", name)), Sense::click());
            let fill = if *name == self.panel.name || response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::BUTTON
            };
            let painter = ui.painter().with_clip_rect(row);
            painter.rect_filled(row, CornerRadius::same(CELL_RADIUS), fill);
            painter.text(
                row.left_center() + Vec2::new(theme::ROW_GAP, 0.0),
                Align2::LEFT_CENTER,
                name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if response.clicked() {
                picked = Some(name.clone());
            }
        }
        if let Some(name) = picked {
            tools
                .hand
                .ask(Asker::Macros, super::control::Ask::ScriptText(name));
        }
    }

    fn buttons(
        &mut self,
        ui: &egui::Ui,
        left_top: Pos2,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        deck: &mut DeckUi,
        live: bool,
    ) {
        let mut at = left_top;
        let mut pressed = None;
        for shown in self.panel.buttons(live) {
            let (area, clicked) = theme::button(ui, at, shown.words, bridge::color(shown.color));
            at.x = area.right() + theme::ROW_GAP;
            if clicked {
                pressed = Some(shown.button);
            }
        }
        let Some(button) = pressed else {
            return;
        };
        let time = tools.time;
        let press = self.panel.press(button, live, time);
        for act in press.acts {
            tools.hand.act(act);
        }
        for ask in press.asks {
            tools.hand.ask(Asker::Macros, ask);
        }
        if let Some(line) = press.pin {
            let pinned = deck.pin_command(&frame.name, &line);
            self.panel.pinned(pinned, time);
        }
    }

    fn show_note(&self, ui: &egui::Ui, panel: Rect, time: f64) {
        let Some((words, failed)) = self.panel.note(time) else {
            return;
        };
        theme::shadowed_text(
            ui.painter(),
            Pos2::new(panel.center().x, panel.bottom() + theme::ROW_GAP),
            Align2::CENTER_TOP,
            words,
            text_font(theme::SIZE_BODY),
            bridge::color(note_color(failed)),
        );
    }
}
