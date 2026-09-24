//! The option table: one row for each option a player sets, in the order
//! the Options screen shows them. A row names its page, its section and its
//! label, says what kind of control it needs, and reads and writes its
//! value in the profile. The Classic and the Modern Options screens both
//! draw from this table, so neither knows the fields of the profile.

use super::choices::*;
use super::keys::KeyBinding;
use super::pages::{CooldownRule, CounterItem, HighlightRule, InfoBarItem, JournalTab};
use super::Profile;
use crate::window::figure::WORN_LAYERS;
use crate::window::model::world_map::ZOOMS;
use crate::window::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
use std::path::PathBuf;

const VOLUME_MIN: f32 = 0.0;
const VOLUME_MAX: f32 = 1.0;
const VOLUME_STEP: f32 = 0.01;
const PERCENT_STEP: f32 = 1.0;
const WHOLE_STEP: f32 = 1.0;
const PERCENT_FULL: f32 = 100.0;
const SCREEN_MAX_WIDTH: f32 = 7680.0;
const SCREEN_MAX_HEIGHT: f32 = 4320.0;
const GAME_WINDOW_MIN_WIDTH: f32 = 320.0;
const GAME_WINDOW_MIN_HEIGHT: f32 = 240.0;
const CORPSE_RANGE_MIN: f32 = 1.0;
const CORPSE_RANGE_MAX: f32 = 5.0;
const CIRCLE_RADIUS_MIN: f32 = 50.0;
const CIRCLE_RADIUS_MAX: f32 = 200.0;
const SKILL_CHANGE_MIN: f32 = 1.0;
const SKILL_CHANGE_MAX: f32 = 100.0;
const MAX_SOUNDS_MIN: f32 = 1.0;
const MAX_SOUNDS_MAX: f32 = 64.0;
const FPS_MIN: f32 = 12.0;
const FPS_MAX: f32 = 250.0;
const INACTIVE_FPS_MIN: f32 = 1.0;
const UI_SCALE_MIN: f32 = 0.5;
const UI_SCALE_MAX: f32 = 3.0;
const UI_SCALE_STEP: f32 = 0.05;
const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 2.5;
const ZOOM_STEP: f32 = 0.1;
/// UO light levels run from 0, full day, to this, full dark.
const LIGHT_LEVEL_MAX: f32 = 30.0;
const TERRAIN_SHADOWS_MIN: f32 = 5.0;
const TERRAIN_SHADOWS_MAX: f32 = 25.0;
const DELAY_MAX_MS: f32 = 1000.0;
const DELAY_STEP_MS: f32 = 10.0;
const ZOOM_PERCENT_MIN: f32 = 50.0;
const ZOOM_PERCENT_MAX: f32 = 200.0;
/// The UO Unicode fonts are numbered from 0 to this.
const UNICODE_FONT_MAX: f32 = 12.0;
const TRUETYPE_SIZE_MIN: f32 = 8.0;
const TRUETYPE_SIZE_MAX: f32 = 32.0;
const SPEECH_DELAY_MAX: f32 = 1000.0;
const JOURNAL_FILES_MIN: f32 = 1.0;
const JOURNAL_FILES_MAX: f32 = 1000.0;
const RANGE_CIRCLE_MIN: f32 = 1.0;
const RANGE_CIRCLE_MAX: f32 = 24.0;
const ABBREVIATE_MIN: f32 = 100.0;
const ABBREVIATE_MAX: f32 = 100_000.0;
const ABBREVIATE_STEP: f32 = 100.0;
const LOW_AMOUNT_MIN: f32 = 1.0;
const LOW_AMOUNT_MAX: f32 = 1000.0;
const COUNTER_CELLS_MIN: f32 = 1.0;
const COUNTER_CELLS_MAX: f32 = 10.0;
const CELL_SIZE_MIN: f32 = 30.0;
const CELL_SIZE_MAX: f32 = 80.0;
const SCALE_PERCENT_MIN: f32 = 50.0;
const SCALE_PERCENT_MAX: f32 = 200.0;
const GRID_CELLS_MIN: f32 = 1.0;
const GRID_CELLS_MAX: f32 = 20.0;
const OPACITY_MIN: f32 = 10.0;
const GUMP_OPACITY_MIN: f32 = 20.0;
const JOURNAL_LINES_MIN: f32 = 100.0;
const JOURNAL_LINES_MAX: f32 = 10_000.0;
const JOURNAL_LINES_STEP: f32 = 100.0;
const MARKER_FONT_STYLE_MIN: f32 = 1.0;
const MARKER_FONT_STYLE_MAX: f32 = 6.0;
/// The zoom steps of the world map, from the farthest to the closest.
const WORLD_MAP_ZOOM_STEP_MIN: f32 = 0.0;
const WORLD_MAP_ZOOM_STEP_MAX: f32 = (ZOOMS.len() - 1) as f32;
const CONTROLLER_SENSITIVITY_MIN: f32 = 1.0;
const CONTROLLER_SENSITIVITY_MAX: f32 = 20.0;

const SECTION_MOVEMENT: &str = "Movement";
const SECTION_MOBILES: &str = "Mobiles";
const SECTION_LAYER_HIDING: &str = "Layer hiding";
const SECTION_GUMPS: &str = "Gumps & context menus";
const SECTION_MISC: &str = "Miscellaneous";
const SECTION_TERRAIN: &str = "Terrain & statics";
const SECTION_VOLUME: &str = "Volume";
const SECTION_SOUND_RULES: &str = "Rules";
const SECTION_WINDOW: &str = "Window";
const SECTION_GAME_WINDOW: &str = "Game window";
const SECTION_ZOOM: &str = "Zoom";
const SECTION_LIGHTS: &str = "Lights";
const SECTION_EFFECTS: &str = "Effects";
const SECTION_KEYS: &str = "Keys";
const SECTION_CONTROLLER: &str = "Game controller";
const SECTION_TOOLTIP: &str = "Tooltip";
const SECTION_GAME_FONT: &str = "Game font";
const SECTION_TRUETYPE: &str = "TrueType font";
const SECTION_SPEECH: &str = "Speech";
const SECTION_CHAT: &str = "Chat";
const SECTION_JOURNAL_FILE: &str = "Journal file";
const SECTION_HUES: &str = "Colors";
const SECTION_COMBAT: &str = "Combat";
const SECTION_NOTORIETY: &str = "Notoriety colors";
const SECTION_SPELLS: &str = "Spells";
const SECTION_COOLDOWNS: &str = "Cooldowns & indicators";
const SECTION_COUNTERS: &str = "Counters";
const SECTION_INFO_BAR: &str = "Info bar";
const SECTION_CONTAINER_GUMPS: &str = "Container gumps";
const SECTION_GRID: &str = "Grid containers";
const SECTION_PLAYERS: &str = "Players";
const SECTION_STYLE: &str = "Style";
const SECTION_PAPERDOLL: &str = "Paperdoll";
const SECTION_EXTRAS: &str = "Extras";
const SECTION_NAMEPLATES: &str = "Nameplates";
const SECTION_TABS: &str = "Tabs";
const SECTION_LINES: &str = "Lines";
const SECTION_LOOK: &str = "Look";
const SECTION_SHOW: &str = "Show";
const SECTION_BEHAVIOR: &str = "Behavior";
const SECTION_FILES: &str = "Files";
const SECTION_AGENT_WINDOWS: &str = "Agent windows";

