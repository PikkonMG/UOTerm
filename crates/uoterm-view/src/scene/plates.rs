//! What floats over each thing: its name and its hit points. The Modern
//! style lays names on dark plates that keep clear of each other and of the
//! character; the Classic style lays the overheads of the classic client.
//! The window measures the words and paints what is laid out here.

use super::{SceneState, PERCENT_MAX};
use crate::art::TextLook;
use crate::geom::{Area, Point, Rgba, Vector};
use crate::look::{self, HitsShown, PlateOf};
use crate::settings::NameplateOptions;
use crate::ui::theme;

/// A Modern plate is at least this wide, for its bar of hit points.
const PLATE_BAR_WIDTH: f32 = 46.0;
const PLATE_GAP: f32 = 3.0;
const PLATE_PAD: f32 = 3.0;
/// The dark edge round the bar of hit points.
const PLATE_BAR_EDGE: f32 = 1.0;
/// A plate that makes room moves this far over the one it would cover.
const PLATE_CLEARANCE: f32 = 1.0;
const PERCENT: f32 = 100.0;
/// The hit points line of the classic client, under the feet.
const HITS_BACK_GUMP: u16 = 0x1068;
const HITS_FILL_GUMP: u16 = 0x1069;
const HITS_LINE_SIZE: Vector = Vector::new(34.0, 8.0);
const HITS_LINE_DROP: f32 = 5.0;
const HITS_LOST_HUE: u16 = 0x0021;
const HITS_FILL_HUE: u16 = 0x005A;
const HITS_POISON_HUE: u16 = 0x003F;
const HITS_YELLOW_HUE: u16 = 0x0035;
/// A line of a mobile the character does not fight is this faint.
const HITS_PASSIVE_ALPHA: f32 = 0.5;
/// The hit points in words over a head, in an ASCII font.
const HITS_FONT: u8 = 3;
const HITS_RISE: f32 = 8.0;
const NAMEPLATE_FONT: u8 = 1;
const NAMEPLATE_PAD: f32 = 2.0;

/// What floats over one thing: its name and its hit points.
#[derive(Clone, Debug, PartialEq)]
pub struct Plate {
    /// The middle of the top of the thing.
    pub top: Point,
    pub foot: Point,
    pub name: String,
    /// The color of the Modern style.
    pub color: Rgba,
    /// The notoriety hue of the Classic style.
    pub hue: u16,
    pub of: PlateOf,
    pub hits_percent: Option<u8>,
    pub target: bool,
    pub hits: HitsShown,
    /// The name plate shows.
    pub named: bool,
    pub poisoned: bool,
    pub yellow_hits: bool,
}

/// One Modern plate, laid out.
#[derive(Clone, Debug, PartialEq, ::serde::Serialize)]
pub struct PlacedPlate {
    /// The dark plate.
    pub area: Area,
    /// The middle of the top of the name.
    pub name_at: Point,
    pub name: String,
    pub name_color: Rgba,
    pub bar: Option<PlateBar>,
}

/// The bar of hit points on a Modern plate.
#[derive(Clone, Copy, Debug, PartialEq, ::serde::Serialize)]
pub struct PlateBar {
    /// The dark edge behind the bar.
    pub back: Area,
    /// The part of the bar the hit points fill.
    pub fill: Area,
    pub color: Rgba,
}

/// One part of the overheads of the Classic style, in paint order.
#[derive(Clone, Debug, PartialEq)]
pub enum Overhead {
    /// A gump picture in a hue, stretched over an area. With no gump art,
    /// a box in the color of the hue takes its place.
    Gump {
        gump: u16,
        hue: u16,
        area: Area,
        alpha: f32,
    },
    /// Words in a UO font, from their top left corner.
    Words {
        text: String,
        look: TextLook,
        at: Point,
    },
    /// A box of one color.
    Fill { area: Area, color: Rgba },
    /// A box in the color of a hue.
    HueFill { area: Area, hue: u16 },
}

impl SceneState {
    /// What floats over one thing, as the Nameplates page says.
    pub(super) fn plate(
        &self,
        of: PlateOf,
        name: &str,
        area: Area,
        foot: Point,
        hue: u16,
        hits_percent: Option<u8>,
    ) -> Plate {
        let full_health = hits_percent.is_none_or(|percent| percent >= PERCENT_MAX);
        Plate {
            top: area.center_top(),
            foot,
            name: name.to_string(),
            color: theme::TEXT,
            hue,
            of,
            hits_percent,
            target: false,
            hits: HitsShown::default(),
            named: look::plate_shows(&self.look.nameplates, of, self.ctrl_shift, full_health)
                && !name.is_empty(),
            poisoned: false,
            yellow_hits: false,
        }
    }
}

/// Moves `area` up until it covers none of `taken`.
fn clear_of(taken: &[Area], mut area: Area) -> Area {
    while let Some(hit) = taken.iter().find(|r| r.intersects(area)) {
        area = area.translate(Vector::new(0.0, hit.min.y - area.max.y - PLATE_CLEARANCE));
    }
    area
}

