//! A gump of the shard (0xB0 and 0xDD), drawn as its maker made it: each
//! picture, line of words, button and field at its own place, from the gump
//! art of the client, in the fonts and by the rules the reference client reads each
//! command with. It is a floating gump of the gump manager in both styles.
//! The pages, the boxes and the fields work at all times; a reply goes to
//! the shard only while the human has control.

use super::boxes_ui::{on_page, FIRST_PAGE};
use super::classic::canvas::{ButtonArt, Canvas, HtmlBox};
use super::classic::registry::{
    well_known, Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpLocks, GumpRules,
};
use super::classic::text::{TextLook, HTML_FONT};
use super::classic::text_field::TextField;
use super::classic::GumpManager;
use super::control::Act;
use super::settings::Profile;
use crate::view::WatchFrame;
use eframe::egui::{Pos2, Rect, Vec2};
use std::collections::HashMap;
use uoterm_world::{GumpLayout, GumpPiece, GumpPieceKind};

pub const SHARD_GUMP: GumpKind = GumpKind {
    id: well_known::SHARD,
    rules: GumpRules {
        both_styles: true,
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(ShardGump::new(serial.unwrap_or_default())),
};

/// A text entry with no limit of its own takes this many chars.
const ENTRY_MAX_CHARS: usize = u8::MAX as usize;
/// A picture hue this low or lower shows the picture as it is.
const PLAIN_HUE_MAX: u16 = 2;
/// Words under a `checkertrans` show at this share of their opacity.
const UNDER_VEIL: f32 = 0.5;
const LINE_BREAK: char = '\n';

/// Opens a gump for each layout the shard sent that is not open yet. A gump
/// the shard took away closes by itself.
pub fn sync(manager: &mut GumpManager, frame: &WatchFrame, profile: &mut Profile) {
    for layout in &frame.gump_layouts {
        let id = GumpId::of(well_known::SHARD, layout.gump);
        if !manager.is_open(&id) {
            manager.open(id, profile);
        }
    }
}

/// The hue a gump picture or a line of words takes, from the number the
/// shard sent: the classic client counts one more.
fn shown_hue(hue: u16) -> u16 {
    hue.wrapping_add(1)
}

/// The hue of a gump picture, where the lowest hues show no hue.
fn picture_hue(hue: u16) -> u16 {
    let hue = shown_hue(hue);
    if hue <= PLAIN_HUE_MAX {
        0
    } else {
        hue
    }
}

/// What the human did to one gump before he answers it.
pub struct ShardGump {
    gump: u32,
    page: u32,
    /// The boxes the human set, by their switch.
    ticks: HashMap<u32, bool>,
    fields: HashMap<u16, TextField>,
    /// The first field took the keys already.
    focused: bool,
}

impl ShardGump {
    fn new(gump: u32) -> Self {
        Self {
            gump,
            page: FIRST_PAGE,
            ticks: HashMap::new(),
            fields: HashMap::new(),
            focused: false,
        }
    }

    fn layout<'f>(&self, frame: &'f WatchFrame) -> Option<&'f GumpLayout> {
        frame.gump_layouts.iter().find(|l| l.gump == self.gump)
    }
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

/// A click on a box. A radio box clears the other radio boxes of its group.
fn click_box(layout: &GumpLayout, ticks: &mut HashMap<u32, bool>, clicked: &GumpPiece) {
    let GumpPieceKind::Choice {
        switch,
        radio,
        group,
        ..
    } = clicked.what
    else {
        return;
    };
    let on = ticked(layout, ticks).contains(&switch);
    if !radio {
        ticks.insert(switch, !on);
        return;
    }
    for piece in &layout.pieces {
        if let GumpPieceKind::Choice {
            switch: rival,
            radio: true,
            group: rival_group,
            ..
        } = piece.what
        {
            if rival_group == group {
                ticks.insert(rival, rival == switch);
            }
        }
    }
}

/// The size a piece takes on the gump, as `checkertrans` measures it.
fn piece_size(g: &mut Canvas<'_>, piece: &GumpPiece) -> Vec2 {
    let size = |w: i32, h: i32| Vec2::new(w as f32, h as f32);
    match &piece.what {
        GumpPieceKind::Background { w, h, .. }
        | GumpPieceKind::Tiled { w, h, .. }
        | GumpPieceKind::Words { w, h, .. }
        | GumpPieceKind::Entry { w, h, .. }
        | GumpPieceKind::Veil { w, h } => size(*w, *h),
        GumpPieceKind::Image { gump, .. } => g.gump_size(*gump).unwrap_or(Vec2::ZERO),
        GumpPieceKind::Button { normal, art, .. } => match art {
            Some(art) if art.w > 0 && art.h > 0 => size(art.w, art.h),
            _ => g.gump_size(*normal).unwrap_or(Vec2::ZERO),
        },
        GumpPieceKind::Choice { off, .. } => g.gump_size(*off).unwrap_or(Vec2::ZERO),
        GumpPieceKind::Item { graphic, .. } => g.item_size(*graphic),
    }
}

