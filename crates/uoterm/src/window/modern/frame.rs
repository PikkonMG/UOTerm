//! The frame of every Modern panel the player moves: the glass, the title
//! that drags it, the corner that sizes it, and the lock, close and fold
//! marks. The profile keeps where each panel was left, and whether it is
//! folded to its title.

use super::super::boxes_ui::Tools;
use super::super::model::places;
use super::super::settings::Profile;
use super::super::theme::{self, title_font};
use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};

pub const TITLE_ROW: f32 = 28.0;
/// A panel is at least this wide, so its title and marks fit.
const MIN_WIDTH: f32 = 170.0;
const MARK_SIDE: f32 = 18.0;
const MARK_GAP: f32 = 6.0;
const MARK_STROKE: f32 = 1.5;
/// The most marks the frame puts at the right end of a title: the fold
/// mark, the lock and the close mark.
const MOST_MARKS: usize = 3;
const GRIP_SIDE: f32 = 16.0;
const GRIP_MARKS: [f32; 3] = [4.0, 8.0, 12.0];
/// The padlock stands this far inside its mark area, so its stroke never
/// leaves it.
const LOCK_INSET: f32 = MARK_STROKE;
/// The body of the padlock, as shares of the room of the padlock.
const LOCK_BODY_HEIGHT: f32 = 0.45;
const LOCK_BODY_WIDTH: f32 = 0.72;
const LOCK_BODY_ROUNDING: u8 = 2;
/// The round top of the shackle, as a share of the width of the padlock.
const LOCK_SHACKLE_RADIUS: f32 = 0.2;
/// How far an open shackle stands raised, as a share of the height of the
/// padlock.
const LOCK_SHACKLE_LIFT: f32 = 0.17;
/// The round top of the shackle is drawn as this many straight pieces.
const LOCK_ARC_PIECES: usize = 12;
/// The arms of the fold mark reach this share of its side from the middle.
const CHEVRON_SHARE: f32 = 0.25;
/// A folded panel shows its title alone.
pub const FOLDED_HEIGHT: f32 = TITLE_ROW + theme::PANEL_PAD * 2.0;
const HINT_MOVE: &str = "Drag: move.  Double-click: put it back.";
const HINT_LOCK: &str = "Lock or free the panel.";
const HINT_CLOSE: &str = "Close.";
const HINT_SIZE: &str = "Drag: size.";
const HINT_FOLD: &str = "Fold to the title, or unfold.";

/// What a panel is: its id in the profile, its title, where it stands
/// before the player moves it, and whether he sizes and closes it.
pub struct PanelSpec<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub default: Rect,
    /// The smallest size of a panel the player can size. None for a panel
    /// of a fixed size.
    pub min_size: Option<Vec2>,
    pub closable: bool,
}

/// What the player did to the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameEvent {
    Closed,
}

/// A size made wide enough for the title and the marks.
pub fn with_title_room(size: Vec2) -> Vec2 {
    size.max(Vec2::new(MIN_WIDTH, 0.0))
}

/// Where the panel stands now.
pub fn place(window: Rect, spec: &PanelSpec<'_>, profile: &Profile) -> Rect {
    places::placed_rect(
        window,
        spec.default,
        places::kept(profile, spec.id),
        spec.min_size,
    )
}

/// The place a panel takes: its title alone when it is folded.
pub fn shown_rect(rect: Rect, folded: bool) -> Rect {
    if folded {
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), FOLDED_HEIGHT))
    } else {
        rect
    }
}

/// Draws the glass and the title. Gives the room under the title.
pub fn draw(painter: &egui::Painter, rect: Rect, title: &str) -> Rect {
    draw_tinted(painter, rect, title, theme::GLASS, theme::GLASS_EDGE)
}

/// Draws a glass of its own fill and edge, and the title.
pub fn draw_tinted(
    painter: &egui::Painter,
    rect: Rect,
    title: &str,
    fill: Color32,
    edge: Color32,
) -> Rect {
    theme::panel_with(painter, rect, fill, edge);
    draw_title(painter, title_room(rect, MOST_MARKS), title);
    let inner = rect.shrink(theme::PANEL_PAD);
    Rect::from_min_max(inner.left_top() + Vec2::new(0.0, TITLE_ROW), inner.max)
}

