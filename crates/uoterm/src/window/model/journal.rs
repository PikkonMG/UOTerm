//! The journal of both styles: every line the window saw, kept past the few
//! the session sends, sorted into kinds for the tabs of the Journal page,
//! with the search, the scroll back and the save to a file, and the tabs
//! the player adds, sets and deletes in the journal itself; and the journal
//! file the Speech page keeps, written line by line as the lines come.

use crate::view::{WatchFrame, WatchSpeech};
use crate::window::settings::{
    IgnoreOptions, JournalKind, JournalOptions, JournalTab, SpeechOptions,
};
use std::collections::VecDeque;
use std::io::Write;
use std::ops::Range;
use std::path::{Path, PathBuf};
use uoterm_protocol::types::{
    SPEECH_ALLIANCE, SPEECH_EMOTE, SPEECH_GUILD, SPEECH_LABEL, SPEECH_REGULAR, SPEECH_SYSTEM,
    SPEECH_WHISPER, SPEECH_YELL,
};
use uoterm_world::{SPEECH_KIND_PARTY, SPEECH_KIND_PARTY_PRIVATE};

/// The message type of the words of a spell.
pub const SPEECH_SPELL: u8 = 10;
/// Serials from this one up are items, below it mobiles.
const FIRST_ITEM_SERIAL: u32 = 0x4000_0000;
const JOURNALS_DIR: &str = "journals";
const JOURNAL_FILE_STAMP: &str = "%Y%m%d-%H%M%S";
const JOURNAL_FILE_EXTENSION: &str = "txt";
/// The journal files the Speech page keeps lie here in the journals folder,
/// each named for the time it began, as the reference client names them.
const JOURNAL_LOGS_DIR: &str = "logs";
const JOURNAL_LOG_STAMP: &str = "%Y_%m_%d_%H_%M_%S";
const JOURNAL_LOG_SUFFIX: &str = "_journal.txt";
const JOURNAL_LOG_TIME: &str = "%Y-%m-%d %H:%M:%S";
/// The mark in front of a web page the shard pointed at.
pub const WEB_PAGE_MARK: &str = "The shard points at ";

/// The journal lines that came since the last look. The first look only
/// marks where the journal is, so the lines from before start nothing.
///
/// A line the window prints itself carries the number of the newest line
/// when it was printed, and lies after it; the look counts the lines of
/// the newest number it saw, so such a line is new once too.
#[derive(Default)]
pub struct NewLines {
    last: Option<u64>,
    /// How many lines with the number `last` the looks saw.
    at_last: usize,
}

impl NewLines {
    /// Lines whose first look gives every line the frame has, for a log
    /// that keeps them all.
    pub fn from_start() -> Self {
        Self {
            last: Some(0),
            at_last: 0,
        }
    }

    pub fn take<'a>(&mut self, speech: &'a [WatchSpeech]) -> Vec<&'a WatchSpeech> {
        let newest = speech.iter().map(|line| line.seq).max().unwrap_or_default();
        let count_at = |seq: u64| speech.iter().filter(|line| line.seq == seq).count();
        let Some(last) = self.last else {
            self.last = Some(newest);
            self.at_last = count_at(newest);
            return Vec::new();
        };
        let mut seen_at_last = 0;
        let new = speech
            .iter()
            .filter(|line| match line.seq.cmp(&last) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Equal => {
                    seen_at_last += 1;
                    seen_at_last > self.at_last
                }
                std::cmp::Ordering::Less => false,
            })
            .collect();
        if newest > last {
            self.last = Some(newest);
            self.at_last = count_at(newest);
        } else {
            self.at_last = self.at_last.max(seen_at_last);
        }
        new
    }
}

/// The kind of a journal line, for the tabs.
pub fn kind_of(line: &WatchSpeech) -> JournalKind {
    if line.serial == 0 {
        return JournalKind::System;
    }
    match line.kind {
        SPEECH_REGULAR => JournalKind::Speech,
        SPEECH_EMOTE => JournalKind::Emote,
        SPEECH_LABEL => JournalKind::Label,
        SPEECH_WHISPER => JournalKind::Whisper,
        SPEECH_YELL => JournalKind::Yell,
        SPEECH_SPELL => JournalKind::Spell,
        SPEECH_GUILD => JournalKind::Guild,
        SPEECH_ALLIANCE => JournalKind::Alliance,
        SPEECH_KIND_PARTY | SPEECH_KIND_PARTY_PRIVATE => JournalKind::Party,
        SPEECH_SYSTEM => JournalKind::System,
        _ => JournalKind::Speech,
    }
}

