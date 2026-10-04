//! The Options screen of the Modern style, as both windows run it. It
//! shows every row of the option table, page by page. As the classic
//! Options gump does, it edits a copy of the profile: Apply and Okay make
//! it the profile, Cancel lets it go, Default puts the page back to its
//! defaults, and "Save as default" makes it the start of each new
//! character. A hue row opens the color picker: the grid of hues with its
//! shade slider and the eyedropper. The rows are `settings::table`'s and
//! the copy is `model::options_draft`'s; here are the words, the places,
//! the new entries of the lists, and what the buttons do.

use super::hues::picker_size;
use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::{GOAL, PANEL_PAD, ROW_GAP, TEXT};
use crate::actions::editor::MacroEditor;
use crate::geom::{Area, Rgba, Vector};
use crate::model::hue_grid::HuePick;
use crate::model::options_draft::Draft;
use crate::settings::{
    CooldownRule, CooldownSource, CounterItem, InfoBarData, InfoBarItem, JournalTab, Page, Profile,
    PropertyNeed, DEFAULT_COOLDOWN_SECONDS, NO_HUE,
};

pub const OPTIONS_ID: &str = "modern:options";
pub const PICKER_ID: &str = "modern:color_picker";
pub const OPTIONS_SIZE: Vector = Vector::new(800.0, 600.0);
pub const OPTIONS_LEAST: Vector = Vector::new(560.0, 360.0);
pub const PAGE_LIST_WIDTH: f32 = 170.0;
pub const PAGE_ROW: f32 = 24.0;
pub const OPTIONS_FOOT_ROW: f32 = 30.0;
pub const FOOT_BUTTON_WIDTH: f32 = 84.0;
const SAVE_DEFAULT_WIDTH: f32 = 130.0;
/// The swatch of the hue the color picker holds.
pub const PICKED_SWATCH: Vector = Vector::new(64.0, 40.0);
/// The digits of a hue in hex.
pub const HUE_DIGITS: usize = 4;
pub const HEX_PREFIX: &str = "0x";
const HEX_RADIX: u32 = 16;
const DECIMAL_BASE: f32 = 10.0;
const ID_SEPARATORS: [char; 4] = [',', ' ', '\n', ';'];
const ID_JOIN: &str = ", ";
pub const LINE_BREAK: &str = "\n";
pub const FIRST_PAGE: Page = Page::General;

pub const WORDS_OPTIONS: &str = "Options";
pub const WORDS_CANCEL: &str = "Cancel";
const WORDS_APPLY: &str = "Apply";
const WORDS_DEFAULT: &str = "Default";
pub const WORDS_OKAY: &str = "Okay";
pub const WORDS_COLOR: &str = "Color";
pub const HINT_DEFAULT: &str = "Puts this page back to its defaults.";
pub const HINT_SWATCH: &str = "Pick the color.";
pub const WORDS_SAVE_DEFAULT: &str = "Save as default";
pub const WORDS_PRESS_KEY: &str = "Press a key";
pub const WORDS_PRESS_BUTTON: &str = "Press a button";
pub const WORDS_NO_KEY: &str = "No key";
pub const WORDS_NO_BUTTON: &str = "No button";
pub const WORDS_CLEAR: &str = "Clear";
pub const WORDS_STEPS: &str = "Steps";
pub const WORDS_ADD_MACRO: &str = "Add macro";
pub const WORDS_ADD_STEP: &str = "Add step";
pub const WORDS_UP: &str = "Up";
pub const WORDS_DOWN: &str = "Down";
pub const WORDS_DEFAULT_KEYS: &str = "Default keys (the Experimental page turns them off)";
pub const WORDS_DEFAULT_BUTTONS: &str = "Default controller buttons";
pub const WORDS_SUGGESTIONS: &str = "Pick";
pub const WORDS_ADD_ITEM: &str = "Add item";
pub const WORDS_ADD_TAB: &str = "Add tab";
pub const WORDS_REMOVE: &str = "Remove";
pub const WORDS_NEW_TAB: &str = "New tab";
pub const WORDS_ADD_COOLDOWN: &str = "Add cooldown bar";
pub const WORDS_ADD_RULE: &str = "Add rule";
pub const WORDS_ADD_NEED: &str = "Add property";
pub const WORDS_ADD_PRESET: &str = "Add";
pub const WORDS_RESTART: &str = "Start again when it comes again";
pub const WORDS_NEED_ALL: &str = "Needs every property";
pub const WORDS_CORPSES_ONLY: &str = "Corpses only";
pub const WORDS_AT_LEAST: &str = "at least";
pub const HINT_MACRO_NAME: &str = "macro name";
pub const HINT_LABEL: &str = "label";
pub const HINT_TAB_NAME: &str = "tab name";
pub const HINT_NO_FILE: &str = "no file";
pub const HINT_IDS: &str = "0x0123, 0x0456";
pub const HINT_LINES: &str = "one on each line";
pub const HINT_TRIGGER: &str = "words in the journal";
pub const HINT_RULE_NAME: &str = "rule name";
pub const HINT_PROPERTY: &str = "property words";
pub const SECONDS_SUFFIX: &str = " s";
pub const COOLDOWN_SECONDS_MIN: f32 = 0.1;
pub const COOLDOWN_SECONDS_MAX: f32 = 3600.0;
pub const COOLDOWN_SECONDS_STEP: f64 = 0.1;
pub const NEW_INFO_BAR_DATA: InfoBarData = InfoBarData::HitPoints;

