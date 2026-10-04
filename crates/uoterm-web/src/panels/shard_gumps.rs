//! The gumps of the shard (0xB0 and 0xDD) in their own layout: each
//! picture, line of words, button, box and field at the place its maker
//! put it, in gump pixels from the corner of the gump. Pictures come from
//! the gump art of the client; HTML words come as runs of text the page
//! draws as text, laid out in lines by the classic rules; plain words come
//! as pictures in the fonts of the client. The pages, the boxes and the
//! fields work at all times; an answer goes to the shard only while the
//! human has control, as in the Rust window. What the human did to a gump
//! is `uoterm_view::ui::shard_gump`.

use super::{TipKey, PANEL_GUMP_PREFIX};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uoterm_view::art::{hue_color, ArtRequest, TextLook, WorldArt};
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Rgba, Vector};
use uoterm_view::ui::gump_frame::{frame_part_ids, frame_parts, rest_place, FRAME_PARTS};
use uoterm_view::ui::gumps::{picture_hue, shown_hue};
use uoterm_view::ui::html::{
    html_base_look, html_font_size, html_lines, parse_html, run_advances, HtmlBox, HTML_BACKGROUND,
    HTML_BAR_ROOM, HTML_FONT, HTML_LINE_HEIGHT,
};
use uoterm_view::ui::shard_gump::{
    gump_first_place, tile_art_place, veiled, PieceArt, ShardGumpState, UNDER_VEIL, VEIL_ALPHA,
};
use uoterm_view::ui::theme::{css_color, SIZE_BODY};
use uoterm_world::{GumpLayout, GumpPiece, GumpPieceKind};

const OPAQUE: f32 = 1.0;
/// The hue of a picture shown as it is.
const NO_HUE: u16 = 0;

/// The lines of the HTML words of a piece, with the words and the width
/// they were laid for, and the color behind them.
struct KeptLines {
    text: String,
    width: u32,
    color: Rgba,
    lines: Vec<HtmlLineData>,
    background: Option<String>,
}

/// One box of HTML words of a gump: its place in the layout and on the
/// gump, its words and how they sit, how opaque it shows and its tip.
struct HtmlPiece<'a> {
    index: usize,
    at: Point,
    size: Vector,
    text: &'a str,
    look: HtmlBox,
    alpha: f32,
    tip: Option<TipKey>,
}

/// One gump of the shard the view keeps: where the player moved it, the
/// size it was last drawn at, and the lines of its HTML words by the place
/// of each piece, kept while their words and their width stay.
struct OpenGump {
    state: ShardGumpState,
    moved: Option<Point>,
    size: Vector,
    lines: HashMap<usize, KeptLines>,
}

#[derive(Default)]
pub(crate) struct ShardGumpsState {
    open: Vec<OpenGump>,
}

/// A gump of the shard: where it stands in the panel layer, its size, and
/// its pieces in the order they draw, in gump pixels.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShardGumpData {
    pub panel: String,
    pub at: Point,
    pub size: Vector,
    /// The player may move it by dragging it.
    pub movable: bool,
    /// A right click answers it with no button.
    pub closable: bool,
    pub live: bool,
    pub pieces: Vec<GumpPieceData>,
}

/// A picture laid at a place of the gump, stretched or laid side by side.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GumpPicture {
    pub at: Point,
    pub size: Vector,
    pub picture: String,
    pub tiled: bool,
}

/// One run of HTML words in one look.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HtmlSpan {
    pub words: String,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// The size of the text font, in points.
    pub size: f32,
}

/// One line of HTML words, from its left in the box.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HtmlLineData {
    pub left: f32,
    pub spans: Vec<HtmlSpan>,
}

