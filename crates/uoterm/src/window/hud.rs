//! The panels that float on the map: what the agent does, the vitals, and
//! the pack. Each owns one subject and starts at its own place, so the eye
//! learns where to look; the player moves each by its title and locks it,
//! and the profile keeps where he left it. The journal and the near list
//! are Modern panels of their own. The plan of the Modern style
//! (`modern::layout`) gives each panel its first place.

use super::boxes_ui::Tools;
use super::modern::frame::{self as panel_frame, PanelSpec};
use super::modern::layout::{self, Spot};
use super::settings::Profile;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{Danger, WatchFrame};
use crate::window::bridge;
use eframe::egui::{
    self,
    epaint::{Mesh, Vertex, WHITE_UV},
    text::LayoutJob,
    Align2, Color32, Painter, Pos2, Rect, Shape, Vec2,
};
use uoterm_view::ui::hud::{
    self as rules, activity_details, activity_height, alarm_share, clock_words, goal_words,
    hits_look, message_height, pack_height, pack_lists, share, vitals_height, weight_color,
    weight_words, Bar, ACTIVITY_ID, BAR_COUNT, BAR_ROW_GAP, PACK_ID, PACK_WIDTH, ROW_HEIGHT,
    SIDE_PANEL_WIDTH, TITLE_HEIGHT, VITALS_ID, WORDS_FIGHTS, WORDS_GOLD, WORDS_HITS, WORDS_MANA,
    WORDS_NO_JOB, WORDS_PACK, WORDS_STAMINA, WORDS_TIME, WORDS_WEIGHT,
};
use uoterm_view::ui::theme::{ALARM_DEPTH, VIGNETTE_DEPTH, VITAL_LABEL_WIDTH as BAR_LABEL_WIDTH};

const MESSAGE_WIDTH: f32 = 440.0;
const CHIP_PAD: Vec2 = Vec2::new(7.0, 3.0);
const CHIP_GAP: f32 = 6.0;
const CHIP_RADIUS: u8 = 4;
const CHIP_FILL_ALPHA: f32 = 0.18;

#[derive(Default)]
pub struct Hud {
    bars: [Bar; BAR_COUNT],
    /// Where the panels were drawn this frame, by their ids.
    panels: Vec<(&'static str, Rect)>,
}

/// What the panels tell the rest of the window after they are drawn.
pub struct Drawn {
    /// True while a bar still moves or the alarm pulses.
    pub moving: bool,
    /// The pack panel at the bottom middle. The hotbar sits on it.
    pub pack: Rect,
}

/// How one bar looks: its fill, its height, and its numbers.
#[derive(Clone, Copy)]
struct BarLook {
    fill: Color32,
    height: f32,
    number_size: f32,
    number_color: Color32,
}

impl BarLook {
    fn small(fill: Color32) -> Self {
        Self {
            fill,
            height: theme::BAR_HEIGHT,
            number_size: theme::SIZE_SMALL,
            number_color: theme::TEXT,
        }
    }
}

/// A column of rows inside one panel. Each call draws one row and moves down.
struct Rows<'a> {
    painter: &'a Painter,
    left: f32,
    right: f32,
    y: f32,
}

