//! The colors, sizes and type of the watch window. No other file of the
//! window names a color or a size.

use eframe::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontFamily, FontId, Painter, Rect, Stroke,
    StrokeKind,
};

// Ground behind the map, and the glass of the panels. The panels are tinted
// toward blue so that they read as one family on any terrain.
pub const VOID: Color32 = Color32::from_rgb(6, 8, 12);
pub const GLASS: Color32 = Color32::from_rgba_premultiplied(9, 13, 20, 224);
pub const GLASS_EDGE: Color32 = Color32::from_rgba_premultiplied(44, 52, 66, 140);
pub const BUTTON: Color32 = Color32::from_rgba_premultiplied(40, 48, 62, 230);
pub const BUTTON_HOVER: Color32 = Color32::from_rgba_premultiplied(62, 74, 94, 240);
pub const TRACK: Color32 = Color32::from_rgba_premultiplied(2, 3, 5, 190);

pub const TEXT: Color32 = Color32::from_rgb(236, 240, 247);
pub const TEXT_DIM: Color32 = Color32::from_rgb(158, 170, 190);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(112, 124, 146);
pub const TEXT_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 215);

/// The one alarm color. Nothing else in the window is this red.
pub const ALARM: Color32 = Color32::from_rgb(255, 64, 56);
pub const GOAL: Color32 = Color32::from_rgb(92, 224, 236);
pub const WAITING: Color32 = Color32::from_rgb(255, 196, 84);

pub const HITS: Color32 = Color32::from_rgb(222, 58, 64);
pub const HITS_POISONED: Color32 = Color32::from_rgb(84, 196, 92);
pub const MANA: Color32 = Color32::from_rgb(66, 132, 246);
pub const STAM: Color32 = Color32::from_rgb(236, 186, 60);
/// The part of a bar that was lost a moment ago.
pub const BAR_GHOST: Color32 = Color32::from_rgb(250, 238, 222);

pub const NOTO_SELF: Color32 = Color32::from_rgb(255, 214, 92);
/// The figure of the character on the map. No notoriety has this color.
pub const SELF_FIGURE: Color32 = Color32::from_rgb(250, 250, 252);
pub const PLATE_BACK: Color32 = Color32::from_rgba_premultiplied(4, 6, 10, 165);
const NOTO_INNOCENT: Color32 = Color32::from_rgb(92, 164, 255);
const NOTO_FRIEND: Color32 = Color32::from_rgb(84, 212, 120);
const NOTO_GREY: Color32 = Color32::from_rgb(178, 184, 196);
const NOTO_ENEMY: Color32 = Color32::from_rgb(255, 150, 58);
// A rose red, so that it is not the alarm red.
const NOTO_MURDERER: Color32 = Color32::from_rgb(244, 72, 128);
const NOTO_INVULNERABLE: Color32 = Color32::from_rgb(246, 226, 110);

// The map drawn without client files.
pub const FLAT_WALK: Color32 = Color32::from_rgb(38, 62, 50);
pub const FLAT_WALK_ALT: Color32 = Color32::from_rgb(35, 57, 46);
/// A tile past the edge of the radar. Nothing is known about it.
pub const FLAT_UNKNOWN: Color32 = Color32::from_rgb(20, 28, 30);
pub const FLAT_WATER: Color32 = Color32::from_rgb(24, 58, 98);
pub const FLAT_BLOCK_TOP: Color32 = Color32::from_rgb(86, 92, 106);
pub const FLAT_BLOCK_LEFT: Color32 = Color32::from_rgb(58, 63, 75);
pub const FLAT_BLOCK_RIGHT: Color32 = Color32::from_rgb(42, 46, 56);
pub const FLAT_DOOR: Color32 = Color32::from_rgb(176, 124, 62);
pub const FLAT_ITEM: Color32 = Color32::from_rgb(222, 178, 86);
pub const CORPSE: Color32 = Color32::from_rgb(214, 204, 184);

