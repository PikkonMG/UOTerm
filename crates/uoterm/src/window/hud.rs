//! The panels that float on the map. Each panel has one fixed place and
//! owns one subject, so the eye learns where to look.

use super::theme::{self, number_font, text_font, title_font};
use crate::view::{Danger, WatchFrame, WatchMobile, JOURNAL_LINES, MOBILE_LINES};
use eframe::egui::{
    self,
    epaint::{Mesh, Vertex, WHITE_UV},
    text::LayoutJob,
    Align2, Color32, FontId, Painter, Pos2, Rect, Shape, TextFormat, Vec2,
};

const SIDE_PANEL_WIDTH: f32 = 300.0;
const JOURNAL_WIDTH: f32 = 400.0;
const PACK_WIDTH: f32 = 340.0;
const MESSAGE_WIDTH: f32 = 440.0;
const ROW_HEIGHT: f32 = 20.0;
const TITLE_HEIGHT: f32 = 28.0;
const BAR_ROW_GAP: f32 = 10.0;
const BAR_LABEL_WIDTH: f32 = 58.0;
const CHIP_PAD: Vec2 = Vec2::new(7.0, 3.0);
const CHIP_GAP: f32 = 6.0;
const PACK_MIN_GAP: f32 = 12.0;
const CHIP_RADIUS: u8 = 4;
const CHIP_FILL_ALPHA: f32 = 0.18;
const DOT_RADIUS: f32 = 4.0;
const ROSTER_PIP_WIDTH: f32 = 40.0;
const ROSTER_DIST_WIDTH: f32 = 30.0;
const JOURNAL_OLD_ALPHA: f32 = 0.55;
const JOURNAL_LINE_GAP: f32 = 4.0;
const CHAT_ROW_HEIGHT: f32 = 30.0;
const PERCENT: f32 = 100.0;

/// The share of the way to its goal that a bar covers each second.
const BAR_RATE: f32 = 10.0;
/// The lost part of a bar stays for a moment, then follows more slowly.
const GHOST_RATE: f32 = 1.6;
const BAR_AT_REST: f32 = 0.002;

const VIGNETTE_DEPTH: f32 = 170.0;
const ALARM_DEPTH: f32 = 120.0;
const VIGNETTE_ALPHA: f32 = 0.55;
const PULSE_PER_SECOND: f32 = 1.2;
const PULSE_LOW: f32 = 0.25;
const FIGHT_ALPHA: f32 = 0.20;
const CRITICAL_ALPHA: f32 = 0.50;
const DEAD_ALPHA: f32 = 0.38;

const WEIGHT_WARN_SHARE: f32 = 0.9;

#[derive(Default)]
pub struct Hud {
    bars: [Bar; 3],
    /// Where the panels were drawn this frame.
    panels: Vec<Rect>,
    journal_filter: JournalFilter,
    /// The left middle of the row of filter words, when the journal has them.
    filter_row: Option<Pos2>,
}

/// Which lines the journal shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum JournalFilter {
    #[default]
    All,
    /// What mobiles said.
    Talk,
    /// What the shard wrote.
    System,
}

const JOURNAL_FILTERS: [(JournalFilter, &str); 3] = [
    (JournalFilter::All, "All"),
    (JournalFilter::Talk, "Talk"),
    (JournalFilter::System, "System"),
];
const FILTER_LEFT: f32 = 96.0;
const FILTER_GAP: f32 = 12.0;
/// The message types of lines the shard wrote: a system line, a name label.
const KINDS_SYSTEM: [u8; 2] = [1, 6];

/// The lines of the journal in words. The full journal of `watch` can be
/// filtered; the short one of `observe` shows as it is.
fn journal_texts(frame: &WatchFrame, filter: JournalFilter) -> Vec<String> {
    if frame.speech.is_empty() {
        return frame.journal.clone();
    }
    frame
        .speech
        .iter()
        .filter(|line| {
            let system = line.serial == 0 || KINDS_SYSTEM.contains(&line.kind);
            match filter {
                JournalFilter::All => true,
                JournalFilter::Talk => !system,
                JournalFilter::System => system,
            }
        })
        .map(|line| {
            if line.name.is_empty() {
                line.text.clone()
            } else {
                format!("{}: {}", line.name, line.text)
            }
        })
        .collect()
}

