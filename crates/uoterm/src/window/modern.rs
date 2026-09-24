//! The extras of the Modern style: the journal with tabs, the
//! nearby-loot, durability, damage and agent windows, the cooldown bars,
//! the cast indicator, the counter bar, the buff bar and the info bar, the
//! radar, the near list and the health bars, the network statistics and
//! the debug window, and the question before the game quits, with the UI
//! scale, the font, the opacity and the title-bar stats. Each panel draws
//! here; what it shows comes from `model`.

pub mod abilities_ui;
mod agents_ui;
pub mod arrow_ui;
mod ask_ui;
mod bars_ui;
mod buff_ui;
mod combat_ui;
mod counter_ui;
mod dps_ui;
mod durability_ui;
mod dye_ui;
mod entry_ui;
pub mod frame;
mod grid_ui;
pub mod hue_ui;
mod info_ui;
mod journal_ui;
pub mod layout;
mod loot_ui;
mod markers_ui;
pub mod party_ui;
mod race_ui;
mod radar_ui;
mod rows;
pub mod skills_ui;
pub mod spells_ui;
mod stats_ui;
pub mod status_ui;
#[cfg(test)]
pub mod testing;
mod tip_ui;

pub use durability_ui::wear_color;
pub use dye_ui::DyeUi;
pub use entry_ui::EntryUi;
pub use grid_ui::GridUi;
pub use journal_ui::JOURNAL_ID;
pub use markers_ui::{MarkersAsk, MarkersUi};
pub use race_ui::RaceUi;
pub use radar_ui::RADAR_ID;
pub use tip_ui::TipUi;

use super::boxes_ui::Tools;
use super::model::agents::AgentPanel;
use super::model::info_bar::title_words;
use super::model::{fonts, journal, places};
use super::settings::{
    Profile, UiStyle, DEFAULT_GUMP_OPACITY, DEFAULT_TRUETYPE_SIZE, DEFAULT_UI_SCALE,
};
use super::theme;
use crate::view::{WatchFrame, WINDOW_TITLE};
use agents_ui::{AgentsUi, AGENTS_ID, IGNORE_ID};
use ask_ui::{Asked, Question};
use bars_ui::BarsUi;
use combat_ui::CombatUi;
use counter_ui::CounterUi;
use eframe::egui::{self, Id, Rect, Vec2, ViewportCommand};
use frame::{FrameEvent, PanelSpec};
use journal_ui::JournalUi;
use layout::Spot;
use loot_ui::LootUi;
use radar_ui::RadarUi;
use stats_ui::{StatsUi, DEBUG_ID, NET_STATS_ID};
use std::path::PathBuf;

const LAUNCHER_ID: &str = "modern:launcher";
const LAUNCHER_WIDTH: f32 = 220.0;
const LAUNCHER_COLUMNS: usize = 2;
const LAUNCHER_ROW: f32 = 28.0;
const LAUNCHER_GAP: f32 = 6.0;

/// The words of the launcher, and of the button of the control bar that
/// opens it.
pub const WORDS_LAUNCHER: &str = "Panels";
const WORDS_LOOT: &str = "Loot";
const WORDS_DURABILITY: &str = "Durability";
const WORDS_DAMAGE: &str = "Damage";
const WORDS_AGENTS: &str = "Agents";
const WORDS_COUNTERS: &str = "Counters";
const WORDS_INFO: &str = "Info bar";
const WORDS_RADAR: &str = "Radar";
const WORDS_JOURNAL: &str = "Journal";
const WORDS_BUFFS: &str = "Buffs";
const WORDS_NET_STATS: &str = "Network";
const WORDS_DEBUG: &str = "Debug";

/// The panels a close of every window closes, by the ids that keep them
/// open.
const CLOSING_PANELS: [&str; 6] = [
    durability_ui::DURABILITY_ID,
    dps_ui::DPS_ID,
    AGENTS_ID,
    IGNORE_ID,
    NET_STATS_ID,
    DEBUG_ID,
];

/// The look the window has now, so it changes only when the profile does.
/// The Classic style scales and letters its own gumps, so it keeps the
/// plain look.
#[derive(Clone, PartialEq)]
struct Look {
    ui_scale: f32,
    opacity: u8,
    font: Option<PathBuf>,
    font_size: f32,
}

