//! The making of a new character: one glass panel over the model of
//! `model::creation`, in the look of the Modern panels. A row of steps at
//! the top shows where the player is (the look, the skills, the start town,
//! the name). A large figure of the new character stands at the left on
//! every page and turns to the left or to the right. The page is in the
//! middle. At the bottom are Back and Next, and the reason when Next cannot
//! go on. Enter is Next and Esc is Back.

use super::map_view::{Lay, MapPictures};
use super::model::creation::{
    at_limit, facet_name, find_skills, Creation, CreationFiles, Limit, Paint, Palette, Race, Stage,
    Step, Style, FACINGS, NAME_MAX, NAME_MIN, SKILL_RANGE, STAT_RANGE,
};
use super::scene::{Scene, DOLL_FACING};
use super::theme::{self, text_font, title_font};
use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Galley, Id, Key, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, TextFormat, Vec2,
};
use std::sync::Arc;
use uoterm_nav::{Profession, ProfessionKind};
use uoterm_protocol::StartTown;

// The frame of the screen.
/// The panel fills the window up to this size.
const PANEL_MOST: Vec2 = Vec2::new(1200.0, 780.0);
const PANEL_INSET: f32 = 20.0;
const HEADER_ROW: f32 = 44.0;
const FOOTER_ROW: f32 = 44.0;
const PART_GAP: f32 = 16.0;
const PREVIEW_WIDTH: f32 = 250.0;
const RADIUS: u8 = 8;

// The figure.
/// The figure grows by whole steps up to this, so each pixel of the art
/// stays sharp.
pub const PREVIEW_MOST_SCALE: f32 = 6.0;
const TURN_ROW: f32 = 34.0;
const CAPTION_ROW: f32 = 44.0;
/// The feet of the figure stand this far above the bottom of its box.
const FLOOR_PAD: f32 = 26.0;
const SHADOW_RADIUS: Vec2 = Vec2::new(34.0, 8.0);
/// The facet the figure is drawn for.
const FIGURE_MAP: u8 = 0;

// The row of steps.
const STEP_HEIGHT: f32 = 32.0;
const STEP_DOT: f32 = 20.0;
const STEP_PAD: f32 = 10.0;
const STEP_JOIN: f32 = 18.0;
const LINE_WIDTH: f32 = 2.0;

// The parts of the pages.
const HEADING_ROW: f32 = 28.0;
const LABEL_ROW: f32 = 22.0;
const CHOICE_ROW: f32 = 36.0;
const CHIP_ROW: f32 = 32.0;
const SEX_WIDTH: f32 = 110.0;
const RACE_WIDTH: f32 = 130.0;
const CHIP_MIN_WIDTH: f32 = 92.0;
const SECTION_GAP: f32 = 14.0;
const GAP: f32 = 8.0;
const CELL_MOST: f32 = 18.0;
const PICKED_WIDTH: f32 = 2.0;
const CARD_HEIGHT: f32 = 72.0;
const CARD_MIN_WIDTH: f32 = 300.0;
const CARD_ICON: f32 = 56.0;
const CARD_TEXT_ROWS: usize = 2;
const DETAIL_ROW: f32 = 22.0;
const STAT_LABEL_WIDTH: f32 = 100.0;
/// The stats take this share of the Custom page, the skills the rest.
const STATS_SHARE: f32 = 0.45;
const POINTS_ROW: f32 = 44.0;
const POINTS_BUTTON: f32 = 28.0;
const POINTS_NUMBER: f32 = 36.0;
const HANDLE_RADIUS: f32 = 7.0;
const PICKER_WIDTH: f32 = 170.0;
const PICKER_LIST_HEIGHT: f32 = 260.0;
const PICKER_LIST_WIDTH: f32 = 240.0;
const BIG_BUTTON_WIDTH: f32 = 140.0;
const ICON_GAP: f32 = 12.0;
/// A locked profession shows its picture this faint.
const LOCKED_ALPHA: f32 = 0.5;
/// The profession pictures show in their own colors.
const NO_HUE: u16 = 0;
const TOWN_LIST_WIDTH: f32 = 280.0;
const TOWN_ROW: f32 = 50.0;
const TOWN_MARK: f32 = 4.0;
const MAP_MOST: f32 = 300.0;
/// The words about a town keep at least this much room under its map.
const TOWN_WORDS_LEAST: f32 = 140.0;
/// Tiles across the map of a start town.
const MAP_TILES: f32 = 160.0;
const PIN_RADIUS: f32 = 6.0;
const PIN_RING: f32 = 12.0;
const NAME_FIELD_HEIGHT: f32 = 52.0;
const NAME_FIELD_MOST: f32 = 460.0;
const NAME_MARGIN: egui::Margin = egui::Margin::symmetric(12, 10);
const RULE_DOT: f32 = 3.0;
const SUMMARY_LABEL_WIDTH: f32 = 96.0;
const SUMMARY_SWATCH: f32 = 20.0;
const HALF: f32 = 2.0;
const ELLIPSIS: char = '…';

// The words of the client have tags.
const TAG_OPEN: char = '<';
const TAG_CLOSE: char = '>';
const LINE_BREAK_TAG: &str = "br";
const SELF_CLOSING: char = '/';
const SENTENCE_END: &str = ". ";

const WORDS_TITLE: &str = "New character";
const WORDS_BACK: &str = "Back";
const WORDS_NEXT: &str = "Next";
const WORDS_CREATE: &str = "Create";
const WORDS_KEYS: &str = "Enter: next     Esc: back";
const WORDS_TURN_LEFT: &str = "Turn left";
const WORDS_TURN_RIGHT: &str = "Turn right";
const WORDS_NO_ART: &str = "The figure needs the client files.";
const WORDS_BODY: &str = "Body";
const WORDS_MALE: &str = "Male";
const WORDS_FEMALE: &str = "Female";
const WORDS_MAN: &str = "man";
const WORDS_WOMAN: &str = "woman";
const WORDS_LOCKED: &str = "locked";
const WORDS_HAIR: &str = "Hair";
const WORDS_BEARD: &str = "Beard";
const WORDS_PROFESSION: &str = "Pick a profession";
const WORDS_PROFESSION_HINT: &str = "Or pick Custom to set your own stats and skills.";
const WORDS_CUSTOM: &str = "Custom";
const WORDS_CUSTOM_ABOUT: &str = "Set your own stats and skills on the next page.";
const WORDS_NEEDS_SAMURAI: &str = "Needs Samurai Empire";
const WORDS_CATEGORY: &str = "Opens a list of professions.";
const WORDS_STATS: &str = "Stats";
const WORDS_SKILLS: &str = "Skills";
const WORDS_PICK_SKILL: &str = "Pick a skill";
const WORDS_SEARCH: &str = "Search skills";
const WORDS_TAKEN: &str = "(taken)";
const WORDS_TOWN: &str = "Pick a start town";
const WORDS_NAME: &str = "Name your character";
const WORDS_NAME_HINT: &str = "Type a name";
const WORDS_NAME_GOOD: &str = "The name is good.";
const WORDS_SUMMARY: &str = "Your character";
const WORDS_LOOK: &str = "Look";
const WORDS_COLORS: &str = "Colors";
const WORDS_TRADE: &str = "Profession";
const WORDS_START: &str = "Town";
const STAT_WORDS: [&str; 3] = ["Strength", "Intelligence", "Dexterity"];
const STAT_SHORT: [&str; 3] = ["Str", "Int", "Dex"];
const LIST_JOIN: &str = "  ·  ";
const COLOR_ORDER: [Paint; 5] = [
    Paint::Skin,
    Paint::Hair,
    Paint::Beard,
    Paint::Shirt,
    Paint::Pants,
];
const WORDS_LEAST: &str = "least";
const WORDS_MOST: &str = "most";

/// What the screen asks the login screens to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Back from the first page: the list of characters again.
    Leave,
    /// The character is made: send it to the shard.
    Finish,
}

/// The client art and the map pictures the screen draws with.
pub struct Art<'a> {
    /// None with no client files: the figure and the colors need them.
    pub scene: Option<&'a mut Scene>,
    pub town_map: &'a mut MapPictures,
}

/// Where the parts of the screen go in the window.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    panel: Rect,
    header: Rect,
    preview: Rect,
    content: Rect,
    footer: Rect,
}

fn frame(window: Rect) -> Frame {
    let room = window.shrink(theme::SCREEN_MARGIN);
    let panel = Rect::from_center_size(room.center(), room.size().min(PANEL_MOST));
    let inner = panel.shrink(PANEL_INSET);
    let header = Rect::from_min_size(inner.min, Vec2::new(inner.width(), HEADER_ROW));
    let footer = Rect::from_min_max(
        Pos2::new(inner.left(), inner.bottom() - FOOTER_ROW),
        inner.max,
    );
    let middle = Rect::from_min_max(
        Pos2::new(inner.left(), header.bottom() + PART_GAP),
        Pos2::new(inner.right(), footer.top() - PART_GAP),
    );
    let preview = Rect::from_min_size(middle.min, Vec2::new(PREVIEW_WIDTH, middle.height()));
    let content = Rect::from_min_max(
        Pos2::new(preview.right() + PART_GAP, middle.top()),
        middle.max,
    );
    Frame {
        panel,
        header,
        preview,
        content,
        footer,
    }
}

