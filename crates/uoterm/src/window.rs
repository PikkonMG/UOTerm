//! Optional native watch window. `uoterm watch` opens this. `connect --view`
//! starts the same command in a child process so closing the window does not
//! drop the game socket. `--text-view` prints the radar in the terminal.

use crate::remote;
use crate::view::{
    radar_color, RadarMark, WatchFrame, CELL_PX, JOURNAL_LINES, MOBILE_LINES, WATCH_POLL_MS,
    WATCH_RADAR_SIZE, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

const RADAR_GLYPH_PX: f32 = 9.0;
const MARK_FONT_PX: f32 = 10.0;
const MARK_LABEL: Color32 = Color32::from_rgb(240, 230, 200);

pub fn open(api: String, session: String) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_title(WINDOW_TITLE),
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(move |_cc| Ok(Box::new(WatchApp::start(api, session)))),
    )
    .map_err(|e| e.to_string())
}

struct WatchApp {
    rx: Receiver<WatchFrame>,
    frame: WatchFrame,
}

impl WatchApp {
    fn start(api: String, session: String) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || poll_loop(api, session, tx));
        Self {
            rx,
            frame: WatchFrame::default(),
        }
    }
}

impl eframe::App for WatchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.rx.try_recv() {
            Ok(next) => self.frame = next,
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                if self.frame.error.is_empty() {
                    self.frame.error = "watch ended".into();
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            draw_frame(ui, &self.frame);
        });
        ctx.request_repaint_after(Duration::from_millis(WATCH_POLL_MS));
    }
}

fn poll_loop(api: String, session: String, tx: mpsc::Sender<WatchFrame>) {
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
            match remote::session_observe(&api, &session, WATCH_RADAR_SIZE).await {
                Ok(value) => WatchFrame::from_observe(&value),
                Err(e) => WatchFrame::error_frame(e.to_string()),
            }
        });
        if tx.send(frame).is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(WATCH_POLL_MS));
    }
}

fn draw_frame(ui: &mut egui::Ui, frame: &WatchFrame) {
    if !frame.error.is_empty() {
        ui.colored_label(Color32::from_rgb(220, 80, 80), &frame.error);
        return;
    }
    ui.heading(WINDOW_TITLE);
    ui.label(format!(
        "{}   hp {}/{}   mana {}/{}   stam {}/{}   {},{},{} map {}",
        frame.name,
        frame.hits,
        frame.hits_max,
        frame.mana,
        frame.mana_max,
        frame.stam,
        frame.stam_max,
        frame.x,
        frame.y,
        frame.z,
        frame.map
    ));
    ui.label(format!(
        "goal {}   job {}   war={}   dead={}",
        frame.goal, frame.job, frame.war, frame.dead
    ));
    if let (Some(dx), Some(dy)) = (frame.dest_x, frame.dest_y) {
        ui.label(format!("dest {dx},{dy}"));
    }
    ui.separator();
    ui.horizontal(|ui| {
        draw_radar(ui, &frame.radar, &frame.marks);
        ui.vertical(|ui| {
            ui.strong("near");
            for line in frame.mobiles.iter().take(MOBILE_LINES) {
                ui.label(line);
            }
            ui.separator();
            ui.strong("journal");
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for line in frame.journal.iter().rev().take(JOURNAL_LINES).rev() {
                        ui.label(line);
                    }
                });
        });
    });
}

fn draw_radar(ui: &mut egui::Ui, rows: &[String], marks: &[RadarMark]) {
    if rows.is_empty() {
        ui.label("(no radar)");
        return;
    }
    let cols = rows.iter().map(String::len).max().unwrap_or(0) as f32;
    let size = Vec2::new(cols * CELL_PX, rows.len() as f32 * CELL_PX);
    let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    for (row_i, row) in rows.iter().enumerate() {
        for (col_i, ch) in row.chars().enumerate() {
            let min = Pos2::new(
                rect.min.x + col_i as f32 * CELL_PX,
                rect.min.y + row_i as f32 * CELL_PX,
            );
            let cell = Rect::from_min_size(min, Vec2::splat(CELL_PX - 1.0));
            let (r, g, b) = radar_color(ch);
            painter.rect_filled(cell, 0.0, Color32::from_rgb(r, g, b));
            painter.text(
                cell.center(),
                egui::Align2::CENTER_CENTER,
                ch.to_string(),
                egui::FontId::monospace(RADAR_GLYPH_PX),
                Color32::WHITE,
            );
        }
    }
    let half = rows.len() as i32 / 2;
    for mark in marks {
        let col = half + mark.dx;
        let row = half + mark.dy;
        if col < 0 || row < 0 {
            continue;
        }
        let min = Pos2::new(
            rect.min.x + (col as f32 + 1.0) * CELL_PX,
            rect.min.y + row as f32 * CELL_PX + CELL_PX / 2.0,
        );
        painter.text(
            min,
            egui::Align2::LEFT_CENTER,
            &mark.name,
            egui::FontId::monospace(MARK_FONT_PX),
            MARK_LABEL,
        );
    }
}
