//! How long the shard takes to answer, and how many bytes go to it and come
//! from it. The session pings it now and then, and the time until the same
//! number comes back is the round trip; the bytes of each half second are
//! counted; both as the reference client measures them.

use super::*;

/// The traffic is counted over windows this long.
const TRAFFIC_WINDOW: Duration = Duration::from_millis(500);

/// The last ping sent and when, the last round trip measured, and the
/// traffic.
#[derive(Debug, Default)]
pub(super) struct Latency {
    /// The number the next ping carries. It counts up and wraps.
    next: u8,
    /// The ping on its way: its number and when it left.
    waiting: Option<(u8, Instant)>,
    last: Option<Duration>,
    pub(super) traffic: Traffic,
}

/// The bytes that came from the shard and went to it: counted as they go,
/// and how many went in the last whole window.
#[derive(Debug, Default)]
pub(super) struct Traffic {
    /// The bytes of the window that runs now, in and out.
    counting: (u64, u64),
    /// The bytes of the last whole window, in and out.
    last: (u64, u64),
    /// When the window that runs now began.
    since: Option<Instant>,
}

impl Traffic {
    pub(super) fn received(&mut self, bytes: usize) {
        self.counting.0 += bytes as u64;
    }

    pub(super) fn sent(&mut self, bytes: usize) {
        self.counting.1 += bytes as u64;
    }

    /// Closes the window that runs, once it is whole.
    pub(super) fn tick(&mut self, now: Instant) {
        let since = *self.since.get_or_insert(now);
        if now.saturating_duration_since(since) >= TRAFFIC_WINDOW {
            self.last = std::mem::take(&mut self.counting);
            self.since = Some(now);
        }
    }

    /// The bytes in and out in the last whole window.
    pub(super) fn last(&self) -> (u64, u64) {
        self.last
    }
}

impl Latency {
    /// The next ping, noted as on its way.
    fn ping(&mut self, now: Instant) -> Vec<u8> {
        let sequence = self.next;
        self.next = self.next.wrapping_add(1);
        self.waiting = Some((sequence, now));
        encode::ping(sequence)
    }

    /// The shard echoed a ping. Only the one on its way is timed: an echo of
    /// an older one says nothing about the link now.
    pub(super) fn echo(&mut self, sequence: u8, now: Instant) {
        if let Some((_, sent)) = self.waiting.filter(|(sent, _)| *sent == sequence) {
            self.last = Some(now.saturating_duration_since(sent));
            self.waiting = None;
        }
    }

    /// The last round trip in milliseconds, once one was measured.
    pub(super) fn millis(&self) -> Option<u64> {
        self.last.map(|trip| trip.as_millis() as u64)
    }
}

/// Sends a ping and starts its clock.
pub(super) fn send_ping(inner: &mut Inner) {
    let packet = inner.latency.ping(Instant::now());
    inner.outbound.push_back(packet);
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;

    #[test]
    fn the_round_trip_is_the_time_to_the_echo_of_the_last_ping() {
        const TRIP: Duration = Duration::from_millis(80);
        let mut inner = test_session();
        send_ping(&mut inner);
        send_ping(&mut inner);
        assert_eq!(
            inner.outbound.iter().collect::<Vec<_>>(),
            vec![&encode::ping(0), &encode::ping(1)],
            "each ping carries its own number"
        );
        let sent = inner.latency.waiting.map(|(_, at)| at).unwrap();
        inner.latency.echo(0, sent + TRIP);
        assert_eq!(inner.latency.millis(), None, "an older echo is not timed");
        inner.latency.echo(1, sent + TRIP);
        assert_eq!(inner.latency.millis(), Some(TRIP.as_millis() as u64));
        inner.latency.echo(1, sent + TRIP * 2);
        assert_eq!(
            inner.latency.millis(),
            Some(TRIP.as_millis() as u64),
            "a second echo of one ping is not timed again"
        );
    }

    #[test]
    fn the_traffic_of_each_whole_window_is_kept() {
        const IN: usize = 300;
        const OUT: usize = 40;
        let start = Instant::now();
        let mut traffic = Traffic::default();
        traffic.tick(start);
        traffic.received(IN);
        traffic.sent(OUT);
        traffic.tick(start + TRAFFIC_WINDOW / 2);
        assert_eq!(traffic.last(), (0, 0), "the window is not whole yet");
        traffic.tick(start + TRAFFIC_WINDOW);
        assert_eq!(traffic.last(), (IN as u64, OUT as u64));
        traffic.tick(start + TRAFFIC_WINDOW * 2);
        assert_eq!(traffic.last(), (0, 0), "a quiet window counts nothing");
    }
}
