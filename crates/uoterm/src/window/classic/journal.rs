//! The journals of the classic client. The journal of the reference client is a
//! scroll
//! of paper the player makes longer at its foot, with a knob that folds it
//! away, the dark mode box, the four boxes that hide the lines of the
//! shard, of things, of the client and of the guild and alliance, and a
//! flag to read back. With "Use the resizable journal" on the Journal page
//! its resizable journal shows instead: a dark box the player sizes
//! at its corner, with the tabs of the Journal page (a right click on a tab
//! sets its kinds of lines or deletes it, "+" adds one), a search, a save
//! to a text file and a scroll bar. Both show every journal line the window
//! kept, less the ignore list, in the fonts of the Fonts page, with the time
//! unless the Journal page hides it.

use super::canvas::Canvas;
use super::context_menu::{ContextMenu, MenuLine};
use super::message_box::{BoxLook, MessageBox};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::window::model::journal::{self, Entry};
use crate::window::settings::{
    Choice, FontOptions, GameFontKind, JournalKind, JournalTab, Profile,
};
use eframe::egui::{self, Pos2, Vec2};
use uoterm_nav::TextAlign;

pub const JOURNAL: GumpKind = GumpKind {
    id: well_known::JOURNAL,
    rules: GumpRules {
        first_place: Pos2::new(64.0, 64.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(ScrollJournal::default()),
};

pub const RESIZABLE_JOURNAL: GumpKind = GumpKind {
    id: well_known::RESIZABLE_JOURNAL,
    rules: GumpRules {
        right_click_closes: false,
        resizable: Some((RESIZABLE_FIRST_SIZE, RESIZABLE_LEAST_SIZE)),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(ResizableJournal::default()),
};

/// The kind of journal the Journal page asks for.
pub fn journal_kind(profile: &Profile) -> &'static str {
    if profile.journal.alternate_journal {
        well_known::RESIZABLE_JOURNAL
    } else {
        well_known::JOURNAL
    }
}

/// A line with no hue of its own takes the hue of the shard's messages.
const PLAIN_LINE_HUE: u16 = 0x03B2;
/// The Unicode font the journal forces when the Fonts page asks.
const FORCED_UNICODE_FONT: u8 = 0;
const NO_HUE: u16 = 0;
const HALF: i32 = 2;

// The scroll of paper.
const SCROLL: u16 = 0x1F40;
const SCROLL_TITLE: u16 = 0x082A;
/// The scroll starts under the knob.
const SCROLL_Y: i32 = 22;
const FIRST_HEIGHT: i32 = 300;
const KNOB: u16 = 0x082D;
const FOLDED_KNOB: u16 = 0x0830;
const KNOB_X: i32 = 160;
const KNOB_BOX: (i32, i32) = (23, 24);
/// Dark mode gives the paper this hue.
const DARK_HUE: u16 = 903;
const CHECK: (u16, u16) = (0x00D2, 0x00D3);
const DARK_WORDS: &str = "Dark mode";
const DARK_FONT: u8 = 6;
const DARK_WORDS_HUE: u16 = 0x0288;
const DARK_Y: i32 = SCROLL_Y + 7;
const DARK_RIGHT_GAP: i32 = 2;
const FILTER_FONT: u8 = 6;
const FILTER_HUE: u16 = 0x0386;
const FILTER_X: i32 = 43;
const FILTER_STEP: i32 = 75;
/// The boxes sit this far over the foot of the scroll.
const FILTER_UP: i32 = SCROLL_Y - 10;
const LINES_X: i32 = 25;
const LINES_Y: i32 = SCROLL_Y + 36;
/// The lines end this far over the foot of the gump.
const LINES_FOOT_ROOM: i32 = 98 + SCROLL_Y;
const LINES_RIGHT_ROOM: i32 = 5;
const FLAG: u16 = 0x0828;
const FLAG_RIGHT_ROOM: i32 = 5;
/// The words of a line wrap this much narrower than the list and its time.
const WRAP_ROOM: i32 = 18;
const LEAST_WRAP: i32 = 1;
const STAMP_FONT: u8 = 1;
const STAMP_HUE: u16 = 1150;

// The resizable journal.
/// The journal is as wide as the four tabs of the Journal page and a
/// little room, as the reference client makes it.
const RESIZABLE_WIDTH: i32 = BORDER * HALF + TAB_WIDTH * DEFAULT_TABS + TAB_ROW_ROOM;
const DEFAULT_TABS: i32 = 4;
const TAB_ROW_ROOM: i32 = 20;
const RESIZABLE_FIRST_SIZE: Vec2 = Vec2::new(RESIZABLE_WIDTH as f32, 350.0);
const RESIZABLE_LEAST_SIZE: Vec2 = Vec2::new(RESIZABLE_WIDTH as f32, 100.0);
const BORDER: i32 = 4;
const TOP_BORDER: u16 = 0x0A8C;
const SIDE_BORDER: u16 = 0x0A8D;
const BACKGROUND_OPACITY: f32 = 0.7;
/// Dark mode makes the box as dark as its opacity lets it be.
const DARK_OPACITY: f32 = 1.0;
const PERCENT: f32 = 100.0;
const TAB_WIDTH: i32 = 80;
const TAB_HEIGHT: i32 = 30;
const TAB_FONT: u8 = 1;
const WORDS_HUE: u16 = 0xFFFF;
const NEW_TAB: &str = "+";
const NEW_TAB_WIDTH: i32 = 20;
const NEW_TAB_TIP: &str = "Add a new tab";
const DELETE_TAB: &str = "X Delete Tab";
const SCROLL_BAR_WIDTH: i32 = 14;
const TOOL_ROW: i32 = 25;
const TOOL_GAP: i32 = 4;
const FIELD_FRAME: u16 = 0x0BB8;
const SEARCH_WORDS: &str = "Search:";
const SEARCH_WORDS_WIDTH: i32 = 50;
const SAVE_WORDS: &str = "Save";
const SAVE_WIDTH: i32 = 50;
const STAMP_GAP: i32 = 5;
const NOTE_SECONDS: f64 = 6.0;
const NOTE_HUE: u16 = 0x0035;
const ERROR_HUE: u16 = 0x0021;
const WORDS_SAVED: &str = "Saved to";

/// The words of one journal line in the fonts of the Fonts page: Unicode
/// font 0 when the journal is forced to Unicode, else the speech font.
fn line_look(fonts: &FontOptions, hue: u16) -> TextLook {
    let hue = if hue == NO_HUE { PLAIN_LINE_HUE } else { hue };
    if fonts.force_unicode_journal {
        TextLook::unicode(FORCED_UNICODE_FONT, hue).bordered()
    } else if fonts.override_game_font && fonts.game_font_kind == GameFontKind::Ascii {
        TextLook::ascii(fonts.speech_font, hue)
    } else {
        TextLook::unicode(fonts.speech_font, hue).bordered()
    }
}

/// A tab of every kind of line, for the scroll, whose boxes hide lines.
fn every_kind() -> JournalTab {
    JournalTab {
        name: String::new(),
        kinds: (0..JournalKind::LABELS.len())
            .map(JournalKind::from_index)
            .collect(),
    }
}

/// The value of a scroll bar for lines read back: the newest line is the
/// bottom of the bar.
fn bar_value(count: usize, back: usize) -> (i32, i32) {
    let max = count.saturating_sub(1) as i32;
    (max - back.min(count.saturating_sub(1)) as i32, max)
}

/// Draws lines from the bottom of a box up, `back` lines from the newest,
/// as many as fit, each with its time before it when `stamp` has a look.
/// Gives `back`, held so the oldest line stays in reach.
fn draw_lines(
    g: &mut Canvas<'_>,
    shown: &[&Entry],
    (x, y, w, h): (i32, i32, i32, i32),
    back: usize,
    stamp: Option<&TextLook>,
    fonts: &FontOptions,
) -> usize {
    let (range, back) = journal::visible(shown.len(), back);
    g.clipped(x, y, w, h, |g| {
        let mut bottom = y + h;
        for entry in shown[range].iter().rev() {
            let stamp_words = format!("{} ", entry.stamp);
            let stamp_width = stamp.map_or(0, |look| g.measure(&stamp_words, look).x as i32);
            let wrap = (w - WRAP_ROOM - stamp_width).max(LEAST_WRAP) as u32;
            let look = line_look(fonts, entry.hue).wrap(wrap);
            let words = entry.words(false);
            let height = g.measure(&words, &look).y as i32;
            bottom -= height;
            if let Some(stamp_look) = stamp {
                g.label(x, bottom, &stamp_words, stamp_look);
            }
            g.label(x + stamp_width, bottom, &words, &look);
            if bottom <= y {
                break;
            }
        }
    });
    back
}

/// The classic journal: a scroll of paper.
#[derive(Default)]
pub struct ScrollJournal {
    folded: bool,
    /// How many lines the player read back from the newest.
    back: usize,
}

impl ScrollJournal {
    /// The four boxes that hide lines, at the foot of the scroll.
    fn filters(&self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, scroll_height: i32) {
        let look = TextLook::ascii(FILTER_FONT, FILTER_HUE);
        let check = g.gump_size(CHECK.0).unwrap_or(Vec2::ZERO);
        let y = scroll_height - check.y as i32 - FILTER_UP;
        let options = &mut cx.profile.journal;
        let boxes: [(&str, &mut bool); 4] = [
            ("System", &mut options.show_system_lines),
            ("Objects", &mut options.show_object_lines),
            ("Client", &mut options.show_client_lines),
            ("Guild", &mut options.show_guild_and_alliance),
        ];
        let mut changed = false;
        for (at, (words, ticked)) in boxes.into_iter().enumerate() {
            let x = FILTER_X + FILTER_STEP * at as i32;
            changed |= g.checkbox(("filter", at), x, y, CHECK, ticked, Some((words, &look)));
        }
        if changed {
            cx.profile_changed();
        }
    }
}

impl GumpBody for ScrollJournal {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if cx.profile.journal.alternate_journal {
            cx.close(cx.me);
            cx.open(GumpId::one(well_known::RESIZABLE_JOURNAL));
            return;
        }
        if self.folded {
            g.pic(0, 0, FOLDED_KNOB, NO_HUE);
            if g.body_double_click() {
                self.folded = false;
            }
            return;
        }
        g.pic(KNOB_X, 0, KNOB, NO_HUE);
        if g.click_area("fold", KNOB_X, 0, KNOB_BOX.0, KNOB_BOX.1)
            .clicked()
        {
            self.folded = true;
        }
        let hue = if cx.profile.journal.dark_mode {
            DARK_HUE
        } else {
            NO_HUE
        };
        let default_height = FIRST_HEIGHT - SCROLL_Y;
        let scroll = g.hued_expandable_scroll(0, SCROLL_Y, SCROLL, default_height, hue);
        let (width, scroll_height) = (scroll.x as i32, scroll.y as i32);
        let title = g.gump_size(SCROLL_TITLE).unwrap_or(Vec2::ZERO);
        let top = g.gump_size(SCROLL).unwrap_or(Vec2::ZERO);
        g.pic(
            (top.x - title.x) as i32 / HALF,
            SCROLL_Y + (top.y - title.y) as i32 / HALF,
            SCROLL_TITLE,
            NO_HUE,
        );
        let dark_look = TextLook::ascii(DARK_FONT, DARK_WORDS_HUE);
        // The box and its words stay on the paper.
        let dark_width = g.measure(DARK_WORDS, &dark_look).x as i32
            + g.gump_size(CHECK.0).map_or(0, |size| size.x as i32);
        let mut dark = cx.profile.journal.dark_mode;
        if g.checkbox(
            "dark",
            width - dark_width - DARK_RIGHT_GAP,
            DARK_Y,
            CHECK,
            &mut dark,
            Some((DARK_WORDS, &dark_look)),
        ) {
            cx.profile.journal.dark_mode = dark;
            cx.profile_changed();
        }
        let flag = g.gump_size(FLAG).unwrap_or(Vec2::ZERO);
        let lines_width = width - flag.x as i32 / HALF - LINES_RIGHT_ROOM - LINES_X;
        let lines_height = scroll_height + SCROLL_Y - LINES_FOOT_ROOM;
        let area = (LINES_X, LINES_Y, lines_width, lines_height);
        let hide_timestamps = cx.profile.journal.hide_timestamps;
        let log = cx.journal;
        let shown = log.shown(&every_kind(), &cx.profile.journal, &cx.profile.ignore, "");
        self.back = journal::scrolled_back(self.back, g.wheel_turns(0, 0, width, scroll_height));
        let (mut value, max) = bar_value(shown.len(), self.back);
        if max > 0 {
            let flag_x = LINES_X + lines_width - flag.x as i32 / HALF + FLAG_RIGHT_ROOM;
            g.scroll_flag("flag", flag_x, LINES_Y, lines_height, &mut value, max);
            self.back = (max - value) as usize;
        }
        let stamp_look = TextLook::unicode(STAMP_FONT, STAMP_HUE).bordered();
        let stamp = (!hide_timestamps).then_some(&stamp_look);
        self.back = draw_lines(g, &shown, area, self.back, stamp, &cx.profile.fonts);
        self.filters(g, cx, scroll_height);
    }
}

/// A note under the lines for a while, such as where the journal was saved.
struct Note {
    words: String,
    failed: bool,
    since: f64,
}

/// The reference client's resizable journal, with the tabs of the Journal page.
#[derive(Default)]
pub struct ResizableJournal {
    tab: usize,
    back: usize,
    search: TextField,
    /// The name of a new tab while the player types it.
    new_tab: Option<TextField>,
    /// The tab whose menu is open.
    menu_tab: usize,
    menu: ContextMenu,
    note: Option<Note>,
}

/// The menu of a tab: each kind of line, ticked when the tab shows it, and
/// the line that deletes the tab.
fn tab_menu(tab: &JournalTab) -> Vec<MenuLine> {
    let mut lines: Vec<MenuLine> = JournalKind::LABELS
        .iter()
        .enumerate()
        .map(|(at, words)| MenuLine::tick(*words, tab.kinds.contains(&JournalKind::from_index(at))))
        .collect();
    lines.push(MenuLine::plain(DELETE_TAB));
    lines
}

impl ResizableJournal {
    /// The tabs, the new tab button, and the tab under a right click.
    fn tabs(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) -> Option<usize> {
        let look = TextLook::unicode(TAB_FONT, WORDS_HUE)
            .bordered()
            .aligned(TextAlign::Center);
        let count = cx.profile.journal.tabs.len();
        self.tab = self.tab.min(count.saturating_sub(1));
        let mut right_clicked = None;
        for at in 0..count {
            let x = at as i32 * TAB_WIDTH + BORDER;
            let name = cx.profile.journal.tabs[at].name.clone();
            let selected = at == self.tab;
            if g.nice_button(
                ("tab", at),
                x,
                0,
                TAB_WIDTH,
                TAB_HEIGHT,
                &name,
                &look,
                selected,
            ) {
                self.tab = at;
                self.back = 0;
            }
            if g.right_click() && g.hovered(x, 0, TAB_WIDTH, TAB_HEIGHT) {
                right_clicked = Some(at);
            }
        }
        let plus_x = count as i32 * TAB_WIDTH + BORDER;
        let mut named = None;
        match &mut self.new_tab {
            Some(field) => {
                g.frame(plus_x, 0, TAB_WIDTH, TAB_HEIGHT, FIELD_FRAME);
                let typed = g.text_box("new-tab", plus_x, 0, TAB_WIDTH, TAB_HEIGHT, field, &look);
                if typed.submitted {
                    named = Some(field.text().trim().to_string());
                }
            }
            None => {
                let plus = g.nice_button(
                    "new-tab-button",
                    plus_x,
                    0,
                    NEW_TAB_WIDTH,
                    TAB_HEIGHT,
                    NEW_TAB,
                    &look,
                    false,
                );
                g.tooltip(NEW_TAB_TIP);
                if plus {
                    self.new_tab = Some(TextField::new(""));
                    g.focus("new-tab");
                }
            }
        }
        if let Some(name) = named {
            self.new_tab = None;
            if let Some(tab) = journal::new_tab(&name) {
                cx.profile.journal.tabs.push(tab);
                cx.profile_changed();
            }
        }
        right_clicked
    }

    /// The search field and the save button. True when save was pressed.
    fn tool_row(&mut self, g: &mut Canvas<'_>, width: i32) -> bool {
        let look = TextLook::unicode(TAB_FONT, WORDS_HUE).bordered();
        let y = TAB_HEIGHT;
        g.label(BORDER, y, SEARCH_WORDS, &look);
        let field_x = BORDER + SEARCH_WORDS_WIDTH;
        let field_width = width - field_x - SAVE_WIDTH - BORDER - TOOL_GAP;
        g.frame(field_x, y, field_width, TOOL_ROW, FIELD_FRAME);
        if g.text_box(
            "search",
            field_x,
            y,
            field_width,
            TOOL_ROW,
            &mut self.search,
            &look,
        )
        .changed
        {
            self.back = 0;
        }
        g.nice_button(
            "save",
            width - SAVE_WIDTH - BORDER,
            y,
            SAVE_WIDTH,
            TOOL_ROW,
            SAVE_WORDS,
            &look.aligned(TextAlign::Center),
            false,
        )
    }

    fn save(&mut self, shown: &[&Entry], cx: &GumpContext<'_>, time: f64) {
        let with_stamp = !cx.profile.journal.hide_timestamps;
        self.note = Some(
            match journal::save(&journal::journals_dir(), &cx.frame.name, shown, with_stamp) {
                Ok(file) => Note {
                    words: format!("{WORDS_SAVED} {}", file.display()),
                    failed: false,
                    since: time,
                },
                Err(error) => Note {
                    words: error.to_string(),
                    failed: true,
                    since: time,
                },
            },
        );
    }

    /// The menu of the tab the player right-clicked.
    fn show_tab_menu(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(tab) = cx.profile.journal.tabs.get(self.menu_tab) else {
            return;
        };
        let name = tab.name.clone();
        let lines = tab_menu(tab);
        let scale = cx.profile.video.ui_scale;
        let Some(path) = self.menu.show(g, (cx.me, "tab-menu"), scale, &lines) else {
            return;
        };
        let Some(&picked) = path.first() else {
            return;
        };
        if picked < JournalKind::LABELS.len() {
            if let Some(tab) = cx.profile.journal.tabs.get_mut(self.menu_tab) {
                journal::flip_kind(tab, JournalKind::from_index(picked));
                cx.profile_changed();
            }
            return;
        }
        let question = journal::delete_question(&name);
        cx.open_with(
            GumpId::one(well_known::QUESTION),
            Box::new(MessageBox::new(
                question,
                BoxLook::Question,
                Box::new(
                    move |yes: bool, cx: &mut GumpContext<'_>, _: &egui::Context| {
                        if yes {
                            journal::delete_tab(&mut cx.profile.journal.tabs, &name);
                            cx.profile_changed();
                        }
                    },
                ),
            )),
        );
    }
}

