//! Two windows about people and places: the map item the character opened,
//! with its pins, and the profile a player wrote about a character.
//!
//! A map item: a treasure map with its pins, or a
//! city map. The picture is the land of the map, drawn from the radar
//! colors of the client files. A click puts a pin where the human clicked,
//! when the shard lets the map be drawn on.

use super::boxes_ui::{Tools, CELL_RADIUS};
use super::control::{Act, Answer, Ask};
use super::scene::Scene;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchFrame, WatchMap};
use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, Id, Pos2, Rect, Sense, TextureHandle,
    TextureOptions, Vec2,
};

/// The largest picture the window draws for a map item. A larger map is
/// drawn into this and shown at its own size.
const PICTURE_MAX: usize = 400;
const TITLE_ROW: f32 = 32.0;
const FOOT_ROW: f32 = 40.0;
const PIN_RADIUS: f32 = 4.0;
const PIN_RING: f32 = 1.5;
const UNKNOWN: Color32 = Color32::from_rgb(28, 30, 34);
const PAPER_EDGE: f32 = 2.0;

const WORDS_TITLE: &str = "Map";
const WORDS_CLEAR: &str = "Clear pins";
const WORDS_EDIT: &str = "Let me draw";
const WORDS_CLOSE: &str = "Close";
const WORDS_NO_FILES: &str = "The picture needs the client files.";
const HINT_PIN: &str = "Click: put a pin here.";
const WORDS_MARK: &str = "Mark";
const HINT_WISH: &str = "Say the place in plain words, for example: Britain bank";
const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
const WORDS_ASKING: &str = "Jev looks for the place...";
const MARK_WIDTH: f32 = 70.0;
const FIELD_ROW: f32 = 30.0;
const GAP: f32 = 10.0;

struct Picture {
    serial: u32,
    texture: TextureHandle,
}

#[derive(Default)]
pub struct MapItemUi {
    picture: Option<Picture>,
    wish: String,
    /// Words for the human about the last thing Jev did.
    note: Option<(String, bool)>,
}

/// The pixel of a map picture that a tile of the world lies on.
fn pixel_of(map: &WatchMap, x: u16, y: u16) -> (u16, u16) {
    let along = |start: u16, end: u16, of: u16, at: u16| {
        let span = u32::from(end.saturating_sub(start)).max(1);
        let from_start = u32::from(at.saturating_sub(start));
        (from_start * u32::from(of) / span) as u16
    };
    (
        along(map.start_x, map.end_x, map.width, x),
        along(map.start_y, map.end_y, map.height, y),
    )
}

/// How large the picture is in pixels, and how many tiles each side covers.
fn picture_size(map: &WatchMap) -> (usize, usize) {
    let across = usize::from(map.end_x.saturating_sub(map.start_x)).max(1);
    let down = usize::from(map.end_y.saturating_sub(map.start_y)).max(1);
    (across.min(PICTURE_MAX), down.min(PICTURE_MAX))
}

/// The tile of the world at one pixel of the picture.
fn tile_of(map: &WatchMap, pixels: (usize, usize), pixel: (usize, usize)) -> (u16, u16) {
    let span = |start: u16, end: u16, pixels: usize, at: usize| {
        let span = u32::from(end.saturating_sub(start));
        start.saturating_add((span * at as u32 / pixels.max(1) as u32) as u16)
    };
    (
        span(map.start_x, map.end_x, pixels.0, pixel.0),
        span(map.start_y, map.end_y, pixels.1, pixel.1),
    )
}

impl MapItemUi {
    /// The picture of the land of a map, made once for each map.
    fn picture_of(&mut self, ui: &egui::Ui, map: &WatchMap, scene: &mut Scene) -> bool {
        if self
            .picture
            .as_ref()
            .is_some_and(|kept| kept.serial == map.serial)
        {
            return true;
        }
        let (across, down) = picture_size(map);
        let mut image = ColorImage::new([across, down], UNKNOWN);
        let mut any = false;
        for row in 0..down {
            for column in 0..across {
                let (x, y) = tile_of(map, (across, down), (column, row));
                if let Some([r, g, b]) = scene.radar_rgb(map.facet, x, y) {
                    image.pixels[row * across + column] = Color32::from_rgb(r, g, b);
                    any = true;
                }
            }
        }
        if !any {
            self.picture = None;
            return false;
        }
        self.picture = Some(Picture {
            serial: map.serial,
            texture: ui
                .ctx()
                .load_texture("map-item", image, TextureOptions::LINEAR),
        });
        true
    }

