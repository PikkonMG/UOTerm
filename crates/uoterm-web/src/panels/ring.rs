//! The ring that opens round a right click, with what the human can do
//! with the thing and the lines of the shard's own menu, and the tooltips
//! of the things on the panels. Which lines a ring has and where they
//! stand is `uoterm_view::ui::ring`; which words a tooltip shows is
//! `uoterm_view::tips`.

use crate::{TooltipData, WebView};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::Point;
use uoterm_view::ui::ring::{ring_center, ring_lines, ring_points, RingLine, Subject};

/// The ring that is open: where the player clicked, in the points of the
/// panel layer, and the thing it is for.
pub(crate) struct OpenRing {
    pub at: Point,
    pub serial: u32,
    pub name: String,
    pub subject: Subject,
}

#[derive(Default)]
pub(crate) struct RingState {
    pub open: Option<OpenRing>,
    /// The thing on a panel the mouse rests on, for its tooltip.
    pub hover: Option<TipKey>,
}

/// The ring round a right click.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RingData {
    pub center: Point,
    pub name: String,
    pub lines: Vec<RingLineData>,
}

/// One line of the ring, at its place round the middle.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RingLineData {
    pub words: String,
    pub enabled: bool,
    pub at: Point,
}

/// What the tooltip of a thing on a panel shows: the shard's words of a
/// thing by its serial, or plain words, and what a click does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TipKey {
    pub serial: Option<u32>,
    /// The words that show until the shard answers, or always for a thing
    /// with no serial.
    pub words: String,
    pub footer: String,
}

impl TipKey {
    /// The tip of a thing the shard has words for.
    pub fn thing(serial: u32, name: &str, footer: &str) -> Self {
        Self {
            serial: Some(serial),
            words: name.to_string(),
            footer: footer.to_string(),
        }
    }

    /// The tip of something the shard has no words for: a skill, a
    /// command.
    pub fn label(words: &str, footer: &str) -> Self {
        Self {
            serial: None,
            words: words.to_string(),
            footer: footer.to_string(),
        }
    }
}

/// `{"pick": line}` does the act of a line; `{"close": true}` (a click
/// away from the lines) shuts the ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RingAction {
    Pick(usize),
    Close(bool),
}

/// `{"over": TipKey}` while the mouse rests on a thing of a panel, and
/// `{"over": null}` when it leaves.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TipsAction {
    Over(Option<TipKey>),
}

impl WebView {
    /// Opens the ring round `at` of the view, and asks the shard for its
    /// own lines, as the ring of the Rust window does.
    pub(crate) fn open_ring(&mut self, at: Point, serial: u32, name: &str, subject: Subject) {
        self.panels.ring.open = Some(OpenRing {
            at: self.to_panel(at),
            serial,
            name: name.to_string(),
            subject,
        });
        self.hand.act(Act::Menu(serial));
    }

    /// True while the ring is open.
    pub(crate) fn ring_is_open(&self) -> bool {
        self.panels.ring.open.is_some()
    }

    /// The lines of the open ring.
    fn open_lines(&self, frame: &WatchFrame) -> Option<(&OpenRing, Vec<RingLine>)> {
        let ring = self.panels.ring.open.as_ref()?;
        Some((ring, ring_lines(ring.subject, ring.serial, frame)))
    }

    pub(super) fn ring_data(&mut self, frame: &WatchFrame) -> Option<RingData> {
        if !frame.human_control {
            self.panels.ring.open = None;
            return None;
        }
        let room = self.panel_room();
        let (ring, lines) = self.open_lines(frame)?;
        let center = ring_center(ring.at, room);
        let points = ring_points(center, lines.len());
        Some(RingData {
            center,
            name: ring.name.clone(),
            lines: lines
                .into_iter()
                .zip(points)
                .map(|(line, at)| RingLineData {
                    words: line.words,
                    enabled: line.enabled,
                    at,
                })
                .collect(),
        })
    }