impl GumpBody for ResizableJournal {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if !cx.profile.journal.alternate_journal {
            cx.close(cx.me);
            cx.open(GumpId::one(well_known::JOURNAL));
            return;
        }
        let size = g.size().unwrap_or(RESIZABLE_FIRST_SIZE);
        let (width, height) = (size.x as i32, size.y as i32);
        g.pic_tiled(0, 0, width, BORDER, TOP_BORDER, NO_HUE);
        g.pic_tiled(0, height - BORDER, width, BORDER, TOP_BORDER, NO_HUE);
        g.pic_tiled(0, 0, BORDER, height, SIDE_BORDER, NO_HUE);
        g.pic_tiled(width - BORDER, 0, BORDER, height, SIDE_BORDER, NO_HUE);
        let options = &cx.profile.journal;
        let darkness = if options.dark_mode {
            DARK_OPACITY
        } else {
            BACKGROUND_OPACITY
        };
        let opacity = darkness * f32::from(options.opacity) / PERCENT;
        g.shade(
            BORDER,
            BORDER,
            width - BORDER * HALF,
            height - BORDER * HALF,
            NO_HUE,
            opacity,
        );
        let right_clicked = self.tabs(g, cx);
        let save = self.tool_row(g, width);
        let lines_y = TAB_HEIGHT + TOOL_ROW + TOOL_GAP;
        let lines_height = height - lines_y - BORDER;
        let lines_width = width - SCROLL_BAR_WIDTH - BORDER * HALF;
        let tab = cx.profile.journal.tabs.get(self.tab).cloned();
        let log = cx.journal;
        let shown: Vec<&Entry> = tab.as_ref().map_or_else(Vec::new, |tab| {
            log.shown(
                tab,
                &cx.profile.journal,
                &cx.profile.ignore,
                self.search.text(),
            )
        });
        let time = g.ctx().input(|i| i.time);
        if save {
            self.save(&shown, cx, time);
        }
        self.back = journal::scrolled_back(
            self.back,
            g.wheel_turns(BORDER, lines_y, width - BORDER * HALF, lines_height),
        );
        let (mut value, max) = bar_value(shown.len(), self.back);
        if max > 0 {
            let bar_x = width - SCROLL_BAR_WIDTH - BORDER;
            g.scroll_bar("bar", bar_x, lines_y, lines_height, &mut value, max);
            self.back = (max - value) as usize;
        }
        let stamp_look = TextLook::unicode(STAMP_FONT, WORDS_HUE).bordered();
        let stamp = (!cx.profile.journal.hide_timestamps).then_some(&stamp_look);
        let area = (BORDER, lines_y, lines_width - STAMP_GAP, lines_height);
        self.back = draw_lines(g, &shown, area, self.back, stamp, &cx.profile.fonts);
        if let Some(note) = &self.note {
            if time - note.since > NOTE_SECONDS {
                self.note = None;
            } else {
                let hue = if note.failed { ERROR_HUE } else { NOTE_HUE };
                let look = TextLook::unicode(TAB_FONT, hue)
                    .bordered()
                    .wrap((width - BORDER * HALF) as u32);
                g.label(BORDER, lines_y, &note.words, &look);
            }
        }
        match right_clicked {
            Some(tab) => {
                self.menu_tab = tab;
                if let Some(at) = g.ctx().pointer_latest_pos() {
                    self.menu.open_at(at);
                }
            }
            None if g.right_click() => cx.close(cx.me),
            None => {}
        }
        if self.menu.is_open() {
            self.show_tab_menu(g, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::testing::draw_frames;

    #[test]
    fn the_journal_page_picks_the_journal() {
        let mut profile = Profile::default();
        assert_eq!(journal_kind(&profile), well_known::JOURNAL);
        profile.journal.alternate_journal = true;
        assert_eq!(journal_kind(&profile), well_known::RESIZABLE_JOURNAL);
    }

    #[test]
    fn the_bar_holds_the_newest_at_its_foot() {
        assert_eq!(bar_value(10, 0), (9, 9));
        assert_eq!(bar_value(10, 4), (5, 9));
        assert_eq!(bar_value(10, 40), (0, 9));
        assert_eq!(bar_value(0, 3), (0, 0));
    }

    #[test]
    fn the_fonts_page_picks_the_font_of_the_lines() {
        let mut fonts = FontOptions::default();
        let look = line_look(&fonts, 0x0035);
        assert_eq!(
            look.font,
            super::super::text::UoFont::Unicode(fonts.speech_font)
        );
        assert_eq!(line_look(&fonts, NO_HUE).hue, PLAIN_LINE_HUE);
        fonts.override_game_font = true;
        fonts.game_font_kind = GameFontKind::Ascii;
        assert_eq!(
            line_look(&fonts, 1).font,
            super::super::text::UoFont::Ascii(fonts.speech_font)
        );
        fonts.force_unicode_journal = true;
        assert_eq!(
            line_look(&fonts, 1).font,
            super::super::text::UoFont::Unicode(FORCED_UNICODE_FONT)
        );
    }

    #[test]
    fn a_tab_menu_ticks_the_kinds_of_the_tab_and_flips_them() {
        let mut tab = JournalTab {
            name: "Chat".into(),
            kinds: vec![JournalKind::Speech],
        };
        let lines = tab_menu(&tab);
        assert_eq!(lines.len(), JournalKind::LABELS.len() + 1);
        assert_eq!(lines[JournalKind::Speech.index()].ticked, Some(true));
        assert_eq!(lines[JournalKind::Yell.index()].ticked, Some(false));
        assert_eq!(lines.last().unwrap().words, DELETE_TAB);
        journal::flip_kind(&mut tab, JournalKind::Yell);
        assert_eq!(tab_menu(&tab)[JournalKind::Yell.index()].ticked, Some(true));
        assert_eq!(every_kind().kinds.len(), JournalKind::LABELS.len());
    }

    #[test]
    fn both_journals_draw_with_the_client_files() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        manager.open(GumpId::one(well_known::JOURNAL), &mut profile);
        if !draw_frames(&mut manager, &mut profile, &WatchFrame::default()) {
            return;
        }
        assert!(manager.is_open(&GumpId::one(well_known::JOURNAL)));
        profile.journal.alternate_journal = true;
        draw_frames(&mut manager, &mut profile, &WatchFrame::default());
        assert!(manager.is_open(&GumpId::one(well_known::RESIZABLE_JOURNAL)));
        assert!(!manager.is_open(&GumpId::one(well_known::JOURNAL)));
    }
}