/// The room of the title of a panel: the title row, left of `marks` marks.
pub fn title_room(rect: Rect, marks: usize) -> Rect {
    let inner = rect.shrink(theme::PANEL_PAD);
    let right = if marks == 0 {
        inner.right()
    } else {
        mark_area(rect, marks - 1).left() - MARK_GAP
    };
    Rect::from_min_max(
        inner.min,
        Pos2::new(right.max(inner.left()), inner.top() + TITLE_ROW),
    )
}

/// Writes a title in its room, cut short with an ellipsis when it is
/// longer.
pub fn draw_title(painter: &egui::Painter, room: Rect, title: &str) {
    let mut job =
        LayoutJob::simple_singleline(title.to_owned(), title_font(theme::SIZE_TITLE), theme::TEXT);
    job.wrap = TextWrapping::truncate_at_width(room.width());
    let galley = painter.layout_job(job);
    painter
        .with_clip_rect(room)
        .galley(room.left_top(), galley, theme::TEXT);
}

/// The parts of a padlock: its body, and its shackle as one line from the
/// foot of the left leg over the round top to the foot of the right leg.
struct Padlock {
    body: Rect,
    shackle: Vec<Pos2>,
}

/// A padlock inside its mark area. Shut, both legs of the shackle go into
/// the body; open, the shackle stands raised and its right leg is out of
/// the body.
fn padlock(area: Rect, locked: bool) -> Padlock {
    let room = area.shrink(LOCK_INSET);
    let body = Rect::from_min_max(
        Pos2::new(
            room.center().x - room.width() * LOCK_BODY_WIDTH / 2.0,
            room.bottom() - room.height() * LOCK_BODY_HEIGHT,
        ),
        Pos2::new(
            room.center().x + room.width() * LOCK_BODY_WIDTH / 2.0,
            room.bottom(),
        ),
    );
    let radius = room.width() * LOCK_SHACKLE_RADIUS;
    let lift = if locked {
        room.height() * LOCK_SHACKLE_LIFT
    } else {
        0.0
    };
    let middle = Pos2::new(room.center().x, room.top() + radius + lift);
    // Shut, the legs reach the body; open, the right leg ends as far over
    // the body as the shackle is raised.
    let right_foot = body.top() - room.height() * LOCK_SHACKLE_LIFT + lift;
    let mut shackle = vec![Pos2::new(middle.x - radius, body.top())];
    shackle.extend((0..=LOCK_ARC_PIECES).map(|piece| {
        let turn = std::f32::consts::PI * (1.0 + piece as f32 / LOCK_ARC_PIECES as f32);
        middle + Vec2::angled(turn) * radius
    }));
    shackle.push(Pos2::new(middle.x + radius, right_foot));
    Padlock { body, shackle }
}

/// A padlock: shut when locked, open when free.
fn lock_mark(painter: &egui::Painter, area: Rect, locked: bool, color: Color32) {
    let lock = padlock(area, locked);
    painter.add(egui::Shape::line(
        lock.shackle,
        Stroke::new(MARK_STROKE, color),
    ));
    painter.rect_filled(lock.body, CornerRadius::same(LOCK_BODY_ROUNDING), color);
}

/// A chevron that points down on a folded panel, which it unfolds, and up
/// on an open one.
fn fold_mark(painter: &egui::Painter, area: Rect, folded: bool, color: Color32) {
    let arm = area.width() * CHEVRON_SHARE;
    let middle = area.center();
    let (tip, ends) = if folded {
        (middle.y + arm / 2.0, middle.y - arm / 2.0)
    } else {
        (middle.y - arm / 2.0, middle.y + arm / 2.0)
    };
    let stroke = Stroke::new(MARK_STROKE, color);
    let tip = Pos2::new(middle.x, tip);
    painter.line_segment([Pos2::new(middle.x - arm, ends), tip], stroke);
    painter.line_segment([tip, Pos2::new(middle.x + arm, ends)], stroke);
}

fn close_mark(painter: &egui::Painter, area: Rect, color: Color32) {
    let arm = area.shrink(area.width() / 4.0);
    let stroke = Stroke::new(MARK_STROKE, color);
    painter.line_segment([arm.left_top(), arm.right_bottom()], stroke);
    painter.line_segment([arm.right_top(), arm.left_bottom()], stroke);
}

/// The places of the marks at the right end of the title, right first.
pub fn mark_area(rect: Rect, from_right: usize) -> Rect {
    let inner = rect.shrink(theme::PANEL_PAD);
    let right = inner.right() - from_right as f32 * (MARK_SIDE + MARK_GAP);
    Rect::from_min_size(
        Pos2::new(
            right - MARK_SIDE,
            inner.top() + (TITLE_ROW - MARK_SIDE) / 2.0 - MARK_GAP / 2.0,
        ),
        Vec2::splat(MARK_SIDE),
    )
}

