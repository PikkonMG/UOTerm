//! The health bars of the Modern style: the near list of every mobile in
//! view, the bars of their own a row pulled out of the list or a mobile
//! pulled off the map opens, the bars a box drawn on the map opens, and
//! the target bar that follows the last target. What each bar shows and
//! does is `model::health_bars`, as in the Rust window.

use super::{Colored, DropZone, FrameSpec, Framed, TipKey, PANEL_HEALTH_PREFIX, PANEL_NEAR};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::clicks::PickKind;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point};
use uoterm_view::model::health_bars::{
    self, BarFacts, FirstFrame, MapAsk, MapBars, MapDrag, Pointer, RangeWatch, Subject, TargetBar,
    SPELL_CURE, SPELL_GREATER_HEAL,
};
use uoterm_view::model::places;
use uoterm_view::ui::bars::{
    bar_id, bar_size, close_bars, hits_color, line_count, near_list_size, party_buttons,
    restore_bars, subject_bar_id, HINT_ROW, HINT_TARGETING, NEAR_ID, NEAR_LEAST, WORDS_CLOSE_BAR,
    WORDS_CURE, WORDS_HEAL, WORDS_NEAR, WORDS_NOBODY, WORDS_RENAME, WORDS_TARGET,
};
use uoterm_view::ui::layout::{first_place, Spot};
use uoterm_view::ui::ring::Subject as RingSubject;
use uoterm_view::ui::theme::{
    css_color, notoriety_color, ALARM, MANA, STAM, TEXT, TEXT_DIM, TEXT_FAINT,
};

/// The name of the panel of the target bar after the prefix.
const TARGET_PANEL: &str = "target";

/// One bar of its own.
struct OneBar {
    subject: Subject,
    /// Opened from the bars the profile kept, not by the player now.
    restored: bool,
    drawn_once: bool,
    range: RangeWatch,
    /// The name last seen, which a bar out of range keeps.
    name: String,
}

impl OneBar {
    fn new(subject: Subject, restored: bool) -> Self {
        Self {
            subject,
            restored,
            drawn_once: false,
            range: RangeWatch::default(),
            name: String::new(),
        }
    }

    fn id(&self) -> String {
        subject_bar_id(self.subject)
    }

    /// The name of its panel in the `Panel` events.
    fn panel(&self) -> String {
        match self.subject {
            Subject::Mobile(serial) => format!("{PANEL_HEALTH_PREFIX}{serial}"),
            _ => format!("{PANEL_HEALTH_PREFIX}{TARGET_PANEL}"),
        }
    }
}

#[derive(Default)]
pub(crate) struct BarsState {
    map: MapBars,
    bars: Vec<OneBar>,
    target: Option<OneBar>,
    /// The bars the profile kept are open.
    started: bool,
    /// The drag the map handed over this frame.
    pub map_drag: Option<MapDrag>,
    /// A bar pulled off a mobile follows the pointer while the button is
    /// held.
    pulling: Option<u32>,
    /// The box of a drag-select while the button is held, in points of the
    /// view.
    pub selecting: Option<Area>,
}

/// The near list: every mobile in view.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NearData {
    pub rows: Vec<NearRow>,
    pub nobody: Option<&'static str>,
}

/// One row of the near list.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NearRow {
    pub serial: u32,
    pub name: Colored,
    pub title: String,
    /// The color of the dot and of the hits, by notoriety.
    pub color: String,
    /// The hits as a share, when the shard told them.
    pub hits: Option<f32>,
    pub distance: u16,
    pub hover: TipKey,
    pub zone: DropZone,
}

/// One bar of its own.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HealthBarData {
    pub serial: u32,
    /// Hits, mana and stamina: each line its share and color.
    pub lines: Vec<BarLine>,
    /// The words of the party buttons, Heal and Cure, when they show.
    pub party: Option<[&'static str; 2]>,
    /// The lines of the menu of the bar.
    pub target: Option<&'static str>,
    pub rename: Option<&'static str>,
    pub close: &'static str,
    pub zone: DropZone,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BarLine {
    pub share: f32,
    pub color: String,
}

/// Where a click or a drop came up, in points of the view.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct AtThing {
    pub serial: u32,
    pub x: f32,
    pub y: f32,
}

