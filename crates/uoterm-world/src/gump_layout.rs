//! A gump as a screen draws it: each piece with its place, its picture and
//! its page. An agent reads a gump in words, see [`crate::read_gump`]. A
//! window draws this, so a gump of a shard looks as its maker made it.

use crate::gump::{commands, plain, Command};
use serde::{Deserialize, Serialize};
use uoterm_protocol::OpenGump;

const HUE_WORD: &str = "hue=";
/// A `button` of this type turns the page; every other type answers the
/// gump, as the classic client reads it.
const BUTTON_TYPE_PAGE: i64 = 0;
/// The `htmlgump` flag that draws a paper background under the words.
const HTML_BACKGROUND_ON: i64 = 1;
/// The `htmlgump` scroll flag that draws a small flag, not a bar.
const HTML_SCROLL_FLAG: i64 = 2;

/// How the words of an HTML piece scroll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GumpScroll {
    #[default]
    None,
    /// A scroll bar with arrows at the right edge.
    Bar,
    /// A small flag that slides down the right edge.
    Flag,
}

impl GumpScroll {
    fn from_flag(flag: i64) -> Self {
        match flag {
            0 => Self::None,
            HTML_SCROLL_FLAG => Self::Flag,
            _ => Self::Bar,
        }
    }
}

/// The item picture a `buttontileart` shows in the middle of its button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GumpTileArt {
    pub graphic: u16,
    pub hue: u16,
    /// The size of the button, which the picture sits in the middle of.
    pub w: i32,
    pub h: i32,
}

