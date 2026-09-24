//! The options of each page of the Options screen. The defaults are the
//! defaults of the official client and of the reference client. Each page loads with
//! its defaults for any option a kept file does not name.

use super::choices::{
    AuraRule, BackpackStyle, CircleStyle, CloseHealthBar, ContainerPlace, CooldownSource,
    CorpseOpenRule, FieldStyle, GameFontKind, GridLoot, GridSearch, HpShowWhen, HpStyle,
    InfoBarData, InfoBarHighlight, JournalKind, LightLevelRule, ModifierKey, NameplateFilter,
    TitleStats, UiStyle, WindowMode,
};
use super::keys::KeyBinding;
use crate::view::{WINDOW_HEIGHT, WINDOW_WIDTH};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

// General.
pub const DEFAULT_CORPSE_OPEN_RANGE: u8 = 2;
pub const DEFAULT_CORPSE_OPEN_RULE: CorpseOpenRule = CorpseOpenRule::UnlessTargetingOrHidden;
pub const DEFAULT_HP_STYLE: HpStyle = HpStyle::Percentage;
pub const DEFAULT_HP_SHOW_WHEN: HpShowWhen = HpShowWhen::Always;
pub const DEFAULT_POISONED_HUE: u16 = 0x0044;
pub const DEFAULT_PARALYZED_HUE: u16 = 0x014C;
pub const DEFAULT_INVULNERABLE_HUE: u16 = 0x0030;
pub const DEFAULT_AURA_RULE: AuraRule = AuraRule::Never;
pub const DEFAULT_PARTY_AURA_HUE: u16 = 0x0044;
pub const DEFAULT_CLOSE_HEALTH_BAR: CloseHealthBar = CloseHealthBar::Never;
pub const DEFAULT_GRID_LOOT: GridLoot = GridLoot::Off;
pub const DEFAULT_CIRCLE_RADIUS: u16 = 100;
pub const DEFAULT_CIRCLE_STYLE: CircleStyle = CircleStyle::Full;
pub const DEFAULT_DRAG_SELECT_KEY: ModifierKey = ModifierKey::None;
/// Alt with a click on the ground runs there: no other click of the game
/// world takes Alt.
pub const DEFAULT_RUN_CLICK_KEY: ModifierKey = ModifierKey::Alt;
pub const DEFAULT_DRAG_SELECT_START: f32 = 100.0;
pub const DEFAULT_SKILL_CHANGE_TENTHS: u16 = 1;
pub const DEFAULT_FIELD_STYLE: FieldStyle = FieldStyle::Normal;

// Sound. Each volume is from 0 to 1.
pub const DEFAULT_MASTER_VOLUME: f32 = 0.8;
pub const DEFAULT_SOUND_VOLUME: f32 = 0.8;
pub const DEFAULT_MUSIC_VOLUME: f32 = 0.5;
pub const DEFAULT_FOOTSTEPS_VOLUME: f32 = 0.4;
pub const DEFAULT_MAX_SOUNDS: u16 = 32;

// Video.
pub const DEFAULT_WINDOW_MODE: WindowMode = WindowMode::Windowed;
pub const DEFAULT_FPS: u16 = 60;
pub const DEFAULT_INACTIVE_FPS: u16 = 15;
pub const DEFAULT_UI_SCALE: f32 = 1.0;
pub const DEFAULT_GAME_WINDOW_X: f32 = 10.0;
pub const DEFAULT_GAME_WINDOW_Y: f32 = 10.0;
pub const DEFAULT_GAME_WINDOW_WIDTH: f32 = 600.0;
pub const DEFAULT_GAME_WINDOW_HEIGHT: f32 = 480.0;
pub const DEFAULT_ZOOM: f32 = 1.0;
/// The UO light level: 0 is full day, higher is darker.
pub const DEFAULT_LIGHT_LEVEL: u8 = 0;
pub const DEFAULT_LIGHT_LEVEL_RULE: LightLevelRule = LightLevelRule::Absolute;
pub const DEFAULT_TERRAIN_SHADOWS: u8 = 15;

// Macros.
pub const DEFAULT_CONTROLLER_SENSITIVITY: u8 = 10;

// Tooltip.
pub const DEFAULT_TOOLTIP_DELAY_MS: u16 = 250;
pub const DEFAULT_TOOLTIP_ZOOM: u16 = 100;
pub const DEFAULT_TOOLTIP_OPACITY: u8 = 70;
/// The hue that means "the font's own color".
pub const NO_HUE: u16 = 0xFFFF;
pub const DEFAULT_TOOLTIP_FONT: u8 = 1;

// Fonts.
pub const DEFAULT_GAME_FONT_KIND: GameFontKind = GameFontKind::Unicode;
pub const DEFAULT_SPEECH_FONT: u8 = 1;
pub const DEFAULT_TRUETYPE_SIZE: f32 = 16.0;

// Speech.
pub const DEFAULT_SPEECH_DELAY: u16 = 100;
pub const DEFAULT_MAX_JOURNAL_FILES: u16 = 100;
pub const DEFAULT_SPEECH_HUE: u16 = 0x02B2;
pub const DEFAULT_EMOTE_HUE: u16 = 0x0021;
pub const DEFAULT_YELL_HUE: u16 = 0x0021;
pub const DEFAULT_WHISPER_HUE: u16 = 0x0033;
pub const DEFAULT_PARTY_HUE: u16 = 0x0044;
pub const DEFAULT_GUILD_HUE: u16 = 0x0044;
pub const DEFAULT_ALLIANCE_HUE: u16 = 0x0057;
pub const DEFAULT_CHAT_HUE: u16 = 0x0256;

