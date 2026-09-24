//! The context menu of the classic client:
//! a dark see-through list of lines by the pointer, over every gump. A line
//! that is on shows a tick; a line with a list of its own shows it beside
//! itself while the pointer is on it. A click on a line picks it and shuts
//! the menu; a click anywhere else shuts it.

use super::canvas::{Canvas, CanvasInput};
use super::text::TextLook;
use eframe::egui::{self, Color32, Id, Order, Pos2, Rect, Vec2};
use std::hash::Hash;

/// The menu opens this far right of the pointer and this far above it.
const OPEN_OFFSET: Vec2 = Vec2::new(5.0, -20.0);
const LINE_HEIGHT: i32 = 25;
const LEAST_WIDTH: i32 = 100;
const WORDS_X: i32 = 25;
const WORDS_RIGHT_ROOM: i32 = 20;
const TICK: u16 = 0x0838;
const TICK_X: i32 = 3;
const WORDS_FONT: u8 = 1;
const WORDS_HUE: u16 = 0xFFFF;
const BACKGROUND_OPACITY: f32 = 0.7;
const NO_HUE: u16 = 0;
const BORDER: Color32 = Color32::GRAY;
/// The gray bar under the line the pointer is on.
const HOVER: Color32 = Color32::GRAY;
const HOVER_INSET: (i32, i32) = (2, 5);
const MORE: &str = ">";
const HALF: i32 = 2;
const EDGE: i32 = 1;

/// One line of a context menu. A line with no words is a gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuLine {
    pub words: String,
    /// Some for a line that is on or off.
    pub ticked: Option<bool>,
    /// The lines of its own list.
    pub lines: Vec<MenuLine>,
}

impl MenuLine {
    pub fn plain(words: impl Into<String>) -> Self {
        Self {
            words: words.into(),
            ticked: None,
            lines: Vec::new(),
        }
    }

    /// A line that is on or off.
    pub fn tick(words: impl Into<String>, on: bool) -> Self {
        Self {
            ticked: Some(on),
            ..Self::plain(words)
        }
    }

    /// A line with a list of its own.
    pub fn list(words: impl Into<String>, lines: Vec<MenuLine>) -> Self {
        Self {
            lines,
            ..Self::plain(words)
        }
    }

    pub fn gap() -> Self {
        Self::plain(String::new())
    }
}

/// The width of a list: the widest line, and never less than the least.
fn list_width(word_widths: &[i32]) -> i32 {
    word_widths
        .iter()
        .map(|width| WORDS_X + width + WORDS_RIGHT_ROOM)
        .fold(LEAST_WIDTH, i32::max)
}

/// Where a menu of this size opens for a pointer, held inside the screen.
fn open_place(pointer: Pos2, size: Vec2, screen: Rect) -> Pos2 {
    let at = pointer + OPEN_OFFSET;
    Pos2::new(
        at.x.min(screen.right() - size.x).max(screen.left()),
        at.y.min(screen.bottom() - size.y).max(screen.top()),
    )
}

/// An open or shut context menu of one gump.
#[derive(Default)]
pub struct ContextMenu {
    /// Where the pointer was when it opened, in window points.
    pointer: Option<Pos2>,
    /// The line whose own list shows, at each depth.
    open_lists: Vec<usize>,
    /// It opened in this frame, so the click that opened it does not shut
    /// it.
    fresh: bool,
}

impl ContextMenu {
    /// Opens the menu by the pointer.
    pub fn open_at(&mut self, pointer: Pos2) {
        self.pointer = Some(pointer);
        self.open_lists.clear();
        self.fresh = true;
    }

    pub fn is_open(&self) -> bool {
        self.pointer.is_some()
    }

    /// Draws the menu when it is open, `scale` window points for each
    /// pixel. Gives the line picked: its place in the menu, then in each
    /// list of its own.
    pub fn show(
        &mut self,
        g: &mut Canvas<'_>,
        key: impl Hash,
        scale: f32,
        lines: &[MenuLine],
    ) -> Option<Vec<usize>> {
        let pointer = self.pointer?;
        let ctx = g.ctx().clone();
        let id = Id::new(("classic-context-menu", key));
        let widths = measure(g, lines);
        let size = Vec2::new(
            list_width(&widths) as f32,
            (lines.len() as i32 * LINE_HEIGHT) as f32,
        ) * scale;
        let origin = open_place(pointer, size, ctx.screen_rect());
        let input = CanvasInput {
            id,
            origin,
            scale,
            alpha: 1.0,
            pointer: ctx.pointer_hover_pos(),
            body_click: None,
            body_double_click: false,
            right_click: false,
            size: None,
            map: 0,
        };
        let (scene, text) = (&mut *g.scene, &mut *g.text);
        let mut picked = None;
        let mut areas = Vec::new();
        egui::Area::new(id.with("area"))
            .order(Order::Foreground)
            .fixed_pos(origin)
            .constrain(false)
            .show(&ctx, |ui| {
                let mut menu = Canvas::new(ui, scene, text, input);
                picked = self.list(&mut menu, lines, (0, 0), 0, &mut areas);
            });
        let clicked = ctx.input(|i| i.pointer.any_click());
        let outside = ctx
            .pointer_interact_pos()
            .is_none_or(|at| !areas.iter().any(|area: &Rect| area.contains(at)));
        if picked.is_some() || (clicked && outside && !self.fresh) {
            self.pointer = None;
        }
        self.fresh = false;
        picked
    }

