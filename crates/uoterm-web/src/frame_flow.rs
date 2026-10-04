//! One frame of the view, in the order of the frame of the Rust window:
//! the controls, the map, the shard reports, the sky, the floats, then the
//! hotbar keys, the chat line and the clicks on the map. Every decision is
//! a rule of `uoterm_view`; here the rules meet the input of the page and
//! give what to draw and the calls for the page.

use crate::buffers::{DrawBuffers, MeshArrays, PlacedWords, Shapes};
use crate::input::FrameInput;
use crate::out::{Hand, OutCall};
use crate::{kept, TooltipData, WebView};
use uoterm_assist::spells::School;
use uoterm_view::act::Act;
use uoterm_view::actions::controls::{ControlHost, FrameIn};
use uoterm_view::actions::windows::{
    character_view, deck_tab, shown_panel, wanted, CharacterView, Tab,
};
use uoterm_view::actions::{GumpKind, GumpOp, LocalAim, WindowCommand};
use uoterm_view::art::{hue_color, ArtRequest, ItemPaint, TextLook, WorldArt};
use uoterm_view::clicks::{
    act_for_click, escape_on_map, grabbed, EscapeOnMap, GroundClicks, PickKind,
};
use uoterm_view::floats::{self, SPEECH_LINE};
use uoterm_view::frame::WatchFrame;
use uoterm_view::frame::WatchPackItem;
use uoterm_view::geom::{Area, Point, Rgba, Vector};
use uoterm_view::input::KeyPress;
use uoterm_view::keys::chat::{LineKey, Said};
use uoterm_view::keys::Focus;
use uoterm_view::model::asked::asked_commands;
use uoterm_view::model::counters::slot_act;
use uoterm_view::model::health_bars::{MapDrag, Pointer};
use uoterm_view::model::spell_data::book_of;
use uoterm_view::scene::plates::lay_out;
use uoterm_view::scene::{
    overlays, SceneDraw, SceneInput, SceneState, DEATH_FONT, DEATH_HUE, DEATH_WORDS,
};
use uoterm_view::settings::{CombatOptions, Profile, UiStyle};
use uoterm_view::sky::{
    drop_color, effect_area, lightning_bolt, lit_effects, shown_effects, storm_tint, weather_drops,
    Drop, ShownEffect, LIGHTNING_WIDTH, RAIN_WIDTH, SNOW_RADIUS,
};
use uoterm_view::steer::{Movement, SteerInput};
use uoterm_view::ui::deck::hotbar_key_slot;
use uoterm_view::ui::launch::{self, JOURNAL_ID, RADAR_ID};
use uoterm_view::ui::ring::Subject;

const ESCAPE_KEY: &str = "Escape";
const ENTER_KEY: &str = "Enter";

/// The view as the shared controls see it for one frame.
struct WebHost<'a> {
    hand: &'a mut Hand,
    scene: &'a mut SceneState,
    kept_style: UiStyle,
}

impl ControlHost for WebHost<'_> {
    fn act(&mut self, act: Act) {
        self.hand.act(act);
    }

    fn aim(&mut self, aim: LocalAim) {
        self.hand.aim(aim);
    }

    fn watch_over(&mut self, frame: &WatchFrame, combat: &CombatOptions) {
        self.hand.watch_over(frame, combat);
    }

    fn take_notes(&mut self) -> Vec<String> {
        self.hand.take_notes()
    }

    fn zoom(&self) -> f32 {
        self.scene.zoom()
    }

    fn set_zoom(&mut self, zoom: f32) {
        self.scene.set_zoom(zoom);
    }

    fn set_peek(&mut self, peek: Vector) {
        self.scene.set_peek(peek);
    }

    fn ask_screenshot(&mut self) {
        self.hand.push(OutCall::Screenshot);
    }

    fn save_profile(&mut self, profile: &Profile) {
        if let Ok(profile) = serde_json::to_value(kept(profile, self.kept_style)) {
            self.hand.push(OutCall::SaveProfile { profile });
        }
    }
}

