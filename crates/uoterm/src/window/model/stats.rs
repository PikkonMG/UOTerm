//! The words of the network statistics and of the debug window, apart
//! from how they draw: the round trip to the shard and how slow it is, the
//! bytes that came and went, the frames each second, and where the
//! character is. The classic gumps and the Modern panels both show these.

use crate::view::WatchFrame;

/// The round trips under which the shard is quick, fair and slow; above
/// the last it is lagging.
const PING_LEVELS: [(u64, PingLevel); 3] = [
    (150, PingLevel::Quick),
    (200, PingLevel::Fair),
    (300, PingLevel::Slow),
];
const KILOBYTE: f64 = 1024.0;
const SIZE_UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
/// A size shows at most this many digits after the point.
const SIZE_DECIMALS: i32 = 2;
const DECIMAL_BASE: f64 = 10.0;
const SIZE_WIDTH: usize = 6;
const NO_SELECTION: &str = "-";

/// How quick the round trip to the shard is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingLevel {
    Quick,
    Fair,
    Slow,
    Lagging,
}

/// How quick a round trip is.
pub fn ping_level(ping: u64) -> PingLevel {
    PING_LEVELS
        .iter()
        .find(|(under, _)| ping < *under)
        .map_or(PingLevel::Lagging, |(_, level)| *level)
}

/// A count of bytes in the largest unit that keeps it under 1024, as
/// the reference client writes it: "300 B", "1.5 KB".
pub fn size_words(bytes: u64) -> String {
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= KILOBYTE && unit + 1 < SIZE_UNITS.len() {
        size /= KILOBYTE;
        unit += 1;
    }
    let rounding = DECIMAL_BASE.powi(SIZE_DECIMALS);
    let size = (size * rounding).round() / rounding;
    format!("{size} {}", SIZE_UNITS[unit])
}

/// The round trip of a frame, in milliseconds. None when the session has
/// not measured it yet.
pub fn ping(frame: &WatchFrame) -> u64 {
    frame.latency_ms.unwrap_or_default()
}

/// The words of the network statistics: the round trip, and the traffic
/// unless folded.
pub fn net_words(frame: &WatchFrame, folded: bool) -> String {
    let ping_line = format!("Ping: {} ms", ping(frame));
    if folded {
        return ping_line;
    }
    format!(
        "{ping_line}\nIn: {:<width$} Out: {:<width$}",
        size_words(frame.bytes_in),
        size_words(frame.bytes_out),
        width = SIZE_WIDTH
    )
}

/// The frames each second for the time the last frame took.
pub fn fps(seconds: f32) -> f32 {
    if seconds > 0.0 {
        (1.0 / seconds).round()
    } else {
        0.0
    }
}

/// What the debug window shows: the frames and the zoom, and in full
/// where the character is and the serial of the thing under the pointer.
pub struct DebugFacts {
    pub fps: f32,
    pub zoom: f32,
    pub selected: Option<u32>,
}

/// The words of the debug window, short or in full.
pub fn debug_words(frame: &WatchFrame, facts: &DebugFacts, full: bool) -> String {
    let DebugFacts {
        fps,
        zoom,
        selected,
    } = facts;
    if !full {
        return format!("FPS: {fps}\nZoom: {zoom:.2}");
    }
    let selected = selected.map_or_else(
        || NO_SELECTION.to_string(),
        |serial| format!("0x{serial:08X}"),
    );
    format!(
        "- FPS: {fps}, Zoom: {zoom:.2}\n- CharPos: {}, {}, {}\n- Map: {}\n- Selected: {selected}",
        frame.x, frame.y, frame.z, frame.map
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_goes_from_quick_to_lagging() {
        assert_eq!(ping_level(0), PingLevel::Quick);
        assert_eq!(ping_level(150), PingLevel::Fair);
        assert_eq!(ping_level(250), PingLevel::Slow);
        assert_eq!(ping_level(300), PingLevel::Lagging);
    }

    #[test]
    fn sizes_take_the_unit_that_fits() {
        assert_eq!(size_words(300), "300 B");
        assert_eq!(size_words(1536), "1.5 KB");
        assert_eq!(size_words(3 * 1024 * 1024), "3 MB");
        let frame = WatchFrame {
            latency_ms: Some(40),
            bytes_in: 1536,
            bytes_out: 12,
            ..WatchFrame::default()
        };
        assert_eq!(net_words(&frame, true), "Ping: 40 ms");
        assert_eq!(
            net_words(&frame, false),
            "Ping: 40 ms\nIn: 1.5 KB Out: 12 B  "
        );
    }

    #[test]
    fn the_debug_words_are_short_or_full() {
        let frame = WatchFrame {
            x: 1434,
            y: 1699,
            z: 5,
            map: 1,
            ..WatchFrame::default()
        };
        let facts = DebugFacts {
            fps: fps(0.02),
            zoom: 1.0,
            selected: Some(0x4000_0001),
        };
        assert_eq!(debug_words(&frame, &facts, false), "FPS: 50\nZoom: 1.00");
        let full = debug_words(&frame, &facts, true);
        assert!(full.contains("CharPos: 1434, 1699, 5"));
        assert!(full.ends_with("Selected: 0x40000001"));
        assert_eq!(fps(0.0), 0.0);
    }
}
