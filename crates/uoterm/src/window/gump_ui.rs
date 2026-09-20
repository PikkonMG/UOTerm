//! A gump of the shard, drawn as its maker made it: each picture, line of
//! words, button and field at its own place, from the gump art of the
//! client. The operator sees it at all times. The clicks work only while
//! the human has control.

use super::boxes_ui::{on_page, Tools, FIRST_PAGE};
use super::control::Act;
use super::theme::text_font;
use crate::view::WatchFrame;
use eframe::egui::{
    self, text::LayoutJob, Color32, CornerRadius, Id, Pos2, Rect, Sense, TextFormat, Vec2,
};
use std::collections::HashMap;
use uoterm_world::{GumpLayout, GumpPiece, GumpPieceKind};

/// A frame is nine pictures in a row of ids: three across the top, three
/// across the middle, three across the bottom.
const FRAME_PARTS: u16 = 9;
const FRAME_MIDDLE: u16 = 4;
const WORDS_SIZE: f32 = 13.0;
/// The words of a gump are dark, as ink on paper, when the shard gives no
/// color.
const INK: Color32 = Color32::from_rgb(24, 20, 16);
const VEIL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);
const FIELD_PAD: i8 = 2;
const COLOR_CHANNEL_MAX: u32 = 31;
const RED_SHIFT: u32 = 10;
const GREEN_SHIFT: u32 = 5;
const LINE_BREAKS: [&str; 4] = ["<br>", "<br/>", "</p>", "</div>"];

/// What the human did to one gump before he answers it.
#[derive(Default)]
struct State {
    page: Option<u32>,
    /// The boxes the human set, by their switch.
    ticks: HashMap<u32, bool>,
    typed: HashMap<u16, String>,
    /// How far the human moved the gump from the place the shard gave it.
    moved: Vec2,
}

#[derive(Default)]
pub struct GumpUi {
    states: HashMap<u32, State>,
}

/// A 15-bit color of the game as a color of the window.
fn game_color(color: u32) -> Color32 {
    let channel = |shift: u32| {
        (((color >> shift) & COLOR_CHANNEL_MAX) * u32::from(u8::MAX) / COLOR_CHANNEL_MAX) as u8
    };
    Color32::from_rgb(channel(RED_SHIFT), channel(GREEN_SHIFT), channel(0))
}

/// The words of an HTML piece with no tags. A break tag starts a new line.
fn html_lines(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let close = rest[open..]
            .find('>')
            .map_or(rest.len(), |end| open + end + 1);
        let tag = rest[open..close].to_ascii_lowercase();
        if LINE_BREAKS.contains(&tag.as_str()) {
            out.push('\n');
        }
        rest = &rest[close..];
    }
    out.push_str(rest);
    out.replace("&nbsp;", " ")
}

/// The boxes that are ticked now: the ticks the gump came with, with the
/// ticks of the human on top.
fn ticked(layout: &GumpLayout, ticks: &HashMap<u32, bool>) -> Vec<u32> {
    layout
        .pieces
        .iter()
        .filter_map(|piece| match piece.what {
            GumpPieceKind::Choice { switch, ticked, .. } => ticks
                .get(&switch)
                .copied()
                .unwrap_or(ticked)
                .then_some(switch),
            _ => None,
        })
        .collect()
}

/// A click on a box. A radio box clears the other radio boxes of its page.
fn click_box(layout: &GumpLayout, ticks: &mut HashMap<u32, bool>, clicked: &GumpPiece) {
    let GumpPieceKind::Choice { switch, radio, .. } = clicked.what else {
        return;
    };
    let on = ticked(layout, ticks).contains(&switch);
    if !radio {
        ticks.insert(switch, !on);
        return;
    }
    for piece in layout.pieces.iter().filter(|p| p.page == clicked.page) {
        if let GumpPieceKind::Choice {
            switch: rival,
            radio: true,
            ..
        } = piece.what
        {
            ticks.insert(rival, rival == switch);
        }
    }
}