    pub(super) fn ring_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<RingAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone() else {
            return;
        };
        match action {
            RingAction::Pick(at) => {
                let picked = self
                    .open_lines(&frame)
                    .and_then(|(_, lines)| lines.into_iter().nth(at))
                    .filter(|line| line.enabled);
                let Some(line) = picked else {
                    return;
                };
                let shard_line = matches!(line.act, Act::MenuPick { .. });
                self.hand.act(line.act);
                if !shard_line {
                    self.hand.act(Act::MenuClose);
                }
                self.panels.ring.open = None;
            }
            RingAction::Close(_) => self.close_ring(),
        }
    }

    /// Shuts the ring, and the shard's menu with it.
    pub(crate) fn close_ring(&mut self) {
        if self.panels.ring.open.take().is_some() {
            self.hand.act(Act::MenuClose);
        }
    }

    pub(super) fn tips_action(&mut self, action: Value) {
        if let Ok(TipsAction::Over(hover)) = serde_json::from_value(action) {
            self.panels.ring.hover = hover;
        }
    }

    /// The tooltip of the thing on a panel the mouse rests on, at `time`.
    /// The shard is asked for its words while the mouse rests.
    pub(crate) fn panel_tooltip(&mut self, time: f64) -> Option<TooltipData> {
        let hover = self.panels.ring.hover.clone()?;
        let lines = match hover.serial {
            Some(serial) => {
                let hand = &mut self.hand;
                self.tips
                    .rest_on(serial, time, |serial| hand.want_tip(serial));
                self.tips
                    .shown(serial, &hover.words, &[])
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            }
            None => vec![hover.words],
        };
        Some(TooltipData {
            lines,
            footer: hover.footer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::super::{PANEL_RING, PANEL_TIPS};
    use super::*;
    use crate::out::OutCall;
    use crate::tests::{settled, HATCHET};
    use serde_json::json;
    use uoterm_view::clicks::PickKind;
    use uoterm_world::tool_names::TOOL_PROPERTIES;

    const ORC: u32 = 9;

    #[test]
    fn a_line_of_the_ring_acts_and_closes_the_shards_menu_as_the_window() {
        let mut view = settled();
        let at = Point::new(300.0, 300.0);
        view.open_ring(at, ORC, "an orc", Subject::OnMap(PickKind::Mobile));
        assert_eq!(
            out_acts(&view.take_out_native()),
            vec![Act::Menu(ORC).for_page()]
        );
        let ring = view.panel_data(0.0).ring.unwrap();
        assert_eq!(ring.name, "an orc");
        assert_eq!(ring.lines[0].words, "Look");
        let out = press(&mut view, PANEL_RING, json!({ "pick": 0 }));
        assert_eq!(
            out_acts(&out),
            vec![Act::Look(ORC).for_page(), Act::MenuClose.for_page()]
        );
        assert!(view.panel_data(0.0).ring.is_none());
    }

    #[test]
    fn a_click_away_shuts_the_ring_and_the_shards_menu() {
        let mut view = settled();
        view.open_ring(Point::new(10.0, 10.0), ORC, "", Subject::Packed);
        view.take_out_native();
        let out = press(&mut view, PANEL_RING, json!({ "close": true }));
        assert_eq!(out_acts(&out), vec![Act::MenuClose.for_page()]);
    }

    #[test]
    fn a_thing_on_a_panel_asks_the_shard_for_its_tooltip() {
        let mut view = settled();
        let hover = TipKey::thing(HATCHET, "hatchet", "Click: use.");
        press(&mut view, PANEL_TIPS, json!({ "over": hover }));
        let tip = view.panel_tooltip(0.0).unwrap();
        assert_eq!(tip.lines, vec!["hatchet".to_string()]);
        view.panel_tooltip(1.0);
        let asked = view.take_out_native();
        assert!(asked
            .iter()
            .any(|call| matches!(call, OutCall::Read { tool, .. } if tool == TOOL_PROPERTIES)));
        press(&mut view, PANEL_TIPS, json!({ "over": null }));
        assert!(view.panel_tooltip(1.0).is_none());
    }
}
