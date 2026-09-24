//! The Options screen of the Modern style. It draws every row of the option
//! table, page by page, with egui controls on a glass panel the player
//! moves, sizes and locks. As the classic Options gump does, it edits a
//! copy of the profile: Apply and Okay make it the profile, Cancel lets it
//! go, Default puts the page back to its defaults, and "Save as default"
//! makes it the start of each new character. A hue row opens the color
//! picker: the grid of hues with its shade slider and the eyedropper.

use super::actions::editor::{step_words, Capture, MacroEditor, Move};
use super::actions::{ActionId, Group, ACTIONS};
use super::audio::Audio;
use super::boxes_ui::Tools;
use super::keys::default_keys;
use super::model::highlight;
use super::model::hue_grid::HuePick;
use super::model::options_draft::Draft;
use super::model::places;
use super::modern::frame::{self as panel_frame, FrameEvent, PanelSpec};
use super::modern::hue_ui::{self, HueGridUi};
use super::modern::layout::{self, Spot};
use super::pad::{default_buttons, pressed_this_frame};
use super::settings::{
    rows_on, Choice, CooldownRule, CooldownSource, CounterItem, HighlightRule, InfoBarData,
    InfoBarItem, JournalKind, JournalTab, KeyBinding, KeyChord, MacroStep, OptionKind, OptionRow,
    OptionValue, Page, Profile, PropertyNeed, DEFAULT_COOLDOWN_SECONDS, NO_HUE,
};
use super::theme::{self, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Event, Id, Key, Pos2, Rect, RichText, Sense, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

const OPTIONS_ID: &str = "modern:options";
const PICKER_ID: &str = "modern:color_picker";
const PANEL_SIZE: Vec2 = Vec2::new(800.0, 600.0);
const PANEL_LEAST: Vec2 = Vec2::new(560.0, 360.0);
const PAGE_LIST_WIDTH: f32 = 170.0;
const PAGE_ROW: f32 = 24.0;
const COLUMN_GAP: f32 = 16.0;
const CONTROL_WIDTH: f32 = 240.0;
const NUMBER_WIDTH: f32 = 72.0;
const CHORD_WIDTH: f32 = 140.0;
const WORDS_WIDTH: f32 = 150.0;
const HUE_WIDTH: f32 = 72.0;
const SWATCH: Vec2 = Vec2::new(22.0, 18.0);
const PICKED_SWATCH: Vec2 = Vec2::new(64.0, 40.0);
const FOOT_ROW: f32 = 30.0;
const FOOT_BUTTON_WIDTH: f32 = 84.0;
const SAVE_DEFAULT_WIDTH: f32 = 130.0;
const SECTION_GAP: f32 = 10.0;
const LIST_LINES: usize = 4;
const HUE_DIGITS: usize = 4;
const HEX_PREFIX: &str = "0x";
const HEX_RADIX: u32 = 16;
const DECIMAL_BASE: f32 = 10.0;
const ID_SEPARATORS: [char; 4] = [',', ' ', '\n', ';'];
const ID_JOIN: &str = ", ";
const LINE_BREAK: &str = "\n";

const WORDS_TITLE: &str = "Options";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_APPLY: &str = "Apply";
const WORDS_DEFAULT: &str = "Default";
const WORDS_OKAY: &str = "Okay";
const WORDS_COLOR: &str = "Color";
const HINT_DEFAULT: &str = "Puts this page back to its defaults.";
const HINT_SWATCH: &str = "Pick the color.";
pub(super) const WORDS_SAVE_DEFAULT: &str = "Save as default";
pub(super) const WORDS_PRESS_KEY: &str = "Press a key";
pub(super) const WORDS_PRESS_BUTTON: &str = "Press a button";
pub(super) const WORDS_NO_KEY: &str = "No key";
pub(super) const WORDS_NO_BUTTON: &str = "No button";
pub(super) const WORDS_CLEAR: &str = "Clear";
pub(super) const WORDS_STEPS: &str = "Steps";
pub(super) const WORDS_ADD_MACRO: &str = "Add macro";
pub(super) const WORDS_ADD_STEP: &str = "Add step";
pub(super) const WORDS_UP: &str = "Up";
pub(super) const WORDS_DOWN: &str = "Down";
pub(super) const WORDS_DEFAULT_KEYS: &str = "Default keys (the Experimental page turns them off)";
pub(super) const WORDS_DEFAULT_BUTTONS: &str = "Default controller buttons";
const WORDS_SUGGESTIONS: &str = "Pick";
pub(super) const WORDS_ADD_ITEM: &str = "Add item";
pub(super) const WORDS_ADD_TAB: &str = "Add tab";
pub(super) const WORDS_REMOVE: &str = "Remove";
pub(super) const WORDS_NEW_TAB: &str = "New tab";
const HINT_MACRO_NAME: &str = "macro name";
const NAME_WIDTH: f32 = 140.0;
const ACTION_WIDTH: f32 = 200.0;
const STEP_NUMBER_WIDTH: f32 = 20.0;
const HINT_LABEL: &str = "label";
const HINT_TAB_NAME: &str = "tab name";
const HINT_NO_FILE: &str = "no file";
const HINT_IDS: &str = "0x0123, 0x0456";
const HINT_LINES: &str = "one on each line";
const FIRST_PAGE: Page = Page::General;
const WORDS_ADD_COOLDOWN: &str = "Add cooldown bar";
const WORDS_ADD_RULE: &str = "Add rule";
const WORDS_ADD_NEED: &str = "Add property";
const WORDS_ADD_PRESET: &str = "Add";
const WORDS_RESTART: &str = "Start again when it comes again";
const WORDS_NEED_ALL: &str = "Needs every property";
const WORDS_CORPSES_ONLY: &str = "Corpses only";
const WORDS_AT_LEAST: &str = "at least";
const HINT_TRIGGER: &str = "words in the journal";
const HINT_RULE_NAME: &str = "rule name";
const HINT_PROPERTY: &str = "property words";
const SECONDS_SUFFIX: &str = " s";
const COOLDOWN_SECONDS_MIN: f32 = 0.1;
const COOLDOWN_SECONDS_MAX: f32 = 3600.0;
const COOLDOWN_SECONDS_STEP: f64 = 0.1;
pub(super) const NEW_INFO_BAR_DATA: InfoBarData = InfoBarData::HitPoints;

/// What the player pressed while a binding waited for a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Pressed {
    Cancel,
    Chord(KeyChord),
}

