//! Walk prediction for the character a human steers. The window shows each
//! step the moment the session sends it, as the classic client does, and
//! does not wait for the shard to take it. It shows no step the session has
//! not sent: a step the window guessed by itself was shown before it went
//! out, and when the human stopped first it never went out at all, so the
//! character was put back a second later and the screen jerked. When the
//! shard refuses a step, the session takes it off the wire, and the window
//! puts the character back at once, as the classic client does.

/// One tile and the height on it.
pub type Spot = (u16, u16, i8);

/// Where the window shows the character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub spot: Spot,
    /// The shard refused the steps shown: the character goes back at once.
    pub snapped_back: bool,
}

#[derive(Default)]
pub struct WalkPrediction {
    /// The tile the sent steps led to when the window looked last.
    sent: Option<Spot>,
}

fn same_tile(a: Spot, b: Spot) -> bool {
    (a.0, a.1) == (b.0, b.1)
}

impl WalkPrediction {
    /// Where to show the character now. `shard` is where the shard has him,
    /// and `sent` the tile the steps the session sent leave him on, None
    /// when none are out.
    pub fn show(&mut self, shard: Spot, sent: Option<Spot>) -> Shown {
        // The steps shown left the wire and the shard did not put him where
        // they led: it refused them.
        let snapped_back =
            sent.is_none() && self.sent.is_some_and(|shown| !same_tile(shown, shard));
        self.sent = sent;
        Shown {
            spot: sent.unwrap_or(shard),
            snapped_back,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: Spot = (100, 100, 0);
    const EAST_OF_HOME: Spot = (101, 100, 0);

    #[test]
    fn a_sent_step_shows_at_once_and_the_shard_confirms_it() {
        let mut walk = WalkPrediction::default();
        assert_eq!(walk.show(HOME, None).spot, HOME, "nothing sent");
        let sent = walk.show(HOME, Some(EAST_OF_HOME));
        assert_eq!(sent.spot, EAST_OF_HOME, "the step shows once it is out");
        let taken = walk.show(EAST_OF_HOME, None);
        assert_eq!(taken.spot, EAST_OF_HOME);
        assert!(!taken.snapped_back);
    }

    /// The human lets go of the key. The session sends nothing more, so the
    /// window shows nothing more, and nothing has to be taken back.
    #[test]
    fn a_stop_shows_no_step_that_was_not_sent() {
        let mut walk = WalkPrediction::default();
        walk.show(HOME, Some(EAST_OF_HOME));
        walk.show(EAST_OF_HOME, None);
        for _ in 0..2 {
            let resting = walk.show(EAST_OF_HOME, None);
            assert_eq!(resting.spot, EAST_OF_HOME);
            assert!(!resting.snapped_back);
        }
    }

    #[test]
    fn a_refused_step_snaps_back_at_once() {
        let mut walk = WalkPrediction::default();
        walk.show(HOME, Some(EAST_OF_HOME));
        let back = walk.show(HOME, None);
        assert!(back.snapped_back);
        assert_eq!(back.spot, HOME);
        assert!(!walk.show(HOME, None).snapped_back, "once");
    }

    #[test]
    fn the_shard_wins_when_it_moves_him_with_nothing_sent() {
        let mut walk = WalkPrediction::default();
        walk.show(HOME, None);
        let slid = (101, 101, 0);
        let shown = walk.show(slid, None);
        assert_eq!(shown.spot, slid);
        assert!(!shown.snapped_back);
    }
}
