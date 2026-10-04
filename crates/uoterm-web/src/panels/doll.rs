//! The paperdoll of the Modern style: the words the shard put at its top,
//! the figure and the health of its mobile, what he wears, and its
//! buttons. Which doll shows and what a click does are the rules of
//! `uoterm_view::ui::doll`, as in the Rust window. The clicks work only
//! while the human has control.

use super::{click_item, DropZone, FrameSpec, Framed, TipKey, PANEL_PAPERDOLL};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::frame::{WatchFrame, WatchPackItem};
use uoterm_view::model::clicks::ClickDelay;
use uoterm_view::model::dolls;
use uoterm_view::scene::doll_figure;
use uoterm_view::ui::deck::layer_words;
use uoterm_view::ui::doll::{
    doll_buttons, doll_first_place, doll_health, doll_look, doll_worn, doll_zone, worn_footer,
    DollPanel, ShownDoll, DOLL_ID, WORDS_CLOSE, WORDS_OUT_OF_SIGHT, WORDS_WEARS_NOTHING,
};

/// The paperdoll that shows, and the click that waits for its name.
#[derive(Default)]
pub(crate) struct DollState {
    pub panel: DollPanel,
    clicks: ClickDelay,
}

/// The paperdoll.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PaperdollData {
    pub live: bool,
    pub figure: Option<String>,
    pub out_of_sight: Option<&'static str>,
    /// The share of hits of the mobile, when known.
    pub health: Option<f32>,
    pub rows: Vec<DollRow>,
    pub nothing: Option<&'static str>,
    /// The character may take items off and put them on.
    pub dresses: bool,
    pub buttons: Vec<&'static str>,
    /// Close, while the human has control.
    pub close: Option<&'static str>,
    /// Where an item dropped on the doll goes, when he dresses it.
    pub zone: Option<DropZone>,
}

/// One worn item.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DollRow {
    pub serial: u32,
    pub picture: Option<String>,
    pub words: String,
    pub hover: TipKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DollAction {
    Click(u32),
    Double(u32),
    Drag(u32),
    /// A button of the doll, by its place.
    Button(usize),
}

impl WebView {
    /// The paperdoll in one frame: a new one the shard sent shows, and a
    /// click that waited long enough asks the name.
    pub(crate) fn follow_doll(&mut self, frame: &WatchFrame, time: f64) {
        let doll = &mut self.panels.doll;
        doll.panel.follow(frame);
        if let Some(act) = doll.clicks.due_look(time) {
            self.hand.act(act);
        }
    }

    fn shown_doll(&self) -> Option<ShownDoll> {
        self.panels.doll.panel.doll.clone()
    }

    pub(super) fn doll_spec(&self) -> Option<FrameSpec> {
        let doll = self.shown_doll()?;
        let default = doll_first_place(self.panel_room());
        Some(FrameSpec::fixed(DOLL_ID, &doll.text, default).closable())
    }

    pub(super) fn doll_data(&mut self, frame: &WatchFrame) -> Option<Framed<PaperdollData>> {
        let doll = self.shown_doll()?;
        let spec = self.doll_spec()?;
        let live = frame.human_control;
        let dresses = dolls::dresses(frame, doll.serial, doll.can_lift);
        let look = doll_look(frame, doll.serial).cloned();
        let mut rows = Vec::new();
        if let Some(look) = &look {
            for item in doll_worn(look) {
                let request = self.item_picture_request(item.graphic, item.hue);
                rows.push(DollRow {
                    serial: item.serial,
                    picture: Some(self.picture_key(&request)),
                    words: layer_words(item.layer),
                    hover: TipKey::thing(item.serial, "", worn_footer(live, dresses)),
                });
            }
        }
        let figure = look
            .as_ref()
            .map(|look| self.picture_key(&doll_figure(look)));
        let body = PaperdollData {
            live,
            figure,
            out_of_sight: look.is_none().then_some(WORDS_OUT_OF_SIGHT),
            health: doll_health(frame, doll.serial),
            nothing: (look.is_some() && rows.is_empty()).then_some(WORDS_WEARS_NOTHING),
            rows,
            dresses,
            buttons: if live {
                doll_buttons(doll.serial, doll.serial == frame.serial)
                    .into_iter()
                    .map(|(words, _)| words)
                    .collect()
            } else {
                Vec::new()
            },
            close: live.then_some(WORDS_CLOSE),
            zone: (dresses && live).then(|| doll_zone(frame, doll.serial).into()),
        };
        Some(self.framed(PANEL_PAPERDOLL, &spec, body))
    }