// Combat and spells.
pub const DEFAULT_INNOCENT_HUE: u16 = 0x005A;
pub const DEFAULT_FRIEND_HUE: u16 = 0x0044;
pub const DEFAULT_CRIMINAL_HUE: u16 = 0x03B2;
pub const DEFAULT_CAN_ATTACK_HUE: u16 = 0x03B2;
pub const DEFAULT_MURDERER_HUE: u16 = 0x0023;
pub const DEFAULT_ENEMY_HUE: u16 = 0x0031;
pub const DEFAULT_BENEFIC_HUE: u16 = 0x0059;
pub const DEFAULT_HARMFUL_HUE: u16 = 0x0020;
pub const DEFAULT_NEUTRAL_HUE: u16 = 0x03B1;
pub const DEFAULT_SPELL_FORMAT: &str = "{power} [{spell}]";
pub const DEFAULT_RANGE_CIRCLE_TILES: u8 = 12;
pub const DEFAULT_RANGE_CIRCLE_HUE: u16 = 0x0022;

/// A new cooldown bar runs this long until the player sets its time.
pub const DEFAULT_COOLDOWN_SECONDS: f32 = 10.0;

// Counters.
pub const DEFAULT_COUNTER_ABBREVIATE_AT: u32 = 1000;
pub const DEFAULT_COUNTER_LOW_AMOUNT: u32 = 5;
pub const DEFAULT_COUNTER_ROWS: u8 = 1;
pub const DEFAULT_COUNTER_COLUMNS: u8 = 1;
pub const DEFAULT_COUNTER_CELL_SIZE: u8 = 40;

// Info bar.
pub const DEFAULT_INFO_BAR_HIGHLIGHT: InfoBarHighlight = InfoBarHighlight::TextColor;
pub const INFO_BAR_NAME_HUE: u16 = 0x03D2;
pub const INFO_BAR_HITS_HUE: u16 = 0x01B6;
pub const INFO_BAR_MANA_HUE: u16 = 0x01ED;
pub const INFO_BAR_STAMINA_HUE: u16 = 0x022E;
pub const INFO_BAR_WEIGHT_HUE: u16 = 0x03D2;
pub const INFO_BAR_WORDS_HITS: &str = "Hits";
pub const INFO_BAR_WORDS_MANA: &str = "Mana";
pub const INFO_BAR_WORDS_STAMINA: &str = "Stam";
pub const INFO_BAR_WORDS_WEIGHT: &str = "Weight";

// Containers.
pub const DEFAULT_BACKPACK_STYLE: BackpackStyle = BackpackStyle::Default;
pub const DEFAULT_CONTAINER_SCALE: u16 = 100;
pub const DEFAULT_CONTAINER_PLACE: ContainerPlace = ContainerPlace::NearContainer;
pub const DEFAULT_GRID_COLUMNS: u8 = 5;
pub const DEFAULT_GRID_ROWS: u8 = 5;
pub const DEFAULT_GRID_SCALE: u16 = 100;
pub const DEFAULT_GRID_OPACITY: u8 = 100;
pub const DEFAULT_GRID_BORDER_HUE: u16 = 0x0000;
pub const DEFAULT_GRID_SEARCH: GridSearch = GridSearch::Highlight;

// Interface.
pub const DEFAULT_UI_STYLE: UiStyle = UiStyle::Modern;
pub const DEFAULT_TITLE_STATS: TitleStats = TitleStats::Numbers;
pub const DEFAULT_GUMP_OPACITY: u8 = 100;
pub const DEFAULT_DURABILITY_WARNING: u8 = 20;

// Nameplates.
pub const DEFAULT_NAMEPLATE_FILTER: NameplateFilter = NameplateFilter::All;
pub const DEFAULT_NAMEPLATE_OPACITY: u8 = 75;

// Journal.
pub const DEFAULT_JOURNAL_OPACITY: u8 = 100;
pub const DEFAULT_JOURNAL_MAX_LINES: u16 = 1500;
pub const JOURNAL_TAB_ALL: &str = "All";
pub const JOURNAL_TAB_CHAT: &str = "Chat";
pub const JOURNAL_TAB_GROUPS: &str = "Guild & Party";
pub const JOURNAL_TAB_SYSTEM: &str = "System";

// World map.
/// The zoom step the Classic world map opens at: one tile for each pixel.
pub const DEFAULT_WORLD_MAP_ZOOM_STEP: u8 = 4;
/// The first of the six font styles of the marker names.
pub const DEFAULT_MARKER_FONT_STYLE: u8 = 1;

