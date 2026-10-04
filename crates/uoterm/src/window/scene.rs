//! The map behind the panels, painted. The rules of the map are
//! `uoterm_view::scene`: each frame it builds a draw list, and here the
//! window paints that list with egui. The pictures the panels ask for, the
//! words in UO fonts and the death screen are painted here too.

pub use uoterm_view::clicks::PickKind;
pub use uoterm_view::scene::Pick;

use super::art_host::NativeArt;
use super::audio::Step;
use super::bridge;
use super::lights::LightMap;
use super::model::health_bars::MapDrag;
use super::model::house_design::{StoreyLook, STOREYS};
use super::settings::Profile;
use super::theme;
use crate::art::client_art::ClientArt;
use crate::view::{WatchFrame, WatchLook};
use eframe::egui::{
    self,
    epaint::{self, Vertex},
    Align2, Color32, Galley, Painter, Pos2, Rect, Shape, Stroke, Vec2,
};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use uoterm_nav::CursorShape;
use uoterm_view::art::{ArtRequest, ItemPaint, Paint, Sprite, TextLook, WorldArt};
use uoterm_view::geom::Rgba;
use uoterm_view::scene::plates::{self, Overhead, PlacedPlate};
pub use uoterm_view::scene::DOLL_FACING;
use uoterm_view::scene::{
    overlays, standing_figure, Mesh, Overlay, Plate, SceneInput, SceneState, DEATH_FONT, DEATH_HUE,
    DEATH_WORDS,
};

/// A fire or a fountain shows its next picture this often, so the window
/// draws again at least this often.
const ART_CYCLE_SECONDS: f64 = 0.1;

pub const NOTE_NO_UOPATH: &str = "No client files. Give --uopath to see the real map.";

pub struct Scene {
    art: NativeArt,
    note: String,
    /// The rules of the map and what they keep between frames.
    state: SceneState,
    lights: LightMap,
    /// The footsteps the sound has not taken yet.
    steps: Vec<Step>,
    /// A drag the human started on the map, until the gumps take it.
    map_drag: Option<MapDrag>,
}

/// Words ready to draw: a picture in a UO font, or the window's own font.
pub enum Words {
    Picture(egui::TextureId, Sprite),
    Font(Arc<Galley>, Color32),
}

impl Words {
    pub fn size(&self) -> Vec2 {
        match self {
            Self::Picture(_, sprite) => Vec2::new(sprite.width, sprite.height),
            Self::Font(galley, _) => galley.size(),
        }
    }

    /// Draws the words with their top left corner at `min`.
    pub fn paint(&self, painter: &Painter, min: Pos2, alpha: f32) {
        match self {
            Self::Picture(texture, sprite) => {
                painter.image(
                    *texture,
                    Rect::from_min_size(min, self.size()),
                    bridge::rect(sprite.uv),
                    Color32::WHITE.gamma_multiply(alpha),
                );
            }
            Self::Font(galley, color) => {
                painter.galley_with_override_text_color(
                    min,
                    galley.clone(),
                    color.gamma_multiply(alpha),
                );
            }
        }
    }
}

/// The picture of a request, with the texture it is in.
fn picture(art: &mut NativeArt, request: &ArtRequest) -> Option<(egui::TextureId, Sprite)> {
    let sprite = art.sprite(request).ready()?;
    Some((art.texture_id()?, sprite))
}

/// Words in a UO font, as `look` says, ready to draw. With no UO fonts in
/// the client files they come in the window's own font.
fn words_of(art: &mut NativeArt, painter: &Painter, text: &str, look: TextLook) -> Words {
    let request = ArtRequest::Text {
        text: text.to_string(),
        look,
    };
    match picture(art, &request) {
        Some((texture, sprite)) => Words::Picture(texture, sprite),
        None => {
            let color = hue_color(art, look.hue);
            let font = theme::title_font(theme::SIZE_PLATE);
            let galley = match look.width {
                Some(wrap) => painter.layout(text.to_string(), font, color, wrap as f32),
                None => painter.layout_no_wrap(text.to_string(), font, color),
            };
            Words::Font(galley, color)
        }
    }
}