/// The buttons at the foot of the Options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foot {
    Cancel,
    Apply,
    Default,
    Okay,
    SaveAsDefault,
}

pub const FOOT: [(Foot, &str, f32); 5] = [
    (Foot::Cancel, WORDS_CANCEL, FOOT_BUTTON_WIDTH),
    (Foot::Apply, WORDS_APPLY, FOOT_BUTTON_WIDTH),
    (Foot::Default, WORDS_DEFAULT, FOOT_BUTTON_WIDTH),
    (Foot::Okay, WORDS_OKAY, FOOT_BUTTON_WIDTH),
    (Foot::SaveAsDefault, WORDS_SAVE_DEFAULT, SAVE_DEFAULT_WIDTH),
];

/// The color of a button of the foot: Apply and Okay light up while the
/// copy holds changes.
pub fn foot_color(foot: Foot, changed: bool) -> Rgba {
    match foot {
        Foot::Apply | Foot::Okay if changed => GOAL,
        _ => TEXT,
    }
}

/// The numbers of a list field: hex with `0x`, or decimal. Words that are
/// not numbers are left out.
pub fn parse_ids(words: &str) -> Vec<u16> {
    words
        .split(ID_SEPARATORS)
        .map(str::trim)
        .filter_map(|word| match word.strip_prefix(HEX_PREFIX) {
            Some(hex) => u16::from_str_radix(hex, HEX_RADIX).ok(),
            None => word.parse().ok(),
        })
        .collect()
}

pub fn format_ids(ids: &[u16]) -> String {
    ids.iter()
        .map(|id| hue_words(*id))
        .collect::<Vec<_>>()
        .join(ID_JOIN)
}

/// A hue or a graphic number in hex, as the rows show it.
pub fn hue_words(number: u16) -> String {
    format!("{HEX_PREFIX}{number:0width$X}", width = HUE_DIGITS)
}

/// A hue or a graphic number from hex words, with or without `0x`.
pub fn parse_hue(words: &str) -> Option<u16> {
    let words = words.trim();
    u16::from_str_radix(words.strip_prefix(HEX_PREFIX).unwrap_or(words), HEX_RADIX).ok()
}

