//! Things that happen and are gone: a hit for some damage, a body that
//! swings or bows. A snapshot of the world cannot hold them, so the world
//! keeps the last few with a number, and a window plays each one once.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uoterm_protocol::Serial;

/// How many cues the world keeps. A window asks many times each second.
pub const CUE_CAP: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CueKind {
    /// The mobile lost this many hits.
    Damage { amount: u16 },
    /// The body of the mobile plays this action.
    Animation { action: u16 },
    /// A picture that flies, flashes or stays for a moment. The serial of
    /// the cue is the source of the effect.
    Effect {
        effect: uoterm_protocol::GraphicEffect,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cue {
    /// Counts up from one. A window plays each cue with a number above the
    /// last one it played.
    pub seq: u64,
    pub serial: Serial,
    #[serde(flatten)]
    pub what: CueKind,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cues {
    cues: VecDeque<Cue>,
    last_seq: u64,
}

impl Cues {
    pub fn push(&mut self, serial: Serial, what: CueKind) {
        self.last_seq += 1;
        self.cues.push_back(Cue {
            seq: self.last_seq,
            serial,
            what,
        });
        while self.cues.len() > CUE_CAP {
            self.cues.pop_front();
        }
    }

    pub fn all(&self) -> Vec<Cue> {
        self.cues.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORC: Serial = Serial(9);

    #[test]
    fn light_weather_and_effects_reach_the_world() {
        use crate::state::WEATHER_NONE;
        use crate::World;
        use uoterm_protocol::{GraphicEffect, Inbound, Point3, EFFECT_AT_PLACE};
        const RAIN: u8 = 0;
        let mut world = World::default();
        world.apply(&Inbound::GlobalLight { level: 20 });
        world.apply(&Inbound::Weather {
            kind: RAIN,
            count: 40,
        });
        assert_eq!((world.light, world.weather), (20, Some((RAIN, 40))));
        world.apply(&Inbound::Weather {
            kind: WEATHER_NONE,
            count: 0,
        });
        assert_eq!(world.weather, None);
        let place = Point3 { x: 1, y: 2, z: 0 };
        let effect = GraphicEffect {
            kind: EFFECT_AT_PLACE,
            source: Serial(9),
            target: Serial(0),
            graphic: 0x3728,
            from: place,
            to: place,
            speed: 10,
            duration: 15,
            hue: 0,
        };
        world.apply(&Inbound::Effect(effect));
        let cue = world.cues.all().pop().unwrap();
        assert_eq!(cue.serial, Serial(9));
        assert_eq!(cue.what, CueKind::Effect { effect });
    }

    #[test]
    fn cues_count_up_and_old_ones_drop_off() {
        let mut cues = Cues::default();
        for _ in 0..CUE_CAP + 2 {
            cues.push(ORC, CueKind::Damage { amount: 5 });
        }
        let all = cues.all();
        assert_eq!(all.len(), CUE_CAP);
        assert_eq!(all[0].seq, 3);
        assert_eq!(all.last().unwrap().seq, CUE_CAP as u64 + 2);
    }

    #[test]
    fn a_cue_reads_as_flat_json() {
        let mut cues = Cues::default();
        cues.push(ORC, CueKind::Animation { action: 9 });
        let json = serde_json::to_value(cues.all()).unwrap();
        assert_eq!(json[0]["kind"], "animation");
        assert_eq!(json[0]["action"], 9);
        assert_eq!(json[0]["seq"], 1);
    }
}