pub struct OptionsUi {
    open: bool,
    page: Page,
    /// The copy of the profile the player edits, until Apply.
    draft: Option<Draft>,
    /// The macro editor of the Macros page.
    macros: MacroEditor,
    /// The words the player types in a list field, kept while the field has
    /// the keys, so a line he has only begun stays as he typed it.
    drafts: HashMap<(Page, &'static str), String>,
    hues: HueRows,
}

impl Default for OptionsUi {
    fn default() -> Self {
        Self {
            open: false,
            page: FIRST_PAGE,
            draft: None,
            macros: MacroEditor::default(),
            drafts: HashMap::new(),
            hues: HueRows::default(),
        }
    }
}

/// Where a hue row sits: the page, the row and a place inside the row.
type HueKey = (Page, &'static str, usize);

/// The color picker a hue row opened, for that row.
struct HueChoice {
    key: HueKey,
    pick: HuePick,
    grid: HueGridUi,
}

/// The hue rows of the pages, the color picker one of them opened, and the
/// hue it picked, until its row takes it.
#[derive(Default)]
struct HueRows {
    open: Option<HueChoice>,
    picked: Option<(HueKey, u16)>,
}

impl HueRows {
    /// A hue: its number, and a swatch that opens the color picker. True
    /// when it changed.
    fn row(&mut self, ui: &mut egui::Ui, key: HueKey, hue: &mut u16, tools: &Tools<'_>) -> bool {
        let mut changed = hue_box(ui, hue);
        let (area, response) = ui.allocate_exact_size(SWATCH, Sense::click());
        hue_ui::swatch(ui, area, *hue, tools);
        if response.hovered() {
            super::tips::label(ui, HINT_SWATCH, "");
        }
        if response.clicked() {
            self.open = Some(HueChoice {
                key,
                pick: HuePick::of(*hue),
                grid: HueGridUi::default(),
            });
        }
        if let Some((_, picked)) = self.picked.take_if(|(at, _)| *at == key) {
            changed |= *hue != picked;
            *hue = picked;
        }
        changed
    }

    /// The color picker, when a row opened it. Gives its place.
    fn picker(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let choice = self.open.as_mut()?;
        choice.grid.take_picked(&mut choice.pick, frame, tools);
        let size = hue_ui::picker_size()
            + Vec2::new(
                theme::ROW_GAP * 2.0 + PICKED_SWATCH.x,
                panel_frame::TITLE_ROW + FOOT_ROW + theme::ROW_GAP,
            )
            + Vec2::splat(theme::PANEL_PAD * 2.0);
        let spec = PanelSpec {
            id: PICKER_ID,
            title: WORDS_COLOR,
            default: layout::first_place(rect, Spot::Middle(0), size),
            min_size: None,
            closable: true,
        };
        let panel = panel_frame::place(rect, &spec, profile);
        let body = panel_frame::draw(ui.painter(), panel, WORDS_COLOR);
        let grid = choice
            .grid
            .draw(ui, body.min, PICKER_ID, &mut choice.pick, tools, true);
        let shown = Rect::from_min_size(
            Pos2::new(grid.right() + theme::ROW_GAP * 2.0, grid.top()),
            PICKED_SWATCH,
        );
        hue_ui::swatch(ui, shown, choice.pick.hue(), tools);
        ui.painter().text(
            shown.center_bottom() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::CENTER_TOP,
            format!("{HEX_PREFIX}{:04X}", choice.pick.hue()),
            text_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let foot = Pos2::new(body.left(), grid.bottom() + theme::ROW_GAP);
        let dropper = choice.grid.eyedropper(ui, foot, PICKER_ID, frame, tools);
        let button = |at: f32| {
            Rect::from_min_size(
                Pos2::new(at, foot.y),
                Vec2::new(FOOT_BUTTON_WIDTH, hue_ui::EYEDROPPER_SIZE.y),
            )
        };
        let okay_area = button(dropper.right() + theme::ROW_GAP);
        let cancel_area = button(okay_area.right() + theme::ROW_GAP);
        let okay = theme::segment_keyed(
            ui,
            okay_area,
            Id::new("picker-okay"),
            WORDS_OKAY,
            theme::GOAL,
        );
        let cancel = theme::segment_keyed(
            ui,
            cancel_area,
            Id::new("picker-cancel"),
            WORDS_CANCEL,
            theme::TEXT,
        );
        let closed =
            panel_frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if okay {
            self.picked = Some((choice.key, choice.pick.hue()));
        }
        if okay || cancel || closed {
            choice.grid.stop();
            self.open = None;
        }
        Some(panel)
    }
}

/// The numbers of a list field: hex with `0x`, or decimal. Words that are
/// not numbers are left out.
pub(super) fn parse_ids(words: &str) -> Vec<u16> {
    words
        .split(ID_SEPARATORS)
        .map(str::trim)
        .filter_map(|word| match word.strip_prefix(HEX_PREFIX) {
            Some(hex) => u16::from_str_radix(hex, HEX_RADIX).ok(),
            None => word.parse().ok(),
        })
        .collect()
}

pub(super) fn format_ids(ids: &[u16]) -> String {
    ids.iter()
        .map(|id| format!("{HEX_PREFIX}{id:04X}"))
        .collect::<Vec<_>>()
        .join(ID_JOIN)
}

/// The lines of a list field, with no empty ones.
pub(super) fn parse_lines(words: &str) -> Vec<String> {
    words
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// The key the player pressed this frame, taken from the input so no other
/// part of the window acts on it. Escape cancels.
pub(super) fn take_pressed(ui: &egui::Ui) -> Option<Pressed> {
    ui.input_mut(|input| {
        let (key, modifiers) = input.events.iter().find_map(|event| match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => Some((*key, *modifiers)),
            _ => None,
        })?;
        input.consume_key(modifiers, key);
        Some(if key == Key::Escape {
            Pressed::Cancel
        } else {
            Pressed::Chord(KeyChord::from_egui(key, modifiers))
        })
    })
}

/// The slider value nearest `value` that is a whole number of steps from
/// `min`, rounded to the decimals of the step, so 0.81 is kept as 0.81.
pub(super) fn snap(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let stepped = ((value - min) / step).round() * step + min;
    let scale = DECIMAL_BASE.powf((-step.log10().floor()).max(0.0));
    ((stepped * scale).round() / scale).clamp(min, max)
}

/// A drop-down list of choices. Gives the index chosen.
fn choice_box(ui: &mut egui::Ui, id: Id, labels: &[&str], index: usize) -> usize {
    let mut chosen = index;
    egui::ComboBox::from_id_salt(id)
        .width(CONTROL_WIDTH)
        .selected_text(labels.get(index).copied().unwrap_or_default())
        .show_ui(ui, |ui| {
            for (at, words) in labels.iter().enumerate() {
                ui.selectable_value(&mut chosen, at, *words);
            }
        });
    chosen
}

/// A hue number, in hex. True when the player changed it.
fn hue_box(ui: &mut egui::Ui, hue: &mut u16) -> bool {
    ui.add_sized(
        [HUE_WIDTH, ui.spacing().interact_size.y],
        egui::DragValue::new(hue)
            .range(0..=u16::MAX)
            .hexadecimal(HUE_DIGITS, false, true)
            .prefix(HEX_PREFIX),
    )
    .changed()
}

/// Draws each entry of a list in a frame with a Remove button, and an Add
/// button under them. `add` gives the new entry, or None when it comes
/// later. True when the list changed.
fn edit_list<T>(
    ui: &mut egui::Ui,
    items: &mut Vec<T>,
    add_words: &str,
    add: impl FnOnce() -> Option<T>,
    mut entry: impl FnMut(&mut egui::Ui, usize, &mut T) -> bool,
) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (at, item) in items.iter_mut().enumerate() {
        ui.group(|ui| {
            changed |= entry(ui, at, item);
            if ui.button(WORDS_REMOVE).clicked() {
                remove = Some(at);
            }
        });
    }
    if let Some(at) = remove {
        items.remove(at);
        changed = true;
    }
    if ui.button(add_words).clicked() {
        if let Some(item) = add() {
            items.push(item);
            changed = true;
        }
    }
    changed
}

/// The buttons at the foot of the Options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Foot {
    Cancel,
    Apply,
    Default,
    Okay,
    SaveAsDefault,
}

const FOOT: [(Foot, &str, f32); 5] = [
    (Foot::Cancel, WORDS_CANCEL, FOOT_BUTTON_WIDTH),
    (Foot::Apply, WORDS_APPLY, FOOT_BUTTON_WIDTH),
    (Foot::Default, WORDS_DEFAULT, FOOT_BUTTON_WIDTH),
    (Foot::Okay, WORDS_OKAY, FOOT_BUTTON_WIDTH),
    (Foot::SaveAsDefault, WORDS_SAVE_DEFAULT, SAVE_DEFAULT_WIDTH),
];

impl OptionsUi {
    /// Opens the panel, or closes it. The button is in the control bar.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Draws the panel when it is open, and the color picker a hue row
    /// opened. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        audio: &Audio,
    ) -> Vec<Rect> {
        if !self.open {
            self.draft = None;
            self.hues = HueRows::default();
            return Vec::new();
        }
        let mut draft = self.draft.take().unwrap_or_else(|| Draft::of(profile));
        let spec = PanelSpec {
            id: OPTIONS_ID,
            title: WORDS_TITLE,
            default: layout::first_place(rect, Spot::Middle(0), PANEL_SIZE),
            min_size: Some(PANEL_LEAST),
            closable: true,
        };
        let panel = panel_frame::place(rect, &spec, profile);
        let inner = panel_frame::draw(ui.painter(), panel, WORDS_TITLE);
        self.page_list(ui, inner.left_top());
        let foot_top = inner.bottom() - FOOT_ROW;
        let rows_left = inner.left() + PAGE_LIST_WIDTH + COLUMN_GAP;
        let pressed = foot_buttons(ui, Pos2::new(rows_left, foot_top), draft.changed());
        let rows_area = Rect::from_min_max(
            Pos2::new(rows_left, inner.top()),
            Pos2::new(inner.right(), foot_top - theme::ROW_GAP),
        );
        let mut rows_ui = ui.new_child(egui::UiBuilder::new().max_rect(rows_area));
        egui::ScrollArea::vertical()
            .id_salt(("options-rows", self.page.index()))
            .auto_shrink(false)
            .show(&mut rows_ui, |ui| {
                self.page_rows(ui, &mut draft.edited, audio, tools);
            });
        let closed =
            panel_frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if pressed == Some(Foot::Default) {
            draft.reset_page(self.page);
            let page = self.page;
            self.drafts.retain(|(on, _), _| *on != page);
        }
        if matches!(
            pressed,
            Some(Foot::Apply | Foot::Okay | Foot::SaveAsDefault)
        ) {
            let sound_before = profile.sound.clone();
            draft.apply(profile);
            tools.keep_profile(profile);
            if profile.sound != sound_before {
                audio.options_changed(&profile.sound);
            }
        }
        if pressed == Some(Foot::SaveAsDefault) {
            tools
                .profile_home
                .save_as_default(&places::for_saving(profile));
        }
        let mut covered = vec![panel];
        if closed || matches!(pressed, Some(Foot::Cancel | Foot::Okay)) {
            self.open = false;
            self.macros.cancel_capture();
            self.drafts.clear();
            self.hues = HueRows::default();
        } else {
            self.draft = Some(draft);
            covered.extend(self.hues.picker(ui, rect, frame, tools, profile));
        }
        covered
    }

