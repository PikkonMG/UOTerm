//! The journal of the Modern style: the tabs of the Journal page, the
//! lines of each kept past what the session sends, the wheel to read back,
//! a search, a save to a text file, dark mode and its own opacity. The
//! player adds a tab with "+", and a right click on a tab renames it, sets
//! the kinds of lines it shows or deletes it; the filters under the search
//! hide the lines of the shard, of things, of the client and of the guild,
//! as the classic journal's boxes do. The journal folds to its title and
//! shuts; the chat box sits in its last row, and stays in a strip of its
//! own while the journal is shut.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::journal::{self, Entry, JournalLog, Origin};
use super::super::model::places;
use super::super::settings::{Choice, JournalKind, JournalOptions, Profile};
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{
    self, text::LayoutJob, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense,
    TextFormat, Vec2,
};

pub const JOURNAL_ID: &str = "modern:journal";
/// The journal stands at the bottom right, as wide as this.
pub(super) const JOURNAL_WIDTH: f32 = 400.0;
pub(super) const JOURNAL_HEIGHT: f32 = 330.0;
pub(super) const JOURNAL_LEAST: Vec2 = Vec2::new(260.0, 200.0);
const TAB_ROW: f32 = 24.0;
const TAB_GAP: f32 = 4.0;
const NEW_TAB_WIDTH: f32 = 24.0;
const NEW_TAB_FIELD_WIDTH: f32 = 90.0;
const TOOL_ROW: f32 = 24.0;
const FILTER_ROW: f32 = 20.0;
const SAVE_WIDTH: f32 = 54.0;
const LINE_GAP: f32 = 4.0;
const CHAT_ROW_HEIGHT: f32 = 30.0;
const NOTE_SECONDS: f64 = 6.0;
const PERCENT: f32 = 100.0;
/// The wheel gives its step in points. This many points are one notch.
const WHEEL_NOTCH: f32 = 50.0;

const WORDS_TITLE: &str = "Journal";
const WORDS_SAVE: &str = "Save";
const WORDS_NO_LINES: &str = "No lines yet.";
const WORDS_SAVED: &str = "Saved to";
const WORDS_BACK: &str = "lines back";
const WORDS_NEW_TAB: &str = "+";
const WORDS_RENAME: &str = "Rename";
const WORDS_DELETE_TAB: &str = "Delete tab";
const HINT_SEARCH: &str = "search the journal";
const HINT_NEW_TAB: &str = "Add a tab.";
const HINT_TAB: &str = "Right-click: rename, kinds of lines, delete.";
const HINT_TAB_NAME: &str = "tab name";

/// The option of the Journal page one filter flips.
type FilterOption = fn(&mut JournalOptions) -> &mut bool;

/// The filters under the search: their words, and the option each flips.
const FILTERS: [(&str, FilterOption); 4] = [
    ("Shard", |options| &mut options.show_system_lines),
    ("Things", |options| &mut options.show_object_lines),
    ("Client", |options| &mut options.show_client_lines),
    ("Guild", |options| &mut options.show_guild_and_alliance),
];

/// What the journal tells the Modern panels after it is drawn.
pub struct JournalDrawn {
    pub panel: Rect,
    /// The row of the chat box.
    pub chat_row: Rect,
    /// The tab the player asked to delete, which the question confirms.
    pub delete_tab: Option<String>,
}

#[derive(Default)]
pub struct JournalUi {
    log: JournalLog,
    tab: usize,
    /// How many lines the player read back from the newest.
    back: usize,
    search: String,
    note: Option<(String, bool, f64)>,
    /// The name of a tab the player adds, while he types it.
    new_tab: Option<String>,
    /// The name the player types for a tab in its menu.
    renaming: String,
}

fn waiting_words(persons: usize) -> String {
    if persons == 1 {
        "1 person waits for an answer".to_string()
    } else {
        format!("{persons} persons wait for an answer")
    }
}

/// The whole turns of the wheel: up reads back.
fn wheel_turns(notches: f32) -> i32 {
    (notches.signum() * notches.abs().ceil()) as i32
}