/// The scale a figure of `art` pixels is drawn at in `room`: the largest
/// whole scale that fits, up to [`PREVIEW_MOST_SCALE`], so the pixels stay
/// sharp. Art larger than the room shrinks to fit it.
pub fn preview_scale(room: Vec2, art: Vec2) -> f32 {
    let fit = (room.x / art.x).min(room.y / art.y);
    if fit >= 1.0 {
        fit.floor().min(PREVIEW_MOST_SCALE)
    } else {
        fit
    }
}

/// How many boxes of `least` width fit in a row of `width`, with gaps.
fn columns_in(width: f32, least: f32) -> usize {
    (((width + GAP) / (least + GAP)).floor() as usize).max(1)
}

/// Words of the client with their tags taken out: a line break tag starts
/// a new line.
fn plain_words(html: &str) -> String {
    let mut words = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, TAG_OPEN) => tag = Some(String::new()),
            (Some(name), TAG_CLOSE) => {
                let name = name.trim().trim_end_matches(SELF_CLOSING);
                if name.eq_ignore_ascii_case(LINE_BREAK_TAG) {
                    words.push('\n');
                }
                tag = None;
            }
            (Some(name), c) => name.push(c),
            (None, c) => words.push(c),
        }
    }
    words
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// The first sentence of some words, as the short text of a card.
fn first_sentence(words: &str) -> &str {
    let line = words.lines().next().unwrap_or_default();
    line.find(SENTENCE_END)
        .map_or(line, |at| &line[..=at])
        .trim()
}

/// Words wrapped to a width, cut after `rows` lines with an ellipsis.
fn wrapped(
    ui: &egui::Ui,
    words: &str,
    font: FontId,
    color: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(words.to_owned(), TextFormat::simple(font, color));
    job.wrap = TextWrapping {
        max_width: width,
        max_rows: rows,
        break_anywhere: false,
        overflow_character: Some(ELLIPSIS),
    };
    ui.painter().layout_job(job)
}

fn heading(ui: &egui::Ui, at: Pos2, words: &str) {
    ui.painter().text(
        at,
        Align2::LEFT_TOP,
        words,
        title_font(theme::SIZE_HEADING),
        theme::TEXT,
    );
}

fn label(ui: &egui::Ui, at: Pos2, words: &str) {
    ui.painter().text(
        at,
        Align2::LEFT_TOP,
        words,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
}

/// One choice that fills `area`: filled and ringed in the goal color when
/// it is picked. True when it was clicked.
fn choice(ui: &egui::Ui, area: Rect, key: Id, words: &str, picked: bool, faint: bool) -> bool {
    let response = ui.interact(area, key, Sense::click());
    let fill = match (picked, response.hovered()) {
        (true, _) => theme::CHOSEN,
        (false, true) => theme::BUTTON_HOVER,
        (false, false) => theme::BUTTON,
    };
    let radius = CornerRadius::same(RADIUS);
    ui.painter().rect_filled(area, radius, fill);
    if picked {
        ui.painter().rect_stroke(
            area,
            radius,
            Stroke::new(PICKED_WIDTH, theme::GOAL),
            StrokeKind::Inside,
        );
    }
    let color = match (picked, faint) {
        (true, _) => theme::GOAL,
        (false, true) => theme::TEXT_FAINT,
        (false, false) => theme::TEXT,
    };
    ui.painter().text(
        area.center(),
        Align2::CENTER_CENTER,
        words,
        text_font(theme::SIZE_BODY),
        color,
    );
    response.clicked()
}

/// A large button of the bottom row. The main one is filled with the goal
/// color. True when it was clicked while it is enabled.
fn big_button(ui: &egui::Ui, area: Rect, words: &str, main: bool, enabled: bool) -> bool {
    let response = ui.interact(area, Id::new(("creation-button", words)), Sense::click());
    let hovered = response.hovered() && enabled;
    let (fill, color) = match (main && enabled, hovered) {
        (true, false) => (theme::GOAL, theme::VOID),
        (true, true) => (theme::TEXT, theme::VOID),
        (false, true) => (theme::BUTTON_HOVER, theme::TEXT),
        (false, false) if enabled => (theme::BUTTON, theme::TEXT),
        (false, false) => (theme::BUTTON, theme::TEXT_FAINT),
    };
    ui.painter()
        .rect_filled(area, CornerRadius::same(RADIUS), fill);
    ui.painter().text(
        area.center(),
        Align2::CENTER_CENTER,
        words,
        title_font(theme::SIZE_HEADING),
        color,
    );
    response.clicked() && enabled
}

/// Draws the screen in the window. Gives what the player asked.
pub fn draw(
    ui: &mut egui::Ui,
    window: Rect,
    creation: &mut Creation,
    files: &CreationFiles,
    mut art: Art<'_>,
) -> Option<Asked> {
    // A list that is open takes Enter and Esc for itself.
    let popup_open = ui.memory(|memory| memory.any_popup_open());
    let typing = ui.ctx().wants_keyboard_input();
    let frame = frame(window);
    ui.painter()
        .rect_filled(window, CornerRadius::ZERO, theme::VOID);
    theme::panel(ui.painter(), frame.panel);
    header(ui, frame.header, creation.step.stage());
    preview(ui, frame.preview, creation, files, art.scene.as_deref_mut());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(frame.content));
    let page = &mut child;
    match creation.step {
        Step::Look => look_page(page, frame.content, creation, art.scene.as_deref()),
        Step::Profession(_) => {
            profession_page(
                page,
                frame.content,
                creation,
                files,
                art.scene.as_deref_mut(),
            );
        }
        Step::Trade => trade_page(page, frame.content, creation, files),
        Step::Town => town_page(page, frame.content, creation, files, &mut art),
        Step::Name => name_page(page, frame.content, creation, files, art.scene.as_deref()),
    }
    let (back, next) = footer(ui, frame.footer, creation);
    let keys = !popup_open;
    let back = back || (keys && ui.input(|input| input.key_pressed(Key::Escape)));
    let next = next || (keys && ui.input(|input| input.key_pressed(Key::Enter)));
    if !typing && keys {
        let (left, right) = ui.input(|input| {
            (
                input.key_pressed(Key::ArrowLeft),
                input.key_pressed(Key::ArrowRight),
            )
        });
        if left || right {
            creation.turn(right);
        }
    }
    if back {
        return creation.back().then_some(Asked::Leave);
    }
    (next && creation.next()).then_some(Asked::Finish)
}

/// The title, and the row of steps with the one that shows picked.
fn header(ui: &egui::Ui, area: Rect, now: Stage) {
    ui.painter().text(
        area.left_center(),
        Align2::LEFT_CENTER,
        WORDS_TITLE,
        title_font(theme::SIZE_TITLE),
        theme::TEXT,
    );
    let at_now = Stage::ALL.iter().position(|stage| *stage == now);
    let pills: Vec<(usize, Arc<Galley>)> = Stage::ALL
        .iter()
        .enumerate()
        .map(|(at, stage)| {
            let galley = ui.painter().layout_no_wrap(
                stage.words().to_string(),
                text_font(theme::SIZE_PLATE),
                theme::TEXT,
            );
            (at, galley)
        })
        .collect();
    // A pad before the dot, between the dot and the words, and after them.
    let pill_width = |galley: &Galley| STEP_PAD + STEP_DOT + STEP_PAD + galley.size().x + STEP_PAD;
    let whole: f32 = pills.iter().map(|(_, g)| pill_width(g)).sum::<f32>()
        + STEP_JOIN * (pills.len() - 1) as f32;
    let mut left = area.right() - whole;
    for (at, galley) in pills {
        let pill = Rect::from_min_size(
            Pos2::new(left, area.center().y - STEP_HEIGHT / HALF),
            Vec2::new(pill_width(&galley), STEP_HEIGHT),
        );
        let (done, current) = match at_now {
            Some(now) => (at < now, at == now),
            None => (false, false),
        };
        let radius = CornerRadius::same(u8::MAX);
        ui.painter().rect_filled(
            pill,
            radius,
            if current {
                theme::CHOSEN
            } else {
                theme::BUTTON
            },
        );
        if current {
            ui.painter().rect_stroke(
                pill,
                radius,
                Stroke::new(PICKED_WIDTH, theme::GOAL),
                StrokeKind::Inside,
            );
        }
        let dot = Pos2::new(pill.left() + STEP_PAD + STEP_DOT / HALF, pill.center().y);
        let (dot_fill, number_color, words_color) = match (current, done) {
            (true, _) => (theme::GOAL, theme::VOID, theme::TEXT),
            (false, true) => (theme::CHOSEN, theme::GOAL, theme::TEXT_DIM),
            (false, false) => (theme::TRACK, theme::TEXT_FAINT, theme::TEXT_FAINT),
        };
        ui.painter().circle(
            dot,
            STEP_DOT / HALF,
            dot_fill,
            Stroke::new(LINE_WIDTH / HALF, number_color),
        );
        ui.painter().text(
            dot,
            Align2::CENTER_CENTER,
            (at + 1).to_string(),
            text_font(theme::SIZE_SMALL),
            number_color,
        );
        ui.painter().galley(
            Pos2::new(
                dot.x + STEP_DOT / HALF + STEP_PAD,
                pill.center().y - galley.size().y / HALF,
            ),
            galley,
            words_color,
        );
        if at + 1 < Stage::ALL.len() {
            let join_color = if done { theme::GOAL } else { theme::GLASS_EDGE };
            ui.painter().line_segment(
                [
                    Pos2::new(pill.right(), pill.center().y),
                    Pos2::new(pill.right() + STEP_JOIN, pill.center().y),
                ],
                Stroke::new(LINE_WIDTH, join_color),
            );
        }
        left = pill.right() + STEP_JOIN;
    }
}

