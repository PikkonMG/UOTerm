//! Gumps as an agent reads them: the words on them, the buttons with the
//! words beside each one, and the choices it can tick.
//!
//! A shard draws a gump as a layout of commands. Many of its words are text
//! numbers from the client files, not words, and a button has no words of
//! its own: its label is the text drawn on the same row. So the moongate gump
//! arrives as `{ radio 200 35 210 211 0 0 }{ xmfhtmlgump 225 35 150 20 1012003
//! 0 0 }`, which a player reads as a round button beside "Moonglow".

use serde::{Deserialize, Serialize};
use uoterm_protocol::OpenGump;

/// How far apart in height, in pixels, a control and a text can be and
/// still be on one row. Rows on a gump are at least 20 pixels apart; a label
/// often sits a few pixels off its button.
const ROW_SLACK: i32 = 10;
/// A `button` whose type is this sends a reply; the other type turns the
/// page.
const BUTTON_TYPE_REPLY: u32 = 1;
/// Marks the ends of the arguments of an `xmfhtmltok` or a `tooltip`.
const ARGUMENTS_MARK: char = '@';
/// A line break inside the words of a gump.
const HTML_BREAK: &str = "<br>";

/// One open gump in words.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GumpView {
    /// The gump id, to tell one gump from another.
    pub gump: u32,
    /// The words that are not the label of a button or a choice, in the
    /// order the gump draws them.
    pub texts: Vec<GumpText>,
    /// The buttons. One with an `id` answers the gump through
    /// `gump_respond`; one with a `to_page` only shows another page.
    pub buttons: Vec<GumpButton>,
    /// The round and square buttons a player ticks before a reply. Send the
    /// `switch` of each one to tick in `gump_respond` `switches`.
    pub choices: Vec<GumpChoice>,
}

/// Words on a gump.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GumpText {
    /// The page the words are on. Page 0 shows on every page.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub page: u32,
    pub words: String,
}

/// A button and the words beside it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GumpButton {
    /// The button id to send in `gump_respond`. None for a button that
    /// turns the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// The page a page button shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_page: Option<u32>,
    /// The page the button is on. Page 0 shows on every page.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub page: u32,
    pub label: String,
}

/// A round button (one of a group) or a square one (any number).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GumpChoice {
    /// The number to send in `gump_respond` `switches`.
    pub switch: u32,
    pub kind: ChoiceKind,
    /// Ticked when the gump opened.
    pub on: bool,
    /// The page the choice is on. Page 0 shows on every page.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub page: u32,
    /// The label of the page button that shows this page, such as the map
    /// name on the moongate gump.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub section: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceKind {
    Radio,
    Checkbox,
}

