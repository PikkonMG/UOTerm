//! The Options gump of the classic look, as the reference client draws it: the pages as glowing buttons at the left, the options of the page
//! in a scroll area at the right under their section titles, and Cancel,
//! Apply, Default and Okay at the bottom, with Save as default beside them.
//! Every row of the option table shows with the control its kind needs: a
//! check box, a slider, a drop-down list, a color box with the color
//! picker, a text box, or an editor for a list. The gump edits a copy of
//! the profile (`model::options_draft`, as the Modern Options panel does);
//! Apply and Okay make it the profile, and Default puts the page back to
//! its defaults.

use super::canvas::{ButtonArt, Canvas, SliderStyle, COLOR_BOX};
use super::hue_picker::HuePicker;
use super::macro_steps::{take_capture, StepList};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::window::actions::editor::{step_words, Capture, MacroEditor};
use crate::window::keys::default_keys;
use crate::window::model::options_draft::Draft;
use crate::window::options_ui::{
    format_ids, parse_ids, parse_lines, snap, NEW_INFO_BAR_DATA, WORDS_ADD_ITEM, WORDS_ADD_MACRO,
    WORDS_ADD_TAB, WORDS_CLEAR, WORDS_DEFAULT_BUTTONS, WORDS_DEFAULT_KEYS, WORDS_NEW_TAB,
    WORDS_NO_BUTTON, WORDS_NO_KEY, WORDS_PRESS_BUTTON, WORDS_PRESS_KEY, WORDS_REMOVE,
    WORDS_SAVE_DEFAULT, WORDS_STEPS,
};
use crate::window::pad::default_buttons;
use crate::window::settings::{
    rows_on, Choice, InfoBarData, InfoBarItem, JournalKind, JournalTab, KeyBinding, OptionKind,
    OptionRow, OptionValue, Page, Profile, NO_HUE,
};
use eframe::egui::{Color32, Pos2, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

pub const OPTIONS: GumpKind = GumpKind {
    id: well_known::OPTIONS,
    rules: GumpRules {
        first_place: Pos2::new(90.0, 60.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(Options::default()),
};

const WIDTH: i32 = 700;
const HEIGHT: i32 = 500;
const SHADE_HUE: u16 = 999;
const SHADE_OPACITY: f32 = 0.95;
const PAGE_X: i32 = 10;
const PAGE_Y: i32 = 10;
/// The classic gump puts a page button every 30 pixels; with the pages of
/// UOTerm that runs past its foot, so they sit closer.
const PAGE_STEP: i32 = 26;
const PAGE_WIDTH: i32 = 140;
const PAGE_HEIGHT: i32 = 25;
const RULE_COLOR: Color32 = Color32::GRAY;
const SIDE_RULE: (i32, i32, i32, i32) = (160, 5, 1, HEIGHT - 10);
const FOOT_RULE: (i32, i32, i32, i32) = (160, 441, WIDTH - 160, 1);
const CANCEL: ButtonArt = ButtonArt::new(0x00F3, 0x00F1, 0x00F2);
const APPLY: ButtonArt = ButtonArt::new(0x00EF, 0x00F0, 0x00EE);
const DEFAULT: ButtonArt = ButtonArt::new(0x00F6, 0x00F4, 0x00F5);
const OKAY: ButtonArt = ButtonArt::new(0x00F9, 0x00F8, 0x00F7);
const FOOT_Y: i32 = 465;
const CANCEL_X: i32 = 214;
const APPLY_X: i32 = 308;
const DEFAULT_X: i32 = 406;
const OKAY_X: i32 = 503;
const SAVE_DEFAULT_AT: (i32, i32, i32, i32) = (575, 462, 115, 25);
const AREA: (i32, i32, i32, i32) = (190, 20, WIDTH - 210, 420);
// The rows.
const FONT: u8 = 1;
const WHITE: u16 = 0xFFFF;
const NOTE_HUE: u16 = 0x0021;
const SECTION_RULE: Color32 = Color32::from_rgb(0xC2, 0xBD, 0xBA);
const SECTION_TITLE_X: i32 = 5;
const SECTION_RULE_ROOM: i32 = 30;
const ROW_X: i32 = 15;
const SECTION_GAP: i32 = 10;
pub(super) const ROW_GAP: i32 = 2;
const RIGHT_GAP: i32 = 15;
pub(super) const LABEL_GAP: i32 = 2;
const CHECK: (u16, u16) = (0x00D2, 0x00D3);
const CONTROL_WIDTH: i32 = 200;
const FIELD_FRAME: u16 = 0x0BB8;
pub(super) const FIELD_HEIGHT: i32 = 25;
const FIELD_PAD: i32 = 4;
const LIST_WIDTH: i32 = 400;
const LIST_HEIGHT: i32 = 80;
const COLOR_LABEL_GAP: i32 = 10;
pub(super) const COMBO_HEIGHT: i32 = 25;
pub(super) const BUTTON_HEIGHT: i32 = 22;
pub(super) const SMALL_BUTTON: i32 = 50;
const WIDE_BUTTON: i32 = 110;
const NAME_WIDTH: i32 = 120;
const INFO_LABEL_WIDTH: i32 = 120;
const JOURNAL_NAME_WIDTH: i32 = 150;
const JOURNAL_KIND_STEP: i32 = 110;
const JOURNAL_KINDS_IN_ROW: usize = 4;
const HUE_KEY_PREFIX: &str = "options";
const FIRST_PAGE: Page = Page::General;
const WORDS_OPEN_IGNORE_LIST: &str = "Ignore List";

/// Where the fields of the gump keep their words: the page, the row and a
/// place inside the row.
type FieldKey = (usize, &'static str, usize);

pub struct Options {
    page: Page,
    draft: Option<Draft>,
    fields: HashMap<FieldKey, TextField>,
    macros: MacroEditor,
    steps: StepList,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            page: FIRST_PAGE,
            draft: None,
            fields: HashMap::new(),
            macros: MacroEditor::default(),
            steps: StepList::default(),
        }
    }
}

pub(super) fn text() -> TextLook {
    TextLook::unicode(FONT, WHITE)
}

/// The key a hue picked for one row comes back under.
fn hue_key(page: Page, label: &str, place: usize) -> String {
    format!("{HUE_KEY_PREFIX}/{}/{label}/{place}", page.index())
}

/// The number of steps a slider has, and the step a value sits on.
fn slider_steps(min: f32, max: f32, step: f32) -> i32 {
    ((max - min) / step).round() as i32
}

fn slider_index(value: f32, min: f32, step: f32) -> i32 {
    ((value - min) / step).round() as i32
}

impl Options {
    /// Opens on the Macros page, for the key that opens the macros.
    pub fn at_page(page: Page) -> Self {
        Self {
            page,
            ..Self::default()
        }
    }

    /// The words of a field, made from `words` the first time.
    fn field(&mut self, key: FieldKey, words: impl FnOnce() -> String) -> &mut TextField {
        self.fields
            .entry(key)
            .or_insert_with(|| TextField::new(&words()))
    }
}

impl GumpBody for Options {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let mut draft = self.draft.take().unwrap_or_else(|| Draft::of(cx.profile));
        g.shade(1, 1, WIDTH - 2, HEIGHT - 2, SHADE_HUE, SHADE_OPACITY);
        for (index, words) in Page::LABELS.iter().enumerate() {
            let page = Page::from_index(index);
            let y = PAGE_Y + PAGE_STEP * index as i32;
            let look = text().bordered();
            if g.nice_button(
                ("page", index),
                PAGE_X,
                y,
                PAGE_WIDTH,
                PAGE_HEIGHT,
                words,
                &look,
                page == self.page,
            ) {
                self.page = page;
                self.macros.cancel_capture();
            }
        }
        let (x, y, w, h) = SIDE_RULE;
        g.fill(x, y, w, h, RULE_COLOR);
        let (x, y, w, h) = FOOT_RULE;
        g.fill(x, y, w, h, RULE_COLOR);
        let (x, y, w, h) = AREA;
        g.scroll_area(("rows", self.page.index()), x, y, w, h, |g| {
            self.page_rows(g, cx, &mut draft.edited, w)
        });
        let cancel = g.button("cancel", CANCEL_X, FOOT_Y, CANCEL);
        let applied = g.button("apply", APPLY_X, FOOT_Y, APPLY);
        let reset = g.button("default", DEFAULT_X, FOOT_Y, DEFAULT);
        let okay = g.button("okay", OKAY_X, FOOT_Y, OKAY);
        let (x, y, w, h) = SAVE_DEFAULT_AT;
        let save_default = g.nice_button(
            "save-default",
            x,
            y,
            w,
            h,
            WORDS_SAVE_DEFAULT,
            &text().bordered(),
            false,
        );
        if reset {
            draft.reset_page(self.page);
            let page = self.page.index();
            self.fields.retain(|(on, _, _), _| *on != page);
        }
        if applied || okay || save_default {
            draft.apply(cx.profile);
            cx.profile_changed();
        }
        if save_default {
            cx.save_as_default();
        }
        if cancel || okay {
            cx.close(cx.me);
        }
        self.draft = Some(draft);
    }

    fn wants_keys(&self) -> bool {
        self.macros.capture.is_some()
    }
}

impl Options {
    /// The rows of the page, under their section titles. Gives the height
    /// they take.
    fn page_rows(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        draft: &mut Profile,
        width: i32,
    ) -> i32 {
        let mut y = 0;
        if self.page == Page::Sound && !cx.sound_note.is_empty() {
            y += g
                .label(
                    SECTION_TITLE_X,
                    y,
                    cx.sound_note,
                    &TextLook::unicode(FONT, NOTE_HUE),
                )
                .y as i32
                + SECTION_GAP;
        }
        if self.page == Page::IgnoreList && cx.has_kind(well_known::IGNORE_LIST) {
            let look = text().bordered();
            if g.nice_button(
                "open-ignore-list",
                SECTION_TITLE_X,
                y,
                WIDE_BUTTON,
                BUTTON_HEIGHT,
                WORDS_OPEN_IGNORE_LIST,
                &look,
                false,
            ) {
                cx.open(GumpId::one(well_known::IGNORE_LIST));
            }
            y += BUTTON_HEIGHT + SECTION_GAP;
        }
        let mut section = "";
        for row in rows_on(self.page) {
            if row.section != section {
                if !section.is_empty() {
                    y += SECTION_GAP;
                }
                section = row.section;
                let title = g.label(SECTION_TITLE_X, y, section, &text());
                y += title.y as i32;
                g.fill(0, y, width - SECTION_RULE_ROOM, 1, SECTION_RULE);
                y += FIELD_PAD;
            }
            let value = (row.get)(draft);
            let (height, changed) = self.row(g, cx, row, value, y);
            if let Some(value) = changed {
                (row.set)(draft, value);
            }
            y += height + ROW_GAP;
        }
        y
    }

    /// Draws one row. Gives its height, and its new value when the player
    /// changed it.
    fn row(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        row: &'static OptionRow,
        value: OptionValue,
        y: i32,
    ) -> (i32, Option<OptionValue>) {
        let look = text();
        let page = self.page.index();
        let key = |place: usize| (page, row.label, place);
        match (row.kind, value) {
            (OptionKind::Toggle, OptionValue::Toggle(mut on)) => {
                let changed =
                    g.checkbox(key(0), ROW_X, y, CHECK, &mut on, Some((row.label, &look)));
                let height = g
                    .measure(row.label, &look)
                    .y
                    .max(g.gump_size(CHECK.0).map_or(0.0, |s| s.y));
                (height as i32, changed.then_some(OptionValue::Toggle(on)))
            }
            (
                OptionKind::Slider {
                    min,
                    max,
                    step,
                    unit,
                },
                OptionValue::Number(number),
            ) => {
                let label = g.label(ROW_X, y, row.label, &look);
                let x = ROW_X + label.x as i32 + RIGHT_GAP;
                let mut index = slider_index(number, min, step);
                let moved = g.slider(
                    key(0),
                    x,
                    y,
                    CONTROL_WIDTH,
                    (0, slider_steps(min, max, step)),
                    &mut index,
                    SliderStyle::Recessed,
                );
                let shown = snap(min + index as f32 * step, min, max, step);
                g.slider_words(x, y, CONTROL_WIDTH, &unit.format(shown), &look);
                let changed = moved && shown != number;
                (
                    label.y as i32,
                    changed.then_some(OptionValue::Number(shown)),
                )
            }
            (OptionKind::Choice { labels }, OptionValue::Choice(mut index)) => {
                let label = g.label(ROW_X, y, row.label, &look);
                let x = ROW_X + label.x as i32 + RIGHT_GAP;
                let changed = g.combobox(key(0), x, y, CONTROL_WIDTH, labels, &mut index);
                (COMBO_HEIGHT, changed.then_some(OptionValue::Choice(index)))
            }
            (OptionKind::Hue, OptionValue::Hue(hue)) => {
                let (height, picked) = self.hue_row(g, cx, row.label, 0, ROW_X, y, hue);
                (height, picked.map(OptionValue::Hue))
            }
            (OptionKind::Text, OptionValue::Text(words)) => {
                let label = g.label(ROW_X, y, row.label, &look);
                let x = ROW_X + label.x as i32 + LABEL_GAP;
                let field = self.field(key(0), || words.clone());
                let changed = input(g, key(0), x, y, CONTROL_WIDTH, FIELD_HEIGHT, field);
                (
                    FIELD_HEIGHT,
                    changed.then(|| OptionValue::Text(field.text().to_string())),
                )
            }
            (OptionKind::FilePath, OptionValue::FilePath(path)) => {
                let label = g.label(ROW_X, y, row.label, &look);
                let x = ROW_X + label.x as i32 + LABEL_GAP;
                let shown = path.map(|p| p.display().to_string()).unwrap_or_default();
                let field = self.field(key(0), || shown);
                let changed = input(g, key(0), x, y, CONTROL_WIDTH, FIELD_HEIGHT, field);
                let words = field.text();
                (
                    FIELD_HEIGHT,
                    changed.then(|| {
                        OptionValue::FilePath((!words.is_empty()).then(|| PathBuf::from(words)))
                    }),
                )
            }
            (OptionKind::TextList, OptionValue::TextList(lines)) => {
                let label = g.label(ROW_X, y, row.label, &look).y as i32;
                let field = self.list_field(key(0), || lines.join("\n"));
                let changed = input(g, key(0), ROW_X, y + label, LIST_WIDTH, LIST_HEIGHT, field);
                (
                    label + LIST_HEIGHT,
                    changed.then(|| OptionValue::TextList(parse_lines(field.text()))),
                )
            }
            (OptionKind::IdList, OptionValue::Ids(ids)) => {
                let label = g.label(ROW_X, y, row.label, &look).y as i32;
                let field = self.list_field(key(0), || format_ids(&ids));
                let changed = input(g, key(0), ROW_X, y + label, LIST_WIDTH, LIST_HEIGHT, field);
                (
                    label + LIST_HEIGHT,
                    changed.then(|| OptionValue::Ids(parse_ids(field.text()))),
                )
            }
            (OptionKind::KeyList, OptionValue::Keys(keys)) => {
                let (height, changed) = self.macro_list(g, keys, y);
                (height, changed.map(OptionValue::Keys))
            }
            (OptionKind::InfoBarItems, OptionValue::InfoBarItems(items)) => {
                let (height, changed) = self.info_bar_items(g, cx, row.label, items, y);
                (height, changed.map(OptionValue::InfoBarItems))
            }
            (OptionKind::JournalTabs, OptionValue::JournalTabs(tabs)) => {
                let (height, changed) = self.journal_tabs(g, row.label, tabs, y);
                (height, changed.map(OptionValue::JournalTabs))
            }
            (kind, value) => {
                tracing::warn!(label = row.label, ?kind, ?value, "option row of two forms");
                (0, None)
            }
        }
    }

    /// A field of many lines.
    fn list_field(&mut self, key: FieldKey, words: impl FnOnce() -> String) -> &mut TextField {
        let field = self.field(key, words);
        field.multiline = true;
        field
    }

    /// A color box with its words at its right. A click opens the color
    /// picker; the hue it picks comes back here. Gives the height and the
    /// hue picked.
    #[allow(clippy::too_many_arguments)]
    fn hue_row(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        label: &'static str,
        place: usize,
        x: i32,
        y: i32,
        hue: u16,
    ) -> (i32, Option<u16>) {
        let key = hue_key(self.page, label, place);
        if g.color_box(("hue", label, place), x, y, hue) {
            let picker = GumpId::one(well_known::HUE_PICKER);
            cx.close(picker);
            cx.open_with(picker, Box::new(HuePicker::new(key.clone(), hue)));
        }
        let size = g.gump_size(COLOR_BOX).unwrap_or(Vec2::ZERO);
        let words = g.label(x + size.x as i32 + COLOR_LABEL_GAP, y, label, &text());
        let picked = cx.take_answer(&key).filter(|picked| *picked != hue);
        (size.y.max(words.y) as i32, picked)
    }

    /// The macros of the Macros page: each with its name, its key, its
    /// controller buttons and, when opened, its steps; then the default
    /// keys and buttons in words.
    fn macro_list(
        &mut self,
        g: &mut Canvas<'_>,
        mut keys: Vec<KeyBinding>,
        top: i32,
    ) -> (i32, Option<Vec<KeyBinding>>) {
        let mut changed = take_capture(&mut self.macros, g, &mut keys);
        let mut y = top;
        let mut remove = None;
        let look = text().bordered();
        for at in 0..keys.len() {
            let page = self.page.index();
            let field = self.field((page, "macro-name", at), || keys[at].name.clone());
            if input(
                g,
                ("macro-name", at),
                ROW_X,
                y,
                NAME_WIDTH,
                FIELD_HEIGHT,
                field,
            ) {
                keys[at].name = field.text().to_string();
                changed = true;
            }
            let mut x = ROW_X + NAME_WIDTH + LABEL_GAP;
            let chord = match (self.macros.capture, &keys[at].chord) {
                (Some(Capture::Chord(waiting)), _) if waiting == at => WORDS_PRESS_KEY.to_string(),
                (_, Some(chord)) => chord.to_string(),
                (_, None) => WORDS_NO_KEY.to_string(),
            };
            if g.nice_button(
                ("chord", at),
                x,
                y,
                WIDE_BUTTON,
                BUTTON_HEIGHT,
                &chord,
                &look,
                false,
            ) {
                self.macros.capture(Capture::Chord(at));
            }
            x += WIDE_BUTTON + LABEL_GAP;
            let pad = match (self.macros.capture, &keys[at].pad) {
                (Some(Capture::Pad(waiting)), _) if waiting == at => WORDS_PRESS_BUTTON.to_string(),
                (_, Some(pad)) => pad.to_string(),
                (_, None) => WORDS_NO_BUTTON.to_string(),
            };
            if g.nice_button(
                ("pad", at),
                x,
                y,
                WIDE_BUTTON,
                BUTTON_HEIGHT,
                &pad,
                &look,
                false,
            ) {
                self.macros.capture(Capture::Pad(at));
            }
            x += WIDE_BUTTON + LABEL_GAP;
            let bound = keys[at].chord.is_some() || keys[at].pad.is_some();
            if bound
                && g.nice_button(
                    ("clear", at),
                    x,
                    y,
                    SMALL_BUTTON,
                    BUTTON_HEIGHT,
                    WORDS_CLEAR,
                    &look,
                    false,
                )
            {
                keys[at].chord = None;
                keys[at].pad = None;
                changed = true;
            }
            x += SMALL_BUTTON + LABEL_GAP;
            let open = self.macros.open == Some(at);
            if g.nice_button(
                ("steps", at),
                x,
                y,
                SMALL_BUTTON,
                BUTTON_HEIGHT,
                WORDS_STEPS,
                &look,
                open,
            ) {
                self.macros.open = (!open).then_some(at);
            }
            x += SMALL_BUTTON + LABEL_GAP;
            if g.nice_button(
                ("remove", at),
                x,
                y,
                SMALL_BUTTON,
                BUTTON_HEIGHT,
                WORDS_REMOVE,
                &look,
                false,
            ) {
                remove = Some(at);
            }
            y += FIELD_HEIGHT + ROW_GAP;
            if open {
                let (height, stepped) = self.steps.draw(g, &mut keys, at, (ROW_X, y));
                y += height;
                changed |= stepped;
            }
        }
        if let Some(at) = remove {
            self.macros.remove(&mut keys, at);
            self.steps.forget_all();
            let page = self.page.index();
            self.fields
                .retain(|(on, row, _), _| *on != page || *row != "macro-name");
            changed = true;
        }
        if g.nice_button(
            "add-macro",
            ROW_X,
            y,
            WIDE_BUTTON,
            BUTTON_HEIGHT,
            WORDS_ADD_MACRO,
            &look,
            false,
        ) {
            self.macros.add(&mut keys);
            changed = true;
        }
        y += BUTTON_HEIGHT + SECTION_GAP;
        for (title, list) in [
            (WORDS_DEFAULT_KEYS, default_keys().collect::<Vec<_>>()),
            (WORDS_DEFAULT_BUTTONS, default_buttons().collect::<Vec<_>>()),
        ] {
            y += g.label(ROW_X, y, title, &text()).y as i32 + ROW_GAP;
            for (trigger, action, argument) in list {
                let words = format!("{trigger}: {}", step_words(action, argument));
                y += g.label(ROW_X, y, &words, &text()).y as i32;
            }
            y += SECTION_GAP;
        }
        (y - top, changed.then_some(keys))
    }

    /// The items of the info bar: each with its words, its hue and what it
    /// shows, and Add item under them.
    fn info_bar_items(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        label: &'static str,
        mut items: Vec<InfoBarItem>,
        top: i32,
    ) -> (i32, Option<Vec<InfoBarItem>>) {
        let look = text().bordered();
        let mut y = top + g.label(ROW_X, top, label, &text()).y as i32 + ROW_GAP;
        let mut changed = false;
        let mut remove = None;
        for (at, item) in items.iter_mut().enumerate() {
            let mut x = ROW_X;
            let key = (self.page.index(), label, at);
            let field = self.field(key, || item.label.clone());
            if input(g, key, x, y, INFO_LABEL_WIDTH, FIELD_HEIGHT, field) {
                item.label = field.text().to_string();
                changed = true;
            }
            x += INFO_LABEL_WIDTH + LABEL_GAP;
            let (_, picked) = self.hue_row(g, cx, label, at, x, y, item.hue);
            if let Some(hue) = picked {
                item.hue = hue;
                changed = true;
            }
            x += CONTROL_WIDTH / 2;
            let mut data = item.data.index();
            if g.combobox(
                ("info-data", at),
                x,
                y,
                CONTROL_WIDTH,
                InfoBarData::LABELS,
                &mut data,
            ) {
                item.data = InfoBarData::from_index(data);
                changed = true;
            }
            x += CONTROL_WIDTH + LABEL_GAP;
            if g.nice_button(
                ("info-remove", at),
                x,
                y,
                SMALL_BUTTON,
                BUTTON_HEIGHT,
                WORDS_REMOVE,
                &look,
                false,
            ) {
                remove = Some(at);
            }
            y += FIELD_HEIGHT + ROW_GAP;
        }
        if let Some(at) = remove {
            items.remove(at);
            self.forget_row(label);
            changed = true;
        }
        if g.nice_button(
            "info-add",
            ROW_X,
            y,
            WIDE_BUTTON,
            BUTTON_HEIGHT,
            WORDS_ADD_ITEM,
            &look,
            false,
        ) {
            items.push(InfoBarItem {
                label: String::new(),
                hue: NO_HUE,
                data: NEW_INFO_BAR_DATA,
            });
            changed = true;
        }
        y += BUTTON_HEIGHT;
        (y - top, changed.then_some(items))
    }

    /// The tabs of the journal: each with its name and the kinds of lines it
    /// shows, and Add tab under them.
    fn journal_tabs(
        &mut self,
        g: &mut Canvas<'_>,
        label: &'static str,
        mut tabs: Vec<JournalTab>,
        top: i32,
    ) -> (i32, Option<Vec<JournalTab>>) {
        let look = text().bordered();
        let mut y = top + g.label(ROW_X, top, label, &text()).y as i32 + ROW_GAP;
        let mut changed = false;
        let mut remove = None;
        let check_height = g.gump_size(CHECK.0).map_or(0, |size| size.y as i32);
        for (at, tab) in tabs.iter_mut().enumerate() {
            let key = (self.page.index(), label, at);
            let field = self.field(key, || tab.name.clone());
            if input(g, key, ROW_X, y, JOURNAL_NAME_WIDTH, FIELD_HEIGHT, field) {
                tab.name = field.text().to_string();
                changed = true;
            }
            let remove_x = ROW_X + JOURNAL_NAME_WIDTH + LABEL_GAP;
            if g.nice_button(
                ("tab-remove", at),
                remove_x,
                y,
                SMALL_BUTTON,
                BUTTON_HEIGHT,
                WORDS_REMOVE,
                &look,
                false,
            ) {
                remove = Some(at);
            }
            y += FIELD_HEIGHT + ROW_GAP;
            for (index, words) in JournalKind::LABELS.iter().enumerate() {
                let kind = JournalKind::from_index(index);
                let column = (index % JOURNAL_KINDS_IN_ROW) as i32;
                let x = ROW_X + column * JOURNAL_KIND_STEP;
                let mut shown = tab.kinds.contains(&kind);
                if g.checkbox(
                    ("tab-kind", at, index),
                    x,
                    y,
                    CHECK,
                    &mut shown,
                    Some((*words, &text())),
                ) {
                    if shown {
                        tab.kinds.push(kind);
                    } else {
                        tab.kinds.retain(|other| *other != kind);
                    }
                    changed = true;
                }
                if column as usize == JOURNAL_KINDS_IN_ROW - 1
                    || index + 1 == JournalKind::LABELS.len()
                {
                    y += check_height + ROW_GAP;
                }
            }
        }
        if let Some(at) = remove {
            tabs.remove(at);
            self.forget_row(label);
            changed = true;
        }
        if g.nice_button(
            "tab-add",
            ROW_X,
            y,
            WIDE_BUTTON,
            BUTTON_HEIGHT,
            WORDS_ADD_TAB,
            &look,
            false,
        ) {
            tabs.push(JournalTab {
                name: WORDS_NEW_TAB.to_string(),
                kinds: Vec::new(),
            });
            changed = true;
        }
        y += BUTTON_HEIGHT;
        (y - top, changed.then_some(tabs))
    }

    /// Forgets the fields of a list row whose entries moved.
    fn forget_row(&mut self, label: &str) {
        let page = self.page.index();
        self.fields
            .retain(|(on, row, _), _| *on != page || *row != label);
    }
}

/// A text box in a stone frame, as the classic Options gump draws its
/// fields. True when the words changed.
pub(super) fn input(
    g: &mut Canvas<'_>,
    key: impl std::hash::Hash,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    field: &mut TextField,
) -> bool {
    g.frame(x, y, w, h, FIELD_FRAME);
    let look = text().bordered();
    g.text_box(
        key,
        x + FIELD_PAD,
        y + FIELD_PAD,
        w - FIELD_PAD * 2,
        h - FIELD_PAD * 2,
        field,
        &look,
    )
    .changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::settings::DEFAULT_MASTER_VOLUME;

    #[test]
    fn a_slider_counts_its_steps_and_finds_its_value() {
        assert_eq!(slider_steps(0.0, 1.0, 0.01), 100);
        assert_eq!(slider_index(0.8, 0.0, 0.01), 80);
        assert_eq!(slider_index(DEFAULT_MASTER_VOLUME, 0.0, 0.01), 80);
        assert_eq!(slider_steps(12.0, 250.0, 1.0), 238);
    }

    #[test]
    fn every_page_draws_and_changes_nothing_by_itself() {
        use crate::view::WatchFrame;
        use crate::window::classic::manager::GumpManager;
        use crate::window::classic::testing::draw_frames;
        for index in 0..Page::LABELS.len() {
            let page = Page::from_index(index);
            let mut profile = Profile::default();
            let mut manager = GumpManager::default();
            let options = Box::new(Options::at_page(page));
            manager.open_body(GumpId::one(well_known::OPTIONS), options, &mut profile);
            let opened = profile.clone();
            if !draw_frames(&mut manager, &mut profile, &WatchFrame::default()) {
                return;
            }
            for row in rows_on(page) {
                assert_eq!((row.get)(&profile), (row.get)(&opened), "{}", row.label);
            }
        }
    }

    #[test]
    fn each_row_has_its_own_hue_key() {
        assert_ne!(
            hue_key(Page::Speech, "Speech", 0),
            hue_key(Page::Speech, "Emote", 0)
        );
        assert_ne!(
            hue_key(Page::InfoBar, "Items", 0),
            hue_key(Page::InfoBar, "Items", 1)
        );
    }
}