/// The bottom row: the reason Next cannot go on, or the keys, and the
/// Back and Next buttons. Gives whether each was clicked.
fn footer(ui: &egui::Ui, area: Rect, creation: &Creation) -> (bool, bool) {
    let button = Vec2::new(BIG_BUTTON_WIDTH, area.height());
    let next_area = Rect::from_min_size(Pos2::new(area.right() - button.x, area.top()), button);
    let back_area = Rect::from_min_size(
        Pos2::new(next_area.left() - GAP - button.x, area.top()),
        button,
    );
    let blocker = creation.blocker();
    let (words, color) = match blocker {
        Some(blocker) => (blocker.words(), theme::WAITING),
        None => (WORDS_KEYS.to_string(), theme::TEXT_FAINT),
    };
    let room = back_area.left() - area.left() - PART_GAP;
    let galley = wrapped(ui, &words, text_font(theme::SIZE_PLATE), color, room, 1);
    ui.painter().galley(
        Pos2::new(area.left(), area.center().y - galley.size().y / HALF),
        galley,
        color,
    );
    let last = creation.step == Step::Name;
    let back = big_button(ui, back_area, WORDS_BACK, false, true);
    let next = big_button(
        ui,
        next_area,
        if last { WORDS_CREATE } else { WORDS_NEXT },
        true,
        blocker.is_none(),
    );
    (back, next)
}

/// The words of the race and the sex, as "Human man".
fn body_words(creation: &Creation) -> String {
    let sex = if creation.female {
        WORDS_WOMAN
    } else {
        WORDS_MAN
    };
    format!("{} {sex}", creation.race.words())
}

/// The name of a profession as the card shows it: Advanced is Custom.
fn profession_name(profession: &Profession, files: &CreationFiles) -> String {
    if profession.is_advanced() {
        WORDS_CUSTOM.to_string()
    } else {
        name_case(&files.words(profession.name_id, &profession.name))
    }
}