/// The General page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralOptions {
    // Movement.
    pub highlight_objects: bool,
    pub pathfinding: bool,
    pub shift_pathfinding: bool,
    /// One click on the ground runs there by pathfinding. A profile of an
    /// older UOTerm names it by its old name, when the click walked.
    #[serde(alias = "pathfind_single_click")]
    pub click_to_run: bool,
    /// A click on the ground with this key held runs there by pathfinding,
    /// whether `click_to_run` is on or not. None turns the key click off.
    pub run_click_key: ModifierKey,
    /// W, A, S and D walk as the arrows do, while the chat line is closed.
    pub wasd_movement: bool,
    pub always_run: bool,
    pub always_run_unless_hidden: bool,
    pub auto_open_doors: bool,
    pub auto_open_corpses: bool,
    pub corpse_open_range: u8,
    pub skip_empty_corpses: bool,
    pub corpse_open_rule: CorpseOpenRule,
    // Mobiles.
    pub out_of_range_no_color: bool,
    pub sallos_easy_grab: bool,
    pub show_house_content: bool,
    pub show_mobile_hp: bool,
    pub mobile_hp_style: HpStyle,
    pub mobile_hp_when: HpShowWhen,
    pub highlight_poisoned: bool,
    pub poisoned_hue: u16,
    pub highlight_paralyzed: bool,
    pub paralyzed_hue: u16,
    pub highlight_invulnerable: bool,
    pub invulnerable_hue: u16,
    pub show_incoming_mobiles: bool,
    pub show_incoming_corpses: bool,
    pub aura_under_feet: AuraRule,
    pub party_aura: bool,
    pub party_aura_hue: u16,
    // Layer hiding.
    /// The worn layers in `hidden_layers` are not drawn.
    pub hidden_layers_enabled: bool,
    /// The layers are hidden on the character only, not on other mobiles.
    pub hide_layers_for_self: bool,
    /// The worn layers not drawn, in layer order.
    pub hidden_layers: Vec<u8>,
    // Gumps and context menus.
    pub hide_menu_bar: bool,
    pub alt_right_click_closes_anchored: bool,
    pub alt_moves_gumps: bool,
    pub right_click_closes_anchored_group: bool,
    pub standard_skills_gump: bool,
    pub old_status_gump: bool,
    pub status_and_bar_exclusive: bool,
    pub party_invite_gump: bool,
    pub custom_health_bars: bool,
    pub opaque_health_bars: bool,
    pub save_health_bars: bool,
    pub close_health_bar: CloseHealthBar,
    pub grid_loot: GridLoot,
    pub shift_for_context_menus: bool,
    pub shift_to_split_stacks: bool,
    // Miscellaneous.
    pub circle_of_transparency: bool,
    pub circle_radius: u16,
    pub circle_style: CircleStyle,
    pub hide_screenshot_message: bool,
    pub object_fading: bool,
    pub text_fading: bool,
    pub target_range_indicator: bool,
    pub drag_select_health_bars: bool,
    pub drag_select_key: ModifierKey,
    pub drag_select_humanoids_only: bool,
    pub drag_select_hostiles_only: bool,
    pub drag_select_start_x: f32,
    pub drag_select_start_y: f32,
    pub drag_select_anchored: bool,
    pub stat_change_messages: bool,
    pub skill_change_messages: bool,
    pub skill_change_tenths: u16,
    // Terrain and statics.
    pub hide_roofs: bool,
    pub trees_to_stumps: bool,
    pub hide_vegetation: bool,
    pub mark_cave_tiles: bool,
    pub field_style: FieldStyle,
}

impl Default for GeneralOptions {
    fn default() -> Self {
        Self {
            highlight_objects: false,
            // A double click on the ground walks there, as a modern client
            // does; the reference client keeps it off until asked.
            pathfinding: true,
            shift_pathfinding: false,
            click_to_run: false,
            run_click_key: DEFAULT_RUN_CLICK_KEY,
            wasd_movement: false,
            always_run: false,
            always_run_unless_hidden: false,
            auto_open_doors: false,
            auto_open_corpses: false,
            corpse_open_range: DEFAULT_CORPSE_OPEN_RANGE,
            skip_empty_corpses: false,
            corpse_open_rule: DEFAULT_CORPSE_OPEN_RULE,
            out_of_range_no_color: false,
            sallos_easy_grab: false,
            show_house_content: false,
            show_mobile_hp: false,
            mobile_hp_style: DEFAULT_HP_STYLE,
            mobile_hp_when: DEFAULT_HP_SHOW_WHEN,
            highlight_poisoned: true,
            poisoned_hue: DEFAULT_POISONED_HUE,
            highlight_paralyzed: true,
            paralyzed_hue: DEFAULT_PARALYZED_HUE,
            highlight_invulnerable: true,
            invulnerable_hue: DEFAULT_INVULNERABLE_HUE,
            show_incoming_mobiles: true,
            show_incoming_corpses: true,
            aura_under_feet: DEFAULT_AURA_RULE,
            party_aura: false,
            party_aura_hue: DEFAULT_PARTY_AURA_HUE,
            hidden_layers_enabled: false,
            hide_layers_for_self: true,
            hidden_layers: Vec::new(),
            hide_menu_bar: false,
            alt_right_click_closes_anchored: true,
            alt_moves_gumps: false,
            right_click_closes_anchored_group: false,
            standard_skills_gump: true,
            old_status_gump: false,
            status_and_bar_exclusive: true,
            party_invite_gump: false,
            custom_health_bars: false,
            opaque_health_bars: false,
            save_health_bars: false,
            close_health_bar: DEFAULT_CLOSE_HEALTH_BAR,
            grid_loot: DEFAULT_GRID_LOOT,
            shift_for_context_menus: false,
            shift_to_split_stacks: false,
            circle_of_transparency: false,
            circle_radius: DEFAULT_CIRCLE_RADIUS,
            circle_style: DEFAULT_CIRCLE_STYLE,
            hide_screenshot_message: false,
            object_fading: true,
            text_fading: true,
            target_range_indicator: false,
            drag_select_health_bars: false,
            drag_select_key: DEFAULT_DRAG_SELECT_KEY,
            drag_select_humanoids_only: false,
            drag_select_hostiles_only: false,
            drag_select_start_x: DEFAULT_DRAG_SELECT_START,
            drag_select_start_y: DEFAULT_DRAG_SELECT_START,
            drag_select_anchored: false,
            stat_change_messages: true,
            skill_change_messages: true,
            skill_change_tenths: DEFAULT_SKILL_CHANGE_TENTHS,
            hide_roofs: false,
            trees_to_stumps: false,
            hide_vegetation: false,
            mark_cave_tiles: false,
            field_style: DEFAULT_FIELD_STYLE,
        }
    }
}