    /// Draws the map item that opened last. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Option<Rect> {
        let Some(map) = frame.maps.last() else {
            self.picture = None;
            return None;
        };
        for answer in tools.hand.new_answers() {
            match answer {
                Answer::Place(Ok((x, y))) => {
                    let (x, y) = pixel_of(map, x, y);
                    tools.hand.act(Act::MapPin { x, y });
                    self.wish.clear();
                    self.note = None;
                }
                Answer::Place(Err(words)) => self.note = Some((words, true)),
                // The macro editor, the designer and the chat take the rest.
                _ => {}
            }
        }
        let paper = Vec2::new(f32::from(map.width.max(1)), f32::from(map.height.max(1)));
        let panel = Rect::from_center_size(
            rect.center(),
            paper
                + Vec2::new(
                    theme::PANEL_PAD * 2.0,
                    TITLE_ROW + FOOT_ROW + theme::PANEL_PAD,
                ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        // The rows are laid from the bottom up, so every one of them stays
        // inside the panel whatever size the map has.
        let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
        let wish_row = Rect::from_min_size(
            Pos2::new(inner.left(), foot.y - GAP - FIELD_ROW),
            Vec2::new(inner.width(), FIELD_ROW),
        );
        let picture = Rect::from_min_max(
            inner.left_top() + Vec2::new(0.0, TITLE_ROW),
            Pos2::new(inner.right(), wish_row.top() - GAP),
        );
        ui.painter()
            .rect_filled(picture, CornerRadius::same(CELL_RADIUS), UNKNOWN);
        if self.picture_of(ui, map, tools.scene) {
            if let Some(kept) = &self.picture {
                ui.painter().image(
                    kept.texture.id(),
                    picture.shrink(PAPER_EDGE),
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        } else {
            ui.painter().text(
                picture.center(),
                Align2::CENTER_CENTER,
                WORDS_NO_FILES,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
        }
        for (i, (x, y)) in map.pins.iter().enumerate() {
            let at = picture.left_top()
                + Vec2::new(
                    f32::from(*x) * picture.width() / f32::from(map.width.max(1)),
                    f32::from(*y) * picture.height() / f32::from(map.height.max(1)),
                );
            ui.painter().circle_filled(at, PIN_RADIUS, theme::ALARM);
            ui.painter()
                .circle_stroke(at, PIN_RADIUS, egui::Stroke::new(PIN_RING, theme::TEXT));
            ui.painter().text(
                at + Vec2::new(PIN_RADIUS * 2.0, 0.0),
                Align2::LEFT_CENTER,
                (i + 1).to_string(),
                number_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
        }
        if !frame.human_control {
            return Some(panel);
        }
        let response = ui.interact(picture, Id::new(("map-item", map.serial)), Sense::click());
        if map.may_plot && response.hovered() {
            super::tips::label(ui, HINT_PIN, "");
        }
        if let Some(at) = response.interact_pointer_pos().filter(|_| map.may_plot) {
            let on_paper = at - picture.left_top();
            let pixel = |along: f32, of: f32, width: u16| {
                (along / of.max(1.0) * f32::from(width)).round().max(0.0) as u16
            };
            tools.hand.act(Act::MapPin {
                x: pixel(on_paper.x, picture.width(), map.width),
                y: pixel(on_paper.y, picture.height(), map.height),
            });
        }
        self.ask_field(ui, wish_row, map, tools);
        let (clear, cleared) = theme::button(ui, foot, WORDS_CLEAR, theme::TEXT);
        let (edit, edited) = theme::button(
            ui,
            Pos2::new(clear.right() + theme::ROW_GAP, foot.y),
            WORDS_EDIT,
            if map.may_plot {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            },
        );
        let (_, closed) = theme::button(
            ui,
            Pos2::new(edit.right() + theme::ROW_GAP, foot.y),
            WORDS_CLOSE,
            theme::TEXT_DIM,
        );
        if let Some((words, failed)) = &self.note {
            let color = if *failed {
                theme::ALARM
            } else {
                theme::WAITING
            };
            ui.painter().text(
                Pos2::new(panel.center().x, panel.bottom() + theme::ROW_GAP),
                Align2::CENTER_TOP,
                words,
                text_font(theme::SIZE_SMALL),
                color,
            );
        }
        if cleared {
            tools.hand.act(Act::MapClear);
        } else if edited {
            tools.hand.act(Act::MapEdit);
        } else if closed {
            tools.hand.act(Act::MapClose(map.serial));
        }
        Some(panel)
    }
}

impl MapItemUi {
    /// The field that takes a place in plain words. Jev picks the place
    /// from the named places that lie on this map, and its tile becomes a
    /// pin. Without a TypeSafe key the field is off.
    fn ask_field(&mut self, ui: &mut egui::Ui, row: Rect, map: &WatchMap, tools: &Tools<'_>) {
        let on = tools.hand.orders_on && map.may_plot;
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - MARK_WIDTH - GAP, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let hint = if tools.hand.orders_on {
            HINT_WISH
        } else {
            HINT_WISH_OFF
        };
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.wish)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 6))
                .hint_text(hint)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let (_, mark) = theme::button(
            ui,
            Pos2::new(field.right() + GAP, row.top()),
            WORDS_MARK,
            if on { theme::GOAL } else { theme::TEXT_FAINT },
        );
        let asked = mark || (typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if asked && on && !self.wish.trim().is_empty() {
            tools.hand.ask(Ask::PlaceOnMap {
                wish: self.wish.trim().to_string(),
                map: map.facet,
                from: (map.start_x, map.start_y),
                to: (map.end_x, map.end_y),
            });
            self.note = Some((WORDS_ASKING.into(), false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn treasure_map() -> WatchMap {
        WatchMap {
            serial: 60,
            start_x: 1000,
            start_y: 1200,
            end_x: 1400,
            end_y: 1600,
            width: 200,
            height: 200,
            ..WatchMap::default()
        }
    }

    #[test]
    fn the_picture_covers_the_land_of_the_map() {
        let map = treasure_map();
        let (across, down) = picture_size(&map);
        assert_eq!((across, down), (400, 400));
        assert_eq!(tile_of(&map, (across, down), (0, 0)), (1000, 1200));
        assert_eq!(tile_of(&map, (across, down), (200, 200)), (1200, 1400));
        assert_eq!(tile_of(&map, (across, down), (399, 399)), (1399, 1599));
    }

    #[test]
    fn a_map_larger_than_the_picture_is_drawn_into_it() {
        let wide = WatchMap {
            end_x: 6000,
            end_y: 6000,
            ..treasure_map()
        };
        let (across, down) = picture_size(&wide);
        assert_eq!((across, down), (PICTURE_MAX, PICTURE_MAX));
        assert_eq!(tile_of(&wide, (across, down), (0, 0)), (1000, 1200));
        let (x, _) = tile_of(&wide, (across, down), (PICTURE_MAX / 2, 0));
        assert_eq!(x, 1000 + (6000 - 1000) / 2);
    }

    #[test]
    fn a_tile_of_the_world_finds_its_pixel_on_the_map() {
        let map = treasure_map();
        assert_eq!(pixel_of(&map, 1000, 1200), (0, 0));
        assert_eq!(pixel_of(&map, 1200, 1400), (100, 100));
        assert_eq!(pixel_of(&map, 1400, 1600), (200, 200));
        // A tile off the map lands on its edge.
        assert_eq!(pixel_of(&map, 900, 1100), (0, 0));
    }

    #[test]
    fn a_map_with_no_land_of_its_own_still_has_a_picture_size() {
        let empty = WatchMap {
            start_x: 500,
            end_x: 500,
            start_y: 500,
            end_y: 500,
            ..treasure_map()
        };
        assert_eq!(picture_size(&empty), (1, 1));
        assert_eq!(tile_of(&empty, (1, 1), (0, 0)), (500, 500));
    }
}

const PROFILE_WIDTH: f32 = 420.0;
const PROFILE_ROWS: usize = 8;
const PROFILE_LINE: f32 = 20.0;
const WORDS_WRITE: &str = "Write";
const WORDS_SAVE: &str = "Save";
const HINT_PROFILE: &str = "What your character says about himself";

/// The window that shows the profile of a character. Only the owner of a
/// character may change his profile, so the shard refuses the rest.
#[derive(Default)]
pub struct ProfileUi {
    /// The character whose profile shows, and the words being written.
    shown: Option<u32>,
    writing: Option<String>,
    /// Show the profile of the character with the first picture.
    show_own: bool,
}

impl ProfileUi {
    /// The window as it starts. An open one shows the profile of the
    /// character as soon as the first picture names him.
    pub fn starting(show_own: bool) -> Self {
        Self {
            show_own,
            ..Self::default()
        }
    }

    /// Asks the session for the profile of a character and shows it.
    pub fn show(&mut self, serial: u32, hand: &super::control::Hand) {
        self.shown = Some(serial);
        self.writing = None;
        hand.act(Act::ProfileRead(serial));
    }

    pub fn close(&mut self) {
        self.shown = None;
        self.writing = None;
    }

    pub fn shows(&self, serial: u32) -> bool {
        self.shown == Some(serial)
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Option<Rect> {
        if std::mem::take(&mut self.show_own) && frame.serial != 0 {
            self.show(frame.serial, tools.hand);
        }
        let serial = self.shown?;
        let known = frame.profiles.iter().find(|kept| kept.serial == serial);
        let body_rows = PROFILE_ROWS as f32 * PROFILE_LINE;
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                PROFILE_WIDTH,
                TITLE_ROW * 2.0 + body_rows + FOOT_ROW + theme::PANEL_PAD * 2.0,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let name = known.map_or("", |kept| kept.name.as_str());
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            if name.is_empty() { "Profile" } else { name },
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        ui.painter().text(
            inner.left_top() + Vec2::new(0.0, TITLE_ROW),
            Align2::LEFT_TOP,
            known.map_or("", |kept| kept.title.as_str()),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let body = Rect::from_min_size(
            inner.left_top() + Vec2::new(0.0, TITLE_ROW * 2.0),
            Vec2::new(inner.width(), body_rows),
        );
        match self.writing.as_mut() {
            Some(words) => {
                ui.painter()
                    .rect_filled(body, CornerRadius::same(CELL_RADIUS), theme::TRACK);
                ui.put(
                    body,
                    egui::TextEdit::multiline(words)
                        .frame(false)
                        .margin(egui::Margin::symmetric(8, 6))
                        .hint_text(HINT_PROFILE)
                        .font(text_font(theme::SIZE_BODY))
                        .text_color(theme::TEXT),
                );
            }
            None => {
                let shard = known.map_or("", |kept| kept.shard_words.as_str());
                let own = known.map_or("", |kept| kept.own_words.as_str());
                let words = if shard.is_empty() {
                    own.to_string()
                } else {
                    format!("{shard}\n\n{own}")
                };
                let mut job = egui::text::LayoutJob::single_section(
                    words,
                    egui::TextFormat::simple(text_font(theme::SIZE_BODY), theme::TEXT),
                );
                job.wrap.max_width = body.width();
                let galley = ui.painter().layout_job(job);
                ui.painter()
                    .with_clip_rect(body)
                    .galley(body.left_top(), galley, theme::TEXT);
            }
        }
        let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
        let mine = serial == frame.serial;
        let mut next = foot;
        if frame.human_control && mine {
            let words = if self.writing.is_some() {
                WORDS_SAVE
            } else {
                WORDS_WRITE
            };
            let (area, pressed) = theme::button(ui, foot, words, theme::GOAL);
            next = Pos2::new(area.right() + theme::ROW_GAP, foot.y);
            if pressed {
                match self.writing.take() {
                    Some(words) => tools.hand.act(Act::ProfileWrite {
                        serial,
                        text: words,
                    }),
                    None => {
                        self.writing =
                            Some(known.map_or_else(String::new, |kept| kept.own_words.clone()));
                    }
                }
            }
        }
        let (_, closed) = theme::button(ui, next, WORDS_CLOSE, theme::TEXT_DIM);
        if closed {
            self.shown = None;
            self.writing = None;
        }
        Some(panel)
    }
}