/// Lays the names of the Modern style from the top of the window down. A
/// name that would lie on one already laid, or on `keep_clear`, moves up
/// until it is clear. `measure` gives the size of a name in the font of
/// the plates.
pub fn lay_out(
    mut plates: Vec<Plate>,
    keep_clear: Area,
    zoom: f32,
    measure: &dyn Fn(&str) -> Vector,
) -> Vec<PlacedPlate> {
    plates.retain(|plate| plate.of == PlateOf::Mobile);
    plates.sort_by(|a, b| a.top.y.total_cmp(&b.top.y));
    let mut taken = Vec::with_capacity(plates.len() + 1);
    taken.push(keep_clear);
    let mut placed = Vec::with_capacity(plates.len());
    for plate in plates {
        let text = measure(&plate.name);
        let bar_height = if plate.hits_percent.is_some() {
            theme::PIP_HEIGHT + PLATE_GAP
        } else {
            0.0
        };
        let size = Vector::new(text.x.max(PLATE_BAR_WIDTH), text.y + bar_height)
            + Vector::splat(PLATE_PAD * 2.0);
        let wanted = Area::from_center_size(
            plate.top - Vector::new(0.0, size.y / 2.0 + PLATE_GAP * zoom),
            size,
        );
        let area = clear_of(&taken, wanted);
        taken.push(area);
        let bar = plate.hits_percent.map(|percent| {
            let track = Area::from_min_size(
                Point::new(
                    area.center().x - PLATE_BAR_WIDTH / 2.0,
                    area.max.y - PLATE_PAD - theme::PIP_HEIGHT,
                ),
                Vector::new(PLATE_BAR_WIDTH, theme::PIP_HEIGHT),
            );
            let mut fill = track;
            fill.max.x = fill.min.x + PLATE_BAR_WIDTH * f32::from(percent) / PERCENT;
            PlateBar {
                back: track.expand(PLATE_BAR_EDGE),
                fill,
                color: plate.color,
            }
        });
        placed.push(PlacedPlate {
            area,
            name_at: Point::new(area.center().x, area.min.y + PLATE_PAD),
            name_color: if plate.target {
                theme::ALARM
            } else {
                plate.color
            },
            name: plate.name,
            bar,
        });
    }
    placed
}

/// Lays the hit points and the name plates of the Classic style over each
/// thing that has them, as the Nameplates page says. `measure` gives the
/// size of words in a UO font.
pub fn overheads(
    mut plates: Vec<Plate>,
    options: &NameplateOptions,
    zoom: f32,
    measure: &mut dyn FnMut(&str, TextLook) -> Vector,
) -> Vec<Overhead> {
    // From the bottom of the window up, so a plate that must make room
    // moves up and away from the ones under it.
    plates.sort_by(|a, b| b.top.y.total_cmp(&a.top.y));
    let mut taken = Vec::new();
    let mut laid = Vec::new();
    for plate in plates {
        if plate.hits.line {
            hits_line(&plate, zoom, &mut laid);
        }
        let mut top = plate.top - Vector::new(0.0, HITS_RISE * zoom);
        if let (true, Some(percent)) = (plate.hits.percent, plate.hits_percent) {
            let text = format!("[{percent}%]");
            let look = TextLook::ascii(HITS_FONT, look::hits_hue(percent));
            let size = measure(&text, look);
            top.y -= size.y;
            let at = top - Vector::new(size.x / 2.0, 0.0);
            laid.push(Overhead::Words { text, look, at });
        }
        if plate.named {
            nameplate(plate, options, top, &mut taken, measure, &mut laid);
        }
    }
    laid
}

/// The name of a thing on a dark plate, as the Nameplates page says: its
/// opacity, a line of its hit points, and plates that keep apart.
fn nameplate(
    plate: Plate,
    options: &NameplateOptions,
    top: Point,
    taken: &mut Vec<Area>,
    measure: &mut dyn FnMut(&str, TextLook) -> Vector,
    laid: &mut Vec<Overhead>,
) {
    let look = TextLook::unicode(NAMEPLATE_FONT, plate.hue).bordered();
    let words = measure(&plate.name, look);
    let bar = plate
        .hits_percent
        .filter(|_| options.health_bar && plate.of == PlateOf::Mobile);
    let bar_room = if bar.is_some() {
        theme::PIP_HEIGHT + NAMEPLATE_PAD
    } else {
        0.0
    };
    let size = words + Vector::new(NAMEPLATE_PAD * 2.0, NAMEPLATE_PAD * 2.0 + bar_room);
    let mut area = Area::from_min_size(top - Vector::new(size.x / 2.0, size.y), size);
    if options.avoid_overlap {
        area = clear_of(taken, area);
        taken.push(area);
    }
    let opacity = f32::from(options.opacity) / PERCENT;
    laid.push(Overhead::Fill {
        area,
        color: Rgba::from_black_alpha((opacity * f32::from(u8::MAX)) as u8),
    });
    laid.push(Overhead::Words {
        text: plate.name,
        look,
        at: area.min + Vector::splat(NAMEPLATE_PAD),
    });
    if let Some(percent) = bar {
        let track = Area {
            min: Point::new(
                area.min.x + NAMEPLATE_PAD,
                area.max.y - NAMEPLATE_PAD - theme::PIP_HEIGHT,
            ),
            max: Point::new(area.max.x - NAMEPLATE_PAD, area.max.y - NAMEPLATE_PAD),
        };
        laid.push(Overhead::HueFill {
            area: track,
            hue: HITS_LOST_HUE,
        });
        let mut fill = track;
        fill.max.x = fill.min.x + track.width() * f32::from(percent.min(PERCENT_MAX)) / PERCENT;
        laid.push(Overhead::HueFill {
            area: fill,
            hue: plate.hue,
        });
    }
}