/// Words in capitals, as the client writes a profession ("SAMURAI"), with
/// only the first letter of each word as a capital.
fn name_case(words: &str) -> String {
    if words.chars().any(char::is_lowercase) {
        return words.to_string();
    }
    words
        .split(' ')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or(String::new(), |first| {
                first.to_string() + &letters.as_str().to_lowercase()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The large figure of the new character, the turn buttons, and its words.
fn preview(
    ui: &egui::Ui,
    area: Rect,
    creation: &mut Creation,
    files: &CreationFiles,
    scene: Option<&mut Scene>,
) {
    let figure_box = Rect::from_min_max(
        area.min,
        Pos2::new(area.right(), area.bottom() - TURN_ROW - CAPTION_ROW - GAP),
    );
    ui.painter()
        .rect_filled(figure_box, CornerRadius::same(RADIUS), theme::TRACK);
    let direction = (DOLL_FACING + creation.turns) % FACINGS;
    let picture =
        scene.and_then(|scene| scene.turned_picture(FIGURE_MAP, &creation.look(), direction));
    match picture {
        Some((texture, sprite)) => {
            let room = Vec2::new(figure_box.width(), figure_box.height() - FLOOR_PAD * HALF);
            let scale = preview_scale(room, Vec2::new(sprite.width, sprite.height));
            let size = Vec2::new(sprite.width, sprite.height) * scale;
            let floor = figure_box.bottom() - FLOOR_PAD;
            let left = (figure_box.center().x - sprite.anchor.x * scale)
                .clamp(figure_box.left(), figure_box.right() - size.x);
            let shown = Rect::from_min_size(Pos2::new(left, floor - size.y).round(), size);
            ui.painter().add(Shape::ellipse_filled(
                Pos2::new(figure_box.center().x, floor),
                SHADOW_RADIUS,
                theme::TEXT_SHADOW,
            ));
            ui.painter()
                .image(texture, shown, sprite.uv, Color32::WHITE);
        }
        None => {
            let galley = wrapped(
                ui,
                WORDS_NO_ART,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
                figure_box.width() - PART_GAP * HALF,
                usize::MAX,
            );
            ui.painter().galley(
                figure_box.center() - galley.size() / HALF,
                galley,
                theme::TEXT_FAINT,
            );
        }
    }
    let turn_top = figure_box.bottom() + GAP;
    let half = (area.width() - GAP) / HALF;
    let turn = |at: f32| Rect::from_min_size(Pos2::new(at, turn_top), Vec2::new(half, TURN_ROW));
    let left_turn = turn(area.left());
    let right_turn = turn(area.right() - half);
    if theme::segment(ui, left_turn, WORDS_TURN_LEFT, theme::TEXT) {
        creation.turn(false);
    }
    if theme::segment(ui, right_turn, WORDS_TURN_RIGHT, theme::TEXT) {
        creation.turn(true);
    }
    let caption_top = left_turn.bottom() + GAP;
    let name = creation.name.trim();
    let profession = creation
        .profession
        .as_ref()
        .map(|profession| profession_name(profession, files));
    let (title, about) = match (name.is_empty(), profession) {
        (true, profession) => (body_words(creation), profession.unwrap_or_default()),
        (false, Some(profession)) => (
            name.to_string(),
            format!("{}, {profession}", body_words(creation)),
        ),
        (false, None) => (name.to_string(), body_words(creation)),
    };
    ui.painter().text(
        Pos2::new(area.center().x, caption_top),
        Align2::CENTER_TOP,
        title,
        title_font(theme::SIZE_HEADING),
        theme::TEXT,
    );
    ui.painter().text(
        Pos2::new(area.center().x, caption_top + LABEL_ROW),
        Align2::CENTER_TOP,
        about,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
}

fn paint_words(paint: Paint, race: Race) -> &'static str {
    match paint {
        Paint::Skin => "Skin",
        Paint::Shirt if race == Race::Gargoyle => "Robe",
        Paint::Shirt => "Shirt",
        Paint::Pants => "Pants",
        Paint::Hair => "Hair color",
        Paint::Beard => "Beard color",
    }
}

/// The look: the body, the hair and beard styles, and every color grid.
fn look_page(ui: &egui::Ui, area: Rect, creation: &mut Creation, scene: Option<&Scene>) {
    heading(ui, area.min, WORDS_BODY);
    let row_top = area.top() + HEADING_ROW;
    let mut left = area.left();
    for (female, words) in [(false, WORDS_MALE), (true, WORDS_FEMALE)] {
        let spot = Rect::from_min_size(Pos2::new(left, row_top), Vec2::new(SEX_WIDTH, CHOICE_ROW));
        let key = Id::new(("creation-sex", female));
        if choice(ui, spot, key, words, creation.female == female, false) {
            creation.set_female(female);
        }
        left = spot.right() + GAP;
    }
    left += PART_GAP;
    for race in creation.races_shown() {
        let allowed = creation.race_allowed(race);
        let words = if allowed {
            race.words().to_string()
        } else {
            format!("{} ({WORDS_LOCKED})", race.words())
        };
        let spot = Rect::from_min_size(Pos2::new(left, row_top), Vec2::new(RACE_WIDTH, CHOICE_ROW));
        let key = Id::new(("creation-race", race.words()));
        if choice(ui, spot, key, &words, creation.race == race, !allowed) {
            creation.set_race(race);
        }
        left = spot.right() + GAP;
    }
    let styles_top = row_top + CHOICE_ROW + SECTION_GAP;
    let hair = creation.hair_styles();
    let beards = creation.beard_styles().unwrap_or_default();
    let both = (hair.len() + beards.len()) as f32;
    let shared = area.width() - if beards.is_empty() { 0.0 } else { PART_GAP };
    let hair_width = shared * hair.len() as f32 / both;
    let hair_area = Rect::from_min_size(
        Pos2::new(area.left(), styles_top),
        Vec2::new(hair_width, area.bottom() - styles_top),
    );
    let mut styles_bottom = style_chips(ui, hair_area, WORDS_HAIR, hair, &mut creation.hair);
    if !beards.is_empty() {
        let beard_area = Rect::from_min_max(
            Pos2::new(hair_area.right() + PART_GAP, styles_top),
            area.max,
        );
        let bottom = style_chips(ui, beard_area, WORDS_BEARD, beards, &mut creation.beard);
        styles_bottom = styles_bottom.max(bottom);
    }
    let colors = Rect::from_min_max(
        Pos2::new(area.left(), styles_bottom + SECTION_GAP),
        area.max,
    );
    color_grids(ui, colors, creation, scene);
}

/// The styles of a list as a grid of choices under a label. Gives the
/// bottom of the grid.
fn style_chips(
    ui: &egui::Ui,
    area: Rect,
    words: &str,
    styles: &[Style],
    picked: &mut usize,
) -> f32 {
    label(ui, area.min, words);
    let columns = columns_in(area.width(), CHIP_MIN_WIDTH);
    let width = (area.width() - GAP * (columns - 1) as f32) / columns as f32;
    let top = area.top() + LABEL_ROW;
    let mut bottom = top;
    for (at, style) in styles.iter().enumerate() {
        let (column, row) = ((at % columns) as f32, (at / columns) as f32);
        let spot = Rect::from_min_size(
            Pos2::new(
                area.left() + column * (width + GAP),
                top + row * (CHIP_ROW + GAP),
            ),
            Vec2::new(width, CHIP_ROW),
        );
        let key = Id::new(("creation-style", words, at));
        if choice(ui, spot, key, style.words, *picked == at, false) {
            *picked = at;
        }
        bottom = spot.bottom();
    }
    bottom
}

/// How the color grids lay out: how many go in the first row, and the
/// side of one box. The grids go in one row, or in two when two rows give
/// larger boxes. No box is larger than [`CELL_MOST`].
fn grid_layout(room: Vec2, palettes: &[Palette]) -> (usize, f32) {
    let row_width = |row: &[Palette]| {
        let columns: usize = row.iter().map(|palette| palette.columns).sum();
        (room.x - PART_GAP * row.len().saturating_sub(1) as f32) / columns.max(1) as f32
    };
    let row_height = |row: &[Palette]| row.iter().map(|palette| palette.rows).max().unwrap_or(0);
    (1..=palettes.len())
        .map(|split| {
            let (first, second) = palettes.split_at(split);
            let rows = if second.is_empty() { 1.0 } else { HALF };
            let tall = row_height(first) + row_height(second);
            let labels = LABEL_ROW * rows + SECTION_GAP * (rows - 1.0);
            let wide = if second.is_empty() {
                row_width(first)
            } else {
                row_width(first).min(row_width(second))
            };
            let cell = wide
                .min((room.y - labels) / tall.max(1) as f32)
                .min(CELL_MOST)
                .floor();
            (split, cell)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((palettes.len(), CELL_MOST))
}

/// The colors of the look in the order the page shows them: the body
/// first, then the clothes.
fn shown_paints(creation: &Creation) -> Vec<Paint> {
    let paints = creation.paints();
    COLOR_ORDER
        .into_iter()
        .filter(|paint| paints.contains(paint))
        .collect()
}

/// Every color of the look, each as a grid of its hues with the picked one
/// ringed.
fn color_grids(ui: &egui::Ui, area: Rect, creation: &mut Creation, scene: Option<&Scene>) {
    let paints = shown_paints(creation);
    let palettes: Vec<Palette> = paints
        .iter()
        .map(|paint| creation.palette(*paint))
        .collect();
    let (first_row, cell) = grid_layout(area.size(), &palettes);
    let second_top = area.top()
        + LABEL_ROW
        + palettes[..first_row]
            .iter()
            .map(|palette| palette.rows)
            .max()
            .unwrap_or(0) as f32
            * cell
        + SECTION_GAP;
    let mut left = area.left();
    for (at, (paint, palette)) in paints.into_iter().zip(palettes).enumerate() {
        if at == first_row {
            left = area.left();
        }
        let top = if at < first_row {
            area.top()
        } else {
            second_top
        };
        label(ui, Pos2::new(left, top), paint_words(paint, creation.race));
        let origin = Pos2::new(left, top + LABEL_ROW);
        let key = Id::new(("creation-colors", paint_words(paint, creation.race)));
        let color = |hue: u16| scene.map_or(theme::BUTTON, |scene| scene.words_color(hue));
        let picked = creation.color_place(paint);
        if let Some(at) = palette_grid(ui, origin, cell, key, &palette, color, picked) {
            creation.set_color(paint, at);
        }
        left += palette.columns as f32 * cell + PART_GAP;
    }
}

/// The hues of a palette as a grid of boxes in their colors, the picked
/// one ringed. Gives the place of a box the player clicked.
fn palette_grid(
    ui: &egui::Ui,
    origin: Pos2,
    cell: f32,
    key: Id,
    palette: &Palette,
    color: impl Fn(u16) -> Color32,
    picked: usize,
) -> Option<usize> {
    let size = Vec2::new(palette.columns as f32, palette.rows as f32) * cell;
    let area = Rect::from_min_size(origin, size);
    let response = ui.interact(area, key, Sense::click());
    let cell_at = |at: usize| {
        let (column, row) = ((at % palette.columns) as f32, (at / palette.columns) as f32);
        Rect::from_min_size(origin + Vec2::new(column, row) * cell, Vec2::splat(cell))
    };
    let count = palette.hues.len().min(palette.rows * palette.columns);
    for (at, hue) in palette.hues.iter().take(count).enumerate() {
        ui.painter()
            .rect_filled(cell_at(at), CornerRadius::ZERO, color(*hue));
    }
    if picked < count {
        let ring = cell_at(picked);
        ui.painter().rect_stroke(
            ring,
            CornerRadius::ZERO,
            Stroke::new(PICKED_WIDTH, theme::TEXT),
            StrokeKind::Outside,
        );
        ui.painter().rect_stroke(
            ring,
            CornerRadius::ZERO,
            Stroke::new(PICKED_WIDTH / HALF, theme::VOID),
            StrokeKind::Inside,
        );
    }
    let pointer = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let spot = (pointer - origin) / cell;
    let at = spot.y as usize * palette.columns + spot.x as usize;
    (at < count).then_some(at)
}

/// The professions as cards, and what the picked one gives.
fn profession_page(
    ui: &mut egui::Ui,
    area: Rect,
    creation: &mut Creation,
    files: &CreationFiles,
    mut scene: Option<&mut Scene>,
) {
    heading(ui, area.min, WORDS_PROFESSION);
    label(
        ui,
        area.min + Vec2::new(0.0, HEADING_ROW),
        WORDS_PROFESSION_HINT,
    );
    let cards_top = area.top() + HEADING_ROW + LABEL_ROW + GAP;
    let details_top = area.bottom() - DETAIL_ROW * HALF;
    let cards = Rect::from_min_max(
        Pos2::new(area.left(), cards_top),
        Pos2::new(area.right(), details_top - GAP),
    );
    let columns = columns_in(cards.width(), CARD_MIN_WIDTH);
    let width = (cards.width() - GAP * (columns - 1) as f32) / columns as f32;
    let professions: Vec<Profession> = creation.professions(files).into_iter().cloned().collect();
    let mut picked = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(cards), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("creation-professions")
            .max_height(cards.height())
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(GAP);
                for row in professions.chunks(columns) {
                    ui.horizontal(|ui| {
                        for profession in row {
                            let (spot, _) = ui
                                .allocate_exact_size(Vec2::new(width, CARD_HEIGHT), Sense::hover());
                            if card(ui, spot, creation, profession, files, scene.as_deref_mut()) {
                                picked = Some(profession.clone());
                            }
                        }
                    });
                }
            });
    });
    if let Some(profession) = picked {
        creation.pick_profession(&profession, files);
    }
    profession_details(
        ui,
        Rect::from_min_max(Pos2::new(area.left(), details_top), area.max),
        creation,
        files,
    );
}

/// One card of a profession: its picture, its name and its first words.
/// True when it was clicked and may be taken.
fn card(
    ui: &egui::Ui,
    spot: Rect,
    creation: &Creation,
    profession: &Profession,
    files: &CreationFiles,
    scene: Option<&mut Scene>,
) -> bool {
    let locked = creation.profession_locked(profession, files);
    let picked = creation
        .profession
        .as_ref()
        .is_some_and(|known| known.true_name == profession.true_name);
    let key = Id::new(("creation-profession", &profession.true_name));
    let about = if profession.is_advanced() {
        WORDS_CUSTOM_ABOUT.to_string()
    } else if profession.kind == ProfessionKind::Category {
        WORDS_CATEGORY.to_string()
    } else {
        plain_words(&files.words(profession.description_id, ""))
    };
    let mut response = ui.interact(spot, key, Sense::click());
    if !about.is_empty() {
        response = response.on_hover_text(about.clone());
    }
    let fill = match (picked, response.hovered() && !locked) {
        (true, _) => theme::CHOSEN,
        (false, true) => theme::BUTTON_HOVER,
        (false, false) => theme::BUTTON,
    };
    let radius = CornerRadius::same(RADIUS);
    ui.painter().rect_filled(spot, radius, fill);
    if picked {
        ui.painter().rect_stroke(
            spot,
            radius,
            Stroke::new(PICKED_WIDTH, theme::GOAL),
            StrokeKind::Inside,
        );
    }
    let icon = Rect::from_min_size(
        Pos2::new(spot.left() + GAP, spot.center().y - CARD_ICON / HALF),
        Vec2::splat(CARD_ICON),
    );
    ui.painter()
        .rect_filled(icon, CornerRadius::same(RADIUS), theme::TRACK);
    let tint = if locked {
        theme::with_alpha(Color32::WHITE, LOCKED_ALPHA)
    } else {
        Color32::WHITE
    };
    if let Some((texture, sprite)) =
        scene.and_then(|scene| scene.gump_picture(profession.gump, NO_HUE))
    {
        let shown = theme::fit(icon, sprite.width, sprite.height);
        ui.painter().image(texture, shown, sprite.uv, tint);
    }
    let text_left = icon.right() + ICON_GAP;
    let text_width = spot.right() - GAP - text_left;
    let (name_color, about_color) = if locked {
        (theme::TEXT_FAINT, theme::TEXT_FAINT)
    } else {
        (theme::TEXT, theme::TEXT_DIM)
    };
    ui.painter().text(
        Pos2::new(text_left, spot.top() + GAP),
        Align2::LEFT_TOP,
        profession_name(profession, files),
        title_font(theme::SIZE_HEADING),
        name_color,
    );
    let (short, short_color) = if locked {
        (WORDS_NEEDS_SAMURAI, theme::WAITING)
    } else {
        (first_sentence(&about), about_color)
    };
    let galley = wrapped(
        ui,
        short,
        text_font(theme::SIZE_BODY),
        short_color,
        text_width,
        CARD_TEXT_ROWS,
    );
    ui.painter().galley(
        Pos2::new(text_left, spot.top() + GAP + LABEL_ROW + GAP / HALF),
        galley,
        short_color,
    );
    response.clicked() && !locked
}

/// The stats and skills the picked profession gives, in words.
fn trade_words(creation: &Creation, files: &CreationFiles) -> (String, String) {
    let stats = STAT_SHORT
        .iter()
        .zip(creation.stats)
        .map(|(words, value)| format!("{words} {value}"))
        .collect::<Vec<_>>()
        .join(LIST_JOIN);
    let skills = creation
        .skills
        .iter()
        .filter_map(|pick| {
            let name = files.skill_names.get(usize::from(pick.skill?))?;
            Some(format!("{name} {}", pick.value))
        })
        .collect::<Vec<_>>()
        .join(LIST_JOIN);
    (stats, skills)
}

fn profession_details(ui: &egui::Ui, area: Rect, creation: &Creation, files: &CreationFiles) {
    let Some(profession) = creation.profession.as_ref() else {
        return;
    };
    let lines = if profession.is_advanced() {
        [WORDS_CUSTOM_ABOUT.to_string(), String::new()]
    } else {
        let (stats, skills) = trade_words(creation, files);
        [
            format!("{WORDS_STATS}: {stats}"),
            format!("{WORDS_SKILLS}: {skills}"),
        ]
    };
    for (at, line) in lines.iter().enumerate() {
        let galley = wrapped(
            ui,
            line,
            text_font(theme::SIZE_BODY),
            theme::TEXT,
            area.width(),
            1,
        );
        ui.painter().galley(
            Pos2::new(area.left(), area.top() + at as f32 * DETAIL_ROW),
            galley,
            theme::TEXT,
        );
    }
}

/// A number the player sets: a minus button, a bar he drags, a plus
/// button, and the number. Gives the value he asked for.
fn points(ui: &egui::Ui, area: Rect, key: Id, value: i32, (min, max): (i32, i32)) -> Option<i32> {
    let minus = Rect::from_min_size(
        Pos2::new(area.left(), area.center().y - POINTS_BUTTON / HALF),
        Vec2::splat(POINTS_BUTTON),
    );
    let number = Rect::from_min_max(
        Pos2::new(area.right() - POINTS_NUMBER, area.top()),
        area.max,
    );
    let plus = Rect::from_min_size(
        Pos2::new(number.left() - GAP - POINTS_BUTTON, minus.top()),
        Vec2::splat(POINTS_BUTTON),
    );
    let track = Rect::from_min_max(
        Pos2::new(
            minus.right() + GAP + HANDLE_RADIUS,
            area.center().y - theme::BAR_HEIGHT / HALF,
        ),
        Pos2::new(
            plus.left() - GAP - HANDLE_RADIUS,
            area.center().y + theme::BAR_HEIGHT / HALF,
        ),
    );
    let mut asked = None;
    if theme::segment_keyed(ui, minus, key.with("minus"), "-", theme::TEXT) {
        asked = Some(value - 1);
    }
    if theme::segment_keyed(ui, plus, key.with("plus"), "+", theme::TEXT) {
        asked = Some(value + 1);
    }
    let grab = track.expand2(Vec2::new(
        HANDLE_RADIUS,
        POINTS_ROW / HALF - theme::BAR_HEIGHT,
    ));
    let response = ui.interact(grab, key.with("bar"), Sense::click_and_drag());
    if let Some(pointer) = response.interact_pointer_pos() {
        let share = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let wanted = min + (share * (max - min) as f32).round() as i32;
        if wanted != value {
            asked = Some(wanted);
        }
    }
    let share = (value - min) as f32 / (max - min) as f32;
    theme::bar(ui.painter(), track, share, theme::GOAL);
    let handle = Pos2::new(track.left() + track.width() * share, track.center().y);
    ui.painter().circle(
        handle,
        HANDLE_RADIUS,
        theme::TEXT,
        Stroke::new(LINE_WIDTH / HALF, theme::VOID),
    );
    ui.painter().text(
        number.right_center(),
        Align2::RIGHT_CENTER,
        value.to_string(),
        theme::number_font(theme::SIZE_PLATE),
        theme::TEXT,
    );
    asked.map(|wanted| wanted.clamp(min, max))
}

/// A heading with the total at its right: in the goal color when the
/// values add up to it, in the waiting color when not.
fn total_heading(ui: &egui::Ui, area: Rect, words: &str, values: i32, total: i32) {
    heading(ui, area.min, words);
    let color = if values == total {
        theme::GOAL
    } else {
        theme::WAITING
    };
    ui.painter().text(
        Pos2::new(area.right(), area.top()),
        Align2::RIGHT_TOP,
        format!("Total {values} of {total}"),
        text_font(theme::SIZE_PLATE),
        color,
    );
}

/// The rule of a group of values, and a note when one is at an end.
fn limit_words(
    ui: &egui::Ui,
    at: Pos2,
    width: f32,
    rule: &str,
    limit: Option<(String, Limit, i32)>,
) {
    let galley = wrapped(
        ui,
        rule,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
        width,
        usize::MAX,
    );
    let below = at.y + galley.size().y + GAP / HALF;
    ui.painter().galley(at, galley, theme::TEXT_DIM);
    if let Some((name, limit, value)) = limit {
        let end = match limit {
            Limit::Least => WORDS_LEAST,
            Limit::Most => WORDS_MOST,
        };
        ui.painter().text(
            Pos2::new(at.x, below),
            Align2::LEFT_TOP,
            format!("{name} is at its {end}: {value}."),
            text_font(theme::SIZE_BODY),
            theme::WAITING,
        );
    }
}

/// The stats and the skills of the Custom choice.
fn trade_page(ui: &mut egui::Ui, area: Rect, creation: &mut Creation, files: &CreationFiles) {
    let stats_width = (area.width() - PART_GAP * HALF) * STATS_SHARE;
    let stats_area = Rect::from_min_size(area.min, Vec2::new(stats_width, area.height()));
    let stat_sum: i32 = creation.stats.iter().sum();
    total_heading(ui, stats_area, WORDS_STATS, stat_sum, creation.stat_total());
    let mut top = area.top() + HEADING_ROW + GAP;
    for (at, words) in STAT_WORDS.iter().enumerate() {
        let row = Rect::from_min_size(
            Pos2::new(stats_area.left(), top),
            Vec2::new(stats_area.width(), POINTS_ROW),
        );
        ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            *words,
            text_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        let control =
            Rect::from_min_max(Pos2::new(row.left() + STAT_LABEL_WIDTH, row.top()), row.max);
        if let Some(value) = points(
            ui,
            control,
            Id::new(("creation-stat", at)),
            creation.stats[at],
            STAT_RANGE,
        ) {
            creation.set_stat(at, value);
        }
        top = row.bottom();
    }
    let stat_rule = format!(
        "Each stat is {} to {}. Moving one moves the others.",
        STAT_RANGE.0, STAT_RANGE.1
    );
    let stat_limit = at_limit(&creation.stats, STAT_RANGE)
        .map(|(at, limit)| (STAT_WORDS[at].to_string(), limit, creation.stats[at]));
    limit_words(
        ui,
        Pos2::new(stats_area.left(), top + GAP),
        stats_area.width(),
        &stat_rule,
        stat_limit,
    );

    let skills_area = Rect::from_min_max(
        Pos2::new(stats_area.right() + PART_GAP * HALF, area.top()),
        area.max,
    );
    let values: Vec<i32> = creation.skills.iter().map(|pick| pick.value).collect();
    total_heading(
        ui,
        skills_area,
        WORDS_SKILLS,
        values.iter().sum(),
        creation.skill_total(),
    );
    let menu = creation.skill_menu(files);
    let mut top = area.top() + HEADING_ROW + GAP;
    for at in 0..creation.skills.len() {
        let row = Rect::from_min_size(
            Pos2::new(skills_area.left(), top),
            Vec2::new(skills_area.width(), POINTS_ROW),
        );
        let picker = Rect::from_min_size(
            Pos2::new(row.left(), row.center().y - CHIP_ROW / HALF),
            Vec2::new(PICKER_WIDTH, CHIP_ROW),
        );
        skill_picker(ui, picker, at, creation, &menu);
        let control = Rect::from_min_max(Pos2::new(picker.right() + GAP, row.top()), row.max);
        let key = Id::new(("creation-skill-points", at));
        if let Some(value) = points(ui, control, key, creation.skills[at].value, SKILL_RANGE) {
            creation.set_skill_value(at, value);
        }
        top = row.bottom();
    }
    let skill_rule = format!(
        "Each skill is {} to {}. Pick a different skill in each row. Moving one moves the others.",
        SKILL_RANGE.0, SKILL_RANGE.1
    );
    let skill_limit = at_limit(&values, SKILL_RANGE).map(|(at, limit)| {
        let name = creation.skills[at]
            .skill
            .and_then(|skill| files.skill_names.get(usize::from(skill)).cloned())
            .unwrap_or_else(|| format!("Skill {}", at + 1));
        (name, limit, values[at])
    });
    limit_words(
        ui,
        Pos2::new(skills_area.left(), top + GAP),
        skills_area.width(),
        &skill_rule,
        skill_limit,
    );
}

/// The button of one skill row, which opens a list of the skills with a
/// search field. A skill of another row shows but cannot be picked.
fn skill_picker(
    ui: &mut egui::Ui,
    area: Rect,
    row: usize,
    creation: &mut Creation,
    menu: &[(u8, String)],
) {
    let key = Id::new(("creation-skill", row));
    let pick = creation.skills[row].skill;
    let shown = pick
        .and_then(|skill| menu.iter().find(|(known, _)| *known == skill))
        .map(|(_, name)| name.as_str());
    let response = ui.interact(area, key, Sense::click());
    let fill = if response.hovered() {
        theme::BUTTON_HOVER
    } else {
        theme::BUTTON
    };
    ui.painter()
        .rect_filled(area, CornerRadius::same(RADIUS), fill);
    let (words, color) = match shown {
        Some(name) => (name, theme::TEXT),
        None => (WORDS_PICK_SKILL, theme::WAITING),
    };
    let galley = wrapped(
        ui,
        words,
        text_font(theme::SIZE_BODY),
        color,
        area.width() - STEP_PAD * HALF,
        1,
    );
    ui.painter().galley(
        Pos2::new(
            area.left() + STEP_PAD,
            area.center().y - galley.size().y / HALF,
        ),
        galley,
        color,
    );
    let popup = key.with("list");
    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup));
    }
    let search_key = key.with("search");
    let mut chosen = None;
    egui::popup_below_widget(
        ui,
        popup,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(area.width().max(PICKER_LIST_WIDTH));
            let mut search =
                ui.data_mut(|data| data.get_temp::<String>(search_key).unwrap_or_default());
            let field = ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text(WORDS_SEARCH)
                    .font(text_font(theme::SIZE_BODY)),
            );
            if !field.has_focus() && search.is_empty() {
                field.request_focus();
            }
            egui::ScrollArea::vertical()
                .max_height(PICKER_LIST_HEIGHT)
                .show(ui, |ui| {
                    for (skill, name) in find_skills(menu, &search) {
                        let taken = creation.skill_taken(*skill, row);
                        let words = if taken {
                            format!("{name} {WORDS_TAKEN}")
                        } else {
                            name.clone()
                        };
                        let item = egui::SelectableLabel::new(pick == Some(*skill), words);
                        if ui.add_enabled(!taken, item).clicked() {
                            chosen = Some(*skill);
                        }
                    }
                });
            ui.data_mut(|data| data.insert_temp(search_key, search));
        },
    );
    if let Some(skill) = chosen {
        creation.set_skill(row, skill);
        ui.data_mut(|data| data.remove::<String>(search_key));
        ui.memory_mut(|memory| memory.close_popup());
    }
}

