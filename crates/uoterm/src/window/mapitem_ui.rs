//! Two windows about people and places: each map item the character
//! opened, with its pins, and the profile a player wrote about a character.
//! The player moves and locks each one.
//!
//! A map item: a treasure map with its pins, or a city map. The picture is
//! the land of the map, drawn from the radar colors of the client files,
//! with the course line from one pin to the next. When the shard lets the
//! map be drawn on, a click puts a pin where the human clicked, a pin drags
//! to a new place, and a double click takes it off.

use super::boxes_ui::{Tools, CELL_RADIUS};
use super::control::{Act, Answer, Ask, Asker, Hand};
use super::model::map_item::{pixel_at, pixel_of, point_of, LandPicture, UNKNOWN};
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::layout::{self, Spot};
use super::settings::Profile;
use super::theme::{self, number_font, text_font};
use crate::view::{WatchFrame, WatchMap};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;

const MAP_PLACE_ID: &str = "modern:map_item:";
const PROFILE_ID: &str = "modern:profile";
const TITLE_ROW: f32 = 32.0;
const FOOT_ROW: f32 = 40.0;
const PIN_RADIUS: f32 = 4.0;
/// A pin takes clicks this far round it.
const PIN_REACH: f32 = 8.0;
const PIN_RING: f32 = 1.5;
const COURSE_WIDTH: f32 = 1.5;
const PAPER_EDGE: f32 = 2.0;

const WORDS_TITLE: &str = "Map";
const WORDS_CLEAR: &str = "Clear pins";
const WORDS_EDIT: &str = "Plot course";
const WORDS_STOP: &str = "Stop plotting";
const WORDS_CLOSE: &str = "Close";
const WORDS_NO_FILES: &str = "The picture needs the client files.";
const HINT_PIN: &str = "Click: put a pin here.";
const HINT_PIN_MOVE: &str = "Drag: move the pin.  Double-click: take it off.";
const WORDS_MARK: &str = "Mark";
const HINT_WISH: &str = "Say the place in plain words, for example: Britain bank";
const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
const WORDS_ASKING: &str = "Jev looks for the place...";
const MARK_WIDTH: f32 = 70.0;
const FIELD_ROW: f32 = 30.0;
const GAP: f32 = 10.0;

/// A pin the player drags: its map, its place in the list, and where it
/// is now.
struct DraggedPin {
    map: u32,
    pin: usize,
    at: Pos2,
}

#[derive(Default)]
pub struct MapItemUi {
    /// The picture of the land of each open map.
    pictures: HashMap<u32, LandPicture>,
    /// The place each map is asked for in plain words.
    wishes: HashMap<u32, String>,
    /// The map Jev looks for a place on.
    asked_for: Option<u32>,
    /// Words for the human about the last thing Jev did.
    note: Option<(String, bool)>,
    dragging: Option<DraggedPin>,
}

/// What the player did to a pin in one frame.
enum PinDeed {
    Moved(usize, (u16, u16)),
    Removed(usize),
}