impl Look {
    fn of(profile: &Profile) -> Self {
        if profile.interface.ui_style != UiStyle::Modern {
            return Self {
                ui_scale: DEFAULT_UI_SCALE,
                opacity: DEFAULT_GUMP_OPACITY,
                font: None,
                font_size: DEFAULT_TRUETYPE_SIZE,
            };
        }
        Self {
            ui_scale: profile.video.ui_scale,
            opacity: profile.interface.gump_opacity,
            font: profile.fonts.truetype_font.clone(),
            font_size: profile.fonts.truetype_size,
        }
    }
}

/// What the Modern panels tell the window after they are drawn.
pub struct ModernDrawn {
    pub covered: Vec<Rect>,
    /// The row of the chat box, at the bottom of the journal.
    pub chat_row: Rect,
}

/// A button of the launcher: its words, and whether its panel shows.
#[derive(Clone, Copy)]
enum Launch {
    Loot,
    Durability,
    Damage,
    Agents,
    Counters,
    InfoBar,
    Radar,
    Journal,
    Buffs,
    NetStats,
    Debug,
}

const LAUNCHES: [(Launch, &str); 11] = [
    (Launch::Radar, WORDS_RADAR),
    (Launch::Journal, WORDS_JOURNAL),
    (Launch::Loot, WORDS_LOOT),
    (Launch::Durability, WORDS_DURABILITY),
    (Launch::Damage, WORDS_DAMAGE),
    (Launch::Agents, WORDS_AGENTS),
    (Launch::Counters, WORDS_COUNTERS),
    (Launch::InfoBar, WORDS_INFO),
    (Launch::Buffs, WORDS_BUFFS),
    (Launch::NetStats, WORDS_NET_STATS),
    (Launch::Debug, WORDS_DEBUG),
];

impl Launch {
    fn shows(self, profile: &Profile) -> bool {
        match self {
            Self::Loot => profile.interface.nearby_loot_window,
            Self::Durability => places::is_open(profile, durability_ui::DURABILITY_ID),
            Self::Damage => places::is_open(profile, dps_ui::DPS_ID),
            Self::Agents => places::is_open(profile, AGENTS_ID),
            Self::Counters => profile.counters.enabled,
            Self::InfoBar => profile.info_bar.enabled,
            Self::Radar => !places::is_shut(profile, RADAR_ID),
            Self::Journal => !places::is_shut(profile, JOURNAL_ID),
            Self::Buffs => profile.combat.improved_buff_bar,
            Self::NetStats => places::is_open(profile, NET_STATS_ID),
            Self::Debug => places::is_open(profile, DEBUG_ID),
        }
    }

    fn set(self, profile: &mut Profile, shows: bool) {
        match self {
            Self::Loot => profile.interface.nearby_loot_window = shows,
            Self::Durability => places::set_open(profile, durability_ui::DURABILITY_ID, shows),
            Self::Damage => places::set_open(profile, dps_ui::DPS_ID, shows),
            Self::Agents => places::set_open(profile, AGENTS_ID, shows),
            Self::Counters => profile.counters.enabled = shows,
            Self::InfoBar => profile.info_bar.enabled = shows,
            Self::Radar => places::set_shut(profile, RADAR_ID, !shows),
            Self::Journal => places::set_shut(profile, JOURNAL_ID, !shows),
            Self::Buffs => profile.combat.improved_buff_bar = shows,
            Self::NetStats => places::set_open(profile, NET_STATS_ID, shows),
            Self::Debug => places::set_open(profile, DEBUG_ID, shows),
        }
    }
}

#[derive(Default)]
pub struct ModernUi {
    journal: JournalUi,
    loot: LootUi,
    combat: CombatUi,
    counters: CounterUi,
    agents: AgentsUi,
    bars: BarsUi,
    radar: RadarUi,
    stats: StatsUi,
    /// The question that waits for Yes or No.
    question: Option<Question>,
    /// The launcher shows: the control bar opens and closes it.
    launcher_open: bool,
    look: Option<Look>,
    title: String,
}