impl JournalUi {
    /// Keeps the new lines of the frame. Call it once in each frame.
    pub fn take(&mut self, frame: &WatchFrame, profile: &Profile) {
        let max = usize::from(profile.journal.max_lines);
        self.log.take(frame, &journal::stamp_now(), max);
    }

    /// Draws the journal, or the chat strip while it is shut.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> JournalDrawn {
        let default = layout::first_place(
            rect,
            Spot::Journal,
            Vec2::new(JOURNAL_WIDTH, JOURNAL_HEIGHT),
        );
        let spec = PanelSpec {
            id: JOURNAL_ID,
            title: WORDS_TITLE,
            default,
            min_size: Some(JOURNAL_LEAST),
            closable: true,
        };
        let whole = frame::place(rect, &spec, profile);
        if places::is_shut(profile, JOURNAL_ID) {
            return Self::chat_strip(ui, whole);
        }
        let folded = places::is_folded(profile, JOURNAL_ID);
        let panel = if folded {
            Rect::from_min_size(
                whole.min,
                Vec2::new(whole.width(), frame::FOLDED_HEIGHT + CHAT_ROW_HEIGHT),
            )
        } else {
            whole
        };
        let options = &profile.journal;
        let glass = if options.dark_mode {
            theme::DARK_GLASS
        } else {
            theme::GLASS
        };
        let fill = theme::with_alpha(glass, f32::from(options.opacity) / PERCENT);
        let body = frame::draw_tinted(ui.painter(), panel, WORDS_TITLE, fill, theme::GLASS_EDGE);
        if frame.unanswered > 0 {
            ui.painter().text(
                Pos2::new(
                    frame::mark_area(panel, frame::marks(&spec)).left(),
                    panel.top() + theme::PANEL_PAD,
                ),
                Align2::RIGHT_TOP,
                waiting_words(frame.unanswered),
                text_font(theme::SIZE_SMALL),
                theme::WAITING,
            );
        }
        let chat_row = Rect::from_min_max(
            Pos2::new(body.left(), body.bottom() - CHAT_ROW_HEIGHT + LINE_GAP),
            body.right_bottom(),
        );
        let mut delete_tab = None;
        if !folded {
            delete_tab = self.contents(ui, body, chat_row, frame, tools, profile);
        }
        if frame::foldable_controls(ui, whole, &spec, profile, tools) == Some(FrameEvent::Closed) {
            places::set_shut(profile, JOURNAL_ID, true);
            tools.keep_profile(profile);
        }
        JournalDrawn {
            panel,
            chat_row,
            delete_tab,
        }
    }

    /// The glass of the chat box alone, at the foot of where the journal
    /// stands.
    fn chat_strip(ui: &egui::Ui, whole: Rect) -> JournalDrawn {
        let height = CHAT_ROW_HEIGHT + theme::PANEL_PAD * 2.0;
        let panel = Rect::from_min_max(
            Pos2::new(whole.left(), whole.bottom() - height),
            whole.right_bottom(),
        );
        theme::panel(ui.painter(), panel);
        JournalDrawn {
            panel,
            chat_row: panel.shrink(theme::PANEL_PAD),
            delete_tab: None,
        }
    }

    /// The tabs, the search, the filters and the lines. Gives the tab the
    /// player asked to delete.
    fn contents(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        chat_row: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<String> {
        let tabs = Rect::from_min_size(body.min, Vec2::new(body.width(), TAB_ROW));
        let delete_tab = self.tabs(ui, tabs, tools, profile);
        let tool_row = Rect::from_min_size(
            Pos2::new(body.left(), tabs.bottom() + TAB_GAP),
            Vec2::new(body.width(), TOOL_ROW),
        );
        let filter_row = Rect::from_min_size(
            Pos2::new(body.left(), tool_row.bottom() + TAB_GAP),
            Vec2::new(body.width(), FILTER_ROW),
        );
        let lines_area = Rect::from_min_max(
            Pos2::new(body.left(), filter_row.bottom() + TAB_GAP),
            Pos2::new(body.right(), chat_row.top() - LINE_GAP),
        );
        let save_pressed = self.tool_row(ui, tool_row, tools.time);
        filters(ui, filter_row, tools, profile);
        let tab = profile.journal.tabs.get(self.tab).cloned();
        let shown: Vec<&Entry> = tab.as_ref().map_or_else(Vec::new, |tab| {
            self.log
                .shown(tab, &profile.journal, &profile.ignore, &self.search)
        });
        let with_stamp = !profile.journal.hide_timestamps;
        if save_pressed {
            self.note = Some(
                match journal::save(&journal::journals_dir(), &frame.name, &shown, with_stamp) {
                    Ok(file) => (
                        format!("{WORDS_SAVED} {}", file.display()),
                        false,
                        tools.time,
                    ),
                    Err(error) => (error.to_string(), true, tools.time),
                },
            );
        }
        self.back = lines(ui, lines_area, &shown, self.back, with_stamp, tools);
        delete_tab
    }

    /// The tabs, each with its menu, and "+" that adds one. Gives the tab
    /// the player asked to delete.
    fn tabs(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        tools: &Tools<'_>,
        profile: &mut Profile,
    ) -> Option<String> {
        let count = profile.journal.tabs.len();
        self.tab = self.tab.min(count.saturating_sub(1));
        let adder = if self.new_tab.is_some() {
            NEW_TAB_FIELD_WIDTH
        } else {
            NEW_TAB_WIDTH
        };
        let tabs_width = row.width() - adder - TAB_GAP;
        let width = if count == 0 {
            0.0
        } else {
            (tabs_width - TAB_GAP * (count - 1) as f32) / count as f32
        };
        let mut delete_tab = None;
        let mut changed = false;
        for at in 0..count {
            let area = Rect::from_min_size(
                row.left_top() + Vec2::new(at as f32 * (width + TAB_GAP), 0.0),
                Vec2::new(width, row.height()),
            );
            let color = if at == self.tab {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            let name = profile.journal.tabs[at].name.clone();
            let key = Id::new(("journal-tab", at));
            if theme::segment_keyed(ui, area, key, &name, color) {
                self.tab = at;
                self.back = 0;
            }
            let response = ui.interact(area, key, Sense::click());
            if response.hovered() {
                super::super::tips::label(ui, &name, HINT_TAB);
            }
            response.context_menu(|ui| {
                let tab = &mut profile.journal.tabs[at];
                ui.horizontal(|ui| {
                    let typed = ui.add(
                        egui::TextEdit::singleline(&mut self.renaming).hint_text(HINT_TAB_NAME),
                    );
                    let entered =
                        typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let named = self.renaming.trim().to_string();
                    if (ui.button(WORDS_RENAME).clicked() || entered) && !named.is_empty() {
                        tab.name = named;
                        self.renaming.clear();
                        changed = true;
                        ui.close_menu();
                    }
                });
                ui.separator();
                for (index, words) in JournalKind::LABELS.iter().enumerate() {
                    let kind = JournalKind::from_index(index);
                    let mut shown = tab.kinds.contains(&kind);
                    if ui.checkbox(&mut shown, *words).changed() {
                        journal::flip_kind(tab, kind);
                        changed = true;
                    }
                }
                ui.separator();
                if ui.button(WORDS_DELETE_TAB).clicked() {
                    delete_tab = Some(tab.name.clone());
                    ui.close_menu();
                }
            });
        }
        let adder_area = Rect::from_min_size(
            Pos2::new(row.right() - adder, row.top()),
            Vec2::new(adder, row.height()),
        );
        changed |= self.adder(ui, adder_area, profile);
        if changed {
            tools.keep_profile(profile);
        }
        delete_tab
    }

    /// "+", and the field of the new tab's name after a click on it. True
    /// when a tab was added.
    fn adder(&mut self, ui: &mut egui::Ui, area: Rect, profile: &mut Profile) -> bool {
        let Some(name) = self.new_tab.as_mut() else {
            if theme::segment_keyed(
                ui,
                area,
                Id::new("journal-new-tab"),
                WORDS_NEW_TAB,
                theme::GOAL,
            ) {
                self.new_tab = Some(String::new());
            }
            if ui.rect_contains_pointer(area) {
                super::super::tips::label(ui, HINT_NEW_TAB, "");
            }
            return false;
        };
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            area,
            egui::TextEdit::singleline(name)
                .id(Id::new("journal-new-tab-name"))
                .frame(false)
                .hint_text(HINT_TAB_NAME)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        if !typed.has_focus() && !typed.lost_focus() {
            typed.request_focus();
        }
        if !typed.lost_focus() {
            return false;
        }
        let entered = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let added = entered.then(|| journal::new_tab(name)).flatten();
        self.new_tab = None;
        match added {
            Some(tab) => {
                profile.journal.tabs.push(tab);
                self.tab = profile.journal.tabs.len() - 1;
                true
            }
            None => false,
        }
    }

    /// The search field and the save button. True when save was pressed.
    fn tool_row(&mut self, ui: &mut egui::Ui, row: Rect, time: f64) -> bool {
        let save = Rect::from_min_max(Pos2::new(row.right() - SAVE_WIDTH, row.top()), row.max);
        let field = Rect::from_min_max(row.min, Pos2::new(save.left() - TAB_GAP, row.bottom()));
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.search)
                .id(Id::new("journal-search"))
                .frame(false)
                .hint_text(HINT_SEARCH)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        if typed.changed() {
            self.back = 0;
        }
        if let Some((words, failed, since)) = &self.note {
            if time - since > NOTE_SECONDS {
                self.note = None;
            } else {
                ui.painter().text(
                    Pos2::new(row.left(), row.top() - LINE_GAP),
                    Align2::LEFT_BOTTOM,
                    words,
                    text_font(theme::SIZE_SMALL),
                    if *failed {
                        theme::ALARM
                    } else {
                        theme::WAITING
                    },
                );
            }
        }
        theme::segment_keyed(ui, save, Id::new("journal-save"), WORDS_SAVE, theme::TEXT)
    }
}