impl MapItemUi {
    /// Draws each open map item. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        let open = |serial: &u32| frame.maps.iter().any(|map| map.serial == *serial);
        self.pictures.retain(|serial, _| open(serial));
        self.wishes.retain(|serial, _| open(serial));
        if self
            .dragging
            .as_ref()
            .is_some_and(|drag| !frame.maps.iter().any(|map| map.serial == drag.map))
        {
            self.dragging = None;
        }
        self.take_answers(frame, tools.hand);
        frame
            .maps
            .iter()
            .enumerate()
            .map(|(index, map)| self.map(ui, rect, index, map, frame, tools, profile))
            .collect()
    }

    /// Takes the place Jev found: its tile becomes a pin of the map it was
    /// asked for.
    fn take_answers(&mut self, frame: &WatchFrame, hand: &Hand) {
        for answer in hand.new_answers(Asker::MapItem) {
            let map = self
                .asked_for
                .and_then(|serial| frame.maps.iter().find(|map| map.serial == serial));
            match (answer, map) {
                (Answer::Place(Ok((x, y))), Some(map)) => {
                    let (x, y) = pixel_of(map, x, y);
                    hand.act(Act::MapPin { x, y });
                    self.wishes.remove(&map.serial);
                    self.note = None;
                }
                (Answer::Place(Err(words)), _) => self.note = Some((words, true)),
                // The map item asks only for a place, and its map is gone.
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn map(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        index: usize,
        map: &WatchMap,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let live = frame.human_control;
        let paper = Vec2::new(f32::from(map.width.max(1)), f32::from(map.height.max(1)));
        let size = paper
            + Vec2::new(
                theme::PANEL_PAD * 2.0,
                frame::TITLE_ROW + FIELD_ROW + GAP * 2.0 + FOOT_ROW + theme::PANEL_PAD * 2.0,
            );
        let id = format!("{MAP_PLACE_ID}{}", index + 1);
        let spec = PanelSpec {
            id: &id,
            title: WORDS_TITLE,
            default: layout::first_place(rect, Spot::Middle(index), size),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
        // The rows are laid from the bottom up, so every one of them stays
        // inside the panel whatever size the map has.
        let foot = Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP);
        let wish_row = Rect::from_min_size(
            Pos2::new(body.left(), foot.y - GAP - FIELD_ROW),
            Vec2::new(body.width(), FIELD_ROW),
        );
        let picture = Rect::from_min_max(body.min, Pos2::new(body.right(), wish_row.top() - GAP));
        ui.painter()
            .rect_filled(picture, CornerRadius::same(CELL_RADIUS), UNKNOWN);
        let land = picture.shrink(PAPER_EDGE);
        let scene = &mut *tools.scene;
        let texture =
            self.pictures
                .entry(map.serial)
                .or_default()
                .texture(ui.ctx(), map, |facet, x, y| scene.radar_rgb(facet, x, y));
        match texture {
            Some(texture) => {
                ui.painter().image(
                    texture,
                    land,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                ui.painter().text(
                    picture.center(),
                    Align2::CENTER_CENTER,
                    WORDS_NO_FILES,
                    text_font(theme::SIZE_BODY),
                    theme::TEXT_FAINT,
                );
            }
        }
        let plotting = live && map.may_plot;
        if plotting {
            let response = ui.interact(land, Id::new(("map-item", map.serial)), Sense::click());
            if response.hovered() {
                super::tips::label(ui, HINT_PIN, "");
            }
            if let Some(at) = response
                .interact_pointer_pos()
                .filter(|_| response.clicked())
            {
                let (x, y) = pixel_at(map, land, at);
                tools.hand.act(Act::MapPin { x, y });
            }
        } else {
            self.dragging = None;
        }
        match self.pins(ui, land, map, plotting) {
            Some(PinDeed::Moved(pin, (x, y))) => tools.hand.act(Act::MapPinMove {
                pin: u8::try_from(pin).unwrap_or(u8::MAX),
                x,
                y,
            }),
            Some(PinDeed::Removed(pin)) => {
                tools
                    .hand
                    .act(Act::MapPinRemove(u8::try_from(pin).unwrap_or(u8::MAX)));
            }
            None => {}
        }
        if live {
            self.ask_field(ui, wish_row, map, tools);
            self.buttons(ui, foot, map, tools);
        }
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
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) && live {
            tools.hand.act(Act::MapClose(map.serial));
        }
        panel
    }

    /// The pins, the course line from one to the next, and their numbers.
    /// While the player plots, a pin drags and a double click takes it
    /// off. Gives what he did to one of them.
    fn pins(
        &mut self,
        ui: &egui::Ui,
        land: Rect,
        map: &WatchMap,
        plotting: bool,
    ) -> Option<PinDeed> {
        let spots: Vec<Pos2> = map
            .pins
            .iter()
            .enumerate()
            .map(|(at, pixel)| match &self.dragging {
                Some(drag) if drag.map == map.serial && drag.pin == at => drag.at,
                _ => point_of(map, land, *pixel),
            })
            .collect();
        for pair in spots.windows(2) {
            ui.painter()
                .line_segment([pair[0], pair[1]], Stroke::new(COURSE_WIDTH, theme::TEXT));
        }
        let mut deed = None;
        for (at, spot) in spots.into_iter().enumerate() {
            ui.painter().circle_filled(spot, PIN_RADIUS, theme::ALARM);
            ui.painter()
                .circle_stroke(spot, PIN_RADIUS, Stroke::new(PIN_RING, theme::TEXT));
            ui.painter().text(
                spot + Vec2::new(PIN_RADIUS * 2.0, 0.0),
                Align2::LEFT_CENTER,
                (at + 1).to_string(),
                number_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
            if !plotting {
                continue;
            }
            let area = Rect::from_center_size(spot, Vec2::splat(PIN_REACH * 2.0));
            let response = ui.interact(
                area,
                Id::new(("map-pin", map.serial, at)),
                Sense::click_and_drag(),
            );
            if response.hovered() {
                super::tips::label(ui, HINT_PIN_MOVE, "");
            }
            if response.drag_started() {
                self.dragging = Some(DraggedPin {
                    map: map.serial,
                    pin: at,
                    at: spot,
                });
            }
            if let Some(drag) = self
                .dragging
                .as_mut()
                .filter(|drag| drag.map == map.serial && drag.pin == at)
            {
                if response.dragged() {
                    let moved = drag.at + response.drag_delta();
                    drag.at = Pos2::new(
                        moved.x.clamp(land.left(), land.right()),
                        moved.y.clamp(land.top(), land.bottom()),
                    );
                }
                if response.drag_stopped() {
                    deed = Some(PinDeed::Moved(at, pixel_at(map, land, drag.at)));
                    self.dragging = None;
                }
            } else if response.double_clicked() {
                deed = Some(PinDeed::Removed(at));
            }
        }
        deed
    }

    fn buttons(&self, ui: &egui::Ui, foot: Pos2, map: &WatchMap, tools: &Tools<'_>) {
        let (clear, cleared) = theme::button(ui, foot, WORDS_CLEAR, theme::TEXT);
        let (edit_words, edit_color) = if map.may_plot {
            (WORDS_STOP, theme::WAITING)
        } else {
            (WORDS_EDIT, theme::GOAL)
        };
        let (edit, edited) = theme::button(
            ui,
            Pos2::new(clear.right() + theme::ROW_GAP, foot.y),
            edit_words,
            edit_color,
        );
        let (_, closed) = theme::button(
            ui,
            Pos2::new(edit.right() + theme::ROW_GAP, foot.y),
            WORDS_CLOSE,
            theme::TEXT_DIM,
        );
        if cleared {
            tools.hand.act(Act::MapClear);
        } else if edited {
            tools.hand.act(Act::MapEdit);
        } else if closed {
            tools.hand.act(Act::MapClose(map.serial));
        }
    }

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
        let wish = self.wishes.entry(map.serial).or_default();
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(wish)
                .id(Id::new(("map-wish", map.serial)))
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
        if asked && on && !wish.trim().is_empty() {
            tools.hand.ask(
                Asker::MapItem,
                Ask::PlaceOnMap {
                    wish: wish.trim().to_string(),
                    map: map.facet,
                    from: (map.start_x, map.start_y),
                    to: (map.end_x, map.end_y),
                },
            );
            self.asked_for = Some(map.serial);
            self.note = Some((WORDS_ASKING.into(), false));
        }
    }
}

const PROFILE_WIDTH: f32 = 420.0;
const PROFILE_ROWS: usize = 8;
const PROFILE_LINE: f32 = 20.0;
const WORDS_PROFILE: &str = "Profile";
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
    pub fn show(&mut self, serial: u32, hand: &Hand) {
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
        profile: &mut Profile,
    ) -> Option<Rect> {
        if std::mem::take(&mut self.show_own) && frame.serial != 0 {
            self.show(frame.serial, tools.hand);
        }
        let serial = self.shown?;
        let known = frame.profiles.iter().find(|kept| kept.serial == serial);
        let body_rows = PROFILE_ROWS as f32 * PROFILE_LINE;
        let name = known.map_or("", |kept| kept.name.as_str());
        let title = if name.is_empty() { WORDS_PROFILE } else { name };
        let spec = PanelSpec {
            id: PROFILE_ID,
            title,
            default: layout::first_place(
                rect,
                Spot::Middle(0),
                Vec2::new(
                    PROFILE_WIDTH,
                    frame::TITLE_ROW + TITLE_ROW + body_rows + FOOT_ROW + theme::PANEL_PAD * 2.0,
                ),
            ),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let inner = frame::draw(ui.painter(), panel, title);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            known.map_or("", |kept| kept.title.as_str()),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let body = Rect::from_min_size(
            inner.left_top() + Vec2::new(0.0, TITLE_ROW),
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
        let closed_mark =
            frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if closed || closed_mark {
            self.close();
        }
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::modern::testing::draw_frames;

    fn map(serial: u32) -> WatchMap {
        WatchMap {
            serial,
            start_x: 1000,
            start_y: 1200,
            end_x: 1400,
            end_y: 1600,
            width: 200,
            height: 200,
            may_plot: true,
            pins: vec![(40, 90), (100, 20)],
            ..WatchMap::default()
        }
    }

    #[test]
    fn each_open_map_has_its_panel_and_its_picture_goes_with_it() {
        let frame = WatchFrame {
            human_control: true,
            maps: vec![map(1), map(2)],
            ..WatchFrame::default()
        };
        let mut maps = MapItemUi::default();
        let mut profile = Profile::default();
        let mut covered = Vec::new();
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            covered = maps.draw(ui, rect, &frame, tools, profile);
        });
        assert_eq!(covered.len(), 2);
        assert_ne!(covered[0], covered[1], "each map opens in its own place");
        assert_eq!(maps.pictures.len(), 2);
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            covered = maps.draw(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(covered.is_empty() && maps.pictures.is_empty());
    }
}