/// A row of the near list: `{"click": serial}`, `{"double": serial}`,
/// `{"menu": {serial, x, y}}` (a right click opens the ring), or
/// `{"pull": {serial, x, y}}` (a row dragged out opens a bar there).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NearAction {
    Click(u32),
    Double(u32),
    Menu(AtThing),
    Pull(AtThing),
}

/// A bar of its own: `{"click": true}`, `{"double": true}`,
/// `{"heal": true}`, `{"cure": true}`, `{"target": true}` or
/// `{"rename": words}`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BarAction {
    Click(bool),
    Double(bool),
    Heal(bool),
    Cure(bool),
    Target(bool),
    Rename(String),
}

impl WebView {
    /// Sends an act of the character, when the human has control.
    fn act_in_control(&mut self, frame: &WatchFrame, act: Act) {
        if frame.human_control {
            self.hand.act(act);
        }
    }

    fn has_bar(&self, serial: u32) -> bool {
        self.panels
            .bars
            .bars
            .iter()
            .any(|bar| bar.subject == Subject::Mobile(serial))
    }

    /// Opens the bar of a mobile, when it has none.
    fn open_bar(&mut self, serial: u32) {
        places::set_open(&mut self.profile, &bar_id(serial), true);
        if !self.has_bar(serial) {
            self.panels
                .bars
                .bars
                .push(OneBar::new(Subject::Mobile(serial), false));
        }
    }

    /// Closes every bar of its own, or only those whose mobile is out of
    /// view.
    pub(crate) fn close_health_bars(&mut self, frame: &WatchFrame, inactive_only: bool) {
        let open: Vec<Subject> = self
            .panels
            .bars
            .bars
            .iter()
            .map(|bar| bar.subject)
            .collect();
        let closing = close_bars(open, frame, inactive_only, &mut self.profile);
        self.panels
            .bars
            .bars
            .retain(|bar| !closing.contains(&bar.id()));
        if !inactive_only {
            self.panels.bars.target = None;
        }
    }

    /// The bars in one frame, as the bars of the Rust window follow it: the
    /// bars the profile kept come back, the target bar follows the last
    /// target, the map opens bars, and each bar asks the shard and closes
    /// by its rules.
    pub(crate) fn follow_bars(&mut self, frame: &WatchFrame, pointer: Pointer) {
        if !self.panels.bars.started {
            self.panels.bars.started = true;
            self.panels.bars.bars = restore_bars(&self.profile)
                .into_iter()
                .map(|serial| OneBar::new(Subject::Mobile(serial), true))
                .collect();
        }
        let new_targets = self.profile.combat.new_target_system;
        match self.panels.bars.map.follow_target(frame, new_targets) {
            Some(TargetBar::Open) => {
                self.panels.bars.target = Some(OneBar::new(Subject::LastTarget, false));
            }
            Some(TargetBar::Close) => self.panels.bars.target = None,
            None => {}
        }
        self.follow_map(frame, pointer);
        let mut closing = Vec::new();
        let mut bars = std::mem::take(&mut self.panels.bars.bars);
        bars.extend(self.panels.bars.target.take());
        for bar in &mut bars {
            if !self.follow_bar(frame, bar) {
                closing.push(bar.id());
            }
        }
        bars.retain(|bar| !closing.contains(&bar.id()));
        for bar in bars {
            if bar.subject == Subject::LastTarget {
                self.panels.bars.target = Some(bar);
            } else {
                self.panels.bars.bars.push(bar);
            }
        }
        if !closing.is_empty() {
            for id in &closing {
                places::set_open(&mut self.profile, id, false);
            }
            self.keep_profile();
        }
    }

    /// One bar in one frame. False when it closes.
    fn follow_bar(&mut self, frame: &WatchFrame, bar: &mut OneBar) -> bool {
        let Some(serial) = bar.subject.serial(frame) else {
            return true;
        };
        if !bar.drawn_once {
            bar.drawn_once = true;
            match health_bars::first_frame(frame, serial, bar.restored, &self.profile.general) {
                FirstFrame::Close => return false,
                FirstFrame::Ask(ask) => self.act_in_control(frame, ask),
                FirstFrame::Stay => {}
            }
        }
        let facts = health_bars::facts(frame, serial);
        if !facts.name.is_empty() {
            bar.name.clone_from(&facts.name);
        }
        if let Some(ask) = bar.range.follow(frame, serial, &facts) {
            self.act_in_control(frame, ask);
        }
        let rule = self.profile.general.close_health_bar;
        !health_bars::closes_by_rule(rule, &facts, bar.range.hits_gone(), false)
    }

