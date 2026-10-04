//! The journal files: the time stamps of the computer, the journals saved
//! from the journal, and the journal file the Speech page keeps, written
//! line by line as the lines come. The lines are made by the rules of
//! `uoterm_view::model::journal`.

use crate::view::WatchFrame;
use crate::window::settings::SpeechOptions;
use chrono::{Datelike, Timelike};
use std::io::Write;
use std::path::{Path, PathBuf};
use uoterm_view::model::journal::{
    log_line, saved_file_name, saved_text, Entry, LocalTime, NewLines,
};

const JOURNALS_DIR: &str = "journals";
/// The journal files the Speech page keeps lie here in the journals folder,
/// each named for the time it began, as the reference client names them.
const JOURNAL_LOGS_DIR: &str = "logs";
const JOURNAL_LOG_STAMP: &str = "%Y_%m_%d_%H_%M_%S";
const JOURNAL_LOG_SUFFIX: &str = "_journal.txt";
const JOURNAL_LOG_TIME: &str = "%Y-%m-%d %H:%M:%S";

/// The time of the computer now.
fn local_now() -> LocalTime {
    let now = chrono::Local::now();
    LocalTime {
        year: now.year(),
        month: now.month(),
        day: now.day(),
        hour: now.hour(),
        minute: now.minute(),
        second: now.second(),
    }
}

/// The time of the computer as the journal stamps a line.
pub fn stamp_now() -> String {
    local_now().line_stamp()
}

/// The folder the journals are saved in.
pub fn journals_dir() -> PathBuf {
    uoterm_runtime::config::config_dir().join(JOURNALS_DIR)
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
    let stamp = local_now().file_stamp();
    let file = dir.join(saved_file_name(character, &stamp));
    std::fs::write(&file, saved_text(lines, with_stamp))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchSpeech;
    use uoterm_protocol::types::{SPEECH_REGULAR, SPEECH_SYSTEM};
    use uoterm_view::model::journal::JournalLog;
    use uoterm_view::settings::{IgnoreOptions, JournalOptions};

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

    #[test]
    fn a_saved_journal_is_a_text_file_of_the_lines() {
        let mut log = JournalLog::default();
        let frame = WatchFrame {
            speech: vec![
                line(1, 5, "Ann", SPEECH_REGULAR, "hail"),
                line(2, 6, "Bob", SPEECH_REGULAR, "well met"),
            ],
            ..WatchFrame::default()
        };
        log.take(&frame, STAMP, 10);
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
}
