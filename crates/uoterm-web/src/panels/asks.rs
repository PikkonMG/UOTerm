//! The windows where the shard asks the player something: the dialog for
//! words it waits for, the race change, the tip of the day or the notice,
//! and the dye panel with its grid of hues. What each keeps and what its
//! buttons send are `uoterm_view::ui::shard_asks` and `ui::hues`, as in
//! the Rust window. The answers go only while the human has control.

use super::{Colored, FrameSpec, Framed, PANEL_DYE, PANEL_ENTRY, PANEL_RACE, PANEL_TIP};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::art::hue_color;
use uoterm_view::frame::{WatchFrame, WatchPackItem};
use uoterm_view::guard::LocalAim;
use uoterm_view::model::asked::asked_dialog;
use uoterm_view::model::hue_grid::{
    grid_hue, GRADUATION_MAX, GRADUATION_MIN, GRID_COLUMNS, GRID_ROWS,
};
use uoterm_view::model::race_change::{paints, palette, palette_columns, style_lists};
use uoterm_view::ui::hues::{
    dye_first_place, DyePanel, Eyedropper, DYE_ID, WORDS_DYE, WORDS_EYEDROPPER,
    WORDS_OKAY as WORDS_DYE_OKAY, WORDS_SHADE,
};
use uoterm_view::ui::lists::{race_change_words, race_preview_look};
use uoterm_view::ui::shard_asks::{
    entry_first_place, entry_hint, entry_text_room, entry_title, race_first_place, tip_first_place,
    AskedField, NoticePanel, RacePanel, ENTRY_ID, HINT_COLOR, RACE_ID, TIP_ID, TIP_LEAST_SIZE,
    WORDS_CANCEL, WORDS_CHANGE, WORDS_KEEP, WORDS_NEXT, WORDS_OKAY, WORDS_PREVIOUS,
    WORDS_TAKE_CONTROL,
};
use uoterm_view::ui::theme::{css_color, TEXT, WAITING};

/// What the dialogs of the shard keep between frames.
#[derive(Default)]
pub(crate) struct AsksState {
    entry: AskedField,
    race: RacePanel,
    notice: NoticePanel,
    dye: DyePanel,
    eyedropper: Eyedropper,
}

/// The dialog for words the shard waits for.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EntryData {
    pub live: bool,
    pub description: String,
    pub hint: &'static str,
    pub words: String,
    /// The field takes the keys now, once as the dialog opens.
    pub focus: bool,
    pub okay: Option<&'static str>,
    pub cancel: Option<&'static str>,
    pub take_control: Option<&'static str>,
}

/// The race change.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RaceData {
    pub live: bool,
    pub styles: Vec<StyleList>,
    pub figure: Option<String>,
    pub paints: Vec<PaintRow>,
    pub hint: &'static str,
    /// The palette of the color being picked.
    pub palette: Option<PaletteData>,
    pub change: Option<&'static str>,
    pub keep: Option<&'static str>,
}

/// A list of styles of the hair or the beard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StyleList {
    pub label: &'static str,
    pub choices: Vec<&'static str>,
    pub chosen: usize,
}

/// The color of a part.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PaintRow {
    pub label: &'static str,
    pub color: String,
    pub picking: bool,
}

/// The hues of one color.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PaletteData {
    pub columns: usize,
    pub hues: Vec<String>,
    pub chosen: usize,
}

/// The tip of the day or the notice of the shard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TipData {
    pub words: String,
    pub previous: Option<&'static str>,
    pub next: Option<&'static str>,
}

/// The dye panel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DyeData {
    pub live: bool,
    pub grid: HueGridData,
    /// The tub in the picked hue, and the hue.
    pub tub: Option<String>,
    pub hue: String,
    pub okay: Option<&'static str>,
    pub eyedropper: Option<Colored>,
}

/// A grid of hues with the shade slider under it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HueGridData {
    pub columns: usize,
    pub cells: Vec<String>,
    pub chosen: usize,
    pub shade: i32,
    pub shade_least: i32,
    pub shade_most: i32,
    pub shade_words: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EntryAction {
    Words(String),
    Focused(bool),
    Okay(bool),
    Cancel(bool),
}

