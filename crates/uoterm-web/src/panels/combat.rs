//! The bars of the Modern style that follow the fight and the pack: the
//! buff bar, the cooldown bars, the cast indicator, the counter bar and
//! the info bar. Each shows while its page has it on, as in the Rust
//! window; what each shows and what a click on a counter does are
//! `uoterm_view::ui::{buff_bar, combat, counter_bar, info_bar}`'.

use super::{
    Colored, DropZone, FrameSpec, Framed, TipKey, PANEL_BUFFS, PANEL_CAST, PANEL_COOLDOWNS,
    PANEL_COUNTERS, PANEL_INFO_BAR,
};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::art::ArtRequest;
use uoterm_view::frame::WatchFrame;
use uoterm_view::model::buffs;
use uoterm_view::model::casting::spell_hue;
use uoterm_view::model::counters::{self, Changes};
use uoterm_view::settings::NO_HUE;
use uoterm_view::ui::buff_bar::{
    buffs_first_place, short_name, time_color, BUFFS_ID, BUFF_ICON, WORDS_BUFFS,
};
use uoterm_view::ui::combat::{
    cast_aside_words, cast_first_place, cooldowns_first_place, seconds_words, CombatWatch, CAST_ID,
    COOLDOWNS_ID, WORDS_CAST, WORDS_COOLDOWNS,
};
use uoterm_view::ui::counter_bar::{
    amount_color, cell_hint, counters_first_place, fixed_color, flashing, uses_item, COUNTERS_ID,
    COUNTER_GAP, HINT_EMPTY, HINT_FIXED, WORDS_COUNTERS, WORDS_FIXED,
};
use uoterm_view::ui::info_bar::{info_first_place, info_parts, INFO_BAR_ID, WORDS_INFO};
use uoterm_view::ui::theme::css_color;

/// The bars that follow the journal and the cursor, and the changes of the
/// counted amounts.
#[derive(Default)]
pub(crate) struct CombatState {
    watch: CombatWatch,
    changes: Changes,
}

/// The buffs on the character, the one that ends first on the left.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BuffsData {
    pub side: f32,
    pub icons: Vec<BuffIcon>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BuffIcon {
    /// The gump picture of the buff, when the client files have one.
    pub picture: Option<String>,
    /// The words that stand for a buff with no picture.
    pub short: String,
    /// The time it has left, when the Combat page shows it.
    pub time: Option<Colored>,
    pub hover: TipKey,
}

/// The cooldown bars that run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CooldownsData {
    pub bars: Vec<CooldownBar>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CooldownBar {
    pub label: String,
    pub seconds: String,
    /// The share of the time left.
    pub fill: f32,
    pub color: String,
}

/// The spell being cast.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CastData {
    pub name: Colored,
    pub aside: String,
    /// The share of the cast done.
    pub fill: f32,
}

/// The counter bar.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CountersData {
    pub side: f32,
    pub gap: f32,
    pub columns: usize,
    pub cells: Vec<CounterCell>,
    /// The switch in the title that fixes the cells.
    pub fixed: Colored,
    pub fixed_hint: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CounterCell {
    pub picture: Option<String>,
    /// How many the pack holds, in the color that tells when it runs low.
    pub amount: Option<Colored>,
    pub flashing: bool,
    pub hover: TipKey,
    /// Where an item dropped on the cell counts from then on, unless the
    /// cells are fixed.
    pub zone: Option<DropZone>,
}

/// The info bar.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InfoBarData {
    pub parts: Vec<InfoPartData>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InfoPartData {
    pub label: Colored,
    pub words: Colored,
    /// How full the colored bar under the value is, when it has one.
    pub fill: Option<f32>,
    pub bar_color: String,
}

/// A cell of the counter bar: `{"click": cell}`, `{"double": cell}`,
/// `{"menu": {cell, alt}}` (a right click), and `{"fixed": true}` for the
/// switch in the title.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CounterAction {
    Click(usize),
    Double(usize),
    Menu { cell: usize, alt: bool },
    Fixed(bool),
}

impl WebView {
    /// The bars in one frame: the journal starts cooldowns and the cursor
    /// ends a cast.
    pub(crate) fn follow_combat(&mut self, frame: &WatchFrame, time: f64) {
        self.panels.combat.watch.observe(frame, &self.profile, time);
    }

    /// The CSS color of a hue of the shard, as words show it.
    pub(crate) fn hue_css(&self, hue: u16) -> String {
        css_color(uoterm_view::art::hue_color(&self.art, hue))
    }