    /// The pages, one button for each.
    fn page_list(&mut self, ui: &egui::Ui, left_top: Pos2) {
        let mut y = left_top.y;
        for (index, words) in Page::LABELS.iter().enumerate() {
            let page = Page::from_index(index);
            let color = if page == self.page {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            let area = Rect::from_min_size(
                Pos2::new(left_top.x, y),
                Vec2::new(PAGE_LIST_WIDTH, PAGE_ROW - theme::ROW_GAP),
            );
            if theme::segment_keyed(ui, area, Id::new(("options-page", index)), words, color) {
                self.page = page;
                self.macros.cancel_capture();
            }
            y += PAGE_ROW;
        }
    }

    fn page_rows(
        &mut self,
        ui: &mut egui::Ui,
        profile: &mut Profile,
        audio: &Audio,
        tools: &Tools<'_>,
    ) {
        if self.page == Page::Sound && !audio.note().is_empty() {
            ui.label(
                RichText::new(audio.note())
                    .font(text_font(theme::SIZE_BODY))
                    .color(theme::WAITING),
            );
        }
        let mut section = "";
        for row in rows_on(self.page) {
            if row.section != section {
                section = row.section;
                ui.add_space(SECTION_GAP);
                ui.label(
                    RichText::new(section)
                        .font(title_font(theme::SIZE_BODY))
                        .color(theme::GOAL),
                );
            }
            if let Some(value) = self.control(ui, row, (row.get)(profile), tools) {
                (row.set)(profile, value);
            }
        }
    }

    /// Draws the control of one row. Gives the new value when the player
    /// changed it.
    fn control(
        &mut self,
        ui: &mut egui::Ui,
        row: &'static OptionRow,
        value: OptionValue,
        tools: &Tools<'_>,
    ) -> Option<OptionValue> {
        let page = self.page;
        let height = ui.spacing().interact_size.y;
        match (row.kind, value) {
            (OptionKind::Toggle, OptionValue::Toggle(mut on)) => ui
                .checkbox(&mut on, row.label)
                .changed()
                .then_some(OptionValue::Toggle(on)),
            (
                OptionKind::Slider {
                    min,
                    max,
                    step,
                    unit,
                },
                OptionValue::Number(mut number),
            ) => {
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = CONTROL_WIDTH;
                    let mut moved = number;
                    let slider = egui::Slider::new(&mut moved, min..=max).show_value(false);
                    let touched = ui.add(slider).changed();
                    let snapped = snap(moved, min, max, step);
                    let changed = touched && snapped != number;
                    if changed {
                        number = snapped;
                    }
                    ui.add_sized(
                        [NUMBER_WIDTH, height],
                        egui::Label::new(unit.format(number)),
                    );
                    ui.label(row.label);
                    changed.then_some(OptionValue::Number(number))
                })
                .inner
            }
            (OptionKind::Choice { labels }, OptionValue::Choice(index)) => {
                ui.horizontal(|ui| {
                    let id = Id::new(("options-choice", self.page.index(), row.label));
                    let chosen = choice_box(ui, id, labels, index);
                    ui.label(row.label);
                    (chosen != index).then_some(OptionValue::Choice(chosen))
                })
                .inner
            }
            (OptionKind::Hue, OptionValue::Hue(mut hue)) => {
                ui.horizontal(|ui| {
                    let changed = self.hues.row(ui, (page, row.label, 0), &mut hue, tools);
                    ui.label(row.label);
                    changed.then_some(OptionValue::Hue(hue))
                })
                .inner
            }
            (OptionKind::Text, OptionValue::Text(mut words)) => {
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut words).desired_width(CONTROL_WIDTH);
                    let changed = ui.add(edit).changed();
                    ui.label(row.label);
                    changed.then_some(OptionValue::Text(words))
                })
                .inner
            }
            (OptionKind::FilePath, OptionValue::FilePath(path)) => {
                let mut words = path
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut words)
                        .desired_width(CONTROL_WIDTH)
                        .hint_text(HINT_NO_FILE);
                    let changed = ui.add(edit).changed();
                    ui.label(row.label);
                    changed.then(|| {
                        OptionValue::FilePath((!words.is_empty()).then(|| PathBuf::from(&words)))
                    })
                })
                .inner
            }
            (OptionKind::TextList, OptionValue::TextList(lines)) => {
                ui.label(row.label);
                self.drafted_field(ui, row.label, lines.join(LINE_BREAK), HINT_LINES)
                    .map(|words| OptionValue::TextList(parse_lines(&words)))
            }
            (OptionKind::IdList, OptionValue::Ids(ids)) => {
                ui.label(row.label);
                self.drafted_field(ui, row.label, format_ids(&ids), HINT_IDS)
                    .map(|words| OptionValue::Ids(parse_ids(&words)))
            }
            (OptionKind::KeyList, OptionValue::Keys(keys)) => {
                ui.label(row.label);
                self.key_list(ui, keys).map(OptionValue::Keys)
            }
            (OptionKind::InfoBarItems, OptionValue::InfoBarItems(items)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.hues, page, row.label, tools);
                info_bar_items(ui, items, hues).map(OptionValue::InfoBarItems)
            }
            (OptionKind::JournalTabs, OptionValue::JournalTabs(tabs)) => {
                ui.label(row.label);
                journal_tabs(ui, tabs).map(OptionValue::JournalTabs)
            }
            (OptionKind::Cooldowns, OptionValue::Cooldowns(rules)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.hues, page, row.label, tools);
                cooldown_rules(ui, rules, hues).map(OptionValue::Cooldowns)
            }
            (OptionKind::HighlightRules, OptionValue::HighlightRules(rules)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.hues, page, row.label, tools);
                highlight_rules(ui, rules, hues).map(OptionValue::HighlightRules)
            }
            (OptionKind::CounterItems, OptionValue::CounterItems(items)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.hues, page, row.label, tools);
                counter_items(ui, items, hues).map(OptionValue::CounterItems)
            }
            (kind, value) => {
                tracing::warn!(label = row.label, ?kind, ?value, "option row of two forms");
                None
            }
        }
    }

    /// A field of many lines. What the player typed stays as he typed it
    /// until the field loses the keys. Gives the words when they changed.
    fn drafted_field(
        &mut self,
        ui: &mut egui::Ui,
        label: &'static str,
        kept: String,
        hint: &str,
    ) -> Option<String> {
        let key = (self.page, label);
        let mut words = self.drafts.get(&key).cloned().unwrap_or(kept);
        let response = ui.add(
            egui::TextEdit::multiline(&mut words)
                .desired_rows(LIST_LINES)
                .desired_width(f32::INFINITY)
                .hint_text(hint),
        );
        if response.lost_focus() {
            self.drafts.remove(&key);
        } else if response.changed() {
            self.drafts.insert(key, words.clone());
        }
        response.changed().then_some(words)
    }

    /// Waits for the key or the controller buttons of a macro. The keys of
    /// the window do not run while it waits.
    pub fn capturing(&self) -> bool {
        self.macros.capture.is_some()
    }

    /// The macros of the Macros page: each with its name, its key, its
    /// controller buttons and, when opened, its steps.
    fn key_list(
        &mut self,
        ui: &mut egui::Ui,
        mut keys: Vec<KeyBinding>,
    ) -> Option<Vec<KeyBinding>> {
        let mut changed = self.take_capture(ui, &mut keys);
        let mut remove = None;
        for at in 0..keys.len() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    changed |= self.macro_head(ui, at, &mut keys[at]);
                    if ui.button(WORDS_REMOVE).clicked() {
                        remove = Some(at);
                    }
                });
                if self.macros.open == Some(at) {
                    changed |= macro_steps(ui, at, &mut keys);
                }
            });
        }
        if let Some(at) = remove {
            self.macros.remove(&mut keys, at);
            changed = true;
        }
        if ui.button(WORDS_ADD_MACRO).clicked() {
            self.macros.add(&mut keys);
            changed = true;
        }
        default_lists(ui);
        changed.then_some(keys)
    }

    /// Takes the key or the buttons the editor waits for. Esc cancels.
    fn take_capture(&mut self, ui: &egui::Ui, keys: &mut [KeyBinding]) -> bool {
        match self.macros.capture {
            Some(Capture::Chord(_)) => match take_pressed(ui) {
                Some(Pressed::Cancel) => self.macros.cancel_capture(),
                Some(Pressed::Chord(chord)) => return self.macros.take_chord(keys, chord),
                None => {}
            },
            Some(Capture::Pad(_)) => {
                if matches!(take_pressed(ui), Some(Pressed::Cancel)) {
                    self.macros.cancel_capture();
                } else if let Some(chord) = pressed_this_frame(ui.ctx()) {
                    return self.macros.take_pad(keys, chord);
                }
            }
            None => {}
        }
        false
    }

    /// The name, the key, the buttons and the Steps switch of one macro.
    fn macro_head(&mut self, ui: &mut egui::Ui, at: usize, binding: &mut KeyBinding) -> bool {
        let height = ui.spacing().interact_size.y;
        let name = egui::TextEdit::singleline(&mut binding.name)
            .hint_text(HINT_MACRO_NAME)
            .desired_width(NAME_WIDTH);
        let mut changed = ui.add(name).changed();
        let chord_words = match (self.macros.capture, &binding.chord) {
            (Some(Capture::Chord(waiting)), _) if waiting == at => WORDS_PRESS_KEY.to_string(),
            (_, Some(chord)) => chord.to_string(),
            (_, None) => WORDS_NO_KEY.to_string(),
        };
        if ui
            .add_sized([CHORD_WIDTH, height], egui::Button::new(chord_words))
            .clicked()
        {
            self.macros.capture(Capture::Chord(at));
        }
        let pad_words = match (self.macros.capture, &binding.pad) {
            (Some(Capture::Pad(waiting)), _) if waiting == at => WORDS_PRESS_BUTTON.to_string(),
            (_, Some(pad)) => pad.to_string(),
            (_, None) => WORDS_NO_BUTTON.to_string(),
        };
        if ui
            .add_sized([CHORD_WIDTH, height], egui::Button::new(pad_words))
            .clicked()
        {
            self.macros.capture(Capture::Pad(at));
        }
        if (binding.chord.is_some() || binding.pad.is_some()) && ui.button(WORDS_CLEAR).clicked() {
            binding.chord = None;
            binding.pad = None;
            changed = true;
        }
        let open = self.macros.open == Some(at);
        if ui.selectable_label(open, WORDS_STEPS).clicked() {
            self.macros.open = (!open).then_some(at);
        }
        changed
    }
}