/// The line of hit points under the feet, from the gump art of the
/// classic client: the notoriety hue behind, the lost part in red, and
/// the rest in blue, green for poison or yellow for the blessed.
fn hits_line(plate: &Plate, zoom: f32, laid: &mut Vec<Overhead>) {
    let Some(percent) = plate.hits_percent else {
        return;
    };
    let area = Area::from_center_size(
        plate.foot + Vector::new(0.0, HITS_LINE_DROP * zoom),
        HITS_LINE_SIZE,
    );
    let alpha = if plate.target {
        1.0
    } else {
        HITS_PASSIVE_ALPHA
    };
    let left = HITS_LINE_SIZE.x * f32::from(percent.min(PERCENT_MAX)) / PERCENT;
    let fill_hue = if plate.poisoned {
        HITS_POISON_HUE
    } else if plate.yellow_hits {
        HITS_YELLOW_HUE
    } else {
        HITS_FILL_HUE
    };
    let parts = [
        (HITS_BACK_GUMP, plate.hue, area),
        (
            HITS_FILL_GUMP,
            HITS_LOST_HUE,
            Area {
                min: Point::new(area.min.x + left, area.min.y),
                max: area.max,
            },
        ),
        (
            HITS_FILL_GUMP,
            fill_hue,
            Area::from_min_size(area.min, Vector::new(left, area.height())),
        ),
    ];
    for (gump, hue, part) in parts {
        if part.width() <= 0.0 {
            continue;
        }
        laid.push(Overhead::Gump {
            gump,
            hue,
            area: part,
            alpha,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name is this big in these tests.
    const NAME: Vector = Vector::new(30.0, 12.0);
    const ZOOM: f32 = 1.0;

    fn plate(top: Point, hits: Option<u8>) -> Plate {
        Plate {
            top,
            foot: top + Vector::new(0.0, 60.0),
            name: "Orc".into(),
            color: theme::TEXT,
            hue: 0x0026,
            of: PlateOf::Mobile,
            hits_percent: hits,
            target: false,
            hits: HitsShown::default(),
            named: true,
            poisoned: false,
            yellow_hits: false,
        }
    }

    #[test]
    fn a_plate_moves_up_off_the_character_and_off_the_plate_above() {
        let keep_clear = Area::from_center_size(Point::new(100.0, 100.0), Vector::new(30.0, 60.0));
        let plates = vec![
            plate(Point::new(100.0, 100.0), Some(50)),
            plate(Point::new(100.0, 101.0), None),
        ];
        let placed = lay_out(plates, keep_clear, ZOOM, &|_| NAME);
        assert_eq!(placed.len(), 2);
        assert!(placed[0].area.max.y < keep_clear.min.y);
        assert!(placed[1].area.max.y < placed[0].area.min.y);
        let bar = placed[0].bar.unwrap();
        assert_eq!(bar.fill.width(), PLATE_BAR_WIDTH / 2.0);
        assert!(placed[1].bar.is_none());
    }

    #[test]
    fn an_item_has_no_modern_plate() {
        let mut item = plate(Point::new(0.0, 0.0), None);
        item.of = PlateOf::Item;
        assert!(lay_out(vec![item], Area::default(), ZOOM, &|_| NAME).is_empty());
    }

    #[test]
    fn a_classic_overhead_shows_the_hits_line_the_percent_and_the_name() {
        let mut shown = plate(Point::new(50.0, 50.0), Some(40));
        shown.hits = HitsShown {
            line: true,
            percent: true,
        };
        let options = NameplateOptions {
            health_bar: true,
            ..NameplateOptions::default()
        };
        let laid = overheads(vec![shown], &options, ZOOM, &mut |_, _| NAME);
        let gumps = laid
            .iter()
            .filter(|part| matches!(part, Overhead::Gump { .. }))
            .count();
        assert_eq!(gumps, 3, "the back, the lost part and the rest");
        let words: Vec<&str> = laid
            .iter()
            .filter_map(|part| match part {
                Overhead::Words { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, ["[40%]", "Orc"]);
        let hue_fills = laid
            .iter()
            .filter(|part| matches!(part, Overhead::HueFill { .. }))
            .count();
        assert_eq!(hue_fills, 2, "the bar track and its fill");
    }
}