    /// Takes the drags of the map: a bar pulled off a mobile, and the box
    /// of a drag-select. A pulled bar follows the pointer until the button
    /// comes up.
    fn follow_map(&mut self, frame: &WatchFrame, pointer: Pointer) {
        let drag = self.panels.bars.map_drag.take();
        self.panels.bars.selecting = None;
        match self
            .panels
            .bars
            .map
            .follow(drag, pointer, &self.profile.general)
        {
            Some(MapAsk::Pull { serial, .. }) => {
                self.open_bar(serial);
                self.panels.bars.pulling = Some(serial);
            }
            Some(MapAsk::Selecting(area)) => self.panels.bars.selecting = Some(area),
            Some(MapAsk::Selected(area)) => self.select_bars(area, frame),
            None => {}
        }
        let Some(serial) = self.panels.bars.pulling else {
            return;
        };
        match pointer.at.filter(|_| pointer.down) {
            Some(at) => {
                let size = bar_size(&health_bars::facts(frame, serial));
                let center = self.to_panel(at);
                places::remember(
                    &mut self.profile,
                    &bar_id(serial),
                    Area::from_center_size(center, size),
                    false,
                );
            }
            None => {
                self.panels.bars.pulling = None;
                self.keep_profile();
            }
        }
    }

    /// A point of the view in the points of the panel layer.
    pub(crate) fn to_panel(&self, at: Point) -> Point {
        let scale = self.profile.video.ui_scale.max(f32::EPSILON);
        Point::new(at.x / scale, at.y / scale)
    }

    /// Opens a bar for each mobile in the box that has none, by the
    /// options, laid out from the start place of the General page.
    fn select_bars(&mut self, area: Area, frame: &WatchFrame) {
        let general = &self.profile.general;
        let chosen =
            health_bars::selected_mobiles(self.scene.mobiles_in(area), frame, general, |serial| {
                self.has_bar(serial)
            });
        let size = bar_size(&BarFacts::default());
        let open: Vec<(u32, Area)> = self
            .panels
            .bars
            .bars
            .iter()
            .filter_map(|bar| match bar.subject {
                Subject::Mobile(serial) => {
                    places::kept(&self.profile, &bar_id(serial)).map(|place| {
                        (
                            serial,
                            Area::from_min_size(Point::new(place.x, place.y), size),
                        )
                    })
                }
                _ => None,
            })
            .collect();
        let room = self.panel_room();
        let start = health_bars::select_start(room, general);
        let joined = general.drag_select_anchored;
        let placed =
            health_bars::select_layout(chosen.len(), start, size, room, joined, open, |at| {
                chosen[at]
            });
        for (serial, (place, _)) in chosen.iter().zip(placed) {
            self.open_bar(*serial);
            places::remember(
                &mut self.profile,
                &bar_id(*serial),
                Area::from_min_size(place, size),
                false,
            );
        }
        if !chosen.is_empty() {
            self.keep_profile();
        }
    }

    pub(super) fn near_spec(&self, frame: &WatchFrame) -> FrameSpec {
        let default = first_place(self.panel_room(), Spot::Near, near_list_size(frame));
        FrameSpec::fixed(NEAR_ID, WORDS_NEAR, default)
            .sized(NEAR_LEAST)
            .foldable()
    }

    pub(super) fn near_data(&self, frame: &WatchFrame) -> Framed<NearData> {
        let hint = if frame.target_cursor {
            HINT_TARGETING
        } else {
            HINT_ROW
        };
        let rows = frame
            .mobiles
            .iter()
            .map(|mobile| {
                let marked =
                    mobile.name == frame.combatant || frame.last_target == Some(mobile.serial);
                NearRow {
                    serial: mobile.serial,
                    name: Colored {
                        words: mobile.name.clone(),
                        color: css_color(if marked { ALARM } else { TEXT }),
                    },
                    title: mobile.title.clone(),
                    color: css_color(notoriety_color(mobile.notoriety)),
                    hits: mobile
                        .hits_percent
                        .map(|percent| f32::from(percent) / health_bars::PERCENT_FULL as f32),
                    distance: mobile.dist,
                    hover: TipKey::label(&mobile.name, hint),
                    zone: DropZone::Into(mobile.serial),
                }
            })
            .collect::<Vec<_>>();
        let body = NearData {
            nobody: rows.is_empty().then_some(WORDS_NOBODY),
            rows,
        };
        let mut framed = self.framed(PANEL_NEAR, &self.near_spec(frame), body);
        framed.frame.aside = Some(Colored {
            words: frame.mobiles.len().to_string(),
            color: css_color(TEXT_DIM),
        });
        framed
    }