/// What the panels tell the rest of the window after they are drawn.
pub struct Drawn {
    /// True while a bar still moves or the alarm pulses.
    pub moving: bool,
    /// The free row under the journal, when one was asked for.
    pub chat_row: Option<Rect>,
    /// The pack panel at the bottom middle. The hotbar sits on it.
    pub pack: Rect,
}

/// What one bar shows now, as shares of its full length.
#[derive(Clone, Copy, Default)]
struct Bar {
    fill: f32,
    ghost: f32,
}

impl Bar {
    fn follow(&mut self, goal: f32, dt: f32) -> bool {
        let step = |rate: f32| 1.0 - (-rate * dt).exp();
        self.fill += (goal - self.fill) * step(BAR_RATE);
        self.ghost = if self.ghost < self.fill {
            self.fill
        } else {
            self.ghost + (self.fill - self.ghost) * step(GHOST_RATE)
        };
        (goal - self.fill).abs() > BAR_AT_REST || (self.ghost - self.fill).abs() > BAR_AT_REST
    }
}

fn share(now: u16, max: u16) -> f32 {
    if max == 0 {
        0.0
    } else {
        (f32::from(now) / f32::from(max)).clamp(0.0, 1.0)
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
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

/// One panel on the map. The window keeps its place, so that a click on it
/// is not a click on the map.
fn glass(painter: &Painter, panels: &mut Vec<Rect>, panel: Rect) {
    theme::panel(painter, panel);
    panels.push(panel);
}

fn panel_height(content: f32) -> f32 {
    content + theme::PANEL_PAD * 2.0
}

impl Hud {
    /// Draws every panel. `chat_row` keeps a row free for the chat box.
    pub fn draw(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        chat_row: bool,
        time: f64,
        dt: f32,
    ) -> Drawn {
        let danger = frame.danger();
        vignette(painter, rect, danger, time);
        let area = rect.shrink(theme::SCREEN_MARGIN);
        self.panels.clear();
        activity(painter, &mut self.panels, area, frame);
        roster(painter, &mut self.panels, area, frame);
        let lines = journal_texts(frame, self.journal_filter);
        let (chat_row, title_row) =
            journal(painter, &mut self.panels, area, frame, &lines, chat_row);
        self.filter_row = (!frame.speech.is_empty()).then_some(title_row);
        let pack = pack(painter, &mut self.panels, area, frame);
        let bars_move = self.vitals(painter, area, frame, dt);
        Drawn {
            moving: bars_move || danger != Danger::Calm,
            chat_row,
            pack,
        }
    }

    /// The words that pick which journal lines show. Call this after
    /// `draw`, with the `Ui` that takes the clicks.
    pub fn journal_filters(&mut self, ui: &egui::Ui) {
        let Some(mut at) = self.filter_row else {
            return;
        };
        for (filter, words) in JOURNAL_FILTERS {
            let color = if filter == self.journal_filter {
                theme::GOAL
            } else {
                theme::TEXT_FAINT
            };
            let galley =
                ui.painter()
                    .layout_no_wrap(words.to_string(), text_font(theme::SIZE_SMALL), color);
            let area = Align2::LEFT_CENTER.anchor_size(at, galley.size());
            let response = ui.interact(
                area.expand(FILTER_GAP / 2.0),
                egui::Id::new(("journal-filter", words)),
                egui::Sense::click(),
            );
            ui.painter().galley(area.min, galley, color);
            if response.clicked() {
                self.journal_filter = filter;
            }
            at.x = area.right() + FILTER_GAP;
        }
    }

    /// True when the point is on a panel and not on the map.
    pub fn covers(&self, point: Pos2) -> bool {
        self.panels.iter().any(|panel| panel.contains(point))
    }

    fn vitals(&mut self, painter: &Painter, area: Rect, frame: &WatchFrame, dt: f32) -> bool {
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
        let states = states(frame);
        let fights = !frame.combatant.is_empty();
        let content = TITLE_HEIGHT
            + if states.is_empty() {
                0.0
            } else {
                ROW_HEIGHT + theme::ROW_GAP
            }
            + theme::BAR_HEIGHT_MAIN
            + theme::BAR_HEIGHT * (self.bars.len() - 1) as f32
            + BAR_ROW_GAP * self.bars.len() as f32
            + if fights { ROW_HEIGHT } else { 0.0 };
        let panel = Rect::from_min_size(
            Pos2::new(area.left(), area.bottom() - panel_height(content)),
            Vec2::new(SIDE_PANEL_WIDTH, panel_height(content)),
        );
        glass(painter, panels, panel);
        let mut rows = rows(painter, panel);
        rows.title(&frame.name, theme::NOTO_SELF);
        if !states.is_empty() {
            rows.chips(&states);
        }
        let hits = BarLook {
            fill: if frame.poisoned {
                theme::HITS_POISONED
            } else {
                theme::HITS
            },
            height: theme::BAR_HEIGHT_MAIN,
            number_size: theme::SIZE_BODY,
            number_color: if frame.danger() >= Danger::Critical {
                theme::ALARM
            } else {
                theme::TEXT
            },
        };
        rows.bar("Hits", self.bars[0], frame.hits, frame.hits_max, hits);
        rows.bar(
            "Mana",
            self.bars[1],
            frame.mana,
            frame.mana_max,
            BarLook::small(theme::MANA),
        );
        rows.bar(
            "Stamina",
            self.bars[2],
            frame.stam,
            frame.stam_max,
            BarLook::small(theme::STAM),
        );
        if fights {
            rows.pair("Fights", &frame.combatant, theme::ALARM);
        }
        moving
    }
}

fn states(frame: &WatchFrame) -> Vec<(&'static str, Color32)> {
    [
        (frame.dead, "Dead", theme::ALARM),
        (frame.war, "War mode", theme::ALARM),
        (frame.poisoned, "Poisoned", theme::HITS_POISONED),
        (frame.paralyzed, "Paralyzed", theme::WAITING),
        (frame.hidden, "Hidden", theme::TEXT_DIM),
    ]
    .into_iter()
    .filter(|(on, _, _)| *on)
    .map(|(_, word, color)| (word, color))
    .collect()
}

/// What the agent does now: the goal is the heading, the detail is below it.
fn activity(painter: &Painter, panels: &mut Vec<Rect>, area: Rect, frame: &WatchFrame) {
    let mut detail: Vec<(&str, String, Color32)> = Vec::new();
    if frame.job != "-" && !frame.job.is_empty() {
        let job = if frame.phase.is_empty() {
            frame.job.clone()
        } else {
            format!("{}, {}", frame.job, frame.phase)
        };
        detail.push(("Job", job, theme::TEXT));
    }
    if let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) {
        let tiles = x.abs_diff(frame.x).max(y.abs_diff(frame.y));
        detail.push((
            "Walks to",
            format!("{x}, {y}  ({tiles} tiles)"),
            theme::GOAL,
        ));
    }
    if !frame.following.is_empty() {
        detail.push(("Follows", frame.following.clone(), theme::TEXT));
    }
    if !frame.script.is_empty() {
        detail.push(("Script", frame.script.clone(), theme::TEXT));
    }
    let idle = detail.is_empty();
    let content = TITLE_HEIGHT + ROW_HEIGHT * detail.len().max(1) as f32;
    let panel = Rect::from_min_size(
        area.left_top(),
        Vec2::new(SIDE_PANEL_WIDTH, panel_height(content)),
    );
    glass(painter, panels, panel);
    let mut rows = rows(painter, panel);
    let goal = if frame.goal.is_empty() || frame.goal == "-" {
        "Idle".to_string()
    } else {
        capitalized(&frame.goal)
    };
    rows.title(&goal, theme::TEXT);
    if idle {
        rows.line("No job. No walk goal.", theme::TEXT_FAINT);
    }
    for (label, value, color) in detail {
        rows.pair(label, &value, color);
    }
}

fn roster(painter: &Painter, panels: &mut Vec<Rect>, area: Rect, frame: &WatchFrame) {
    let shown: Vec<&WatchMobile> = frame.mobiles.iter().take(MOBILE_LINES).collect();
    let content = TITLE_HEIGHT + ROW_HEIGHT * shown.len().max(1) as f32;
    let panel = Rect::from_min_size(
        Pos2::new(area.right() - SIDE_PANEL_WIDTH, area.top()),
        Vec2::new(SIDE_PANEL_WIDTH, panel_height(content)),
    );
    glass(painter, panels, panel);
    let mut rows = rows(painter, panel);
    painter.text(
        Pos2::new(rows.right, rows.y + TITLE_HEIGHT / 2.0 - theme::ROW_GAP),
        Align2::RIGHT_CENTER,
        frame.mobiles.len().to_string(),
        number_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    rows.title("Near", theme::TEXT);
    if shown.is_empty() {
        rows.line("Nobody is near.", theme::TEXT_FAINT);
    }
    for mobile in shown {
        roster_row(painter, &rows, mobile, mobile.name == frame.combatant);
        rows.y += ROW_HEIGHT;
    }
}

fn roster_row(painter: &Painter, rows: &Rows<'_>, mobile: &WatchMobile, target: bool) {
    let middle = rows.y + ROW_HEIGHT / 2.0 - theme::ROW_GAP / 2.0;
    let color = theme::notoriety_color(mobile.notoriety);
    painter.circle_filled(Pos2::new(rows.left + DOT_RADIUS, middle), DOT_RADIUS, color);
    let name_left = rows.left + DOT_RADIUS * 2.0 + CHIP_GAP;
    let pip_left = rows.right - ROSTER_DIST_WIDTH - ROSTER_PIP_WIDTH;
    let mut name = LayoutJob::default();
    name.wrap.max_width = pip_left - CHIP_GAP - name_left;
    name.wrap.max_rows = 1;
    name.wrap.break_anywhere = true;
    let name_color = if target { theme::ALARM } else { theme::TEXT };
    let format = |color: Color32| TextFormat::simple(text_font(theme::SIZE_BODY), color);
    name.append(&mobile.name, 0.0, format(name_color));
    if !mobile.title.is_empty() {
        name.append(&mobile.title, CHIP_GAP, format(theme::TEXT_FAINT));
    }
    let galley = painter.layout_job(name);
    painter.galley(
        Pos2::new(name_left, middle - galley.size().y / 2.0),
        galley,
        name_color,
    );
    if let Some(percent) = mobile.hits_percent {
        let track = Rect::from_min_size(
            Pos2::new(pip_left, middle - theme::PIP_HEIGHT / 2.0),
            Vec2::new(ROSTER_PIP_WIDTH - CHIP_GAP, theme::PIP_HEIGHT),
        );
        painter.rect_filled(track, theme::BAR_RADIUS, theme::TRACK);
        let mut fill = track;
        fill.set_width(track.width() * f32::from(percent) / PERCENT);
        painter.rect_filled(fill, theme::BAR_RADIUS, color);
    }
    painter.text(
        Pos2::new(rows.right, middle),
        Align2::RIGHT_CENTER,
        mobile.dist.to_string(),
        number_font(theme::SIZE_SMALL),
        theme::TEXT_DIM,
    );
}

/// The newest lines, newest at the bottom. The speaker is dim and the words
/// are bright. Older lines fade.
/// `chat_row` keeps one row free at the bottom for the chat box, and gives
/// its place.
fn journal(
    painter: &Painter,
    panels: &mut Vec<Rect>,
    area: Rect,
    frame: &WatchFrame,
    all_lines: &[String],
    chat_row: bool,
) -> (Option<Rect>, Pos2) {
    let width = JOURNAL_WIDTH - theme::PANEL_PAD * 2.0;
    let start = all_lines.len().saturating_sub(JOURNAL_LINES);
    let lines = &all_lines[start..];
    let galleys: Vec<_> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let age = (lines.len() - 1 - i) as f32 / JOURNAL_LINES as f32;
            let alpha = 1.0 - age * (1.0 - JOURNAL_OLD_ALPHA);
            painter.layout_job(journal_job(line, width, alpha))
        })
        .collect();
    let text_height: f32 = galleys
        .iter()
        .map(|g| g.size().y + JOURNAL_LINE_GAP)
        .sum::<f32>()
        .max(ROW_HEIGHT);
    let row_height = if chat_row { CHAT_ROW_HEIGHT } else { 0.0 };
    let content = TITLE_HEIGHT + text_height + row_height;
    let panel = Rect::from_min_size(
        Pos2::new(
            area.right() - JOURNAL_WIDTH,
            area.bottom() - panel_height(content),
        ),
        Vec2::new(JOURNAL_WIDTH, panel_height(content)),
    );
    glass(painter, panels, panel);
    let mut rows = rows(painter, panel);
    if frame.unanswered > 0 {
        painter.text(
            Pos2::new(rows.right, rows.y + TITLE_HEIGHT / 2.0 - theme::ROW_GAP),
            Align2::RIGHT_CENTER,
            waiting_words(frame.unanswered),
            text_font(theme::SIZE_SMALL),
            theme::WAITING,
        );
    }
    let title_row = Pos2::new(
        rows.left + FILTER_LEFT,
        rows.y + TITLE_HEIGHT / 2.0 - theme::ROW_GAP,
    );
    rows.title("Journal", theme::TEXT);
    if galleys.is_empty() {
        rows.line("No lines yet.", theme::TEXT_FAINT);
    }
    for galley in galleys {
        let height = galley.size().y;
        painter.galley(Pos2::new(rows.left, rows.y), galley, theme::TEXT);
        rows.y += height + JOURNAL_LINE_GAP;
    }
    let chat = chat_row.then(|| {
        let inner = panel.shrink(theme::PANEL_PAD);
        Rect::from_min_max(
            Pos2::new(
                inner.left(),
                inner.bottom() - CHAT_ROW_HEIGHT + JOURNAL_LINE_GAP,
            ),
            inner.right_bottom(),
        )
    });
    (chat, title_row)
}