/// How many marks the title of a panel holds at its right end.
pub fn marks(spec: &PanelSpec<'_>) -> usize {
    1 + usize::from(spec.closable)
}

/// Takes the drags on the title and on the corner, and the clicks on the
/// lock and on the close mark. Call it after the body, so these take
/// their clicks first.
pub fn controls(
    ui: &egui::Ui,
    rect: Rect,
    spec: &PanelSpec<'_>,
    profile: &mut Profile,
    tools: &Tools<'_>,
) -> Option<FrameEvent> {
    controls_leaving(ui, rect, spec, 0, profile, tools)
}

/// The controls of a panel that folds to its title, with the fold mark
/// left of the lock. `rect` is the whole panel, unfolded.
pub fn foldable_controls(
    ui: &egui::Ui,
    rect: Rect,
    spec: &PanelSpec<'_>,
    profile: &mut Profile,
    tools: &Tools<'_>,
) -> Option<FrameEvent> {
    let folded = places::is_folded(profile, spec.id);
    let area = mark_area(rect, marks(spec));
    let response = ui.interact(area, Id::new(("panel-fold", spec.id)), Sense::click());
    fold_mark(ui.painter(), area, folded, mark_color(response.hovered()));
    if response.hovered() {
        super::super::tips::label(ui, HINT_FOLD, "");
    }
    if response.clicked() {
        places::set_folded(profile, spec.id, !folded);
        tools.keep_profile(profile);
    }
    controls_leaving(ui, rect, spec, 1, profile, tools)
}

/// The controls, with room for `more_marks` marks of the panel's own left
/// of the lock, such as a fold mark or a switch of its own.
pub fn controls_leaving(
    ui: &egui::Ui,
    rect: Rect,
    spec: &PanelSpec<'_>,
    more_marks: usize,
    profile: &mut Profile,
    tools: &Tools<'_>,
) -> Option<FrameEvent> {
    let folded = places::is_folded(profile, spec.id);
    let locked = places::is_locked(profile, spec.id);
    let sized = spec.min_size.is_some();
    let painter = ui.painter();
    let mut event = None;
    let mut next = 0;
    if spec.closable {
        let area = mark_area(rect, next);
        next += 1;
        let response = ui.interact(area, Id::new(("panel-close", spec.id)), Sense::click());
        close_mark(painter, area, mark_color(response.hovered()));
        if response.hovered() {
            super::super::tips::label(ui, HINT_CLOSE, "");
        }
        if response.clicked() {
            event = Some(FrameEvent::Closed);
        }
    }
    let area = mark_area(rect, next);
    let response = ui.interact(area, Id::new(("panel-lock", spec.id)), Sense::click());
    lock_mark(
        painter,
        area,
        locked,
        mark_color(response.hovered() || locked),
    );
    if response.hovered() {
        super::super::tips::label(ui, HINT_LOCK, "");
    }
    if response.clicked() {
        places::set_locked(profile, spec.id, rect, sized, !locked);
        tools.keep_profile(profile);
    }
    if locked {
        return event;
    }
    let title = Rect::from_min_max(
        rect.min,
        Pos2::new(
            mark_area(rect, next + more_marks).left() - MARK_GAP,
            rect.top() + TITLE_ROW + theme::PANEL_PAD,
        ),
    );
    let mover = ui.interact(
        title,
        Id::new(("panel-move", spec.id)),
        Sense::click_and_drag(),
    );
    if mover.double_clicked() {
        places::forget(profile, spec.id);
        tools.keep_profile(profile);
    } else if mover.dragged() {
        places::remember(profile, spec.id, rect.translate(mover.drag_delta()), sized);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if mover.hovered() {
        super::super::tips::label(ui, HINT_MOVE, "");
    }
    let mut stopped = mover.drag_stopped();
    if let Some(min) = spec.min_size.filter(|_| !folded) {
        let grip = Rect::from_min_max(rect.max - Vec2::splat(GRIP_SIDE), rect.max);
        for mark in GRIP_MARKS {
            painter.line_segment(
                [
                    Pos2::new(rect.right() - mark, rect.bottom()),
                    Pos2::new(rect.right(), rect.bottom() - mark),
                ],
                Stroke::new(MARK_STROKE, theme::TEXT_FAINT),
            );
        }
        let sizer = ui.interact(grip, Id::new(("panel-size", spec.id)), Sense::drag());
        if sizer.dragged() {
            let size = (rect.size() + sizer.drag_delta()).max(min);
            places::remember(profile, spec.id, Rect::from_min_size(rect.min, size), true);
        }
        if sizer.hovered() || sizer.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            super::super::tips::label(ui, HINT_SIZE, "");
        }
        stopped |= sizer.drag_stopped();
    }
    if stopped {
        tools.keep_profile(profile);
    }
    event
}