impl GumpView {
    /// Every word the gump shows: its texts and the labels of its buttons
    /// and choices, in that order.
    pub fn words(&self) -> Vec<&str> {
        self.texts
            .iter()
            .map(|t| t.words.as_str())
            .chain(self.buttons.iter().map(|b| b.label.as_str()))
            .chain(self.choices.iter().map(|c| c.label.as_str()))
            .filter(|words| !words.is_empty())
            .collect()
    }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A place on the gump.
#[derive(Clone, Copy)]
struct Spot {
    page: u32,
    x: i32,
    y: i32,
}

impl Spot {
    fn on_the_row_of(self, other: Spot) -> bool {
        self.page == other.page && (self.y - other.y).abs() <= ROW_SLACK
    }
}

enum Control {
    Button {
        id: Option<u32>,
        to_page: Option<u32>,
    },
    Choice {
        switch: u32,
        kind: ChoiceKind,
        on: bool,
    },
}

struct Placed<T> {
    at: Spot,
    what: T,
}

/// Reads an open gump into words. `words` gives the sentence for a text
/// number with its tab separated arguments, from the client files; a number
/// it does not know stays as `#number`.
pub fn read_gump(gump: &OpenGump, words: &dyn Fn(u32, &str) -> Option<String>) -> GumpView {
    let localized = |number: u32, arguments: &str| {
        words(number, arguments).map_or_else(|| format!("#{number}"), |text| plain(&text))
    };
    let mut page = 0;
    let mut texts: Vec<Placed<String>> = Vec::new();
    let mut controls: Vec<Placed<Control>> = Vec::new();
    let mut tips: Vec<(usize, String)> = Vec::new();
    for command in gump.layout.split('{').filter_map(|c| c.split('}').next()) {
        let (head, arguments) = match command.split_once(ARGUMENTS_MARK) {
            Some((head, rest)) => (head, rest.trim_end().trim_end_matches(ARGUMENTS_MARK)),
            None => (command, ""),
        };
        let fields: Vec<&str> = head.split_whitespace().collect();
        let Some((name, numbers)) = fields.split_first() else {
            continue;
        };
        let n = |i: usize| {
            numbers
                .get(i)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        };
        let at = Spot {
            page,
            x: n(0) as i32,
            y: n(1) as i32,
        };
        let text_line = |index: i64| {
            usize::try_from(index)
                .ok()
                .and_then(|i| gump.text.get(i))
                .map(|line| plain(line))
        };
        match name.to_ascii_lowercase().as_str() {
            "page" => page = n(0) as u32,
            "button" | "buttontileart" => {
                let reply = n(4) as u32 == BUTTON_TYPE_REPLY;
                controls.push(Placed {
                    at,
                    what: Control::Button {
                        id: reply.then_some(n(6) as u32),
                        to_page: (!reply).then_some(n(5) as u32),
                    },
                });
            }
            kind @ ("radio" | "checkbox") => controls.push(Placed {
                at,
                what: Control::Choice {
                    switch: n(5) as u32,
                    kind: if kind == "radio" {
                        ChoiceKind::Radio
                    } else {
                        ChoiceKind::Checkbox
                    },
                    on: n(4) != 0,
                },
            }),
            "text" => texts.extend(text_line(n(3)).map(|what| Placed { at, what })),
            "croppedtext" => texts.extend(text_line(n(5)).map(|what| Placed { at, what })),
            "htmlgump" => texts.extend(text_line(n(4)).map(|what| Placed { at, what })),
            "xmfhtmlgump" | "xmfhtmlgumpcolor" => texts.push(Placed {
                at,
                what: localized(n(4) as u32, ""),
            }),
            "xmfhtmltok" => texts.push(Placed {
                at,
                what: localized(n(7) as u32, arguments),
            }),
            "tooltip" if !controls.is_empty() => {
                tips.push((controls.len() - 1, localized(n(0) as u32, arguments)));
            }
            _ => {}
        }
    }
    texts.retain(|t| !t.what.is_empty());
    let mut used = vec![false; texts.len()];
    let mut labels: Vec<String> = controls
        .iter()
        .map(|control| label_for(control.at, &texts, &mut used))
        .collect();
    for (index, tip) in tips {
        if let Some(label) = labels.get_mut(index).filter(|label| label.is_empty()) {
            *label = tip;
        }
    }
    let section_of = |page: u32| {
        controls
            .iter()
            .zip(&labels)
            .find(|(c, _)| {
                c.at.page == 0
                    && matches!(c.what, Control::Button { to_page: Some(p), .. } if p == page)
            })
            .map(|(_, label)| label.clone())
            .unwrap_or_default()
    };
    let mut view = GumpView {
        gump: gump.gump_id,
        ..GumpView::default()
    };
    for (control, label) in controls.iter().zip(&labels) {
        match control.what {
            Control::Button { id, to_page } => view.buttons.push(GumpButton {
                id,
                to_page,
                page: control.at.page,
                label: label.clone(),
            }),
            Control::Choice { switch, kind, on } => view.choices.push(GumpChoice {
                switch,
                kind,
                on,
                page: control.at.page,
                section: if control.at.page == 0 {
                    String::new()
                } else {
                    section_of(control.at.page)
                },
                label: label.clone(),
            }),
        }
    }
    view.texts = texts
        .into_iter()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|(text, _)| GumpText {
            page: text.at.page,
            words: text.what,
        })
        .collect();
    view
}

/// The words on the row of a control: the nearest unused text to its right,
/// else the nearest to its left.
fn label_for(at: Spot, texts: &[Placed<String>], used: &mut [bool]) -> String {
    let on_row = |i: &usize| !used[*i] && texts[*i].at.on_the_row_of(at);
    let right = (0..texts.len())
        .filter(on_row)
        .filter(|&i| texts[i].at.x >= at.x)
        .min_by_key(|&i| texts[i].at.x - at.x);
    let left = || {
        (0..texts.len())
            .filter(on_row)
            .min_by_key(|&i| at.x - texts[i].at.x)
    };
    match right.or_else(left) {
        Some(i) => {
            used[i] = true;
            texts[i].what.clone()
        }
        None => String::new(),
    }
}