/// The filters: each shows or hides one kind of lines.
fn filters(ui: &egui::Ui, row: Rect, tools: &Tools<'_>, profile: &mut Profile) {
    let width = (row.width() - TAB_GAP * (FILTERS.len() - 1) as f32) / FILTERS.len() as f32;
    for (at, (words, option)) in FILTERS.into_iter().enumerate() {
        let area = Rect::from_min_size(
            row.left_top() + Vec2::new(at as f32 * (width + TAB_GAP), 0.0),
            Vec2::new(width, row.height()),
        );
        let shown = option(&mut profile.journal);
        let color = if *shown {
            theme::GOAL
        } else {
            theme::TEXT_FAINT
        };
        if theme::segment_keyed(ui, area, Id::new(("journal-filter", at)), words, color) {
            *shown = !*shown;
            tools.keep_profile(profile);
        }
    }
}

/// The color of a line's words: the hue the shard gave it, or the plain
/// colors of the window.
fn words_color(entry: &Entry, tools: &Tools<'_>) -> Color32 {
    match entry.origin {
        Origin::System | Origin::Client => theme::TEXT_DIM,
        Origin::Mobile | Origin::Object if entry.hue == 0 => theme::TEXT,
        Origin::Mobile | Origin::Object => tools.scene.words_color(entry.hue),
    }
}