impl Rows<'_> {
    fn title(&mut self, text: &str, color: Color32) {
        self.painter.text(
            Pos2::new(self.left, self.y),
            Align2::LEFT_TOP,
            text,
            title_font(theme::SIZE_TITLE),
            color,
        );
        self.y += TITLE_HEIGHT;
    }

    /// A dim label on the left and its value after it.
    fn pair(&mut self, label: &str, value: &str, value_color: Color32) {
        let label_end = self
            .painter
            .text(
                Pos2::new(self.left, self.y),
                Align2::LEFT_TOP,
                label,
                text_font(theme::SIZE_BODY),
                theme::TEXT_DIM,
            )
            .right();
        self.painter.text(
            Pos2::new(label_end + CHIP_GAP, self.y),
            Align2::LEFT_TOP,
            value,
            text_font(theme::SIZE_BODY),
            value_color,
        );
        self.y += ROW_HEIGHT;
    }

    fn line(&mut self, text: &str, color: Color32) {
        self.painter.text(
            Pos2::new(self.left, self.y),
            Align2::LEFT_TOP,
            text,
            text_font(theme::SIZE_BODY),
            color,
        );
        self.y += ROW_HEIGHT;
    }

    fn chips(&mut self, chips: &[(&str, Color32)]) {
        let mut x = self.left;
        let font = text_font(theme::SIZE_SMALL);
        let mut height = 0.0_f32;
        for (word, color) in chips {
            let galley = self
                .painter
                .layout_no_wrap((*word).to_string(), font.clone(), *color);
            let rect = Rect::from_min_size(Pos2::new(x, self.y), galley.size() + CHIP_PAD * 2.0);
            self.painter.rect_filled(
                rect,
                CHIP_RADIUS,
                theme::with_alpha(*color, CHIP_FILL_ALPHA),
            );
            self.painter.galley(rect.min + CHIP_PAD, galley, *color);
            x = rect.right() + CHIP_GAP;
            height = height.max(rect.height());
        }
        self.y += height + theme::ROW_GAP;
    }

    fn bar(&mut self, label: &str, bar: Bar, now: u16, max: u16, look: BarLook) {
        let middle = self.y + look.height / 2.0;
        self.painter.text(
            Pos2::new(self.left, middle),
            Align2::LEFT_CENTER,
            label,
            text_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let numbers = self
            .painter
            .text(
                Pos2::new(self.right, middle),
                Align2::RIGHT_CENTER,
                format!("{now}/{max}"),
                number_font(look.number_size),
                look.number_color,
            )
            .left();
        let track = Rect::from_min_max(
            Pos2::new(self.left + BAR_LABEL_WIDTH, self.y),
            Pos2::new(
                numbers.min(self.right - BAR_LABEL_WIDTH * 1.5) - CHIP_GAP,
                self.y + look.height,
            ),
        );
        let part = |share: f32| {
            let mut rect = track;
            rect.set_width(track.width() * share);
            rect
        };
        self.painter
            .rect_filled(track, theme::BAR_RADIUS, theme::TRACK);
        self.painter
            .rect_filled(part(bar.ghost), theme::BAR_RADIUS, theme::BAR_GHOST);
        self.painter
            .rect_filled(part(bar.fill), theme::BAR_RADIUS, look.fill);
        self.y += look.height + BAR_ROW_GAP;
    }
}

fn rows<'a>(painter: &'a Painter, panel: Rect) -> Rows<'a> {
    let inner = panel.shrink(theme::PANEL_PAD);
    Rows {
        painter,
        left: inner.left(),
        right: inner.right(),
        y: inner.top(),
    }
}