/// The steps of an opened macro, each with its action, its argument and
/// the buttons that move and remove it, and Add step under them.
fn macro_steps(ui: &mut egui::Ui, at: usize, keys: &mut [KeyBinding]) -> bool {
    let mut changed = false;
    let mut moved = None;
    let mut removed = None;
    for step in 0..keys[at].steps.len() {
        ui.horizontal(|ui| {
            ui.add_sized(
                [STEP_NUMBER_WIDTH, ui.spacing().interact_size.y],
                egui::Label::new(format!("{}", step + 1)),
            );
            let current = keys[at].steps[step].clone();
            if let Some(action) = action_box(ui, (at, step), &current.action) {
                MacroEditor::set_action(keys, at, step, action);
                changed = true;
            }
            changed |= argument_editor(ui, (at, step), &mut keys[at].steps[step]);
            if ui.small_button(WORDS_UP).clicked() {
                moved = Some((step, Move::Up));
            }
            if ui.small_button(WORDS_DOWN).clicked() {
                moved = Some((step, Move::Down));
            }
            if ui.small_button(WORDS_REMOVE).clicked() {
                removed = Some(step);
            }
        });
    }
    if let Some((step, way)) = moved {
        changed |= MacroEditor::move_step(keys, at, step, way);
    }
    if let Some(step) = removed {
        MacroEditor::remove_step(keys, at, step);
        changed = true;
    }
    if let Some(action) = add_step_box(ui, at) {
        MacroEditor::add_step(keys, at, action);
        changed = true;
    }
    changed
}