/// Where a line came from, for the lines the Journal page can hide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Mobile,
    /// An item, or the label of a thing.
    Object,
    /// The shard's own words.
    System,
    /// Words of the window: a notice the shard gave.
    Client,
}

/// One kept line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub serial: u32,
    pub name: String,
    pub text: String,
    pub hue: u16,
    pub kind: JournalKind,
    pub origin: Origin,
    /// The time of the computer when it came, as "HH:MM".
    pub stamp: String,
}

impl Entry {
    /// The line as a player reads it, with its time when asked.
    pub fn words(&self, with_stamp: bool) -> String {
        let said = if self.name.is_empty() {
            self.text.clone()
        } else {
            format!("{}: {}", self.name, self.text)
        };
        if with_stamp {
            format!("[{}] {said}", self.stamp)
        } else {
            said
        }
    }
}

fn entry_of(line: &WatchSpeech, stamp: &str) -> Entry {
    let kind = kind_of(line);
    let origin = if kind == JournalKind::System {
        Origin::System
    } else if kind == JournalKind::Label || line.serial >= FIRST_ITEM_SERIAL {
        Origin::Object
    } else {
        Origin::Mobile
    };
    Entry {
        serial: line.serial,
        name: line.name.clone(),
        text: line.text.clone(),
        hue: line.hue,
        kind,
        origin,
        stamp: stamp.to_string(),
    }
}

fn client_entry(text: String, stamp: &str) -> Entry {
    Entry {
        serial: 0,
        name: String::new(),
        text,
        hue: 0,
        kind: JournalKind::System,
        origin: Origin::Client,
        stamp: stamp.to_string(),
    }
}

/// The lines the window keeps, from the first the session sends.
pub struct JournalLog {
    entries: VecDeque<Entry>,
    new_lines: NewLines,
    /// The notices of the shard already kept, so each is kept once.
    notices: Vec<String>,
}

impl Default for JournalLog {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            new_lines: NewLines::from_start(),
            notices: Vec::new(),
        }
    }
}

impl JournalLog {
    /// Keeps the new lines of a frame, and its notices, and drops the oldest
    /// past `max_lines`.
    pub fn take(&mut self, frame: &WatchFrame, stamp: &str, max_lines: usize) {
        for line in self.new_lines.take(&frame.speech) {
            self.entries.push_back(entry_of(line, stamp));
        }
        let notices = [
            frame.shard_notice.clone(),
            frame
                .shard_url
                .as_ref()
                .map(|url| format!("{WEB_PAGE_MARK}{url}")),
        ];
        for notice in notices.into_iter().flatten() {
            if !self.notices.contains(&notice) {
                self.notices.push(notice.clone());
                self.entries.push_back(client_entry(notice, stamp));
            }
        }
        while self.entries.len() > max_lines {
            self.entries.pop_front();
        }
    }

    /// The lines one tab shows: its kinds, less what the Journal page and
    /// the ignore list hide, and only those that hold the search words.
    pub fn shown(
        &self,
        tab: &JournalTab,
        options: &JournalOptions,
        ignore: &IgnoreOptions,
        search: &str,
    ) -> Vec<&Entry> {
        let search = search.trim().to_lowercase();
        self.entries
            .iter()
            .filter(|entry| tab.kinds.contains(&entry.kind) || entry.origin == Origin::Client)
            .filter(|entry| match entry.origin {
                Origin::Client => options.show_client_lines,
                Origin::Object => options.show_object_lines,
                Origin::System => options.show_system_lines,
                Origin::Mobile => true,
            })
            .filter(|entry| {
                options.show_guild_and_alliance
                    || !matches!(entry.kind, JournalKind::Guild | JournalKind::Alliance)
            })
            .filter(|entry| {
                !ignore
                    .names
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(entry.name.trim()))
            })
            .filter(|entry| {
                search.is_empty() || entry.words(false).to_lowercase().contains(&search)
            })
            .collect()
    }
}

/// The wheel reads this many lines back for each turn.
pub const LINES_PER_TURN: usize = 3;
/// The kind of lines a new tab shows first.
pub const NEW_TAB_KIND: JournalKind = JournalKind::Speech;