/// Which pieces a later `checkertrans` of their page lies over, as the
/// classic client fades them.
fn veiled(g: &mut Canvas<'_>, shown: &[&GumpPiece]) -> Vec<bool> {
    let areas: Vec<Rect> = shown
        .iter()
        .map(|piece| {
            Rect::from_min_size(
                Pos2::new(piece.x as f32, piece.y as f32),
                piece_size(g, piece),
            )
        })
        .collect();
    let mut faded = vec![false; shown.len()];
    for (at, piece) in shown.iter().enumerate() {
        if !matches!(piece.what, GumpPieceKind::Veil { .. }) {
            continue;
        }
        for before in 0..at {
            let on_page = shown[before].page == 0 || shown[before].page == piece.page;
            faded[before] |= on_page && areas[before].intersects(areas[at]);
        }
    }
    faded
}

impl GumpBody for ShardGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(layout) = self.layout(cx.frame) else {
            return;
        };
        let shown: Vec<&GumpPiece> = layout
            .pieces
            .iter()
            .filter(|piece| on_page(piece.page, self.page))
            .collect();
        let faded = veiled(g, &shown);
        let mut reply = None;
        for (index, piece) in shown.iter().enumerate() {
            let share = if faded[index] { UNDER_VEIL } else { 1.0 };
            g.faded(share, |g| {
                if let Some(button) = self.piece(g, layout, piece, index) {
                    reply = Some(button);
                }
            });
            if let Some(tooltip) = &piece.tooltip {
                g.tooltip(tooltip);
            }
            if let Some(serial) = piece.property.filter(|_| g.last_hovered()) {
                if let Some(lines) = cx.tips.lines_of(cx.hand, serial) {
                    g.tooltip(&lines.join(&LINE_BREAK.to_string()));
                }
            }
        }
        if let Some(button) = reply {
            cx.act(Act::GumpButton {
                gump: self.gump,
                button,
                switches: ticked(layout, &self.ticks),
                texts: self
                    .fields
                    .iter()
                    .map(|(id, field)| (*id, field.text().to_string()))
                    .collect(),
            });
        }
    }

    fn first_place(&self, frame: &WatchFrame) -> Option<Pos2> {
        self.layout(frame)
            .map(|layout| Pos2::new(layout.x as f32, layout.y as f32))
    }

    fn locks(&self, frame: &WatchFrame) -> GumpLocks {
        self.layout(frame)
            .map_or_else(GumpLocks::default, |layout| GumpLocks {
                no_move: layout.no_move,
                no_close: layout.no_close,
            })
    }

    /// A right click answers the gump with no button, as the classic
    /// client does. It stays until the shard takes it away.
    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::GumpClose(self.gump));
        Closing::Wait
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.layout(frame).is_some()
    }
}

