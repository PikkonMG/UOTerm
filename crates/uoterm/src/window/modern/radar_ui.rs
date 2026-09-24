//! The radar of the Modern style, as the minimap of the classic client: the
//! land round the character, turned as the play field is turned, with the
//! mobiles in the colors of their notoriety, the party, the markers and the
//! character in the middle. It shows until the player shuts it; he moves
//! it, locks it, folds it to its title, zooms it with the wheel, and a
//! double click makes it large or small, which the World Map page keeps as
//! the classic minimap's large picture. The land and the marks are
//! `map_view`'s, shared with the world maps.

use super::super::boxes_ui::Tools;
use super::super::map_ui::mark_look;
use super::super::map_view::{self, Lay, MapFilesCache, MapPictures, Marks};
use super::super::model::places;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Id, Rect, Sense, Vec2};

pub const RADAR_ID: &str = "modern:radar";
const SMALL_SIDE: f32 = 190.0;
const LARGE_SIDE: f32 = 320.0;
/// The radar starts close, and the wheel takes it closer or back to the
/// whole picture round the character.
const FIRST_ZOOM: f32 = 4.0;
const ZOOM_MAX: f32 = 16.0;
const HALF: f32 = 2.0;
const WORDS_TITLE: &str = "Radar";
const WORDS_NO_FILES: &str = "The radar needs the client files.";
const HINT_RADAR: &str = "Wheel: zoom. Double-click: larger or smaller.";

/// The side of the field for the size the World Map page keeps.
/// The size of the radar panel, large or small.
pub(super) fn panel_size(large: bool) -> Vec2 {
    Vec2::splat(field_side(large))
        + Vec2::new(0.0, frame::TITLE_ROW)
        + Vec2::splat(theme::PANEL_PAD * 2.0)
}

fn field_side(large: bool) -> f32 {
    if large {
        LARGE_SIDE
    } else {
        SMALL_SIDE
    }
}

pub struct RadarUi {
    pictures: MapPictures,
    files: MapFilesCache,
    zoom: f32,
}

impl Default for RadarUi {
    fn default() -> Self {
        Self {
            pictures: MapPictures::default(),
            files: MapFilesCache::default(),
            zoom: FIRST_ZOOM,
        }
    }
}

impl RadarUi {
    /// Draws the radar unless the player shut it. Gives its place.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        if places::is_shut(profile, RADAR_ID) {
            return None;
        }
        let size = panel_size(profile.world_map.minimap_large);
        let spec = PanelSpec {
            id: RADAR_ID,
            title: WORDS_TITLE,
            default: layout::first_place(rect, Spot::Radar, size),
            min_size: None,
            closable: true,
        };
        let whole = frame::place(rect, &spec, profile);
        let folded = places::is_folded(profile, RADAR_ID);
        let panel = frame::shown_rect(whole, folded);
        let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
        if !folded {
            self.field(ui, body, frame, tools, profile);
        }
        if frame::foldable_controls(ui, whole, &spec, profile, tools) == Some(FrameEvent::Closed) {
            places::set_shut(profile, RADAR_ID, true);
            tools.keep_profile(profile);
        }
        Some(panel)
    }

    fn field(
        &mut self,
        ui: &egui::Ui,
        field: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let response = ui.interact(field, Id::new("radar-field"), Sense::click());
        let notches = map_view::wheel_notches(ui, &response);
        if notches != 0.0 {
            self.zoom = map_view::zoomed(self.zoom, notches, ZOOM_MAX);
        }
        if response.hovered() {
            super::super::tips::label(ui, HINT_RADAR, "");
        }
        if response.double_clicked() {
            profile.world_map.minimap_large = !profile.world_map.minimap_large;
            tools.keep_profile(profile);
        }
        if !self
            .pictures
            .make_near(ui.ctx(), tools.scene, frame.map, (frame.x, frame.y))
        {
            ui.painter().text(
                field.center(),
                Align2::CENTER_CENTER,
                WORDS_NO_FILES,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            return;
        }
        let lay = Lay::Turned {
            center: field.center(),
            from: Vec2::new(f32::from(frame.x), f32::from(frame.y)),
            unit: field.width().min(field.height()) / (map_view::SPAN as f32 * HALF) * self.zoom,
        };
        let painter = ui.painter().with_clip_rect(field);
        self.pictures.draw_near(&painter, lay);
        let session = map_view::session_markers(tools.readings, profile, frame.map);
        let marks = Marks {
            frame,
            map: frame.map,
            profile,
            files: self.files.get(profile),
            session: &session,
            looking_at: None,
        };
        map_view::overlays(&painter, field, lay, &marks, &mark_look());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_large_radar_is_larger() {
        assert!(field_side(true) > field_side(false));
        assert_eq!(RadarUi::default().zoom, FIRST_ZOOM);
    }
}