/// One piece of a gump. `alpha` is less than one under a veil; `tip` is
/// the tip of the piece, when it has one.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GumpPieceData {
    /// Pictures: a frame of nine, one picture, one laid side by side, an
    /// item, or words in a font of the client.
    Pictures {
        pictures: Vec<GumpPicture>,
        alpha: f32,
        tip: Option<TipKey>,
    },
    Html {
        at: Point,
        size: Vector,
        /// The paper under the words.
        paper: Vec<GumpPicture>,
        /// Where the words start in the box, and the width they wrap to.
        pad: f32,
        width: f32,
        /// The box scrolls its words.
        scroll: bool,
        /// The color a `<body bgcolor>` asks for behind the words.
        background: Option<String>,
        line_height: f32,
        lines: Vec<HtmlLineData>,
        alpha: f32,
        tip: Option<TipKey>,
    },
    Button {
        /// The place of the piece in the layout, for its action.
        piece: usize,
        at: Point,
        size: Vector,
        normal: String,
        pressed: Option<String>,
        item: Option<GumpPicture>,
        alpha: f32,
        tip: Option<TipKey>,
    },
    Choice {
        piece: usize,
        at: Point,
        size: Vector,
        picture: String,
        alpha: f32,
        tip: Option<TipKey>,
    },
    Entry {
        id: u16,
        at: Point,
        size: Vector,
        words: String,
        color: String,
        most: Option<usize>,
        /// The field takes the keys when the gump opens.
        focus: bool,
        alpha: f32,
    },
    /// Half-dark glass over a part of the gump, in its color.
    Veil {
        at: Point,
        size: Vector,
        color: String,
    },
}

/// The words of a field of the gump.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct FieldWords {
    id: u16,
    words: String,
}

/// `{"button": piece}`, `{"tick": piece}`, `{"field": {id, words}}`,
/// `{"place": {x, y}}` after a drag, `{"close": true}` on a right click.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GumpAction {
    Button(usize),
    Tick(usize),
    Field(FieldWords),
    Place(Point),
    Close(bool),
}

fn place_of(piece: &GumpPiece) -> Point {
    Point::new(piece.x as f32, piece.y as f32)
}

fn size_of(w: i32, h: i32) -> Vector {
    Vector::new(w as f32, h as f32)
}

/// The panel name of a gump.
fn gump_panel(gump: u32) -> String {
    format!("{PANEL_GUMP_PREFIX}{gump}")
}

/// The tip of a piece: the words of its `tooltip`, or the words the shard
/// has for the thing of its property.
fn piece_tip(piece: &GumpPiece) -> Option<TipKey> {
    match (&piece.tooltip, piece.property) {
        (_, Some(serial)) => Some(TipKey::thing(serial, "", "")),
        (Some(words), None) => Some(TipKey::label(words, "")),
        (None, None) => None,
    }
}

impl WebView {
    /// The gumps in one frame: a layout the shard sent opens its gump, and
    /// a gump the shard took away goes.
    pub(crate) fn follow_gumps(&mut self, frame: &WatchFrame) {
        let open = &mut self.panels.gumps.open;
        open.retain(|gump| gump.state.layout(frame).is_some());
        for layout in &frame.gump_layouts {
            if !open.iter().any(|gump| gump.state.gump == layout.gump) {
                open.push(OpenGump {
                    state: ShardGumpState::new(layout.gump),
                    moved: None,
                    size: Vector::ZERO,
                    lines: HashMap::new(),
                });
            }
        }
    }

    /// The art of a gump picture in a hue: its key, and its size once its
    /// pixels came.
    fn gump_art(&mut self, gump: u16, hue: u16) -> (String, Option<Vector>) {
        let request = ArtRequest::Gump {
            gump,
            hue,
            partial: false,
        };
        let size = self
            .art
            .sprite(&request)
            .ready()
            .map(|sprite| Vector::new(sprite.width, sprite.height));
        (request.key().to_string(), size)
    }

    /// The art of an item picture in a hue, as a gump shows it.
    fn gump_item_art(&mut self, graphic: u16, hue: u16) -> (String, Option<Vector>) {
        let request = self.item_picture_request(graphic, hue);
        let size = self
            .art
            .sprite(&request)
            .ready()
            .map(|sprite| Vector::new(sprite.width, sprite.height));
        (request.key().to_string(), size)
    }

    fn piece_art_size(&mut self, art: PieceArt) -> Option<Vector> {
        match art {
            PieceArt::Gump(gump) => self.gump_art(gump, NO_HUE).1,
            PieceArt::Item(graphic) => self.gump_item_art(graphic, NO_HUE).1,
        }
    }