impl WebView {
    /// One frame with a picture of the session, in the order of the frame
    /// of the Rust window: the controls, the map, the shard reports, the
    /// sky, the floats, then the clicks on the map and the chat line.
    pub(crate) fn run_frame(
        &mut self,
        frame: &WatchFrame,
        now: f64,
        view: Area,
        mouse: Option<Point>,
        input: FrameInput,
    ) -> DrawBuffers {
        let focus = self.inputs.focus(self.chat.text.is_empty());
        let mut host = WebHost {
            hand: &mut self.hand,
            scene: &mut self.scene,
            kept_style: self.kept_style,
        };
        let controls = self.controls.frame(
            FrameIn {
                frame,
                profile: &mut self.profile,
                time: now,
                presses: &input.presses,
                focus,
                mouse,
                view,
                pad_note: "",
            },
            &mut host,
        );
        for command in controls.style {
            self.style_command(frame, command);
        }
        match controls.history_older {
            Some(true) => self.chat.older(),
            Some(false) => self.chat.newer(),
            None => {}
        }
        let unused: Vec<KeyPress> = input
            .presses
            .iter()
            .filter(|press| press.pressed)
            .filter(|press| {
                !controls
                    .used
                    .iter()
                    .any(|(mods, key)| *key == press.key && *mods == press.mods)
            })
            .cloned()
            .collect();
        // A key that fired something types nothing, as egui drops the
        // letters of a frame whose key was used.
        let texts: &[String] = if controls.used.is_empty() {
            &input.texts
        } else {
            &[]
        };
        let mods = self.inputs.mods();
        let scene_input = SceneInput {
            mouse,
            scroll: input.scroll,
            zoom_delta: input.zoom_delta,
            ctrl: mods.ctrl,
            shift: mods.shift,
            pixels_per_point: self.pixels_per_point,
        };
        let draw = self
            .scene
            .build(&mut self.art, view, frame, now, &self.profile, &scene_input);
        let mut world = Shapes::new(self.art.white_uv());
        let mut overlay = Shapes::new(self.art.white_uv());
        let mut buffers = DrawBuffers {
            moving: draw.moving,
            ..DrawBuffers::default()
        };
        self.lay_world(view, draw, &mut world, &mut buffers);
        let arrivals = self.scene.take_arrivals();
        for act in self.reports.acts(frame, &self.profile, arrivals, view, now) {
            self.hand.act(act);
        }
        buffers.moving |= self.lay_sky(frame, now, view, &mut world, &mut buffers);
        buffers.floats = self.lay_floats(frame, now, view);
        buffers.moving |= !self.floats.live().is_empty();
        self.tips.begin(self.hand.take_tips(), now);
        self.hand.begin(now);
        if let Some((arrow, _)) = overlays::quest_arrow(&self.scene.projection(view), frame) {
            overlay.overlay(arrow);
        }
        self.tooltip = None;
        // The hotbar, the chat line, then the map, as the Rust window draws
        // them.
        let escape = unused.iter().any(|press| press.key.0 == ESCAPE_KEY);
        // The question of the Modern style takes Escape for its No; the
        // ring and the picker of the hotbar shut on it, and the map sees it
        // too.
        let escape = if escape && self.panels.bar.question.is_some() {
            self.answer_asked(false);
            false
        } else {
            if escape {
                self.close_ring();
                self.panels.deck.picking = None;
            }
            escape
        };
        if frame.human_control {
            if focus == Focus::Free {
                for slot in unused
                    .iter()
                    .filter_map(|press| hotbar_key_slot(&press.key))
                {
                    self.press_slot(frame, slot);
                }
            }
            if !self.chat.is_hidden() {
                self.chat_keys(frame, focus, &unused, texts);
            }
            let on_map = MapInput {
                view,
                mouse,
                escape,
                movement: &controls.movement,
            };
            self.act_on_map(frame, now, &input, on_map, &mut overlay);
            buffers.moving |= self.steer.walks();
        } else {
            self.panels.deck.picking = None;
            self.panels.desk.dragging = None;
        }
        buffers.moving |= self.follow_panels(frame, now, mouse);
        if let Some(area) = self.panels.bars.selecting {
            overlay.select_box(area);
        }
        self.hand.ask_due();
        self.scene.set_panels(self.covered.clone());
        self.art.end_frame();
        buffers.world = MeshArrays::from(world.into_mesh());
        buffers.overlay = MeshArrays::from(overlay.into_mesh());
        buffers.uploads = self.art.take_uploads();
        buffers.atlas_reset = self.art.take_atlas_reset();
        buffers
    }