fn waiting_words(persons: usize) -> String {
    if persons == 1 {
        "1 person waits for an answer".to_string()
    } else {
        format!("{persons} persons wait for an answer")
    }
}

fn journal_job(line: &str, width: f32, alpha: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let format =
        |font: FontId, color: Color32| TextFormat::simple(font, theme::with_alpha(color, alpha));
    let font = text_font(theme::SIZE_BODY);
    match line.split_once(": ") {
        Some((speaker, words)) => {
            job.append(speaker, 0.0, format(font.clone(), theme::GOAL));
            job.append(words, CHIP_GAP, format(font, theme::TEXT));
        }
        None => job.append(line, 0.0, format(font, theme::TEXT_DIM)),
    }
    job
}

/// The pack, the gold, the buffs and the party, in the space between the
/// vitals and the journal.
fn pack(painter: &Painter, panels: &mut Vec<Rect>, area: Rect, frame: &WatchFrame) -> Rect {
    let lists: Vec<(&str, String)> = [("Buffs", &frame.buffs), ("Party", &frame.party)]
        .into_iter()
        .filter(|(_, list)| !list.is_empty())
        .map(|(label, list)| (label, list.join(", ")))
        .collect();
    let content = ROW_HEIGHT * (1 + lists.len()) as f32 - theme::ROW_GAP;
    let free_left = area.left() + SIDE_PANEL_WIDTH + PACK_MIN_GAP;
    let free_right = area.right() - JOURNAL_WIDTH - PACK_MIN_GAP;
    let width = PACK_WIDTH.min(free_right - free_left);
    let panel = Rect::from_min_size(
        Pos2::new(
            (free_left + free_right - width) / 2.0,
            area.bottom() - panel_height(content),
        ),
        Vec2::new(width, panel_height(content)),
    );
    glass(painter, panels, panel);
    let mut rows = rows(painter, panel);
    let heavy = f32::from(frame.weight) >= f32::from(frame.weight_max) * WEIGHT_WARN_SHARE
        && frame.weight_max > 0;
    let weight_color = if heavy { theme::WAITING } else { theme::TEXT };
    let gold_word = painter
        .text(
            Pos2::new(rows.right, rows.y),
            Align2::RIGHT_TOP,
            "gold",
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
    rows.pair(
        "Weight",
        &format!("{}/{} stones", frame.weight, frame.weight_max),
        weight_color,
    );
    for (label, list) in lists {
        rows.pair(label, &list, theme::TEXT);
    }
    panel
}

/// A dark edge that keeps the panels readable, and the alarm color over it
/// when there is danger. The alarm pulses; a dead character holds it still.
fn vignette(painter: &Painter, rect: Rect, danger: Danger, time: f64) {
    let pulse = {
        let wave = (time as f32 * PULSE_PER_SECOND * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        PULSE_LOW + (1.0 - PULSE_LOW) * wave
    };
    let alarm = match danger {
        Danger::Calm => 0.0,
        Danger::Fight => FIGHT_ALPHA * pulse,
        Danger::Critical => CRITICAL_ALPHA * pulse,
        Danger::Dead => DEAD_ALPHA,
    };
    edge_glow(
        painter,
        rect,
        VIGNETTE_DEPTH,
        Color32::BLACK.gamma_multiply(VIGNETTE_ALPHA),
    );
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
    let content = TITLE_HEIGHT + ROW_HEIGHT * lines.len() as f32;
    let panel = Rect::from_center_size(
        rect.center(),
        Vec2::new(MESSAGE_WIDTH, panel_height(content)),
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
    const HALF: f32 = 0.5;

    #[test]
    fn the_journal_filter_keeps_talk_or_system_lines() {
        use crate::view::WatchSpeech;
        let line = |serial: u32, name: &str, kind: u8, text: &str| WatchSpeech {
            serial,
            name: name.into(),
            kind,
            text: text.into(),
            ..WatchSpeech::default()
        };
        let frame = WatchFrame {
            journal: vec!["short journal".into()],
            speech: vec![
                line(5, "Ann", 0, "hail"),
                line(0, "", 1, "The world will save."),
                line(5, "Ann", 6, "Ann"),
            ],
            ..WatchFrame::default()
        };
        assert_eq!(journal_texts(&frame, JournalFilter::All).len(), 3);
        assert_eq!(
            journal_texts(&frame, JournalFilter::Talk),
            vec!["Ann: hail"]
        );
        assert_eq!(journal_texts(&frame, JournalFilter::System).len(), 2);
        let from_observe = WatchFrame {
            journal: vec!["short journal".into()],
            ..WatchFrame::default()
        };
        assert_eq!(
            journal_texts(&from_observe, JournalFilter::Talk),
            vec!["short journal"]
        );
    }

    #[test]
    fn a_lost_part_of_a_bar_stays_behind_the_fill() {
        let mut bar = Bar {
            fill: 1.0,
            ghost: 1.0,
        };
        assert!(bar.follow(HALF, ONE_FRAME));
        assert!(bar.fill < 1.0);
        assert!(bar.ghost > bar.fill);
    }

    #[test]
    fn a_bar_that_grows_has_no_ghost() {
        let mut bar = Bar::default();
        bar.follow(HALF, ONE_FRAME);
        assert_eq!(bar.ghost, bar.fill);
    }

    #[test]
    fn one_person_waits_and_two_persons_wait() {
        assert_eq!(waiting_words(1), "1 person waits for an answer");
        assert_eq!(waiting_words(2), "2 persons wait for an answer");
    }

    #[test]
    fn a_bar_with_no_maximum_is_empty() {
        assert_eq!(share(10, 0), 0.0);
        assert_eq!(share(30, 20), 1.0);
    }
}
