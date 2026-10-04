//! The windows of the launcher that read the session or the link: the
//! damage meter, the durability of what the character wears, the network
//! statistics and the debug window. Each shows while the launcher has it
//! open, as in the Rust window; what each reads and shows is
//! `uoterm_view::ui::{meters, info_bar}`'.

use super::{
    Colored, FrameSpec, Framed, PANEL_DEBUG, PANEL_DPS, PANEL_DURABILITY, PANEL_NET_STATS,
};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Point, Vector};
use uoterm_view::model::dps::DamageReport;
use uoterm_view::model::durability;
use uoterm_view::model::places;
use uoterm_view::model::stats::{self, DebugFacts};
use uoterm_view::ui::info_bar::{stats_first_place, stats_title, StatsLook, StatsPanel, HINT_MORE};
use uoterm_view::ui::launch::{DEBUG_ID, DPS_ID, DURABILITY_ID, NET_STATS_ID};
use uoterm_view::ui::lists::wear_color;
use uoterm_view::ui::meters::{
    dealt_words, dps_buttons, dps_first_place, durability_first_place, meter_key, per_second_color,
    per_second_words, total_words, wear_words, DPS_MAX_AGE, DPS_MAX_ROWS, DURABILITY_ART,
    DURABILITY_MAX_ROWS, WORDS_DAMAGE, WORDS_DURABILITY, WORDS_NO_DAMAGE, WORDS_NO_WEAR,
};
use uoterm_view::ui::theme::css_color;
use uoterm_world::tool_names::TOOL_DAMAGE_METER;

/// The words of the statistics as they show, and the facts of the debug
/// window from the last frame.
pub(crate) struct MetersState {
    look: StatsLook,
    facts: DebugFacts,
    /// The clock of the last frame, for the frames each second.
    last: Option<f64>,
}

impl Default for MetersState {
    fn default() -> Self {
        Self {
            look: StatsLook::default(),
            facts: DebugFacts {
                fps: 0.0,
                zoom: 0.0,
                selected: None,
            },
            last: None,
        }
    }
}

/// The damage meter.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DpsData {
    /// Start, Pause or Resume, and Stop, while the human has control.
    pub buttons: Vec<&'static str>,
    pub per_second: Colored,
    pub total: String,
    pub rows: Vec<DealtRow>,
    pub none: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DealtRow {
    pub name: String,
    pub words: String,
}

/// The worn items with a durability, the most worn first.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DurabilityData {
    pub art: f32,
    pub rows: Vec<WearRow>,
    pub none: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WearRow {
    pub picture: Option<String>,
    pub name: String,
    pub words: Colored,
    pub fill: f32,
}

/// The words of a statistics panel, in their color.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StatsData {
    pub words: Colored,
    pub hint: &'static str,
}

/// `{"button": i}` presses a button of the damage meter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DpsAction {
    Button(usize),
}

/// `{"double": true}` shows the other words of a statistics panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StatsAction {
    Double(bool),
}

impl WebView {
    /// The facts of the debug window in one frame: the frames each second,
    /// the zoom, and the thing under the mouse.
    pub(crate) fn follow_meters(&mut self, now: f64, mouse: Option<Point>) {
        let meters = &mut self.panels.meters;
        let seconds = meters.last.map_or(0.0, |last| (now - last).max(0.0)) as f32;
        meters.last = Some(now);
        meters.facts = DebugFacts {
            fps: stats::fps(seconds),
            zoom: self.scene.zoom(),
            selected: mouse
                .and_then(|at| self.scene.thing_at(at))
                .map(|thing| thing.serial),
        };
    }

    /// The damage meter as the session reads it now.
    fn damage_report(&mut self) -> DamageReport {
        self.hand
            .reads()
            .want(meter_key(), DPS_MAX_AGE)
            .map(DamageReport::read)
            .unwrap_or_default()
    }

    pub(super) fn dps_spec(&mut self) -> Option<FrameSpec> {
        if !places::is_open(&self.profile, DPS_ID) {
            return None;
        }
        let report = self.damage_report();
        let default = dps_first_place(self.panel_room(), &report);
        Some(FrameSpec::fixed(DPS_ID, WORDS_DAMAGE, default).closable())
    }

    pub(super) fn dps_data(&mut self, frame: &WatchFrame) -> Option<Framed<DpsData>> {
        let spec = self.dps_spec()?;
        let report = self.damage_report();
        let body = DpsData {
            buttons: if frame.human_control {
                dps_buttons(&report)
                    .into_iter()
                    .map(|(words, _)| words)
                    .collect()
            } else {
                Vec::new()
            },
            per_second: Colored {
                words: per_second_words(&report),
                color: css_color(per_second_color(&report)),
            },
            total: total_words(&report),
            rows: report
                .mobiles
                .iter()
                .take(DPS_MAX_ROWS)
                .map(|dealt| DealtRow {
                    name: dealt.name.clone(),
                    words: dealt_words(dealt),
                })
                .collect(),
            none: report.mobiles.is_empty().then_some(WORDS_NO_DAMAGE),
        };
        Some(self.framed(PANEL_DPS, &spec, body))
    }

    pub(super) fn dps_action(&mut self, action: Value) {
        if !self.frame.as_ref().is_some_and(|frame| frame.human_control) {
            return;
        }
        let Ok(DpsAction::Button(at)) = serde_json::from_value::<DpsAction>(action) else {
            return;
        };
        let report = self.damage_report();
        if let Some((_, act)) = dps_buttons(&report).into_iter().nth(at) {
            self.hand.act(act);
            self.hand.reads().refresh(TOOL_DAMAGE_METER);
        }
    }