    /// The panels in one frame, as the Rust window draws them: the
    /// journal keeps its lines, the vitals move, the health bars follow
    /// the map and the shard, and the sheet takes its answers. True while
    /// something of them still moves.
    fn follow_panels(&mut self, frame: &WatchFrame, now: f64, mouse: Option<Point>) -> bool {
        self.follow_journal(frame);
        let moving = self.follow_vitals(frame, now);
        let pointer = Pointer {
            at: mouse,
            down: self.inputs.primary_down(),
            mods: self.inputs.mods(),
        };
        self.follow_bars(frame, pointer);
        self.follow_sheet(frame, now);
        self.follow_deals(frame, now);
        self.follow_grids(frame, now);
        self.follow_pages(frame);
        self.follow_doll(frame, now);
        self.follow_asks(frame);
        self.follow_gumps(frame);
        self.follow_build(frame, now);
        self.follow_map_items(frame);
        self.follow_marker_changes(now);
        moving || frame.danger() != uoterm_view::frame::Danger::Calm
    }

    /// Does a command of the windows of the Modern style, as the Modern
    /// windows of the Rust window do. The windows of the page that the
    /// view does not keep go to the page.
    pub(crate) fn style_command(&mut self, frame: &WatchFrame, command: WindowCommand) {
        let before = self.profile.clone();
        match command {
            WindowCommand::ToggleChat => self.chat.toggle_hidden(),
            WindowCommand::PasteToChat => {
                self.chat.paste();
            }
            WindowCommand::UseCounterSlot(slot) => {
                let act = slot_act(frame, &self.profile.counters, slot);
                if let Some(act) = act.filter(|_| frame.human_control) {
                    self.hand.act(act);
                }
            }
            WindowCommand::QuitGame => self.ask_quit(),
            WindowCommand::CloseHealthBars { inactive_only } => {
                self.close_health_bars(frame, inactive_only);
            }
            WindowCommand::CloseAllGumps => self.close_all(frame),
            WindowCommand::CloseCorpses => self.panels.grids.closed.close_open(frame, true),
            WindowCommand::Gump(op, kind) => {
                if !self.gump(frame, op, kind) {
                    self.hand.push(OutCall::Window {
                        command: WindowCommand::Gump(op, kind),
                    });
                }
            }
            command => self.hand.push(OutCall::Window { command }),
        }
        if self.profile != before {
            self.keep_profile();
        }
    }

    /// Closes every panel of the view that closes, as the classic client's
    /// "close all gumps": the launcher, the sheet, the panels of the
    /// launcher, the health bars and the question; and the book, the board
    /// and the map items the shard opened.
    fn close_all(&mut self, frame: &WatchFrame) {
        self.panels.bar.launcher_open = false;
        self.panels.bar.question = None;
        self.panels.sheet.open = false;
        launch::close_all(&mut self.profile);
        self.close_health_bars(frame, false);
        self.panels.grids.closed.close_open(frame, false);
        self.close_doll();
        self.close_world_map();
        self.panels.build.chat.open = false;
        self.panels.map_items.profile.close();
        if frame.human_control {
            if frame.book.is_some() {
                self.hand.act(Act::BookClose);
            }
            if frame.board.is_some() {
                self.hand.act(Act::BoardClose);
            }
            for map in &frame.maps {
                self.hand.act(Act::MapClose(map.serial));
            }
        }
    }

