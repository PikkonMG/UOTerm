//! The colors and sizes of the Modern panels. The Rust window draws with
//! them, and `css_tokens` gives them to the page of the browser, so the
//! two windows never hold two copies.

use crate::geom::{Area, Rgba, Vector};

/// Each color once: its constant, and its line among the CSS tokens.
macro_rules! colors {
    ($($(#[$doc:meta])* $vis:vis $name:ident = $value:expr;)*) => {
        $($(#[$doc])* $vis const $name: Rgba = $value;)*
        const COLORS: &[(&str, Rgba)] = &[$((stringify!($name), $name)),*];
    };
}

/// Each size once, in screen points: its constant, and its line among the
/// CSS tokens.
macro_rules! sizes {
    ($($(#[$doc:meta])* $vis:vis $name:ident = $value:expr;)*) => {
        $($(#[$doc])* $vis const $name: f32 = $value;)*
        const SIZES: &[(&str, f32)] = &[$((stringify!($name), $name)),*];
    };
}

colors! {
    // Ground behind the map, and the glass of the panels. The panels are
    // tinted toward blue so that they read as one family on any terrain.
    pub VOID = Rgba::from_rgb(6, 8, 12);
    pub GLASS = Rgba::from_rgba_premultiplied(9, 13, 20, 224);
    pub GLASS_EDGE = Rgba::from_rgba_premultiplied(44, 52, 66, 140);
    pub BUTTON = Rgba::from_rgba_premultiplied(40, 48, 62, 230);
    pub BUTTON_HOVER = Rgba::from_rgba_premultiplied(62, 74, 94, 240);
    pub TRACK = Rgba::from_rgba_premultiplied(2, 3, 5, 190);
    /// The fill of a choice the player picked: a dark tint of the goal color.
    pub CHOSEN = Rgba::from_rgba_premultiplied(16, 56, 62, 240);
    /// The near-black glass of the journal in dark mode.
    pub DARK_GLASS = Rgba::from_rgba_premultiplied(2, 2, 4, 240);
    /// The shadow under a panel.
    pub PANEL_SHADOW = Rgba::from_rgba_premultiplied(0, 0, 0, 150);
    /// The dark edge round the map that keeps the panels readable.
    pub VIGNETTE = Rgba::from_rgba_premultiplied(0, 0, 0, 140);

    pub TEXT = Rgba::from_rgb(236, 240, 247);
    pub TEXT_DIM = Rgba::from_rgb(158, 170, 190);
    pub TEXT_FAINT = Rgba::from_rgb(112, 124, 146);
    pub TEXT_SHADOW = Rgba::from_rgba_premultiplied(0, 0, 0, 215);

    /// The one alarm color. Nothing else in the window is this red.
    pub ALARM = Rgba::from_rgb(255, 64, 56);
    pub GOAL = Rgba::from_rgb(92, 224, 236);
    pub WAITING = Rgba::from_rgb(255, 196, 84);

    pub HITS = Rgba::from_rgb(222, 58, 64);
    pub HITS_POISONED = Rgba::from_rgb(84, 196, 92);
    pub MANA = Rgba::from_rgb(66, 132, 246);
    pub STAM = Rgba::from_rgb(236, 186, 60);
    /// The part of a bar that was lost a moment ago.
    pub BAR_GHOST = Rgba::from_rgb(250, 238, 222);

    pub NOTO_SELF = Rgba::from_rgb(255, 214, 92);
    /// The figure of the character on the map. No notoriety has this color.
    pub SELF_FIGURE = Rgba::from_rgb(250, 250, 252);
    pub PLATE_BACK = Rgba::from_rgba_premultiplied(4, 6, 10, 165);
    NOTO_INNOCENT = Rgba::from_rgb(92, 164, 255);
    NOTO_FRIEND = Rgba::from_rgb(84, 212, 120);
    NOTO_GREY = Rgba::from_rgb(178, 184, 196);
    NOTO_ENEMY = Rgba::from_rgb(255, 150, 58);
    // A rose red, so that it is not the alarm red.
    NOTO_MURDERER = Rgba::from_rgb(244, 72, 128);
    NOTO_INVULNERABLE = Rgba::from_rgb(246, 226, 110);

    // The map drawn without client files.
    pub FLAT_WALK = Rgba::from_rgb(38, 62, 50);
    pub FLAT_WALK_ALT = Rgba::from_rgb(35, 57, 46);
    /// A tile past the edge of the radar. Nothing is known about it.
    pub FLAT_UNKNOWN = Rgba::from_rgb(20, 28, 30);
    pub FLAT_WATER = Rgba::from_rgb(24, 58, 98);
    pub FLAT_BLOCK_TOP = Rgba::from_rgb(86, 92, 106);
    pub FLAT_BLOCK_LEFT = Rgba::from_rgb(58, 63, 75);
    pub FLAT_BLOCK_RIGHT = Rgba::from_rgb(42, 46, 56);
    pub FLAT_DOOR = Rgba::from_rgb(176, 124, 62);
    pub FLAT_ITEM = Rgba::from_rgb(222, 178, 86);
    pub CORPSE = Rgba::from_rgb(214, 204, 184);
}

sizes! {
    pub PANEL_RADIUS = 10.0;
    /// How far the shadow of a panel falls below it, and how soft it is.
    /// Its color is `PANEL_SHADOW`.
    pub PANEL_SHADOW_DROP = 6.0;
    pub PANEL_SHADOW_BLUR = 22.0;
    /// How deep the dark edge round the map reaches, and the alarm color
    /// over it.
    pub VIGNETTE_DEPTH = 170.0;
    pub ALARM_DEPTH = 120.0;
    pub PANEL_PAD = 14.0;
    pub SCREEN_MARGIN = 16.0;
    pub ROW_GAP = 6.0;
    /// The width of the light edge of a panel.
    pub EDGE_WIDTH = 1.0;
    pub BAR_HEIGHT = 12.0;
    /// The hits bar is the one that tells of danger, so it is the largest.
    pub BAR_HEIGHT_MAIN = 18.0;
    pub BAR_RADIUS = 3.0;
    pub PIP_HEIGHT = 4.0;
    /// A picture stands this far inside its cell.
    pub CELL_ART_PAD = 4.0;

    pub SIZE_TITLE = 22.0;
    /// The heading of one part of a screen.
    pub SIZE_HEADING = 18.0;
    pub SIZE_BODY = 14.0;
    pub SIZE_SMALL = 12.0;
    pub SIZE_PLATE = 15.0;
    /// The numbers of damage over heads.
    pub SIZE_DAMAGE = 20.0;
}

/// Small pictures grow to this, so a coin is not a dot. More would blur.
const ART_MAX_SCALE: f32 = 2.0;

const NOTORIETY_INNOCENT: u8 = 1;
const NOTORIETY_FRIEND: u8 = 2;
const NOTORIETY_ENEMY: u8 = 5;
const NOTORIETY_MURDERER: u8 = 6;
const NOTORIETY_INVULNERABLE: u8 = 7;

/// A CSS alpha is a share of this.
const FULL_CHANNEL: f32 = u8::MAX as f32;
/// A CSS alpha keeps this many places after the point.
const ALPHA_ROUNDING: f32 = 1000.0;
const NAME_SPLIT: char = '_';
const CSS_NAME_SPLIT: &str = "-";

/// Grey and criminal share one grey, as in the game.
pub fn notoriety_color(notoriety: u8) -> Rgba {
    match notoriety {
        NOTORIETY_INNOCENT => NOTO_INNOCENT,
        NOTORIETY_FRIEND => NOTO_FRIEND,
        NOTORIETY_ENEMY => NOTO_ENEMY,
        NOTORIETY_MURDERER => NOTO_MURDERER,
        NOTORIETY_INVULNERABLE => NOTO_INVULNERABLE,
        _ => NOTO_GREY,
    }
}

/// A picture scaled to fit a cell, with its proportions kept.
pub fn fit(cell: Area, width: f32, height: f32) -> Area {
    fit_up_to(cell, width, height, ART_MAX_SCALE)
}

/// A picture that shrinks to fit a cell, and grows no more than
/// `max_scale`.
pub fn fit_up_to(cell: Area, width: f32, height: f32, max_scale: f32) -> Area {
    let room = cell.expand(-CELL_ART_PAD);
    let scale = (room.width() / width)
        .min(room.height() / height)
        .min(max_scale);
    Area::from_center_size(room.center(), Vector::new(width, height) * scale)
}

/// The CSS name of a constant: `GLASS_EDGE` is `--glass-edge`.
fn css_name(name: &str) -> String {
    let words: Vec<String> = name.split(NAME_SPLIT).map(str::to_lowercase).collect();
    format!("--{}", words.join(CSS_NAME_SPLIT))
}

/// A color as CSS writes it: plain channels, and the alpha as a share.
fn css_rgba(color: Rgba) -> String {
    let [r, g, b, a] = color.to_array();
    let plain = |channel: u8| {
        if a == 0 {
            0
        } else {
            ((f32::from(channel) * FULL_CHANNEL / f32::from(a)).round() as u32)
                .min(u32::from(u8::MAX))
        }
    };
    let alpha = (f32::from(a) / FULL_CHANNEL * ALPHA_ROUNDING).round() / ALPHA_ROUNDING;
    format!("rgba({}, {}, {}, {alpha})", plain(r), plain(g), plain(b))
}

/// A color as the page writes it: the CSS token of the theme color it is,
/// `var(--hits)`, or else its plain CSS value, as for a hue of the shard.
pub fn css_color(color: Rgba) -> String {
    COLORS
        .iter()
        .find(|(_, theme)| *theme == color)
        .map_or_else(
            || css_rgba(color),
            |(name, _)| format!("var({})", css_name(name)),
        )
}

/// Every color and size of the theme as CSS custom properties, one to a
/// line: `--hits: rgba(222, 58, 64, 1);` and `--panel-pad: 14px;`.
pub fn css_tokens() -> String {
    let colors = COLORS
        .iter()
        .map(|(name, color)| format!("{}: {};\n", css_name(name), css_rgba(*color)));
    let sizes = SIZES
        .iter()
        .map(|(name, size)| format!("{}: {size}px;\n", css_name(name)));
    colors.chain(sizes).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    #[test]
    fn a_large_picture_shrinks_to_the_cell_and_a_small_one_grows_a_little() {
        const CELL: f32 = 46.0;
        let cell = Area::from_min_size(Point::default(), Vector::new(CELL, CELL));
        let large = fit(cell, 100.0, 50.0);
        assert!(large.width() <= CELL && (large.width() / large.height() - 2.0).abs() < 0.01);
        let small = fit(cell, 10.0, 10.0);
        assert_eq!(
            small.size(),
            Vector::new(10.0 * ART_MAX_SCALE, 10.0 * ART_MAX_SCALE)
        );
    }

    #[test]
    fn a_see_through_color_takes_its_plain_channels_and_a_share() {
        assert_eq!(css_rgba(GLASS), "rgba(10, 15, 23, 0.878)");
        assert_eq!(css_rgba(Rgba::default()), "rgba(0, 0, 0, 0)");
    }

    #[test]
    fn a_theme_color_goes_to_the_page_as_its_token() {
        assert_eq!(css_color(HITS), "var(--hits)");
        assert_eq!(css_color(GLASS_EDGE), "var(--glass-edge)");
        assert_eq!(css_color(Rgba::from_rgb(1, 2, 3)), "rgba(1, 2, 3, 1)");
    }

    #[test]
    fn sizes_are_points_and_a_grey_notoriety_is_the_default() {
        assert!(css_tokens().contains("--panel-pad: 14px;"));
        assert!(css_tokens().contains("--panel-shadow-drop: 6px;"));
        assert_eq!(notoriety_color(0), notoriety_color(3));
        assert_ne!(notoriety_color(NOTORIETY_MURDERER), ALARM);
    }
}

#[cfg(test)]
mod css_tests {
    use super::*;

    #[test]
    fn css_tokens_carry_every_color_once() {
        let css = css_tokens();
        assert!(css.contains("--hits: rgba(222, 58, 64, 1)"));
        assert_eq!(css.matches("--void:").count(), 1);
    }
}
