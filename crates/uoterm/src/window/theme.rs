//! The colors, sizes and type of the watch window. No other file of the
//! window names a color or a size. The colors and the sizes are those of
//! `uoterm_view::ui::theme`; the type, the glass and the buttons are
//! egui's, here.

pub use uoterm_view::ui::theme::{
    BAR_HEIGHT, BAR_HEIGHT_MAIN, CELL_ART_PAD, PANEL_PAD, PIP_HEIGHT, ROW_GAP, SCREEN_MARGIN,
    SIZE_BODY, SIZE_HEADING, SIZE_PLATE, SIZE_SMALL, SIZE_TITLE,
};

use super::bridge;
use eframe::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontFamily, FontId, FontTweak, Painter, Rect,
    Stroke, StrokeKind,
};
use std::sync::atomic::{AtomicU8, Ordering};
use uoterm_view::ui::theme as shared;

/// The colors of the theme as egui keeps them. Each is the color of
/// `uoterm_view::ui::theme`, which the browser shares.
macro_rules! egui_colors {
    ($($name:ident),* $(,)?) => {
        $(pub const $name: Color32 = bridge::color(shared::$name);)*
    };
}

egui_colors!(
    VOID,
    GLASS,
    GLASS_EDGE,
    BUTTON,
    BUTTON_HOVER,
    TRACK,
    CHOSEN,
    DARK_GLASS,
    TEXT,
    TEXT_DIM,
    TEXT_FAINT,
    TEXT_SHADOW,
    ALARM,
    GOAL,
    WAITING,
    HITS,
    HITS_POISONED,
    MANA,
    STAM,
    BAR_GHOST,
    NOTO_SELF,
    PLATE_BACK,
    VIGNETTE,
);

pub const PANEL_RADIUS: u8 = shared::PANEL_RADIUS as u8;
pub const BAR_RADIUS: u8 = shared::BAR_RADIUS as u8;

const PANEL_SHADOW: Shadow = Shadow {
    offset: [0, shared::PANEL_SHADOW_DROP as i8],
    blur: shared::PANEL_SHADOW_BLUR as u8,
    spread: 0,
    color: bridge::color(shared::PANEL_SHADOW),
};
const PERCENT: f32 = 100.0;
/// The opacity of the panels, in percent, from the Interface page.
static PANEL_OPACITY: AtomicU8 = AtomicU8::new(100);
/// The player's own font takes the place of Barlow under this name.
const PLAYER_FACE: &str = "player-font";

// Barlow, under the SIL Open Font License. See assets/fonts/OFL.txt.
const TEXT_FACE: &str = "Barlow-Medium";
const TITLE_FACE: &str = "BarlowCondensed-SemiBold";
const TEXT_FACE_BYTES: &[u8] = include_bytes!("../../assets/fonts/Barlow-Medium.ttf");
const TITLE_FACE_BYTES: &[u8] = include_bytes!("../../assets/fonts/BarlowCondensed-SemiBold.ttf");
const TEXT_SHADOW_OFFSET: egui::Vec2 = egui::vec2(1.0, 1.0);

/// Grey and criminal share one grey, as in the game.
pub fn notoriety_color(notoriety: u8) -> Color32 {
    bridge::color(shared::notoriety_color(notoriety))
}

pub fn text_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// The heavy, narrow face for panel titles and for names on the map. It
/// stays readable on busy art.
pub fn title_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(TITLE_FACE.into()))
}

/// Numbers are measurements, so they take the fixed-width face and do not
/// move when a digit changes.
pub fn number_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    color.gamma_multiply(alpha.clamp(0.0, 1.0))
}

/// Sets how opaque every panel is, in percent.
pub fn set_panel_opacity(percent: u8) {
    PANEL_OPACITY.store(percent, Ordering::Relaxed);
}

/// The share of the panel opacity, from 0 to 1.
pub fn panel_opacity() -> f32 {
    f32::from(PANEL_OPACITY.load(Ordering::Relaxed)) / PERCENT
}

/// One floating glass panel: shadow, fill, light edge.
pub fn panel(painter: &Painter, rect: Rect) {
    panel_with(painter, rect, GLASS, GLASS_EDGE);
}

/// A glass panel of its own fill and edge, under the panel opacity.
pub fn panel_with(painter: &Painter, rect: Rect, fill: Color32, edge: Color32) {
    let radius = CornerRadius::same(PANEL_RADIUS);
    let opacity = panel_opacity();
    let shadow = Shadow {
        color: with_alpha(PANEL_SHADOW.color, opacity),
        ..PANEL_SHADOW
    };
    painter.add(shadow.as_shape(rect, radius));
    painter.rect_filled(rect, radius, with_alpha(fill, opacity));
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(shared::EDGE_WIDTH, with_alpha(edge, opacity)),
        StrokeKind::Inside,
    );
}