    /// Opens or closes a window of the view. False when the view keeps no
    /// such window: the page has it.
    fn gump(&mut self, frame: &WatchFrame, op: GumpOp, kind: GumpKind) -> bool {
        if let Some(school) = kind.school() {
            return self.spellbook(frame, op, school);
        }
        match kind {
            GumpKind::WorldMap => {
                if wanted(op, self.world_map_open()) != self.world_map_open() {
                    self.toggle_world_map();
                }
                return true;
            }
            GumpKind::Chat => {
                let chat = &mut self.panels.build.chat;
                chat.open = wanted(op, chat.open);
                return true;
            }
            _ => {}
        }
        let sheet = &self.panels.sheet;
        if let Some(view) = character_view(kind) {
            let shows = sheet.open && sheet.tab == Tab::Character && sheet.view == view;
            self.show_sheet(op, shows, Tab::Character, Some(view));
            return true;
        }
        if let Some(tab) = deck_tab(kind) {
            let shows = sheet.open && sheet.tab == tab;
            self.show_sheet(op, shows, tab, None);
            return true;
        }
        match kind {
            GumpKind::Backpack => {
                let Some(bag) = frame.backpack() else {
                    return false;
                };
                if let Some(act) = self.panels.grids.closed.backpack(frame, bag, op) {
                    self.hand.act(act);
                }
            }
            GumpKind::Journal => shown_panel(op, JOURNAL_ID, &mut self.profile),
            GumpKind::Minimap => shown_panel(op, RADAR_ID, &mut self.profile),
            GumpKind::Counters => {
                self.profile.counters.enabled = wanted(op, self.profile.counters.enabled);
            }
            GumpKind::InfoBar => {
                self.profile.info_bar.enabled = wanted(op, self.profile.info_bar.enabled);
            }
            GumpKind::Buffs => {
                let combat = &mut self.profile.combat;
                combat.improved_buff_bar = wanted(op, combat.improved_buff_bar);
            }
            _ => return false,
        }
        true
    }

    /// Opens the sheet on a tab and a view, or closes it when it shows them
    /// and the command closes.
    fn show_sheet(&mut self, op: GumpOp, shows: bool, tab: Tab, view: Option<CharacterView>) {
        let sheet = &mut self.panels.sheet;
        if wanted(op, shows) {
            sheet.open = true;
            sheet.tab = tab;
            if let Some(view) = view {
                sheet.view = view;
            }
        } else if shows {
            sheet.open = false;
        }
    }

    /// Opens or closes the spells tab on a book of a school. Without a
    /// book of it the shard is asked to open one, and the tab turns to it
    /// when it comes. A book of masteries has no open command.
    fn spellbook(&mut self, frame: &WatchFrame, op: GumpOp, school: School) -> bool {
        let Some(book) = book_of(school) else {
            return false;
        };
        let sheet = &self.panels.sheet;
        let shows =
            sheet.open && sheet.tab == Tab::Spells && self.shown_school(frame) == Some(school);
        if !wanted(op, shows) {
            if shows {
                self.panels.sheet.open = false;
            }
            return true;
        }
        if !shows && !self.choose_school(frame, school) {
            if school == School::Mastery {
                return false;
            }
            self.hand.act(Act::OpenSpellbook(book.name));
        }
        self.panels.sheet.open = true;
        self.panels.sheet.tab = Tab::Spells;
        true
    }