impl GeneralOptions {
    /// True when the worn `layer` is not drawn on a mobile. `own` is the
    /// character himself.
    pub fn hides_layer(&self, layer: u8, own: bool) -> bool {
        self.hidden_layers_enabled
            && (own || !self.hide_layers_for_self)
            && self.hidden_layers.contains(&layer)
    }

    /// Hides or shows one worn layer. The list stays in layer order.
    pub fn set_layer_hidden(&mut self, layer: u8, hidden: bool) {
        match (self.hidden_layers.binary_search(&layer), hidden) {
            (Err(at), true) => self.hidden_layers.insert(at, layer),
            (Ok(at), false) => {
                self.hidden_layers.remove(at);
            }
            _ => {}
        }
    }
}

/// The Sound page. Each volume is from 0 to 1, and plays under the master
/// volume. `muted` silences all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundOptions {
    pub muted: bool,
    pub master_volume: f32,
    pub sound_on: bool,
    pub sound_volume: f32,
    pub music_on: bool,
    pub music_volume: f32,
    pub footsteps_on: bool,
    pub footsteps_volume: f32,
    pub combat_music: bool,
    pub play_in_background: bool,
    pub rain_sound: bool,
    pub max_sounds_at_once: u16,
    /// Sound effects that never play.
    pub sound_filter: Vec<u16>,
    /// Music tracks that never play.
    pub music_filter: Vec<u16>,
    /// A SoundFont file for the MIDI music of old clients.
    pub midi_sound_font: Option<PathBuf>,
}

impl Default for SoundOptions {
    fn default() -> Self {
        Self {
            muted: false,
            master_volume: DEFAULT_MASTER_VOLUME,
            sound_on: true,
            sound_volume: DEFAULT_SOUND_VOLUME,
            music_on: true,
            music_volume: DEFAULT_MUSIC_VOLUME,
            footsteps_on: true,
            footsteps_volume: DEFAULT_FOOTSTEPS_VOLUME,
            combat_music: true,
            play_in_background: false,
            rain_sound: true,
            max_sounds_at_once: DEFAULT_MAX_SOUNDS,
            sound_filter: Vec::new(),
            music_filter: Vec::new(),
            midi_sound_font: None,
        }
    }
}

/// The Video page: the window, the game view in it, zoom and lights.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoOptions {
    // Window.
    pub window_mode: WindowMode,
    pub window_width: f32,
    pub window_height: f32,
    pub fps: u16,
    pub reduce_fps_when_inactive: bool,
    pub inactive_fps: u16,
    pub vsync: bool,
    pub ui_scale: f32,
    // Game window.
    pub game_window_full_size: bool,
    pub game_window_locked: bool,
    pub game_window_x: f32,
    pub game_window_y: f32,
    pub game_window_width: f32,
    pub game_window_height: f32,
    // Zoom.
    pub default_zoom: f32,
    pub wheel_zoom: bool,
    pub ctrl_release_restores_zoom: bool,
    pub keep_zoom_after_close: bool,
    // Lights.
    pub alternative_lights: bool,
    pub custom_light_level: bool,
    pub light_level: u8,
    pub light_level_rule: LightLevelRule,
    pub dark_nights: bool,
    pub colored_lights: bool,
    /// Lamps, torches and candles flicker.
    pub candle_flicker: bool,
    // Effects.
    pub death_screen: bool,
    pub black_and_white_when_dead: bool,
    pub aura_on_mouse: bool,
    pub animated_water: bool,
    pub shadows: bool,
    pub statics_shadows: bool,
    pub terrain_shadows_level: u8,
    pub weather_effects: bool,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self {
            window_mode: DEFAULT_WINDOW_MODE,
            window_width: WINDOW_WIDTH,
            window_height: WINDOW_HEIGHT,
            fps: DEFAULT_FPS,
            reduce_fps_when_inactive: true,
            inactive_fps: DEFAULT_INACTIVE_FPS,
            vsync: false,
            ui_scale: DEFAULT_UI_SCALE,
            game_window_full_size: false,
            game_window_locked: false,
            game_window_x: DEFAULT_GAME_WINDOW_X,
            game_window_y: DEFAULT_GAME_WINDOW_Y,
            game_window_width: DEFAULT_GAME_WINDOW_WIDTH,
            game_window_height: DEFAULT_GAME_WINDOW_HEIGHT,
            default_zoom: DEFAULT_ZOOM,
            wheel_zoom: false,
            ctrl_release_restores_zoom: false,
            keep_zoom_after_close: false,
            alternative_lights: false,
            custom_light_level: false,
            light_level: DEFAULT_LIGHT_LEVEL,
            light_level_rule: DEFAULT_LIGHT_LEVEL_RULE,
            dark_nights: false,
            colored_lights: true,
            candle_flicker: false,
            death_screen: true,
            black_and_white_when_dead: true,
            aura_on_mouse: true,
            animated_water: false,
            shadows: true,
            statics_shadows: true,
            terrain_shadows_level: DEFAULT_TERRAIN_SHADOWS,
            weather_effects: true,
        }
    }
}