/// The start towns as a list, and the picked one on the map with its words.
fn town_page(
    ui: &mut egui::Ui,
    area: Rect,
    creation: &mut Creation,
    files: &CreationFiles,
    art: &mut Art<'_>,
) {
    heading(ui, area.min, WORDS_TOWN);
    let body_top = area.top() + HEADING_ROW + GAP;
    let list = Rect::from_min_max(
        Pos2::new(area.left(), body_top),
        Pos2::new(area.left() + TOWN_LIST_WIDTH, area.bottom()),
    );
    let mut picked = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("creation-towns")
            .max_height(list.height())
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(GAP / HALF);
                for (at, town) in creation.towns().iter().enumerate() {
                    let (row, response) =
                        ui.allocate_exact_size(Vec2::new(list.width(), TOWN_ROW), Sense::click());
                    if town_row(ui, row, &response, town, creation.town == at) {
                        picked = Some(at);
                    }
                }
            });
    });
    if let Some(at) = picked {
        creation.set_town(at);
    }
    let Some(town) = creation.towns().get(creation.town).cloned() else {
        return;
    };
    let right = Rect::from_min_max(Pos2::new(list.right() + PART_GAP, body_top), area.max);
    let side = right
        .width()
        .min(right.height() - TOWN_WORDS_LEAST)
        .min(MAP_MOST);
    let map_area = Rect::from_min_size(right.min, Vec2::splat(side));
    let words_top = if town_map(ui, map_area, &town, art) {
        map_area.bottom() + GAP
    } else {
        right.top()
    };
    let words = plain_words(&files.town_words(&town, creation.town));
    let words_area = Rect::from_min_max(Pos2::new(right.left(), words_top), right.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(words_area), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("creation-town-words")
            .max_height(words_area.height())
            .show(ui, |ui| {
                let galley = wrapped(
                    ui,
                    &words,
                    text_font(theme::SIZE_BODY),
                    theme::TEXT,
                    words_area.width(),
                    usize::MAX,
                );
                let (spot, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                ui.painter().galley(spot.min, galley, theme::TEXT);
            });
    });
}

