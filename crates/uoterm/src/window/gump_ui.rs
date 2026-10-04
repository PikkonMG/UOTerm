//! A gump of the shard (0xB0 and 0xDD), drawn as its maker made it: each
//! picture, line of words, button and field at its own place, from the gump
//! art of the client, in the fonts and by the rules the reference client reads each
//! command with. It is a floating gump of the gump manager in both styles.
//! What the human did to it and its answer are
//! `uoterm_view::ui::shard_gump`, which the browser shares.

use super::bridge;
use super::classic::canvas::{ButtonArt, Canvas};
use super::classic::registry::{
    well_known, Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpLocks, GumpRules,
};
use super::classic::text::{TextLook, HTML_FONT};
use super::classic::GumpManager;
use super::settings::Profile;
use crate::view::WatchFrame;
use eframe::egui::{Pos2, Vec2};
use uoterm_view::ui::gumps::{picture_hue, shown_hue};
use uoterm_view::ui::html::HtmlBox;
use uoterm_view::ui::shard_gump::{
    gump_first_place, tile_art_place, veiled, PieceArt, ShardGumpState, UNDER_VEIL,
};
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

/// One gump of the shard in the gump manager.
pub struct ShardGump {
    state: ShardGumpState,
}

impl ShardGump {
    fn new(gump: u32) -> Self {
        Self {
            state: ShardGumpState::new(gump),
        }
    }
}

/// The size of a picture of a piece, as the canvas has it.
fn art_size(g: &mut Canvas<'_>, art: PieceArt) -> Option<uoterm_view::geom::Vector> {
    let size = match art {
        PieceArt::Gump(gump) => g.gump_size(gump)?,
        PieceArt::Item(graphic) => g.item_size(graphic),
    };
    Some(bridge::vector(size))
}

impl GumpBody for ShardGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(layout) = self.state.layout(cx.frame) else {
            return;
        };
        let shown = self.state.shown(layout);
        let faded = veiled(&shown, |art| art_size(g, art));
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
                if let Some(lines) = cx.tips.lines_of(serial, |serial| cx.hand.want_tip(serial)) {
                    g.tooltip(&lines.join(&LINE_BREAK.to_string()));
                }
            }
        }
        if let Some(button) = reply {
            cx.act(self.state.answer(layout, button));
        }
    }

    fn first_place(&self, frame: &WatchFrame) -> Option<Pos2> {
        self.state
            .layout(frame)
            .map(|layout| bridge::pos2(gump_first_place(layout)))
    }

    fn locks(&self, frame: &WatchFrame) -> GumpLocks {
        self.state
            .layout(frame)
            .map_or_else(GumpLocks::default, |layout| GumpLocks {
                no_move: layout.no_move,
                no_close: layout.no_close,
            })
    }

    /// A right click answers the gump with no button, as the classic
    /// client does. It stays until the shard takes it away.
    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(self.state.close());
        Closing::Wait
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.state.layout(frame).is_some()
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
        let key = (self.state.page, index);
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
                art,
                ..
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
                        let item = bridge::vector(g.item_size(tile.graphic));
                        let (left, top) = tile_art_place(x, y, tile, item);
                        g.item(left, top, tile.graphic, tile.hue);
                        clicked
                    }
                    None => g.button(key, x, y, button),
                };
                if clicked {
                    return self.state.press(piece);
                }
            }
            GumpPieceKind::Choice {
                off,
                on,
                switch,
                radio,
                ..
            } => {
                let is_on = self.state.is_ticked(layout, *switch);
                let clicked = if *radio {
                    g.radio(key, x, y, (*off, *on), is_on, None)
                } else {
                    let mut flipped = is_on;
                    g.checkbox(key, x, y, (*off, *on), &mut flipped, None)
                };
                if clicked {
                    self.state.click_box(layout, piece);
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
                let look = TextLook::unicode(HTML_FONT, shown_hue(*hue)).bordered();
                if self.state.take_focus() {
                    g.focus(key);
                }
                let field = self.state.field(*id, text, *limit);
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

    #[test]
    fn a_gump_of_the_shard_opens_draws_and_closes_with_its_layout() {
        use crate::window::classic::testing::draw_frames;
        let layout = GumpLayout {
            gump: 77,
            pieces: vec![
                GumpPiece {
                    page: 0,
                    x: 0,
                    y: 0,
                    what: GumpPieceKind::Choice {
                        off: 210,
                        on: 211,
                        switch: 1,
                        radio: true,
                        ticked: true,
                        group: 0,
                    },
                    tooltip: None,
                    property: None,
                },
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
}