/// A pick in a list: its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct ListPick {
    list: usize,
    at: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RaceAction {
    /// A style of the list `list`.
    Style(ListPick),
    /// A click on the color of a part, by its place.
    Paint(usize),
    /// A hue of the palette that is open.
    Hue(usize),
    Change(bool),
    Keep(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TipAction {
    Previous(bool),
    Next(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DyeAction {
    Cell(usize),
    Shade(i32),
    Okay(bool),
    Eyedropper(bool),
}

/// A spec closable only while the human has control.
fn closable_while(spec: FrameSpec, live: bool) -> FrameSpec {
    if live {
        spec.closable()
    } else {
        spec
    }
}

impl WebView {
    /// The dialogs of the shard in one frame: a new question gets an empty
    /// field, a new race change starts over, and the dye takes the hue
    /// the eyedropper clicked.
    pub(crate) fn follow_asks(&mut self, frame: &WatchFrame) {
        let asks = &mut self.panels.asks;
        asks.entry.follow(asked_dialog(frame).as_ref());
        match frame.race_change {
            Some(change) => asks.race.follow(change),
            None => asks.race.stop(),
        }
        let aiming = self.hand.aiming();
        let Some(pick) = asks.dye.follow(frame.dye.as_ref(), &mut asks.eyedropper) else {
            return;
        };
        let hand = &mut self.hand;
        let picked = || hand.take_picked(LocalAim::PickThing);
        if let Some(words) = asks.eyedropper.follow(aiming, picked, pick, frame) {
            self.hand.report(words);
        }
    }

    /// The height the description of the dialog takes in the body font.
    fn description_height(&self, description: &str) -> f32 {
        let Some(measure) = self.body_measure.as_ref() else {
            return 0.0;
        };
        let size = measure(description);
        let lines = (size.x / entry_text_room()).ceil().max(1.0);
        lines * size.y
    }

    pub(super) fn entry_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let dialog = asked_dialog(frame)?;
        let height = self.description_height(&dialog.description);
        let default = entry_first_place(self.panel_room(), height);
        let spec = FrameSpec::fixed(ENTRY_ID, entry_title(&dialog), default);
        Some(closable_while(
            spec,
            frame.human_control && dialog.can_cancel,
        ))
    }

    pub(super) fn entry_data(&self, frame: &WatchFrame) -> Option<Framed<EntryData>> {
        let dialog = asked_dialog(frame)?;
        let spec = self.entry_spec(frame)?;
        let live = frame.human_control;
        let field = &self.panels.asks.entry;
        let body = EntryData {
            live,
            description: dialog.description.clone(),
            hint: entry_hint(&dialog),
            words: field.words.clone(),
            focus: live && !field.focused,
            okay: live.then_some(WORDS_OKAY),
            cancel: (live && dialog.can_cancel).then_some(WORDS_CANCEL),
            take_control: (!live).then_some(WORDS_TAKE_CONTROL),
        };
        Some(self.framed(PANEL_ENTRY, &spec, body))
    }

    pub(super) fn entry_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(dialog) = asked_dialog(&frame) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<EntryAction>(action) else {
            return;
        };
        let field = &mut self.panels.asks.entry;
        match action {
            EntryAction::Words(words) => field.typed(&words, &dialog),
            EntryAction::Focused(_) => field.focused = true,
            EntryAction::Okay(_) => self.hand.act(dialog.commands.answer_act(&field.words)),
            EntryAction::Cancel(_) if dialog.can_cancel => {
                self.hand.act(dialog.commands.cancel_act());
            }
            EntryAction::Cancel(_) => {}
        }
    }

    /// The close mark of the dialog says no.
    pub(super) fn close_entry(&mut self) {
        self.entry_action(serde_json::json!({ "cancel": true }));
    }

    pub(super) fn race_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let change = frame.race_change?;
        let title = race_change_words(change);
        let spec = FrameSpec::fixed(RACE_ID, &title, race_first_place(self.panel_room()));
        Some(closable_while(spec, frame.human_control))
    }

    pub(super) fn race_data(&mut self, frame: &WatchFrame) -> Option<Framed<RaceData>> {
        let change = frame.race_change?;
        let spec = self.race_spec(frame)?;
        let live = frame.human_control;
        let race = &self.panels.asks.race;
        let mut picks = race.picks;
        let styles = style_lists(change)
            .into_iter()
            .map(|(part, (_, label), styles)| StyleList {
                label,
                choices: styles.iter().map(|style| style.words).collect(),
                chosen: *picks.style_place(part),
            })
            .collect();
        let picking = race.picking.filter(|_| live);
        let paint_rows = paints(change)
            .into_iter()
            .map(|(paint, (_, label))| PaintRow {
                label,
                color: css_color(hue_color(&self.art, picks.hue(change, paint))),
                picking: picking == Some(paint),
            })
            .collect();
        let palette = picking.map(|paint| {
            let hues = palette(change, paint);
            PaletteData {
                columns: palette_columns(hues.len()),
                hues: hues
                    .iter()
                    .map(|hue| css_color(hue_color(&self.art, *hue)))
                    .collect(),
                chosen: *picks.hue_place(paint),
            }
        });
        let look = race_preview_look(change, &picks);
        let body = RaceData {
            live,
            styles,
            figure: Some(self.picture_key(&uoterm_view::scene::doll_figure(&look))),
            paints: paint_rows,
            hint: HINT_COLOR,
            palette,
            change: live.then_some(WORDS_CHANGE),
            keep: live.then_some(WORDS_KEEP),
        };
        Some(self.framed(PANEL_RACE, &spec, body))
    }

    pub(super) fn race_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(change) = frame.race_change else {
            return;
        };
        let Ok(action) = serde_json::from_value::<RaceAction>(action) else {
            return;
        };
        let race = &mut self.panels.asks.race;
        match action {
            RaceAction::Style(pick) => {
                let lists = style_lists(change);
                if let Some((part, _, styles)) = lists.get(pick.list) {
                    if pick.at < styles.len() {
                        race.pick_style(*part, pick.at);
                    }
                }
            }
            RaceAction::Paint(at) => {
                if let Some((paint, _)) = paints(change).get(at) {
                    race.click_paint(*paint);
                }
            }
            RaceAction::Hue(at) => {
                if let Some(paint) = race.picking {
                    if at < palette(change, paint).len() {
                        race.pick_hue(paint, at);
                    }
                }
            }
            RaceAction::Change(_) => {
                let looks = race.picks.looks(change);
                self.hand.act(Act::RaceChange(Some(looks)));
            }
            RaceAction::Keep(_) => self.hand.act(Act::RaceChange(None)),
        }
    }

    /// The close mark of the race change keeps the old looks.
    pub(super) fn close_race(&mut self) {
        self.race_action(serde_json::json!({ "keep": true }));
    }

    pub(super) fn tip_spec(&mut self, frame: &WatchFrame) -> Option<FrameSpec> {
        let shown = self.panels.asks.notice.shown(frame)?;
        let default = tip_first_place(self.panel_room());
        Some(
            FrameSpec::fixed(TIP_ID, shown.title, default)
                .sized(TIP_LEAST_SIZE)
                .closable(),
        )
    }

    pub(super) fn tip_data(&mut self, frame: &WatchFrame) -> Option<Framed<TipData>> {
        let spec = self.tip_spec(frame)?;
        let shown = self.panels.asks.notice.shown(frame)?;
        let buttons = shown.tip && frame.human_control;
        let body = TipData {
            words: shown.words.to_string(),
            previous: buttons.then_some(WORDS_PREVIOUS),
            next: buttons.then_some(WORDS_NEXT),
        };
        Some(self.framed(PANEL_TIP, &spec, body))
    }

    pub(super) fn tip_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        if frame.shard_tip.is_none() {
            return;
        }
        match serde_json::from_value::<TipAction>(action) {
            Ok(TipAction::Previous(_)) => self.hand.act(Act::Tip { next: false }),
            Ok(TipAction::Next(_)) => self.hand.act(Act::Tip { next: true }),
            Err(_) => {}
        }
    }

    /// The close mark hides the words until the shard sends others.
    pub(super) fn close_tip(&mut self) {
        if let Some(words) = self
            .frame
            .as_ref()
            .and_then(|frame| frame.shard_notice.clone())
        {
            self.panels.asks.notice.close(&words);
        }
    }

    pub(super) fn dye_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        frame.dye.as_ref()?;
        Some(FrameSpec::fixed(
            DYE_ID,
            WORDS_DYE,
            dye_first_place(self.panel_room()),
        ))
    }

    pub(super) fn dye_data(&mut self, frame: &WatchFrame) -> Option<Framed<DyeData>> {
        let dye = frame.dye.as_ref()?;
        let spec = self.dye_spec(frame)?;
        let live = frame.human_control;
        let (_, pick) = self.panels.asks.dye.tub?;
        let picking = self.panels.asks.eyedropper.picking;
        let cells = (0..GRID_ROWS * GRID_COLUMNS)
            .map(|index| css_color(hue_color(&self.art, grid_hue(pick.graduation, index))))
            .collect();
        let tub = WatchPackItem {
            graphic: dye.graphic,
            hue: pick.hue(),
            ..WatchPackItem::default()
        };
        let body = DyeData {
            live,
            grid: HueGridData {
                columns: GRID_COLUMNS,
                cells,
                chosen: pick.index,
                shade: pick.graduation,
                shade_least: GRADUATION_MIN,
                shade_most: GRADUATION_MAX,
                shade_words: WORDS_SHADE,
            },
            tub: self.item_picture(&tub),
            hue: pick.hue().to_string(),
            okay: live.then_some(WORDS_DYE_OKAY),
            eyedropper: live.then(|| Colored {
                words: WORDS_EYEDROPPER.to_string(),
                color: css_color(if picking { WAITING } else { TEXT }),
            }),
        };
        Some(self.framed(PANEL_DYE, &spec, body))
    }

    pub(super) fn dye_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<DyeAction>(action) else {
            return;
        };
        let asks = &mut self.panels.asks;
        let Some((_, pick)) = asks.dye.tub.as_mut() else {
            return;
        };
        match action {
            DyeAction::Cell(index) if index < GRID_ROWS * GRID_COLUMNS => pick.index = index,
            DyeAction::Shade(shade) => {
                pick.graduation = shade.clamp(GRADUATION_MIN, GRADUATION_MAX)
            }
            DyeAction::Okay(_) => {
                let hue = pick.hue();
                self.hand.act(Act::Dye(hue));
            }
            DyeAction::Eyedropper(_) => {
                if let Some(act) = asks.eyedropper.press(&frame) {
                    self.hand.act(act);
                }
                self.hand.aim(LocalAim::PickThing);
            }
            DyeAction::Cell(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::super::{PANEL_DYE, PANEL_ENTRY, PANEL_RACE, PANEL_TIP};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, settled};
    use serde_json::json;
    use uoterm_view::model::asked::TEXT_ENTRY;
    use uoterm_view::model::hue_grid::HuePick;
    use uoterm_view::model::race_change::RacePicks;
    use uoterm_world::{Race, RaceChange};

    const TUB: u32 = 0x4000_0100;
    const ELF_WOMAN: RaceChange = RaceChange {
        race: Race::Elf,
        female: true,
    };

    fn view_with(key: &str, value: serde_json::Value, control: bool) -> WebView {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch[key] = value;
        watch["human_control"] = json!(control);
        let mut view = settled();
        view.frame(&watch.to_string(), 0.1);
        view.tick_native(0.1, crate::tests::VIEW, None);
        view.take_out_native();
        view
    }

    fn entry(control: bool) -> WebView {
        view_with(
            "text_entry",
            json!({ "title": "How many?", "description": "Say", "can_cancel": true,
                "style": 2, "max_length": 3 }),
            control,
        )
    }

    #[test]
    fn the_entry_keeps_the_rules_of_the_shard_and_answers_as_the_window() {
        let mut view = entry(true);
        let data = view.panel_data(0.0).entry.unwrap();
        assert_eq!(data.frame.title, "How many?");
        assert!(data.body.focus, "the field takes the keys once");
        press(&mut view, PANEL_ENTRY, json!({"focused": true}));
        assert!(!view.panel_data(0.0).entry.unwrap().body.focus);
        press(&mut view, PANEL_ENTRY, json!({"words": "12a345"}));
        assert_eq!(view.panel_data(0.0).entry.unwrap().body.words, "123");
        let out = press(&mut view, PANEL_ENTRY, json!({"okay": true}));
        assert_eq!(
            out_acts(&out),
            vec![TEXT_ENTRY.answer_act("123").for_page()]
        );
        let out = press(&mut view, PANEL_ENTRY, json!({"close": true}));
        assert_eq!(out_acts(&out), vec![TEXT_ENTRY.cancel_act().for_page()]);
    }

    #[test]
    fn the_race_change_picks_and_sends_the_looks_of_the_window() {
        let mut view = view_with(
            "race_change",
            json!({ "race": "elf", "female": true }),
            true,
        );
        press(
            &mut view,
            PANEL_RACE,
            json!({"style": {"list": 0, "at": 1}}),
        );
        press(&mut view, PANEL_RACE, json!({"paint": 0}));
        assert!(view.panel_data(0.0).race.unwrap().body.palette.is_some());
        press(&mut view, PANEL_RACE, json!({"hue": 2}));
        assert!(view.panel_data(0.0).race.unwrap().body.palette.is_none());
        let mut picks = RacePicks::default();
        picks.follow(ELF_WOMAN);
        picks.hair = 1;
        picks.hues[0] = 2;
        let out = press(&mut view, PANEL_RACE, json!({"change": true}));
        let looks = Act::RaceChange(Some(picks.looks(ELF_WOMAN)));
        assert_eq!(out_acts(&out), vec![looks.for_page()]);
        let out = press(&mut view, PANEL_RACE, json!({"close": true}));
        assert_eq!(out_acts(&out), vec![Act::RaceChange(None).for_page()]);
    }

    #[test]
    fn a_tip_asks_for_another_and_closes_until_new_words() {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["shard_notice"] = json!("Hail");
        watch["shard_tip"] = json!(4);
        let mut view = settled();
        view.frame(&watch.to_string(), 0.1);
        let tip = view.panel_data(0.0).tip.unwrap();
        assert_eq!(
            (tip.frame.title.as_str(), tip.body.words.as_str()),
            ("Tip of the day", "Hail")
        );
        let out = press(&mut view, PANEL_TIP, json!({"next": true}));
        assert_eq!(out_acts(&out), vec![Act::Tip { next: true }.for_page()]);
        press(&mut view, PANEL_TIP, json!({"close": true}));
        assert!(view.panel_data(0.0).tip.is_none());
    }

    #[test]
    fn the_dye_picks_a_hue_and_sends_it_as_the_window() {
        let mut view = view_with("dye", json!({ "serial": TUB, "graphic": 0x0FAB }), true);
        press(&mut view, PANEL_DYE, json!({"shade": 3}));
        press(&mut view, PANEL_DYE, json!({"cell": 7}));
        let pick = HuePick {
            graduation: 3,
            index: 7,
        };
        let dye = view.panel_data(0.0).dye.unwrap();
        assert_eq!(dye.body.hue, pick.hue().to_string());
        assert!(!dye.frame.closable, "the shard waits for the colour");
        let out = press(&mut view, PANEL_DYE, json!({"okay": true}));
        assert_eq!(out_acts(&out), vec![Act::Dye(pick.hue()).for_page()]);
        press(&mut view, PANEL_DYE, json!({"eyedropper": true}));
        assert_eq!(view.hand.aiming(), Some(LocalAim::PickThing));
    }

    #[test]
    fn no_dialog_of_the_shard_answers_without_control() {
        let mut view = entry(false);
        assert!(view
            .panel_data(0.0)
            .entry
            .unwrap()
            .body
            .take_control
            .is_some());
        assert!(out_acts(&press(&mut view, PANEL_ENTRY, json!({"okay": true}))).is_empty());
        let mut view = view_with(
            "race_change",
            json!({ "race": "elf", "female": true }),
            false,
        );
        assert!(out_acts(&press(&mut view, PANEL_RACE, json!({"keep": true}))).is_empty());
        let mut view = view_with("dye", json!({ "serial": TUB, "graphic": 0x0FAB }), false);
        assert!(out_acts(&press(&mut view, PANEL_DYE, json!({"okay": true}))).is_empty());
    }
}
