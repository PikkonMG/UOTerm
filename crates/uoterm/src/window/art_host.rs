//! The art of the play window: the client files it reads, and the texture
//! the pictures go into. Each picture is made the first time the scene asks
//! for it and kept under its request, so the answers are never pending.

use super::atlas::Atlas;
use crate::art::client_art::ClientArt;
use eframe::egui;
use std::collections::HashMap;
use uoterm_nav::{Action, AnimRules, ArtPixels, ItemTile, LandTile, LightShape, MultiPiece};
use uoterm_view::art::{Art, ArtRequest, Cell, Picture, Sprite, TextLook, WorldArt};
use uoterm_view::atlas::{ShelfPacker, ATLAS_SIDE, WHITE_SIDE};
use uoterm_view::frame::{WatchLiveMap, WatchLook};
use uoterm_view::geom::{Point, Vector};
use uoterm_view::ui::theme;

const RGBA_BYTES: usize = 4;
/// Words in a UO font are one line this tall when the client files hold no
/// UO fonts.
const NO_FONT_LINE_HEIGHT: f32 = 1.0;

pub struct NativeArt {
    /// None when there are no client files.
    client: Option<ClientArt>,
    /// None until the window gives the texture its first frame.
    atlas: Option<Atlas>,
    packer: ShelfPacker,
    /// The pictures in the texture, by the key of their request. None marks
    /// a picture the client files do not hold.
    sprites: HashMap<u64, Option<Sprite>>,
    /// The tables when the client files hold no animation files.
    no_anim: AnimRules,
}

impl NativeArt {
    pub fn new(client: Option<ClientArt>) -> Self {
        Self {
            client,
            atlas: None,
            packer: ShelfPacker::new(ATLAS_SIDE),
            sprites: HashMap::new(),
            no_anim: AnimRules::default(),
        }
    }

    /// Makes the texture the pictures go into, the first time.
    pub fn make_atlas(&mut self, ctx: &egui::Context) {
        if self.atlas.is_none() {
            self.atlas = Some(Atlas::new(ctx));
            self.start_again();
        }
    }

    /// The texture of the pictures. None before [`NativeArt::make_atlas`].
    pub fn texture_id(&self) -> Option<egui::TextureId> {
        self.atlas.as_ref().map(Atlas::texture_id)
    }

    /// A point of the texture that is plain white. A shape with no picture
    /// takes its color from its vertices alone when it reads this point.
    pub fn white_uv(&self) -> Point {
        self.packer.white_uv()
    }

    /// The pixels of a gump picture as the files hold them, for a gump that
    /// draws into its own picture, as the minimap does.
    pub fn gump_pixels(&self, gump: u16) -> Option<ArtPixels> {
        self.client.as_ref()?.gump_pixels(gump)
    }

    /// Forgets every picture and lays the white square again.
    fn start_again(&mut self) {
        let Some(atlas) = self.atlas.as_mut() else {
            return;
        };
        atlas.clear();
        self.sprites.clear();
        self.packer.reset();
        let white = Picture {
            width: WHITE_SIDE,
            height: WHITE_SIDE,
            rgba: vec![u8::MAX; WHITE_SIDE * WHITE_SIDE * RGBA_BYTES],
            anchor: Vector::ZERO,
        };
        atlas.put(self.packer.white(), &white);
    }

    /// Puts a picture in the texture. When the texture is full, every
    /// picture is forgotten and the texture fills again from the pictures
    /// the next frames ask for.
    fn place(&mut self, picture: &Picture) -> Option<Sprite> {
        let (width, height) = (picture.width, picture.height);
        if !self.packer.fits(width, height) {
            return None;
        }
        let placement = match self.packer.place(width, height) {
            Some(placement) => placement,
            None => {
                self.start_again();
                self.packer.place(width, height)?
            }
        };
        self.atlas.as_mut()?.put(placement, picture);
        Some(Sprite {
            uv: self.packer.uv(placement),
            width: width as f32,
            height: height as f32,
            anchor: picture.anchor,
        })
    }
}

