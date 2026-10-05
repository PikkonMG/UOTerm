//! The Options of the Modern style: every row of the option table, page
//! by page, edited on a copy of the profile that Apply and Okay make the
//! profile, with the macros of the Macros page (their keys and controller
//! buttons are captured from the input the view takes), and the color
//! picker a hue row opens. The rows are `settings::table`'s, the copy is
//! `model::options_draft`'s and the rest is `uoterm_view::ui::options`',
//! as in the Rust window. The page sends what the player typed or picked;
//! the view reads and keeps it.
//!
//! Three rows act as the browser can: a window mode of full screen asks
//! the page for the full screen, the TrueType font is one of the player
//! fonts the server lists, and the MIDI sound font stays a file the server
//! reads.

use super::asks::HueGridData;
use super::sheet::Choice;
use super::{Colored, FrameSpec, Framed, PANEL_COLOR_PICKER, PANEL_OPTIONS};
use crate::out::OutCall;
use crate::{kept, WebView};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uoterm_view::actions::editor::{step_words, Capture, MacroEditor, Move, Pressed};
use uoterm_view::actions::{ActionId, Group, ACTIONS};
use uoterm_view::guard::LocalAim;
use uoterm_view::input::KeyPress;
use uoterm_view::keys::default_keys;
use uoterm_view::model::highlight;
use uoterm_view::model::hue_grid::{GRID_COLUMNS, GRID_ROWS};
use uoterm_view::model::places;
use uoterm_view::pad::default_buttons;
use uoterm_view::settings::{
    rows_on, Choice as Choosable, CooldownSource, InfoBarData, JournalKind, KeyBinding, OptionKind,
    OptionRow, OptionValue, PadChord, Page, WindowMode,
};
use uoterm_view::ui::hues::{Eyedropper, WORDS_EYEDROPPER};
use uoterm_view::ui::options::{
    at_least, foot_color, format_ids, hue_words, new_cooldown, new_counter_item, new_info_bar_item,
    new_journal_tab, new_property_need, options_first_place, parse_hue, parse_ids, parse_lines,
    picker_first_place, snap, Foot, HueKey, OptionsPanel, FOOT, HINT_DEFAULT, HINT_IDS, HINT_LABEL,
    HINT_LINES, HINT_MACRO_NAME, HINT_NO_FILE, HINT_PROPERTY, HINT_RULE_NAME, HINT_SWATCH,
    HINT_TAB_NAME, HINT_TRIGGER, LINE_BREAK, OPTIONS_ID, OPTIONS_LEAST, PICKER_ID,
    WORDS_ADD_COOLDOWN, WORDS_ADD_ITEM, WORDS_ADD_MACRO, WORDS_ADD_NEED, WORDS_ADD_PRESET,
    WORDS_ADD_RULE, WORDS_ADD_STEP, WORDS_ADD_TAB, WORDS_AT_LEAST, WORDS_CANCEL, WORDS_CLEAR,
    WORDS_COLOR, WORDS_CORPSES_ONLY, WORDS_DEFAULT_BUTTONS, WORDS_DEFAULT_KEYS, WORDS_DOWN,
    WORDS_NEED_ALL, WORDS_NO_BUTTON, WORDS_NO_KEY, WORDS_OKAY, WORDS_OPTIONS, WORDS_PRESS_BUTTON,
    WORDS_PRESS_KEY, WORDS_REMOVE, WORDS_RESTART, WORDS_STEPS, WORDS_SUGGESTIONS, WORDS_UP,
};
use uoterm_view::ui::theme::{css_color, TEXT, WAITING};

/// The panel and the eyedropper of its color picker.
#[derive(Default)]
pub(crate) struct OptionsState {
    pub panel: OptionsPanel,
    eyedropper: Eyedropper,
}

/// The Options.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OptionsData {
    pub pages: Vec<Choice>,
    pub rows: Vec<RowData>,
    pub foot: Vec<Colored>,
    /// The tip of Default, the foot button at `default_at`.
    pub default_hint: &'static str,
    pub default_at: usize,
    pub remove: &'static str,
}

/// One row: the heading of its section when a new one starts, its label,
/// and its control.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RowData {
    pub section: Option<&'static str>,
    pub label: &'static str,
    pub control: Control,
}

/// A hue: its number in hex and its color, with the swatch that opens the
/// color picker.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HueData {
    pub words: String,
    pub color: String,
    pub hint: &'static str,
}

/// The control of a row, by its kind.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Control {
    Toggle {
        on: bool,
    },
    Slider {
        min: f32,
        max: f32,
        step: f32,
        value: f32,
        words: String,
    },
    Choice {
        labels: &'static [&'static str],
        index: usize,
    },
    Hue {
        hue: HueData,
    },
    Text {
        words: String,
    },
    /// A file: a name of the player fonts of `/v1/fonts` when `fonts`,
    /// else a file the server reads.
    File {
        words: String,
        hint: &'static str,
        fonts: bool,
    },
    /// Words on many lines: a list of words, or of numbers.
    Lines {
        words: String,
        hint: &'static str,
    },
    Keys(KeysData),
    InfoItems {
        items: Vec<InfoItemData>,
        data_labels: &'static [&'static str],
        hint: &'static str,
        add: &'static str,
    },
    JournalTabs {
        tabs: Vec<TabData>,
        kind_labels: &'static [&'static str],
        hint: &'static str,
        add: &'static str,
    },
    Cooldowns {
        rules: Vec<CooldownData>,
        source_labels: &'static [&'static str],
        hints: [&'static str; 2],
        restart: &'static str,
        add: &'static str,
    },
    Highlights(HighlightsData),
    CounterItems {
        items: Vec<CounterData>,
        hint: &'static str,
        add: &'static str,
    },
}