/// What one piece of a gump is. The numbers of pictures are gump art ids,
/// but `Item` has the graphic of an item. A hue is the number the shard
/// sent; the window turns it into a color as the classic client does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GumpPieceKind {
    /// A frame of nine pictures, from `gump` on, stretched to the size.
    Background {
        w: i32,
        h: i32,
        gump: u16,
    },
    Image {
        gump: u16,
        hue: u16,
    },
    /// One picture laid side by side over the size.
    Tiled {
        w: i32,
        h: i32,
        gump: u16,
    },
    Item {
        graphic: u16,
        hue: u16,
    },
    /// Words. `w` and `h` are zero for a `text` line with no box. `html`
    /// words may hold tags. `color` is a 15-bit color that some shards give.
    Words {
        w: i32,
        h: i32,
        hue: u16,
        color: Option<u32>,
        html: bool,
        text: String,
        /// HTML words on a paper background.
        #[serde(default)]
        background: bool,
        #[serde(default)]
        scroll: GumpScroll,
    },
    /// `id` answers the gump. `to_page` only turns the page.
    Button {
        normal: u16,
        pressed: u16,
        id: Option<u32>,
        to_page: Option<u32>,
        /// The item picture of a `buttontileart`.
        #[serde(default)]
        art: Option<GumpTileArt>,
    },
    Choice {
        off: u16,
        on: u16,
        switch: u32,
        radio: bool,
        ticked: bool,
        /// Radio boxes of one group clear each other. A `group` or an
        /// `endgroup` command starts the next group.
        #[serde(default)]
        group: u32,
    },
    Entry {
        w: i32,
        h: i32,
        hue: u16,
        id: u16,
        text: String,
        limit: Option<u32>,
    },
    /// A part of the gump that lets the world show through.
    Veil {
        w: i32,
        h: i32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GumpPiece {
    /// Page 0 shows on every page.
    pub page: u32,
    pub x: i32,
    pub y: i32,
    #[serde(flatten)]
    pub what: GumpPieceKind,
    /// The words a `tooltip` command put on this piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// The item whose properties an `itemproperty` command shows as the
    /// tooltip of this piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GumpLayout {
    pub gump: u32,
    /// Where the shard put the gump on the screen of the game.
    pub x: i32,
    pub y: i32,
    /// The player cannot close it with a right click.
    pub no_close: bool,
    pub no_move: bool,
    pub pieces: Vec<GumpPiece>,
}

fn hue_word(command: &Command<'_>, place: usize) -> u16 {
    command
        .word(place)
        .and_then(|word| word.strip_prefix(HUE_WORD))
        .and_then(|hue| hue.parse().ok())
        .unwrap_or(0)
}

/// Reads an open gump into pieces. `words` gives the sentence for a text
/// number with its arguments, from the client files.
pub fn gump_layout(gump: &OpenGump, words: &dyn Fn(u32, &str) -> Option<String>) -> GumpLayout {
    let mut layout = GumpLayout {
        gump: gump.gump_id,
        x: gump.x,
        y: gump.y,
        ..GumpLayout::default()
    };
    let text_line = |index: i64| {
        usize::try_from(index)
            .ok()
            .and_then(|i| gump.text.get(i))
            .cloned()
            .unwrap_or_default()
    };
    let localized = |number: i64, arguments: &str| {
        let number = number as u32;
        words(number, arguments).unwrap_or_else(|| format!("#{number}"))
    };
    let mut page = 0;
    let mut group = 0;
    for command in commands(&gump.layout) {
        let n = |place: usize| command.n(place);
        let size = |place: usize| (n(place) as i32, n(place + 1) as i32);
        let html = |text: String, flags: usize, color: Option<u32>| {
            let (w, h) = size(2);
            GumpPieceKind::Words {
                w,
                h,
                hue: 0,
                color: color.filter(|c| *c != 0),
                html: true,
                text,
                background: n(flags) == HTML_BACKGROUND_ON,
                scroll: GumpScroll::from_flag(n(flags + 1)),
            }
        };
        let what = match command.name.as_str() {
            "page" => {
                page = n(0) as u32;
                continue;
            }
            "group" | "endgroup" => {
                group += 1;
                continue;
            }
            "noclose" => {
                layout.no_close = true;
                continue;
            }
            "nomove" => {
                layout.no_move = true;
                continue;
            }
            "tooltip" => {
                if let Some(last) = layout.pieces.last_mut() {
                    let text = localized(n(0), command.arguments);
                    last.tooltip = Some(match last.tooltip.take() {
                        Some(before) => format!("{before}\n{text}"),
                        None => text,
                    });
                }
                continue;
            }
            "itemproperty" => {
                if let Some(last) = layout.pieces.last_mut() {
                    last.property = Some(n(0) as u32);
                }
                continue;
            }
            "resizepic" => GumpPieceKind::Background {
                gump: n(2) as u16,
                w: n(3) as i32,
                h: n(4) as i32,
            },
            "gumppic" | "tilepicasgumppic" => GumpPieceKind::Image {
                gump: n(2) as u16,
                hue: hue_word(&command, 3),
            },
            "gumppichued" | "gumppicphued" => GumpPieceKind::Image {
                gump: n(2) as u16,
                hue: n(3) as u16,
            },
            "gumppictiled" => {
                let (w, h) = size(2);
                GumpPieceKind::Tiled {
                    w,
                    h,
                    gump: n(4) as u16,
                }
            }
            "tilepic" | "tilepichue" => GumpPieceKind::Item {
                graphic: n(2) as u16,
                hue: n(3) as u16,
            },
            "text" => GumpPieceKind::Words {
                w: 0,
                h: 0,
                hue: n(2) as u16,
                color: None,
                html: false,
                text: text_line(n(3)),
                background: false,
                scroll: GumpScroll::None,
            },
            "croppedtext" => {
                let (w, h) = size(2);
                GumpPieceKind::Words {
                    w,
                    h,
                    hue: n(4) as u16,
                    color: None,
                    html: false,
                    text: text_line(n(5)),
                    background: false,
                    scroll: GumpScroll::None,
                }
            }
            "htmlgump" => html(text_line(n(4)), 5, None),
            "xmfhtmlgump" => html(localized(n(4), ""), 5, None),
            "xmfhtmlgumpcolor" => html(localized(n(4), ""), 5, Some(n(7) as u32)),
            "xmfhtmltok" => html(localized(n(7), command.arguments), 4, Some(n(6) as u32)),
            name @ ("button" | "buttontileart") => {
                let reply = n(4) != BUTTON_TYPE_PAGE;
                GumpPieceKind::Button {
                    normal: n(2) as u16,
                    pressed: n(3) as u16,
                    id: reply.then_some(n(6) as u32),
                    to_page: (!reply).then_some(n(5) as u32),
                    art: (name == "buttontileart").then(|| GumpTileArt {
                        graphic: n(7) as u16,
                        hue: n(8) as u16,
                        w: n(9) as i32,
                        h: n(10) as i32,
                    }),
                }
            }
            name @ ("radio" | "checkbox") => GumpPieceKind::Choice {
                off: n(2) as u16,
                on: n(3) as u16,
                ticked: n(4) != 0,
                switch: n(5) as u32,
                radio: name == "radio",
                group,
            },
            name @ ("textentry" | "textentrylimited") => {
                let (w, h) = size(2);
                GumpPieceKind::Entry {
                    w,
                    h,
                    hue: n(4) as u16,
                    id: n(5) as u16,
                    text: plain(&text_line(n(6))),
                    limit: (name == "textentrylimited").then_some(n(7) as u32),
                }
            }
            "checkertrans" => {
                let (w, h) = size(2);
                GumpPieceKind::Veil { w, h }
            }
            // What the window does not draw. The classic client does
            // nothing with `nodispose` and `noresize` either.
            _ => continue,
        };
        layout.pieces.push(GumpPiece {
            page,
            x: n(0) as i32,
            y: n(1) as i32,
            what,
            tooltip: None,
            property: None,
        });
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::Serial;

    fn moongate() -> OpenGump {
        OpenGump {
            serial: Serial(1),
            gump_id: 77,
            x: 100,
            y: 120,
            layout: "{ noclose }{ page 0 }{ resizepic 0 0 9200 380 280 }\
                { gumppic 10 10 2440 hue=33 }{ text 20 20 52 0 }\
                { page 1 }{ radio 200 35 210 211 1 5 }\
                { xmfhtmlgumpcolor 225 35 150 20 1012003 0 0 32767 }\
                { button 10 240 4005 4007 1 0 1 }{ button 50 240 4005 4007 0 2 0 }\
                { textentrylimited 80 200 100 20 0 7 1 12 }{ checkertrans 0 0 380 280 }"
                .into(),
            text: vec!["Pick your destination:".into(), "<b>100</b>".into()],
        }
    }

    #[test]
    fn each_piece_keeps_its_place_its_page_and_its_pictures() {
        let words = |number: u32, _: &str| (number == 1_012_003).then(|| "Moonglow".to_string());
        let layout = gump_layout(&moongate(), &words);
        assert_eq!((layout.gump, layout.x, layout.y), (77, 100, 120));
        assert!(layout.no_close && !layout.no_move);
        let kinds: Vec<&GumpPieceKind> = layout.pieces.iter().map(|p| &p.what).collect();
        assert_eq!(
            kinds[0],
            &GumpPieceKind::Background {
                w: 380,
                h: 280,
                gump: 9200
            }
        );
        assert_eq!(
            kinds[1],
            &GumpPieceKind::Image {
                gump: 2440,
                hue: 33
            }
        );
        assert!(
            matches!(kinds[2], GumpPieceKind::Words { text, hue: 52, .. }
            if text == "Pick your destination:")
        );
        let radio = &layout.pieces[3];
        assert_eq!((radio.page, radio.x, radio.y), (1, 200, 35));
        assert!(matches!(
            radio.what,
            GumpPieceKind::Choice {
                switch: 5,
                radio: true,
                ticked: true,
                ..
            }
        ));
        assert!(
            matches!(kinds[4], GumpPieceKind::Words { text, color: Some(32767), html: true, .. }
            if text == "Moonglow")
        );
        assert!(matches!(
            kinds[5],
            GumpPieceKind::Button {
                id: Some(1),
                to_page: None,
                ..
            }
        ));
        assert!(matches!(
            kinds[6],
            GumpPieceKind::Button {
                id: None,
                to_page: Some(2),
                ..
            }
        ));
        assert!(
            matches!(kinds[7], GumpPieceKind::Entry { id: 7, text, limit: Some(12), .. }
            if text == "100")
        );
        assert_eq!(kinds[8], &GumpPieceKind::Veil { w: 380, h: 280 });
    }

    #[test]
    fn groups_tooltips_properties_and_tile_art_buttons_are_kept() {
        let gump = OpenGump {
            serial: Serial(1),
            gump_id: 9,
            x: 0,
            y: 0,
            layout: "{ radio 0 0 208 209 1 1 }{ group 1 }{ radio 0 20 208 209 0 2 }\
                { tooltip 1011036 }{ tooltip 1011037 }{ gumppichued 5 5 100 33 }\
                { itemproperty 1073741900 }{ buttontileart 10 10 1 2 2 0 7 3821 33 40 50 }\
                { htmlgump 0 0 100 50 0 1 2 }{ nodispose }{ noresize }"
                .into(),
            text: vec!["words".into()],
        };
        let words = |number: u32, _: &str| Some(number.to_string());
        let layout = gump_layout(&gump, &words);
        let groups: Vec<u32> = layout
            .pieces
            .iter()
            .filter_map(|p| match p.what {
                GumpPieceKind::Choice { group, .. } => Some(group),
                _ => None,
            })
            .collect();
        assert_eq!(groups, vec![0, 1]);
        assert_eq!(
            layout.pieces[1].tooltip.as_deref(),
            Some("1011036\n1011037")
        );
        assert_eq!(
            layout.pieces[2].what,
            GumpPieceKind::Image { gump: 100, hue: 33 }
        );
        assert_eq!(layout.pieces[2].property, Some(1_073_741_900));
        assert!(matches!(
            layout.pieces[3].what,
            GumpPieceKind::Button {
                id: Some(7),
                art: Some(GumpTileArt {
                    graphic: 3821,
                    hue: 33,
                    w: 40,
                    h: 50
                }),
                ..
            }
        ));
        assert!(matches!(
            layout.pieces[4].what,
            GumpPieceKind::Words {
                background: true,
                scroll: GumpScroll::Flag,
                html: true,
                ..
            }
        ));
        assert_eq!(layout.pieces.len(), 5);
    }

    #[test]
    fn a_piece_goes_to_json_with_its_kind_beside_its_place() {
        let layout = gump_layout(&moongate(), &|_, _| None);
        let json = serde_json::to_value(&layout.pieces[0]).unwrap();
        assert_eq!(json["kind"], "background");
        assert_eq!(
            (json["x"].as_i64(), json["gump"].as_u64()),
            (Some(0), Some(9200))
        );
    }
}