/// How a slider shows its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Plain,
    /// A share from 0 to 1, shown as a percent.
    Fraction,
    /// A number from 0 to 100, shown as a percent.
    Percent,
    Milliseconds,
    Tiles,
    Pixels,
    FramesPerSecond,
    /// A scale, shown as "x1.25".
    Times,
}

impl Unit {
    /// The number as the player reads it.
    pub fn format(self, value: f32) -> String {
        match self {
            Self::Plain => format!("{value:.0}"),
            Self::Fraction => format!("{:.0}%", value * PERCENT_FULL),
            Self::Percent => format!("{value:.0}%"),
            Self::Milliseconds => format!("{value:.0} ms"),
            Self::Tiles => format!("{value:.0} tiles"),
            Self::Pixels => format!("{value:.0} px"),
            Self::FramesPerSecond => format!("{value:.0} FPS"),
            Self::Times => format!("x{value:.2}"),
        }
    }
}

/// The control a row needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionKind {
    Toggle,
    Slider {
        min: f32,
        max: f32,
        step: f32,
        unit: Unit,
    },
    Choice {
        labels: &'static [&'static str],
    },
    /// A UO hue number.
    Hue,
    Text,
    /// A file on this computer, or none.
    FilePath,
    /// The key bindings of the Macros page.
    KeyList,
    /// A list of words, one on each line.
    TextList,
    /// A list of sound or music numbers.
    IdList,
    InfoBarItems,
    JournalTabs,
    Cooldowns,
    HighlightRules,
    CounterItems,
}

/// The value of one row. Its form follows the kind of the row.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionValue {
    Toggle(bool),
    Number(f32),
    Choice(usize),
    Hue(u16),
    Text(String),
    FilePath(Option<PathBuf>),
    Keys(Vec<KeyBinding>),
    TextList(Vec<String>),
    Ids(Vec<u16>),
    InfoBarItems(Vec<InfoBarItem>),
    JournalTabs(Vec<JournalTab>),
    Cooldowns(Vec<CooldownRule>),
    HighlightRules(Vec<HighlightRule>),
    CounterItems(Vec<CounterItem>),
}

/// One option on the Options screen.
pub struct OptionRow {
    pub page: Page,
    pub section: &'static str,
    /// Plain words, unique on the page.
    pub label: &'static str,
    pub kind: OptionKind,
    pub get: fn(&Profile) -> OptionValue,
    /// Writes the value. A value of the wrong form changes nothing, and a
    /// number is kept inside the slider's range.
    pub set: fn(&mut Profile, OptionValue),
}

/// A number field a slider can hold.
trait SliderNumber: Copy {
    fn to_slider(self) -> f32;
    fn from_slider(value: f32) -> Self;
}

macro_rules! whole_slider_number {
    ($($ty:ty),+) => {
        $(impl SliderNumber for $ty {
            fn to_slider(self) -> f32 {
                self as f32
            }

            fn from_slider(value: f32) -> Self {
                value.round() as $ty
            }
        })+
    };
}

whole_slider_number!(u8, u16, u32);

impl SliderNumber for f32 {
    fn to_slider(self) -> f32 {
        self
    }

    fn from_slider(value: f32) -> Self {
        value
    }
}

macro_rules! toggle {
    ($page:ident, $section:expr, $label:expr, $($field:ident).+) => {
        OptionRow {
            page: Page::$page,
            section: $section,
            label: $label,
            kind: OptionKind::Toggle,
            get: |p| OptionValue::Toggle(p.$($field).+),
            set: |p, value| {
                if let OptionValue::Toggle(on) = value {
                    p.$($field).+ = on;
                }
            },
        }
    };
}

macro_rules! hue {
    ($page:ident, $section:expr, $label:expr, $($field:ident).+) => {
        OptionRow {
            page: Page::$page,
            section: $section,
            label: $label,
            kind: OptionKind::Hue,
            get: |p| OptionValue::Hue(p.$($field).+),
            set: |p, value| {
                if let OptionValue::Hue(hue) = value {
                    p.$($field).+ = hue;
                }
            },
        }
    };
}

macro_rules! slider {
    ($page:ident, $section:expr, $label:expr, $($field:ident).+,
     $min:expr, $max:expr, $step:expr, $unit:ident) => {
        OptionRow {
            page: Page::$page,
            section: $section,
            label: $label,
            kind: OptionKind::Slider {
                min: $min,
                max: $max,
                step: $step,
                unit: Unit::$unit,
            },
            get: |p| OptionValue::Number(SliderNumber::to_slider(p.$($field).+)),
            set: |p, value| {
                if let OptionValue::Number(number) = value {
                    p.$($field).+ = SliderNumber::from_slider(number.clamp($min, $max));
                }
            },
        }
    };
}

macro_rules! choice {
    ($page:ident, $section:expr, $label:expr, $($field:ident).+, $ty:ty) => {
        OptionRow {
            page: Page::$page,
            section: $section,
            label: $label,
            kind: OptionKind::Choice {
                labels: <$ty as Choice>::LABELS,
            },
            get: |p| OptionValue::Choice(p.$($field).+.index()),
            set: |p, value| {
                if let OptionValue::Choice(index) = value {
                    p.$($field).+ = <$ty as Choice>::from_index(index);
                }
            },
        }
    };
}

/// The row that hides the worn layer at `$index` of the named layers.
macro_rules! hide_layer {
    ($index:expr) => {
        OptionRow {
            page: Page::General,
            section: SECTION_LAYER_HIDING,
            label: WORN_LAYERS[$index].1,
            kind: OptionKind::Toggle,
            get: |p| OptionValue::Toggle(p.general.hidden_layers.contains(&WORN_LAYERS[$index].0)),
            set: |p, value| {
                if let OptionValue::Toggle(hidden) = value {
                    p.general.set_layer_hidden(WORN_LAYERS[$index].0, hidden);
                }
            },
        }
    };
}

/// A row whose value is a clone of its field.
macro_rules! owned {
    ($page:ident, $section:expr, $label:expr, $kind:ident, $variant:ident, $($field:ident).+) => {
        OptionRow {
            page: Page::$page,
            section: $section,
            label: $label,
            kind: OptionKind::$kind,
            get: |p| OptionValue::$variant(p.$($field).+.clone()),
            set: |p, value| {
                if let OptionValue::$variant(owned) = value {
                    p.$($field).+ = owned;
                }
            },
        }
    };
}