/// The Macros page: each macro the player made with its key, and the game
/// controller.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MacroOptions {
    pub controller_enabled: bool,
    /// How fast the right stick moves the mouse, from 1 to 20.
    pub controller_mouse_sensitivity: u8,
    pub key_bindings: Vec<KeyBinding>,
}

impl Default for MacroOptions {
    fn default() -> Self {
        Self {
            controller_enabled: false,
            controller_mouse_sensitivity: DEFAULT_CONTROLLER_SENSITIVITY,
            key_bindings: Vec::new(),
        }
    }
}

/// The Tooltip page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TooltipOptions {
    pub enabled: bool,
    pub delay_ms: u16,
    /// Percent of the normal size.
    pub zoom: u16,
    /// Percent.
    pub background_opacity: u8,
    /// `NO_HUE` keeps the font's own color.
    pub text_hue: u16,
    /// The number of the UO Unicode font.
    pub font: u8,
}

impl Default for TooltipOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            delay_ms: DEFAULT_TOOLTIP_DELAY_MS,
            zoom: DEFAULT_TOOLTIP_ZOOM,
            background_opacity: DEFAULT_TOOLTIP_OPACITY,
            text_hue: NO_HUE,
            font: DEFAULT_TOOLTIP_FONT,
        }
    }
}

/// The Fonts page: the UO fonts, and a TrueType font for the Modern style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontOptions {
    pub override_game_font: bool,
    pub game_font_kind: GameFontKind,
    pub force_unicode_journal: bool,
    /// The number of the UO Unicode font for speech.
    pub speech_font: u8,
    /// None keeps the font the window ships with.
    pub truetype_font: Option<PathBuf>,
    pub truetype_size: f32,
}

impl Default for FontOptions {
    fn default() -> Self {
        Self {
            override_game_font: false,
            game_font_kind: DEFAULT_GAME_FONT_KIND,
            force_unicode_journal: false,
            speech_font: DEFAULT_SPEECH_FONT,
            truetype_font: None,
            truetype_size: DEFAULT_TRUETYPE_SIZE,
        }
    }
}

/// The Speech page: speech timing, the chat line, the journal file and the
/// hue of each kind of speech.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechOptions {
    pub speech_delay: u16,
    pub scale_speech_delay: bool,
    pub save_journal: bool,
    pub max_journal_files: u16,
    pub journal_file_with_serial: bool,
    pub chat_on_enter: bool,
    pub chat_prefix_keys: bool,
    pub shift_enter_sends: bool,
    pub hide_chat_gradient: bool,
    pub hide_guild_chat: bool,
    pub hide_alliance_chat: bool,
    pub overhead_party_messages: bool,
    pub speech_hue: u16,
    pub emote_hue: u16,
    pub yell_hue: u16,
    pub whisper_hue: u16,
    pub party_hue: u16,
    pub guild_hue: u16,
    pub alliance_hue: u16,
    pub chat_hue: u16,
}

impl Default for SpeechOptions {
    fn default() -> Self {
        Self {
            speech_delay: DEFAULT_SPEECH_DELAY,
            scale_speech_delay: true,
            save_journal: false,
            max_journal_files: DEFAULT_MAX_JOURNAL_FILES,
            journal_file_with_serial: false,
            chat_on_enter: false,
            chat_prefix_keys: true,
            shift_enter_sends: true,
            hide_chat_gradient: false,
            hide_guild_chat: false,
            hide_alliance_chat: false,
            overhead_party_messages: false,
            speech_hue: DEFAULT_SPEECH_HUE,
            emote_hue: DEFAULT_EMOTE_HUE,
            yell_hue: DEFAULT_YELL_HUE,
            whisper_hue: DEFAULT_WHISPER_HUE,
            party_hue: DEFAULT_PARTY_HUE,
            guild_hue: DEFAULT_GUILD_HUE,
            alliance_hue: DEFAULT_ALLIANCE_HUE,
            chat_hue: DEFAULT_CHAT_HUE,
        }
    }
}