fn mark_color(bright: bool) -> Color32 {
    if bright {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folded_panel_keeps_its_title_and_the_marks_stand_right_first() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(300.0, 200.0));
        assert_eq!(shown_rect(rect, false), rect);
        let folded = shown_rect(rect, true);
        assert_eq!((folded.min, folded.width()), (rect.min, rect.width()));
        assert_eq!(folded.height(), FOLDED_HEIGHT);
        assert!(mark_area(rect, 1).right() < mark_area(rect, 0).left());
        let spec = PanelSpec {
            id: "test",
            title: "",
            default: rect,
            min_size: None,
            closable: true,
        };
        assert_eq!(marks(&spec), 2, "the close mark and the lock");
    }

    /// A padlock and its stroke stay whole inside the mark area.
    fn inside(area: Rect, lock: &Padlock) -> bool {
        let room = area.shrink(MARK_STROKE / 2.0);
        room.contains_rect(lock.body) && lock.shackle.iter().all(|point| room.contains(*point))
    }

    #[test]
    fn the_padlock_is_whole_in_its_mark_and_opens_one_leg() {
        let area = mark_area(Rect::from_min_size(Pos2::ZERO, Vec2::splat(300.0)), 1);
        for locked in [true, false] {
            let lock = padlock(area, locked);
            assert!(inside(area, &lock), "locked: {locked}");
            let (first, last) = (lock.shackle[0], lock.shackle[lock.shackle.len() - 1]);
            assert!(first.x < last.x, "the left leg, then the right one");
            assert!(lock.body.x_range().contains(first.x) && lock.body.x_range().contains(last.x));
            assert_eq!(first.y, lock.body.top(), "the left leg always goes in");
            let top = lock
                .shackle
                .iter()
                .map(|point| point.y)
                .fold(f32::MAX, f32::min);
            assert!(top < first.y, "the shackle rises over the body");
        }
        let shut = padlock(area, true);
        let open = padlock(area, false);
        let foot = |lock: &Padlock| lock.shackle[lock.shackle.len() - 1].y;
        assert_eq!(foot(&shut), shut.body.top(), "shut, the right leg goes in");
        assert!(foot(&open) < open.body.top(), "open, the right leg is out");
        let top = |lock: &Padlock| lock.shackle.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        assert!(top(&open) < top(&shut), "open, the shackle stands raised");
    }

    #[test]
    fn the_fold_and_close_marks_stay_in_their_areas() {
        let area = mark_area(Rect::from_min_size(Pos2::ZERO, Vec2::splat(300.0)), 0);
        let room = area.shrink(MARK_STROKE / 2.0);
        let arm = area.width() * CHEVRON_SHARE;
        assert!(room.contains_rect(Rect::from_center_size(
            area.center(),
            Vec2::splat(arm * 2.0)
        )));
        assert!(room.contains_rect(area.shrink(area.width() / 4.0)));
    }

    #[test]
    fn the_marks_and_the_title_never_lie_on_each_other_at_any_width() {
        const WIDTHS: [f32; 4] = [MIN_WIDTH, 240.0, 400.0, 900.0];
        for width in WIDTHS {
            let rect = Rect::from_min_size(Pos2::new(5.0, 7.0), Vec2::new(width, 200.0));
            for mark in 0..MOST_MARKS {
                let area = mark_area(rect, mark);
                assert!(rect.shrink(theme::PANEL_PAD / 2.0).contains_rect(area));
                if mark > 0 {
                    assert!(area.right() < mark_area(rect, mark - 1).left());
                }
            }
            for marks in 1..=MOST_MARKS {
                let title = title_room(rect, marks);
                assert!(title.right() < mark_area(rect, marks - 1).left(), "{width}");
                assert!(title.width() > 0.0, "{width}");
            }
        }
    }
}