/// Every option, page by page, in the order the Options screen shows them.
pub static OPTION_ROWS: &[OptionRow] = &[
    // General.
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Highlight game objects",
        general.highlight_objects
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Enable pathfinding",
        general.pathfinding
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Use Shift for pathfinding",
        general.shift_pathfinding
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Click on the ground runs there",
        general.click_to_run
    ),
    choice!(
        General,
        SECTION_MOVEMENT,
        "Click with this key held runs there",
        general.run_click_key,
        ModifierKey
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Use W A S D to walk",
        general.wasd_movement
    ),
    toggle!(General, SECTION_MOVEMENT, "Always run", general.always_run),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Always run unless hidden",
        general.always_run_unless_hidden
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Auto open doors",
        general.auto_open_doors
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Auto open corpses",
        general.auto_open_corpses
    ),
    slider!(
        General,
        SECTION_MOVEMENT,
        "Corpse open range",
        general.corpse_open_range,
        CORPSE_RANGE_MIN,
        CORPSE_RANGE_MAX,
        WHOLE_STEP,
        Tiles
    ),
    toggle!(
        General,
        SECTION_MOVEMENT,
        "Skip empty corpses",
        general.skip_empty_corpses
    ),
    choice!(
        General,
        SECTION_MOVEMENT,
        "Corpse open options",
        general.corpse_open_rule,
        CorpseOpenRule
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "No color for objects out of range",
        general.out_of_range_no_color
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Sallos easy grab",
        general.sallos_easy_grab
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Show houses content",
        general.show_house_content
    ),
    toggle!(General, SECTION_MOBILES, "Show HP", general.show_mobile_hp),
    choice!(
        General,
        SECTION_MOBILES,
        "HP mode",
        general.mobile_hp_style,
        HpStyle
    ),
    choice!(
        General,
        SECTION_MOBILES,
        "Show HP when",
        general.mobile_hp_when,
        HpShowWhen
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Highlight poisoned",
        general.highlight_poisoned
    ),
    hue!(
        General,
        SECTION_MOBILES,
        "Poisoned color",
        general.poisoned_hue
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Highlight paralyzed",
        general.highlight_paralyzed
    ),
    hue!(
        General,
        SECTION_MOBILES,
        "Paralyzed color",
        general.paralyzed_hue
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Highlight invulnerable",
        general.highlight_invulnerable
    ),
    hue!(
        General,
        SECTION_MOBILES,
        "Invulnerable color",
        general.invulnerable_hue
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Show incoming new mobiles",
        general.show_incoming_mobiles
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Show incoming new corpses",
        general.show_incoming_corpses
    ),
    choice!(
        General,
        SECTION_MOBILES,
        "Aura under feet",
        general.aura_under_feet,
        AuraRule
    ),
    toggle!(
        General,
        SECTION_MOBILES,
        "Custom color aura for party members",
        general.party_aura
    ),
    hue!(
        General,
        SECTION_MOBILES,
        "Party aura color",
        general.party_aura_hue
    ),
    toggle!(
        General,
        SECTION_LAYER_HIDING,
        "Enable layer hiding",
        general.hidden_layers_enabled
    ),
    toggle!(
        General,
        SECTION_LAYER_HIDING,
        "Only for yourself",
        general.hide_layers_for_self
    ),
    hide_layer!(0),
    hide_layer!(1),
    hide_layer!(2),
    hide_layer!(3),
    hide_layer!(4),
    hide_layer!(5),
    hide_layer!(6),
    hide_layer!(7),
    hide_layer!(8),
    hide_layer!(9),
    hide_layer!(10),
    hide_layer!(11),
    hide_layer!(12),
    hide_layer!(13),
    hide_layer!(14),
    hide_layer!(15),
    hide_layer!(16),
    hide_layer!(17),
    hide_layer!(18),
    hide_layer!(19),
    hide_layer!(20),
    hide_layer!(21),
    hide_layer!(22),
    toggle!(
        General,
        SECTION_GUMPS,
        "Disable the menu bar",
        general.hide_menu_bar
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Hold Alt and right click to close anchored gumps",
        general.alt_right_click_closes_anchored
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Hold Alt to move gumps",
        general.alt_moves_gumps
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Close all anchored gumps when right clicking a group",
        general.right_click_closes_anchored_group
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Use standard skills gump",
        general.standard_skills_gump
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Use old status gump",
        general.old_status_gump
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Status gump and health bar are mutually exclusive",
        general.status_and_bar_exclusive
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Show gump for party invites",
        general.party_invite_gump
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Use custom health bars",
        general.custom_health_bars
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Opaque health bar background",
        general.opaque_health_bars
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Save health bars on logout",
        general.save_health_bars
    ),
    choice!(
        General,
        SECTION_GUMPS,
        "Close health bar when",
        general.close_health_bar,
        CloseHealthBar
    ),
    choice!(
        General,
        SECTION_GUMPS,
        "Grid loot",
        general.grid_loot,
        GridLoot
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Hold Shift for context menus",
        general.shift_for_context_menus
    ),
    toggle!(
        General,
        SECTION_GUMPS,
        "Hold Shift to split stacks",
        general.shift_to_split_stacks
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Enable circle of transparency",
        general.circle_of_transparency
    ),
    slider!(
        General,
        SECTION_MISC,
        "Circle of transparency radius",
        general.circle_radius,
        CIRCLE_RADIUS_MIN,
        CIRCLE_RADIUS_MAX,
        WHOLE_STEP,
        Pixels
    ),
    choice!(
        General,
        SECTION_MISC,
        "Transparency type",
        general.circle_style,
        CircleStyle
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Hide \"Screenshot stored in\" message",
        general.hide_screenshot_message
    ),
    toggle!(General, SECTION_MISC, "Fade objects", general.object_fading),
    toggle!(General, SECTION_MISC, "Fade text", general.text_fading),
    toggle!(
        General,
        SECTION_MISC,
        "Show target range indicator",
        general.target_range_indicator
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Drag-select to open health bars",
        general.drag_select_health_bars
    ),
    choice!(
        General,
        SECTION_MISC,
        "Drag-select modifier key",
        general.drag_select_key,
        ModifierKey
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Select humanoids only",
        general.drag_select_humanoids_only
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Select hostiles only",
        general.drag_select_hostiles_only
    ),
    slider!(
        General,
        SECTION_MISC,
        "Starting X of health bars",
        general.drag_select_start_x,
        0.0,
        SCREEN_MAX_WIDTH,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        General,
        SECTION_MISC,
        "Starting Y of health bars",
        general.drag_select_start_y,
        0.0,
        SCREEN_MAX_HEIGHT,
        WHOLE_STEP,
        Pixels
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Anchor drag-selected health bars",
        general.drag_select_anchored
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Inform when stats change",
        general.stat_change_messages
    ),
    toggle!(
        General,
        SECTION_MISC,
        "Inform when skills change",
        general.skill_change_messages
    ),
    slider!(
        General,
        SECTION_MISC,
        "Skill change step (tenths)",
        general.skill_change_tenths,
        SKILL_CHANGE_MIN,
        SKILL_CHANGE_MAX,
        WHOLE_STEP,
        Plain
    ),
    toggle!(
        General,
        SECTION_TERRAIN,
        "Hide roof tiles",
        general.hide_roofs
    ),
    toggle!(
        General,
        SECTION_TERRAIN,
        "Trees to stumps",
        general.trees_to_stumps
    ),
    toggle!(
        General,
        SECTION_TERRAIN,
        "Hide vegetation",
        general.hide_vegetation
    ),
    toggle!(
        General,
        SECTION_TERRAIN,
        "Mark cave tiles",
        general.mark_cave_tiles
    ),
    choice!(
        General,
        SECTION_TERRAIN,
        "Fields",
        general.field_style,
        FieldStyle
    ),
    // Sound.
    toggle!(Sound, SECTION_VOLUME, "Mute all sound", sound.muted),
    slider!(
        Sound,
        SECTION_VOLUME,
        "Master volume",
        sound.master_volume,
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        Fraction
    ),
    toggle!(Sound, SECTION_VOLUME, "Sounds", sound.sound_on),
    slider!(
        Sound,
        SECTION_VOLUME,
        "Sound volume",
        sound.sound_volume,
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        Fraction
    ),
    toggle!(Sound, SECTION_VOLUME, "Music", sound.music_on),
    slider!(
        Sound,
        SECTION_VOLUME,
        "Music volume",
        sound.music_volume,
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        Fraction
    ),
    toggle!(Sound, SECTION_VOLUME, "Play footsteps", sound.footsteps_on),
    slider!(
        Sound,
        SECTION_VOLUME,
        "Footsteps volume",
        sound.footsteps_volume,
        VOLUME_MIN,
        VOLUME_MAX,
        VOLUME_STEP,
        Fraction
    ),
    toggle!(
        Sound,
        SECTION_SOUND_RULES,
        "Combat music",
        sound.combat_music
    ),
    toggle!(
        Sound,
        SECTION_SOUND_RULES,
        "Play sounds and music when the window is not focused",
        sound.play_in_background
    ),
    toggle!(Sound, SECTION_SOUND_RULES, "Rain sound", sound.rain_sound),
    slider!(
        Sound,
        SECTION_SOUND_RULES,
        "Max sounds at once",
        sound.max_sounds_at_once,
        MAX_SOUNDS_MIN,
        MAX_SOUNDS_MAX,
        WHOLE_STEP,
        Plain
    ),
    owned!(
        Sound,
        SECTION_SOUND_RULES,
        "Sounds that never play",
        IdList,
        Ids,
        sound.sound_filter
    ),
    owned!(
        Sound,
        SECTION_SOUND_RULES,
        "Music that never plays",
        IdList,
        Ids,
        sound.music_filter
    ),
    owned!(
        Sound,
        SECTION_SOUND_RULES,
        "MIDI SoundFont file",
        FilePath,
        FilePath,
        sound.midi_sound_font
    ),
    // Video.
    choice!(
        Video,
        SECTION_WINDOW,
        "Window mode",
        video.window_mode,
        WindowMode
    ),
    slider!(
        Video,
        SECTION_WINDOW,
        "Window width",
        video.window_width,
        WINDOW_MIN_WIDTH,
        SCREEN_MAX_WIDTH,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        Video,
        SECTION_WINDOW,
        "Window height",
        video.window_height,
        WINDOW_MIN_HEIGHT,
        SCREEN_MAX_HEIGHT,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        Video,
        SECTION_WINDOW,
        "FPS",
        video.fps,
        FPS_MIN,
        FPS_MAX,
        WHOLE_STEP,
        FramesPerSecond
    ),
    toggle!(
        Video,
        SECTION_WINDOW,
        "Reduce FPS when the game is inactive",
        video.reduce_fps_when_inactive
    ),
    slider!(
        Video,
        SECTION_WINDOW,
        "Inactive FPS",
        video.inactive_fps,
        INACTIVE_FPS_MIN,
        FPS_MAX,
        WHOLE_STEP,
        FramesPerSecond
    ),
    toggle!(Video, SECTION_WINDOW, "Vertical sync", video.vsync),
    slider!(
        Video,
        SECTION_WINDOW,
        "UI scale",
        video.ui_scale,
        UI_SCALE_MIN,
        UI_SCALE_MAX,
        UI_SCALE_STEP,
        Times
    ),
    toggle!(
        Video,
        SECTION_GAME_WINDOW,
        "Always use full-size game window",
        video.game_window_full_size
    ),
    toggle!(
        Video,
        SECTION_GAME_WINDOW,
        "Lock game window moving and resizing",
        video.game_window_locked
    ),
    slider!(
        Video,
        SECTION_GAME_WINDOW,
        "Game window X",
        video.game_window_x,
        0.0,
        SCREEN_MAX_WIDTH,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        Video,
        SECTION_GAME_WINDOW,
        "Game window Y",
        video.game_window_y,
        0.0,
        SCREEN_MAX_HEIGHT,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        Video,
        SECTION_GAME_WINDOW,
        "Game window width",
        video.game_window_width,
        GAME_WINDOW_MIN_WIDTH,
        SCREEN_MAX_WIDTH,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        Video,
        SECTION_GAME_WINDOW,
        "Game window height",
        video.game_window_height,
        GAME_WINDOW_MIN_HEIGHT,
        SCREEN_MAX_HEIGHT,
        WHOLE_STEP,
        Pixels
    ),
    slider!(
        Video,
        SECTION_ZOOM,
        "Default zoom",
        video.default_zoom,
        ZOOM_MIN,
        ZOOM_MAX,
        ZOOM_STEP,
        Times
    ),
    toggle!(
        Video,
        SECTION_ZOOM,
        "Enable mouse wheel zoom (Ctrl + Scroll)",
        video.wheel_zoom
    ),
    toggle!(
        Video,
        SECTION_ZOOM,
        "Releasing Ctrl restores zoom",
        video.ctrl_release_restores_zoom
    ),
    toggle!(
        Video,
        SECTION_ZOOM,
        "Keep zoom after closing",
        video.keep_zoom_after_close
    ),
    toggle!(
        Video,
        SECTION_LIGHTS,
        "Alternative lights",
        video.alternative_lights
    ),
    toggle!(
        Video,
        SECTION_LIGHTS,
        "Use custom light level",
        video.custom_light_level
    ),
    slider!(
        Video,
        SECTION_LIGHTS,
        "Light level",
        video.light_level,
        0.0,
        LIGHT_LEVEL_MAX,
        WHOLE_STEP,
        Plain
    ),
    choice!(
        Video,
        SECTION_LIGHTS,
        "Light level type",
        video.light_level_rule,
        LightLevelRule
    ),
    toggle!(Video, SECTION_LIGHTS, "Dark nights", video.dark_nights),
    toggle!(
        Video,
        SECTION_LIGHTS,
        "Use colored lights",
        video.colored_lights
    ),
    toggle!(
        Video,
        SECTION_LIGHTS,
        "Candle flicker",
        video.candle_flicker
    ),
    toggle!(
        Video,
        SECTION_EFFECTS,
        "Enable death screen",
        video.death_screen
    ),
    toggle!(
        Video,
        SECTION_EFFECTS,
        "Black and white mode for dead player",
        video.black_and_white_when_dead
    ),
    toggle!(
        Video,
        SECTION_EFFECTS,
        "Aura on mouse target",
        video.aura_on_mouse
    ),
    toggle!(
        Video,
        SECTION_EFFECTS,
        "Animated water effect",
        video.animated_water
    ),
    toggle!(Video, SECTION_EFFECTS, "Shadows", video.shadows),
    toggle!(
        Video,
        SECTION_EFFECTS,
        "Show tree and rock shadows",
        video.statics_shadows
    ),
    slider!(
        Video,
        SECTION_EFFECTS,
        "Terrain shadows level",
        video.terrain_shadows_level,
        TERRAIN_SHADOWS_MIN,
        TERRAIN_SHADOWS_MAX,
        WHOLE_STEP,
        Plain
    ),
    toggle!(
        Video,
        SECTION_EFFECTS,
        "Weather effects",
        video.weather_effects
    ),
    // Macros.
    owned!(
        Macros,
        SECTION_KEYS,
        "Key bindings",
        KeyList,
        Keys,
        macros.key_bindings
    ),
    toggle!(
        Macros,
        SECTION_CONTROLLER,
        "Use a game controller",
        macros.controller_enabled
    ),
    slider!(
        Macros,
        SECTION_CONTROLLER,
        "Mouse speed of the right stick",
        macros.controller_mouse_sensitivity,
        CONTROLLER_SENSITIVITY_MIN,
        CONTROLLER_SENSITIVITY_MAX,
        WHOLE_STEP,
        Plain
    ),
    // Tooltip.
    toggle!(Tooltip, SECTION_TOOLTIP, "Use tooltip", tooltip.enabled),
    slider!(
        Tooltip,
        SECTION_TOOLTIP,
        "Delay before display",
        tooltip.delay_ms,
        0.0,
        DELAY_MAX_MS,
        DELAY_STEP_MS,
        Milliseconds
    ),
    slider!(
        Tooltip,
        SECTION_TOOLTIP,
        "Tooltip zoom",
        tooltip.zoom,
        ZOOM_PERCENT_MIN,
        ZOOM_PERCENT_MAX,
        PERCENT_STEP,
        Percent
    ),
    slider!(
        Tooltip,
        SECTION_TOOLTIP,
        "Tooltip background opacity",
        tooltip.background_opacity,
        0.0,
        PERCENT_FULL,
        PERCENT_STEP,
        Percent
    ),
    hue!(
        Tooltip,
        SECTION_TOOLTIP,
        "Tooltip font hue",
        tooltip.text_hue
    ),
    slider!(
        Tooltip,
        SECTION_TOOLTIP,
        "Tooltip font",
        tooltip.font,
        0.0,
        UNICODE_FONT_MAX,
        WHOLE_STEP,
        Plain
    ),
    // Fonts.
    toggle!(
        Fonts,
        SECTION_GAME_FONT,
        "Override game font",
        fonts.override_game_font
    ),
    choice!(
        Fonts,
        SECTION_GAME_FONT,
        "Game font kind",
        fonts.game_font_kind,
        GameFontKind
    ),
    toggle!(
        Fonts,
        SECTION_GAME_FONT,
        "Force Unicode in journal",
        fonts.force_unicode_journal
    ),
    slider!(
        Fonts,
        SECTION_GAME_FONT,
        "Speech font",
        fonts.speech_font,
        0.0,
        UNICODE_FONT_MAX,
        WHOLE_STEP,
        Plain
    ),
    owned!(
        Fonts,
        SECTION_TRUETYPE,
        "TrueType font file",
        FilePath,
        FilePath,
        fonts.truetype_font
    ),
    slider!(
        Fonts,
        SECTION_TRUETYPE,
        "TrueType font size",
        fonts.truetype_size,
        TRUETYPE_SIZE_MIN,
        TRUETYPE_SIZE_MAX,
        WHOLE_STEP,
        Pixels
    ),
    // Speech.
    slider!(
        Speech,
        SECTION_SPEECH,
        "Speech delay",
        speech.speech_delay,
        0.0,
        SPEECH_DELAY_MAX,
        WHOLE_STEP,
        Plain
    ),
    toggle!(
        Speech,
        SECTION_SPEECH,
        "Scale speech delay",
        speech.scale_speech_delay
    ),
    toggle!(
        Speech,
        SECTION_SPEECH,
        "Show party messages overhead",
        speech.overhead_party_messages
    ),
    toggle!(
        Speech,
        SECTION_CHAT,
        "Activate chat when pressing Enter",
        speech.chat_on_enter
    ),
    toggle!(
        Speech,
        SECTION_CHAT,
        "Speech prefix keys open the chat (! ; : / \\ , . [ | -)",
        speech.chat_prefix_keys
    ),
    toggle!(
        Speech,
        SECTION_CHAT,
        "Use Shift+Enter to send without closing chat",
        speech.shift_enter_sends
    ),
    toggle!(
        Speech,
        SECTION_CHAT,
        "Hide chat gradient",
        speech.hide_chat_gradient
    ),
    toggle!(
        Speech,
        SECTION_CHAT,
        "Hide guild chat",
        speech.hide_guild_chat
    ),
    toggle!(
        Speech,
        SECTION_CHAT,
        "Hide alliance chat",
        speech.hide_alliance_chat
    ),
    toggle!(
        Speech,
        SECTION_JOURNAL_FILE,
        "Save journal to file",
        speech.save_journal
    ),
    slider!(
        Speech,
        SECTION_JOURNAL_FILE,
        "Keep at most this many journal files",
        speech.max_journal_files,
        JOURNAL_FILES_MIN,
        JOURNAL_FILES_MAX,
        WHOLE_STEP,
        Plain
    ),
    toggle!(
        Speech,
        SECTION_JOURNAL_FILE,
        "Add speaker serial to journal entries",
        speech.journal_file_with_serial
    ),
    hue!(Speech, SECTION_HUES, "Speech color", speech.speech_hue),
    hue!(Speech, SECTION_HUES, "Emote color", speech.emote_hue),
    hue!(Speech, SECTION_HUES, "Yell color", speech.yell_hue),
    hue!(Speech, SECTION_HUES, "Whisper color", speech.whisper_hue),
    hue!(
        Speech,
        SECTION_HUES,
        "Party message color",
        speech.party_hue
    ),
    hue!(
        Speech,
        SECTION_HUES,
        "Guild message color",
        speech.guild_hue
    ),
    hue!(
        Speech,
        SECTION_HUES,
        "Alliance message color",
        speech.alliance_hue
    ),
    hue!(Speech, SECTION_HUES, "Chat message color", speech.chat_hue),
    // Combat and spells.
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Use new target system",
        combat.new_target_system
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Hold Tab for combat",
        combat.hold_tab_for_combat
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Query before attack",
        combat.query_before_attack
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Query before beneficial acts on murderers, criminals and grays",
        combat.query_beneficial_acts
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Single-click UI buttons",
        combat.single_click_buttons
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Show buff duration",
        combat.buff_duration
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Enable fast spells assign (Ctrl + Alt)",
        combat.fast_spell_assign
    ),
    toggle!(
        CombatSpells,
        SECTION_COMBAT,
        "Show DPS with damage numbers",
        combat.dps_with_damage
    ),
    hue!(
        CombatSpells,
        SECTION_NOTORIETY,
        "Innocent color",
        combat.innocent_hue
    ),
    hue!(
        CombatSpells,
        SECTION_NOTORIETY,
        "Friend color",
        combat.friend_hue
    ),
    hue!(
        CombatSpells,
        SECTION_NOTORIETY,
        "Criminal color",
        combat.criminal_hue
    ),
    hue!(
        CombatSpells,
        SECTION_NOTORIETY,
        "Can be attacked color",
        combat.can_attack_hue
    ),
    hue!(
        CombatSpells,
        SECTION_NOTORIETY,
        "Murderer color",
        combat.murderer_hue
    ),
    hue!(
        CombatSpells,
        SECTION_NOTORIETY,
        "Enemy color",
        combat.enemy_hue
    ),
    toggle!(
        CombatSpells,
        SECTION_SPELLS,
        "Enable overhead spell format",
        combat.spell_format_on
    ),
    toggle!(
        CombatSpells,
        SECTION_SPELLS,
        "Enable overhead spell hue",
        combat.spell_hue_on
    ),
    hue!(
        CombatSpells,
        SECTION_SPELLS,
        "Beneficial spell hue",
        combat.benefic_spell_hue
    ),
    hue!(
        CombatSpells,
        SECTION_SPELLS,
        "Harmful spell hue",
        combat.harmful_spell_hue
    ),
    hue!(
        CombatSpells,
        SECTION_SPELLS,
        "Neutral spell hue",
        combat.neutral_spell_hue
    ),
    owned!(
        CombatSpells,
        SECTION_SPELLS,
        "Spell overhead format ({power} words, {spell} name)",
        Text,
        Text,
        combat.spell_format
    ),
    toggle!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Show cooldown bars",
        combat.cooldown_bars
    ),
    toggle!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Show spell cast indicator",
        combat.spell_cast_indicator
    ),
    toggle!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Use improved buff bar",
        combat.improved_buff_bar
    ),
    toggle!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Show range circle",
        combat.range_circle
    ),
    slider!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Range circle size",
        combat.range_circle_tiles,
        RANGE_CIRCLE_MIN,
        RANGE_CIRCLE_MAX,
        WHOLE_STEP,
        Tiles
    ),
    hue!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Range circle color",
        combat.range_circle_hue
    ),
    owned!(
        CombatSpells,
        SECTION_COOLDOWNS,
        "Cooldown bars",
        Cooldowns,
        Cooldowns,
        combat.cooldowns
    ),
    // Counters.
    toggle!(
        Counters,
        SECTION_COUNTERS,
        "Enable counters",
        counters.enabled
    ),
    toggle!(
        Counters,
        SECTION_COUNTERS,
        "Highlight on change",
        counters.highlight_on_change
    ),
    toggle!(
        Counters,
        SECTION_COUNTERS,
        "Abbreviate large amounts",
        counters.abbreviate
    ),
    slider!(
        Counters,
        SECTION_COUNTERS,
        "Abbreviate when amount reaches",
        counters.abbreviate_at,
        ABBREVIATE_MIN,
        ABBREVIATE_MAX,
        ABBREVIATE_STEP,
        Plain
    ),
    toggle!(
        Counters,
        SECTION_COUNTERS,
        "Highlight red when amount is low",
        counters.highlight_when_low
    ),
    slider!(
        Counters,
        SECTION_COUNTERS,
        "Low amount",
        counters.low_amount,
        LOW_AMOUNT_MIN,
        LOW_AMOUNT_MAX,
        WHOLE_STEP,
        Plain
    ),
    slider!(
        Counters,
        SECTION_COUNTERS,
        "Rows",
        counters.rows,
        COUNTER_CELLS_MIN,
        COUNTER_CELLS_MAX,
        WHOLE_STEP,
        Plain
    ),
    slider!(
        Counters,
        SECTION_COUNTERS,
        "Columns",
        counters.columns,
        COUNTER_CELLS_MIN,
        COUNTER_CELLS_MAX,
        WHOLE_STEP,
        Plain
    ),
    slider!(
        Counters,
        SECTION_COUNTERS,
        "Cell size",
        counters.cell_size,
        CELL_SIZE_MIN,
        CELL_SIZE_MAX,
        WHOLE_STEP,
        Pixels
    ),
    toggle!(
        Counters,
        SECTION_COUNTERS,
        "Read only: dropped items do not change the bar",
        counters.read_only
    ),
    owned!(
        Counters,
        SECTION_COUNTERS,
        "Counted items",
        CounterItems,
        CounterItems,
        counters.items
    ),
    // Info bar.
    toggle!(InfoBar, SECTION_INFO_BAR, "Show info bar", info_bar.enabled),
    choice!(
        InfoBar,
        SECTION_INFO_BAR,
        "Data highlight type",
        info_bar.highlight,
        InfoBarHighlight
    ),
    owned!(
        InfoBar,
        SECTION_INFO_BAR,
        "Items",
        InfoBarItems,
        InfoBarItems,
        info_bar.items
    ),
    // Containers.
    choice!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Backpack style",
        containers.backpack_style,
        BackpackStyle
    ),
    slider!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Container scale",
        containers.scale,
        SCALE_PERCENT_MIN,
        SCALE_PERCENT_MAX,
        PERCENT_STEP,
        Percent
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Scale items inside containers",
        containers.scale_items
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Use large container gumps",
        containers.large_gumps
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Double click to loot items inside containers",
        containers.double_click_loots
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Relative drag and drop in containers",
        containers.relative_drag_and_drop
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Highlight the container under the mouse",
        containers.highlight_on_hover
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Recolor container gump by item hue",
        containers.hue_gumps
    ),
    toggle!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Override container gump location",
        containers.override_place
    ),
    choice!(
        Containers,
        SECTION_CONTAINER_GUMPS,
        "Container location",
        containers.place,
        ContainerPlace
    ),
    slider!(
        Containers,
        SECTION_GRID,
        "Grid columns",
        containers.grid_columns,
        GRID_CELLS_MIN,
        GRID_CELLS_MAX,
        WHOLE_STEP,
        Plain
    ),
    slider!(
        Containers,
        SECTION_GRID,
        "Grid rows",
        containers.grid_rows,
        GRID_CELLS_MIN,
        GRID_CELLS_MAX,
        WHOLE_STEP,
        Plain
    ),
    slider!(
        Containers,
        SECTION_GRID,
        "Grid scale",
        containers.grid_scale,
        SCALE_PERCENT_MIN,
        SCALE_PERCENT_MAX,
        PERCENT_STEP,
        Percent
    ),
    slider!(
        Containers,
        SECTION_GRID,
        "Grid opacity",
        containers.grid_opacity,
        OPACITY_MIN,
        PERCENT_FULL,
        PERCENT_STEP,
        Percent
    ),
    hue!(
        Containers,
        SECTION_GRID,
        "Grid border color",
        containers.grid_border_hue
    ),
    choice!(
        Containers,
        SECTION_GRID,
        "Grid search",
        containers.grid_search,
        GridSearch
    ),
    toggle!(
        Containers,
        SECTION_GRID,
        "Compare with the worn item in tooltips",
        containers.grid_compare_tooltip
    ),
    owned!(
        Containers,
        SECTION_GRID,
        "Highlight items with these properties",
        TextList,
        TextList,
        containers.grid_highlight_properties
    ),
    owned!(
        Containers,
        SECTION_GRID,
        "Highlight rules",
        HighlightRules,
        HighlightRules,
        containers.grid_highlight_rules
    ),
    toggle!(
        Containers,
        SECTION_GRID,
        "Show what a bag holds under the mouse",
        containers.grid_preview
    ),
    // Experimental.
    toggle!(
        Experimental,
        SECTION_KEYS,
        "Disable default UO hotkeys",
        experimental.disable_default_hotkeys
    ),
    toggle!(
        Experimental,
        SECTION_KEYS,
        "Disable arrow keys for moving",
        experimental.disable_arrow_keys
    ),
    toggle!(
        Experimental,
        SECTION_KEYS,
        "Disable Tab (toggle war mode)",
        experimental.disable_tab_war_mode
    ),
    toggle!(
        Experimental,
        SECTION_KEYS,
        "Disable Ctrl + Q/W (message history)",
        experimental.disable_message_history
    ),
    toggle!(
        Experimental,
        SECTION_KEYS,
        "Disable right and left click auto-move",
        experimental.disable_click_automove
    ),
    // Ignore list.
    owned!(
        IgnoreList,
        SECTION_PLAYERS,
        "Ignored players",
        TextList,
        TextList,
        ignore.names
    ),
    // Interface.
    choice!(
        Interface,
        SECTION_STYLE,
        "UI style",
        interface.ui_style,
        UiStyle
    ),
    slider!(
        Interface,
        SECTION_STYLE,
        "Gump opacity",
        interface.gump_opacity,
        GUMP_OPACITY_MIN,
        PERCENT_FULL,
        PERCENT_STEP,
        Percent
    ),
    toggle!(
        Interface,
        SECTION_STYLE,
        "Remember where gumps are",
        interface.remember_gump_places
    ),
    toggle!(
        Interface,
        SECTION_PAPERDOLL,
        "Show durability bars",
        interface.durability_bars
    ),
    slider!(
        Interface,
        SECTION_PAPERDOLL,
        "Warn when durability is below",
        interface.durability_warning,
        0.0,
        PERCENT_FULL,
        PERCENT_STEP,
        Percent
    ),
    toggle!(
        Interface,
        SECTION_EXTRAS,
        "Show stats in the title bar",
        interface.title_bar_stats
    ),
    choice!(
        Interface,
        SECTION_EXTRAS,
        "Title bar stats as",
        interface.title_bar_mode,
        TitleStats
    ),
    owned!(
        Interface,
        SECTION_EXTRAS,
        "Panels open at start",
        TextList,
        TextList,
        interface.open_panels
    ),
    toggle!(
        Interface,
        SECTION_EXTRAS,
        "Take a screenshot on death",
        interface.screenshot_on_death
    ),
    toggle!(
        Interface,
        SECTION_EXTRAS,
        "Show nearby loot window",
        interface.nearby_loot_window
    ),
    // Nameplates.
    toggle!(
        Nameplates,
        SECTION_NAMEPLATES,
        "Show nameplates",
        nameplates.enabled
    ),
    choice!(
        Nameplates,
        SECTION_NAMEPLATES,
        "Show nameplates on",
        nameplates.filter,
        NameplateFilter
    ),
    toggle!(
        Nameplates,
        SECTION_NAMEPLATES,
        "Health bar on nameplates",
        nameplates.health_bar
    ),
    slider!(
        Nameplates,
        SECTION_NAMEPLATES,
        "Nameplate opacity",
        nameplates.opacity,
        OPACITY_MIN,
        PERCENT_FULL,
        PERCENT_STEP,
        Percent
    ),
    toggle!(
        Nameplates,
        SECTION_NAMEPLATES,
        "Hide at full health",
        nameplates.hide_at_full_health
    ),
    toggle!(
        Nameplates,
        SECTION_NAMEPLATES,
        "Keep nameplates apart",
        nameplates.avoid_overlap
    ),
    // Journal.
    owned!(
        Journal,
        SECTION_TABS,
        "Journal tabs",
        JournalTabs,
        JournalTabs,
        journal.tabs
    ),
    toggle!(
        Journal,
        SECTION_LINES,
        "Show client messages",
        journal.show_client_lines
    ),
    toggle!(
        Journal,
        SECTION_LINES,
        "Show object messages",
        journal.show_object_lines
    ),
    toggle!(
        Journal,
        SECTION_LINES,
        "Show system messages",
        journal.show_system_lines
    ),
    toggle!(
        Journal,
        SECTION_LINES,
        "Show guild and alliance messages",
        journal.show_guild_and_alliance
    ),
    slider!(
        Journal,
        SECTION_LINES,
        "Lines kept",
        journal.max_lines,
        JOURNAL_LINES_MIN,
        JOURNAL_LINES_MAX,
        JOURNAL_LINES_STEP,
        Plain
    ),
    toggle!(Journal, SECTION_LOOK, "Dark mode", journal.dark_mode),
    toggle!(
        Journal,
        SECTION_LOOK,
        "Use the resizable journal",
        journal.alternate_journal
    ),
    slider!(
        Journal,
        SECTION_LOOK,
        "Journal opacity",
        journal.opacity,
        OPACITY_MIN,
        PERCENT_FULL,
        PERCENT_STEP,
        Percent
    ),
    toggle!(
        Journal,
        SECTION_LOOK,
        "Hide timestamps",
        journal.hide_timestamps
    ),
    // World map.
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show party members",
        world_map.show_party
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show your coordinates",
        world_map.show_coordinates
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show mouse coordinates",
        world_map.show_mouse_coordinates
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Use sextant coordinates",
        world_map.sextant_coordinates
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show mobiles",
        world_map.show_mobiles
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show player name",
        world_map.show_player_name
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show player health bar",
        world_map.show_player_bar
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show group names",
        world_map.show_group_names
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show group health bars",
        world_map.show_group_bars
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show markers",
        world_map.show_markers
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show marker names",
        world_map.show_marker_names
    ),
    slider!(
        WorldMap,
        SECTION_SHOW,
        "Marker font style",
        world_map.marker_font_style,
        MARKER_FONT_STYLE_MIN,
        MARKER_FONT_STYLE_MAX,
        WHOLE_STEP,
        Plain
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show houses and boats",
        world_map.show_multis
    ),
    toggle!(
        WorldMap,
        SECTION_SHOW,
        "Show grid when zoomed",
        world_map.grid_when_zoomed
    ),
    toggle!(WorldMap, SECTION_BEHAVIOR, "Free view", world_map.free_view),
    toggle!(
        WorldMap,
        SECTION_BEHAVIOR,
        "Show the whole world",
        world_map.whole_world
    ),
    slider!(
        WorldMap,
        SECTION_BEHAVIOR,
        "Zoom step of the world map",
        world_map.zoom_step,
        WORLD_MAP_ZOOM_STEP_MIN,
        WORLD_MAP_ZOOM_STEP_MAX,
        WHOLE_STEP,
        Plain
    ),
    toggle!(
        WorldMap,
        SECTION_BEHAVIOR,
        "Large minimap and radar",
        world_map.minimap_large
    ),
    toggle!(
        WorldMap,
        SECTION_BEHAVIOR,
        "Keep the map on top",
        world_map.always_on_top
    ),
    toggle!(
        WorldMap,
        SECTION_BEHAVIOR,
        "Turn the map as the game view",
        world_map.flip_map
    ),
    toggle!(
        WorldMap,
        SECTION_BEHAVIOR,
        "A click on the map answers a target of a place",
        world_map.allow_positional_target
    ),
    owned!(
        WorldMap,
        SECTION_FILES,
        "Hidden marker files",
        TextList,
        TextList,
        world_map.hidden_marker_files
    ),
    owned!(
        WorldMap,
        SECTION_FILES,
        "Hidden zone files",
        TextList,
        TextList,
        world_map.hidden_zone_files
    ),
    // Agents.
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Loot", agents.loot),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Scavenger", agents.scavenger),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Bandage", agents.bandage),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Self heal", agents.self_heal),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Buy", agents.buy),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Sell", agents.sell),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Dress", agents.dress),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Organizer", agents.organizer),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Skinning", agents.skinning),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Remount", agents.remount),
    toggle!(Agents, SECTION_AGENT_WINDOWS, "Friends", agents.friends),
];