/// One panel on the map of `size`, where the player left it, else at its
/// spot of the plan. The window keeps its place, so that a click on it is
/// not a click on the map. Gives its place.
fn glass(
    painter: &Painter,
    panels: &mut Vec<(&'static str, Rect)>,
    (id, spot, size): (&'static str, Spot, Vec2),
    window: Rect,
    profile: &Profile,
) -> Rect {
    let default = layout::first_place(window, spot, size);
    let panel = panel_frame::place(window, &spec(id, default), profile);
    theme::panel(painter, panel);
    panels.push((id, panel));
    panel
}

/// The frame of a panel on the map: it moves by its title and locks, and
/// has a size of its own.
fn spec(id: &'static str, default: Rect) -> PanelSpec<'static> {
    PanelSpec {
        id,
        title: "",
        default,
        min_size: None,
        closable: false,
    }
}

impl Hud {
    /// Draws every panel where the player left it.
    pub fn draw(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        (time, dt): (f64, f32),
        profile: &Profile,
    ) -> Drawn {
        let danger = frame.danger();
        vignette(painter, rect, danger, time);
        self.panels.clear();
        let window = Window { rect, profile };
        activity(painter, &mut self.panels, &window, frame);
        let pack = pack(painter, &mut self.panels, &window, frame);
        let bars_move = self.vitals(painter, &window, frame, dt);
        Drawn {
            moving: bars_move || danger != Danger::Calm,
            pack,
        }
    }

    /// Takes the drags on the titles of the panels and the clicks on their
    /// locks. Call it after the panels over them are drawn.
    pub fn controls(&self, ui: &egui::Ui, tools: &Tools<'_>, profile: &mut Profile) {
        for (id, panel) in &self.panels {
            panel_frame::controls(ui, *panel, &spec(id, *panel), profile, tools);
        }
    }

    /// True when the point is on a panel and not on the map.
    pub fn covers(&self, point: Pos2) -> bool {
        self.panels.iter().any(|(_, panel)| panel.contains(point))
    }

    fn vitals(
        &mut self,
        painter: &Painter,
        window: &Window<'_>,
        frame: &WatchFrame,
        dt: f32,
    ) -> bool {
        let panels = &mut self.panels;
        let goals = [
            share(frame.hits, frame.hits_max),
            share(frame.mana, frame.mana_max),
            share(frame.stam, frame.stam_max),
        ];
        let mut moving = false;
        for (bar, goal) in self.bars.iter_mut().zip(goals) {
            moving |= bar.follow(goal, dt);
        }
        let states: Vec<(&str, Color32)> = rules::states(frame)
            .into_iter()
            .map(|(word, color)| (word, bridge::color(color)))
            .collect();
        let fights = !frame.combatant.is_empty();
        let size = Vec2::new(SIDE_PANEL_WIDTH, vitals_height(!states.is_empty(), fights));
        let panel = glass(
            painter,
            panels,
            (VITALS_ID, Spot::Vitals, size),
            window.rect,
            window.profile,
        );
        let mut rows = rows(painter, panel);
        rows.title(&frame.name, theme::NOTO_SELF);
        if !states.is_empty() {
            rows.chips(&states);
        }
        let (fill, number_color) = hits_look(frame);
        let hits = BarLook {
            fill: bridge::color(fill),
            height: theme::BAR_HEIGHT_MAIN,
            number_size: theme::SIZE_BODY,
            number_color: bridge::color(number_color),
        };
        rows.bar(WORDS_HITS, self.bars[0], frame.hits, frame.hits_max, hits);
        rows.bar(
            WORDS_MANA,
            self.bars[1],
            frame.mana,
            frame.mana_max,
            BarLook::small(theme::MANA),
        );
        rows.bar(
            WORDS_STAMINA,
            self.bars[2],
            frame.stam,
            frame.stam_max,
            BarLook::small(theme::STAM),
        );
        if fights {
            rows.pair(WORDS_FIGHTS, &frame.combatant, theme::ALARM);
        }
        moving
    }
}

/// The window the panels stand in, and the profile that keeps their places.
struct Window<'a> {
    rect: Rect,
    profile: &'a Profile,
}

/// What the agent does now: the goal is the heading, the detail is below it.
fn activity(
    painter: &Painter,
    panels: &mut Vec<(&'static str, Rect)>,
    window: &Window<'_>,
    frame: &WatchFrame,
) {
    let detail = activity_details(frame);
    let idle = detail.is_empty();
    let size = Vec2::new(SIDE_PANEL_WIDTH, activity_height(detail.len()));
    let panel = glass(
        painter,
        panels,
        (ACTIVITY_ID, Spot::Activity, size),
        window.rect,
        window.profile,
    );
    let mut rows = rows(painter, panel);
    rows.title(&goal_words(frame), theme::TEXT);
    if idle {
        rows.line(WORDS_NO_JOB, theme::TEXT_FAINT);
    }
    for (label, value, color) in detail {
        rows.pair(label, &value, bridge::color(color));
    }
}

/// The pack, the gold, the clock of the shard, the buffs and the party, in
/// the middle between the vitals and the journal. Each has its own row.
fn pack(
    painter: &Painter,
    panels: &mut Vec<(&'static str, Rect)>,
    window: &Window<'_>,
    frame: &WatchFrame,
) -> Rect {
    let lists = pack_lists(frame);
    let size = Vec2::new(PACK_WIDTH, pack_height(lists.len()));
    let panel = glass(
        painter,
        panels,
        (PACK_ID, Spot::Pack, size),
        window.rect,
        window.profile,
    );
    let mut rows = rows(painter, panel);
    rows.title(WORDS_PACK, theme::TEXT);
    let weight_color = bridge::color(weight_color(frame));
    let gold_word = painter
        .text(
            Pos2::new(rows.right, rows.y),
            Align2::RIGHT_TOP,
            WORDS_GOLD,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        )
        .left();
    painter.text(
        Pos2::new(gold_word - CHIP_GAP, rows.y),
        Align2::RIGHT_TOP,
        frame.gold.to_string(),
        number_font(theme::SIZE_BODY),
        theme::NOTO_SELF,
    );
    rows.pair(WORDS_WEIGHT, &weight_words(frame), weight_color);
    rows.pair(WORDS_TIME, &clock_words(frame), theme::TEXT_DIM);
    for (label, list) in lists {
        rows.pair(label, &list, theme::TEXT);
    }
    panel
}

/// A dark edge that keeps the panels readable, and the alarm color over it
/// when there is danger. The alarm pulses; a dead character holds it still.
fn vignette(painter: &Painter, rect: Rect, danger: Danger, time: f64) {
    let alarm = alarm_share(danger, time);
    edge_glow(painter, rect, VIGNETTE_DEPTH, theme::VIGNETTE);
    if alarm > 0.0 {
        edge_glow(
            painter,
            rect,
            ALARM_DEPTH,
            theme::with_alpha(theme::ALARM, alarm),
        );
    }
}

/// Four bands along the window edges. Each is `color` at the edge and clear
/// on its inner side.
fn edge_glow(painter: &Painter, rect: Rect, depth: f32, color: Color32) {
    let inner = rect.shrink(depth);
    let outer = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let inside = [
        inner.left_top(),
        inner.right_top(),
        inner.right_bottom(),
        inner.left_bottom(),
    ];
    let mut mesh = Mesh::default();
    for (pos, tint) in outer
        .iter()
        .map(|p| (*p, color))
        .chain(inside.iter().map(|p| (*p, Color32::TRANSPARENT)))
    {
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color: tint,
        });
    }
    let corners = outer.len() as u32;
    for i in 0..corners {
        let next = (i + 1) % corners;
        mesh.indices
            .extend([i, next, corners + next, i, corners + next, corners + i]);
    }
    painter.add(Shape::mesh(mesh));
}

/// One panel in the middle of the window, for the times with no picture.
pub fn message(painter: &Painter, rect: Rect, heading: &str, lines: &[&str], color: Color32) {
    painter.rect_filled(rect, 0.0, theme::VOID);
    let panel = Rect::from_center_size(
        rect.center(),
        Vec2::new(MESSAGE_WIDTH, message_height(lines.len())),
    );
    theme::panel(painter, panel);
    let mut rows = rows(painter, panel);
    rows.title(heading, color);
    let width = MESSAGE_WIDTH - theme::PANEL_PAD * 2.0;
    for line in lines {
        let mut job = LayoutJob::simple(
            (*line).to_string(),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
            width,
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        painter.galley(
            Pos2::new(rows.left, rows.y),
            painter.layout_job(job),
            theme::TEXT_DIM,
        );
        rows.y += ROW_HEIGHT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_FRAME: f32 = 1.0 / 60.0;

    #[test]
    fn the_panels_stand_where_the_player_left_them() {
        use crate::window::bridge;
        use crate::window::model::places;
        let window = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
        let kept = Rect::from_min_size(Pos2::new(600.0, 300.0), Vec2::new(10.0, 10.0));
        let mut profile = Profile::default();
        places::remember(&mut profile, VITALS_ID, bridge::area(kept), false);
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut hud = Hud::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            hud.draw(
                &painter,
                window,
                &WatchFrame::default(),
                (0.0, ONE_FRAME),
                &profile,
            );
        });
        let ids: Vec<&str> = hud.panels.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![ACTIVITY_ID, PACK_ID, VITALS_ID]);
        assert!(
            hud.covers(kept.min + Vec2::splat(1.0)),
            "the vitals moved there"
        );
        assert!(!hud.covers(window.left_bottom() - Vec2::new(-20.0, 20.0)));
    }
}
