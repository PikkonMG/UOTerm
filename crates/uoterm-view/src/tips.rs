//! The tooltip of the thing under the mouse, in the words of the shard.
//! The client asks once when the mouse rests on a thing, and keeps the
//! answer for a short time, because the words of an item can change. Each
//! client draws the tooltip its own way.

use crate::act::Tip;
use std::collections::HashMap;

/// The mouse rests this long on a thing before the shard is asked.
pub const REST_SECONDS: f64 = 0.35;
/// The words of a thing are kept this long.
pub const KEEP_SECONDS: f64 = 10.0;

struct Known {
    lines: Vec<String>,
    at: f64,
}

#[derive(Default)]
pub struct Tips {
    known: HashMap<u32, Known>,
    /// The thing the mouse is on, and since when.
    resting: Option<(u32, f64)>,
    asked: Option<u32>,
}

impl Tips {
    /// Call this once in each frame, before the panels point at things,
    /// with the words the shard sent since the last frame.
    pub fn begin(&mut self, new_tips: Vec<Tip>, time: f64) {
        for tip in new_tips {
            if self.asked == Some(tip.serial) {
                self.asked = None;
            }
            self.known.insert(
                tip.serial,
                Known {
                    lines: tip.lines,
                    at: time,
                },
            );
        }
        self.known.retain(|_, known| time - known.at < KEEP_SECONDS);
    }

    /// Asks the shard with `ask` for the words of a thing once, while they
    /// are not known.
    fn ask_once(&mut self, serial: u32, ask: impl FnOnce(u32)) {
        if !self.known.contains_key(&serial) && self.asked != Some(serial) {
            self.asked = Some(serial);
            ask(serial);
        }
    }

    /// The words of a thing as the shard gave them, for a window that draws
    /// its own tooltip. The shard is asked once while they are not known.
    pub fn lines_of(&mut self, serial: u32, ask: impl FnOnce(u32)) -> Option<&[String]> {
        self.ask_once(serial, ask);
        self.known.get(&serial).map(|known| known.lines.as_slice())
    }

    /// The mouse is on this thing at `time`. Once it rested long enough the
    /// shard is asked for the words. True when it rested long enough.
    pub fn rest_on(&mut self, serial: u32, time: f64, ask: impl FnOnce(u32)) -> bool {
        let since = match self.resting {
            Some((resting, since)) if resting == serial => since,
            _ => {
                self.resting = Some((serial, time));
                time
            }
        };
        let rested = time - since >= REST_SECONDS;
        if rested {
            self.ask_once(serial, ask);
        }
        rested
    }

    /// The lines of the tooltip of a thing: the shard's words, or else
    /// `fallback` until the shard answers and when it has no words, then
    /// the window's own `extra` lines.
    pub fn shown<'a>(
        &'a self,
        serial: u32,
        fallback: &'a str,
        extra: &'a [String],
    ) -> Vec<&'a str> {
        let mut lines: Vec<&str> = match self.known.get(&serial) {
            Some(known) if !known.lines.is_empty() => {
                known.lines.iter().map(String::as_str).collect()
            }
            _ if fallback.is_empty() => Vec::new(),
            _ => vec![fallback],
        };
        lines.extend(extra.iter().map(String::as_str));
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAG: u32 = 0x4000_0002;

    #[test]
    fn the_shard_is_asked_once_after_the_mouse_rests() {
        let mut tips = Tips::default();
        let mut asked = Vec::new();
        assert!(!tips.rest_on(BAG, 0.0, |serial| asked.push(serial)));
        assert!(tips.rest_on(BAG, REST_SECONDS, |serial| asked.push(serial)));
        assert!(tips.rest_on(BAG, REST_SECONDS * 2.0, |serial| asked.push(serial)));
        assert_eq!(asked, [BAG]);
        assert_eq!(tips.shown(BAG, "a bag", &[]), ["a bag"]);
        tips.begin(
            vec![Tip {
                serial: BAG,
                lines: vec!["Backpack".into()],
            }],
            1.0,
        );
        let extra = ["7 items".to_string()];
        assert_eq!(tips.shown(BAG, "a bag", &extra), ["Backpack", "7 items"]);
        tips.begin(Vec::new(), 1.0 + KEEP_SECONDS);
        assert_eq!(tips.lines_of(BAG, |serial| asked.push(serial)), None);
        assert_eq!(asked, [BAG, BAG], "forgotten words are asked again");
    }
}
