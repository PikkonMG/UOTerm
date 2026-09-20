//! Optional native watch window. `uoterm watch` opens this. `connect --view`
//! starts the same command in a child process so closing the window does not
//! drop the game socket. `--text-view` prints the radar in the terminal.
//!
//! The window only looks, until the human takes control. Then each click
//! and each line of text is one tool call of the session, marked as made by
//! a human, and the agent waits.

mod atlas;
mod audio;
mod boxes_ui;
mod client_art;
mod control;
mod control_ui;
mod deal_ui;
mod deck_ui;
mod desk;
mod figure;
mod floats;
mod gump_ui;
mod hud;
mod kept;
mod link;
mod login_ui;
mod macros_ui;
mod map_ui;
mod mapitem_ui;
mod options_ui;
mod orders;
mod pages_ui;
mod ring_ui;
mod scene;
mod sky;
mod steer;
mod theme;
mod tips;

use crate::view::{
    WatchFrame, WINDOW_HEIGHT, WINDOW_RADAR_SIZE, WINDOW_RADAR_SIZE_WITH_ART, WINDOW_TITLE,
    WINDOW_WIDTH,
};
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
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
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
}

/// A panel the operator can ask for when the window starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Panel {
    Sheet,
    Map,
    Macros,
    Profile,
}

/// How the window starts, and whether it is one picture only.
#[derive(Clone, Debug, Default)]
pub struct Shown {
    /// Save one picture of the window to this PNG file, then close.
    pub snapshot: Option<PathBuf>,
    pub open: Vec<Panel>,
}

pub use login_ui::{Connect, LoginForm, SavedLogin};

/// What `uoterm play` starts with: the login screens, then the game.
pub struct PlayOptions {
    pub form: LoginForm,
    pub saved: Vec<SavedLogin>,
    pub connect: Connect,
    /// The client files for the real map.
    pub uopath: Option<PathBuf>,
    /// Log in with the form as it is, with no click on Connect.
    pub connect_at_once: bool,
}

/// Opens the login screens. When the character is in the world, the same
/// window becomes the game window, and the human has control.
pub fn play(options: PlayOptions) -> Result<(), String> {
    run(Box::new(move |ctx| {
        Box::new(PlayApp {
            login: login_ui::LoginUi::new(
                options.form,
                options.saved,
                options.connect,
                options.connect_at_once,
            ),
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
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_title(WINDOW_TITLE),
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
    audio: Audio,
    floats: floats::Floats,
    sky: sky::Sky,
    desk: desk::Desk,
    tips: tips::Tips,
    ring: ring_ui::RingUi,
    deck: deck_ui::DeckUi,
    deals: deal_ui::DealUi,
    pages: pages_ui::PagesUi,
    gumps: gump_ui::GumpUi,
    world_map: map_ui::MapUi,
    macros: macros_ui::MacrosUi,
    map_items: mapitem_ui::MapItemUi,
    profiles: mapitem_ui::ProfileUi,
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
        let scene = Scene::new(options.uopath.as_deref());
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
        thread::spawn(move || poll_loop(link, radar_size, tx, ctx));
        Self {
            rx,
            frame: None,
            scene,
            hud: Hud::default(),
            hand,
            control_ui: ControlUi::default(),
            boxes_ui: BoxesUi::default(),
            options_ui: OptionsUi::default(),
            audio,
            floats: floats::Floats::default(),
            sky: sky::Sky::default(),
            desk: desk::Desk::default(),
            tips: tips::Tips::default(),
            ring: ring_ui::RingUi::default(),
            deck: deck_ui::DeckUi::starting(options.shown.open.contains(&Panel::Sheet)),
            deals: deal_ui::DealUi::default(),
            pages: pages_ui::PagesUi::default(),
            gumps: gump_ui::GumpUi::default(),
            map_items: mapitem_ui::MapItemUi::default(),
            profiles: mapitem_ui::ProfileUi::starting(options.shown.open.contains(&Panel::Profile)),
            macros: macros_ui::MacrosUi::starting(options.shown.open.contains(&Panel::Macros)),
            world_map: map_ui::MapUi::starting(options.shown.open.contains(&Panel::Map)),
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
                Ok(next) => self.frame = Some(next),
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive();
        let (time, dt) = ctx.input(|i| (i.time, i.stable_dt));
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
                        moving = self.scene.draw(ui, rect, frame, time);
                        moving |= self.sky.draw(&painter, rect, frame, &mut self.scene, time);
                        moving |= self.floats.draw(&painter, rect, frame, &self.scene, time);
                        self.audio.play(frame, &self.scene.take_steps());
                        let drawn = self.hud.draw(&painter, rect, frame, true, time, dt);
                        moving |= drawn.moving;
                        let map = control_ui::map_sense(ui, rect);
                        self.hud.journal_filters(ui);
                        self.desk.begin();
                        self.tips.begin(&self.hand, time);
                        let mut tools = boxes_ui::Tools {
                            scene: &mut self.scene,
                            hand: &self.hand,
                            desk: &mut self.desk,
                            tips: &mut self.tips,
                            ring: &mut self.ring,
                            time,
                        };
                        let gumps_as_lists = !tools.scene.has_gump_art();
                        let mut covered =
                            self.boxes_ui
                                .draw(ui, rect, frame, &mut tools, gumps_as_lists);
                        if !gumps_as_lists {
                            covered.extend(self.gumps.draw(ui, rect, frame, &mut tools));
                        }
                        let notes = usize::from(!self.audio.note().is_empty());
                        covered.extend(self.options_ui.panel(rect, notes));
                        covered.extend(self.deck.draw(ui, rect, frame, &mut tools, drawn.pack));
                        covered.extend(self.deals.draw(ui, rect, frame, &mut tools));
                        covered.extend(self.pages.draw(ui, rect, frame, &mut tools));
                        covered.extend(self.map_items.draw(ui, rect, frame, &mut tools));
                        covered.extend(self.profiles.draw(ui, rect, frame, &mut tools));
                        covered.extend(self.world_map.draw(
                            ui,
                            rect,
                            frame,
                            tools.scene,
                            tools.hand,
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
                        covered.extend(tools.desk.split_box(ui, rect, tools.hand));
                        let places = Places {
                            map: &map,
                            chat_row: drawn.chat_row,
                            covered: &covered,
                            boxes: &mut self.boxes_ui,
                            options: &mut self.options_ui,
                            deck: &mut self.deck,
                            world_map: &mut self.world_map,
                            macros: &mut self.macros,
                            profiles: &mut self.profiles,
                        };
                        self.control_ui
                            .draw(ui, rect, frame, &self.hud, &mut tools, places);
                        self.scene.set_panels(covered);
                        self.options_ui.draw(ui, rect, &mut self.audio);
                    }
                }
            });
        moving |= self.take_snapshot(ctx);
        if moving {
            ctx.request_repaint();
        }
    }
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
