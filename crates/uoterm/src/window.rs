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
mod figure;
mod hud;
mod options_ui;
mod orders;
mod scene;
mod theme;

use crate::remote;
use crate::view::{
    WatchFrame, WINDOW_HEIGHT, WINDOW_POLL_MS, WINDOW_RADAR_SIZE, WINDOW_RADAR_SIZE_WITH_ART,
    WINDOW_TITLE, WINDOW_WIDTH,
};
use audio::Audio;
use boxes_ui::BoxesUi;
use control::Hand;
use control_ui::{ControlUi, Places};
use eframe::egui::{self, ColorImage, Event, UserData, ViewportCommand};
use hud::Hud;
use options_ui::OptionsUi;
use scene::Scene;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const WINDOW_MIN_WIDTH: f32 = 1080.0;
const WINDOW_MIN_HEIGHT: f32 = 640.0;
/// A snapshot waits this long after the first picture, so the bars and the
/// camera are at rest in it.
const SNAPSHOT_SETTLE: Duration = Duration::from_millis(2500);
const ENDED: &str = "The watch ended.";

pub struct WatchOptions {
    pub api: String,
    pub session: String,
    /// The client files for the real map. None draws the map in flat colors.
    pub uopath: Option<PathBuf>,
    /// Save one picture of the window to this PNG file, then close.
    pub snapshot: Option<PathBuf>,
}

pub fn open(options: WatchOptions) -> Result<(), String> {
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
            Ok(Box::new(WatchApp::start(options, cc.egui_ctx.clone())))
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
        let (api, session) = (options.api, options.session);
        let scene = Scene::new(options.uopath.as_deref());
        // A snapshot is one silent picture.
        let audio = Audio::new(
            options
                .uopath
                .as_deref()
                .filter(|_| options.snapshot.is_none()),
        );
        let radar_size = if scene.note().is_empty() {
            WINDOW_RADAR_SIZE_WITH_ART
        } else {
            WINDOW_RADAR_SIZE
        };
        let hand = Hand::start(api.clone(), session.clone(), ctx.clone());
        thread::spawn(move || poll_loop(api, session, radar_size, tx, ctx));
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
            snapshot: options.snapshot.map(|path| Snapshot {
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
                        self.audio.play(frame, &self.scene.take_steps());
                        let drawn = self.hud.draw(&painter, rect, frame, true, time, dt);
                        moving |= drawn.moving;
                        let map = control_ui::map_sense(ui, rect);
                        let mut covered =
                            self.boxes_ui
                                .draw(ui, rect, frame, &mut self.scene, &self.hand);
                        let notes = usize::from(!self.audio.note().is_empty());
                        covered.extend(self.options_ui.panel(rect, notes));
                        let places = Places {
                            map: &map,
                            chat_row: drawn.chat_row,
                            covered: &covered,
                            boxes: &mut self.boxes_ui,
                            options: &mut self.options_ui,
                        };
                        self.control_ui.draw(
                            ui,
                            rect,
                            frame,
                            &mut self.scene,
                            &self.hud,
                            &self.hand,
                            places,
                            time,
                        );
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
fn poll_loop(
    api: String,
    session: String,
    radar_size: u16,
    tx: mpsc::Sender<WatchFrame>,
    ctx: egui::Context,
) {
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
            match remote::session_observe(&api, &session, radar_size).await {
                Ok(value) => WatchFrame::from_observe(&value),
                Err(e) => WatchFrame::error_frame(e.to_string()),
            }
        });
        if tx.send(frame).is_err() {
            break;
        }
        ctx.request_repaint();
        thread::sleep(Duration::from_millis(WINDOW_POLL_MS));
    }
}