    pub(super) fn durability_spec(&mut self, frame: &WatchFrame) -> Option<FrameSpec> {
        if !places::is_open(&self.profile, DURABILITY_ID) {
            return None;
        }
        let rows = durability::worn_wear(frame, self.hand.reads()).len();
        let default = durability_first_place(self.panel_room(), rows);
        Some(FrameSpec::fixed(DURABILITY_ID, WORDS_DURABILITY, default).closable())
    }

    pub(super) fn durability_data(&mut self, frame: &WatchFrame) -> Option<Framed<DurabilityData>> {
        let spec = self.durability_spec(frame)?;
        let warning = self.profile.interface.durability_warning;
        let wears = durability::worn_wear(frame, self.hand.reads());
        let rows = wears
            .iter()
            .take(DURABILITY_MAX_ROWS)
            .map(|wear| {
                let request = self.item_picture_request(wear.graphic, wear.hue);
                WearRow {
                    picture: Some(self.picture_key(&request)),
                    name: wear.name.clone(),
                    words: Colored {
                        words: wear_words(wear),
                        color: css_color(wear_color(wear, warning)),
                    },
                    fill: wear.share(),
                }
            })
            .collect();
        let body = DurabilityData {
            art: DURABILITY_ART,
            rows,
            none: wears.is_empty().then_some(WORDS_NO_WEAR),
        };
        Some(self.framed(PANEL_DURABILITY, &spec, body))
    }

    /// The words of a statistics panel now, and their size in the font of
    /// the panels.
    fn stats_words(&self, panel: StatsPanel, frame: &WatchFrame) -> (Colored, Vector) {
        let meters = &self.panels.meters;
        let (words, color) = meters.look.words(panel, frame, &meters.facts);
        let measure = self.body_measure.as_ref();
        let size = words.lines().fold(Vector::new(0.0, 0.0), |size, line| {
            let line = measure.map_or(Vector::new(0.0, 0.0), |measure| measure(line));
            Vector::new(size.x.max(line.x), size.y + line.y)
        });
        let words = Colored {
            words,
            color: css_color(color),
        };
        (words, size)
    }

    pub(super) fn stats_spec(&self, panel: StatsPanel, frame: &WatchFrame) -> Option<FrameSpec> {
        let id = stats_id(panel);
        if !places::is_open(&self.profile, id) {
            return None;
        }
        let (_, size) = self.stats_words(panel, frame);
        let default = stats_first_place(self.panel_room(), panel, size);
        Some(FrameSpec::fixed(id, stats_title(panel), default).closable())
    }

    pub(super) fn stats_data(
        &self,
        panel: StatsPanel,
        frame: &WatchFrame,
    ) -> Option<Framed<StatsData>> {
        let spec = self.stats_spec(panel, frame)?;
        let (words, _) = self.stats_words(panel, frame);
        let body = StatsData {
            words,
            hint: HINT_MORE,
        };
        Some(self.framed(stats_panel_name(panel), &spec, body))
    }

    pub(super) fn stats_action(&mut self, panel: StatsPanel, action: Value) {
        if let Ok(StatsAction::Double(_)) = serde_json::from_value::<StatsAction>(action) {
            self.panels.meters.look.flip(panel);
        }
    }
}

/// The id of the place of a statistics panel.
fn stats_id(panel: StatsPanel) -> &'static str {
    match panel {
        StatsPanel::Network => NET_STATS_ID,
        StatsPanel::Debug => DEBUG_ID,
    }
}

/// The name of a statistics panel in its `Panel` events.
pub(super) fn stats_panel_name(panel: StatsPanel) -> &'static str {
    match panel {
        StatsPanel::Network => PANEL_NET_STATS,
        StatsPanel::Debug => PANEL_DEBUG,
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, view_with};
    use super::*;
    use serde_json::json;

    /// A view with control or not, and the damage meter open.
    fn meter_open(control: bool) -> WebView {
        let mut view = view_with("human_control", json!(control), control);
        places::set_open(&mut view.profile, DPS_ID, true);
        view
    }

    #[test]
    fn the_buttons_of_the_meter_make_the_acts_of_the_window() {
        let mut view = meter_open(true);
        let data = view.panel_data(0.0).dps.unwrap().body;
        assert_eq!(data.buttons.len(), 3);
        let report = DamageReport::default();
        for (at, (_, act)) in dps_buttons(&report).into_iter().enumerate() {
            let acts = out_acts(&press(&mut view, PANEL_DPS, json!({ "button": at })));
            assert_eq!(acts, vec![act.for_page()]);
        }
        assert_eq!(data.none, Some(WORDS_NO_DAMAGE));
    }

    #[test]
    fn the_meter_acts_not_without_control() {
        let mut view = meter_open(false);
        assert!(view.panel_data(0.0).dps.unwrap().body.buttons.is_empty());
        let acts = out_acts(&press(&mut view, PANEL_DPS, json!({ "button": 0 })));
        assert!(acts.is_empty());
    }

    #[test]
    fn a_closed_window_shuts_by_its_mark_and_a_double_click_shows_more() {
        let mut view = meter_open(true);
        places::set_open(&mut view.profile, DEBUG_ID, true);
        let short = view.panel_data(0.0).debug.unwrap().body.words.words;
        press(&mut view, PANEL_DEBUG, json!({ "double": true }));
        let full = view.panel_data(0.0).debug.unwrap().body.words.words;
        assert!(full.lines().count() > short.lines().count());
        press(&mut view, PANEL_DPS, json!({ "close": true }));
        assert!(view.panel_data(0.0).dps.is_none());
        assert!(!places::is_open(&view.profile, DPS_ID));
    }
}