pub const PANEL_RADIUS: u8 = 10;
pub const PANEL_PAD: f32 = 14.0;
pub const SCREEN_MARGIN: f32 = 16.0;
pub const ROW_GAP: f32 = 6.0;
pub const BAR_HEIGHT: f32 = 12.0;
/// The hits bar is the one that tells of danger, so it is the largest.
pub const BAR_HEIGHT_MAIN: f32 = 18.0;
pub const BAR_RADIUS: u8 = 3;
pub const PIP_HEIGHT: f32 = 4.0;

pub const SIZE_TITLE: f32 = 22.0;
pub const SIZE_BODY: f32 = 14.0;
pub const SIZE_SMALL: f32 = 12.0;
pub const SIZE_PLATE: f32 = 15.0;

const PANEL_SHADOW: Shadow = Shadow {
    offset: [0, 6],
    blur: 22,
    spread: 0,
    color: Color32::from_rgba_premultiplied(0, 0, 0, 150),
};
const EDGE_WIDTH: f32 = 1.0;

// Barlow, under the SIL Open Font License. See assets/fonts/OFL.txt.
const TEXT_FACE: &str = "Barlow-Medium";
const TITLE_FACE: &str = "BarlowCondensed-SemiBold";
const TEXT_FACE_BYTES: &[u8] = include_bytes!("../../assets/fonts/Barlow-Medium.ttf");
const TITLE_FACE_BYTES: &[u8] = include_bytes!("../../assets/fonts/BarlowCondensed-SemiBold.ttf");
const TEXT_SHADOW_OFFSET: egui::Vec2 = egui::vec2(1.0, 1.0);

const NOTORIETY_INNOCENT: u8 = 1;
const NOTORIETY_FRIEND: u8 = 2;
const NOTORIETY_ENEMY: u8 = 5;
const NOTORIETY_MURDERER: u8 = 6;
const NOTORIETY_INVULNERABLE: u8 = 7;

/// Grey and criminal share one grey, as in the game.
pub fn notoriety_color(notoriety: u8) -> Color32 {
    match notoriety {
        NOTORIETY_INNOCENT => NOTO_INNOCENT,
        NOTORIETY_FRIEND => NOTO_FRIEND,
        NOTORIETY_ENEMY => NOTO_ENEMY,
        NOTORIETY_MURDERER => NOTO_MURDERER,
        NOTORIETY_INVULNERABLE => NOTO_INVULNERABLE,
        _ => NOTO_GREY,
    }
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

/// One floating glass panel: shadow, fill, light edge.
pub fn panel(painter: &Painter, rect: Rect) {
    let radius = CornerRadius::same(PANEL_RADIUS);
    painter.add(PANEL_SHADOW.as_shape(rect, radius));
    painter.rect_filled(rect, radius, GLASS);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(EDGE_WIDTH, GLASS_EDGE),
        StrokeKind::Inside,
    );
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

pub const CELL_ART_PAD: f32 = 4.0;
/// Small pictures grow to this, so a coin is not a dot. More would blur.
const ART_MAX_SCALE: f32 = 2.0;

/// A picture scaled to fit a cell, with its proportions kept.
pub fn fit(cell: Rect, width: f32, height: f32) -> Rect {
    let room = cell.shrink(CELL_ART_PAD);
    let scale = (room.width() / width)
        .min(room.height() / height)
        .min(ART_MAX_SCALE);
    Rect::from_center_size(room.center(), egui::Vec2::new(width, height) * scale)
}

pub fn install(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = VOID;
    visuals.window_fill = VOID;
    ctx.set_visuals(visuals);
    ctx.set_fonts(fonts());
}

/// The bundled egui faces stay as the last choice, so a glyph that Barlow
/// does not hold still shows.
fn fonts() -> egui::FontDefinitions {
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
    let mut title = vec![TITLE_FACE.to_string()];
    title.extend(bundled);
    fonts
        .families
        .insert(FontFamily::Name(TITLE_FACE.into()), title);
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_picture_shrinks_to_the_cell_and_a_small_one_grows_a_little() {
        const CELL: f32 = 46.0;
        let cell = Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(CELL));
        let large = fit(cell, 100.0, 50.0);
        assert!(large.width() <= CELL && (large.width() / large.height() - 2.0).abs() < 0.01);
        let small = fit(cell, 10.0, 10.0);
        assert_eq!(small.size(), egui::Vec2::splat(10.0 * ART_MAX_SCALE));
    }
}