/// A bar: its dark track, and its fill for a share from 0 to 1.
pub fn bar(painter: &Painter, track: Rect, share: f32, fill: Color32) {
    painter.rect_filled(track, CornerRadius::same(BAR_RADIUS), TRACK);
    let mut part = track;
    part.set_width(track.width() * share.clamp(0.0, 1.0));
    painter.rect_filled(part, CornerRadius::same(BAR_RADIUS), fill);
}

/// Text that stays readable on the map: a dark copy one pixel down-right.
pub fn shadowed_text(
    painter: &Painter,
    pos: egui::Pos2,
    anchor: egui::Align2,
    text: &str,
    font: FontId,
    color: Color32,
) -> Rect {
    painter.text(
        pos + TEXT_SHADOW_OFFSET,
        anchor,
        text,
        font.clone(),
        TEXT_SHADOW,
    );
    painter.text(pos, anchor, text, font, color)
}

const BUTTON_PAD: egui::Vec2 = egui::vec2(12.0, 6.0);
const BUTTON_RADIUS: u8 = 6;

/// One text button. Gives its place, and true when it was clicked.
pub fn button(ui: &egui::Ui, left_top: egui::Pos2, words: &str, color: Color32) -> (Rect, bool) {
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(words.to_string(), text_font(SIZE_BODY), color);
    let rect = Rect::from_min_size(left_top, galley.size() + BUTTON_PAD * 2.0);
    let response = ui.interact(rect, egui::Id::new(("button", words)), egui::Sense::click());
    let fill = if response.hovered() {
        BUTTON_HOVER
    } else {
        BUTTON
    };
    painter.rect_filled(rect, CornerRadius::same(BUTTON_RADIUS), fill);
    painter.galley(rect.min + BUTTON_PAD, galley, color);
    (rect, response.clicked())
}

/// One button that fills `area`, with its words in the middle. A row of
/// these with equal areas is one tidy strip. True when it was clicked.
pub fn segment(ui: &egui::Ui, area: Rect, words: &str, color: Color32) -> bool {
    segment_keyed(ui, area, egui::Id::new(("segment", words)), words, color)
}

/// A segment whose words are not its own alone, such as the "+" of each row
/// of a list. The caller gives the key.
pub fn segment_keyed(
    ui: &egui::Ui,
    area: Rect,
    key: egui::Id,
    words: &str,
    color: Color32,
) -> bool {
    let response = ui.interact(area, key, egui::Sense::click());
    let fill = if response.hovered() {
        BUTTON_HOVER
    } else {
        BUTTON
    };
    ui.painter()
        .rect_filled(area, CornerRadius::same(BUTTON_RADIUS), fill);
    ui.painter().text(
        area.center(),
        egui::Align2::CENTER_CENTER,
        words,
        text_font(SIZE_BODY),
        color,
    );
    response.clicked()
}

/// A picture scaled to fit a cell, with its proportions kept.
pub fn fit(cell: Rect, width: f32, height: f32) -> Rect {
    bridge::rect(shared::fit(bridge::area(cell), width, height))
}

/// A picture that shrinks to fit a cell, and grows no more than
/// `max_scale`.
pub fn fit_up_to(cell: Rect, width: f32, height: f32, max_scale: f32) -> Rect {
    bridge::rect(shared::fit_up_to(
        bridge::area(cell),
        width,
        height,
        max_scale,
    ))
}

pub fn install(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = VOID;
    visuals.window_fill = VOID;
    ctx.set_visuals(visuals);
    ctx.set_fonts(fonts(None));
}

/// Puts the player's TrueType font in the place of Barlow, at its scale,
/// or Barlow back with None.
pub fn use_player_font(ctx: &egui::Context, font: Option<(Vec<u8>, f32)>) {
    ctx.set_fonts(fonts(font));
}

/// The bundled egui faces stay as the last choice, so a glyph that Barlow
/// does not hold still shows. The player's font, when he chose one, comes
/// before Barlow.
fn fonts(player: Option<(Vec<u8>, f32)>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let bundled = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (face, bytes) in [(TEXT_FACE, TEXT_FACE_BYTES), (TITLE_FACE, TITLE_FACE_BYTES)] {
        fonts.font_data.insert(
            face.to_string(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, TEXT_FACE.to_string());
    if let Some((bytes, scale)) = player {
        fonts.font_data.insert(
            PLAYER_FACE.to_string(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes).tweak(FontTweak {
                scale,
                ..FontTweak::default()
            })),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, PLAYER_FACE.to_string());
    }
    let mut title = vec![TITLE_FACE.to_string()];
    title.extend(bundled);
    fonts
        .families
        .insert(FontFamily::Name(TITLE_FACE.into()), title);
    fonts
}