fn line_job(entry: &Entry, width: f32, with_stamp: bool, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let font: FontId = text_font(theme::SIZE_BODY);
    let format = |color: Color32| TextFormat::simple(font.clone(), color);
    if with_stamp {
        job.append(&format!("{} ", entry.stamp), 0.0, format(theme::TEXT_FAINT));
    }
    if !entry.name.is_empty() {
        job.append(&format!("{}: ", entry.name), 0.0, format(theme::GOAL));
    }
    job.append(&entry.text, 0.0, format(color));
    job
}

/// Where a line whose foot is at `bottom` stands: its top, when the line
/// fits whole under `room_top`. The newest line (`at` 0) always stands,
/// cut when it is taller than the room.
fn whole_line_top(at: usize, bottom: f32, height: f32, room_top: f32) -> Option<f32> {
    let top = bottom - height;
    (at == 0 || top >= room_top).then_some(top)
}

/// Draws the lines from the bottom up, `back` lines back from the newest,
/// as many as fit whole. The wheel reads further back. Gives the new
/// `back`.
fn lines(
    ui: &egui::Ui,
    area: Rect,
    shown: &[&Entry],
    back: usize,
    with_stamp: bool,
    tools: &Tools<'_>,
) -> usize {
    let painter = ui.painter().with_clip_rect(area);
    if shown.is_empty() {
        painter.text(
            area.left_top(),
            Align2::LEFT_TOP,
            WORDS_NO_LINES,
            text_font(theme::SIZE_BODY),
            theme::TEXT_FAINT,
        );
        return 0;
    }
    let notches = ui.input(|i| {
        let over = i.pointer.hover_pos().is_some_and(|p| area.contains(p));
        if over {
            i.raw_scroll_delta.y / WHEEL_NOTCH
        } else {
            0.0
        }
    });
    let back = journal::scrolled_back(back, wheel_turns(notches));
    let (range, back) = journal::visible(shown.len(), back);
    let mut bottom = area.bottom();
    for (at, entry) in shown[range].iter().rev().enumerate() {
        let job = line_job(entry, area.width(), with_stamp, words_color(entry, tools));
        let galley = painter.layout_job(job);
        let Some(top) = whole_line_top(at, bottom, galley.size().y, area.top()) else {
            break;
        };
        painter.galley(Pos2::new(area.left(), top), galley, theme::TEXT);
        bottom = top - LINE_GAP;
    }
    if back > 0 {
        theme::shadowed_text(
            ui.painter(),
            area.right_top(),
            Align2::RIGHT_TOP,
            &format!("{back} {WORDS_BACK}"),
            text_font(theme::SIZE_SMALL),
            theme::WAITING,
        );
    }
    back
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_person_waits_and_two_persons_wait() {
        assert_eq!(waiting_words(1), "1 person waits for an answer");
        assert_eq!(waiting_words(2), "2 persons wait for an answer");
    }

    #[test]
    fn only_whole_lines_stand_under_the_filters() {
        const ROOM_TOP: f32 = 100.0;
        const LINE: f32 = 18.0;
        assert_eq!(whole_line_top(1, 130.0, LINE, ROOM_TOP), Some(112.0));
        assert_eq!(whole_line_top(1, 118.0, LINE, ROOM_TOP), Some(ROOM_TOP));
        assert_eq!(
            whole_line_top(1, 117.0, LINE, ROOM_TOP),
            None,
            "a line cut by the filters is not drawn"
        );
        assert_eq!(
            whole_line_top(0, 110.0, LINE, ROOM_TOP),
            Some(92.0),
            "the newest line shows even in a room too low for it"
        );
    }

    #[test]
    fn a_part_of_a_notch_is_a_whole_turn_each_way() {
        assert_eq!(wheel_turns(0.2), 1);
        assert_eq!(wheel_turns(-1.5), -2);
        assert_eq!(wheel_turns(0.0), 0);
    }

    #[test]
    fn each_filter_flips_its_own_option() {
        let mut options = JournalOptions::default();
        for (_, option) in FILTERS {
            let before = *option(&mut options);
            *option(&mut options) = !before;
            assert_ne!(*option(&mut options), before);
        }
        assert_ne!(options, JournalOptions::default());
    }

    #[test]
    fn a_shut_journal_keeps_the_chat_box_and_a_folded_one_its_title() {
        use super::super::testing::draw_frames;
        let mut journal = JournalUi::default();
        let mut profile = Profile::default();
        let frame = WatchFrame::default();
        let mut drawn = Vec::new();
        for (shut, folded) in [(false, false), (false, true), (true, false)] {
            places::set_shut(&mut profile, JOURNAL_ID, shut);
            places::set_folded(&mut profile, JOURNAL_ID, folded);
            draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
                drawn.push(journal.draw(ui, rect, &frame, tools, profile));
            });
        }
        let [open, folded, shut] = [&drawn[0], &drawn[1], &drawn[2]];
        assert!(folded.panel.height() < open.panel.height());
        assert!(shut.panel.height() < folded.panel.height());
        for one in [open, folded, shut] {
            assert!(one.panel.contains_rect(one.chat_row), "the chat box stays");
        }
    }
}
