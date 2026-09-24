//! Optional native watch window. `uoterm watch` opens this. `connect --view`
//! starts the same command in a child process so closing the window does not
//! drop the game socket. `--text-view` prints the radar in the terminal.
//!
//! The window only looks, until the human takes control. Then each click
//! and each line of text is one tool call of the session, marked as made by
//! a human, and the agent waits.

mod actions;
mod atlas;
mod audio;
mod boxes_ui;
mod build_ui;
mod classic;
mod client_art;
mod control;
mod control_ui;
mod creation_ui;
mod cursor;
mod deal_ui;
mod deck_ui;
mod desk;
mod figure;
mod filters;
mod floats;
mod gump_ui;
mod hud;
mod kept;
mod keys;
mod lights;
mod link;
mod login_ui;
mod look;
mod macros_ui;
mod map_ui;
mod map_view;
mod mapitem_ui;
mod model;
mod modern;
mod options_ui;
mod orders;
mod pad;
mod pages_ui;
mod predict;
mod ring_ui;
mod scene;
mod settings;
mod sky;
mod steer;
mod theme;
mod tips;
mod video;

use crate::view::{WatchFrame, WINDOW_RADAR_SIZE, WINDOW_RADAR_SIZE_WITH_ART, WINDOW_TITLE};
use audio::Audio;
use boxes_ui::BoxesUi;
use control::Hand;
use control_ui::{ControlUi, Places};
use eframe::egui::{self, ColorImage, Event, UserData, ViewportCommand};
use hud::Hud;
pub use link::Link;
use options_ui::OptionsUi;
use scene::Scene;
use serde_json::json;
use settings::{Profile, ProfileHome};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use uoterm_protocol::ClientVersion;
use uoterm_runtime::tools::TOOL_WATCH;

const WINDOW_MIN_WIDTH: f32 = 1080.0;
const WINDOW_MIN_HEIGHT: f32 = 640.0;
/// A snapshot waits this long after the first picture, so the bars and the
/// camera are at rest in it.
const SNAPSHOT_SETTLE: Duration = Duration::from_millis(2500);
const ENDED: &str = "The watch ended.";

pub struct WatchOptions {
    /// How to reach the session.
    pub link: Link,
    /// The client files for the real map. None draws the map in flat colors.
    pub uopath: Option<PathBuf>,
    pub shown: Shown,
    /// The login server's address, from [`shard_address`]. Each character
    /// of a known shard has its own profile. None keeps the global profile.
    pub shard: Option<String>,
}

/// A panel the operator can ask for when the window starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Panel {
    Sheet,
    Map,
    Macros,
    Profile,
    Chat,
}

/// How the window starts, and whether it is one picture only.
#[derive(Clone, Debug, Default)]
pub struct Shown {
    /// Save one picture of the window to this PNG file, then close.
    pub snapshot: Option<PathBuf>,
    pub open: Vec<Panel>,
}

pub use login_ui::{Connect, LoginForm, SavedLogin};
pub use settings::shard_address;

/// What `uoterm play` starts with: the login screens, then the game.
pub struct PlayOptions {
    pub form: LoginForm,
    pub saved: Vec<SavedLogin>,
    pub connect: Connect,
    /// The client files for the real map.
    pub uopath: Option<PathBuf>,
    /// Log in with the form as it is, with no click on Connect.
    pub connect_at_once: bool,
    /// The client version of a login with no saved login, which a new
    /// character follows.
    pub version: ClientVersion,
}

/// Opens the login screens. When the character is in the world, the same
/// window becomes the game window.
pub fn play(options: PlayOptions) -> Result<(), String> {
    run(Box::new(move |ctx| {
        Box::new(PlayApp {
            login: login_ui::LoginUi::new(login_ui::LoginStart {
                form: options.form,
                saved: options.saved,
                connect: options.connect,
                connect_at_once: options.connect_at_once,
                version: options.version,
                uopath: options.uopath.as_deref(),
            }),
            uopath: options.uopath,
            game: None,
            ctx: ctx.clone(),
        })
    }))
}

/// The login screens, and after them the game window.
struct PlayApp {
    login: login_ui::LoginUi,
    uopath: Option<PathBuf>,
    game: Option<WatchApp>,
    ctx: egui::Context,
}