/// The lines of a list field, with no empty ones.
pub fn parse_lines(words: &str) -> Vec<String> {
    words
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// The slider value nearest `value` that is a whole number of steps from
/// `min`, rounded to the decimals of the step, so 0.81 is kept as 0.81.
pub fn snap(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let stepped = ((value - min) / step).round() * step + min;
    let scale = DECIMAL_BASE.powf((-step.log10().floor()).max(0.0));
    ((stepped * scale).round() / scale).clamp(min, max)
}

/// A new item of the info bar.
pub fn new_info_bar_item() -> InfoBarItem {
    InfoBarItem {
        label: String::new(),
        hue: NO_HUE,
        data: NEW_INFO_BAR_DATA,
    }
}

/// A new tab of the journal.
pub fn new_journal_tab() -> JournalTab {
    JournalTab {
        name: WORDS_NEW_TAB.to_string(),
        kinds: Vec::new(),
    }
}

/// A new cooldown bar.
pub fn new_cooldown() -> CooldownRule {
    CooldownRule {
        label: String::new(),
        hue: NO_HUE,
        trigger: String::new(),
        seconds: DEFAULT_COOLDOWN_SECONDS,
        source: CooldownSource::Anyone,
        restart: true,
    }
}

/// A new property a highlight rule needs.
pub fn new_property_need() -> PropertyNeed {
    PropertyNeed {
        words: String::new(),
        min: None,
    }
}

/// The least number of a property, when "at least" is on: it starts at
/// zero.
pub fn at_least(on: bool) -> Option<f32> {
    on.then_some(0.0)
}

/// A new item of the counter bar.
pub fn new_counter_item() -> CounterItem {
    CounterItem {
        label: String::new(),
        graphic: 0,
        hue: NO_HUE,
    }
}

/// Where the Options first stand in `window`.
pub fn options_first_place(window: Area) -> Area {
    first_place(window, Spot::Middle(0), OPTIONS_SIZE)
}

/// Where the color picker first stands in `window`.
pub fn picker_first_place(window: Area) -> Area {
    let size = picker_size()
        + Vector::new(
            ROW_GAP * 2.0 + PICKED_SWATCH.x,
            TITLE_ROW + OPTIONS_FOOT_ROW + ROW_GAP,
        )
        + Vector::splat(PANEL_PAD * 2.0);
    first_place(window, Spot::Middle(0), size)
}

/// Where a hue row sits: the page, the row and a place inside the row.
pub type HueKey = (Page, &'static str, usize);

/// The hue rows of the pages, the color picker one of them opened with
/// its pick, and the hue it picked until its row takes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HueRows {
    pub open: Option<(HueKey, HuePick)>,
    picked: Option<(HueKey, u16)>,
}

impl HueRows {
    /// The swatch of a row opens the color picker on its hue.
    pub fn open(&mut self, key: HueKey, hue: u16) {
        self.open = Some((key, HuePick::of(hue)));
    }

    /// The hue the picker gave the row `key`, when it gave one.
    pub fn take_picked(&mut self, key: HueKey) -> Option<u16> {
        self.picked
            .take_if(|(at, _)| *at == key)
            .map(|(_, hue)| hue)
    }

    /// The picker closes: with Okay its hue goes to its row.
    pub fn finish(&mut self, okay: bool) {
        if let Some((key, pick)) = self.open.take() {
            if okay {
                self.picked = Some((key, pick.hue()));
            }
        }
    }
}

/// What a press of the foot did, for the window to finish: the profile
/// was made from the copy, and it is also the start of new characters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FootDone {
    pub applied: bool,
    pub save_as_default: bool,
}

/// The Options: open or not, the page that shows, the copy of the profile
/// being edited, the macro editor of the Macros page and the hue rows.
#[derive(Clone, Debug)]
pub struct OptionsPanel {
    pub open: bool,
    pub page: Page,
    pub draft: Option<Draft>,
    pub macros: MacroEditor,
    pub hues: HueRows,
}

impl Default for OptionsPanel {
    fn default() -> Self {
        Self {
            open: false,
            page: FIRST_PAGE,
            draft: None,
            macros: MacroEditor::default(),
            hues: HueRows::default(),
        }
    }
}