/// One row of the list of towns. True when it was clicked.
fn town_row(
    ui: &egui::Ui,
    row: Rect,
    response: &egui::Response,
    town: &StartTown,
    picked: bool,
) -> bool {
    let fill = match (picked, response.hovered()) {
        (true, _) => theme::CHOSEN,
        (false, true) => theme::BUTTON_HOVER,
        (false, false) => theme::BUTTON,
    };
    let radius = CornerRadius::same(RADIUS);
    ui.painter().rect_filled(row, radius, fill);
    if picked {
        let mark = Rect::from_min_size(row.min, Vec2::new(TOWN_MARK, row.height()));
        ui.painter().rect_filled(mark, radius, theme::GOAL);
    }
    let left = row.left() + STEP_PAD + TOWN_MARK;
    ui.painter().text(
        Pos2::new(left, row.top() + GAP),
        Align2::LEFT_TOP,
        &town.name,
        text_font(theme::SIZE_PLATE),
        if picked { theme::GOAL } else { theme::TEXT },
    );
    ui.painter().text(
        Pos2::new(left, row.bottom() - GAP),
        Align2::LEFT_BOTTOM,
        &town.building,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    if let Some(place) = town.place {
        ui.painter().text(
            Pos2::new(row.right() - STEP_PAD, row.top() + GAP),
            Align2::RIGHT_TOP,
            facet_name(place.map),
            text_font(theme::SIZE_SMALL),
            theme::TEXT_FAINT,
        );
    }
    response.clicked()
}

/// The land round a start town with the town marked, when the list gives
/// its place and the client files have the map. True when it was drawn.
fn town_map(ui: &egui::Ui, area: Rect, town: &StartTown, art: &mut Art<'_>) -> bool {
    let (Some(place), Some(scene)) = (town.place, art.scene.as_deref_mut()) else {
        return false;
    };
    let Ok(map) = u8::try_from(place.map) else {
        return false;
    };
    if !art
        .town_map
        .make_near(ui.ctx(), scene, map, (place.x, place.y))
    {
        return false;
    }
    let painter = ui.painter_at(area);
    painter.rect_filled(area, CornerRadius::same(RADIUS), theme::TRACK);
    let (x, y) = (f32::from(place.x), f32::from(place.y));
    let lay = Lay::NorthUp {
        center: area.center(),
        middle: Vec2::new(x, y),
        scale: area.width() / MAP_TILES,
    };
    art.town_map.draw_near(&painter, lay);
    let pin = lay.screen(x, y);
    painter.circle_stroke(pin, PIN_RING, Stroke::new(LINE_WIDTH, theme::GOAL));
    painter.circle(
        pin,
        PIN_RADIUS,
        theme::GOAL,
        Stroke::new(LINE_WIDTH / HALF, theme::VOID),
    );
    theme::shadowed_text(
        &painter,
        pin - Vec2::new(0.0, PIN_RING + GAP / HALF),
        Align2::CENTER_BOTTOM,
        &town.name,
        title_font(theme::SIZE_HEADING),
        theme::TEXT,
    );
    painter.rect_stroke(
        area,
        CornerRadius::same(RADIUS),
        Stroke::new(LINE_WIDTH / HALF, theme::GLASS_EDGE),
        StrokeKind::Inside,
    );
    true
}

/// The rules of a name, as the page lists them.
fn name_rules() -> [String; 3] {
    [
        format!("{NAME_MIN} to {NAME_MAX} characters."),
        "Letters, spaces and the marks - . ' only. Start with a letter, and put a letter between two marks.".into(),
        "No title at the start, such as Lord, Lady, GM or Seer.".into(),
    ]
}

/// The name field with its rules, and all the choices before Create.
fn name_page(
    ui: &mut egui::Ui,
    area: Rect,
    creation: &mut Creation,
    files: &CreationFiles,
    scene: Option<&Scene>,
) {
    let half = (area.width() - PART_GAP) / HALF;
    let left = Rect::from_min_size(area.min, Vec2::new(half, area.height()));
    heading(ui, left.min, WORDS_NAME);
    let field = Rect::from_min_size(
        Pos2::new(left.left(), left.top() + HEADING_ROW + GAP),
        Vec2::new(left.width().min(NAME_FIELD_MOST), NAME_FIELD_HEIGHT),
    );
    ui.painter()
        .rect_filled(field, CornerRadius::same(RADIUS), theme::TRACK);
    let edit = egui::TextEdit::singleline(&mut creation.name)
        .id(Id::new("creation-name"))
        .frame(false)
        .margin(NAME_MARGIN)
        .hint_text(WORDS_NAME_HINT)
        .font(text_font(theme::SIZE_TITLE))
        .text_color(theme::TEXT);
    let response = ui.put(field, edit);
    // The one field of the page takes the keys at once.
    if ui.memory(|memory| memory.focused().is_none()) {
        response.request_focus();
    }
    let fault = creation.blocker();
    let edge = if fault.is_some() {
        theme::WAITING
    } else {
        theme::GOAL
    };
    ui.painter().rect_stroke(
        field,
        CornerRadius::same(RADIUS),
        Stroke::new(LINE_WIDTH / HALF, edge),
        StrokeKind::Inside,
    );
    let (verdict, color) = match fault {
        Some(blocker) => (blocker.words(), theme::WAITING),
        None => (WORDS_NAME_GOOD.to_string(), theme::GOAL),
    };
    let mut top = field.bottom() + GAP;
    ui.painter().text(
        Pos2::new(left.left(), top),
        Align2::LEFT_TOP,
        verdict,
        text_font(theme::SIZE_PLATE),
        color,
    );
    top += LABEL_ROW + SECTION_GAP;
    for rule in name_rules() {
        let text_left = left.left() + RULE_DOT * HALF + GAP;
        let galley = wrapped(
            ui,
            &rule,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
            left.right() - text_left,
            usize::MAX,
        );
        let dot = Pos2::new(
            left.left() + RULE_DOT,
            top + galley.rows.first().map_or(0.0, |row| row.height()) / HALF,
        );
        ui.painter().circle_filled(dot, RULE_DOT, theme::TEXT_DIM);
        let height = galley.size().y;
        ui.painter()
            .galley(Pos2::new(text_left, top), galley, theme::TEXT_DIM);
        top += height + GAP;
    }
    let right = Rect::from_min_max(Pos2::new(left.right() + PART_GAP, area.top()), area.max);
    summary(ui, right, creation, files, scene);
}

/// Every choice of the character, row by row.
fn summary(
    ui: &egui::Ui,
    area: Rect,
    creation: &Creation,
    files: &CreationFiles,
    scene: Option<&Scene>,
) {
    ui.painter()
        .rect_filled(area, CornerRadius::same(RADIUS), theme::TRACK);
    let inner = area.shrink(PART_GAP);
    heading(ui, inner.min, WORDS_SUMMARY);
    let hair = creation
        .hair_styles()
        .get(creation.hair)
        .map_or(String::new(), |style| {
            format!(", {WORDS_HAIR}: {}", style.words)
        });
    let beard = creation
        .beard_styles()
        .and_then(|styles| styles.get(creation.beard))
        .map_or(String::new(), |style| {
            format!(", {WORDS_BEARD}: {}", style.words)
        });
    let profession = creation
        .profession
        .as_ref()
        .map_or(String::new(), |profession| {
            profession_name(profession, files)
        });
    let (stats, skills) = trade_words(creation, files);
    let town = creation
        .towns()
        .get(creation.town)
        .map_or(String::new(), |town| match town.place {
            Some(place) => format!(
                "{}, {} ({})",
                town.name,
                town.building,
                facet_name(place.map)
            ),
            None => format!("{}, {}", town.name, town.building),
        });
    let mut top = inner.top() + HEADING_ROW + GAP;
    let value_left = inner.left() + SUMMARY_LABEL_WIDTH;
    let value_width = inner.right() - value_left;
    let rows = [
        (WORDS_LOOK, format!("{}{hair}{beard}", body_words(creation))),
        (WORDS_COLORS, String::new()),
        (WORDS_TRADE, profession),
        (WORDS_STATS, stats),
        (WORDS_SKILLS, skills),
        (WORDS_START, town),
    ];
    for (words, value) in rows {
        label(ui, Pos2::new(inner.left(), top), words);
        let height = if words == WORDS_COLORS {
            summary_swatches(ui, Pos2::new(value_left, top), creation, scene)
        } else {
            let galley = wrapped(
                ui,
                &value,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
                value_width,
                usize::MAX,
            );
            let height = galley.size().y;
            ui.painter()
                .galley(Pos2::new(value_left, top), galley, theme::TEXT);
            height
        };
        top += height.max(LABEL_ROW) + GAP;
    }
}

/// A small box of each color of the look, with its name on hover. Gives
/// the height of the row.
fn summary_swatches(ui: &egui::Ui, at: Pos2, creation: &Creation, scene: Option<&Scene>) -> f32 {
    let mut left = at.x;
    for paint in shown_paints(creation) {
        let spot = Rect::from_min_size(Pos2::new(left, at.y), Vec2::splat(SUMMARY_SWATCH));
        let color = scene.map_or(theme::BUTTON, |scene| {
            scene.words_color(creation.hue(paint))
        });
        ui.painter()
            .rect_filled(spot, CornerRadius::same(theme::BAR_RADIUS), color);
        ui.painter().rect_stroke(
            spot,
            CornerRadius::same(theme::BAR_RADIUS),
            Stroke::new(LINE_WIDTH / HALF, theme::VOID),
            StrokeKind::Inside,
        );
        let key = Id::new(("creation-summary-color", paint_words(paint, creation.race)));
        ui.interact(spot, key, Sense::hover())
            .on_hover_text(paint_words(paint, creation.race));
        left = spot.right() + GAP;
    }
    SUMMARY_SWATCH
}

#[cfg(test)]
mod tests {
    use super::super::modern::testing::{click, typing, Canvas, SCREEN};
    use super::super::{save_png, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
    use super::*;
    use crate::window::model::creation::{sample_choices, Blocker, NameFault};
    use uoterm_protocol::ClientVersion;
    use uoterm_runtime::CharacterChoices;

    /// The side of the texture the art of the client goes into.
    const ART_TEXTURE_SIDE: usize = 4096;
    const SMALLEST: Vec2 = Vec2::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT);
    /// The folder a test saves its pictures in, when it is set.
    const ENV_PICTURES: &str = "UOTERM_TEST_PICTURES";

    /// A screen with its own context, the map pictures and a canvas.
    struct Screen {
        ctx: egui::Context,
        town_map: MapPictures,
        canvas: Canvas,
    }

    impl Screen {
        fn new() -> Self {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            Self {
                ctx,
                town_map: MapPictures::default(),
                canvas: Canvas::default(),
            }
        }

        /// Draws the screen once for each list of events in a window of
        /// `size`. Gives what each frame asked, and the output of the last.
        fn frames(
            &mut self,
            size: Vec2,
            creation: &mut Creation,
            files: &CreationFiles,
            mut scene: Option<&mut Scene>,
            frames: &[Vec<egui::Event>],
        ) -> (Vec<Option<Asked>>, egui::FullOutput) {
            let window = Rect::from_min_size(Pos2::ZERO, size);
            let mut asked = Vec::new();
            let mut last = None;
            for events in frames {
                let input = egui::RawInput {
                    events: events.clone(),
                    screen_rect: Some(window),
                    max_texture_side: Some(ART_TEXTURE_SIDE),
                    ..egui::RawInput::default()
                };
                let output = self.ctx.run(input, |ctx| {
                    if let Some(scene) = scene.as_deref_mut() {
                        scene.make_atlas(ctx);
                    }
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let art = Art {
                            scene: scene.as_deref_mut(),
                            town_map: &mut self.town_map,
                        };
                        asked.push(draw(ui, window, creation, files, art));
                    });
                });
                self.canvas.take(&output.textures_delta);
                last = Some(output);
            }
            (asked, last.expect("at least one frame"))
        }
    }

    fn key(key: Key) -> Vec<egui::Event> {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }]
    }

    /// The words the frame drew that do not show whole: outside the window,
    /// or cut by the part of the screen they are in.
    fn cut_words(output: &egui::FullOutput, size: Vec2) -> Vec<String> {
        fn texts<'a>(shape: &'a Shape, found: &mut Vec<&'a egui::epaint::TextShape>) {
            match shape {
                Shape::Text(text) => found.push(text),
                Shape::Vec(shapes) => shapes.iter().for_each(|shape| texts(shape, found)),
                _ => {}
            }
        }
        let window = Rect::from_min_size(Pos2::ZERO, size);
        let mut cut = Vec::new();
        for clipped in &output.shapes {
            let mut found = Vec::new();
            texts(&clipped.shape, &mut found);
            for text in found {
                if text.galley.text().is_empty() {
                    continue;
                }
                let area = text.visual_bounding_rect();
                let room = clipped.clip_rect.intersect(window).expand(1.0);
                if !room.contains_rect(area) {
                    cut.push(text.galley.text().to_string());
                }
            }
        }
        cut
    }

    /// A creation on each page, with a template profession, all skills
    /// picked for the Custom page, a town and a name.
    fn on_each_page(files: &CreationFiles) -> Vec<Creation> {
        let mut look = Creation::new(ClientVersion::MODERN, sample_choices());
        look.beard = 1;
        let mut profession = look.clone();
        profession.step = Step::Profession(None);
        profession.pick_profession(files.professions.top()[0], files);
        let mut trade = profession.clone();
        let custom = files.professions.top().last().copied().expect("Advanced");
        trade.pick_profession(custom, files);
        trade.next();
        let skills: Vec<u8> = trade
            .skill_menu(files)
            .iter()
            .take(4)
            .map(|(n, _)| *n)
            .collect();
        for (at, skill) in skills.into_iter().enumerate() {
            trade.set_skill(at, skill);
        }
        trade.set_stat(0, 45);
        let mut town = profession.clone();
        town.step = Step::Town;
        town.set_town(2);
        let mut name = town.clone();
        name.step = Step::Name;
        name.name = "Mara Dell".into();
        vec![look, profession, trade, town, name]
    }

    #[test]
    fn the_words_of_the_client_lose_their_tags_and_a_card_keeps_one_sentence() {
        assert_eq!(
            plain_words("<b>Yew</b> is a town.<BR>In the woods."),
            "Yew is a town.\nIn the woods."
        );
        assert_eq!(plain_words("<br/>plain"), "plain");
        assert_eq!(first_sentence("A mage. Casts spells."), "A mage.");
        assert_eq!(first_sentence("One line\nTwo"), "One line");
    }

    #[test]
    fn the_figure_grows_by_whole_steps_and_a_large_one_shrinks_to_fit() {
        let room = Vec2::new(250.0, 300.0);
        assert_eq!(preview_scale(room, Vec2::new(44.0, 70.0)), 4.0);
        assert_eq!(
            preview_scale(room, Vec2::new(10.0, 10.0)),
            PREVIEW_MOST_SCALE
        );
        let large = preview_scale(room, Vec2::new(500.0, 300.0));
        assert!((large - 0.5).abs() < f32::EPSILON);
        assert_eq!(columns_in(726.0, CARD_MIN_WIDTH), 2);
        assert_eq!(columns_in(10.0, CARD_MIN_WIDTH), 1);
    }

    #[test]
    fn the_frame_and_the_color_grids_fit_the_smallest_and_the_usual_window() {
        for size in [SMALLEST, SCREEN] {
            let window = Rect::from_min_size(Pos2::ZERO, size);
            let parts = frame(window);
            assert!(window.contains_rect(parts.panel));
            for part in [parts.header, parts.preview, parts.content, parts.footer] {
                assert!(parts.panel.contains_rect(part), "{size:?}");
            }
            assert!(!parts.preview.intersects(parts.content));
            assert!(parts.header.bottom() < parts.content.top());
            assert!(parts.content.bottom() < parts.footer.top());
            let creation = Creation::new(ClientVersion::MODERN, CharacterChoices::default());
            let palettes: Vec<Palette> = shown_paints(&creation)
                .iter()
                .map(|paint| creation.palette(*paint))
                .collect();
            let (split, cell) = grid_layout(parts.content.size(), &palettes);
            for row in [&palettes[..split], &palettes[split..]] {
                let columns: usize = row.iter().map(|palette| palette.columns).sum();
                let gaps = PART_GAP * row.len().saturating_sub(1) as f32;
                assert!(cell * columns as f32 + gaps <= parts.content.width());
            }
            assert!(cell >= GAP, "a color box is large enough to click: {cell}");
        }
    }

    #[test]
    fn the_color_grids_take_two_rows_when_that_makes_the_boxes_larger() {
        let creation = Creation::new(ClientVersion::MODERN, CharacterChoices::default());
        let palettes: Vec<Palette> = shown_paints(&creation)
            .iter()
            .map(|paint| creation.palette(*paint))
            .collect();
        let columns: usize = palettes.iter().map(|palette| palette.columns).sum();
        let low = Vec2::new(742.0, 220.0);
        assert_eq!(grid_layout(low, &palettes).0, palettes.len(), "one row");
        let tall = Vec2::new(894.0, 380.0);
        let (split, cell) = grid_layout(tall, &palettes);
        assert_eq!(
            shown_paints(&creation)[split..],
            [Paint::Shirt, Paint::Pants],
            "the body colors, then the clothes"
        );
        assert!(cell > (tall.x - PART_GAP * (palettes.len() - 1) as f32) / columns as f32);
        assert!(cell <= CELL_MOST);
    }

    /// Every page draws at the smallest and the usual window, and all its
    /// words show whole.
    #[test]
    fn every_page_shows_all_its_words_at_both_sizes() {
        let files = CreationFiles::sample();
        for size in [SMALLEST, SCREEN] {
            for mut creation in on_each_page(&files) {
                let mut screen = Screen::new();
                let step = creation.step.clone();
                let (asked, output) =
                    screen.frames(size, &mut creation, &files, None, &[Vec::new(), Vec::new()]);
                assert_eq!(asked, vec![None, None]);
                let cut = cut_words(&output, size);
                assert!(cut.is_empty(), "{step:?} at {size:?} cuts {cut:?}");
            }
        }
    }

    #[test]
    fn next_stays_off_with_its_reason_until_the_page_is_whole() {
        let files = CreationFiles::sample();
        let mut creation = Creation::new(ClientVersion::MODERN, sample_choices());
        creation.step = Step::Name;
        let parts = frame(Rect::from_min_size(Pos2::ZERO, SCREEN));
        let next = Pos2::new(
            parts.footer.right() - BIG_BUTTON_WIDTH / HALF,
            parts.footer.center().y,
        );
        let mut screen = Screen::new();
        let (asked, output) = screen.frames(SCREEN, &mut creation, &files, None, &click(next));
        assert!(asked.iter().all(Option::is_none), "no name: no Create");
        assert_eq!(creation.step, Step::Name);
        let reason = Blocker::Name(NameFault::Empty).words();
        let shown = output.shapes.iter().any(
            |clipped| matches!(&clipped.shape, Shape::Text(text) if text.galley.text() == reason),
        );
        assert!(shown, "the reason shows by Next");
        let (asked, _) = screen.frames(
            SCREEN,
            &mut creation,
            &files,
            None,
            &[typing("Mara"), key(Key::Enter)],
        );
        assert_eq!(creation.name, "Mara");
        assert_eq!(asked.last(), Some(&Some(Asked::Finish)));
    }

    #[test]
    fn enter_goes_on_esc_goes_back_and_the_arrows_turn_the_figure() {
        let files = CreationFiles::sample();
        let mut creation = Creation::new(ClientVersion::MODERN, sample_choices());
        let mut screen = Screen::new();
        let frames = [key(Key::Enter), key(Key::ArrowRight), key(Key::Escape)];
        let (asked, _) = screen.frames(SCREEN, &mut creation, &files, None, &frames);
        assert_eq!(asked, vec![None, None, None]);
        assert_eq!((creation.step.clone(), creation.turns), (Step::Look, 1));
        creation.race = Race::Elf;
        creation.choices.list_flags = 0;
        let (asked, _) = screen.frames(SCREEN, &mut creation, &files, None, &[key(Key::Enter)]);
        assert_eq!(creation.step, Step::Look, "a locked race stays");
        assert_eq!(asked, vec![None]);
        let (asked, _) = screen.frames(SCREEN, &mut creation, &files, None, &[key(Key::Escape)]);
        assert_eq!(asked, vec![Some(Asked::Leave)]);
    }

    #[test]
    fn a_click_on_a_card_picks_the_profession_and_a_click_on_a_color_picks_it() {
        let files = CreationFiles::sample();
        let mut creation = Creation::new(ClientVersion::MODERN, sample_choices());
        let parts = frame(Rect::from_min_size(Pos2::ZERO, SCREEN));
        let mut screen = Screen::new();
        // The skin grid is the first, and the hair colors stand next to
        // it, past its eight columns: a click goes on its second box.
        let (_, output) = screen.frames(SCREEN, &mut creation, &files, None, &[Vec::new()]);
        let label_at = |words: &str| {
            output
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    Shape::Text(text) if text.galley.text() == words => Some(text.pos),
                    _ => None,
                })
                .expect("the label of a grid")
        };
        let skin = label_at("Skin");
        let skin_columns = creation.palette(Paint::Skin).columns as f32;
        let cell = (label_at("Hair color").x - skin.x - PART_GAP) / skin_columns;
        let second = Pos2::new(skin.x + cell * 1.5, skin.y + LABEL_ROW + cell / HALF);
        screen.frames(SCREEN, &mut creation, &files, None, &click(second));
        assert_eq!(creation.color_place(Paint::Skin), 1);
        creation.step = Step::Profession(None);
        let first_card = Pos2::new(
            parts.content.left() + CARD_MIN_WIDTH / HALF,
            parts.content.top() + HEADING_ROW + LABEL_ROW + GAP + CARD_HEIGHT / HALF,
        );
        screen.frames(SCREEN, &mut creation, &files, None, &click(first_card));
        let picked = creation.profession.as_ref().map(|p| p.true_name.clone());
        assert_eq!(picked.as_deref(), Some("warrior"));
    }

    /// Draws each page at both sizes with the real client files, and saves
    /// the pictures in the folder of `UOTERM_TEST_PICTURES` to look at.
    #[test]
    fn pictures_of_each_page_with_the_client_art() {
        let (Some(dir), Some(out)) = (
            uoterm_nav::client_data_dir_from_env(),
            std::env::var_os(ENV_PICTURES),
        ) else {
            return;
        };
        let out = std::path::PathBuf::from(out);
        let files = CreationFiles::read(Some(&dir));
        let mut scene = Scene::new(Some(&dir));
        // One context for all: the art of the scene is in its textures.
        let mut screen = Screen::new();
        // An elf woman turned to her right, to show another look.
        let mut elf = Creation::new(ClientVersion::MODERN, sample_choices());
        elf.set_race(Race::Elf);
        elf.set_female(true);
        elf.hair = 2;
        elf.set_color(Paint::Hair, 1);
        elf.set_color(Paint::Shirt, 40);
        elf.turn(true);
        for size in [SMALLEST, SCREEN] {
            let mut pages = on_each_page(&files);
            pages.push(elf.clone());
            for (at, mut creation) in pages.into_iter().enumerate() {
                let frames = [Vec::new(), Vec::new(), Vec::new()];
                let (_, output) =
                    screen.frames(size, &mut creation, &files, Some(&mut scene), &frames);
                let picture = screen.canvas.paint(&screen.ctx, output, size);
                let name = format!("creation-{at}-{}x{}.png", size.x, size.y);
                save_png(&out.join(name), &picture).expect("the picture is saved");
            }
        }
        let look = Creation::new(ClientVersion::MODERN, sample_choices());
        assert!(scene
            .turned_picture(FIGURE_MAP, &look.look(), DOLL_FACING)
            .is_some());
    }
}