    /// The picture of a gump of the client files, asked for.
    pub(crate) fn gump_picture(&mut self, gump: u16, hue: u16) -> String {
        self.picture_key(&ArtRequest::Gump {
            gump,
            hue,
            partial: false,
        })
    }

    pub(super) fn buffs_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let count = buffs::in_order(&frame.buff_icons).len();
        if !self.profile.combat.improved_buff_bar || count == 0 {
            return None;
        }
        let show_time = self.profile.combat.buff_duration;
        let default = buffs_first_place(self.panel_room(), count, show_time);
        Some(FrameSpec::fixed(BUFFS_ID, WORDS_BUFFS, default))
    }

    pub(super) fn buffs_data(&mut self, frame: &WatchFrame) -> Option<Framed<BuffsData>> {
        let spec = self.buffs_spec(frame)?;
        let show_time = self.profile.combat.buff_duration;
        let icons = buffs::in_order(&frame.buff_icons)
            .into_iter()
            .map(|buff| BuffIcon {
                picture: buffs::gump_of(buff.icon).map(|gump| self.gump_picture(gump, 0)),
                short: short_name(buff),
                time: show_time.then(|| Colored {
                    words: buffs::time_words(buff),
                    color: css_color(time_color(buff)),
                }),
                hover: TipKey::label(&buff.title, &buff.text),
            })
            .collect();
        let body = BuffsData {
            side: BUFF_ICON,
            icons,
        };
        Some(self.framed(PANEL_BUFFS, &spec, body))
    }

    pub(super) fn cooldowns_spec(&self, time: f64) -> Option<FrameSpec> {
        let bars = self.panels.combat.watch.bars(time).len();
        if !self.profile.combat.cooldown_bars || bars == 0 {
            return None;
        }
        let default = cooldowns_first_place(self.panel_room(), bars);
        Some(FrameSpec::fixed(COOLDOWNS_ID, WORDS_COOLDOWNS, default))
    }

    pub(super) fn cooldowns_data(&self, time: f64) -> Option<Framed<CooldownsData>> {
        let spec = self.cooldowns_spec(time)?;
        let bars = self
            .panels
            .combat
            .watch
            .bars(time)
            .into_iter()
            .map(|bar| CooldownBar {
                label: bar.label.clone(),
                seconds: seconds_words(bar, time),
                fill: bar.share_left(time),
                color: self.hue_css(bar.hue),
            })
            .collect();
        Some(self.framed(PANEL_COOLDOWNS, &spec, CooldownsData { bars }))
    }

    pub(super) fn cast_spec(&self) -> Option<FrameSpec> {
        self.panels.combat.watch.cast()?;
        if !self.profile.combat.spell_cast_indicator {
            return None;
        }
        let default = cast_first_place(self.panel_room());
        Some(FrameSpec::fixed(CAST_ID, WORDS_CAST, default))
    }

    pub(super) fn cast_data(&self, frame: &WatchFrame, time: f64) -> Option<Framed<CastData>> {
        let spec = self.cast_spec()?;
        let cast = self.panels.combat.watch.cast()?;
        let body = CastData {
            name: Colored {
                words: cast.name.clone(),
                color: self.hue_css(spell_hue(cast.flag, &self.profile.combat)),
            },
            aside: cast_aside_words(cast, frame, &self.profile),
            fill: cast.share_done(time),
        };
        Some(self.framed(PANEL_CAST, &spec, body))
    }

    pub(super) fn counters_spec(&self) -> Option<FrameSpec> {
        let options = &self.profile.counters;
        options.enabled.then(|| {
            let default = counters_first_place(self.panel_room(), options);
            FrameSpec::fixed(COUNTERS_ID, WORDS_COUNTERS, default)
        })
    }

    pub(super) fn counters_data(
        &mut self,
        frame: &WatchFrame,
        time: f64,
    ) -> Option<Framed<CountersData>> {
        let spec = self.counters_spec()?;
        let options = self.profile.counters.clone();
        let bags = counters::backpack_containers(frame);
        let hint = cell_hint(
            frame.human_control,
            self.profile.combat.single_click_buttons,
        );
        let mut cells = Vec::new();
        for index in 0..counters::cells(&options) {
            let zone = (!options.read_only).then_some(DropZone::Counter(index));
            let Some(item) = options.items.get(index) else {
                cells.push(CounterCell {
                    picture: None,
                    amount: None,
                    flashing: false,
                    hover: TipKey::label(HINT_EMPTY, ""),
                    zone,
                });
                continue;
            };
            let amount = counters::count(&bags, item);
            let change = self.panels.combat.changes.observe(index, amount, time);
            let hue = if item.hue == NO_HUE { 0 } else { item.hue };
            let request = self.item_picture_request(item.graphic, hue);
            cells.push(CounterCell {
                picture: Some(self.picture_key(&request)),
                amount: Some(Colored {
                    words: counters::amount_words(amount, &options),
                    color: css_color(amount_color(amount, &options)),
                }),
                flashing: flashing(&options, change, time),
                hover: TipKey::label(&item.label, hint),
                zone,
            });
        }
        let body = CountersData {
            side: f32::from(options.cell_size),
            gap: COUNTER_GAP,
            columns: usize::from(options.columns.max(1)),
            cells,
            fixed: Colored {
                words: WORDS_FIXED.to_string(),
                color: css_color(fixed_color(options.read_only)),
            },
            fixed_hint: HINT_FIXED,
        };
        Some(self.framed(PANEL_COUNTERS, &spec, body))
    }

    pub(super) fn counters_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<CounterAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let options = &self.profile.counters;
        let single_click = self.profile.combat.single_click_buttons;
        match action {
            CounterAction::Click(cell) | CounterAction::Double(cell) => {
                let double = matches!(action, CounterAction::Double(_));
                if !frame.human_control || !uses_item(single_click, double) {
                    return;
                }
                let bags = counters::backpack_containers(&frame);
                let counted = options
                    .items
                    .get(cell)
                    .and_then(|item| counters::first_counted(&bags, item));
                if let Some(pack) = counted {
                    self.hand.act(uoterm_view::act::Act::Use(pack.serial));
                }
            }
            CounterAction::Menu { cell, alt: true } if !options.read_only => {
                if counters::clear_cell(&mut self.profile.counters.items, cell) {
                    self.keep_profile();
                }
            }
            CounterAction::Fixed(_) => {
                let fixed = &mut self.profile.counters.read_only;
                *fixed = !*fixed;
                self.keep_profile();
            }
            CounterAction::Menu { .. } => {}
        }
    }

    /// An item dropped on a cell of the counter bar counts from then on,
    /// unless the cells are fixed. It leaves the mouse.
    pub(crate) fn count_in_cell(&mut self, cell: usize) {
        let desk = &mut self.panels.desk.desk;
        desk.begin();
        let Some((item, _)) = desk.land(true) else {
            return;
        };
        if self.profile.counters.read_only {
            return;
        }
        counters::put_in_cell(
            &mut self.profile.counters.items,
            cell,
            counters::counted(&item),
        );
        self.keep_profile();
    }

    pub(super) fn info_bar_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        if !self.profile.info_bar.enabled {
            return None;
        }
        let parts = info_parts(frame, &self.profile);
        let measure = self.body_measure.as_ref();
        let width = |words: &str| measure.map_or(0.0, |measure| measure(words).x);
        let default = info_first_place(self.panel_room(), &parts, width);
        Some(FrameSpec::fixed(INFO_BAR_ID, WORDS_INFO, default))
    }

    pub(super) fn info_bar_data(&self, frame: &WatchFrame) -> Option<Framed<InfoBarData>> {
        let spec = self.info_bar_spec(frame)?;
        let parts = info_parts(frame, &self.profile)
            .into_iter()
            .map(|part| InfoPartData {
                label: Colored {
                    color: self.hue_css(part.label_hue),
                    words: part.label,
                },
                words: Colored {
                    color: self.hue_css(part.words_hue),
                    words: part.words,
                },
                fill: part.fill,
                bar_color: self.hue_css(part.bar_hue),
            })
            .collect();
        Some(self.framed(PANEL_INFO_BAR, &spec, InfoBarData { parts }))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, saved_profiles, view_with_all};
    use super::*;
    use crate::tests::{settled, VIEW};
    use serde_json::json;
    use uoterm_view::act::Act;
    use uoterm_view::frame::WatchPackItem;
    use uoterm_view::settings::{CounterItem, Profile};

    use crate::tests::{BACKPACK, HATCHET};
    use uoterm_protocol::types::LAYER_BACKPACK;

    const HATCHET_GRAPHIC: u16 = 0x0F43;

    /// A view whose counter bar counts the hatchet of the fixture's
    /// backpack, with "Single-click UI buttons" as given.
    fn counting_hatchet(single_click: bool, control: bool) -> crate::WebView {
        let mut view = view_with_all(
            &[
                (
                    "containers",
                    json!([{ "serial": BACKPACK, "name": "backpack", "total": 1, "contents": [
                        { "serial": HATCHET, "graphic": HATCHET_GRAPHIC, "amount": 1 }] }]),
                ),
                ("self_state", {
                    let mut me: serde_json::Value =
                        serde_json::from_str(&crate::tests::fixture_watch_with_backpack()).unwrap();
                    me["self_state"]["equipment"] =
                        json!([{ "serial": BACKPACK, "graphic": 0x0E75, "layer": LAYER_BACKPACK }]);
                    me["self_state"].take()
                }),
            ],
            control,
        );
        let mut profile = Profile::default();
        profile.counters.enabled = true;
        profile.combat.single_click_buttons = single_click;
        profile.counters.items = vec![CounterItem {
            label: "hatchet".into(),
            graphic: HATCHET_GRAPHIC,
            hue: NO_HUE,
        }];
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        view.tick_native(0.2, VIEW, None);
        view
    }

    #[test]
    fn a_counter_uses_an_item_by_the_click_option_as_the_window_does() {
        for single_click in [true, false] {
            let mut view = counting_hatchet(single_click, true);
            let (used, unused) = if single_click {
                ("click", "double")
            } else {
                ("double", "click")
            };
            assert!(out_acts(&press(&mut view, PANEL_COUNTERS, json!({ unused: 0 }))).is_empty());
            let acts = out_acts(&press(&mut view, PANEL_COUNTERS, json!({ used: 0 })));
            assert_eq!(
                acts,
                vec![Act::Use(HATCHET).for_page()],
                "single click {single_click}"
            );
        }
    }

    #[test]
    fn a_counter_uses_nothing_without_control() {
        let mut view = counting_hatchet(true, false);
        assert!(out_acts(&press(&mut view, PANEL_COUNTERS, json!({ "click": 0 }))).is_empty());
    }

    #[test]
    fn alt_right_click_empties_a_cell_unless_the_cells_are_fixed() {
        let mut view = counting_hatchet(true, true);
        press(&mut view, PANEL_COUNTERS, json!({ "fixed": true }));
        let menu = json!({ "menu": { "cell": 0, "alt": true } });
        assert!(saved_profiles(&press(&mut view, PANEL_COUNTERS, menu.clone())).is_empty());
        press(&mut view, PANEL_COUNTERS, json!({ "fixed": true }));
        let plain = json!({ "menu": { "cell": 0, "alt": false } });
        assert!(saved_profiles(&press(&mut view, PANEL_COUNTERS, plain)).is_empty());
        let saved = saved_profiles(&press(&mut view, PANEL_COUNTERS, menu));
        assert!(saved[0].counters.items.is_empty());
    }

    #[test]
    fn an_item_dropped_on_a_cell_counts_unless_the_cells_are_fixed() {
        for fixed in [true, false] {
            let mut view = counting_hatchet(true, true);
            view.profile.counters.items.clear();
            view.profile.counters.read_only = fixed;
            view.pick_up(&WatchPackItem {
                serial: HATCHET,
                graphic: HATCHET_GRAPHIC,
                name: "hatchet".into(),
                ..WatchPackItem::default()
            });
            let drop = json!({ "drop": { "x": 1.0, "y": 1.0, "zone": { "counter": 1 } } });
            press(&mut view, super::super::PANEL_DESK, drop);
            assert_eq!(
                view.profile.counters.items.len(),
                usize::from(!fixed),
                "fixed {fixed}"
            );
            assert!(!view.carries(), "the item leaves the mouse");
        }
    }

    #[test]
    fn the_bars_show_while_their_pages_have_them_on() {
        let mut view = settled();
        let data = view.panel_data(0.0);
        assert!(data.counters.is_none() && data.info_bar.is_none());
        let mut profile = Profile::default();
        profile.counters.enabled = true;
        profile.info_bar.enabled = true;
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        let data = view.panel_data(0.0);
        assert!(data.counters.is_some());
        let info = data.info_bar.unwrap();
        assert_eq!(info.body.parts.len(), profile.info_bar.items.len());
        assert!(data.buffs.is_none(), "no buff is on");
    }
}