impl eframe::App for PlayApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if let Some(game) = self.game.as_mut() {
            game.raw_input_hook(ctx, raw_input);
        }
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if let Some(game) = self.game.as_mut() {
            game.on_exit(gl);
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(game) = self.game.as_mut() {
            return game.update(ctx, frame);
        }
        let mut link = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| link = self.login.draw(ui, ui.max_rect()));
        if let Some(link) = link {
            let options = WatchOptions {
                link,
                uopath: self.uopath.clone(),
                shown: Shown::default(),
                shard: Some(self.login.shard_address()),
            };
            let game = WatchApp::start(options, self.ctx.clone());
            // The one who logged in is a human. He has the character.
            game.hand.act(control::Act::Take);
            self.game = Some(game);
        }
    }
}

pub fn open(options: WatchOptions) -> Result<(), String> {
    run(Box::new(move |ctx| {
        Box::new(WatchApp::start(options, ctx.clone()))
    }))
}

type MakeApp = Box<dyn FnOnce(&egui::Context) -> Box<dyn eframe::App>>;

fn run(make: MakeApp) -> Result<(), String> {
    // The window opens as the global profile's Video page says.
    let opening = ProfileHome::new(None).first_profile().video;
    let native = eframe::NativeOptions {
        viewport: video::opening_viewport(
            egui::ViewportBuilder::default()
                .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
                .with_title(WINDOW_TITLE),
            &opening,
        ),
        vsync: opening.vsync,
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        native,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Ok(make(&cc.egui_ctx))
        }),
    )
    .map_err(|e| e.to_string())
}

struct WatchApp {
    rx: Receiver<WatchFrame>,
    /// None until the session sends the first picture.
    frame: Option<WatchFrame>,
    scene: Scene,
    hud: Hud,
    hand: Hand,
    control_ui: ControlUi,
    boxes_ui: BoxesUi,
    options_ui: OptionsUi,
    /// Every option of the player. It is the global profile until the
    /// first frame names the character.
    profile: Profile,
    /// The Video page as the window has put it into effect.
    video: video::Video,
    /// The last choice about public house content the shard was told.
    house_content_sent: Option<bool>,
    profile_home: ProfileHome,
    audio: Audio,
    floats: floats::Floats,
    sky: sky::Sky,
    desk: desk::Desk,
    tips: tips::Tips,
    ring: ring_ui::RingUi,
    deck: deck_ui::DeckUi,
    deals: deal_ui::DealUi,
    pages: pages_ui::PagesUi,
    /// The floating gumps: every window of the Classic style, and the
    /// gumps of the shard in both styles.
    classic: classic::ClassicUi,
    world_map: map_ui::MapUi,
    /// The extras of the Modern style, and what they read of the session.
    modern: modern::ModernUi,
    readings: model::reads::Readings,
    /// The journal file of the Speech page, in both styles.
    journal_file: model::journal::JournalFile,
    layers: model::compare::ItemLayers,
    macros: macros_ui::MacrosUi,
    map_items: mapitem_ui::MapItemUi,
    build: build_ui::BuildUi,
    chat: build_ui::ChatUi,
    profiles: mapitem_ui::ProfileUi,
    /// The keys, the controller and the macros, the same in each style.
    controls: actions::client::Controls,
    snapshot: Option<Snapshot>,
}

struct Snapshot {
    path: PathBuf,
    first_picture: Option<Instant>,
    asked: bool,
}