/// A drop-down list of every action, by group. Gives the action picked.
fn action_list(ui: &mut egui::Ui, current: Option<ActionId>) -> Option<ActionId> {
    let mut picked = None;
    for group in Group::ALL {
        ui.label(RichText::new(group.label()).color(theme::GOAL));
        for spec in ACTIONS.iter().filter(|spec| spec.group == group) {
            if ui
                .selectable_label(current == Some(spec.action), spec.label)
                .clicked()
            {
                picked = Some(spec.action);
            }
        }
    }
    picked
}

/// The action of a step. Gives the new action when the player picked one.
fn action_box(ui: &mut egui::Ui, place: (usize, usize), action: &str) -> Option<ActionId> {
    let current = ActionId::from_id(action);
    let words = step_words(action, "");
    egui::ComboBox::from_id_salt(("macro-step-action", place))
        .width(ACTION_WIDTH)
        .selected_text(words)
        .show_ui(ui, |ui| action_list(ui, current))
        .inner
        .flatten()
        .filter(|picked| Some(*picked) != current)
}

/// The Add step list under the steps of a macro.
fn add_step_box(ui: &mut egui::Ui, at: usize) -> Option<ActionId> {
    egui::ComboBox::from_id_salt(("macro-add-step", at))
        .width(ACTION_WIDTH)
        .selected_text(WORDS_ADD_STEP)
        .show_ui(ui, |ui| action_list(ui, None))
        .inner
        .flatten()
}