    pub(super) fn near_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<NearAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone() else {
            return;
        };
        match action {
            NearAction::Click(serial) => self.bar_click(&frame, serial),
            NearAction::Double(serial) => self.bar_double(&frame, serial),
            NearAction::Menu(at) => {
                let name = frame
                    .mobiles
                    .iter()
                    .find(|mobile| mobile.serial == at.serial)
                    .map(|mobile| mobile.name.clone())
                    .unwrap_or_default();
                self.ring_on(&frame, at, &name);
            }
            NearAction::Pull(at) => {
                self.open_bar(at.serial);
                let size = bar_size(&health_bars::facts(&frame, at.serial));
                let center = self.to_panel(Point::new(at.x, at.y));
                places::remember(
                    &mut self.profile,
                    &bar_id(at.serial),
                    Area::from_center_size(center, size),
                    false,
                );
                self.keep_profile();
            }
        }
    }

    /// A click on a row or a bar: a target while the shard waits for one.
    fn bar_click(&mut self, frame: &WatchFrame, serial: u32) {
        if let Some(act) = health_bars::click_act(frame, serial) {
            self.act_in_control(frame, act);
        }
    }

    /// A double click on a row or a bar: an attack in war, a use in peace.
    fn bar_double(&mut self, frame: &WatchFrame, serial: u32) {
        if let Some(act) = health_bars::double_click_act(frame, serial) {
            self.act_in_control(frame, act);
        }
    }

    /// The ring of a mobile at the place of a right click, while the human
    /// has control.
    fn ring_on(&mut self, frame: &WatchFrame, at: AtThing, name: &str) {
        if frame.human_control {
            let point = Point::new(at.x, at.y);
            self.open_ring(point, at.serial, name, RingSubject::OnMap(PickKind::Mobile));
        }
    }

    /// The bar of the panel `panel`.
    fn bar_of(&self, panel: &str) -> Option<&OneBar> {
        let name = panel.strip_prefix(PANEL_HEALTH_PREFIX)?;
        let state = &self.panels.bars;
        if name == TARGET_PANEL {
            return state.target.as_ref();
        }
        let serial: u32 = name.parse().ok()?;
        state
            .bars
            .iter()
            .find(|bar| bar.subject == Subject::Mobile(serial))
    }

    fn bar_spec(&self, frame: &WatchFrame, bar: &OneBar) -> Option<(FrameSpec, BarFacts, u32)> {
        let serial = bar.subject.serial(frame)?;
        let facts = health_bars::facts(frame, serial);
        let default = first_place(self.panel_room(), Spot::MiddleTop(0), bar_size(&facts));
        let spec = FrameSpec::fixed(&bar.id(), &bar.name, default).closable();
        Some((spec, facts, serial))
    }

    pub(super) fn health_bar_spec(&self, frame: &WatchFrame, panel: &str) -> Option<FrameSpec> {
        let bar = self.bar_of(panel)?;
        self.bar_spec(frame, bar).map(|(spec, ..)| spec)
    }

    pub(super) fn health_bars_data(&self, frame: &WatchFrame) -> Vec<Framed<HealthBarData>> {
        let state = &self.panels.bars;
        state
            .bars
            .iter()
            .chain(state.target.as_ref())
            .filter_map(|bar| {
                let (spec, facts, serial) = self.bar_spec(frame, bar)?;
                Some(self.bar_data(frame, bar, &spec, &facts, serial))
            })
            .collect()
    }

    fn bar_data(
        &self,
        frame: &WatchFrame,
        bar: &OneBar,
        spec: &FrameSpec,
        facts: &BarFacts,
        serial: u32,
    ) -> Framed<HealthBarData> {
        let name_color = match facts.notoriety {
            Some(notoriety) if facts.in_range && !facts.dead => notoriety_color(notoriety),
            _ => TEXT_FAINT,
        };
        let lines = [
            (facts.hits, hits_color(facts)),
            (facts.mana, MANA),
            (facts.stam, STAM),
        ]
        .into_iter()
        .take(line_count(facts))
        .map(|(value, color)| BarLine {
            share: health_bars::share(value.filter(|_| facts.in_range)).unwrap_or_default(),
            color: css_color(color),
        })
        .collect();
        let body = HealthBarData {
            serial,
            lines,
            party: party_buttons(facts).then_some([WORDS_HEAL, WORDS_CURE]),
            target: frame.target_cursor.then_some(WORDS_TARGET),
            rename: health_bars::name_editable(frame, facts).then_some(WORDS_RENAME),
            close: WORDS_CLOSE_BAR,
            zone: DropZone::Into(serial),
        };
        let mut framed = self.framed(&bar.panel(), spec, body);
        framed.frame.title_color = Some(css_color(name_color));
        if facts.marked && facts.in_range {
            framed.frame.edge = Some(css_color(ALARM));
        }
        framed
    }

    pub(super) fn health_bar_action(&mut self, panel: &str, action: Value) {
        let Ok(action) = serde_json::from_value::<BarAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Some(serial) = self
            .bar_of(panel)
            .and_then(|bar| bar.subject.serial(&frame))
        else {
            return;
        };
        match action {
            BarAction::Click(_) => self.bar_click(&frame, serial),
            BarAction::Double(_) => self.bar_double(&frame, serial),
            BarAction::Heal(_) => {
                self.act_in_control(&frame, health_bars::cast_on(SPELL_GREATER_HEAL, serial));
            }
            BarAction::Cure(_) => {
                self.act_in_control(&frame, health_bars::cast_on(SPELL_CURE, serial));
            }
            BarAction::Target(_) if frame.target_cursor => {
                self.act_in_control(&frame, Act::Target(serial));
            }
            BarAction::Rename(name) => {
                let facts = health_bars::facts(&frame, serial);
                if health_bars::name_editable(&frame, &facts) {
                    if let Some(renamed) = health_bars::rename(serial, &name) {
                        self.act_in_control(&frame, renamed);
                    }
                }
            }
            BarAction::Target(_) => {}
        }
    }

    /// Closes the bar of the panel `panel`.
    pub(super) fn close_health_bar(&mut self, panel: &str) {
        let Some(id) = self.bar_of(panel).map(OneBar::id) else {
            return;
        };
        let state = &mut self.panels.bars;
        state.bars.retain(|bar| bar.id() != id);
        if state.target.as_ref().is_some_and(|bar| bar.id() == id) {
            state.target = None;
        }
        places::set_open(&mut self.profile, &id, false);
        self.keep_profile();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, VIEW};
    use serde_json::json;

    const ORC: u32 = 0x0000_0B01;

    /// A view of Mara in control, with an orc near.
    fn view_with_orc(war: bool) -> WebView {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["mobiles"] = json!([{
            "serial": ORC, "name": "an orc", "notoriety": 6, "hits": 5, "hits_max": 10,
            "location": { "x": 1001, "y": 1000, "z": 0 }, "body": 17
        }]);
        watch["self_state"]["war"] = json!(war);
        view.frame(&watch.to_string(), 0.0);
        view.tick_native(0.0, VIEW, None);
        view.take_out_native();
        view
    }

    #[test]
    fn a_double_click_on_a_row_attacks_in_war_as_the_near_list_of_the_window() {
        let mut view = view_with_orc(true);
        let frame = view.frame_ref().unwrap().clone();
        let near = view.panel_data(0.0).near.unwrap();
        assert_eq!(near.body.rows[0].name.words, "an orc");
        assert_eq!(near.frame.aside.unwrap().words, "1");
        let out = press(&mut view, PANEL_NEAR, json!({ "double": ORC }));
        let expected = health_bars::double_click_act(&frame, ORC).unwrap();
        assert_eq!(out_acts(&out), vec![expected.for_page()]);
    }

    #[test]
    fn a_row_pulled_out_opens_a_bar_that_asks_the_status_and_closes() {
        let mut view = view_with_orc(false);
        press(
            &mut view,
            PANEL_NEAR,
            json!({ "pull": { "serial": ORC, "x": 400, "y": 300 } }),
        );
        let frame = view.frame_ref().unwrap().clone();
        view.follow_bars(&frame, Pointer::default());
        let asked = out_acts(&view.take_out_native());
        let status = Act::MobileStatus {
            serial: ORC,
            close: false,
        };
        assert_eq!(asked, vec![status.for_page()]);
        let bars = view.panel_data(0.0).bars;
        assert_eq!(bars.len(), 1);
        let panel = bars[0].frame.panel.clone();
        assert_eq!(bars[0].body.lines[0].share, 0.5);
        press(&mut view, &panel, json!({ "close": true }));
        assert!(view.panel_data(0.0).bars.is_empty());
    }

    #[test]
    fn a_right_click_on_a_row_opens_the_ring_of_the_mobile() {
        let mut view = view_with_orc(false);
        let out = press(
            &mut view,
            PANEL_NEAR,
            json!({ "menu": { "serial": ORC, "x": 50, "y": 60 } }),
        );
        assert_eq!(out_acts(&out), vec![Act::Menu(ORC).for_page()]);
        assert_eq!(view.panel_data(0.0).ring.unwrap().name, "an orc");
    }

    /// A view whose orc, a pet of Mara, has a bar of its own, with the
    /// watch it came from.
    fn view_with_orc_bar(watch: impl Fn(&mut Value)) -> WebView {
        let mut view = view_with_orc(false);
        press(
            &mut view,
            PANEL_NEAR,
            json!({ "pull": { "serial": ORC, "x": 400, "y": 300 } }),
        );
        let mut changed: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        changed["mobiles"] = json!([{
            "serial": ORC, "name": "an orc", "notoriety": 6, "hits": 5, "hits_max": 10,
            "follower": true, "location": { "x": 1001, "y": 1000, "z": 0 }, "body": 17
        }]);
        watch(&mut changed);
        view.frame(&changed.to_string(), 0.1);
        view.take_out_native();
        view
    }

    const BAR: &str = "health:2817";

    #[test]
    fn each_action_of_a_bar_makes_the_act_of_the_window() {
        let mut view = view_with_orc_bar(|_| {});
        let frame = view.frame_ref().unwrap().clone();
        let cases = [
            (
                json!({ "double": true }),
                health_bars::double_click_act(&frame, ORC),
            ),
            (
                json!({ "heal": true }),
                Some(health_bars::cast_on(SPELL_GREATER_HEAL, ORC)),
            ),
            (
                json!({ "cure": true }),
                Some(health_bars::cast_on(SPELL_CURE, ORC)),
            ),
            (
                json!({ "rename": "Grub" }),
                health_bars::rename(ORC, "Grub"),
            ),
            (
                json!({ "click": true }),
                health_bars::click_act(&frame, ORC),
            ),
            (json!({ "target": true }), None),
        ];
        for (action, act) in cases {
            let out = press(&mut view, BAR, action.clone());
            let expected: Vec<_> = act.into_iter().map(|act| act.for_page()).collect();
            assert_eq!(out_acts(&out), expected, "{action}");
        }
        let mut aiming = view_with_orc_bar(|watch| watch["pending_target"] = json!(true));
        let frame = aiming.frame_ref().unwrap().clone();
        for action in [json!({ "click": true }), json!({ "target": true })] {
            let out = press(&mut aiming, BAR, action);
            let expected = health_bars::click_act(&frame, ORC).unwrap();
            assert_eq!(out_acts(&out), vec![expected.for_page()]);
            assert_eq!(expected, Act::Target(ORC));
        }
    }

    #[test]
    fn no_action_of_a_bar_acts_without_control() {
        let mut view = view_with_orc_bar(|watch| {
            watch["human_control"] = json!(false);
            watch["pending_target"] = json!(true);
        });
        for action in [
            json!({ "click": true }),
            json!({ "double": true }),
            json!({ "heal": true }),
            json!({ "cure": true }),
            json!({ "target": true }),
            json!({ "rename": "Grub" }),
        ] {
            assert!(
                out_acts(&press(&mut view, BAR, action.clone())).is_empty(),
                "{action}"
            );
        }
    }
}