impl WatchApp {
    fn start(options: WatchOptions, ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let link = options.link;
        let mut scene = Scene::new(options.uopath.as_deref());
        scene.set_poll_every(Duration::from_millis(link.poll_ms()));
        // A snapshot is one silent picture.
        let audio = Audio::new(
            options
                .uopath
                .as_deref()
                .filter(|_| options.shown.snapshot.is_none()),
        );
        let radar_size = if scene.note().is_empty() {
            WINDOW_RADAR_SIZE_WITH_ART
        } else {
            WINDOW_RADAR_SIZE
        };
        let hand = Hand::start(link.clone(), ctx.clone());
        let readings = model::reads::Readings::start(link.clone());
        let layers = model::compare::ItemLayers::load(options.uopath.as_deref());
        thread::spawn(move || poll_loop(link, radar_size, tx, ctx));
        let profile_home = ProfileHome::new(options.shard);
        let profile = profile_home.first_profile();
        scene.set_zoom(profile.video.default_zoom);
        let video = video::Video::opened_as(&profile.video);
        Self {
            video,
            house_content_sent: None,
            journal_file: model::journal::JournalFile::default(),
            rx,
            frame: None,
            scene,
            hud: Hud::default(),
            hand,
            control_ui: ControlUi::default(),
            boxes_ui: BoxesUi::default(),
            options_ui: OptionsUi::default(),
            profile,
            profile_home,
            audio,
            floats: floats::Floats::default(),
            sky: sky::Sky::default(),
            desk: desk::Desk::default(),
            tips: tips::Tips::default(),
            ring: ring_ui::RingUi::default(),
            deck: deck_ui::DeckUi::starting(options.shown.open.contains(&Panel::Sheet)),
            deals: deal_ui::DealUi::default(),
            pages: pages_ui::PagesUi::default(),
            classic: classic::ClassicUi::new(options.uopath.as_deref()),
            map_items: mapitem_ui::MapItemUi::default(),
            build: build_ui::BuildUi::default(),
            chat: build_ui::ChatUi::starting(options.shown.open.contains(&Panel::Chat)),
            profiles: mapitem_ui::ProfileUi::starting(options.shown.open.contains(&Panel::Profile)),
            macros: macros_ui::MacrosUi::starting(options.shown.open.contains(&Panel::Macros)),
            world_map: map_ui::MapUi::starting(options.shown.open.contains(&Panel::Map)),
            modern: modern::ModernUi::default(),
            readings,
            layers,
            controls: actions::client::Controls::default(),
            snapshot: options.shown.snapshot.map(|path| Snapshot {
                path,
                first_picture: None,
                asked: false,
            }),
        }
    }

    fn receive(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(mut next) => {
                    if let Some(own) = self.profile_home.follow(&next.name) {
                        self.profile = own;
                        self.classic.profile_replaced();
                        self.audio.options_changed(&self.profile.sound);
                        self.scene.set_zoom(self.profile.video.default_zoom);
                    }
                    self.controls.received(&mut next);
                    self.frame = Some(next);
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.frame = Some(WatchFrame::error_frame(ENDED));
                    return;
                }
            }
        }
    }

    /// Asks for the snapshot when the window is at rest, and saves it when
    /// it comes. True while the snapshot still needs the next frame.
    fn take_snapshot(&mut self, ctx: &egui::Context) -> bool {
        let Some(snapshot) = self.snapshot.as_mut() else {
            return false;
        };
        let has_picture = self.frame.as_ref().is_some_and(|f| f.error.is_empty());
        if has_picture && snapshot.first_picture.is_none() {
            snapshot.first_picture = Some(Instant::now());
        }
        let settled = snapshot
            .first_picture
            .is_some_and(|at| at.elapsed() >= SNAPSHOT_SETTLE);
        if settled && !snapshot.asked {
            snapshot.asked = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                Event::Screenshot { image, .. } => Some(Arc::clone(image)),
                _ => None,
            })
        });
        if let Some(image) = image {
            if let Err(e) = save_png(&snapshot.path, &image) {
                tracing::error!(error = %e, "snapshot");
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        true
    }
}