    /// The nine pictures of a frame of gump `first` over an area, as far as
    /// their pixels came.
    fn frame_pictures(&mut self, first: u16, area: Area) -> Vec<GumpPicture> {
        let mut sizes = [None; FRAME_PARTS];
        let mut keys = Vec::with_capacity(FRAME_PARTS);
        for (size, part) in sizes.iter_mut().zip(frame_part_ids(first)) {
            let (key, known) = self.gump_art(part, NO_HUE);
            *size = known;
            keys.push((part, key));
        }
        frame_parts(first, &sizes, area)
            .into_iter()
            .filter_map(|part| {
                let (_, key) = keys.iter().find(|(gump, _)| *gump == part.gump)?;
                Some(GumpPicture {
                    at: part.area.min,
                    size: part.area.size(),
                    picture: key.clone(),
                    tiled: part.tiled,
                })
            })
            .collect()
    }

    /// The lines of HTML words wrapped to `width`, as the classic client
    /// lays them, measured in the text font of the page.
    fn html_lines_data(&self, text: &str, width: u32, color: Rgba) -> KeptLines {
        let read = parse_html(text, html_base_look(color.to_array()), &|_| true);
        let measure = self.body_measure.as_ref();
        let steps = run_advances(&read.chars, |words, look| {
            measure.map_or(0.0, |measure| {
                measure(words).x * html_font_size(look.font) / SIZE_BODY
            })
        });
        let lines = html_lines(&read.chars, width, |at| steps[at])
            .iter()
            .map(|line| HtmlLineData {
                left: line.left(width) as f32,
                spans: line
                    .runs(&read.chars)
                    .map(|run| {
                        let look = run[0].look;
                        let [r, g, b, a] = look.color;
                        HtmlSpan {
                            words: run.iter().map(|c| c.ch).collect(),
                            color: css_color(Rgba::from_rgba_premultiplied(r, g, b, a)),
                            bold: look.bold,
                            italic: look.italic,
                            underline: look.underline,
                            size: html_font_size(look.font),
                        }
                    })
                    .collect(),
            })
            .collect();
        KeptLines {
            text: text.to_string(),
            width,
            color,
            lines,
            background: read
                .background
                .map(|[r, g, b, a]| css_color(Rgba::from_rgba_premultiplied(r, g, b, a))),
        }
    }

    pub(super) fn gumps_data(&mut self, frame: &WatchFrame) -> Vec<ShardGumpData> {
        let mut open = std::mem::take(&mut self.panels.gumps.open);
        let data = open
            .iter_mut()
            .filter_map(|gump| {
                let layout = gump.state.layout(frame)?;
                Some(self.gump_data(frame, layout, gump))
            })
            .collect();
        self.panels.gumps.open = open;
        data
    }

    fn gump_data(
        &mut self,
        frame: &WatchFrame,
        layout: &GumpLayout,
        gump: &mut OpenGump,
    ) -> ShardGumpData {
        let live = frame.human_control;
        let shown = gump.state.shown(layout);
        let faded = veiled(&shown, |art| self.piece_art_size(art));
        let mut pieces = Vec::with_capacity(shown.len());
        for (at, piece) in shown.iter().enumerate() {
            let alpha = if faded[at] { UNDER_VEIL } else { OPAQUE };
            let index = place_in(layout, piece);
            pieces.push(self.piece_data(layout, piece, index, alpha, gump));
        }
        let size = pieces.iter().fold(Vector::ZERO, |size, piece| {
            let far = piece_far(piece);
            Vector::new(size.x.max(far.x), size.y.max(far.y))
        });
        gump.size = size;
        ShardGumpData {
            panel: gump_panel(layout.gump),
            at: gump.moved.unwrap_or_else(|| gump_first_place(layout)),
            size,
            movable: !layout.no_move,
            closable: live && !layout.no_close,
            live,
            pieces,
        }
    }

