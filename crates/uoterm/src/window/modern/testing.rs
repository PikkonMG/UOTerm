//! Draws Modern panels in a window with no screen and no client files, for
//! the tests of what they show and keep. The acts they send reach no
//! session.

use super::super::boxes_ui::Tools;
use super::super::classic::testing::idle_hand;
use super::super::desk::Desk;
use super::super::link::Link;
use super::super::model::compare::ItemLayers;
use super::super::model::reads::Readings;
use super::super::ring_ui::RingUi;
use super::super::scene::Scene;
use super::super::settings::{Profile, ProfileHome};
use super::super::tips::Tips;
use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};

/// An address no session answers.
const NO_API: &str = "http://127.0.0.1:1";
pub const SCREEN: Vec2 = Vec2::new(1280.0, 800.0);

/// Draws a panel once for each list of input events, as the frames of a
/// player who moves the mouse, clicks and types. `draw` gets the screen
/// and the tools of the window.
pub fn draw_frames(
    profile: &mut Profile,
    frames: &[Vec<egui::Event>],
    mut draw: impl FnMut(&mut egui::Ui, Rect, &mut Tools<'_>, &mut Profile),
) {
    let ctx = egui::Context::default();
    // The panels write their titles in the faces of the theme.
    super::super::theme::install(&ctx);
    let mut scene = Scene::new(None);
    let hand = idle_hand();
    let mut desk = Desk::default();
    let mut tips = Tips::default();
    let mut ring = RingUi::default();
    // The panels keep the profile as they change it: in a folder of the
    // test's own, never in the player's profiles.
    let home_dir = std::env::temp_dir().join(format!("uoterm-panels-{}", uuid::Uuid::new_v4()));
    let home = ProfileHome::in_dir(&home_dir);
    let mut readings = Readings::start(Link::Http {
        api: NO_API.into(),
        session: String::new(),
    });
    let layers = ItemLayers::default();
    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);
    for (at, events) in frames.iter().enumerate() {
        desk.begin();
        let input = RawInput {
            events: events.clone(),
            screen_rect: Some(screen),
            ..RawInput::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut tools = Tools {
                    scene: &mut scene,
                    hand: &hand,
                    desk: &mut desk,
                    tips: &mut tips,
                    ring: &mut ring,
                    time: at as f64,
                    profile_home: &home,
                    readings: &mut readings,
                    layers: &layers,
                };
                draw(ui, screen, &mut tools, profile);
            });
        });
    }
    // The folder is there only when a panel kept the profile.
    if home_dir.exists() {
        std::fs::remove_dir_all(&home_dir).expect("the test's own profile folder");
    }
}

/// The events of typing words.
pub fn typing(words: &str) -> Vec<egui::Event> {
    vec![egui::Event::Text(words.into())]
}

/// The frames of one click of the left button at a place: the mouse comes,
/// the button goes down, and it comes up.
pub fn click(at: Pos2) -> Vec<Vec<egui::Event>> {
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    vec![
        vec![egui::Event::PointerMoved(at)],
        vec![button(true)],
        vec![button(false)],
    ]
}