/// The rows of one page, in order.
pub fn rows_on(page: Page) -> impl Iterator<Item = &'static OptionRow> {
    OPTION_ROWS.iter().filter(move |row| row.page == page)
}

#[cfg(test)]
mod tests {
    use super::super::keys::MacroStep;
    use super::*;
    use std::collections::{BTreeMap, HashSet};

    /// A value of the same form as `value` that differs from it.
    fn other_value(kind: OptionKind, value: &OptionValue) -> OptionValue {
        match (kind, value) {
            (_, OptionValue::Toggle(on)) => OptionValue::Toggle(!on),
            (OptionKind::Slider { min, max, .. }, OptionValue::Number(number)) => {
                OptionValue::Number(if *number == max { min } else { max })
            }
            (OptionKind::Choice { labels }, OptionValue::Choice(index)) => {
                OptionValue::Choice((index + 1) % labels.len())
            }
            (_, OptionValue::Hue(hue)) => OptionValue::Hue(hue ^ 1),
            (_, OptionValue::Text(words)) => OptionValue::Text(format!("{words}!")),
            (_, OptionValue::FilePath(_)) => OptionValue::FilePath(Some("fonts/test.ttf".into())),
            (_, OptionValue::Keys(keys)) => {
                let mut keys = keys.clone();
                keys.push(KeyBinding {
                    chord: Some("Ctrl+F1".parse().unwrap()),
                    steps: vec![MacroStep::new("say", "hail")],
                    ..KeyBinding::default()
                });
                OptionValue::Keys(keys)
            }
            (_, OptionValue::TextList(lines)) => {
                let mut lines = lines.clone();
                lines.push("one more".into());
                OptionValue::TextList(lines)
            }
            (_, OptionValue::Ids(ids)) => {
                let mut ids = ids.clone();
                ids.push(0x0123);
                OptionValue::Ids(ids)
            }
            (_, OptionValue::InfoBarItems(items)) => OptionValue::InfoBarItems(items[1..].to_vec()),
            (_, OptionValue::JournalTabs(tabs)) => OptionValue::JournalTabs(tabs[1..].to_vec()),
            (_, OptionValue::Cooldowns(rules)) => {
                let mut rules = rules.clone();
                rules.push(CooldownRule {
                    label: "Hide".into(),
                    hue: 1,
                    trigger: "You have hidden yourself".into(),
                    seconds: 10.0,
                    source: CooldownSource::System,
                    restart: true,
                });
                OptionValue::Cooldowns(rules)
            }
            (_, OptionValue::HighlightRules(rules)) => {
                let mut rules = rules.clone();
                rules.push(HighlightRule {
                    name: "Legendary".into(),
                    hue: 1,
                    needs: Vec::new(),
                    need_all: false,
                    corpses_only: false,
                });
                OptionValue::HighlightRules(rules)
            }
            (_, OptionValue::CounterItems(items)) => {
                let mut items = items.clone();
                items.push(CounterItem {
                    label: "Bandages".into(),
                    graphic: 0x0E21,
                    hue: 0,
                });
                OptionValue::CounterItems(items)
            }
            (kind, value) => panic!("{value:?} does not fit {kind:?}"),
        }
    }