    fn piece_data(
        &mut self,
        layout: &GumpLayout,
        piece: &GumpPiece,
        index: usize,
        alpha: f32,
        gump: &mut OpenGump,
    ) -> GumpPieceData {
        let at = place_of(piece);
        let tip = piece_tip(piece);
        let pictures = |pictures| GumpPieceData::Pictures {
            pictures,
            alpha,
            tip: piece_tip(piece),
        };
        match &piece.what {
            GumpPieceKind::Background { w, h, gump: first } => {
                let area = Area::from_min_size(at, size_of(*w, *h));
                pictures(self.frame_pictures(*first, area))
            }
            GumpPieceKind::Image { gump: picture, hue } => {
                let (key, known) = self.gump_art(*picture, picture_hue(*hue));
                pictures(known.map_or_else(Vec::new, |size| {
                    vec![GumpPicture {
                        at,
                        size,
                        picture: key,
                        tiled: false,
                    }]
                }))
            }
            GumpPieceKind::Tiled {
                w,
                h,
                gump: picture,
            } => {
                let (key, _) = self.gump_art(*picture, NO_HUE);
                pictures(vec![GumpPicture {
                    at,
                    size: size_of(*w, *h),
                    picture: key,
                    tiled: true,
                }])
            }
            GumpPieceKind::Item { graphic, hue } => {
                let (key, known) = self.gump_item_art(*graphic, *hue);
                pictures(known.map_or_else(Vec::new, |size| {
                    vec![GumpPicture {
                        at,
                        size,
                        picture: key,
                        tiled: false,
                    }]
                }))
            }
            GumpPieceKind::Words {
                w,
                h,
                color,
                html: true,
                text,
                background,
                scroll,
                ..
            } => {
                let html = HtmlPiece {
                    index,
                    at,
                    size: size_of(*w, *h),
                    text,
                    look: HtmlBox {
                        background: *background,
                        scroll: *scroll,
                        color: *color,
                    },
                    alpha,
                    tip,
                };
                self.html_data(html, gump)
            }
            GumpPieceKind::Words { w, hue, text, .. } => {
                let look = TextLook::unicode(HTML_FONT, shown_hue(*hue)).bordered();
                let look = if *w > 0 {
                    look.cropped(*w as u32)
                } else {
                    look
                };
                let request = ArtRequest::Text {
                    text: text.clone(),
                    look,
                };
                let known = self.art.sprite(&request).ready();
                pictures(known.map_or_else(Vec::new, |sprite| {
                    vec![GumpPicture {
                        at,
                        size: Vector::new(sprite.width, sprite.height),
                        picture: request.key().to_string(),
                        tiled: false,
                    }]
                }))
            }
            GumpPieceKind::Button {
                normal,
                pressed,
                art,
                ..
            } => {
                let (normal_key, normal_size) = self.gump_art(*normal, NO_HUE);
                let pressed = (*pressed != 0).then(|| self.gump_art(*pressed, NO_HUE).0);
                let (size, item) = match art {
                    Some(tile) => {
                        let (key, known) = self.gump_item_art(tile.graphic, tile.hue);
                        let item = known.map(|item| {
                            let (left, top) = tile_art_place(piece.x, piece.y, tile, item);
                            GumpPicture {
                                at: Point::new(left as f32, top as f32),
                                size: item,
                                picture: key,
                                tiled: false,
                            }
                        });
                        (size_of(tile.w, tile.h), item)
                    }
                    None => (normal_size.unwrap_or(Vector::ZERO), None),
                };
                GumpPieceData::Button {
                    piece: index,
                    at,
                    size,
                    normal: normal_key,
                    pressed,
                    item,
                    alpha,
                    tip,
                }
            }
            GumpPieceKind::Choice {
                off, on, switch, ..
            } => {
                let ticked = gump.state.is_ticked(layout, *switch);
                let (key, known) = self.gump_art(if ticked { *on } else { *off }, NO_HUE);
                GumpPieceData::Choice {
                    piece: index,
                    at,
                    size: known.unwrap_or(Vector::ZERO),
                    picture: key,
                    alpha,
                    tip,
                }
            }
            GumpPieceKind::Entry {
                w,
                h,
                hue,
                id,
                text,
                limit,
            } => {
                let focus = gump.state.take_focus();
                let field = gump.state.field(*id, text, *limit);
                GumpPieceData::Entry {
                    id: *id,
                    at,
                    size: size_of(*w, *h),
                    words: field.text().to_string(),
                    color: css_color(hue_color(&self.art, shown_hue(*hue))),
                    most: field.max_chars,
                    focus,
                    alpha,
                }
            }
            GumpPieceKind::Veil { w, h } => GumpPieceData::Veil {
                at,
                size: size_of(*w, *h),
                color: css_color(Rgba::from_rgb(0, 0, 0).with_alpha(VEIL_ALPHA)),
            },
        }
    }