impl ShardGump {
    /// Draws one piece. Gives the button the human pressed, when he pressed
    /// one that answers the gump.
    fn piece(
        &mut self,
        g: &mut Canvas<'_>,
        layout: &GumpLayout,
        piece: &GumpPiece,
        index: usize,
    ) -> Option<u32> {
        let (x, y) = (piece.x, piece.y);
        let key = (self.page, index);
        match &piece.what {
            GumpPieceKind::Background { w, h, gump } => g.frame(x, y, *w, *h, *gump),
            GumpPieceKind::Image { gump, hue } => {
                g.pic(x, y, *gump, picture_hue(*hue));
            }
            GumpPieceKind::Tiled { w, h, gump } => g.pic_tiled(x, y, *w, *h, *gump, 0),
            GumpPieceKind::Item { graphic, hue } => {
                g.item(x, y, *graphic, *hue);
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
                let look = HtmlBox {
                    background: *background,
                    scroll: *scroll,
                    color: *color,
                };
                g.html(key, x, y, *w, *h, text, &look);
            }
            GumpPieceKind::Words { w, hue, text, .. } => {
                let look = TextLook::unicode(HTML_FONT, shown_hue(*hue)).bordered();
                let look = if *w > 0 {
                    look.cropped(*w as u32)
                } else {
                    look
                };
                g.label(x, y, text, &look);
            }
            GumpPieceKind::Button {
                normal,
                pressed,
                id,
                to_page,
                art,
            } => {
                let button = ButtonArt::new(*normal, *pressed, 0);
                let clicked = match art {
                    Some(tile) => {
                        let clicked = g.button_sized(
                            key,
                            x,
                            y,
                            button,
                            Vec2::new(tile.w as f32, tile.h as f32),
                        );
                        let item = g.item_size(tile.graphic);
                        let left = x + ((tile.w - item.x as i32) / 2).max(0);
                        let top = y + ((tile.h - item.y as i32) / 2).max(0);
                        g.item(left, top, tile.graphic, tile.hue);
                        clicked
                    }
                    None => g.button(key, x, y, button),
                };
                if clicked {
                    match (id, to_page) {
                        (Some(id), _) => return Some(*id),
                        (None, Some(to_page)) => self.page = *to_page,
                        (None, None) => {}
                    }
                }
            }
            GumpPieceKind::Choice {
                off,
                on,
                switch,
                radio,
                ..
            } => {
                let is_on = ticked(layout, &self.ticks).contains(switch);
                let clicked = if *radio {
                    g.radio(key, x, y, (*off, *on), is_on, None)
                } else {
                    let mut flipped = is_on;
                    g.checkbox(key, x, y, (*off, *on), &mut flipped, None)
                };
                if clicked {
                    click_box(layout, &mut self.ticks, piece);
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
                let max = limit.map_or(ENTRY_MAX_CHARS, |limit| limit as usize);
                let field = self
                    .fields
                    .entry(*id)
                    .or_insert_with(|| TextField::new(text).with_max_chars(Some(max)));
                let look = TextLook::unicode(HTML_FONT, shown_hue(*hue)).bordered();
                if !self.focused {
                    self.focused = true;
                    g.focus(key);
                }
                g.text_box(key, x, y, *w, *h, field, &look);
            }
            GumpPieceKind::Veil { w, h } => g.checker_trans(x, y, *w, *h),
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(page: u32, switch: u32, radio: bool, ticked: bool, group: u32) -> GumpPiece {
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
                group,
            },
            tooltip: None,
            property: None,
        }
    }

    #[test]
    fn a_radio_box_clears_the_radios_of_its_group_and_a_check_box_flips() {
        let layout = GumpLayout {
            pieces: vec![
                choice(1, 1, true, true, 0),
                choice(2, 2, true, false, 0),
                choice(1, 3, false, false, 0),
                choice(1, 4, true, true, 1),
            ],
            ..GumpLayout::default()
        };
        let mut ticks = HashMap::new();
        assert_eq!(ticked(&layout, &ticks), vec![1, 4]);
        click_box(&layout, &mut ticks, &layout.pieces[1]);
        assert_eq!(
            ticked(&layout, &ticks),
            vec![2, 4],
            "group 1 keeps its tick"
        );
        click_box(&layout, &mut ticks, &layout.pieces[2]);
        assert_eq!(ticked(&layout, &ticks), vec![2, 3, 4]);
        click_box(&layout, &mut ticks, &layout.pieces[2]);
        assert_eq!(ticked(&layout, &ticks), vec![2, 4]);
    }

    #[test]
    fn a_gump_of_the_shard_opens_draws_and_closes_with_its_layout() {
        use crate::window::classic::testing::draw_frames;
        let layout = GumpLayout {
            gump: 77,
            pieces: vec![
                choice(0, 1, true, true, 0),
                GumpPiece {
                    page: 0,
                    x: 0,
                    y: 30,
                    what: GumpPieceKind::Words {
                        w: 200,
                        h: 60,
                        hue: 0,
                        color: None,
                        html: true,
                        text: "<center><b>Title</b></center>".into(),
                        background: true,
                        scroll: uoterm_world::GumpScroll::Bar,
                    },
                    tooltip: Some("words".into()),
                    property: None,
                },
            ],
            ..GumpLayout::default()
        };
        let mut frame = WatchFrame {
            gump_layouts: vec![layout],
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        let mut manager = GumpManager::default();
        sync(&mut manager, &frame, &mut profile);
        let id = GumpId::of(well_known::SHARD, 77);
        assert!(manager.is_open(&id));
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        frame.gump_layouts.clear();
        draw_frames(&mut manager, &mut profile, &frame);
        assert!(!manager.is_open(&id), "the shard took it away");
        assert!(profile.gumps.is_empty(), "a gump of the shard is not kept");
    }

    #[test]
    fn hues_count_one_more_and_low_picture_hues_show_none() {
        assert_eq!(shown_hue(0), 1);
        assert_eq!(shown_hue(0x0481), 0x0482);
        assert_eq!(picture_hue(0), 0);
        assert_eq!(picture_hue(1), 0);
        assert_eq!(picture_hue(2), 3);
    }
}