/// The Combat & Spells page: targeting, questions, notoriety and spell
/// hues, and the cooldown bars and cast indicators.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CombatOptions {
    pub new_target_system: bool,
    pub hold_tab_for_combat: bool,
    pub query_before_attack: bool,
    pub query_beneficial_acts: bool,
    pub spell_format_on: bool,
    pub spell_hue_on: bool,
    pub single_click_buttons: bool,
    pub buff_duration: bool,
    pub fast_spell_assign: bool,
    pub dps_with_damage: bool,
    pub innocent_hue: u16,
    pub friend_hue: u16,
    pub criminal_hue: u16,
    pub can_attack_hue: u16,
    pub murderer_hue: u16,
    pub enemy_hue: u16,
    pub benefic_spell_hue: u16,
    pub harmful_spell_hue: u16,
    pub neutral_spell_hue: u16,
    /// `{power}` is the power words, `{spell}` the spell name.
    pub spell_format: String,
    // Cooldowns and indicators.
    pub cooldown_bars: bool,
    pub spell_cast_indicator: bool,
    pub improved_buff_bar: bool,
    pub range_circle: bool,
    pub range_circle_tiles: u8,
    pub range_circle_hue: u16,
    /// The cooldown bars that journal lines start.
    pub cooldowns: Vec<CooldownRule>,
}

/// One cooldown bar: a journal line with its trigger words starts it, and
/// it runs down for its time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CooldownRule {
    pub label: String,
    pub hue: u16,
    /// Words the journal line holds, in any case.
    pub trigger: String,
    pub seconds: f32,
    pub source: CooldownSource,
    /// A new trigger starts the bar again. Without it a running bar stays.
    pub restart: bool,
}

impl Default for CombatOptions {
    fn default() -> Self {
        Self {
            new_target_system: true,
            hold_tab_for_combat: true,
            query_before_attack: true,
            query_beneficial_acts: false,
            spell_format_on: false,
            spell_hue_on: false,
            single_click_buttons: false,
            buff_duration: false,
            fast_spell_assign: false,
            dps_with_damage: true,
            innocent_hue: DEFAULT_INNOCENT_HUE,
            friend_hue: DEFAULT_FRIEND_HUE,
            criminal_hue: DEFAULT_CRIMINAL_HUE,
            can_attack_hue: DEFAULT_CAN_ATTACK_HUE,
            murderer_hue: DEFAULT_MURDERER_HUE,
            enemy_hue: DEFAULT_ENEMY_HUE,
            benefic_spell_hue: DEFAULT_BENEFIC_HUE,
            harmful_spell_hue: DEFAULT_HARMFUL_HUE,
            neutral_spell_hue: DEFAULT_NEUTRAL_HUE,
            spell_format: DEFAULT_SPELL_FORMAT.to_string(),
            cooldown_bars: true,
            spell_cast_indicator: false,
            improved_buff_bar: false,
            range_circle: false,
            range_circle_tiles: DEFAULT_RANGE_CIRCLE_TILES,
            range_circle_hue: DEFAULT_RANGE_CIRCLE_HUE,
            cooldowns: Vec::new(),
        }
    }
}

/// The Counters page: the bar that counts items in the backpack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CounterOptions {
    pub enabled: bool,
    pub highlight_on_change: bool,
    pub abbreviate: bool,
    pub abbreviate_at: u32,
    pub highlight_when_low: bool,
    pub low_amount: u32,
    pub rows: u8,
    pub columns: u8,
    pub cell_size: u8,
    /// The bar takes no dropped items and keeps its cells as they are.
    pub read_only: bool,
    /// The items the bar counts, one in each cell.
    pub items: Vec<CounterItem>,
}

/// One item the counter bar counts: every item of the graphic, or only
/// those of the hue. `NO_HUE` counts every hue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterItem {
    pub label: String,
    pub graphic: u16,
    pub hue: u16,
}

impl Default for CounterOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            highlight_on_change: true,
            abbreviate: false,
            abbreviate_at: DEFAULT_COUNTER_ABBREVIATE_AT,
            highlight_when_low: false,
            low_amount: DEFAULT_COUNTER_LOW_AMOUNT,
            rows: DEFAULT_COUNTER_ROWS,
            columns: DEFAULT_COUNTER_COLUMNS,
            cell_size: DEFAULT_COUNTER_CELL_SIZE,
            read_only: false,
            items: Vec::new(),
        }
    }
}

/// One value on the info bar, with its label and the hue of the label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfoBarItem {
    pub label: String,
    pub hue: u16,
    pub data: InfoBarData,
}

/// The Info Bar page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InfoBarOptions {
    pub enabled: bool,
    pub highlight: InfoBarHighlight,
    pub items: Vec<InfoBarItem>,
}

impl Default for InfoBarOptions {
    fn default() -> Self {
        let item = |label: &str, data, hue| InfoBarItem {
            label: label.to_string(),
            hue,
            data,
        };
        Self {
            enabled: false,
            highlight: DEFAULT_INFO_BAR_HIGHLIGHT,
            items: vec![
                item("", InfoBarData::Name, INFO_BAR_NAME_HUE),
                item(
                    INFO_BAR_WORDS_HITS,
                    InfoBarData::HitPoints,
                    INFO_BAR_HITS_HUE,
                ),
                item(INFO_BAR_WORDS_MANA, InfoBarData::Mana, INFO_BAR_MANA_HUE),
                item(
                    INFO_BAR_WORDS_STAMINA,
                    InfoBarData::Stamina,
                    INFO_BAR_STAMINA_HUE,
                ),
                item(
                    INFO_BAR_WORDS_WEIGHT,
                    InfoBarData::Weight,
                    INFO_BAR_WEIGHT_HUE,
                ),
            ],
        }
    }
}