/// The lines back from the newest after turns of the wheel: up reads back.
pub fn scrolled_back(back: usize, turns: i32) -> usize {
    let step = turns.unsigned_abs() as usize * LINES_PER_TURN;
    if turns > 0 {
        back + step
    } else {
        back.saturating_sub(step)
    }
}

/// A new tab of this name, which shows speech. None for no name.
pub fn new_tab(name: &str) -> Option<JournalTab> {
    let name = name.trim();
    (!name.is_empty()).then(|| JournalTab {
        name: name.to_string(),
        kinds: vec![NEW_TAB_KIND],
    })
}

/// Shows a kind of line on a tab when it is hidden, and hides it when it
/// shows.
pub fn flip_kind(tab: &mut JournalTab, kind: JournalKind) {
    match tab.kinds.iter().position(|known| *known == kind) {
        Some(at) => {
            tab.kinds.remove(at);
        }
        None => tab.kinds.push(kind),
    }
}

/// The question before a tab is deleted.
pub fn delete_question(name: &str) -> String {
    format!("Delete [{name}] tab?")
}

/// Deletes the tabs of a name.
pub fn delete_tab(tabs: &mut Vec<JournalTab>, name: &str) {
    tabs.retain(|tab| tab.name != name);
}

/// The lines that may show, `back` lines up from the newest: the newest
/// of them shows at the bottom. Gives the range and `back`, held so the
/// oldest line stays in reach.
pub fn visible(count: usize, back: usize) -> (Range<usize>, usize) {
    let back = back.min(count.saturating_sub(1));
    (0..count - back, back)
}

/// The time of the computer as the journal stamps a line.
pub fn stamp_now() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

/// The folder the journals are saved in.
pub fn journals_dir() -> PathBuf {
    uoterm_runtime::config::config_dir().join(JOURNALS_DIR)
}

/// One line of the journal file, as the reference client writes it: the time, the
/// serial of the speaker when the Speech page asks, the name and the words.
fn log_line(line: &WatchSpeech, time: &str, with_serial: bool) -> String {
    let serial = if with_serial && line.serial != 0 {
        format!("<0x{:08X}> ", line.serial)
    } else {
        String::new()
    };
    if line.name.trim().is_empty() {
        format!("[{time}]  {serial}{}", line.text)
    } else {
        format!("[{time}]  {serial}{}: {}", line.name, line.text)
    }
}

/// Opens a new journal file in `dir`, after it drops the oldest files past
/// the most the Speech page keeps, the new one among them.
fn new_log(dir: &Path, most: usize) -> std::io::Result<std::fs::File> {
    std::fs::create_dir_all(dir)?;
    let mut old: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(JOURNAL_LOG_SUFFIX))
        })
        .collect();
    old.sort();
    let keep = most.saturating_sub(1);
    let dropped = old.len().saturating_sub(keep);
    for path in old.into_iter().take(dropped) {
        std::fs::remove_file(path)?;
    }
    let name = format!(
        "{}{JOURNAL_LOG_SUFFIX}",
        chrono::Local::now().format(JOURNAL_LOG_STAMP)
    );
    std::fs::File::create(dir.join(name))
}

/// The journal file of the Speech page: each new line of the journal is
/// written to it as it comes, while the page asks for it. A file that will
/// not open is not tried again until the page turns it off and on.
#[derive(Default)]
pub struct JournalFile {
    new_lines: NewLines,
    file: Option<std::fs::File>,
    failed: bool,
    /// The folder the files go to; the journals folder when none is set.
    dir: Option<PathBuf>,
}

impl JournalFile {
    /// Writes the new lines of a frame. Call it once in each frame.
    pub fn take(&mut self, frame: &WatchFrame, speech: &SpeechOptions) {
        let lines = self.new_lines.take(&frame.speech);
        if !speech.save_journal {
            self.file = None;
            self.failed = false;
            return;
        }
        if lines.is_empty() || self.failed {
            return;
        }
        if self.file.is_none() {
            let dir = self
                .dir
                .clone()
                .unwrap_or_else(|| journals_dir().join(JOURNAL_LOGS_DIR));
            match new_log(&dir, usize::from(speech.max_journal_files)) {
                Ok(file) => self.file = Some(file),
                Err(error) => {
                    tracing::warn!(error = %error, "the journal file does not open");
                    self.failed = true;
                    return;
                }
            }
        }
        let time = chrono::Local::now().format(JOURNAL_LOG_TIME).to_string();
        let Some(file) = self.file.as_mut() else {
            return;
        };
        for line in lines {
            let words = log_line(line, &time, speech.journal_file_with_serial);
            if let Err(error) = writeln!(file, "{words}") {
                tracing::warn!(error = %error, "the journal file does not take a line");
                self.file = None;
                self.failed = true;
                return;
            }
        }
    }
}