/// The color of words in a hue. Without client files, the plain color.
fn hue_color(art: &NativeArt, hue: u16) -> Color32 {
    bridge::color(uoterm_view::art::hue_color(art, hue))
}

/// The mesh of the draw list as egui keeps it, with the texture it reads.
fn egui_mesh(mesh: Mesh, texture: egui::TextureId) -> epaint::Mesh {
    epaint::Mesh {
        indices: mesh.indices,
        vertices: mesh
            .vertices
            .iter()
            .map(|vertex| Vertex {
                pos: Pos2::from(vertex.pos),
                uv: Pos2::from(vertex.uv),
                color: bridge::color(Rgba(vertex.rgba)),
            })
            .collect(),
        texture_id: texture,
    }
}

/// Paints one shape over the world. Pictures need the texture of the map.
fn paint_overlay(painter: &Painter, overlay: Overlay, texture: Option<egui::TextureId>) {
    let stroke = |width: f32, color: Rgba| Stroke::new(width, bridge::color(color));
    match overlay {
        Overlay::ClosedLine {
            points,
            width,
            color,
        } => {
            let points = points.into_iter().map(bridge::pos2).collect();
            painter.add(Shape::closed_line(points, stroke(width, color)));
        }
        Overlay::Segment {
            from,
            to,
            width,
            color,
        } => {
            painter.line_segment([bridge::pos2(from), bridge::pos2(to)], stroke(width, color));
        }
        Overlay::Dashed {
            from,
            to,
            width,
            color,
            dash,
            gap,
        } => {
            let line = [bridge::pos2(from), bridge::pos2(to)];
            painter.extend(Shape::dashed_line(&line, stroke(width, color), dash, gap));
        }
        Overlay::Disc {
            center,
            radius,
            color,
        } => {
            painter.circle_filled(bridge::pos2(center), radius, bridge::color(color));
        }
        Overlay::Circle {
            center,
            radius,
            width,
            color,
        } => {
            painter.circle_stroke(bridge::pos2(center), radius, stroke(width, color));
        }
        Overlay::Polygon {
            points,
            fill,
            width,
            edge,
        } => {
            let points = points.into_iter().map(bridge::pos2).collect();
            painter.add(Shape::convex_polygon(
                points,
                bridge::color(fill),
                stroke(width, edge),
            ));
        }
        Overlay::Pictures(mesh) => {
            if let Some(texture) = texture {
                painter.add(Shape::mesh(egui_mesh(mesh, texture)));
            }
        }
    }
}

/// Paints the names of the Modern style as `plates::lay_out` lays them.
fn paint_plates(painter: &Painter, placed: Vec<PlacedPlate>) {
    let font = theme::title_font(theme::SIZE_PLATE);
    for plate in placed {
        painter.rect_filled(
            bridge::rect(plate.area),
            theme::BAR_RADIUS,
            theme::PLATE_BACK,
        );
        theme::shadowed_text(
            painter,
            bridge::pos2(plate.name_at),
            Align2::CENTER_TOP,
            &plate.name,
            font.clone(),
            bridge::color(plate.name_color),
        );
        if let Some(bar) = plate.bar {
            painter.rect_filled(
                bridge::rect(bar.back),
                theme::BAR_RADIUS,
                theme::TEXT_SHADOW,
            );
            painter.rect_filled(
                bridge::rect(bar.fill),
                theme::BAR_RADIUS,
                bridge::color(bar.color),
            );
        }
    }
}

impl Scene {
    pub fn new(uopath: Option<&Path>) -> Self {
        let (client, note) = match uopath.map(ClientArt::open) {
            Some(Ok(client)) => (Some(client), String::new()),
            Some(Err(e)) => (None, format!("No real map: {e}.")),
            None => (None, NOTE_NO_UOPATH.to_string()),
        };
        Self {
            art: NativeArt::new(client),
            note,
            state: SceneState::new(),
            lights: LightMap::default(),
            steps: Vec::new(),
            map_drag: None,
        }
    }