/// The Containers page: container gumps and the grid containers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContainerOptions {
    pub backpack_style: BackpackStyle,
    /// Percent.
    pub scale: u16,
    pub scale_items: bool,
    pub large_gumps: bool,
    pub double_click_loots: bool,
    pub relative_drag_and_drop: bool,
    pub highlight_on_hover: bool,
    pub hue_gumps: bool,
    pub override_place: bool,
    pub place: ContainerPlace,
    /// The middle of the container gump the player dragged last, in window
    /// points, for the "Last dragged position" place.
    pub last_dragged: Option<(f32, f32)>,
    // Grid containers.
    pub grid_columns: u8,
    pub grid_rows: u8,
    /// Percent.
    pub grid_scale: u16,
    /// Percent.
    pub grid_opacity: u8,
    pub grid_border_hue: u16,
    pub grid_search: GridSearch,
    pub grid_compare_tooltip: bool,
    /// Items with any of these properties are highlighted.
    pub grid_highlight_properties: Vec<String>,
    /// Items that pass a rule are marked in the hue of the rule.
    pub grid_highlight_rules: Vec<HighlightRule>,
    /// The mouse on a bag in a grid shows what the bag holds.
    pub grid_preview: bool,
    /// The bag that "Move to favorite bag" fills.
    pub favorite_bag: Option<u32>,
    /// The slots of each grid container, by the container's serial in hex.
    pub grid_layouts: BTreeMap<String, GridLayout>,
}

/// A property an item must have: its words, and the least number it may
/// carry when the property has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyNeed {
    pub words: String,
    pub min: Option<f32>,
}

/// A rule that marks the items that pass it, as a rarity, a slayer or a
/// resist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HighlightRule {
    pub name: String,
    pub hue: u16,
    pub needs: Vec<PropertyNeed>,
    /// Every need must pass. Without it one is enough.
    pub need_all: bool,
    /// The rule marks the items of corpses only.
    pub corpses_only: bool,
}

/// The slots of one grid container that the player locked: each slot, as
/// its number in text, holds its item.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GridLayout {
    pub locked: BTreeMap<String, u32>,
}

impl Default for ContainerOptions {
    fn default() -> Self {
        Self {
            backpack_style: DEFAULT_BACKPACK_STYLE,
            scale: DEFAULT_CONTAINER_SCALE,
            scale_items: false,
            large_gumps: false,
            double_click_loots: false,
            relative_drag_and_drop: false,
            highlight_on_hover: false,
            hue_gumps: true,
            override_place: false,
            place: DEFAULT_CONTAINER_PLACE,
            last_dragged: None,
            grid_columns: DEFAULT_GRID_COLUMNS,
            grid_rows: DEFAULT_GRID_ROWS,
            grid_scale: DEFAULT_GRID_SCALE,
            grid_opacity: DEFAULT_GRID_OPACITY,
            grid_border_hue: DEFAULT_GRID_BORDER_HUE,
            grid_search: DEFAULT_GRID_SEARCH,
            grid_compare_tooltip: true,
            grid_highlight_properties: Vec::new(),
            grid_highlight_rules: Vec::new(),
            grid_preview: true,
            favorite_bag: None,
            grid_layouts: BTreeMap::new(),
        }
    }
}

/// The Experimental page: turn off the keys the official client has.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperimentalOptions {
    pub disable_default_hotkeys: bool,
    pub disable_arrow_keys: bool,
    pub disable_tab_war_mode: bool,
    pub disable_message_history: bool,
    pub disable_click_automove: bool,
}

/// The Ignore List page: the names of players whose speech is hidden.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IgnoreOptions {
    pub names: Vec<String>,
}

impl IgnoreOptions {
    /// Puts a name on the list. False when it is empty or there already.
    pub fn add(&mut self, name: &str) -> bool {
        let name = name.trim();
        let new = !name.is_empty() && !self.names.iter().any(|kept| kept == name);
        if new {
            self.names.push(name.to_string());
        }
        new
    }

    /// Takes a name off the list. False when it was not there.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.names.len();
        self.names.retain(|kept| kept != name);
        self.names.len() != before
    }
}

/// The Interface page: the look of the play window and of its gumps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceOptions {
    pub ui_style: UiStyle,
    /// Percent.
    pub gump_opacity: u8,
    pub remember_gump_places: bool,
    pub durability_bars: bool,
    /// Percent of durability under which an item warns.
    pub durability_warning: u8,
    pub title_bar_stats: bool,
    pub title_bar_mode: TitleStats,
    /// The Modern panels that open when the window starts.
    pub open_panels: Vec<String>,
    pub screenshot_on_death: bool,
    pub nearby_loot_window: bool,
}

impl Default for InterfaceOptions {
    fn default() -> Self {
        Self {
            ui_style: DEFAULT_UI_STYLE,
            gump_opacity: DEFAULT_GUMP_OPACITY,
            remember_gump_places: true,
            durability_bars: true,
            durability_warning: DEFAULT_DURABILITY_WARNING,
            title_bar_stats: false,
            title_bar_mode: DEFAULT_TITLE_STATS,
            open_panels: Vec::new(),
            screenshot_on_death: false,
            nearby_loot_window: false,
        }
    }
}

