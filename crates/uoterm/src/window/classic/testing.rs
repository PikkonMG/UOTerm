//! Draws the gump manager for a frame in a window with no screen, with the
//! real client files, for the tests of the gumps.

use super::canvas::{Canvas, CanvasInput};
use super::manager::{GumpManager, ManagerInputs};
use super::text::{TextKit, UoFonts};
use crate::view::WatchFrame;
use crate::window::control::Hand;
use crate::window::desk::Desk;
use crate::window::keys::Focus;
use crate::window::link::Link;
use crate::window::model::journal::JournalLog;
use crate::window::model::reads::Readings;
use crate::window::scene::Scene;
use crate::window::settings::Profile;
use crate::window::tips::Tips;
use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};

/// An address no session answers; the tests send no acts.
const NO_API: &str = "http://127.0.0.1:1";
pub const SCREEN: Vec2 = Vec2::new(1280.0, 800.0);
/// The side of the largest texture the headless window takes, as large as
/// the texture the pictures go into.
const MOST_TEXTURE_SIDE: usize = 8192;
/// Two frames: the first lays the gumps out, the second draws them where
/// the first put them.
const FRAMES: usize = 2;
const TEST_CANVAS_ID: &str = "test-canvas";

/// The pictures and the fonts of the client files, or none when the files
/// are not there.
fn client_kit() -> Option<(Scene, TextKit)> {
    let dir = uoterm_nav::client_data_dir_from_env()?;
    let scene = Scene::new(Some(&dir));
    let text = TextKit::new(UoFonts::open(&dir).expect("the fonts of the client"), false);
    Some((scene, text))
}

/// The input of each frame of a test: its events, and a texture side as
/// large as the atlas.
fn frame_input(events: &[egui::Event]) -> RawInput {
    RawInput {
        events: events.to_vec(),
        max_texture_side: Some(MOST_TEXTURE_SIDE),
        ..RawInput::default()
    }
}

/// Draws on a canvas at the top left corner of the screen once for each
/// list of input events, with the client files. False when the files are
/// not there and the test has nothing to draw with.
pub fn draw_canvas(
    frames: &[Vec<egui::Event>],
    mut draw: impl FnMut(&mut Canvas<'_>, &egui::Context),
) -> bool {
    let Some((mut scene, mut text)) = client_kit() else {
        return false;
    };
    let ctx = egui::Context::default();
    for events in frames {
        let _ = ctx.run(frame_input(events), |ctx| {
            scene.make_atlas(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                let input = CanvasInput {
                    id: egui::Id::new(TEST_CANVAS_ID),
                    origin: Pos2::ZERO,
                    scale: 1.0,
                    alpha: 1.0,
                    pointer: None,
                    body_click: None,
                    body_double_click: false,
                    right_click: false,
                    size: None,
                    map: 0,
                };
                let mut g = Canvas::new(ui, &mut scene, &mut text, input);
                draw(&mut g, ctx);
            });
        });
    }
    true
}

/// A hand whose acts reach no session, for the tests of what acts.
pub fn idle_hand() -> Hand {
    let link = Link::Http {
        api: NO_API.into(),
        session: String::new(),
    };
    Hand::start(link, egui::Context::default())
}

/// Draws every open gump of `manager` for `frame`. False when the client
/// files are not there and the test has nothing to draw with.
pub fn draw_frames(manager: &mut GumpManager, profile: &mut Profile, frame: &WatchFrame) -> bool {
    let quiet = vec![Vec::new(); FRAMES];
    draw_with_input(manager, profile, frame, &mut Desk::default(), &quiet)
}

/// Draws every open gump of `manager` for `frame` once for each list of
/// input events, as the frames of a player who moves the mouse and clicks,
/// with the desk the test keeps. False when the client files are not there.
pub fn draw_with_input(
    manager: &mut GumpManager,
    profile: &mut Profile,
    frame: &WatchFrame,
    desk: &mut Desk,
    frames: &[Vec<egui::Event>],
) -> bool {
    draw_in(
        &egui::Context::default(),
        manager,
        profile,
        frame,
        desk,
        frames,
    )
}

/// Where the keys go after every open gump of `manager` drew for `frame`.
/// None when the client files are not there.
pub fn focus_after_drawing(
    manager: &mut GumpManager,
    profile: &mut Profile,
    frame: &WatchFrame,
) -> Option<Focus> {
    let ctx = egui::Context::default();
    let quiet = vec![Vec::new(); FRAMES];
    draw_in(&ctx, manager, profile, frame, &mut Desk::default(), &quiet)
        .then(|| Focus::of(&ctx, egui::Id::NULL, true))
}

fn draw_in(
    ctx: &egui::Context,
    manager: &mut GumpManager,
    profile: &mut Profile,
    frame: &WatchFrame,
    desk: &mut Desk,
    frames: &[Vec<egui::Event>],
) -> bool {
    let Some((mut scene, mut text)) = client_kit() else {
        return false;
    };
    let link = Link::Http {
        api: NO_API.into(),
        session: String::new(),
    };
    let mut readings = Readings::start(link.clone());
    let hand = Hand::start(link, ctx.clone());
    let journal = JournalLog::default();
    let mut tips = Tips::default();
    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);
    for events in frames {
        desk.begin();
        let input = RawInput {
            screen_rect: Some(screen),
            ..frame_input(events)
        };
        let _ = ctx.run(input, |ctx| {
            scene.make_atlas(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                manager.draw(
                    ui,
                    screen,
                    ManagerInputs {
                        frame,
                        scene: &mut scene,
                        text: &mut text,
                        hand: &hand,
                        tips: &mut tips,
                        profile,
                        desk: &mut *desk,
                        journal: &journal,
                        readings: &mut readings,
                        time: 0.0,
                        sound_note: "",
                        shared_only: false,
                    },
                );
            });
        });
    }
    true
}