    /// The world, the marks over it and the name plates; the death screen
    /// in their place while it shows.
    fn lay_world(
        &mut self,
        view: Area,
        draw: SceneDraw,
        world: &mut Shapes,
        buffers: &mut DrawBuffers,
    ) {
        buffers.steps = draw.steps;
        if draw.death.is_some() {
            world.fill(view, BLACK);
            let words = ArtRequest::Text {
                text: DEATH_WORDS.to_string(),
                look: TextLook::ascii(DEATH_FONT, DEATH_HUE),
            };
            if let Some(sprite) = self.art.sprite(&words).ready() {
                let size = Vector::new(sprite.width, sprite.height);
                world.picture(
                    Area::from_center_size(view.center(), size),
                    sprite.uv,
                    Rgba::WHITE,
                );
            }
            buffers.moving = true;
            return;
        }
        world.append(draw.mesh);
        for shape in draw.overlays {
            world.overlay(shape);
        }
        let combat = &self.profile.combat;
        if combat.range_circle {
            let color = hue_color(&self.art, combat.range_circle_hue);
            let projection = self.scene.projection(view);
            world.overlay(overlays::range_diamond(
                &projection,
                combat.range_circle_tiles,
                color,
            ));
        }
        if let Some(measure) = &self.measure {
            let keep_clear = self.scene.character_area(view);
            buffers.plates = lay_out(draw.plates, keep_clear, self.scene.zoom(), measure.as_ref());
        }
    }

    /// The pictures of spells and the weather under the light, and the
    /// light map. True while something of the sky still moves.
    fn lay_sky(
        &mut self,
        frame: &WatchFrame,
        now: f64,
        view: Area,
        world: &mut Shapes,
        buffers: &mut DrawBuffers,
    ) -> bool {
        self.sky.take_in(frame, now);
        let scene = &self.scene;
        let shown = shown_effects(&self.sky, now, |serial| scene.place_of(frame, serial));
        for effect in &shown {
            match *effect {
                ShownEffect::Bolt { struck, born } => {
                    let struck = self.scene.screen_of(view, struck);
                    let bolt = lightning_bolt(view, struck, born);
                    world.line(&bolt, false, LIGHTNING_WIDTH, Rgba::WHITE);
                }
                ShownEffect::Picture {
                    place,
                    graphic,
                    hue,
                } => {
                    let paint = ItemPaint {
                        hue,
                        ..ItemPaint::default()
                    };
                    let picture = self.scene.item_sprite(&mut self.art, graphic, paint, true);
                    if let Some(sprite) = picture.ready() {
                        let foot = self.scene.screen_of(view, place);
                        let zoom = self.scene.zoom();
                        let area = effect_area(foot, sprite.width, sprite.height, zoom);
                        world.picture(area, sprite.uv, Rgba::WHITE);
                    }
                }
            }
        }
        let weather = frame.weather.filter(|_| self.profile.video.weather_effects);
        if let Some((kind, count)) = weather {
            if let Some(tint) = storm_tint(kind) {
                world.fill(view, tint);
            }
            let color = drop_color(kind);
            for drop in weather_drops(view, kind, count, now) {
                match drop {
                    Drop::Flake(middle) => world.disc(middle, SNOW_RADIUS, color),
                    Drop::Streak(tail, head) => world.segment(tail, head, RAIN_WIDTH, color),
                }
            }
        }
        buffers.light = self
            .scene
            .light_cells(&mut self.art, view, frame, &lit_effects(&shown));
        !self.sky.live().is_empty() || weather.is_some()
    }

    /// The words and numbers over heads, laid out for the page to draw.
    fn lay_floats(&mut self, frame: &WatchFrame, now: f64, view: Area) -> Vec<PlacedWords> {
        let art = &self.art;
        self.floats.take_in(
            frame,
            now,
            &self.profile,
            |words, look| art.text_lines(words, look),
            |hue| hue_color(art, hue),
        );
        let live = self.floats.live();
        let heads: Vec<Option<Point>> = live
            .iter()
            .map(|float| self.scene.head_of(view, frame, float.serial))
            .collect();
        let fading = self.profile.general.text_fading;
        floats::lay_out(live, now, fading, |at| heads[at], |_| SPEECH_LINE)
            .into_iter()
            .map(|place| {
                let float = &live[place.index];
                PlacedWords {
                    words: float.words.clone(),
                    x: place.bottom.x,
                    y: place.bottom.y,
                    color: float.color,
                    alpha: place.alpha,
                    number: float.number,
                }
            })
            .collect()
    }

