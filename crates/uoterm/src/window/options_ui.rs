//! The Options screen of the Modern style. It draws every row of the option
//! table, page by page, with egui controls on a glass panel the player
//! moves, sizes and locks. As the classic Options gump does, it edits a
//! copy of the profile: Apply and Okay make it the profile, Cancel lets it
//! go, Default puts the page back to its defaults, and "Save as default"
//! makes it the start of each new character. A hue row opens the color
//! picker: the grid of hues with its shade slider and the eyedropper.

use super::actions::editor::{step_words, Capture, MacroEditor, Move, Pressed};
use super::actions::{ActionId, Group, ACTIONS};
use super::audio::Audio;
use super::boxes_ui::Tools;
use super::bridge;
use super::model::highlight;
use super::model::places;
use super::modern::frame::{self as panel_frame, FrameEvent, PanelSpec};
use super::modern::hue_ui::{self, HueGridUi};
use super::pad::pressed_this_frame;
use super::settings::{
    rows_on, Choice, CooldownRule, CooldownSource, CounterItem, HighlightRule, InfoBarData,
    InfoBarItem, JournalKind, JournalTab, KeyBinding, MacroStep, OptionKind, OptionRow,
    OptionValue, Page, Profile, PropertyNeed,
};
use super::theme::{self, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Event, Id, Pos2, Rect, RichText, Sense, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use uoterm_view::ui::options::{
    at_least, chord_words, default_lines, foot_color, format_ids, hue_words, new_cooldown,
    new_counter_item, new_info_bar_item, new_journal_tab, new_property_need, options_first_place,
    pad_words, parse_ids, parse_lines, picker_first_place, preset_words, snap, step_shown, Foot,
    HueKey, HueRows, OptionsPanel, COOLDOWN_SECONDS_MAX, COOLDOWN_SECONDS_MIN,
    COOLDOWN_SECONDS_STEP, FOOT, HEX_PREFIX, HINT_DEFAULT, HINT_IDS, HINT_LABEL, HINT_LINES,
    HINT_MACRO_NAME, HINT_NO_FILE, HINT_PROPERTY, HINT_RULE_NAME, HINT_SWATCH, HINT_TAB_NAME,
    HINT_TRIGGER, HUE_DIGITS, LINE_BREAK, OPTIONS_FOOT_ROW as FOOT_ROW, OPTIONS_ID, OPTIONS_LEAST,
    PAGE_LIST_WIDTH, PAGE_ROW, PICKED_SWATCH, PICKER_ID, SECONDS_SUFFIX, WORDS_ADD_COOLDOWN,
    WORDS_ADD_ITEM, WORDS_ADD_MACRO, WORDS_ADD_NEED, WORDS_ADD_RULE, WORDS_ADD_STEP, WORDS_ADD_TAB,
    WORDS_AT_LEAST, WORDS_CANCEL, WORDS_CLEAR, WORDS_COLOR, WORDS_CORPSES_ONLY, WORDS_DOWN,
    WORDS_NEED_ALL, WORDS_OKAY, WORDS_OPTIONS as WORDS_TITLE, WORDS_REMOVE, WORDS_RESTART,
    WORDS_STEPS, WORDS_UP,
};

const COLUMN_GAP: f32 = 16.0;
const CONTROL_WIDTH: f32 = 240.0;
const NUMBER_WIDTH: f32 = 72.0;
const CHORD_WIDTH: f32 = 140.0;
const WORDS_WIDTH: f32 = 150.0;
const HUE_WIDTH: f32 = 72.0;
const SWATCH: Vec2 = Vec2::new(22.0, 18.0);
const SECTION_GAP: f32 = 10.0;
const LIST_LINES: usize = 4;
const NAME_WIDTH: f32 = 140.0;
const ACTION_WIDTH: f32 = 200.0;
const STEP_NUMBER_WIDTH: f32 = 20.0;

#[derive(Default)]
pub struct OptionsUi {
    panel: OptionsPanel,
    /// The words the player types in a list field, kept while the field has
    /// the keys, so a line he has only begun stays as he typed it.
    drafts: HashMap<(Page, &'static str), String>,
    /// The grid of the color picker a hue row opened.
    grid: HueGridUi,
}

/// A hue: its number, and a swatch that opens the color picker. True when
/// it changed.
fn hue_row(
    rows: &mut HueRows,
    ui: &mut egui::Ui,
    key: HueKey,
    hue: &mut u16,
    tools: &Tools<'_>,
) -> bool {
    let mut changed = hue_box(ui, hue);
    let (area, response) = ui.allocate_exact_size(SWATCH, Sense::click());
    hue_ui::swatch(ui, area, *hue, tools);
    if response.hovered() {
        super::tips::label(ui, HINT_SWATCH, "");
    }
    if response.clicked() {
        rows.open(key, *hue);
    }
    if let Some(picked) = rows.take_picked(key) {
        changed |= *hue != picked;
        *hue = picked;
    }
    changed
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
        Some(Pressed::of(&bridge::key_name(key), bridge::mods(modifiers)))
    })
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

impl OptionsUi {
    /// Opens the panel, or closes it. The button is in the control bar.
    pub fn toggle(&mut self) {
        self.panel.toggle();
        if !self.panel.open {
            self.forget();
        }
    }

    pub fn is_open(&self) -> bool {
        self.panel.open
    }

    /// Lets the words typed in the fields and the color picker go, as the
    /// panel closes.
    fn forget(&mut self) {
        self.drafts.clear();
        self.grid = HueGridUi::default();
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
        audio: &mut Audio,
    ) -> Vec<Rect> {
        if !self.panel.open {
            return Vec::new();
        }
        let mut draft = self.panel.take_draft(profile);
        let spec = PanelSpec {
            id: OPTIONS_ID,
            title: WORDS_TITLE,
            default: bridge::rect(options_first_place(bridge::area(rect))),
            min_size: Some(bridge::vec2(OPTIONS_LEAST)),
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
            .id_salt(("options-rows", self.panel.page.index()))
            .auto_shrink(false)
            .show(&mut rows_ui, |ui| {
                self.page_rows(ui, &mut draft.edited, audio, tools);
            });
        self.panel.draft = Some(draft);
        let closed =
            panel_frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if let Some(foot) = pressed {
            let page = self.panel.page;
            let sound_before = profile.sound.clone();
            let done = self.panel.press_foot(foot, profile);
            if foot == Foot::Default {
                self.drafts.retain(|(on, _), _| *on != page);
            }
            if done.applied {
                tools.keep_profile(profile);
                if profile.sound != sound_before {
                    audio.options_changed(&profile.sound);
                }
            }
            if done.save_as_default {
                tools
                    .profile_home
                    .save_as_default(&places::for_saving(profile));
            }
        }
        if closed {
            self.panel.close();
        }
        let mut covered = vec![panel];
        if self.panel.open {
            covered.extend(self.picker(ui, rect, frame, tools, profile));
        } else {
            self.forget();
        }
        covered
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
        let (_, pick) = self.panel.hues.open.as_mut()?;
        self.grid.take_picked(pick, frame, tools);
        let spec = PanelSpec {
            id: PICKER_ID,
            title: WORDS_COLOR,
            default: bridge::rect(picker_first_place(bridge::area(rect))),
            min_size: None,
            closable: true,
        };
        let panel = panel_frame::place(rect, &spec, profile);
        let body = panel_frame::draw(ui.painter(), panel, WORDS_COLOR);
        let grid = self.grid.draw(ui, body.min, PICKER_ID, pick, tools, true);
        let shown = Rect::from_min_size(
            Pos2::new(grid.right() + theme::ROW_GAP * 2.0, grid.top()),
            bridge::vec2(PICKED_SWATCH),
        );
        hue_ui::swatch(ui, shown, pick.hue(), tools);
        ui.painter().text(
            shown.center_bottom() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::CENTER_TOP,
            hue_words(pick.hue()),
            text_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let foot = Pos2::new(body.left(), grid.bottom() + theme::ROW_GAP);
        let dropper = self.grid.eyedropper(ui, foot, PICKER_ID, frame, tools);
        let button = |at: f32| {
            Rect::from_min_size(
                Pos2::new(at, foot.y),
                Vec2::new(
                    uoterm_view::ui::options::FOOT_BUTTON_WIDTH,
                    hue_ui::EYEDROPPER_SIZE.y,
                ),
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
        if okay || cancel || closed {
            self.grid.stop();
            self.panel.hues.finish(okay);
        }
        Some(panel)
    }

    /// The pages, one button for each.
    fn page_list(&mut self, ui: &egui::Ui, left_top: Pos2) {
        let mut y = left_top.y;
        for (index, words) in Page::LABELS.iter().enumerate() {
            let page = Page::from_index(index);
            let color = if page == self.panel.page {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            let area = Rect::from_min_size(
                Pos2::new(left_top.x, y),
                Vec2::new(PAGE_LIST_WIDTH, PAGE_ROW - theme::ROW_GAP),
            );
            if theme::segment_keyed(ui, area, Id::new(("options-page", index)), words, color) {
                self.panel.choose_page(page);
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
        if self.panel.page == Page::Sound && !audio.note().is_empty() {
            ui.label(
                RichText::new(audio.note())
                    .font(text_font(theme::SIZE_BODY))
                    .color(theme::WAITING),
            );
        }
        let mut section = "";
        for row in rows_on(self.panel.page) {
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
        let page = self.panel.page;
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
                    let id = Id::new(("options-choice", self.panel.page.index(), row.label));
                    let chosen = choice_box(ui, id, labels, index);
                    ui.label(row.label);
                    (chosen != index).then_some(OptionValue::Choice(chosen))
                })
                .inner
            }
            (OptionKind::Hue, OptionValue::Hue(mut hue)) => {
                ui.horizontal(|ui| {
                    let changed = hue_row(
                        &mut self.panel.hues,
                        ui,
                        (page, row.label, 0),
                        &mut hue,
                        tools,
                    );
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
                let hues = HueList::new(&mut self.panel.hues, page, row.label, tools);
                info_bar_items(ui, items, hues).map(OptionValue::InfoBarItems)
            }
            (OptionKind::JournalTabs, OptionValue::JournalTabs(tabs)) => {
                ui.label(row.label);
                journal_tabs(ui, tabs).map(OptionValue::JournalTabs)
            }
            (OptionKind::Cooldowns, OptionValue::Cooldowns(rules)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.panel.hues, page, row.label, tools);
                cooldown_rules(ui, rules, hues).map(OptionValue::Cooldowns)
            }
            (OptionKind::HighlightRules, OptionValue::HighlightRules(rules)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.panel.hues, page, row.label, tools);
                highlight_rules(ui, rules, hues).map(OptionValue::HighlightRules)
            }
            (OptionKind::CounterItems, OptionValue::CounterItems(items)) => {
                ui.label(row.label);
                let hues = HueList::new(&mut self.panel.hues, page, row.label, tools);
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
        let key = (self.panel.page, label);
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
        self.panel.capturing()
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
                if self.panel.macros.open == Some(at) {
                    changed |= macro_steps(ui, at, &mut keys);
                }
            });
        }
        if let Some(at) = remove {
            self.panel.macros.remove(&mut keys, at);
            changed = true;
        }
        if ui.button(WORDS_ADD_MACRO).clicked() {
            self.panel.macros.add(&mut keys);
            changed = true;
        }
        default_lists(ui);
        changed.then_some(keys)
    }

    /// Takes the key or the buttons the editor waits for. Esc cancels.
    fn take_capture(&mut self, ui: &egui::Ui, keys: &mut [KeyBinding]) -> bool {
        if self.panel.macros.capture.is_none() {
            return false;
        }
        let pressed = take_pressed(ui);
        self.panel
            .macros
            .take_capture(keys, pressed, pressed_this_frame(ui.ctx()))
    }

    /// The name, the key, the buttons and the Steps switch of one macro.
    fn macro_head(&mut self, ui: &mut egui::Ui, at: usize, binding: &mut KeyBinding) -> bool {
        let height = ui.spacing().interact_size.y;
        let name = egui::TextEdit::singleline(&mut binding.name)
            .hint_text(HINT_MACRO_NAME)
            .desired_width(NAME_WIDTH);
        let mut changed = ui.add(name).changed();
        let chord_words = chord_words(self.panel.macros.capture, at, binding.chord.as_ref());
        if ui
            .add_sized([CHORD_WIDTH, height], egui::Button::new(chord_words))
            .clicked()
        {
            self.panel.macros.capture(Capture::Chord(at));
        }
        let pad_words = pad_words(self.panel.macros.capture, at, binding.pad.as_ref());
        if ui
            .add_sized([CHORD_WIDTH, height], egui::Button::new(pad_words))
            .clicked()
        {
            self.panel.macros.capture(Capture::Pad(at));
        }
        if (binding.chord.is_some() || binding.pad.is_some()) && ui.button(WORDS_CLEAR).clicked() {
            binding.chord = None;
            binding.pad = None;
            changed = true;
        }
        let open = self.panel.macros.open == Some(at);
        if ui.selectable_label(open, WORDS_STEPS).clicked() {
            self.panel.macros.open = (!open).then_some(at);
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
    let shown = step_shown(kind.is_typed(), &step.argument);
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
    for (title, lines) in default_lines() {
        ui.add_space(SECTION_GAP);
        ui.label(RichText::new(title).color(theme::TEXT_DIM));
        for line in lines {
            ui.label(line);
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
        hue_row(self.rows, ui, (self.page, self.label, at), hue, self.tools)
    }
}

/// The buttons at the foot. Apply and Okay light up while the copy holds
/// changes. Gives the one pressed.
fn foot_buttons(ui: &egui::Ui, left_top: Pos2, changed: bool) -> Option<Foot> {
    let mut x = left_top.x;
    let mut pressed = None;
    for (foot, words, width) in FOOT {
        let area = Rect::from_min_size(Pos2::new(x, left_top.y), Vec2::new(width, FOOT_ROW));
        let color = bridge::color(foot_color(foot, changed));
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
    let new_item = || Some(new_info_bar_item());
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
    let new_tab = || Some(new_journal_tab());
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
    let new_rule = || Some(new_cooldown());
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
                || Some(new_property_need()),
                |ui, _, need| property_need(ui, need),
            );
            changed
        },
    );
    ui.horizontal(|ui| {
        for preset in highlight::presets() {
            if ui.button(preset_words(&preset.name)).clicked() {
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
            need.min = at_least(has_min);
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
    let new_item = || Some(new_counter_item());
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
        let mut audio = Audio::new(None);
        let frame = WatchFrame::default();
        let mut covered = Vec::new();
        draw_frames(profile, &[Vec::new()], |ui, rect, tools, profile| {
            covered = options.draw(ui, rect, &frame, tools, profile, &mut audio);
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
        let draft = options.panel.draft.as_mut().unwrap();
        draft.edited.general.always_run = !profile.general.always_run;
        draw_once(&mut options, &mut profile);
        assert_eq!(profile, Profile::default(), "not applied yet");
        options.panel.draft.as_mut().unwrap().apply(&mut profile);
        assert_ne!(
            profile.general.always_run,
            Profile::default().general.always_run
        );
        options.toggle();
        draw_once(&mut options, &mut profile);
        assert!(
            options.panel.draft.is_none(),
            "a closed panel lets its copy go"
        );
    }

    #[test]
    fn every_page_draws_and_changes_nothing_by_itself() {
        let mut options = OptionsUi::default();
        options.toggle();
        let mut profile = Profile::default();
        for index in 0..Page::LABELS.len() {
            options.panel.page = Page::from_index(index);
            draw_once(&mut options, &mut profile);
            let draft = options.panel.draft.as_ref().unwrap();
            for row in rows_on(options.panel.page) {
                let drawn = (row.get)(&draft.edited);
                assert_eq!(drawn, (row.get)(&Profile::default()), "{}", row.label);
            }
            assert!(!draft.changed(), "{:?}", options.panel.page);
        }
        assert_eq!(profile, Profile::default());
    }

    #[test]
    fn an_open_macro_draws_every_kind_of_step_and_changes_nothing() {
        let mut options = OptionsUi::default();
        options.toggle();
        options.panel.page = Page::Macros;
        options.panel.macros.open = Some(0);
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
        assert!(!options.panel.draft.as_ref().unwrap().changed() && !options.capturing());
    }
}
