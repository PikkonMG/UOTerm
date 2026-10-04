//! The text watch of one session, and the sizes the watch asks for. The
//! frame it prints is `uoterm_view::frame`, which the window and the web
//! client share; its types stay at this path for the window.

pub use uoterm_view::frame::*;

pub const WATCH_POLL_MS: u64 = 250;
pub const WATCH_RADAR_SIZE: u16 = 31;
/// The window asks for the largest radar, so the map without client files
/// covers as much of the window as it can.
pub const WINDOW_RADAR_SIZE: u16 = 41;
pub use uoterm_view::model::info_bar::WINDOW_TITLE;
pub use uoterm_world::WINDOW_RADAR_SIZE_WITH_ART;
pub const JOURNAL_LINES: usize = 12;

/// The picture as lines of text: vitals, goal, radar, the people near and
/// the journal. The text watch prints it.
pub fn text(frame: &WatchFrame) -> String {
    let mut out = String::new();
    if !frame.error.is_empty() {
        out.push_str(&frame.error);
        out.push('\n');
        return out;
    }
    out.push_str(&format!(
        "{}  hp {}/{}  mana {}/{}  stam {}/{}  at {},{},{} map {}  war={} dead={}\n",
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
        frame.map,
        frame.war,
        frame.dead
    ));
    out.push_str(&format!("goal {}  job {}\n", frame.goal, frame.job));
    if let (Some(dx), Some(dy)) = (frame.dest_x, frame.dest_y) {
        out.push_str(&format!("dest {dx},{dy}\n"));
    }
    for row in &frame.radar {
        out.push_str(row);
        out.push('\n');
    }
    if !frame.mobiles.is_empty() {
        out.push_str("near:\n");
        for mobile in frame.mobiles.iter().take(MOBILE_LINES) {
            out.push_str(&format!("{} d={}\n", mobile.name, mobile.dist));
        }
    }
    if !frame.journal.is_empty() {
        out.push_str("journal:\n");
        for line in frame.journal.iter().rev().take(JOURNAL_LINES).rev() {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_shows_the_radar_the_goal_the_people_near_and_the_journal() {
        let frame = WatchFrame {
            name: "Mara".to_string(),
            radar: vec!["...".to_string(), ".@X".to_string(), "...".to_string()],
            dest_x: Some(11),
            dest_y: Some(10),
            mobiles: vec![WatchMobile {
                name: "a zombie".to_string(),
                dist: 1,
                ..WatchMobile::default()
            }],
            journal: vec!["a zombie is attacking you".to_string()],
            ..WatchFrame::default()
        };
        let printed = text(&frame);
        assert!(printed.contains("\n.@X\n"));
        assert!(printed.contains("dest 11,10"));
        assert!(printed.contains("near:\na zombie d=1\n"));
        assert!(printed.contains("journal:\na zombie is attacking you\n"));
    }

    #[test]
    fn an_error_frame_prints_only_its_error() {
        assert_eq!(text(&WatchFrame::error_frame("gone")), "gone\n");
    }
}