    /// The keys and the clicks of the human on the map: Escape, walking,
    /// a building that waits for its place, the tooltip, and the act of a
    /// click.
    fn act_on_map(
        &mut self,
        frame: &WatchFrame,
        now: f64,
        input: &FrameInput,
        on_map: MapInput<'_>,
        overlay: &mut Shapes,
    ) {
        let MapInput {
            view,
            mouse,
            escape,
            movement,
        } = on_map;
        match escape_on_map(self.hand.aiming().is_some(), frame.target_cursor).filter(|_| escape) {
            Some(EscapeOnMap::CancelAim) => self.hand.cancel_aim(),
            Some(EscapeOnMap::Act(act)) => self.hand.act(act),
            None => {}
        }
        let covered = self.covered.clone();
        let on_map =
            |at: &Point| view.contains(*at) && !covered.iter().any(|area| area.contains(*at));
        // While the ring is open the map takes no input: a click away
        // shuts the ring.
        let ring_open = self.ring_is_open();
        let mouse_on_map = mouse.filter(on_map).filter(|_| !ring_open);
        let character = self
            .scene
            .place_of(frame, frame.serial)
            .map_or(view.center(), |place| self.scene.screen_of(view, place));
        let steer_input = SteerInput {
            shift: self.inputs.mods().shift,
            right_down: self.inputs.secondary_down(),
            right_pressed: input.secondary_pressed,
            left_pressed: input.primary_pressed,
            mouse_way: mouse_on_map.map(|mouse| (character, mouse)),
        };
        let keys_down = self.inputs.keys_down().to_vec();
        for act in self.steer.decide(&keys_down, steer_input, now, movement) {
            self.hand.act(act);
        }
        let Some(mouse) = mouse_on_map else {
            return;
        };
        if self.carries() || self.steer.by_mouse() {
            return;
        }
        // While the designer is open, a click on the house builds with the
        // part the human picked.
        if frame.designing.is_some() {
            self.tooltip = Some(TooltipData {
                lines: vec![self.panels.build.design.hint().to_string()],
                footer: String::new(),
            });
            if let Some(click) = input.clicks.iter().find(|click| on_map(&click.at)) {
                let tile = self.scene.tile_at(&mut self.art, view, frame, click.at);
                self.click_on_house(frame, tile);
            }
            return;
        }
        if let Some(shapes) = self
            .scene
            .placing_preview(&mut self.art, view, frame, mouse)
        {
            for shape in shapes {
                overlay.overlay(shape);
            }
            // The building goes where the click came up.
            if let Some(click) = input.clicks.iter().find(|click| on_map(&click.at)) {
                let (x, y, z) = self.scene.tile_at(&mut self.art, view, frame, click.at);
                self.hand.act(Act::TargetGround { x, y, z });
            }
            return;
        }
        let thing = self.scene.thing_at(mouse).cloned();
        self.tooltip = match (self.hand.aiming(), &thing) {
            (Some(aim), _) => Some(TooltipData {
                lines: vec![uoterm_view::guard::aim_words(aim).to_string()],
                footer: String::new(),
            }),
            (None, Some(thing)) => {
                let hand = &mut self.hand;
                self.tips
                    .rest_on(thing.serial, now, |serial| hand.want_tip(serial));
                Some(TooltipData {
                    lines: self
                        .tips
                        .shown(thing.serial, &thing.name, &[])
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    footer: uoterm_view::clicks::hint_for(frame, thing.kind).to_string(),
                })
            }
            (None, None) => None,
        };
        // A right click opens the ring of the thing it came up on.
        for at in input.menu_clicks.iter().filter(|at| on_map(at)) {
            if let Some(thing) = self.scene.thing_at(*at).cloned() {
                self.open_ring(*at, thing.serial, &thing.name, Subject::OnMap(thing.kind));
                return;
            }
        }
        if self.drag_on_map(frame, mouse) {
            return;
        }
        // A click acts on what lies where its button came up.
        for click in input.clicks.iter().filter(|click| on_map(&click.at)) {
            let picked = self
                .scene
                .thing_at(click.at)
                .map(|thing| (thing.serial, thing.kind));
            let tile = self.scene.tile_at(&mut self.art, view, frame, click.at);
            let ground = GroundClicks::of(&self.profile.general, click.mods);
            if let Some(act) = act_for_click(frame, picked, tile, click.double, ground) {
                if let Act::Use(thing) = act {
                    self.panels.grids.closed.used(thing);
                }
                self.hand.act(act);
            }
        }
    }