    pub(super) fn doll_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(doll) = self.shown_doll() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<DollAction>(action) else {
            return;
        };
        let worn = doll_look(&frame, doll.serial)
            .map(|look| doll_worn(look).into_iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let clicks = &mut self.panels.doll.clicks;
        let is_worn = |serial: u32| worn.iter().any(|item| item.serial == serial);
        match action {
            DollAction::Click(serial) | DollAction::Double(serial) if !is_worn(serial) => {}
            DollAction::Click(serial) => {
                click_item(clicks, &mut self.hand, &frame, serial, false);
            }
            DollAction::Double(serial) => {
                if click_item(clicks, &mut self.hand, &frame, serial, true) {
                    self.hand.act(Act::Use(serial));
                }
            }
            DollAction::Drag(serial) => {
                let lifts = dolls::dresses(&frame, doll.serial, doll.can_lift);
                if let Some(item) = worn.iter().find(|item| item.serial == serial && lifts) {
                    self.pick_up(&WatchPackItem {
                        serial: item.serial,
                        graphic: item.graphic,
                        hue: item.hue,
                        amount: 1,
                        ..WatchPackItem::default()
                    });
                }
            }
            DollAction::Button(at) => {
                let own = doll.serial == frame.serial;
                if let Some((_, act)) = doll_buttons(doll.serial, own).into_iter().nth(at) {
                    self.hand.act(act);
                }
            }
        }
    }

    /// The close mark and Close of the paperdoll, as "close all gumps".
    pub(crate) fn close_doll(&mut self) {
        self.panels.doll.panel.close();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::super::PANEL_PAPERDOLL;
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, settled};
    use serde_json::json;

    const ME: u32 = 1;
    const SWORD: u32 = 0x4000_0050;

    /// Mara wearing a sword, and the shard's paperdoll `seq` of her.
    fn watch(seq: u64, control: bool) -> serde_json::Value {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["human_control"] = json!(control);
        watch["self_state"]["equipment"] =
            json!([{ "serial": SWORD, "graphic": 0x13B9, "layer": 1 }]);
        watch["paperdoll"] = json!({ "serial": ME, "text": "Mara the Bold", "seq": seq,
            "can_lift": true });
        watch
    }

    fn view_with_doll(control: bool) -> WebView {
        let mut view = settled();
        view.frame(&watch(1, control).to_string(), 0.1);
        view.tick_native(0.1, crate::tests::VIEW, None);
        view.frame(&watch(2, control).to_string(), 0.2);
        view.tick_native(0.2, crate::tests::VIEW, None);
        view.take_out_native();
        view
    }

    #[test]
    fn a_paperdoll_the_shard_sends_shows_with_the_buttons_of_the_window() {
        let mut view = view_with_doll(true);
        let doll = view.panel_data(0.0).paperdoll.unwrap();
        assert_eq!(doll.frame.title, "Mara the Bold");
        assert_eq!(doll.body.rows[0].serial, SWORD);
        assert_eq!(doll.body.zone, Some(DropZone::Wear));
        let frame = view.frame_ref().unwrap().clone();
        let buttons = doll_buttons(ME, true);
        assert_eq!(doll.body.buttons.len(), buttons.len());
        let out = press(&mut view, PANEL_PAPERDOLL, json!({"button": 0}));
        assert_eq!(out_acts(&out), vec![buttons[0].1.clone().for_page()]);
        let out = press(&mut view, PANEL_PAPERDOLL, json!({"double": SWORD}));
        assert_eq!(out_acts(&out), vec![Act::Use(SWORD).for_page()]);
        assert!(dolls::dresses(&frame, ME, true));
        press(&mut view, PANEL_PAPERDOLL, json!({"drag": SWORD}));
        assert!(view.carries());
        press(&mut view, PANEL_PAPERDOLL, json!({"close": true}));
        assert!(view.panel_data(0.0).paperdoll.is_none());
    }

    #[test]
    fn the_paperdoll_does_nothing_without_control() {
        let mut view = view_with_doll(false);
        let doll = view.panel_data(0.0).paperdoll.unwrap();
        assert!(doll.body.buttons.is_empty() && doll.body.zone.is_none());
        for action in [
            json!({"button": 0}),
            json!({"double": SWORD}),
            json!({"drag": SWORD}),
        ] {
            assert!(out_acts(&press(&mut view, PANEL_PAPERDOLL, action)).is_empty());
        }
        assert!(!view.carries());
    }

    #[test]
    fn a_thing_not_worn_takes_no_click_and_close_shows_only_with_control() {
        let mut view = view_with_doll(true);
        let elsewhere = crate::tests::HATCHET;
        assert!(out_acts(&press(
            &mut view,
            PANEL_PAPERDOLL,
            json!({"double": elsewhere})
        ))
        .is_empty());
        press(&mut view, PANEL_PAPERDOLL, json!({"click": elsewhere}));
        view.follow_doll(&view.frame_ref().unwrap().clone(), 5.0);
        assert!(out_acts(&view.take_out_native()).is_empty());
        assert!(view.panel_data(0.0).paperdoll.unwrap().body.close.is_some());
        let mut view = view_with_doll(false);
        assert!(view.panel_data(0.0).paperdoll.unwrap().body.close.is_none());
    }
}