/// Saves lines to a new text file in `dir`, named for the character and the
/// time. Gives the file.
pub fn save(
    dir: &Path,
    character: &str,
    lines: &[&Entry],
    with_stamp: bool,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let name: String = character
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let file = dir.join(format!(
        "{name}-{}.{JOURNAL_FILE_EXTENSION}",
        chrono::Local::now().format(JOURNAL_FILE_STAMP)
    ));
    let text: Vec<String> = lines.iter().map(|entry| entry.words(with_stamp)).collect();
    std::fs::write(&file, text.join("\n"))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAMP: &str = "12:30";

    fn line(seq: u64, serial: u32, name: &str, kind: u8, text: &str) -> WatchSpeech {
        WatchSpeech {
            seq,
            serial,
            name: name.into(),
            kind,
            text: text.into(),
            ..WatchSpeech::default()
        }
    }

    fn tab(kinds: &[JournalKind]) -> JournalTab {
        JournalTab {
            name: "tab".into(),
            kinds: kinds.to_vec(),
        }
    }

    fn filled() -> JournalLog {
        let mut log = JournalLog::default();
        let frame = WatchFrame {
            speech: vec![
                line(1, 5, "Ann", SPEECH_REGULAR, "hail"),
                line(2, 0, "", SPEECH_SYSTEM, "The world will save."),
                line(3, 6, "Bob", SPEECH_GUILD, "to the bank"),
                line(4, 0x4000_0001, "a sign", SPEECH_LABEL, "a sign"),
                line(5, 7, "Cid", SPEECH_KIND_PARTY, "come"),
            ],
            shard_notice: Some("Welcome".into()),
            ..WatchFrame::default()
        };
        log.take(&frame, STAMP, 10);
        log.take(&frame, STAMP, 10);
        log
    }

    #[test]
    fn the_wheel_reads_back_and_tabs_are_added_set_and_deleted() {
        assert_eq!(scrolled_back(0, 1), LINES_PER_TURN);
        assert_eq!(scrolled_back(2, -1), 0);
        assert_eq!(scrolled_back(5, 0), 5);
        assert_eq!(new_tab("  "), None);
        let mut tab = new_tab(" Chat ").unwrap();
        assert_eq!(
            (tab.name.as_str(), tab.kinds.as_slice()),
            ("Chat", &[NEW_TAB_KIND][..])
        );
        flip_kind(&mut tab, JournalKind::Yell);
        flip_kind(&mut tab, JournalKind::Speech);
        assert_eq!(tab.kinds, vec![JournalKind::Yell]);
        let mut tabs = vec![tab, new_tab("All").unwrap()];
        delete_tab(&mut tabs, "Chat");
        assert_eq!(tabs.len(), 1);
        assert_eq!(delete_question("All"), "Delete [All] tab?");
    }

    #[test]
    fn new_lines_are_kept_once_and_sorted_into_kinds() {
        let log = filled();
        let options = JournalOptions::default();
        let ignore = IgnoreOptions::default();
        let all = JournalOptions::default().tabs[0].clone();
        assert_eq!(log.shown(&all, &options, &ignore, "").len(), 6);
        let party = log.shown(&tab(&[JournalKind::Party]), &options, &ignore, "");
        assert_eq!(
            party.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            vec!["Cid", ""]
        );
        assert_eq!(party[0].words(true), "[12:30] Cid: come");
        let found = log.shown(&all, &options, &ignore, "BANK");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn the_page_and_the_ignore_list_hide_lines() {
        let log = filled();
        let all = JournalOptions::default().tabs[0].clone();
        let options = JournalOptions {
            show_system_lines: false,
            show_object_lines: false,
            show_client_lines: false,
            show_guild_and_alliance: false,
            ..JournalOptions::default()
        };
        let ignore = IgnoreOptions {
            names: vec!["ann".into()],
        };
        let shown = log.shown(&all, &options, &ignore, "");
        assert_eq!(
            shown.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            vec!["Cid"]
        );
    }

    #[test]
    fn the_oldest_lines_go_past_the_most_and_the_scroll_stays_inside() {
        let mut log = filled();
        log.take(&WatchFrame::default(), STAMP, 2);
        assert_eq!(log.entries.len(), 2);
        assert_eq!(visible(10, 0), (0..10, 0));
        assert_eq!(visible(10, 3), (0..7, 3));
        assert_eq!(visible(10, 50), (0..1, 9));
        assert_eq!(visible(0, 2), (0..0, 0));
    }

    #[test]
    fn a_saved_journal_is_a_text_file_of_the_lines() {
        let log = filled();
        let all = JournalOptions::default().tabs[0].clone();
        let lines = log.shown(
            &all,
            &JournalOptions::default(),
            &IgnoreOptions::default(),
            "hail",
        );
        let dir = std::env::temp_dir().join(format!("uoterm-journal-{}", uuid::Uuid::new_v4()));
        let file = save(&dir, "Mara the Red", &lines, false).unwrap();
        assert!(file
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("MaratheRed-"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "Ann: hail");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_journal_file_takes_each_new_line_and_keeps_the_newest_files() {
        const MOST_FILES: u16 = 2;
        let dir = std::env::temp_dir().join(format!("uoterm-journal-log-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        for old in ["2020_01_01_00_00_00", "2021_01_01_00_00_00"] {
            std::fs::write(dir.join(format!("{old}{JOURNAL_LOG_SUFFIX}")), "old").unwrap();
        }
        std::fs::write(dir.join("notes.txt"), "mine").unwrap();
        let mut file = JournalFile {
            dir: Some(dir.clone()),
            ..JournalFile::default()
        };
        let speech = SpeechOptions {
            save_journal: true,
            max_journal_files: MOST_FILES,
            journal_file_with_serial: true,
            ..SpeechOptions::default()
        };
        let first = WatchFrame {
            speech: vec![line(1, 5, "Ann", SPEECH_REGULAR, "old line")],
            ..WatchFrame::default()
        };
        file.take(&first, &speech);
        let next = WatchFrame {
            speech: vec![
                line(1, 5, "Ann", SPEECH_REGULAR, "old line"),
                line(2, 5, "Ann", SPEECH_REGULAR, "hail"),
                line(3, 0, "", SPEECH_SYSTEM, "The world will save."),
            ],
            ..WatchFrame::default()
        };
        file.take(&next, &speech);
        drop(file);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names.len(), 3, "the oldest log went: {names:?}");
        assert!(names.contains(&"notes.txt".to_string()));
        let kept_old = format!("2021_01_01_00_00_00{JOURNAL_LOG_SUFFIX}");
        assert!(names.contains(&kept_old));
        let log = names
            .iter()
            .find(|name| name.ends_with(JOURNAL_LOG_SUFFIX) && **name != kept_old)
            .unwrap();
        let written = std::fs::read_to_string(dir.join(log)).unwrap();
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(lines.len(), 2, "the line from before is not written");
        assert!(lines[0].ends_with("<0x00000005> Ann: hail"));
        assert!(lines[1].ends_with("]  The world will save."));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_first_look_marks_and_a_later_look_gives_the_new_lines() {
        let mut new_lines = NewLines::default();
        assert!(new_lines.take(&[line(4, 1, "", 0, "old")]).is_empty());
        let lines = [line(4, 1, "", 0, "old"), line(5, 1, "", 0, "new")];
        assert_eq!(new_lines.take(&lines).len(), 1);
        assert!(new_lines.take(&lines).is_empty());
        assert_eq!(
            NewLines::from_start().take(&lines).len(),
            2,
            "a log keeps the old lines"
        );
        // A line of the window after the newest, with its number.
        let printed = [
            line(4, 1, "", 0, "old"),
            line(5, 1, "", 0, "new"),
            line(5, 0, "", 0, "You are not in a party."),
        ];
        let taken = new_lines.take(&printed);
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].text, "You are not in a party.");
        assert!(new_lines.take(&printed).is_empty(), "new only once");
    }
}