/// The argument of a step, as its kind needs: a list to pick from, a field
/// to type in, or both for a hotkey. True when the player changed it.
fn argument_editor(ui: &mut egui::Ui, place: (usize, usize), step: &mut MacroStep) -> bool {
    let Some(kind) = ActionId::from_id(&step.action).map(|a| a.spec().argument) else {
        return false;
    };
    let choices = kind.choices();
    let mut changed = false;
    if kind.is_typed() {
        let field = egui::TextEdit::singleline(&mut step.argument)
            .hint_text(kind.hint())
            .desired_width(WORDS_WIDTH);
        changed |= ui.add(field).changed();
    }
    if choices.is_empty() {
        return changed;
    }
    let shown = if kind.is_typed() {
        WORDS_SUGGESTIONS
    } else {
        step.argument.as_str()
    };
    let picked = egui::ComboBox::from_id_salt(("macro-step-argument", place))
        .width(CONTROL_WIDTH)
        .selected_text(shown.to_string())
        .show_ui(ui, |ui| {
            let mut picked = None;
            for choice in &choices {
                if ui
                    .selectable_label(choice.eq_ignore_ascii_case(&step.argument), choice.as_str())
                    .clicked()
                {
                    picked = Some(choice.clone());
                }
            }
            picked
        })
        .inner
        .flatten();
    if let Some(choice) = picked.filter(|choice| *choice != step.argument) {
        step.argument = choice;
        changed = true;
    }
    changed
}

/// The default keys and controller buttons, in words, under the macros.
fn default_lists(ui: &mut egui::Ui) {
    for (title, list) in [
        (WORDS_DEFAULT_KEYS, default_keys().collect::<Vec<_>>()),
        (WORDS_DEFAULT_BUTTONS, default_buttons().collect::<Vec<_>>()),
    ] {
        ui.add_space(SECTION_GAP);
        ui.label(RichText::new(title).color(theme::TEXT_DIM));
        for (trigger, action, argument) in list {
            ui.label(format!("{trigger}: {}", step_words(action, argument)));
        }
    }
}

/// The hue rows of one list of a page: each entry's hue opens the color
/// picker for its place in the list.
struct HueList<'a, 't> {
    rows: &'a mut HueRows,
    page: Page,
    label: &'static str,
    tools: &'a Tools<'t>,
}

impl<'a, 't> HueList<'a, 't> {
    fn new(rows: &'a mut HueRows, page: Page, label: &'static str, tools: &'a Tools<'t>) -> Self {
        Self {
            rows,
            page,
            label,
            tools,
        }
    }

    /// The hue of the entry at `at`. True when it changed.
    fn row(&mut self, ui: &mut egui::Ui, at: usize, hue: &mut u16) -> bool {
        self.rows
            .row(ui, (self.page, self.label, at), hue, self.tools)
    }
}

/// The buttons at the foot. Apply and Okay light up while the copy holds
/// changes. Gives the one pressed.
fn foot_buttons(ui: &egui::Ui, left_top: Pos2, changed: bool) -> Option<Foot> {
    let mut x = left_top.x;
    let mut pressed = None;
    for (foot, words, width) in FOOT {
        let area = Rect::from_min_size(Pos2::new(x, left_top.y), Vec2::new(width, FOOT_ROW));
        let color = match foot {
            Foot::Apply | Foot::Okay if changed => theme::GOAL,
            _ => theme::TEXT,
        };
        if theme::segment_keyed(ui, area, Id::new(("options-foot", words)), words, color) {
            pressed = Some(foot);
        }
        if foot == Foot::Default && ui.rect_contains_pointer(area) {
            super::tips::label(ui, HINT_DEFAULT, "");
        }
        x = area.right() + theme::ROW_GAP;
    }
    pressed
}

fn info_bar_items(
    ui: &mut egui::Ui,
    mut items: Vec<InfoBarItem>,
    mut hues: HueList<'_, '_>,
) -> Option<Vec<InfoBarItem>> {
    let new_item = || {
        Some(InfoBarItem {
            label: String::new(),
            hue: NO_HUE,
            data: NEW_INFO_BAR_DATA,
        })
    };
    let changed = edit_list(ui, &mut items, WORDS_ADD_ITEM, new_item, |ui, at, item| {
        ui.horizontal(|ui| {
            let label = egui::TextEdit::singleline(&mut item.label)
                .hint_text(HINT_LABEL)
                .desired_width(WORDS_WIDTH);
            let mut changed = ui.add(label).changed();
            changed |= hues.row(ui, at, &mut item.hue);
            let index = item.data.index();
            let chosen = choice_box(
                ui,
                Id::new(("options-info-bar", at)),
                InfoBarData::LABELS,
                index,
            );
            if chosen != index {
                item.data = InfoBarData::from_index(chosen);
                changed = true;
            }
            changed
        })
        .inner
    });
    changed.then_some(items)
}