    /// Draws one list at a place and the list of its own that shows.
    fn list(
        &mut self,
        g: &mut Canvas<'_>,
        lines: &[MenuLine],
        (x, y): (i32, i32),
        depth: usize,
        areas: &mut Vec<Rect>,
    ) -> Option<Vec<usize>> {
        let width = list_width(&measure(g, lines));
        let height = lines.len() as i32 * LINE_HEIGHT;
        areas.push(g.area(x, y, Vec2::new(width as f32, height as f32)));
        g.fill(x - EDGE, y - EDGE, width + EDGE * HALF, EDGE, BORDER);
        g.fill(x - EDGE, y + height, width + EDGE * HALF, EDGE, BORDER);
        g.fill(x - EDGE, y, EDGE, height, BORDER);
        g.fill(x + width, y, EDGE, height, BORDER);
        g.shade(x, y, width, height, NO_HUE, BACKGROUND_OPACITY);
        let look = TextLook::unicode(WORDS_FONT, WORDS_HUE).bordered();
        let mut picked = None;
        for (at, line) in lines.iter().enumerate() {
            let top = y + at as i32 * LINE_HEIGHT;
            let response = g.click_area(("line", depth, at), x, top, width, LINE_HEIGHT);
            let words = !line.words.trim().is_empty();
            if words && response.hovered() {
                g.fill(
                    x + HOVER_INSET.0,
                    top + HOVER_INSET.1,
                    width - HOVER_INSET.0 * HALF,
                    LINE_HEIGHT - HOVER_INSET.1 * HALF,
                    HOVER,
                );
                if !line.lines.is_empty() {
                    self.open_lists.truncate(depth);
                    self.open_lists.push(at);
                }
            }
            if line.ticked == Some(true) {
                let tick = g.gump_size(TICK).unwrap_or(Vec2::ZERO);
                g.pic(
                    x + TICK_X,
                    top + (LINE_HEIGHT - tick.y as i32) / HALF,
                    TICK,
                    NO_HUE,
                );
            }
            let size = g.measure(&line.words, &look);
            let words_y = top + (LINE_HEIGHT - size.y as i32) / HALF;
            g.label(x + WORDS_X, words_y, &line.words, &look);
            if !line.lines.is_empty() {
                let more = g.measure(MORE, &look);
                g.label(x + width - more.x as i32, words_y, MORE, &look);
            }
            if words && line.lines.is_empty() && response.clicked() {
                picked = Some(vec![at]);
            }
        }
        if let Some(&open) = self.open_lists.get(depth) {
            if let Some(line) = lines.get(open).filter(|line| !line.lines.is_empty()) {
                let top = y + open as i32 * LINE_HEIGHT;
                if let Some(mut path) =
                    self.list(g, &line.lines, (x + width, top), depth + 1, areas)
                {
                    path.insert(0, open);
                    picked = Some(path);
                }
            }
        }
        picked
    }
}

/// The width of the words of each line.
fn measure(g: &mut Canvas<'_>, lines: &[MenuLine]) -> Vec<i32> {
    let look = TextLook::unicode(WORDS_FONT, WORDS_HUE).bordered();
    lines
        .iter()
        .map(|line| g.measure(&line.words, &look).x as i32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_as_wide_as_its_widest_line_and_never_less_than_the_least() {
        assert_eq!(list_width(&[]), LEAST_WIDTH);
        assert_eq!(list_width(&[10, 30]), LEAST_WIDTH);
        assert_eq!(list_width(&[200]), WORDS_X + 200 + WORDS_RIGHT_ROOM);
    }

    #[test]
    fn a_menu_opens_by_the_pointer_and_stays_on_the_screen() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let size = Vec2::new(100.0, 50.0);
        assert_eq!(
            open_place(Pos2::new(100.0, 100.0), size, screen),
            Pos2::new(105.0, 80.0)
        );
        assert_eq!(
            open_place(Pos2::new(790.0, 590.0), size, screen),
            Pos2::new(700.0, 550.0)
        );
        assert_eq!(open_place(Pos2::new(0.0, 5.0), size, screen).y, 0.0);
    }

    #[test]
    fn lines_say_what_they_are() {
        let list = MenuLine::list("Markers", vec![MenuLine::tick("Show", true)]);
        assert_eq!(list.lines.len(), 1);
        assert_eq!(list.lines[0].ticked, Some(true));
        assert!(MenuLine::gap().words.is_empty());
        assert_eq!(MenuLine::plain("Close").ticked, None);
    }
}