    /// Every leaf of a TOML value, by its path.
    fn leaves(value: &toml::Value, path: String, into: &mut BTreeMap<String, toml::Value>) {
        match value {
            toml::Value::Table(table) => {
                for (key, inner) in table {
                    leaves(inner, format!("{path}.{key}"), into);
                }
            }
            leaf => {
                into.insert(path, leaf.clone());
            }
        }
    }

    #[test]
    fn each_row_writes_its_own_field_and_reads_it_back() {
        let defaults = Profile::default();
        for row in OPTION_ROWS {
            let before = (row.get)(&defaults);
            let changed = other_value(row.kind, &before);
            let mut profile = defaults.clone();
            (row.set)(&mut profile, changed.clone());
            assert_eq!((row.get)(&profile), changed, "{}", row.label);
            (row.set)(&mut profile, before.clone());
            assert_eq!(profile, defaults, "{} did not go back", row.label);
            (row.set)(&mut profile, changed);
            for other in OPTION_ROWS
                .iter()
                .filter(|other| !std::ptr::eq(*other, row))
            {
                assert_eq!(
                    (other.get)(&profile),
                    (other.get)(&defaults),
                    "{} also changed {}",
                    row.label,
                    other.label
                );
            }
        }
    }

    #[test]
    fn a_value_of_the_wrong_form_changes_nothing_and_a_number_stays_in_range() {
        let defaults = Profile::default();
        for row in OPTION_ROWS {
            let mut profile = defaults.clone();
            let wrong = match (row.get)(&defaults) {
                OptionValue::Toggle(_) => OptionValue::Hue(1),
                _ => OptionValue::Toggle(true),
            };
            (row.set)(&mut profile, wrong);
            assert_eq!(profile, defaults, "{}", row.label);
            if let OptionKind::Slider { max, .. } = row.kind {
                (row.set)(&mut profile, OptionValue::Number(max * 2.0 + 1.0));
                assert_eq!(
                    (row.get)(&profile),
                    OptionValue::Number(max),
                    "{}",
                    row.label
                );
            }
        }
    }