fn journal_tabs(ui: &mut egui::Ui, mut tabs: Vec<JournalTab>) -> Option<Vec<JournalTab>> {
    let new_tab = || {
        Some(JournalTab {
            name: WORDS_NEW_TAB.to_string(),
            kinds: Vec::new(),
        })
    };
    let changed = edit_list(ui, &mut tabs, WORDS_ADD_TAB, new_tab, |ui, _, tab| {
        let name = egui::TextEdit::singleline(&mut tab.name)
            .hint_text(HINT_TAB_NAME)
            .desired_width(WORDS_WIDTH);
        let mut changed = ui.add(name).changed();
        ui.horizontal_wrapped(|ui| {
            for (index, words) in JournalKind::LABELS.iter().enumerate() {
                let kind = JournalKind::from_index(index);
                let mut shown = tab.kinds.contains(&kind);
                if ui.checkbox(&mut shown, *words).changed() {
                    if shown {
                        tab.kinds.push(kind);
                    } else {
                        tab.kinds.retain(|other| *other != kind);
                    }
                    changed = true;
                }
            }
        });
        changed
    });
    changed.then_some(tabs)
}

/// The cooldown bars of the Combat & Spells page: each label, hue, time,
/// trigger words and whose lines start it.
fn cooldown_rules(
    ui: &mut egui::Ui,
    mut rules: Vec<CooldownRule>,
    mut hues: HueList<'_, '_>,
) -> Option<Vec<CooldownRule>> {
    let new_rule = || {
        Some(CooldownRule {
            label: String::new(),
            hue: NO_HUE,
            trigger: String::new(),
            seconds: DEFAULT_COOLDOWN_SECONDS,
            source: CooldownSource::Anyone,
            restart: true,
        })
    };
    let changed = edit_list(
        ui,
        &mut rules,
        WORDS_ADD_COOLDOWN,
        new_rule,
        |ui, at, rule| {
            let mut changed = ui
                .horizontal(|ui| {
                    let label = egui::TextEdit::singleline(&mut rule.label)
                        .hint_text(HINT_LABEL)
                        .desired_width(WORDS_WIDTH);
                    let mut changed = ui.add(label).changed();
                    changed |= hues.row(ui, at, &mut rule.hue);
                    let seconds = egui::DragValue::new(&mut rule.seconds)
                        .range(COOLDOWN_SECONDS_MIN..=COOLDOWN_SECONDS_MAX)
                        .speed(COOLDOWN_SECONDS_STEP)
                        .suffix(SECONDS_SUFFIX);
                    changed | ui.add(seconds).changed()
                })
                .inner;
            changed |= ui
                .horizontal(|ui| {
                    let trigger = egui::TextEdit::singleline(&mut rule.trigger)
                        .hint_text(HINT_TRIGGER)
                        .desired_width(WORDS_WIDTH);
                    let mut changed = ui.add(trigger).changed();
                    let index = rule.source.index();
                    let chosen = choice_box(
                        ui,
                        Id::new(("options-cooldown", at)),
                        CooldownSource::LABELS,
                        index,
                    );
                    if chosen != index {
                        rule.source = CooldownSource::from_index(chosen);
                        changed = true;
                    }
                    changed | ui.checkbox(&mut rule.restart, WORDS_RESTART).changed()
                })
                .inner;
            changed
        },
    );
    changed.then_some(rules)
}

/// The highlight rules of the grid containers: each name, hue, and the
/// properties it needs, with buttons that add the usual rules.
fn highlight_rules(
    ui: &mut egui::Ui,
    mut rules: Vec<HighlightRule>,
    mut hues: HueList<'_, '_>,
) -> Option<Vec<HighlightRule>> {
    let mut changed = edit_list(
        ui,
        &mut rules,
        WORDS_ADD_RULE,
        || Some(highlight::blank()),
        |ui, at, rule| {
            let mut changed = ui
                .horizontal(|ui| {
                    let name = egui::TextEdit::singleline(&mut rule.name)
                        .hint_text(HINT_RULE_NAME)
                        .desired_width(WORDS_WIDTH);
                    let mut changed = ui.add(name).changed();
                    changed |= hues.row(ui, at, &mut rule.hue);
                    changed |= ui.checkbox(&mut rule.need_all, WORDS_NEED_ALL).changed();
                    changed
                        | ui.checkbox(&mut rule.corpses_only, WORDS_CORPSES_ONLY)
                            .changed()
                })
                .inner;
            changed |= edit_list(
                ui,
                &mut rule.needs,
                WORDS_ADD_NEED,
                || {
                    Some(PropertyNeed {
                        words: String::new(),
                        min: None,
                    })
                },
                |ui, _, need| property_need(ui, need),
            );
            changed
        },
    );
    ui.horizontal(|ui| {
        for preset in highlight::presets() {
            if ui
                .button(format!("{WORDS_ADD_PRESET} {}", preset.name))
                .clicked()
            {
                rules.push(preset);
                changed = true;
            }
        }
    });
    changed.then_some(rules)
}

/// One property a highlight rule needs: its words, and its least number
/// when it has one.
fn property_need(ui: &mut egui::Ui, need: &mut PropertyNeed) -> bool {
    ui.horizontal(|ui| {
        let words = egui::TextEdit::singleline(&mut need.words)
            .hint_text(HINT_PROPERTY)
            .desired_width(WORDS_WIDTH);
        let mut changed = ui.add(words).changed();
        let mut has_min = need.min.is_some();
        if ui.checkbox(&mut has_min, WORDS_AT_LEAST).changed() {
            need.min = has_min.then_some(0.0);
            changed = true;
        }
        if let Some(min) = need.min.as_mut() {
            changed |= ui.add(egui::DragValue::new(min)).changed();
        }
        changed
    })
    .inner
}