/// Lays one picture side by side over an area, cut at the edges.
fn tile(
    painter: &egui::Painter,
    texture: egui::TextureId,
    sprite: super::atlas::Sprite,
    area: Rect,
) {
    if sprite.width < 1.0 || sprite.height < 1.0 {
        return;
    }
    let mut y = area.top();
    while y < area.bottom() {
        let height = sprite.height.min(area.bottom() - y);
        let mut x = area.left();
        while x < area.right() {
            let width = sprite.width.min(area.right() - x);
            let uv = Rect::from_min_size(
                sprite.uv.min,
                Vec2::new(
                    sprite.uv.width() * width / sprite.width,
                    sprite.uv.height() * height / sprite.height,
                ),
            );
            let part = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
            painter.image(texture, part, uv, Color32::WHITE);
            x += sprite.width;
        }
        y += sprite.height;
    }
}

impl GumpUi {
    /// Draws each open gump. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Vec<Rect> {
        self.states
            .retain(|gump, _| frame.gump_layouts.iter().any(|l| l.gump == *gump));
        frame
            .gump_layouts
            .iter()
            .map(|layout| self.gump(ui, rect, layout, frame, tools))
            .collect()
    }

    fn gump(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        layout: &GumpLayout,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let state = self.states.entry(layout.gump).or_default();
        let page = state.page.unwrap_or(FIRST_PAGE);
        let origin = rect.min + Vec2::new(layout.x as f32, layout.y as f32) + state.moved;
        let live = frame.human_control;
        let painter = ui.painter().clone();
        let mut bounds = Rect::NOTHING;
        // The body takes the drags and the right click. The buttons come
        // after it, so they win their own clicks.
        let body_id = Id::new(("gump-body", layout.gump));
        let mut pressed: Option<Act> = None;
        let shown: Vec<&GumpPiece> = layout
            .pieces
            .iter()
            .filter(|piece| on_page(piece.page, page))
            .collect();
        let sized =
            |w: i32, h: i32, at: Pos2| Rect::from_min_size(at, Vec2::new(w as f32, h as f32));
        let body_area = shown.iter().fold(Rect::NOTHING, |all, piece| {
            let at = origin + Vec2::new(piece.x as f32, piece.y as f32);
            match piece.what {
                GumpPieceKind::Background { w, h, .. } | GumpPieceKind::Tiled { w, h, .. } => {
                    all.union(sized(w, h, at))
                }
                _ => all,
            }
        });
        let body = (body_area != Rect::NOTHING)
            .then(|| ui.interact(body_area, body_id, Sense::click_and_drag()));
        for (index, piece) in shown.into_iter().enumerate() {
            let at = origin + Vec2::new(piece.x as f32, piece.y as f32);
            let key = Id::new(("gump-piece", layout.gump, page, index));
            let area = match &piece.what {
                GumpPieceKind::Background { w, h, gump } => {
                    let area = sized(*w, *h, at);
                    frame_of_nine(&painter, tools, *gump, area);
                    area
                }
                GumpPieceKind::Tiled { w, h, gump } => {
                    let area = sized(*w, *h, at);
                    if let Some((texture, sprite)) = tools.scene.gump_picture(*gump, 0) {
                        tile(&painter, texture, sprite, area);
                    }
                    area
                }
                GumpPieceKind::Image { gump, hue } => picture(&painter, tools, *gump, *hue, at),
                GumpPieceKind::Item { graphic, hue } => {
                    match tools.scene.item_picture(frame.map, *graphic, *hue) {
                        Some((texture, sprite)) => {
                            let area =
                                Rect::from_min_size(at, Vec2::new(sprite.width, sprite.height));
                            painter.image(texture, area, sprite.uv, Color32::WHITE);
                            area
                        }
                        None => Rect::from_min_size(at, Vec2::ZERO),
                    }
                }
                GumpPieceKind::Words {
                    w,
                    h,
                    hue,
                    color,
                    html,
                    text,
                } => {
                    let color = match (color, hue) {
                        (Some(color), _) => game_color(*color),
                        (None, 0) => INK,
                        (None, hue) => tools.scene.words_color(*hue),
                    };
                    let words = if *html {
                        html_lines(text)
                    } else {
                        text.clone()
                    };
                    let mut job = LayoutJob::single_section(
                        words,
                        TextFormat::simple(text_font(WORDS_SIZE), color),
                    );
                    job.wrap.max_width = if *w > 0 { *w as f32 } else { f32::INFINITY };
                    let galley = painter.layout_job(job);
                    let area = if *w > 0 && *h > 0 {
                        sized(*w, *h, at)
                    } else {
                        Rect::from_min_size(at, galley.size())
                    };
                    painter.with_clip_rect(area).galley(at, galley, color);
                    area
                }
                GumpPieceKind::Button {
                    normal,
                    pressed: down,
                    id,
                    to_page,
                } => {
                    let size = tools
                        .scene
                        .gump_picture(*normal, 0)
                        .map_or(Vec2::ZERO, |(_, s)| Vec2::new(s.width, s.height));
                    let area = Rect::from_min_size(at, size);
                    let response = ui.interact(area, key, Sense::click());
                    let shown = if live && response.is_pointer_button_down_on() {
                        *down
                    } else {
                        *normal
                    };
                    picture(&painter, tools, shown, 0, at);
                    if live && response.clicked() {
                        match (id, to_page) {
                            (Some(id), _) => {
                                pressed = Some(Act::GumpButton {
                                    gump: layout.gump,
                                    button: *id,
                                    switches: ticked(layout, &state.ticks),
                                    texts: state
                                        .typed
                                        .iter()
                                        .map(|(id, words)| (*id, words.clone()))
                                        .collect(),
                                });
                            }
                            (None, Some(to_page)) => state.page = Some(*to_page),
                            (None, None) => {}
                        }
                    }
                    area
                }
                GumpPieceKind::Choice {
                    off, on, switch, ..
                } => {
                    let is_on = ticked(layout, &state.ticks).contains(switch);
                    let area = picture(&painter, tools, if is_on { *on } else { *off }, 0, at);
                    if live && ui.interact(area, key, Sense::click()).clicked() {
                        click_box(layout, &mut state.ticks, piece);
                    }
                    area
                }
                GumpPieceKind::Entry {
                    w,
                    h,
                    hue,
                    id,
                    text,
                    limit,
                } => {
                    let area = sized(*w, *h, at);
                    let color = match hue {
                        0 => INK,
                        hue => tools.scene.words_color(*hue),
                    };
                    let words = state.typed.entry(*id).or_insert_with(|| text.clone());
                    let limit = limit.map_or(usize::MAX, |limit| limit as usize);
                    ui.add_enabled_ui(live, |ui| {
                        ui.put(
                            area,
                            egui::TextEdit::singleline(words)
                                .frame(false)
                                .char_limit(limit)
                                .margin(egui::Margin::same(FIELD_PAD))
                                .font(text_font(WORDS_SIZE))
                                .text_color(color),
                        );
                    });
                    area
                }
                GumpPieceKind::Veil { w, h } => {
                    let area = sized(*w, *h, at);
                    painter.rect_filled(area, CornerRadius::ZERO, VEIL);
                    area
                }
            };
            bounds = bounds.union(area);
        }
        if let Some(body) = body.filter(|_| live) {
            if body.dragged() && !layout.no_move {
                state.moved += body.drag_delta();
            }
            if body.secondary_clicked() && !layout.no_close {
                pressed = Some(Act::GumpClose(layout.gump));
            }
        }
        if let Some(act) = pressed {
            tools.hand.act(act);
        }
        bounds.intersect(rect)
    }
}