fn save_png(path: &std::path::Path, picture: &ColorImage) -> Result<(), String> {
    image::save_buffer(
        path,
        picture.as_raw(),
        picture.width() as u32,
        picture.height() as u32,
        image::ColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}

impl eframe::App for WatchApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.controls.raw_input(ctx, raw_input, &self.profile);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive();
        self.modern.apply_look(ctx, &self.profile);
        let (time, dt) = ctx.input(|i| (i.time, i.stable_dt));
        let focused = window_focused(ctx);
        let mut moving = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);
                match &self.frame {
                    None => hud::message(
                        &painter,
                        rect,
                        "Waiting for the session",
                        &["The first picture comes in a moment."],
                        theme::TEXT,
                    ),
                    Some(frame) if !frame.error.is_empty() => hud::message(
                        &painter,
                        rect,
                        "No picture from the session",
                        &[&frame.error, "The window tries again by itself."],
                        theme::ALARM,
                    ),
                    Some(frame) => {
                        // The Classic look needs the gump art and the fonts
                        // of the client; without them the window keeps the
                        // Modern look.
                        let classic_style = self.profile.interface.ui_style
                            == settings::UiStyle::Classic
                            && self.classic.ready(&self.scene);
                        // The Classic style draws the world inside its
                        // game window, on black.
                        let view = if classic_style {
                            classic::viewport::view_rect(&self.profile.video, rect)
                        } else {
                            rect
                        };
                        if view != rect {
                            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
                        }
                        let view_painter = ui.painter_at(view);
                        self.ring.set_classic(classic_style);
                        let mut env = actions::client::FrameEnv {
                            ctx: ui.ctx(),
                            frame,
                            profile: &mut self.profile,
                            home: &self.profile_home,
                            hand: &self.hand,
                            scene: &mut self.scene,
                            view,
                            time,
                            chat_id: control_ui::chat_id(),
                            chat_empty: self.control_ui.chat_empty(),
                            keys_paused: self.options_ui.capturing() || self.classic.wants_keys(),
                        };
                        let controls = self.controls.frame(&mut env);
                        // The buttons of the control bar give commands too,
                        // such as Quit in the Classic style.
                        let from_buttons = self.control_ui.take_window_commands();
                        for command in controls.style.iter().chain(&from_buttons) {
                            let done = if classic_style {
                                let mut windows = classic::windows::ClassicWindows {
                                    frame,
                                    hand: &self.hand,
                                    manager: self.classic.manager(),
                                    profile: &mut self.profile,
                                    control: &mut self.control_ui,
                                };
                                actions::StyleWindows::command(&mut windows, command)
                            } else {
                                let mut modern = actions::modern::ModernWindows {
                                    frame,
                                    hand: &self.hand,
                                    profile: &mut self.profile,
                                    home: &self.profile_home,
                                    modern: &mut self.modern,
                                    options: &mut self.options_ui,
                                    deck: &mut self.deck,
                                    world_map: &mut self.world_map,
                                    macros: &mut self.macros,
                                    chat: &mut self.chat,
                                    profiles: &mut self.profiles,
                                    boxes: &mut self.boxes_ui,
                                    control: &mut self.control_ui,
                                    pages: &mut self.pages,
                                };
                                actions::StyleWindows::command(&mut modern, command)
                            };
                            if !done {
                                self.controls.style_cannot(frame, command);
                            }
                        }
                        if let Some(older) = controls.history_older {
                            self.control_ui.history(older);
                        }
                        self.scene
                            .set_storey_looks(self.build.design().borrow().storeys);
                        moving = self.scene.draw(ui, view, frame, time, &self.profile);
                        let combat = &self.profile.combat;
                        if combat.range_circle {
                            let color = self.scene.words_color(combat.range_circle_hue);
                            self.scene.draw_range_circle(
                                &view_painter,
                                view,
                                combat.range_circle_tiles,
                                color,
                            );
                        }
                        // The classic client asks for the name of each
                        // mobile and corpse as it comes into view.
                        for serial in self.scene.take_arrivals() {
                            if frame.human_control {
                                self.hand.act(control::Act::Look(serial));
                            }
                        }
                        // The shard learns whether to show what stands in
                        // public houses when a human plays, and each time
                        // the General page changes it.
                        let house_content = self.profile.general.show_house_content;
                        if frame.human_control && self.house_content_sent != Some(house_content) {
                            self.hand.act(control::Act::HouseContent(house_content));
                            self.house_content_sent = Some(house_content);
                        }
                        moving |= self.sky.draw(
                            &view_painter,
                            view,
                            frame,
                            &mut self.scene,
                            time,
                            &self.profile.video,
                        );
                        moving |= self.floats.draw(
                            &view_painter,
                            view,
                            frame,
                            &mut self.scene,
                            time,
                            &self.profile,
                        );
                        self.audio.play(
                            frame,
                            &self.scene.take_steps(),
                            &self.profile.sound,
                            focused,
                        );
                        let drawn = (!classic_style).then(|| {
                            self.hud
                                .draw(&painter, rect, frame, (time, dt), &self.profile)
                        });
                        // The arrow goes over the panels, as a compass
                        // needle does: it must never be hidden. The Classic
                        // style draws the arrow of the classic client, which
                        // the player clicks.
                        let modern_arrow = if classic_style {
                            classic::quest_arrow::draw(
                                ui.ctx(),
                                view,
                                frame,
                                &mut self.scene,
                                &self.hand,
                                self.profile.video.ui_scale,
                            );
                            None
                        } else {
                            self.scene.draw_quest_arrow(&view_painter, view, frame)
                        };
                        self.journal_file.take(frame, &self.profile.speech);
                        moving |= drawn.as_ref().is_some_and(|drawn| drawn.moving);
                        let map = control_ui::map_sense(ui, view);
                        // The arrow takes its clicks over the map.
                        let arrow_area = modern_arrow
                            .map(|arrow| modern::arrow_ui::click(ui, arrow, frame, &self.hand));
                        if classic_style
                            && self.classic.frame_view(
                                ui,
                                view,
                                rect,
                                &mut self.scene,
                                &mut self.profile.video,
                            )
                        {
                            self.profile_home
                                .save(&model::places::for_saving(&self.profile));
                        }
                        // The Classic style has its chat line at the foot
                        // of its game window; the Modern one under its
                        // journal.
                        if classic_style {
                            moving |= self.classic.chat_line(
                                ui,
                                view,
                                &mut self.scene,
                                self.control_ui.chat_line(),
                                classic::chat_line::ChatInputs {
                                    frame,
                                    profile: &self.profile,
                                    hand: &self.hand,
                                    time,
                                },
                            );
                        }
                        self.desk.begin();
                        self.tips.begin(&self.hand, time);
                        self.readings.begin(time);
                        let gumps_ready = self.classic.ready(&self.scene);
                        let mut covered: Vec<egui::Rect> = arrow_area.into_iter().collect();
                        if gumps_ready {
                            let layer = self.classic.draw(
                                ui,
                                rect,
                                classic::ClassicInputs {
                                    frame,
                                    scene: &mut self.scene,
                                    hand: &self.hand,
                                    tips: &mut self.tips,
                                    profile: &mut self.profile,
                                    desk: &mut self.desk,
                                    readings: &mut self.readings,
                                    time,
                                    sound_note: self.audio.note(),
                                    classic: classic_style,
                                },
                            );
                            for steps in layer.macros {
                                self.controls.run_macro(steps);
                            }
                            for sound in layer.sounds {
                                self.audio.play_effect(sound, &self.profile.sound);
                            }
                            let kept = model::places::for_saving(&self.profile);
                            if layer.profile_changed {
                                self.profile_home.save(&kept);
                                self.audio.options_changed(&self.profile.sound);
                            }
                            if layer.save_as_default {
                                self.profile_home.save_as_default(&kept);
                            }
                            covered.extend(layer.covered);
                        }
                        let mut tools = boxes_ui::Tools {
                            scene: &mut self.scene,
                            hand: &self.hand,
                            desk: &mut self.desk,
                            tips: &mut self.tips,
                            ring: &mut self.ring,
                            time,
                            profile_home: &self.profile_home,
                            readings: &mut self.readings,
                            layers: &self.layers,
                        };
                        let mut chat_row = None;
                        if let Some(drawn) = drawn {
                            // The panels on the map lie under the others, so
                            // the others take their clicks first. The
                            // containers and the gumps the shard opens lie
                            // over the Modern panels that stay open.
                            self.hud.controls(ui, &tools, &mut self.profile);
                            let modern =
                                self.modern
                                    .draw(ui, rect, frame, &mut tools, &mut self.profile);
                            covered.extend(modern.covered);
                            chat_row = Some(modern.chat_row);
                            covered.extend(self.boxes_ui.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                                !gumps_ready,
                            ));
                            covered.extend(self.deck.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                                drawn.pack,
                            ));
                            for steps in self.deck.take_macros() {
                                self.controls.run_macro(steps);
                            }
                        }
                        // The Classic style shows the shop and the trades
                        // as gumps.
                        if !classic_style {
                            covered.extend(self.deals.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                            ));
                        }
                        covered.extend(self.pages.draw(
                            ui,
                            rect,
                            frame,
                            &mut tools,
                            &mut self.profile,
                            classic_style,
                            gumps_ready,
                        ));
                        // The Classic style shows the map items and the chat
                        // as classic gumps.
                        if !classic_style {
                            covered.extend(self.map_items.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                            ));
                        }
                        // The Classic style shows the house designer as
                        // its gump.
                        if classic_style {
                            classic::house::follow(
                                self.classic.manager(),
                                frame,
                                &mut self.profile,
                                &self.build.design(),
                            );
                        } else {
                            covered.extend(self.build.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                            ));
                        }
                        if !classic_style {
                            covered.extend(self.chat.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                            ));
                        }
                        covered.extend(self.profiles.draw(
                            ui,
                            rect,
                            frame,
                            &mut tools,
                            &mut self.profile,
                        ));
                        covered.extend(self.world_map.draw(
                            ui,
                            rect,
                            frame,
                            &mut tools,
                            &mut self.profile,
                        ));
                        covered.extend(self.macros.draw(
                            ui,
                            rect,
                            frame,
                            &mut tools,
                            &mut self.deck,
                        ));
                        covered.extend(tools.ring.draw(
                            ui,
                            rect,
                            frame,
                            tools.hand,
                            &mut self.profiles,
                        ));
                        covered.extend(desk::split_box(ui, rect, &mut tools, &mut self.profile));
                        // The Modern Options stand over the other panels.
                        if !classic_style {
                            covered.extend(self.options_ui.draw(
                                ui,
                                rect,
                                frame,
                                &mut tools,
                                &mut self.profile,
                                &self.audio,
                            ));
                        }
                        let places = Places {
                            map: &map,
                            chat_row,
                            covered: &covered,
                            boxes: &mut self.boxes_ui,
                            options: &mut self.options_ui,
                            deck: &mut self.deck,
                            world_map: &mut self.world_map,
                            macros: &mut self.macros,
                            profiles: &mut self.profiles,
                            chat: &mut self.chat,
                            build: &self.build,
                            modern: &mut self.modern,
                            profile: &self.profile,
                            movement: &controls.movement,
                            classic: classic_style,
                            view,
                        };
                        self.control_ui
                            .draw(ui, rect, frame, &self.hud, &mut tools, places);
                        if let Some((place, serial)) = self.ring.take_classic_ask() {
                            self.classic.open_popup(place, serial, &mut self.profile);
                        }
                        self.scene.set_panels(covered);
                        if self.profile.interface.ui_style == settings::UiStyle::Classic {
                            cursor::draw(ui.ctx(), &mut self.scene, frame, view);
                        }
                        cursor::draw_targeting(ui.ctx(), &self.scene, frame, view, &self.profile);
                        // The Options button of the control bar opens the
                        // Options gump in the Classic style.
                        if classic_style && self.options_ui.is_open() {
                            self.options_ui.toggle();
                            self.classic.toggle(
                                classic::registry::GumpId::one(
                                    classic::registry::well_known::OPTIONS,
                                ),
                                &mut self.profile,
                            );
                        }
                    }
                }
            });
        moving |= self.take_snapshot(ctx);
        self.video.apply(ctx, &self.profile.video);
        if moving {
            ctx.request_repaint_after(video::frame_interval(&self.profile.video, focused));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.profile.video.keep_zoom_after_close {
            self.profile.video.default_zoom = self.scene.zoom();
            self.profile_home.save(&self.profile);
        }
    }
}

/// True while the window has the keyboard. A system that does not tell
/// counts as focused.
fn window_focused(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().focused.unwrap_or(true))
}

/// Reads the session again and again, and wakes the window for each picture.
fn poll_loop(link: Link, radar_size: u16, tx: mpsc::Sender<WatchFrame>, ctx: egui::Context) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            let _ = tx.send(WatchFrame::error_frame(e.to_string()));
            return;
        }
    };
    loop {
        let frame = rt.block_on(async {
            match link.call(TOOL_WATCH, json!({ "size": radar_size })).await {
                Ok(value) => WatchFrame::from_observe(&value),
                Err(words) => WatchFrame::error_frame(words),
            }
        });
        if tx.send(frame).is_err() {
            break;
        }
        ctx.request_repaint();
        thread::sleep(Duration::from_millis(link.poll_ms()));
    }
}