impl ModernUi {
    /// Gives the window the UI scale, the panel opacity and the font of
    /// the profile in the Modern style, when they changed.
    pub fn apply_look(&mut self, ctx: &egui::Context, profile: &Profile) {
        let look = Look::of(profile);
        if self.look.as_ref() == Some(&look) {
            return;
        }
        ctx.set_zoom_factor(look.ui_scale);
        theme::set_panel_opacity(look.opacity);
        let font = look.font.as_deref().and_then(|chosen| {
            let dir = fonts::fonts_dir();
            let bytes = fonts::load(&dir, chosen);
            if bytes.is_none() {
                let there: Vec<String> = fonts::fonts_in(&dir)
                    .iter()
                    .map(|font| font.display().to_string())
                    .collect();
                tracing::warn!(
                    font = %chosen.display(),
                    ?there,
                    "the chosen font does not read; these are in the Fonts folder"
                );
            }
            bytes.map(|bytes| (bytes, look.font_size / DEFAULT_TRUETYPE_SIZE))
        });
        theme::use_player_font(ctx, font);
        self.look = Some(look);
    }

    /// Draws every Modern panel that shows. Gives the places they cover
    /// and the row of the chat box.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> ModernDrawn {
        self.journal.take(frame, profile);
        self.combat.observe(frame, profile, tools.time);
        self.loot.open_corpses(frame, tools, profile);
        self.title_bar(ui.ctx(), frame, profile);
        let mut covered: Vec<Rect> = self
            .launcher(ui, rect, tools, profile)
            .into_iter()
            .collect();
        covered.extend(self.radar.draw(ui, rect, frame, tools, profile));
        covered.extend(self.stats.draw(ui, rect, frame, tools, profile));
        if profile.info_bar.enabled {
            covered.push(info_ui::draw(ui, rect, frame, tools, profile));
        }
        if profile.combat.improved_buff_bar {
            covered.extend(buff_ui::draw(ui, rect, frame, tools, profile));
        }
        if profile.combat.cooldown_bars {
            covered.extend(self.combat.cooldown_bars(ui, rect, tools, profile));
        }
        if profile.combat.spell_cast_indicator {
            covered.extend(self.combat.cast_indicator(ui, rect, frame, tools, profile));
        }
        if profile.counters.enabled {
            covered.push(self.counters.draw(ui, rect, frame, tools, profile));
        }
        if profile.interface.nearby_loot_window {
            let (panel, closed) = self.loot.draw(ui, rect, frame, tools, profile);
            covered.push(panel);
            if closed {
                Launch::Loot.set(profile, false);
                tools.keep_profile(profile);
            }
        }
        for (id, draw) in [
            (
                durability_ui::DURABILITY_ID,
                durability_ui::draw as PanelDraw,
            ),
            (dps_ui::DPS_ID, dps_ui::draw as PanelDraw),
        ] {
            if places::is_open(profile, id) {
                let (panel, closed) = draw(ui, rect, frame, tools, profile);
                covered.push(panel);
                if closed {
                    places::set_open(profile, id, false);
                    tools.keep_profile(profile);
                }
            }
        }
        covered.extend(self.agents.draw(ui, rect, frame, tools, profile));
        let journal = self.journal.draw(ui, rect, frame, tools, profile);
        covered.push(journal.panel);
        if let Some(name) = journal.delete_tab {
            self.question = Some(Question::delete_journal_tab(&name));
        }
        covered.extend(self.bars.draw(ui, rect, frame, tools, profile));
        covered.extend(self.question(ui, rect, tools, profile));
        ModernDrawn {
            covered,
            chat_row: journal.chat_row,
        }
    }

    /// True while the launcher shows.
    pub fn launcher_shows(&self) -> bool {
        self.launcher_open
    }

    /// Opens the launcher, or closes it.
    pub fn toggle_launcher(&mut self) {
        self.launcher_open = !self.launcher_open;
    }

    /// Asks before the game quits, as the classic client does.
    pub fn ask_quit(&mut self) {
        self.question = Some(Question::quit());
    }

    /// Closes the health bars of their own, or only those of mobiles out
    /// of view.
    pub fn close_health_bars(
        &mut self,
        frame: &WatchFrame,
        inactive_only: bool,
        profile: &mut Profile,
    ) {
        self.bars.close_bars(frame, inactive_only, profile);
    }

    /// Closes every panel of its own that closes: the launcher, the loot,
    /// durability, damage and agent windows, the network statistics and
    /// the debug window, the health bars and the question. The journal,
    /// the radar and the bars the pages switch on stay, as in the classic
    /// client.
    pub fn close_all(&mut self, frame: &WatchFrame, profile: &mut Profile) {
        self.launcher_open = false;
        profile.interface.nearby_loot_window = false;
        for id in CLOSING_PANELS {
            places::set_open(profile, id, false);
        }
        for agent in AgentPanel::ALL {
            places::set_open(profile, &agent.place_id(), false);
        }
        self.bars.close_bars(frame, false, profile);
        self.question = None;
    }

    /// The question, while it waits. Does what its Yes asks.
    fn question(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let question = self.question.as_ref()?;
        let (panel, answer) = ask_ui::draw(ui, rect, question, tools, profile);
        let yes = answer?;
        let asked = self.question.take().map(|question| question.asked);
        match asked.filter(|_| yes) {
            Some(Asked::Quit) => tools.hand.quit(ui.ctx()),
            Some(Asked::DeleteJournalTab(name)) => {
                journal::delete_tab(&mut profile.journal.tabs, &name);
                tools.keep_profile(profile);
            }
            None => {}
        }
        Some(panel)
    }

    /// The small panel that opens and closes the others, while it shows.
    /// Gives its place.
    fn launcher(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        if !self.launcher_open {
            return None;
        }
        let rows = LAUNCHES.len().div_ceil(LAUNCHER_COLUMNS) as f32;
        let height = frame::TITLE_ROW + rows * LAUNCHER_ROW + theme::PANEL_PAD * 2.0;
        let spec = PanelSpec {
            id: LAUNCHER_ID,
            title: WORDS_LAUNCHER,
            default: layout::first_place(rect, Spot::Launcher, Vec2::new(LAUNCHER_WIDTH, height)),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_LAUNCHER);
        let width = (body.width() - LAUNCHER_GAP) / LAUNCHER_COLUMNS as f32;
        let mut changed = false;
        for (at, (launch, words)) in LAUNCHES.into_iter().enumerate() {
            let (column, row) = (at % LAUNCHER_COLUMNS, at / LAUNCHER_COLUMNS);
            let button = Rect::from_min_size(
                body.left_top()
                    + Vec2::new(
                        column as f32 * (width + LAUNCHER_GAP),
                        row as f32 * LAUNCHER_ROW,
                    ),
                Vec2::new(width, LAUNCHER_ROW - LAUNCHER_GAP),
            );
            let shows = launch.shows(profile);
            let color = if shows { theme::GOAL } else { theme::TEXT_DIM };
            if theme::segment_keyed(ui, button, Id::new(("launch", at)), words, color) {
                launch.set(profile, !shows);
                changed = true;
            }
        }
        if changed {
            tools.keep_profile(profile);
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.launcher_open = false;
        }
        Some(panel)
    }

    /// Puts the vitals in the title of the window when the Interface page
    /// asks for it, and takes them out when it does not.
    fn title_bar(&mut self, ctx: &egui::Context, frame: &WatchFrame, profile: &Profile) {
        let interface = &profile.interface;
        let title = if interface.title_bar_stats && !frame.name.is_empty() {
            title_words(WINDOW_TITLE, frame, interface.title_bar_mode)
        } else {
            WINDOW_TITLE.to_string()
        };
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

/// A panel that draws on its own and may be closed.
type PanelDraw = fn(&egui::Ui, Rect, &WatchFrame, &mut Tools<'_>, &mut Profile) -> (Rect, bool);

#[cfg(test)]
mod tests {
    use super::testing::{draw_frames, SCREEN};
    use super::*;
    use eframe::egui::Pos2;

    #[test]
    fn the_launcher_is_shut_at_first_and_opens_under_the_control_bar() {
        let mut modern = ModernUi::default();
        let mut profile = Profile::default();
        let mut drawn = Vec::new();
        let mut draw = |modern: &mut ModernUi, profile: &mut Profile| {
            draw_frames(profile, &[Vec::new()], |ui, rect, tools, profile| {
                drawn.push(modern.launcher(ui, rect, tools, profile));
            });
        };
        draw(&mut modern, &mut profile);
        modern.toggle_launcher();
        assert!(modern.launcher_shows());
        draw(&mut modern, &mut profile);
        let window = Rect::from_min_size(Pos2::ZERO, SCREEN);
        let spot = layout::first_place(window, Spot::Launcher, Vec2::splat(1.0)).min;
        assert_eq!(drawn[0], None, "shut at the first start");
        assert_eq!(drawn[1].map(|panel| panel.min), Some(spot));
        modern.close_all(&WatchFrame::default(), &mut profile);
        assert!(!modern.launcher_shows(), "a close of every window shuts it");
    }
}