/// The items the counter bar counts: each label, graphic and hue.
fn counter_items(
    ui: &mut egui::Ui,
    mut items: Vec<CounterItem>,
    mut hues: HueList<'_, '_>,
) -> Option<Vec<CounterItem>> {
    let new_item = || {
        Some(CounterItem {
            label: String::new(),
            graphic: 0,
            hue: NO_HUE,
        })
    };
    let changed = edit_list(ui, &mut items, WORDS_ADD_ITEM, new_item, |ui, at, item| {
        ui.horizontal(|ui| {
            let label = egui::TextEdit::singleline(&mut item.label)
                .hint_text(HINT_LABEL)
                .desired_width(WORDS_WIDTH);
            let mut changed = ui.add(label).changed();
            changed |= hue_box(ui, &mut item.graphic);
            changed | hues.row(ui, at, &mut item.hue)
        })
        .inner
    });
    changed.then_some(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::modern::testing::draw_frames;

    /// Draws the Options once. Gives the places they cover.
    fn draw_once(options: &mut OptionsUi, profile: &mut Profile) -> Vec<Rect> {
        let audio = Audio::new(None);
        let frame = WatchFrame::default();
        let mut covered = Vec::new();
        draw_frames(profile, &[Vec::new()], |ui, rect, tools, profile| {
            covered = options.draw(ui, rect, &frame, tools, profile, &audio);
        });
        covered
    }

    #[test]
    fn the_panel_edits_a_copy_that_only_apply_makes_the_profile() {
        let mut options = OptionsUi::default();
        let mut profile = Profile::default();
        assert!(draw_once(&mut options, &mut profile).is_empty());
        options.toggle();
        let covered = draw_once(&mut options, &mut profile);
        let window = Rect::from_min_size(Pos2::ZERO, crate::window::modern::testing::SCREEN);
        assert!(covered.iter().all(|panel| window.contains_rect(*panel)));
        let draft = options.draft.as_mut().unwrap();
        draft.edited.general.always_run = !profile.general.always_run;
        draw_once(&mut options, &mut profile);
        assert_eq!(profile, Profile::default(), "not applied yet");
        options.draft.as_mut().unwrap().apply(&mut profile);
        assert_ne!(
            profile.general.always_run,
            Profile::default().general.always_run
        );
        options.toggle();
        draw_once(&mut options, &mut profile);
        assert!(options.draft.is_none(), "a closed panel lets its copy go");
    }

    #[test]
    fn a_picked_hue_goes_to_its_own_row() {
        let mut rows = HueRows {
            open: None,
            picked: Some(((Page::Speech, "Emote", 0), 0x0035)),
        };
        let mut hue = 0;
        let mut other = 0;
        draw_frames(&mut Profile::default(), &[Vec::new()], |ui, _, tools, _| {
            assert!(!rows.row(ui, (Page::Speech, "Speech", 0), &mut other, tools));
            assert!(rows.row(ui, (Page::Speech, "Emote", 0), &mut hue, tools));
        });
        assert_eq!((hue, other), (0x0035, 0));
        assert!(rows.picked.is_none());
    }

    #[test]
    fn a_list_of_ids_reads_hex_and_decimal_and_skips_other_words() {
        assert_eq!(
            parse_ids("0x0123, 45;x 0x00FF\n7"),
            vec![0x0123, 45, 0x00FF, 7]
        );
        assert_eq!(parse_ids(&format_ids(&[1, 0xABCD])), vec![1, 0xABCD]);
        assert!(parse_ids("").is_empty());
    }

    #[test]
    fn a_slider_keeps_to_its_steps_and_its_range() {
        assert_eq!(snap(0.8134, 0.0, 1.0, 0.01), 0.81);
        assert_eq!(snap(0.8, 0.0, 1.0, 0.01), 0.8);
        assert_eq!(snap(17.4, 5.0, 25.0, 1.0), 17.0);
        assert_eq!(snap(260.0, 12.0, 250.0, 1.0), 250.0);
        assert_eq!(snap(1.0, 0.5, 3.0, 0.05), 1.0);
    }

    #[test]
    fn a_list_of_lines_has_no_empty_lines() {
        assert_eq!(parse_lines(" Bob \n\n Mara\n"), vec!["Bob", "Mara"]);
    }

    #[test]
    fn every_page_draws_and_changes_nothing_by_itself() {
        let mut options = OptionsUi::default();
        options.toggle();
        let mut profile = Profile::default();
        for index in 0..Page::LABELS.len() {
            options.page = Page::from_index(index);
            draw_once(&mut options, &mut profile);
            let draft = options.draft.as_ref().unwrap();
            for row in rows_on(options.page) {
                let drawn = (row.get)(&draft.edited);
                assert_eq!(drawn, (row.get)(&Profile::default()), "{}", row.label);
            }
            assert!(!draft.changed(), "{:?}", options.page);
        }
        assert_eq!(profile, Profile::default());
    }

    #[test]
    fn an_open_macro_draws_every_kind_of_step_and_changes_nothing() {
        let mut options = OptionsUi::default();
        options.toggle();
        options.page = Page::Macros;
        options.macros.open = Some(0);
        let mut profile = Profile::default();
        profile.macros.key_bindings.push(KeyBinding {
            name: "every step".into(),
            chord: Some("Ctrl+F1".parse().unwrap()),
            pad: Some("LeftTrigger+South".parse().unwrap()),
            steps: ACTIONS
                .iter()
                .map(|spec| crate::window::actions::new_step(spec.action))
                .collect(),
        });
        let before = profile.clone();
        draw_once(&mut options, &mut profile);
        assert_eq!(profile, before);
        assert!(!options.draft.as_ref().unwrap().changed() && !options.capturing());
    }
}