/// The Nameplates page: names over things.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NameplateOptions {
    pub enabled: bool,
    pub filter: NameplateFilter,
    pub health_bar: bool,
    /// Percent.
    pub opacity: u8,
    pub hide_at_full_health: bool,
    pub avoid_overlap: bool,
}

impl Default for NameplateOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            filter: DEFAULT_NAMEPLATE_FILTER,
            health_bar: true,
            opacity: DEFAULT_NAMEPLATE_OPACITY,
            hide_at_full_health: false,
            avoid_overlap: true,
        }
    }
}

/// One tab of the journal and the kinds of lines it shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalTab {
    pub name: String,
    pub kinds: Vec<JournalKind>,
}

/// The Journal page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JournalOptions {
    pub tabs: Vec<JournalTab>,
    pub show_client_lines: bool,
    pub show_object_lines: bool,
    pub show_system_lines: bool,
    pub show_guild_and_alliance: bool,
    pub dark_mode: bool,
    /// Percent.
    pub opacity: u8,
    pub hide_timestamps: bool,
    pub max_lines: u16,
    /// The Classic style shows the resizable journal with tabs instead of
    /// the scroll of paper, as the reference client's "Use alternate journal".
    pub alternate_journal: bool,
}

impl Default for JournalOptions {
    fn default() -> Self {
        use JournalKind::*;
        let tab = |name: &str, kinds: &[JournalKind]| JournalTab {
            name: name.to_string(),
            kinds: kinds.to_vec(),
        };
        Self {
            tabs: vec![
                tab(
                    JOURNAL_TAB_ALL,
                    &[
                        Speech, Emote, Whisper, Yell, System, Label, Spell, Party, Guild, Alliance,
                        Chat,
                    ],
                ),
                tab(
                    JOURNAL_TAB_CHAT,
                    &[Speech, Emote, Whisper, Yell, Party, Guild, Alliance, Chat],
                ),
                tab(JOURNAL_TAB_GROUPS, &[Party, Guild, Alliance]),
                tab(JOURNAL_TAB_SYSTEM, &[System]),
            ],
            show_client_lines: true,
            show_object_lines: true,
            show_system_lines: true,
            show_guild_and_alliance: true,
            dark_mode: false,
            opacity: DEFAULT_JOURNAL_OPACITY,
            hide_timestamps: false,
            max_lines: DEFAULT_JOURNAL_MAX_LINES,
            alternate_journal: false,
        }
    }
}

/// The World Map page: what the big map shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldMapOptions {
    pub show_party: bool,
    pub show_coordinates: bool,
    pub show_mouse_coordinates: bool,
    pub sextant_coordinates: bool,
    pub show_mobiles: bool,
    pub show_player_name: bool,
    pub show_player_bar: bool,
    pub show_group_names: bool,
    pub show_group_bars: bool,
    pub show_markers: bool,
    pub show_marker_names: bool,
    pub show_multis: bool,
    pub grid_when_zoomed: bool,
    pub free_view: bool,
    pub always_on_top: bool,
    /// The map shows the whole facet, not the land round the character.
    pub whole_world: bool,
    /// The Classic world map is turned as the play field is turned.
    pub flip_map: bool,
    /// A click on the Classic world map answers a target cursor that asks
    /// for a place.
    pub allow_positional_target: bool,
    /// The zoom step of the Classic world map, from its list of zooms.
    pub zoom_step: u8,
    /// The font style of the marker names, from 1 to 6, as the
    /// reference client offers six.
    pub marker_font_style: u8,
    /// The Classic minimap shows its large picture.
    pub minimap_large: bool,
    /// Marker files whose markers the map hides.
    pub hidden_marker_files: Vec<String>,
    /// Zone files whose zones the map hides.
    pub hidden_zone_files: Vec<String>,
}

impl Default for WorldMapOptions {
    fn default() -> Self {
        Self {
            show_party: true,
            show_coordinates: true,
            show_mouse_coordinates: true,
            sextant_coordinates: false,
            show_mobiles: true,
            show_player_name: true,
            show_player_bar: true,
            show_group_names: true,
            show_group_bars: true,
            show_markers: true,
            show_marker_names: true,
            show_multis: true,
            grid_when_zoomed: true,
            free_view: false,
            always_on_top: false,
            whole_world: false,
            flip_map: true,
            allow_positional_target: false,
            zoom_step: DEFAULT_WORLD_MAP_ZOOM_STEP,
            marker_font_style: DEFAULT_MARKER_FONT_STYLE,
            minimap_large: false,
            hidden_marker_files: Vec::new(),
            hidden_zone_files: Vec::new(),
        }
    }
}

/// The Agents page: which agent windows the play window offers. The agents
/// themselves and their settings live in the session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentPanelOptions {
    pub loot: bool,
    pub scavenger: bool,
    pub bandage: bool,
    pub self_heal: bool,
    pub buy: bool,
    pub sell: bool,
    pub dress: bool,
    pub organizer: bool,
    pub skinning: bool,
    pub remount: bool,
    pub friends: bool,
}

impl Default for AgentPanelOptions {
    fn default() -> Self {
        Self {
            loot: true,
            scavenger: true,
            bandage: true,
            self_heal: true,
            buy: true,
            sell: true,
            dress: true,
            organizer: true,
            skinning: true,
            remount: true,
            friends: true,
        }
    }
}