impl WorldArt for NativeArt {
    fn has_art(&self) -> bool {
        self.client.is_some()
    }

    fn sprite(&mut self, request: &ArtRequest) -> Art<Sprite> {
        if self.atlas.is_none() {
            return Art::Missing;
        }
        let key = request.key();
        if let Some(known) = self.sprites.get(&key) {
            return (*known).into();
        }
        let picture = self
            .client
            .as_ref()
            .and_then(|client| client.picture(request));
        let sprite = picture.and_then(|picture| self.place(&picture));
        self.sprites.insert(key, sprite);
        sprite.into()
    }

    fn cell(&mut self, map: u8, x: u16, y: u16) -> Art<&Cell> {
        self.client.as_mut().and_then(|c| c.cell(map, x, y)).into()
    }

    fn take_live_map(&mut self, live: &WatchLiveMap) {
        if let Some(client) = self.client.as_mut() {
            client.take_live_map(live);
        }
    }

    fn item_tile(&self, graphic: u16) -> Option<&ItemTile> {
        self.client.as_ref()?.item_tile(graphic)
    }

    fn land_tile(&self, land_id: u16) -> Option<&LandTile> {
        self.client.as_ref()?.land_tile(land_id)
    }

    fn multi_pieces(&mut self, multi: u16) -> Art<&[MultiPiece]> {
        self.client
            .as_ref()
            .map(|client| client.multi_pieces(multi))
            .into()
    }

    fn shown_graphic(&self, graphic: u16, time_ms: u64) -> u16 {
        self.client
            .as_ref()
            .map_or(graphic, |client| client.shown_graphic(graphic, time_ms))
    }

    fn season_land(&self, season: u8, land_id: u16) -> u16 {
        self.client
            .as_ref()
            .map_or(land_id, |client| client.season_land(season, land_id))
    }

    fn season_item(&self, season: u8, graphic: u16) -> u16 {
        self.client
            .as_ref()
            .map_or(graphic, |client| client.season_item(season, graphic))
    }

    fn radar_rgb(&mut self, map: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        self.client.as_mut()?.radar_rgb(map, x, y)
    }

    fn land_z(&mut self, map: u8, x: u16, y: u16) -> Option<i8> {
        self.client.as_mut()?.land_z(map, x, y)
    }

    fn light_shape(&mut self, id: u8) -> Art<&LightShape> {
        self.client.as_mut().and_then(|c| c.light_shape(id)).into()
    }

    fn anim(&self) -> &AnimRules {
        self.client
            .as_ref()
            .and_then(ClientArt::anim_rules)
            .unwrap_or(&self.no_anim)
    }

    fn frame_count(&mut self, look: &WatchLook, action: Action) -> Art<usize> {
        self.client
            .as_ref()
            .and_then(|client| client.frame_count(look, action))
            .into()
    }

    fn line_height(&self, look: &TextLook) -> f32 {
        self.client
            .as_ref()
            .and_then(|client| client.line_height(look))
            .map_or(NO_FONT_LINE_HEIGHT, |height| height as f32)
    }

    fn text_lines(&self, text: &str, look: &TextLook) -> Vec<String> {
        self.client
            .as_ref()
            .map(|client| client.text_lines(text, look))
            .unwrap_or_default()
    }

    fn text_rgb(&self, hue: u16) -> [u8; 3] {
        let [red, green, blue, _] = theme::TEXT.to_array();
        self.client
            .as_ref()
            .and_then(|client| client.text_rgb(hue))
            .unwrap_or([red, green, blue])
    }

    fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.gump_drawn_at(gump, x, y))
    }

    fn has_gump_art(&self) -> bool {
        self.client.as_ref().is_some_and(ClientArt::has_gump_art)
    }
}