    /// A box of HTML words: its paper, and its lines, kept while its words,
    /// its width and its color stay.
    fn html_data(&mut self, html: HtmlPiece<'_>, gump: &mut OpenGump) -> GumpPieceData {
        let HtmlPiece {
            index,
            at,
            size,
            text,
            look,
            alpha,
            tip,
        } = html;
        let (color, room) = look.color_and_room();
        let bar_room = if look.has_bar() { HTML_BAR_ROOM } else { 0 };
        let paper = if look.background {
            let area = Area::from_min_size(at, size - Vector::new(bar_room as f32, 0.0));
            self.frame_pictures(HTML_BACKGROUND, area)
        } else {
            Vec::new()
        };
        let width = (size.x as i32 - room).max(1) as u32;
        let [r, g, b, a] = color;
        let color = Rgba::from_rgba_premultiplied(r, g, b, a);
        let fresh = gump
            .lines
            .get(&index)
            .is_some_and(|kept| kept.text == text && kept.width == width && kept.color == color);
        if !fresh {
            let kept = self.html_lines_data(text, width, color);
            gump.lines.insert(index, kept);
        }
        let kept = gump.lines.get(&index);
        GumpPieceData::Html {
            at,
            size,
            paper,
            pad: look.pad() as f32,
            width: width as f32,
            scroll: look.has_bar(),
            background: kept.and_then(|kept| kept.background.clone()),
            line_height: HTML_LINE_HEIGHT as f32,
            lines: kept.map(|kept| kept.lines.clone()).unwrap_or_default(),
            alpha,
            tip,
        }
    }

    pub(super) fn gump_action(&mut self, panel: &str, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<GumpAction>(action) else {
            return;
        };
        let room = self.panel_room();
        let Some(gump) = self
            .panels
            .gumps
            .open
            .iter_mut()
            .find(|gump| gump_panel(gump.state.gump) == panel)
        else {
            return;
        };
        let Some(layout) = gump.state.layout(&frame) else {
            return;
        };
        let live = frame.human_control;
        let shown: Vec<usize> = gump
            .state
            .shown(layout)
            .iter()
            .map(|piece| place_in(layout, piece))
            .collect();
        let on_page = |index: usize| {
            shown
                .contains(&index)
                .then(|| layout.pieces.get(index))
                .flatten()
        };
        let act = match action {
            GumpAction::Button(index) => {
                let Some(piece) = on_page(index) else {
                    return;
                };
                let button = gump.state.press(piece);
                button
                    .filter(|_| live)
                    .map(|button| gump.state.answer(layout, button))
            }
            GumpAction::Tick(index) => {
                if let Some(piece) = on_page(index) {
                    gump.state.click_box(layout, piece);
                }
                None
            }
            GumpAction::Field(typed) => {
                // Only a field of the page that shows takes words.
                let entry = shown.iter().find_map(|at| match &layout.pieces[*at].what {
                    GumpPieceKind::Entry {
                        id, text, limit, ..
                    } if *id == typed.id => Some((text, *limit)),
                    _ => None,
                });
                if let Some((text, limit)) = entry {
                    gump.state
                        .field(typed.id, text, limit)
                        .set_text(&typed.words);
                }
                None
            }
            GumpAction::Place(to) if !layout.no_move => {
                gump.moved = Some(rest_place(to, gump.size, room));
                None
            }
            GumpAction::Close(true) if live && !layout.no_close => Some(gump.state.close()),
            _ => None,
        };
        if let Some(act) = act {
            self.hand.act(act);
        }
    }
}

/// The place of a piece in its layout.
fn place_in(layout: &GumpLayout, piece: &GumpPiece) -> usize {
    layout
        .pieces
        .iter()
        .position(|known| std::ptr::eq(known, piece))
        .unwrap_or_default()
}