/// Words without the HTML a gump draws them with.
fn plain(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let tag_end = rest[open..]
            .find('>')
            .map_or(rest.len(), |end| open + end + 1);
        if rest[open..tag_end].eq_ignore_ascii_case(HTML_BREAK) {
            out.push(' ');
        }
        rest = &rest[tag_end..];
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::Serial;

    const OKAY: u32 = 1011036;
    const CANCEL: u32 = 1011012;
    const PICK: u32 = 1012011;
    const TRAMMEL: u32 = 1012000;
    const FELUCCA: u32 = 1012001;
    const MOONGLOW: u32 = 1012003;
    const BRITAIN: u32 = 1012004;
    const GUMP_ID: u32 = 585180759;

    fn words(number: u32, _arguments: &str) -> Option<String> {
        let text = match number {
            OKAY => "<CENTER>OKAY</CENTER>",
            CANCEL => "CANCEL",
            PICK => "Pick your destination:",
            TRAMMEL => "Trammel",
            FELUCCA => "Felucca",
            MOONGLOW => "Moonglow",
            BRITAIN => "Britain",
            _ => return None,
        };
        Some(text.to_string())
    }

    fn gump(layout: &str, text: &[&str]) -> OpenGump {
        OpenGump {
            serial: Serial(1),
            gump_id: GUMP_ID,
            x: 0,
            y: 0,
            layout: layout.into(),
            text: text.iter().map(|t| t.to_string()).collect(),
        }
    }

    /// The moongate gump as ModernUO draws it: a tab per map on page 0, and
    /// the towns of each map on its own page.
    #[test]
    fn the_moongate_gump_reads_as_towns_under_maps() {
        let layout = format!(
            "{{ page 0 }}{{ resizepic 0 0 5054 380 280 }}\
             {{ button 10 210 4005 4007 1 0 1 }}{{ xmfhtmlgump 45 210 140 25 {OKAY} 0 0 }}\
             {{ button 10 235 4005 4007 1 0 0 }}{{ xmfhtmlgump 45 235 140 25 {CANCEL} 0 0 }}\
             {{ xmfhtmlgump 5 5 200 20 {PICK} 0 0 }}\
             {{ button 10 35 2117 2118 0 1 0 }}{{ xmfhtmlgump 30 35 150 20 {TRAMMEL} 0 0 }}\
             {{ button 10 60 2117 2118 0 2 0 }}{{ xmfhtmlgump 30 60 150 20 {FELUCCA} 0 0 }}\
             {{ page 1 }}{{ button 10 35 2117 2118 0 1 0 }}{{ xmfhtmlgump 30 35 150 20 {TRAMMEL} 0 0 }}\
             {{ radio 200 35 210 211 0 0 }}{{ xmfhtmlgump 225 35 150 20 {MOONGLOW} 0 0 }}\
             {{ radio 200 60 210 211 0 1 }}{{ xmfhtmlgump 225 60 150 20 {BRITAIN} 0 0 }}\
             {{ page 2 }}{{ button 10 60 2117 2118 0 2 0 }}{{ xmfhtmlgump 30 60 150 20 {FELUCCA} 0 0 }}\
             {{ radio 200 35 210 211 0 100 }}{{ xmfhtmlgump 225 35 150 20 {MOONGLOW} 0 0 }}"
        );
        let view = read_gump(&gump(&layout, &[]), &words);
        assert_eq!(view.gump, GUMP_ID);
        assert_eq!(view.texts.len(), 1);
        assert_eq!(view.texts[0].words, "Pick your destination:");
        let okay = &view.buttons[0];
        assert_eq!((okay.id, okay.label.as_str()), (Some(1), "OKAY"));
        assert_eq!(view.buttons[1].label, "CANCEL");
        assert_eq!(view.buttons[2].to_page, Some(1));
        assert_eq!(view.buttons[2].label, "Trammel");
        let picked: Vec<(u32, &str, &str)> = view
            .choices
            .iter()
            .map(|c| (c.switch, c.section.as_str(), c.label.as_str()))
            .collect();
        assert_eq!(
            picked,
            vec![
                (0, "Trammel", "Moonglow"),
                (1, "Trammel", "Britain"),
                (100, "Felucca", "Moonglow"),
            ]
        );
        assert!(view
            .choices
            .iter()
            .all(|c| c.kind == ChoiceKind::Radio && !c.on));
        let words = view.words();
        assert_eq!(words.first(), Some(&"Pick your destination:"));
        assert!(words.contains(&"Britain"));
    }

    /// Text lines come from the gump's own list; a number the files do not
    /// know stays a number; a button with no row label takes its tooltip.
    #[test]
    fn plain_text_unknown_numbers_and_tooltips() {
        const UNKNOWN: u32 = 1099999;
        let layout = format!(
            "{{ text 20 20 0 1 }}{{ htmlgump 20 50 100 20 0 0 0 }}\
             {{ xmfhtmltok 20 80 100 20 0 0 0 {UNKNOWN} @a\tb@ }}\
             {{ checkbox 300 300 210 211 1 7 }}\
             {{ button 400 400 1 2 1 0 9 }}{{ tooltip {OKAY} }}"
        );
        let view = read_gump(
            &gump(&layout, &["<b>Hello</b><br>there", "Name  plate"]),
            &words,
        );
        let texts: Vec<&str> = view.texts.iter().map(|t| t.words.as_str()).collect();
        assert_eq!(texts, vec!["Name plate", "Hello there", "#1099999"]);
        assert_eq!(view.choices[0].kind, ChoiceKind::Checkbox);
        assert!(view.choices[0].on);
        assert_eq!(view.choices[0].label, "");
        assert_eq!(view.buttons[0].label, "OKAY");
    }
}