/// The macros of the Macros page.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct KeysData {
    pub macros: Vec<MacroData>,
    pub groups: Vec<ActionGroup>,
    pub defaults: Vec<DefaultList>,
    pub name_hint: &'static str,
    pub clear: &'static str,
    pub steps: &'static str,
    pub up: &'static str,
    pub down: &'static str,
    pub add_step: &'static str,
    pub add: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MacroData {
    pub name: String,
    /// The key chord in words, "No key", or "Press a key" while it waits.
    pub chord: String,
    pub pad: String,
    /// The macro has a key or buttons to clear.
    pub bound: bool,
    pub open: bool,
    pub steps: Vec<StepData>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StepData {
    pub action: String,
    pub action_id: String,
    pub argument: String,
    /// The player types the argument.
    pub typed: bool,
    pub hint: &'static str,
    pub choices: Vec<String>,
    /// The words of the list of choices: the argument, or "Pick" for a
    /// typed one.
    pub shown: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ActionGroup {
    pub label: &'static str,
    pub actions: Vec<ActionChoice>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ActionChoice {
    pub id: &'static str,
    pub label: &'static str,
}

/// The default keys or controller buttons, in words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DefaultList {
    pub title: &'static str,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InfoItemData {
    pub label: String,
    pub hue: HueData,
    pub data: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TabData {
    pub name: String,
    /// Whether the tab shows each kind of line.
    pub kinds: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CooldownData {
    pub label: String,
    pub hue: HueData,
    pub seconds: f32,
    pub trigger: String,
    pub source: usize,
    pub restart: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HighlightsData {
    pub rules: Vec<HighlightData>,
    pub presets: Vec<String>,
    pub hints: [&'static str; 2],
    pub need_all: &'static str,
    pub corpses_only: &'static str,
    pub at_least: &'static str,
    pub add_need: &'static str,
    pub add: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HighlightData {
    pub name: String,
    pub hue: HueData,
    pub need_all: bool,
    pub corpses_only: bool,
    pub needs: Vec<NeedData>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NeedData {
    pub words: String,
    pub min: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CounterData {
    pub label: String,
    pub graphic: String,
    pub hue: HueData,
}

/// The color picker a hue row opened.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ColorPickerData {
    pub grid: HueGridData,
    pub color: String,
    pub words: String,
    pub eyedropper: Colored,
    pub okay: &'static str,
    pub cancel: &'static str,
}

/// A place in a list row: the row by its label, and the entry.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct EntryAt {
    row: String,
    at: usize,
}

/// One field of an entry of a list row.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EntryField {
    row: String,
    at: usize,
    name: String,
    value: Value,
}

/// One field of a property a highlight rule needs.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct NeedField {
    row: String,
    at: usize,
    need: usize,
    name: String,
    value: Value,
}

/// A property of a highlight rule.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct NeedAt {
    row: String,
    at: usize,
    need: usize,
}

/// A step of a macro.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct StepAt {
    at: usize,
    step: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct StepWords {
    at: usize,
    step: usize,
    words: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct StepMove {
    at: usize,
    step: usize,
    up: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct MacroWords {
    at: usize,
    words: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct MacroAction {
    at: usize,
    action: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct CaptureAt {
    at: usize,
    pad: bool,
}

/// A row by its label, and its new value as the page holds it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RowValue {
    row: String,
    value: Value,
}

/// What the player did on the Options.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OptionsAction {
    Page(usize),
    Set(RowValue),
    Swatch(EntryAt),
    Add(String),
    Remove(EntryAt),
    Field(EntryField),
    Need(NeedField),
    AddNeed(EntryAt),
    RemoveNeed(NeedAt),
    Preset(usize),
    MacroName(MacroWords),
    Capture(CaptureAt),
    ClearKeys(usize),
    Steps(usize),
    StepAction(StepWords),
    StepArgument(StepWords),
    StepMove(StepMove),
    StepRemove(StepAt),
    StepAdd(MacroAction),
    Foot(usize),
}

/// What the player did on the color picker.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PickerAction {
    Cell(usize),
    Shade(i32),
    Okay(bool),
    Cancel(bool),
    Eyedropper(bool),
}

/// The row of a page by its label.
fn row_of(page: Page, label: &str) -> Option<&'static OptionRow> {
    rows_on(page).find(|row| row.label == label)
}

/// The words of a value of the page as a number.
fn number_of(value: &Value) -> Option<f32> {
    value.as_f64().map(|number| number as f32)
}

/// The words of a value of the page.
fn words_of(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// A new value of a row from what the page holds for it: the form the
/// kind of the row takes. None for a value of another form.
fn value_for(row: &OptionRow, value: &Value) -> Option<OptionValue> {
    Some(match row.kind {
        OptionKind::Toggle => OptionValue::Toggle(value.as_bool()?),
        OptionKind::Slider { min, max, step, .. } => {
            OptionValue::Number(snap(number_of(value)?, min, max, step))
        }
        OptionKind::Choice { labels } => {
            let index = usize::try_from(value.as_u64()?).ok()?;
            (index < labels.len()).then_some(OptionValue::Choice(index))?
        }
        OptionKind::Hue => OptionValue::Hue(parse_hue(value.as_str()?)?),
        OptionKind::Text => OptionValue::Text(words_of(value)?),
        OptionKind::FilePath => {
            let words = words_of(value)?;
            OptionValue::FilePath((!words.is_empty()).then(|| PathBuf::from(words)))
        }
        OptionKind::TextList => OptionValue::TextList(parse_lines(value.as_str()?)),
        OptionKind::IdList => OptionValue::Ids(parse_ids(value.as_str()?)),
        _ => return None,
    })
}

impl WebView {
    /// The hue data of a hue row or entry.
    fn hue_data(&self, hue: u16) -> HueData {
        HueData {
            words: hue_words(hue),
            color: self.hue_css(hue),
            hint: HINT_SWATCH,
        }
    }

    /// The control of a row with its value now.
    fn control(&self, page: Page, row: &OptionRow, value: OptionValue) -> Option<Control> {
        Some(match (row.kind, value) {
            (OptionKind::Toggle, OptionValue::Toggle(on)) => Control::Toggle { on },
            (
                OptionKind::Slider {
                    min,
                    max,
                    step,
                    unit,
                },
                OptionValue::Number(value),
            ) => Control::Slider {
                min,
                max,
                step,
                value,
                words: unit.format(value),
            },
            (OptionKind::Choice { labels }, OptionValue::Choice(index)) => {
                Control::Choice { labels, index }
            }
            (OptionKind::Hue, OptionValue::Hue(hue)) => Control::Hue {
                hue: self.hue_data(hue),
            },
            (OptionKind::Text, OptionValue::Text(words)) => Control::Text { words },
            (OptionKind::FilePath, OptionValue::FilePath(path)) => Control::File {
                words: path
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                hint: HINT_NO_FILE,
                fonts: page == Page::Fonts,
            },
            (OptionKind::TextList, OptionValue::TextList(lines)) => Control::Lines {
                words: lines.join(LINE_BREAK),
                hint: HINT_LINES,
            },
            (OptionKind::IdList, OptionValue::Ids(ids)) => Control::Lines {
                words: format_ids(&ids),
                hint: HINT_IDS,
            },
            (OptionKind::KeyList, OptionValue::Keys(keys)) => Control::Keys(self.keys_data(&keys)),
            (OptionKind::InfoBarItems, OptionValue::InfoBarItems(items)) => Control::InfoItems {
                items: items
                    .iter()
                    .map(|item| InfoItemData {
                        label: item.label.clone(),
                        hue: self.hue_data(item.hue),
                        data: item.data.index(),
                    })
                    .collect(),
                data_labels: InfoBarData::LABELS,
                hint: HINT_LABEL,
                add: WORDS_ADD_ITEM,
            },
            (OptionKind::JournalTabs, OptionValue::JournalTabs(tabs)) => Control::JournalTabs {
                tabs: tabs
                    .iter()
                    .map(|tab| TabData {
                        name: tab.name.clone(),
                        kinds: (0..JournalKind::LABELS.len())
                            .map(|index| tab.kinds.contains(&JournalKind::from_index(index)))
                            .collect(),
                    })
                    .collect(),
                kind_labels: JournalKind::LABELS,
                hint: HINT_TAB_NAME,
                add: WORDS_ADD_TAB,
            },
            (OptionKind::Cooldowns, OptionValue::Cooldowns(rules)) => Control::Cooldowns {
                rules: rules
                    .iter()
                    .map(|rule| CooldownData {
                        label: rule.label.clone(),
                        hue: self.hue_data(rule.hue),
                        seconds: rule.seconds,
                        trigger: rule.trigger.clone(),
                        source: rule.source.index(),
                        restart: rule.restart,
                    })
                    .collect(),
                source_labels: CooldownSource::LABELS,
                hints: [HINT_LABEL, HINT_TRIGGER],
                restart: WORDS_RESTART,
                add: WORDS_ADD_COOLDOWN,
            },
            (OptionKind::HighlightRules, OptionValue::HighlightRules(rules)) => {
                Control::Highlights(HighlightsData {
                    rules: rules
                        .iter()
                        .map(|rule| HighlightData {
                            name: rule.name.clone(),
                            hue: self.hue_data(rule.hue),
                            need_all: rule.need_all,
                            corpses_only: rule.corpses_only,
                            needs: rule
                                .needs
                                .iter()
                                .map(|need| NeedData {
                                    words: need.words.clone(),
                                    min: need.min,
                                })
                                .collect(),
                        })
                        .collect(),
                    presets: highlight::presets()
                        .into_iter()
                        .map(|preset| format!("{WORDS_ADD_PRESET} {}", preset.name))
                        .collect(),
                    hints: [HINT_RULE_NAME, HINT_PROPERTY],
                    need_all: WORDS_NEED_ALL,
                    corpses_only: WORDS_CORPSES_ONLY,
                    at_least: WORDS_AT_LEAST,
                    add_need: WORDS_ADD_NEED,
                    add: WORDS_ADD_RULE,
                })
            }
            (OptionKind::CounterItems, OptionValue::CounterItems(items)) => Control::CounterItems {
                items: items
                    .iter()
                    .map(|item| CounterData {
                        label: item.label.clone(),
                        graphic: hue_words(item.graphic),
                        hue: self.hue_data(item.hue),
                    })
                    .collect(),
                hint: HINT_LABEL,
                add: WORDS_ADD_ITEM,
            },
            _ => return None,
        })
    }

    /// The macros of the Macros page, with the capture that waits.
    fn keys_data(&self, keys: &[KeyBinding]) -> KeysData {
        let editor = &self.panels.options.panel.macros;
        let macros = keys
            .iter()
            .enumerate()
            .map(|(at, binding)| MacroData {
                name: binding.name.clone(),
                chord: match (editor.capture, &binding.chord) {
                    (Some(Capture::Chord(waiting)), _) if waiting == at => {
                        WORDS_PRESS_KEY.to_string()
                    }
                    (_, Some(chord)) => chord.to_string(),
                    (_, None) => WORDS_NO_KEY.to_string(),
                },
                pad: match (editor.capture, &binding.pad) {
                    (Some(Capture::Pad(waiting)), _) if waiting == at => {
                        WORDS_PRESS_BUTTON.to_string()
                    }
                    (_, Some(pad)) => pad.to_string(),
                    (_, None) => WORDS_NO_BUTTON.to_string(),
                },
                bound: binding.chord.is_some() || binding.pad.is_some(),
                open: editor.open == Some(at),
                steps: binding
                    .steps
                    .iter()
                    .map(|step| {
                        let kind = ActionId::from_id(&step.action).map(|a| a.spec().argument);
                        let typed = kind.is_some_and(|kind| kind.is_typed());
                        StepData {
                            action: step_words(&step.action, ""),
                            action_id: step.action.clone(),
                            argument: step.argument.clone(),
                            typed,
                            hint: kind.map_or("", |kind| kind.hint()),
                            choices: kind.map(|kind| kind.choices()).unwrap_or_default(),
                            shown: if typed {
                                WORDS_SUGGESTIONS.to_string()
                            } else {
                                step.argument.clone()
                            },
                        }
                    })
                    .collect(),
            })
            .collect();
        let groups = Group::ALL
            .into_iter()
            .map(|group| ActionGroup {
                label: group.label(),
                actions: ACTIONS
                    .iter()
                    .filter(|spec| spec.group == group)
                    .map(|spec| ActionChoice {
                        id: spec.id,
                        label: spec.label,
                    })
                    .collect(),
            })
            .collect();
        let defaults = [
            (WORDS_DEFAULT_KEYS, default_keys().collect::<Vec<_>>()),
            (WORDS_DEFAULT_BUTTONS, default_buttons().collect::<Vec<_>>()),
        ]
        .into_iter()
        .map(|(title, list)| DefaultList {
            title,
            lines: list
                .into_iter()
                .map(|(trigger, action, argument)| {
                    format!("{trigger}: {}", step_words(action, argument))
                })
                .collect(),
        })
        .collect();
        KeysData {
            macros,
            groups,
            defaults,
            name_hint: HINT_MACRO_NAME,
            clear: WORDS_CLEAR,
            steps: WORDS_STEPS,
            up: WORDS_UP,
            down: WORDS_DOWN,
            add_step: WORDS_ADD_STEP,
            add: WORDS_ADD_MACRO,
        }
    }

    pub(crate) fn options_open(&self) -> bool {
        self.panels.options.panel.open
    }

    pub(crate) fn toggle_options(&mut self) {
        self.panels.options.panel.toggle();
        self.panels.options.eyedropper.stop();
    }

    pub(super) fn options_spec(&self) -> Option<FrameSpec> {
        self.options_open().then(|| {
            let default = options_first_place(self.panel_room());
            FrameSpec::fixed(OPTIONS_ID, WORDS_OPTIONS, default)
                .closable()
                .sized(OPTIONS_LEAST)
        })
    }

    pub(super) fn options_data(&mut self) -> Option<Framed<OptionsData>> {
        let spec = self.options_spec()?;
        let draft = self.panels.options.panel.draft(&self.profile).clone();
        let page = self.panels.options.panel.page;
        let mut section = "";
        let mut rows = Vec::new();
        for row in rows_on(page) {
            let Some(control) = self.control(page, row, (row.get)(&draft.edited)) else {
                continue;
            };
            let new_section = row.section != section;
            section = row.section;
            rows.push(RowData {
                section: new_section.then_some(row.section),
                label: row.label,
                control,
            });
        }
        let changed = draft.changed();
        let body = OptionsData {
            pages: Page::LABELS
                .iter()
                .enumerate()
                .map(|(index, words)| Choice {
                    words: (*words).to_string(),
                    chosen: Page::from_index(index) == page,
                })
                .collect(),
            rows,
            foot: FOOT
                .iter()
                .map(|(foot, words, _)| Colored {
                    words: (*words).to_string(),
                    color: css_color(foot_color(*foot, changed)),
                })
                .collect(),
            default_hint: HINT_DEFAULT,
            default_at: FOOT
                .iter()
                .position(|(foot, _, _)| *foot == Foot::Default)
                .unwrap_or_default(),
            remove: WORDS_REMOVE,
        };
        Some(self.framed(PANEL_OPTIONS, &spec, body))
    }

    /// Changes the value of a row of the page that shows, on the copy.
    fn change_row(
        &mut self,
        label: &str,
        change: impl FnOnce(&mut OptionsState, OptionValue) -> Option<OptionValue>,
    ) {
        let page = self.panels.options.panel.page;
        let Some(row) = row_of(page, label) else {
            return;
        };
        let profile = self.profile.clone();
        let value = (row.get)(&self.panels.options.panel.draft(&profile).edited);
        if let Some(value) = change(&mut self.panels.options, value) {
            (row.set)(&mut self.panels.options.panel.draft(&profile).edited, value);
        }
    }

    /// Changes the macros of the Macros page with the editor. `change`
    /// gives true when it changed them.
    fn change_keys(&mut self, change: impl FnOnce(&mut MacroEditor, &mut Vec<KeyBinding>) -> bool) {
        let Some(label) = rows_on(Page::Macros)
            .find(|row| row.kind == OptionKind::KeyList)
            .map(|row| row.label)
        else {
            return;
        };
        let page = self.panels.options.panel.page;
        self.panels.options.panel.page = Page::Macros;
        self.change_row(label, |options, value| {
            let OptionValue::Keys(mut keys) = value else {
                return None;
            };
            change(&mut options.panel.macros, &mut keys).then_some(OptionValue::Keys(keys))
        });
        self.panels.options.panel.page = page;
    }

    /// True while a macro waits for its key or its controller buttons:
    /// the keys of the frame run nothing else.
    pub(crate) fn capturing_keys(&self) -> bool {
        self.panels.options.panel.capturing()
    }

    /// The key or the controller buttons a macro waits for, from the input
    /// of this frame: the first key pressed, and the buttons pressed.
    pub(crate) fn capture_keys(&mut self, presses: &[KeyPress], pad: Option<PadChord>) {
        let pressed = presses
            .iter()
            .find(|press| press.pressed)
            .map(|press| Pressed::of(&press.key, press.mods));
        self.change_keys(|editor, keys| editor.take_capture(keys, pressed, pad));
    }

    pub(super) fn options_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<OptionsAction>(action) else {
            return;
        };
        let page = self.panels.options.panel.page;
        match action {
            OptionsAction::Page(index) if index < Page::LABELS.len() => {
                self.panels
                    .options
                    .panel
                    .choose_page(Page::from_index(index));
            }
            OptionsAction::Page(_) => {}
            OptionsAction::Set(RowValue { row, value }) => {
                let Some(option) = row_of(page, &row) else {
                    return;
                };
                if let Some(value) = value_for(option, &value) {
                    self.change_row(&row, |_, _| Some(value));
                }
            }
            OptionsAction::Swatch(EntryAt { row, at }) => self.open_swatch(&row, at),
            OptionsAction::Add(row) => self.change_row(&row, |options, value| {
                Some(match value {
                    OptionValue::Keys(mut keys) => {
                        options.panel.macros.add(&mut keys);
                        OptionValue::Keys(keys)
                    }
                    OptionValue::InfoBarItems(mut items) => {
                        items.push(new_info_bar_item());
                        OptionValue::InfoBarItems(items)
                    }
                    OptionValue::JournalTabs(mut tabs) => {
                        tabs.push(new_journal_tab());
                        OptionValue::JournalTabs(tabs)
                    }
                    OptionValue::Cooldowns(mut rules) => {
                        rules.push(new_cooldown());
                        OptionValue::Cooldowns(rules)
                    }
                    OptionValue::HighlightRules(mut rules) => {
                        rules.push(highlight::blank());
                        OptionValue::HighlightRules(rules)
                    }
                    OptionValue::CounterItems(mut items) => {
                        items.push(new_counter_item());
                        OptionValue::CounterItems(items)
                    }
                    _ => return None,
                })
            }),
            OptionsAction::Remove(EntryAt { row, at }) => {
                self.change_row(&row, |options, value| remove_entry(options, value, at))
            }
            OptionsAction::Field(field) => {
                self.change_row(&field.row.clone(), |_, value| set_field(value, &field))
            }
            OptionsAction::Need(field) => {
                self.change_row(&field.row.clone(), |_, value| set_need(value, &field))
            }
            OptionsAction::AddNeed(EntryAt { row, at }) => self.change_row(&row, |_, value| {
                let OptionValue::HighlightRules(mut rules) = value else {
                    return None;
                };
                rules.get_mut(at)?.needs.push(new_property_need());
                Some(OptionValue::HighlightRules(rules))
            }),
            OptionsAction::RemoveNeed(NeedAt { row, at, need }) => {
                self.change_row(&row, |_, value| {
                    let OptionValue::HighlightRules(mut rules) = value else {
                        return None;
                    };
                    let needs = &mut rules.get_mut(at)?.needs;
                    (need < needs.len()).then(|| needs.remove(need))?;
                    Some(OptionValue::HighlightRules(rules))
                })
            }
            OptionsAction::Preset(index) => {
                let Some(label) = rows_on(page)
                    .find(|row| row.kind == OptionKind::HighlightRules)
                    .map(|row| row.label)
                else {
                    return;
                };
                self.change_row(label, |_, value| {
                    let OptionValue::HighlightRules(mut rules) = value else {
                        return None;
                    };
                    rules.push(highlight::presets().into_iter().nth(index)?);
                    Some(OptionValue::HighlightRules(rules))
                })
            }
            OptionsAction::MacroName(MacroWords { at, words }) => self.change_keys(|_, keys| {
                keys.get_mut(at)
                    .map(|binding| binding.name = words)
                    .is_some()
            }),
            OptionsAction::Capture(CaptureAt { at, pad }) => {
                let capture = if pad {
                    Capture::Pad(at)
                } else {
                    Capture::Chord(at)
                };
                self.panels.options.panel.macros.capture(capture);
            }
            OptionsAction::ClearKeys(at) => self.change_keys(|_, keys| {
                keys.get_mut(at)
                    .map(|binding| {
                        binding.chord = None;
                        binding.pad = None;
                    })
                    .is_some()
            }),
            OptionsAction::Steps(at) => {
                let editor = &mut self.panels.options.panel.macros;
                editor.open = (editor.open != Some(at)).then_some(at);
            }
            OptionsAction::StepAction(StepWords { at, step, words }) => {
                let Some(action) = ActionId::from_id(&words) else {
                    return;
                };
                self.change_keys(|_, keys| {
                    let known = keys
                        .get(at)
                        .is_some_and(|binding| step < binding.steps.len());
                    if known {
                        MacroEditor::set_action(keys, at, step, action);
                    }
                    known
                })
            }
            OptionsAction::StepArgument(StepWords { at, step, words }) => {
                self.change_keys(|_, keys| {
                    keys.get_mut(at)
                        .and_then(|binding| binding.steps.get_mut(step))
                        .map(|step| step.argument = words)
                        .is_some()
                })
            }
            OptionsAction::StepMove(StepMove { at, step, up }) => {
                let way = if up { Move::Up } else { Move::Down };
                self.change_keys(|_, keys| MacroEditor::move_step(keys, at, step, way))
            }
            OptionsAction::StepRemove(StepAt { at, step }) => self.change_keys(|_, keys| {
                let known = keys
                    .get(at)
                    .is_some_and(|binding| step < binding.steps.len());
                if known {
                    MacroEditor::remove_step(keys, at, step);
                }
                known
            }),
            OptionsAction::StepAdd(MacroAction { at, action }) => {
                let Some(action) = ActionId::from_id(&action) else {
                    return;
                };
                self.change_keys(|_, keys| {
                    let known = at < keys.len();
                    if known {
                        MacroEditor::add_step(keys, at, action);
                    }
                    known
                })
            }
            OptionsAction::Foot(at) => {
                if let Some((foot, _, _)) = FOOT.get(at) {
                    self.press_foot(*foot);
                }
            }
        }
    }

    /// Does what a button of the foot does, as the Rust window does: the
    /// profile is kept, the full screen follows the window mode, and Save
    /// as default keeps the start of new characters.
    fn press_foot(&mut self, foot: Foot) {
        let mode_before = self.profile.video.window_mode;
        let done = self
            .panels
            .options
            .panel
            .press_foot(foot, &mut self.profile);
        if !self.panels.options.panel.open {
            self.panels.options.eyedropper.stop();
        }
        if !done.applied {
            return;
        }
        self.keep_profile();
        let mode = self.profile.video.window_mode;
        if mode != mode_before {
            self.hand.push(OutCall::Fullscreen {
                on: mode == WindowMode::Fullscreen,
            });
        }
        if done.save_as_default {
            let saving = places::for_saving(&self.profile);
            if let Ok(profile) = serde_json::to_value(kept(&saving, self.kept_style)) {
                self.hand.push(OutCall::SaveDefaultProfile { profile });
            }
        }
    }

    /// The swatch of a hue opens the color picker on it.
    fn open_swatch(&mut self, label: &str, at: usize) {
        let page = self.panels.options.panel.page;
        let Some(row) = row_of(page, label) else {
            return;
        };
        let profile = self.profile.clone();
        let value = (row.get)(&self.panels.options.panel.draft(&profile).edited);
        let hue = match value {
            OptionValue::Hue(hue) => Some(hue),
            OptionValue::InfoBarItems(items) => items.get(at).map(|item| item.hue),
            OptionValue::Cooldowns(rules) => rules.get(at).map(|rule| rule.hue),
            OptionValue::HighlightRules(rules) => rules.get(at).map(|rule| rule.hue),
            OptionValue::CounterItems(items) => items.get(at).map(|item| item.hue),
            _ => None,
        };
        if let Some(hue) = hue {
            let key: HueKey = (page, row.label, at);
            self.panels.options.panel.hues.open(key, hue);
            self.panels.options.eyedropper.stop();
        }
    }

    /// The hue the picker gave goes to the row that opened it.
    fn take_picked_hue(&mut self, key: HueKey) {
        let (page, label, at) = key;
        let Some(hue) = self.panels.options.panel.hues.take_picked(key) else {
            return;
        };
        let shown = self.panels.options.panel.page;
        self.panels.options.panel.page = page;
        self.change_row(label, |_, value| set_hue(value, at, hue));
        self.panels.options.panel.page = shown;
    }

    /// The color picker in one frame: the hue the eyedropper clicked.
    pub(crate) fn follow_options(&mut self, frame: &uoterm_view::frame::WatchFrame) {
        let aiming = self.hand.aiming();
        let options = &mut self.panels.options;
        let Some((_, pick)) = options.panel.hues.open.as_mut() else {
            return;
        };
        let hand = &mut self.hand;
        let picked = || hand.take_picked(LocalAim::PickThing);
        if let Some(words) = options.eyedropper.follow(aiming, picked, pick, frame) {
            self.hand.report(words);
        }
    }

    pub(super) fn color_picker_spec(&self) -> Option<FrameSpec> {
        self.panels.options.panel.hues.open.as_ref()?;
        let default = picker_first_place(self.panel_room());
        Some(FrameSpec::fixed(PICKER_ID, WORDS_COLOR, default).closable())
    }

    pub(super) fn color_picker_data(&self) -> Option<Framed<ColorPickerData>> {
        let spec = self.color_picker_spec()?;
        let (_, pick) = self.panels.options.panel.hues.open?;
        let picking = self.panels.options.eyedropper.picking;
        let body = ColorPickerData {
            grid: self.hue_grid(pick),
            color: self.hue_css(pick.hue()),
            words: hue_words(pick.hue()),
            eyedropper: Colored {
                words: WORDS_EYEDROPPER.to_string(),
                color: css_color(if picking { WAITING } else { TEXT }),
            },
            okay: WORDS_OKAY,
            cancel: WORDS_CANCEL,
        };
        Some(self.framed(PANEL_COLOR_PICKER, &spec, body))
    }

    pub(super) fn color_picker_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<PickerAction>(action) else {
            return;
        };
        let options = &mut self.panels.options;
        let Some((key, pick)) = options.panel.hues.open.as_mut() else {
            return;
        };
        let key = *key;
        match action {
            PickerAction::Cell(index) if index < GRID_ROWS * GRID_COLUMNS => pick.index = index,
            PickerAction::Shade(shade) => pick.graduation = HueGridData::shade_in_range(shade),
            PickerAction::Okay(_) | PickerAction::Cancel(_) => {
                options.eyedropper.stop();
                options
                    .panel
                    .hues
                    .finish(matches!(action, PickerAction::Okay(_)));
                self.take_picked_hue(key);
            }
            PickerAction::Eyedropper(_) => {
                let Some(frame) = self.frame.clone() else {
                    return;
                };
                if let Some(act) = options.eyedropper.press(&frame) {
                    self.hand.act(act);
                }
                self.hand.aim(LocalAim::PickThing);
            }
            PickerAction::Cell(_) => {}
        }
    }

    /// The close mark of the color picker: Cancel.
    pub(super) fn close_color_picker(&mut self) {
        self.color_picker_action(serde_json::json!({ "cancel": true }));
    }

    /// The close mark of the Options.
    pub(crate) fn close_options(&mut self) {
        self.panels.options.panel.close();
        self.panels.options.eyedropper.stop();
    }
}

/// A hue set in a hue row, or in the entry `at` of a list row.
fn set_hue(value: OptionValue, at: usize, hue: u16) -> Option<OptionValue> {
    Some(match value {
        OptionValue::Hue(_) => OptionValue::Hue(hue),
        OptionValue::InfoBarItems(mut items) => {
            items.get_mut(at)?.hue = hue;
            OptionValue::InfoBarItems(items)
        }
        OptionValue::Cooldowns(mut rules) => {
            rules.get_mut(at)?.hue = hue;
            OptionValue::Cooldowns(rules)
        }
        OptionValue::HighlightRules(mut rules) => {
            rules.get_mut(at)?.hue = hue;
            OptionValue::HighlightRules(rules)
        }
        OptionValue::CounterItems(mut items) => {
            items.get_mut(at)?.hue = hue;
            OptionValue::CounterItems(items)
        }
        _ => return None,
    })
}

/// A list row without its entry `at`. A macro goes by the editor.
fn remove_entry(options: &mut OptionsState, value: OptionValue, at: usize) -> Option<OptionValue> {
    fn without<T>(mut items: Vec<T>, at: usize) -> Option<Vec<T>> {
        (at < items.len()).then(|| items.remove(at))?;
        Some(items)
    }
    Some(match value {
        OptionValue::Keys(mut keys) => {
            (at < keys.len()).then_some(())?;
            options.panel.macros.remove(&mut keys, at);
            OptionValue::Keys(keys)
        }
        OptionValue::InfoBarItems(items) => OptionValue::InfoBarItems(without(items, at)?),
        OptionValue::JournalTabs(tabs) => OptionValue::JournalTabs(without(tabs, at)?),
        OptionValue::Cooldowns(rules) => OptionValue::Cooldowns(without(rules, at)?),
        OptionValue::HighlightRules(rules) => OptionValue::HighlightRules(without(rules, at)?),
        OptionValue::CounterItems(items) => OptionValue::CounterItems(without(items, at)?),
        _ => return None,
    })
}

/// A field of the entry of a list row set to what the page holds.
fn set_field(value: OptionValue, field: &EntryField) -> Option<OptionValue> {
    let EntryField {
        at,
        name,
        value: new,
        ..
    } = field;
    let index = || new.as_u64().and_then(|index| usize::try_from(index).ok());
    Some(match value {
        OptionValue::InfoBarItems(mut items) => {
            let item = items.get_mut(*at)?;
            match name.as_str() {
                "label" => item.label = words_of(new)?,
                "hue" => item.hue = parse_hue(new.as_str()?)?,
                "data" => {
                    let index = index().filter(|index| *index < InfoBarData::LABELS.len())?;
                    item.data = InfoBarData::from_index(index);
                }
                _ => return None,
            }
            OptionValue::InfoBarItems(items)
        }
        OptionValue::JournalTabs(mut tabs) => {
            let tab = tabs.get_mut(*at)?;
            match name.as_str() {
                "name" => tab.name = words_of(new)?,
                kind => {
                    let index: usize = kind.strip_prefix("kind:")?.parse().ok()?;
                    let kind = (index < JournalKind::LABELS.len())
                        .then(|| JournalKind::from_index(index))?;
                    tab.kinds.retain(|other| *other != kind);
                    if new.as_bool()? {
                        tab.kinds.push(kind);
                    }
                }
            }
            OptionValue::JournalTabs(tabs)
        }
        OptionValue::Cooldowns(mut rules) => {
            let rule = rules.get_mut(*at)?;
            match name.as_str() {
                "label" => rule.label = words_of(new)?,
                "hue" => rule.hue = parse_hue(new.as_str()?)?,
                "seconds" => {
                    rule.seconds = number_of(new)?.clamp(
                        uoterm_view::ui::options::COOLDOWN_SECONDS_MIN,
                        uoterm_view::ui::options::COOLDOWN_SECONDS_MAX,
                    )
                }
                "trigger" => rule.trigger = words_of(new)?,
                "source" => {
                    let index = index().filter(|index| *index < CooldownSource::LABELS.len())?;
                    rule.source = CooldownSource::from_index(index);
                }
                "restart" => rule.restart = new.as_bool()?,
                _ => return None,
            }
            OptionValue::Cooldowns(rules)
        }
        OptionValue::HighlightRules(mut rules) => {
            let rule = rules.get_mut(*at)?;
            match name.as_str() {
                "name" => rule.name = words_of(new)?,
                "hue" => rule.hue = parse_hue(new.as_str()?)?,
                "need_all" => rule.need_all = new.as_bool()?,
                "corpses_only" => rule.corpses_only = new.as_bool()?,
                _ => return None,
            }
            OptionValue::HighlightRules(rules)
        }
        OptionValue::CounterItems(mut items) => {
            let item = items.get_mut(*at)?;
            match name.as_str() {
                "label" => item.label = words_of(new)?,
                "graphic" => item.graphic = parse_hue(new.as_str()?)?,
                "hue" => item.hue = parse_hue(new.as_str()?)?,
                _ => return None,
            }
            OptionValue::CounterItems(items)
        }
        _ => return None,
    })
}

/// A field of a property a highlight rule needs: its words, whether it
/// has a least number, and the number.
fn set_need(value: OptionValue, field: &NeedField) -> Option<OptionValue> {
    let OptionValue::HighlightRules(mut rules) = value else {
        return None;
    };
    let need = rules.get_mut(field.at)?.needs.get_mut(field.need)?;
    match field.name.as_str() {
        "words" => need.words = words_of(&field.value)?,
        "at_least" => need.min = at_least(field.value.as_bool()?),
        "min" => need.min = Some(number_of(&field.value)?).filter(|_| need.min.is_some()),
        _ => return None,
    }
    Some(OptionValue::HighlightRules(rules))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{press, saved_profiles, view_with};
    use super::*;
    use serde_json::json;
    use uoterm_view::input::{KeyName, Mods};
    use uoterm_view::settings::{KeyChord, Profile};

    const APPLY: usize = 1;
    const DEFAULT: usize = 2;
    const OKAY: usize = 3;
    const SAVE_AS_DEFAULT: usize = 4;
    const ALWAYS_RUN: &str = "Always run";

    fn options_open() -> WebView {
        let mut view = view_with("human_control", json!(true), true);
        view.toggle_options();
        view
    }

    #[test]
    fn a_change_waits_on_the_copy_until_apply_keeps_the_profile() {
        let mut view = options_open();
        assert!(
            row_of(Page::General, ALWAYS_RUN).is_some(),
            "a row of the table"
        );
        let out = press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "set": { "row": ALWAYS_RUN, "value": true } }),
        );
        assert!(saved_profiles(&out).is_empty(), "not applied yet");
        assert!(!view.profile.general.always_run);
        let out = press(&mut view, PANEL_OPTIONS, json!({ "foot": APPLY }));
        assert!(saved_profiles(&out)[0].general.always_run);
        assert!(view.options_open());
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "set": { "row": ALWAYS_RUN, "value": false } }),
        );
        press(&mut view, PANEL_OPTIONS, json!({ "foot": DEFAULT }));
        let out = press(&mut view, PANEL_OPTIONS, json!({ "foot": OKAY }));
        assert_eq!(
            saved_profiles(&out)[0].general.always_run,
            Profile::default().general.always_run
        );
        assert!(!view.options_open(), "Okay closes the panel");
    }

    #[test]
    fn save_as_default_keeps_the_start_of_new_characters() {
        let mut view = options_open();
        let out = press(&mut view, PANEL_OPTIONS, json!({ "foot": SAVE_AS_DEFAULT }));
        assert!(out
            .iter()
            .any(|call| matches!(call, OutCall::SaveDefaultProfile { .. })));
    }

    #[test]
    fn full_screen_in_the_window_mode_asks_the_page_for_it() {
        let mut view = options_open();
        let video = Page::Video.index();
        press(&mut view, PANEL_OPTIONS, json!({ "page": video }));
        let fullscreen = WindowMode::Fullscreen.index();
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "set": { "row": "Window mode", "value": fullscreen } }),
        );
        let out = press(&mut view, PANEL_OPTIONS, json!({ "foot": APPLY }));
        assert!(out.contains(&OutCall::Fullscreen { on: true }));
    }

    #[test]
    fn a_key_chord_is_captured_from_the_input_as_the_window_captures_it() {
        let mut view = options_open();
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "page": Page::Macros.index() }),
        );
        let keys = rows_on(Page::Macros)
            .find(|row| row.kind == OptionKind::KeyList)
            .unwrap()
            .label;
        press(&mut view, PANEL_OPTIONS, json!({ "add": keys }));
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "capture": { "at": 0, "pad": false } }),
        );
        let ctrl = Mods {
            ctrl: true,
            command: true,
            ..Mods::default()
        };
        let f1 = KeyPress {
            key: KeyName("F1".into()),
            mods: ctrl,
            pressed: true,
            repeat: false,
        };
        assert!(view.capturing_keys(), "the keys run nothing else");
        view.capture_keys(&[f1], None);
        let draft = view.panels.options.panel.draft_now().unwrap();
        let expected: KeyChord = "Ctrl+F1".parse().unwrap();
        assert_eq!(
            draft.edited.macros.key_bindings.last().unwrap().chord,
            Some(expected)
        );
        assert!(!view.capturing_keys(), "the capture is done");
        let data = view.panel_data(0.0).options.unwrap().body;
        let shown = data.rows.iter().find_map(|row| match &row.control {
            Control::Keys(keys) => keys.macros.last().map(|m| m.chord.clone()),
            _ => None,
        });
        assert_eq!(shown.as_deref(), Some("Ctrl+F1"));
    }

    #[test]
    fn the_steps_of_a_macro_change_by_the_editor() {
        let mut view = options_open();
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "page": Page::Macros.index() }),
        );
        let keys = rows_on(Page::Macros)
            .find(|row| row.kind == OptionKind::KeyList)
            .unwrap()
            .label;
        press(&mut view, PANEL_OPTIONS, json!({ "add": keys }));
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "step_add": { "at": 0, "action": "say" } }),
        );
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "step_argument": { "at": 0, "step": 0, "words": "hail" } }),
        );
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "macro_name": { "at": 0, "words": "greet" } }),
        );
        let out = press(&mut view, PANEL_OPTIONS, json!({ "foot": OKAY }));
        let binding = saved_profiles(&out)[0].macros.key_bindings[0].clone();
        assert_eq!(binding.name, "greet");
        assert_eq!(
            (
                binding.steps[0].action.as_str(),
                binding.steps[0].argument.as_str()
            ),
            ("say", "hail")
        );
    }

    #[test]
    fn a_picked_hue_goes_to_its_entry_and_lists_grow_by_their_rules() {
        let mut view = options_open();
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "page": Page::InfoBar.index() }),
        );
        let items = rows_on(Page::InfoBar)
            .find(|row| row.kind == OptionKind::InfoBarItems)
            .unwrap()
            .label;
        let before = view.profile.info_bar.items.len();
        press(&mut view, PANEL_OPTIONS, json!({ "add": items }));
        press(
            &mut view,
            PANEL_OPTIONS,
            json!({ "swatch": { "row": items, "at": before } }),
        );
        assert!(view.panel_data(0.0).color_picker.is_some());
        press(&mut view, PANEL_COLOR_PICKER, json!({ "cell": 3 }));
        let hue = view.panels.options.panel.hues.open.unwrap().1.hue();
        press(&mut view, PANEL_COLOR_PICKER, json!({ "okay": true }));
        assert!(view.panel_data(0.0).color_picker.is_none());
        let draft = view.panels.options.panel.draft_now().unwrap();
        assert_eq!(draft.edited.info_bar.items[before].hue, hue);
        assert_eq!(
            draft.edited.info_bar.items[before].data,
            new_info_bar_item().data
        );
    }
}