    /// A drag the human started on the map: an item goes on the mouse; a
    /// drag from a mobile pulls off its health bar, and one from the ground
    /// draws the box of a drag-select. True when a drag started.
    fn drag_on_map(&mut self, frame: &WatchFrame, mouse: Point) -> bool {
        let Some(from) = self.inputs.drag_start(Some(mouse)) else {
            return false;
        };
        let pressed_on = self.scene.thing_at(from).cloned();
        let hovered = self.scene.thing_at(mouse).cloned();
        let easy_grab = self.profile.general.sallos_easy_grab;
        let grabbed = grabbed(pressed_on, hovered, easy_grab);
        if let Some(item) = grabbed
            .as_ref()
            .filter(|thing| thing.kind == PickKind::Item)
            .and_then(|thing| frame.items.iter().find(|item| item.serial == thing.serial))
        {
            self.pick_up(&WatchPackItem {
                serial: item.serial,
                graphic: item.graphic,
                hue: item.hue,
                amount: item.amount,
                name: item.name.clone(),
                ..WatchPackItem::default()
            });
            return true;
        }
        let mobile = match &grabbed {
            None => None,
            Some(thing) if thing.kind == PickKind::Mobile => Some(thing.serial),
            Some(_) => return false,
        };
        self.panels.bars.map_drag = Some(MapDrag { from, mobile });
        true
    }

    /// The keys of the chat line, by the field that has them, and the
    /// lines it sends.
    fn chat_keys(
        &mut self,
        frame: &WatchFrame,
        focus: Focus,
        unused: &[KeyPress],
        texts: &[String],
    ) {
        let typed = texts.iter().cloned().map(LineKey::Typed);
        let pressed = unused
            .iter()
            .filter_map(|press| match press.key.0.as_str() {
                ESCAPE_KEY => Some(LineKey::Escape),
                ENTER_KEY => Some(LineKey::Enter {
                    shift: press.mods.shift,
                }),
                _ => None,
            });
        let keys: Vec<LineKey> = typed.chain(pressed).collect();
        let speech = &self.profile.speech;
        let out = self.chat.take_keys(focus, &keys, speech);
        // The field of the page takes the keys while the line is open, and
        // lets them go when the line closes.
        if out.closed {
            self.hand.push(OutCall::ChatFocus { take: false });
        } else if focus == Focus::Free && self.chat.is_open(speech) {
            self.hand.push(OutCall::ChatFocus { take: true });
        }
        if focus == Focus::Free && self.chat.take_paste() {
            self.hand.push(OutCall::ChatPaste);
        }
        for words in out.sent {
            match self
                .chat_mode
                .said(&words, asked_commands(frame), frame, speech)
            {
                Some(Said::Act(act)) => self.hand.act(act),
                Some(Said::Note(words)) => self.hand.note(&words),
                None => {}
            }
        }
    }
}

/// Black, for the death screen.
const BLACK: Rgba = Rgba::from_rgb(0, 0, 0);

/// What the map reads of one frame.
struct MapInput<'a> {
    view: Area,
    mouse: Option<Point>,
    /// Escape was pressed and fired nothing else.
    escape: bool,
    movement: &'a Movement,
}