    /// Why the map shows flat colors. Empty when it shows the real map.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Draws the map into `rect`, as the profile says. True while something
    /// still moves, so the window must draw the next frame at once.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        time: f64,
        profile: &Profile,
    ) -> bool {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::VOID);
        let input = ui.input(|i| SceneInput {
            mouse: i.pointer.hover_pos().map(bridge::point),
            scroll: i.smooth_scroll_delta.y,
            zoom_delta: i.zoom_delta(),
            ctrl: i.modifiers.ctrl,
            shift: i.modifiers.shift,
            pixels_per_point: i.pixels_per_point,
        });
        self.art.make_atlas(ui.ctx());
        let view = bridge::area(rect);
        let draw = self
            .state
            .build(&mut self.art, view, frame, time, profile, &input);
        self.steps.extend(draw.steps);
        // A mobile that stands still moves a little, and a fire burns. Wake
        // for the next picture.
        if self.art.has_art() {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(ART_CYCLE_SECONDS));
        }
        if draw.death.is_some() {
            self.death_screen(&painter, rect);
            return true;
        }
        let Some(texture) = self.art.texture_id() else {
            return draw.moving;
        };
        painter.add(Shape::mesh(egui_mesh(draw.mesh, texture)));
        for overlay in draw.overlays {
            paint_overlay(&painter, overlay, Some(texture));
        }
        if self.state.look().classic() {
            self.paint_overheads(&painter, draw.plates);
        } else {
            let font = theme::title_font(theme::SIZE_PLATE);
            let measure = |name: &str| {
                let galley = painter.layout_no_wrap(name.to_string(), font.clone(), theme::TEXT);
                bridge::vector(galley.size())
            };
            let keep_clear = self.state.character_area(view);
            let placed = plates::lay_out(draw.plates, keep_clear, self.state.zoom(), &measure);
            paint_plates(&painter, placed);
        }
        draw.moving
    }

    /// The death screen: the world is black and says so.
    fn death_screen(&mut self, painter: &Painter, rect: Rect) {
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let words = self.words(painter, DEATH_WORDS, TextLook::ascii(DEATH_FONT, DEATH_HUE));
        words.paint(painter, rect.center() - words.size() / 2.0, 1.0);
        painter
            .ctx()
            .request_repaint_after(Duration::from_secs_f64(ART_CYCLE_SECONDS));
    }

    /// Paints the hit points and the name plates of the Classic style, as
    /// `plates::overheads` lays them.
    fn paint_overheads(&mut self, painter: &Painter, plates: Vec<Plate>) {
        let art = &mut self.art;
        let laid = plates::overheads(
            plates,
            &self.state.look().nameplates,
            self.state.zoom(),
            &mut |text, look| bridge::vector(words_of(art, painter, text, look).size()),
        );
        for part in laid {
            match part {
                Overhead::Gump {
                    gump,
                    hue,
                    area,
                    alpha,
                } => {
                    let request = ArtRequest::Gump {
                        gump,
                        hue,
                        partial: false,
                    };
                    let area = bridge::rect(area);
                    match picture(&mut self.art, &request) {
                        Some((texture, sprite)) => {
                            painter.image(
                                texture,
                                area,
                                bridge::rect(sprite.uv),
                                Color32::WHITE.gamma_multiply(alpha),
                            );
                        }
                        None => {
                            let color = hue_color(&self.art, hue).gamma_multiply(alpha);
                            painter.rect_filled(area, 0.0, color);
                        }
                    }
                }
                Overhead::Words { text, look, at } => {
                    let words = words_of(&mut self.art, painter, &text, look);
                    words.paint(painter, bridge::pos2(at), 1.0);
                }
                Overhead::Fill { area, color } => {
                    painter.rect_filled(bridge::rect(area), 0.0, bridge::color(color));
                }
                Overhead::HueFill { area, hue } => {
                    painter.rect_filled(bridge::rect(area), 0.0, hue_color(&self.art, hue));
                }
            }
        }
    }

    /// The mobiles and corpses that came into view, whose names the window
    /// asks for.
    pub fn take_arrivals(&mut self) -> Vec<u32> {
        self.state.take_arrivals()
    }

    /// The map tells of a left drag the human started on it.
    pub fn start_map_drag(&mut self, drag: MapDrag) {
        self.map_drag = Some(drag);
    }

    /// The drag the human started on the map, once.
    pub fn take_map_drag(&mut self) -> Option<MapDrag> {
        self.map_drag.take()
    }

    /// The mobiles drawn in a box of the screen, first drawn first.
    pub fn mobiles_in(&self, area: Rect) -> Vec<u32> {
        self.state.mobiles_in(bridge::area(area))
    }

    /// How each storey of the house being designed shows, as the designer
    /// sets it.
    pub fn set_storey_looks(&mut self, looks: [StoreyLook; STOREYS]) {
        self.state.set_storey_looks(looks);
    }

    /// True when a point of the window shows the world and no panel.
    pub fn is_on_world(&self, rect: Rect, point: Pos2) -> bool {
        self.state
            .is_on_world(bridge::area(rect), bridge::point(point))
    }

    /// Draws the outline of a building where the mouse points, while the
    /// shard waits for its place. True while one waits.
    pub fn draw_placing(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        mouse: Pos2,
    ) -> bool {
        let view = bridge::area(rect);
        let mouse = bridge::point(mouse);
        let Some(shapes) = self
            .state
            .placing_preview(&mut self.art, view, frame, mouse)
        else {
            return false;
        };
        let Some(texture) = self.art.texture_id() else {
            return false;
        };
        for shape in shapes {
            paint_overlay(painter, shape, Some(texture));
        }
        true
    }

    /// Draws the arrow the shard points at a place. It stands at the edge of
    /// the window when the place is out of view, as a compass needle does.
    /// Gives its box, which the player clicks.
    pub fn draw_quest_arrow(
        &self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
    ) -> Option<Rect> {
        let projection = self.state.projection(bridge::area(rect));
        let (arrow, area) = overlays::quest_arrow(&projection, frame)?;
        paint_overlay(painter, arrow, None);
        Some(bridge::rect(area))
    }

    /// Outlines the ground within `tiles` of the character, as the Combat
    /// page's range circle shows how far a spell reaches.
    pub fn draw_range_circle(&self, painter: &Painter, rect: Rect, tiles: u8, color: Color32) {
        let projection = self.state.projection(bridge::area(rect));
        let diamond = overlays::range_diamond(&projection, tiles, bridge::rgba(color));
        paint_overlay(painter, diamond, None);
    }

    /// Lays the light of the world over the map: the light level, and each
    /// lamp, torch and lit spell in view. `effects` are the places and the
    /// graphics of the spells that show now.
    pub fn draw_lights(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        effects: &[([f32; 3], u16)],
    ) {
        let view = bridge::area(rect);
        if let Some(cells) = self.state.light_cells(&mut self.art, view, frame, effects) {
            self.lights.draw(painter, rect, &cells);
        }
    }

    /// The picture of a mobile as he stands and faces the watcher, for a
    /// paperdoll. It carries what he wears.
    pub fn doll_picture(&mut self, look: &WatchLook) -> Option<(egui::TextureId, Sprite)> {
        let ring = Paint::outlined(bridge::rgba(theme::SELF_FIGURE));
        self.standing_picture(look, DOLL_FACING, ring)
    }

    /// The picture of a mobile as he stands turned to `direction`, with no
    /// ring round it, as the figure of a new character shows.
    pub fn turned_picture(
        &mut self,
        look: &WatchLook,
        direction: u8,
    ) -> Option<(egui::TextureId, Sprite)> {
        self.standing_picture(look, direction, Paint::outlined(Rgba::TRANSPARENT))
    }

    /// The picture of a creature as a shopkeeper shows it for sale:
    /// standing, facing the watcher, with no ring round it.
    pub fn creature_picture(&mut self, body: u16, hue: u16) -> Option<(egui::TextureId, Sprite)> {
        let look = WatchLook {
            body,
            hue,
            ..WatchLook::default()
        };
        self.turned_picture(&look, DOLL_FACING)
    }

    fn standing_picture(
        &mut self,
        look: &WatchLook,
        direction: u8,
        paint: Paint,
    ) -> Option<(egui::TextureId, Sprite)> {
        picture(&mut self.art, &standing_figure(look, direction, paint))
    }

    /// Makes the texture the pictures go into, as the first draw of the
    /// world does, for gumps drawn with no world under them, as the login
    /// screens are.
    pub fn make_atlas(&mut self, ctx: &egui::Context) {
        self.art.make_atlas(ctx);
    }

    /// True when gumps can show in their own pictures.
    pub fn has_gump_art(&self) -> bool {
        self.art.has_gump_art()
    }

    /// A picture of a gump, for a window that is not the map.
    pub fn gump_picture(&mut self, gump: u16, hue: u16) -> Option<(egui::TextureId, Sprite)> {
        self.hued_gump_picture(gump, hue, false)
    }

    /// A picture of a gump in a hue that colors its grey pixels only, as a
    /// body or a worn item on a paperdoll takes it when `partial` is set.
    pub fn hued_gump_picture(
        &mut self,
        gump: u16,
        hue: u16,
        partial: bool,
    ) -> Option<(egui::TextureId, Sprite)> {
        picture(&mut self.art, &ArtRequest::Gump { gump, hue, partial })
    }

    /// True when the gump draws the pixel at `x`, `y` of its picture.
    pub fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool {
        self.art.gump_drawn_at(gump, x, y)
    }

    /// What `Equipconv.def` puts in the place of a worn item on a body.
    pub fn equip_conv(&self, body: u16, worn_anim: u16) -> Option<uoterm_nav::EquipConv> {
        self.art.anim().equip_conv(body, worn_anim)
    }

    /// The color of one tile on a map of the world.
    pub fn radar_rgb(&mut self, map: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        self.art.radar_rgb(map, x, y)
    }

    /// The height of the land of one tile of a map of the world.
    pub fn land_z(&mut self, map: u8, x: u16, y: u16) -> Option<i8> {
        self.art.land_z(map, x, y)
    }

    /// The pixels of a gump picture as the files hold them.
    pub fn gump_pixels(&self, gump: u16) -> Option<uoterm_nav::ArtPixels> {
        self.art.gump_pixels(gump)
    }

    /// Where a place of the world is on the screen.
    pub fn screen_of(&self, rect: Rect, place: [f32; 3]) -> Pos2 {
        bridge::pos2(self.state.screen_of(bridge::area(rect), place))
    }

    /// Where a mobile is drawn now. None for one that is not in view.
    pub fn place_of(&self, frame: &WatchFrame, serial: u32) -> Option<[f32; 3]> {
        self.state.place_of(frame, serial)
    }

    pub fn zoom(&self) -> f32 {
        self.state.zoom()
    }

    /// Tells how often the window reads the session. The news of a step
    /// comes that much later.
    pub fn set_poll_every(&mut self, every: Duration) {
        self.state.set_poll_every(every.as_secs_f64());
    }

    /// Sets the zoom, inside the range the wheel allows.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.state.set_zoom(zoom);
    }

    /// Moves the camera this far from the character, in points. Zero puts
    /// the character back in the middle.
    pub fn set_peek(&mut self, peek: Vec2) {
        self.state.set_peek(bridge::vector(peek));
    }

    /// The window tells where its panels are, for the next frame.
    pub fn set_panels(&mut self, panels: Vec<Rect>) {
        self.state
            .set_panels(panels.into_iter().map(bridge::area).collect());
    }

    /// The footsteps since the last call, for the sound.
    pub fn take_steps(&mut self) -> Vec<Step> {
        std::mem::take(&mut self.steps)
    }

    /// The picture of an item, for a window that is not the map.
    pub fn item_picture(&mut self, graphic: u16, hue: u16) -> Option<(egui::TextureId, Sprite)> {
        self.painted_item_picture(
            graphic,
            ItemPaint {
                hue,
                ..ItemPaint::default()
            },
        )
    }

    /// The picture of an item in a hue that covers every pixel, as a gump
    /// marks the item under the mouse.
    pub fn item_picture_whole_hue(
        &mut self,
        graphic: u16,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        self.painted_item_picture(
            graphic,
            ItemPaint {
                hue,
                whole_hue: true,
                ..ItemPaint::default()
            },
        )
    }

    fn painted_item_picture(
        &mut self,
        graphic: u16,
        paint: ItemPaint,
    ) -> Option<(egui::TextureId, Sprite)> {
        let sprite = self
            .state
            .item_sprite(&mut self.art, graphic, paint, true)
            .ready()?;
        Some((self.art.texture_id()?, sprite))
    }

    /// The tiledata record of an item graphic, when the files have one.
    pub fn item_tile(&self, graphic: u16) -> Option<&uoterm_nav::ItemTile> {
        self.art.item_tile(graphic)
    }

    /// The point over a mobile or a thing that is drawn now, where the
    /// words it says float.
    pub fn head_of(&self, rect: Rect, frame: &WatchFrame, serial: u32) -> Option<Pos2> {
        self.state
            .head_of(bridge::area(rect), frame, serial)
            .map(bridge::pos2)
    }

    /// Words in a UO font, as `look` says, ready to draw. With no UO fonts
    /// in the client files they come in the window's own font.
    pub fn words(&mut self, painter: &Painter, text: &str, look: TextLook) -> Words {
        words_of(&mut self.art, painter, text, look)
    }

    /// The lines words break into in a UO font, as `look` breaks them.
    /// Empty when the client files hold no UO fonts.
    pub fn text_lines(&self, text: &str, look: &TextLook) -> Vec<String> {
        self.art.text_lines(text, look)
    }

    /// The picture of a mouse pointer of the classic client, with its point
    /// as the anchor. None without client files.
    pub fn cursor_picture(
        &mut self,
        shape: CursorShape,
        war: bool,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        picture(&mut self.art, &ArtRequest::Cursor { shape, war, hue })
    }

    /// The color of words in a hue. Without client files, the plain color.
    pub fn words_color(&self, hue: u16) -> Color32 {
        hue_color(&self.art, hue)
    }

    /// The thing on top under the mouse.
    pub fn thing_at(&self, mouse: Pos2) -> Option<&Pick> {
        self.state.thing_at(bridge::point(mouse))
    }

    /// The tile under the mouse, and the height of its floor.
    pub fn tile_at(&mut self, rect: Rect, frame: &WatchFrame, mouse: Pos2) -> (u16, u16, i8) {
        self.state.tile_at(
            &mut self.art,
            bridge::area(rect),
            frame,
            bridge::point(mouse),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));

    #[test]
    fn real_files_pick_the_floor_of_a_building_and_not_the_land_under_it() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const BANK_FLOOR: i8 = 20;
        let mut scene = Scene::new(Some(&dir));
        let frame = WatchFrame {
            x: 3484,
            y: 2570,
            z: BANK_FLOOR,
            map: 1,
            ..WatchFrame::default()
        };
        scene.state.build(
            &mut scene.art,
            bridge::area(WINDOW),
            &frame,
            0.0,
            &Profile::default(),
            &SceneInput::default(),
        );
        let drawn_at = scene.screen_of(WINDOW, [3486.0, 2572.0, f32::from(BANK_FLOOR)]);
        assert_eq!(
            scene.tile_at(WINDOW, &frame, drawn_at),
            (3486, 2572, BANK_FLOOR)
        );
    }

    #[test]
    fn the_draw_list_keeps_its_triangles_and_colors_in_egui() {
        let vertex = uoterm_view::scene::Vertex {
            pos: [1.0, 2.0],
            uv: [0.5, 0.25],
            rgba: [10, 20, 30, 40],
        };
        let mesh = Mesh {
            vertices: vec![vertex; 3],
            indices: vec![0, 1, 2],
        };
        let texture = egui::TextureId::Managed(1);
        let painted = egui_mesh(mesh, texture);
        assert_eq!(painted.indices, [0, 1, 2]);
        assert_eq!(painted.texture_id, texture);
        assert_eq!(painted.vertices[0].pos, Pos2::new(1.0, 2.0));
        assert_eq!(painted.vertices[0].uv, Pos2::new(0.5, 0.25));
        assert_eq!(
            painted.vertices[0].color,
            Color32::from_rgba_premultiplied(10, 20, 30, 40)
        );
    }
}