/// Draws one gump picture at its own size. Gives its place.
fn picture(painter: &egui::Painter, tools: &mut Tools<'_>, gump: u16, hue: u16, at: Pos2) -> Rect {
    match tools.scene.gump_picture(gump, hue) {
        Some((texture, sprite)) => {
            let area = Rect::from_min_size(at, Vec2::new(sprite.width, sprite.height));
            painter.image(texture, area, sprite.uv, Color32::WHITE);
            area
        }
        None => Rect::from_min_size(at, Vec2::ZERO),
    }
}

/// A frame of nine pictures: the corners at their own size, the edges and
/// the middle laid side by side between them.
fn frame_of_nine(painter: &egui::Painter, tools: &mut Tools<'_>, first: u16, area: Rect) {
    let parts: Vec<_> = (0..FRAME_PARTS)
        .map(|part| tools.scene.gump_picture(first + part, 0))
        .collect();
    let size = |part: usize| {
        parts[part]
            .as_ref()
            .map_or(Vec2::ZERO, |(_, s)| Vec2::new(s.width, s.height))
    };
    let (top_left, bottom_right) = (size(0), size(8));
    let columns = [
        area.left(),
        area.left() + top_left.x,
        area.right() - bottom_right.x,
        area.right(),
    ];
    let rows = [
        area.top(),
        area.top() + top_left.y,
        area.bottom() - bottom_right.y,
        area.bottom(),
    ];
    // The middle goes first, so the edges lie on it.
    let order = std::iter::once(usize::from(FRAME_MIDDLE))
        .chain((0..usize::from(FRAME_PARTS)).filter(|part| *part != usize::from(FRAME_MIDDLE)));
    for part in order {
        let Some((texture, sprite)) = parts[part] else {
            continue;
        };
        let (column, row) = (part % 3, part / 3);
        let cell = Rect::from_min_max(
            Pos2::new(columns[column], rows[row]),
            Pos2::new(columns[column + 1], rows[row + 1]),
        );
        if cell.width() > 0.0 && cell.height() > 0.0 {
            tile(painter, texture, sprite, cell);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(page: u32, switch: u32, radio: bool, ticked: bool) -> GumpPiece {
        GumpPiece {
            page,
            x: 0,
            y: 0,
            what: GumpPieceKind::Choice {
                off: 210,
                on: 211,
                switch,
                radio,
                ticked,
            },
        }
    }

    #[test]
    fn a_radio_box_clears_its_rivals_and_a_check_box_flips() {
        let layout = GumpLayout {
            pieces: vec![
                choice(1, 1, true, true),
                choice(1, 2, true, false),
                choice(1, 3, false, false),
                choice(2, 4, true, true),
            ],
            ..GumpLayout::default()
        };
        let mut ticks = HashMap::new();
        assert_eq!(ticked(&layout, &ticks), vec![1, 4]);
        click_box(&layout, &mut ticks, &layout.pieces[1]);
        assert_eq!(ticked(&layout, &ticks), vec![2, 4], "page 2 keeps its tick");
        click_box(&layout, &mut ticks, &layout.pieces[2]);
        assert_eq!(ticked(&layout, &ticks), vec![2, 3, 4]);
        click_box(&layout, &mut ticks, &layout.pieces[2]);
        assert_eq!(ticked(&layout, &ticks), vec![2, 4]);
    }

    #[test]
    fn html_words_lose_their_tags_and_keep_their_lines() {
        assert_eq!(
            html_lines("<center><b>Runebook</b></center><BR>Charges:&nbsp;5"),
            "Runebook\nCharges: 5"
        );
        assert_eq!(html_lines("no tags"), "no tags");
        assert_eq!(html_lines("cut <b"), "cut ");
    }

    #[test]
    fn a_game_color_is_five_bits_for_each_channel() {
        assert_eq!(game_color(0x7FFF), Color32::from_rgb(255, 255, 255));
        assert_eq!(game_color(0x7C00), Color32::from_rgb(255, 0, 0));
        assert_eq!(game_color(0x001F), Color32::from_rgb(0, 0, 255));
    }
}