    #[test]
    fn each_default_is_inside_its_slider() {
        let defaults = Profile::default();
        for row in OPTION_ROWS {
            if let (OptionKind::Slider { min, max, .. }, OptionValue::Number(number)) =
                (row.kind, (row.get)(&defaults))
            {
                assert!((min..=max).contains(&number), "{}", row.label);
            }
        }
    }

    #[test]
    fn no_page_has_two_rows_with_one_label_and_every_page_has_rows() {
        let mut seen = HashSet::new();
        for row in OPTION_ROWS {
            assert!(
                seen.insert((row.page, row.label)),
                "{:?} {}",
                row.page,
                row.label
            );
        }
        for index in 0..Page::LABELS.len() {
            let page = Page::from_index(index);
            assert!(rows_on(page).next().is_some(), "{page:?}");
        }
    }

    #[test]
    fn the_table_reaches_every_option_of_the_profile() {
        let defaults = Profile::default();
        let mut changed = defaults.clone();
        for row in OPTION_ROWS {
            let value = other_value(row.kind, &(row.get)(&defaults));
            (row.set)(&mut changed, value);
        }
        let mut before = BTreeMap::new();
        let mut after = BTreeMap::new();
        leaves(
            &toml::Value::try_from(&defaults).unwrap(),
            String::new(),
            &mut before,
        );
        leaves(
            &toml::Value::try_from(&changed).unwrap(),
            String::new(),
            &mut after,
        );
        let untouched: Vec<&String> = before
            .iter()
            .filter(|(path, value)| after.get(*path) == Some(*value))
            .map(|(path, _)| path)
            .collect();
        assert!(untouched.is_empty(), "no row sets {untouched:?}");
    }

    #[test]
    fn each_named_worn_layer_has_one_hide_row() {
        let hidden: Vec<u8> = rows_on(Page::General)
            .filter(|row| row.section == SECTION_LAYER_HIDING)
            .filter_map(|row| {
                let mut profile = Profile::default();
                (row.set)(&mut profile, OptionValue::Toggle(true));
                match profile.general.hidden_layers.as_slice() {
                    [layer] => Some(*layer),
                    _ => None,
                }
            })
            .collect();
        let named: Vec<u8> = WORN_LAYERS.iter().map(|(layer, _)| *layer).collect();
        assert_eq!(hidden, named);
    }

    #[test]
    fn a_unit_shows_its_number_as_a_player_reads_it() {
        assert_eq!(Unit::Fraction.format(0.5), "50%");
        assert_eq!(Unit::Times.format(1.25), "x1.25");
        assert_eq!(Unit::Milliseconds.format(250.0), "250 ms");
    }
}