impl OptionsPanel {
    /// Opens the panel, or closes it.
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open = true;
        }
    }

    /// Closes the panel: the copy, a capture and the picker go.
    pub fn close(&mut self) {
        self.open = false;
        self.draft = None;
        self.macros.cancel_capture();
        self.hues = HueRows::default();
    }

    /// Waits for the key or the controller buttons of a macro. The keys of
    /// the window do not run while it waits.
    pub fn capturing(&self) -> bool {
        self.open && self.macros.capture.is_some()
    }

    /// The copy being edited, made from `profile` when the panel opened.
    pub fn draft(&mut self, profile: &Profile) -> &mut Draft {
        self.draft.get_or_insert_with(|| Draft::of(profile))
    }

    /// The copy, when the panel holds one.
    pub fn draft_now(&self) -> Option<&Draft> {
        self.draft.as_ref()
    }

    /// Shows another page; a capture of the last one lets go.
    pub fn choose_page(&mut self, page: Page) {
        self.page = page;
        self.macros.cancel_capture();
    }

    /// Does what a button of the foot does to `profile` and to the copy.
    /// Cancel and Okay close the panel.
    pub fn press_foot(&mut self, foot: Foot, profile: &mut Profile) -> FootDone {
        let page = self.page;
        let draft = self.draft(profile);
        let mut done = FootDone::default();
        match foot {
            Foot::Default => draft.reset_page(page),
            Foot::Apply | Foot::Okay | Foot::SaveAsDefault => {
                draft.apply(profile);
                done.applied = true;
                done.save_as_default = foot == Foot::SaveAsDefault;
            }
            Foot::Cancel => {}
        }
        if matches!(foot, Foot::Cancel | Foot::Okay) {
            self.close();
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_edits_a_copy_that_only_apply_makes_the_profile() {
        let mut options = OptionsPanel::default();
        let mut profile = Profile::default();
        options.toggle();
        let draft = options.draft(&profile);
        draft.edited.general.always_run = !profile.general.always_run;
        assert_eq!(profile, Profile::default(), "not applied yet");
        let done = options.press_foot(Foot::Apply, &mut profile);
        assert!(done.applied && !done.save_as_default);
        assert_ne!(
            profile.general.always_run,
            Profile::default().general.always_run
        );
        assert!(options.open, "Apply leaves the panel open");
        options.press_foot(Foot::Okay, &mut profile);
        assert!(!options.open && options.draft_now().is_none());
        options.toggle();
        options.draft(&profile).edited.general.always_run = false;
        options.press_foot(Foot::Default, &mut profile);
        assert_eq!(
            options.draft(&profile).edited.general.always_run,
            Profile::default().general.always_run
        );
        options.press_foot(Foot::Cancel, &mut profile);
        assert!(!options.open);
    }

    #[test]
    fn a_picked_hue_goes_to_its_own_row() {
        let mut rows = HueRows::default();
        let emote = (Page::Speech, "Emote", 0);
        rows.open(emote, 0x0035);
        rows.finish(true);
        assert_eq!(rows.take_picked((Page::Speech, "Speech", 0)), None);
        assert_eq!(rows.take_picked(emote), Some(0x0035));
        assert_eq!(rows.take_picked(emote), None);
        rows.open(emote, 1);
        rows.finish(false);
        assert_eq!(rows.take_picked(emote), None, "Cancel gives nothing");
    }

    #[test]
    fn a_list_of_ids_reads_hex_and_decimal_and_skips_other_words() {
        assert_eq!(
            parse_ids("0x0123, 45;x 0x00FF\n7"),
            vec![0x0123, 45, 0x00FF, 7]
        );
        assert_eq!(parse_ids(&format_ids(&[1, 0xABCD])), vec![1, 0xABCD]);
        assert!(parse_ids("").is_empty());
        assert_eq!(parse_hue(&hue_words(0x0035)), Some(0x0035));
        assert_eq!(parse_hue("21"), Some(0x21));
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
}