/// The far corner of what a piece covers, in gump pixels.
fn piece_far(piece: &GumpPieceData) -> Vector {
    let far = |at: Point, size: Vector| Vector::new(at.x + size.x, at.y + size.y);
    match piece {
        GumpPieceData::Pictures { pictures, .. } => {
            pictures.iter().fold(Vector::ZERO, |most, picture| {
                let end = far(picture.at, picture.size);
                Vector::new(most.x.max(end.x), most.y.max(end.y))
            })
        }
        GumpPieceData::Html { at, size, .. }
        | GumpPieceData::Button { at, size, .. }
        | GumpPieceData::Choice { at, size, .. }
        | GumpPieceData::Entry { at, size, .. }
        | GumpPieceData::Veil { at, size, .. } => far(*at, *size),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, view_with};
    use super::*;
    use serde_json::json;
    use uoterm_view::act::Act;

    const GUMP: u32 = 77;
    const ANSWER: u32 = 5;
    const SWITCH: u32 = 3;
    const FIELD: u16 = 9;
    const NEXT_PAGE: u32 = 2;
    const HIDDEN_FIELD: u16 = 10;

    fn piece(page: u32, what: GumpPieceKind) -> GumpPiece {
        GumpPiece {
            page,
            x: 10,
            y: 20,
            what,
            tooltip: None,
            property: None,
        }
    }

    fn button(id: Option<u32>, to_page: Option<u32>) -> GumpPieceKind {
        GumpPieceKind::Button {
            normal: 247,
            pressed: 248,
            id,
            to_page,
            art: None,
        }
    }

    /// A gump with a box, a field, a button that answers, a button that
    /// turns to page two, HTML words, and a button on page two.
    fn layout() -> GumpLayout {
        GumpLayout {
            gump: GUMP,
            pieces: vec![
                piece(
                    0,
                    GumpPieceKind::Choice {
                        off: 210,
                        on: 211,
                        switch: SWITCH,
                        radio: false,
                        ticked: false,
                        group: 0,
                    },
                ),
                piece(
                    0,
                    GumpPieceKind::Entry {
                        w: 100,
                        h: 20,
                        hue: 0,
                        id: FIELD,
                        text: "old".into(),
                        limit: Some(4),
                    },
                ),
                piece(1, button(Some(ANSWER), None)),
                piece(1, button(None, Some(NEXT_PAGE))),
                piece(
                    1,
                    GumpPieceKind::Words {
                        w: 200,
                        h: 60,
                        hue: 0,
                        color: None,
                        html: true,
                        text: "&lt;script&gt;alert(1)&lt;/script&gt;<b>bold</b>".into(),
                        background: false,
                        scroll: uoterm_world::GumpScroll::None,
                    },
                ),
                piece(NEXT_PAGE, button(Some(ANSWER + 1), None)),
                piece(
                    NEXT_PAGE,
                    GumpPieceKind::Entry {
                        w: 100,
                        h: 20,
                        hue: 0,
                        id: HIDDEN_FIELD,
                        text: "page two".into(),
                        limit: None,
                    },
                ),
            ],
            ..GumpLayout::default()
        }
    }

    fn view(control: bool) -> WebView {
        let layouts = serde_json::to_value(vec![layout()]).unwrap();
        view_with("gump_layouts", layouts, control)
    }

    fn panel() -> String {
        format!("{PANEL_GUMP_PREFIX}{GUMP}")
    }

    #[test]
    fn a_gump_answers_with_its_boxes_and_fields_as_the_window_does() {
        let mut view = view(true);
        let gump = view.panel_data(0.0).gumps.remove(0);
        assert_eq!(gump.panel, panel());
        assert!(gump.closable && gump.movable);
        press(&mut view, &panel(), json!({ "tick": 0 }));
        press(
            &mut view,
            &panel(),
            json!({ "field": { "id": FIELD, "words": "typed" } }),
        );
        let out = press(&mut view, &panel(), json!({ "button": 2 }));
        let layout = layout();
        let mut same = ShardGumpState::new(GUMP);
        same.click_box(&layout, &layout.pieces[0]);
        same.field(FIELD, "old", Some(4)).set_text("typed");
        assert_eq!(same.press(&layout.pieces[2]), Some(ANSWER));
        assert_eq!(
            out_acts(&out),
            vec![same.answer(&layout, ANSWER).for_page()]
        );
        let Act::GumpButton { texts, .. } = same.answer(&layout, ANSWER) else {
            panic!("an answer");
        };
        assert_eq!(texts, vec![(FIELD, "type".into())], "the limit holds");
    }

    #[test]
    fn a_page_button_turns_the_page_and_a_right_click_closes() {
        let mut view = view(true);
        assert!(out_acts(&press(&mut view, &panel(), json!({ "button": 3 }))).is_empty());
        let out = press(&mut view, &panel(), json!({ "button": 2 }));
        assert!(out_acts(&out).is_empty(), "the button of page one is gone");
        let out = press(&mut view, &panel(), json!({ "button": 5 }));
        assert_eq!(out_acts(&out).len(), 1);
        let out = press(&mut view, &panel(), json!({ "close": true }));
        assert_eq!(out_acts(&out), vec![Act::GumpClose(GUMP).for_page()]);
    }

    #[test]
    fn no_gump_answers_without_control_but_its_boxes_still_tick() {
        let mut view = view(false);
        let gump = view.panel_data(0.0).gumps.remove(0);
        assert!(!gump.closable && !gump.live);
        press(&mut view, &panel(), json!({ "tick": 0 }));
        assert!(out_acts(&press(&mut view, &panel(), json!({ "button": 2 }))).is_empty());
        assert!(out_acts(&press(&mut view, &panel(), json!({ "close": true }))).is_empty());
        let gump = view.panel_data(0.0).gumps.remove(0);
        let ticked = gump.pieces.iter().any(|piece| {
            matches!(piece, GumpPieceData::Choice { picture, .. }
                if *picture == ArtRequest::Gump { gump: 211, hue: 0, partial: false }.key().to_string())
        });
        assert!(ticked, "the box shows ticked");
    }

    #[test]
    fn a_field_of_a_page_that_does_not_show_takes_no_words() {
        let mut view = view(true);
        let typed = json!({ "field": { "id": HIDDEN_FIELD, "words": "typed" } });
        press(&mut view, &panel(), typed);
        press(&mut view, &panel(), json!({ "button": 3 }));
        let gump = view.panel_data(0.0).gumps.remove(0);
        let words = gump.pieces.iter().find_map(|piece| match piece {
            GumpPieceData::Entry { id, words, .. } if *id == HIDDEN_FIELD => Some(words.clone()),
            _ => None,
        });
        assert_eq!(words.as_deref(), Some("page two"));
    }

    #[test]
    fn markup_in_the_words_of_the_shard_stays_words() {
        let mut view = view(true);
        let gump = view.panel_data(0.0).gumps.remove(0);
        let lines = gump
            .pieces
            .iter()
            .find_map(|piece| match piece {
                GumpPieceData::Html { lines, .. } => Some(lines.clone()),
                _ => None,
            })
            .unwrap();
        let spans = &lines[0].spans;
        assert_eq!(spans[0].words, "<script>alert(1)</script>");
        assert!(!spans[0].bold);
        assert_eq!(spans[1].words, "bold");
        assert!(spans[1].bold);
    }

    #[test]
    fn a_gump_the_shard_took_away_goes_and_a_moved_one_stays_on_the_screen() {
        let mut view = view(true);
        let drawn = view.panel_data(0.0).gumps.remove(0);
        let far = json!({ "place": { "x": 1.0e6, "y": 1.0e6 } });
        press(&mut view, &panel(), far);
        let gump = view.panel_data(0.0).gumps.remove(0);
        let room = view.panel_room();
        assert!(drawn.size.x > 0.0, "the box and the words give it a size");
        assert!(gump.at.x < room.max.x && gump.at.y < room.max.y);
        let mut watch: Value =
            serde_json::from_str(&crate::tests::fixture_watch_with_backpack()).unwrap();
        watch["human_control"] = json!(true);
        view.frame(&watch.to_string(), 0.2);
        view.tick_native(0.2, crate::tests::VIEW, None);
        assert!(view.panel_data(0.2).gumps.is_empty());
    }
}
