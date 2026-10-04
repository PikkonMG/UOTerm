//! How the character and each mobile move from one place to the next, the
//! footsteps they make, the actions the shard cues, and the things that
//! come into view.

use super::{build::is_person, SceneState, CORPSE_GRAPHIC};
use crate::art::{deed_action, is_mounted, Pose, WorldArt};
use crate::audio::Step;
use crate::frame::{WatchCueKind, WatchFrame, WatchLook, WatchStride};
use std::collections::HashMap;
use uoterm_nav::{Action, Deed};

/// The paces of the game: the time of one tile on foot and on a mount.
const STEP_SECONDS_FOOT_WALK: f64 = 0.4;
const STEP_SECONDS_FOOT_RUN: f64 = 0.2;
const STEP_SECONDS_MOUNT_WALK: f64 = 0.2;
const STEP_SECONDS_MOUNT_RUN: f64 = 0.1;
/// How many tiles wait while a move is under way.
const GLIDE_QUEUE_CAP: usize = 4;
/// A move with no tile queued takes this much longer than its pace.
const GLIDE_STRETCH_ALONE: f64 = 1.02;
/// A gap of this many paces, or more, is a rest between two walks.
const NEWS_REST_FACTOR: f64 = 2.0;
/// How much of each new gap goes into the learned rhythm.
const NEWS_LEARN_SHARE: f64 = 0.35;
/// Each queued tile makes the next move this much faster. A step the
/// session sent catches up by as much for each step it is late.
const GLIDE_CATCH_UP_PER_TILE: f64 = 0.08;
/// A step the session sent starts where the one before it ends when its
/// slot comes this near that end. The clock of the window reads the slot a
/// little off; the shortest real pause between two steps, a turn, is five
/// times longer.
const STRIDE_JOIN_SECONDS: f64 = 0.02;
/// The news of a step reaches the window up to one poll of the session
/// after the step, and then this much later at most: the call itself, and
/// the wait for the next frame of the window. The character takes each step
/// that long after its slot, so the news of the next step always comes
/// before this one ends, and the steps meet end to end.
const STRIDE_READ_MARGIN_SECONDS: f64 = 0.05;
/// The next place comes a moment after a move ends. A mobile keeps his walk
/// for this long, so he does not stand still for one frame between two tiles.
const STEP_LINGER_SECONDS: f64 = 0.12;
/// How long one picture of each action shows.
const STAND_FRAME_SECONDS: f64 = 0.35;
/// The least time between two footstep sounds of one walker, as the game
/// client has it.
const STEP_GAP_ON_FOOT: f64 = 0.52;
const STEP_GAP_MOUNT_RUN: f64 = 0.195;
const STEP_GAP_MOUNT_WALK: f64 = 0.455;
/// A walker who made no step for this long is forgotten.
const STEP_MEMORY_SECONDS: f64 = 5.0;
/// The character has no serial in the frame. No mobile has this one.
const SELF_STEP_KEY: u32 = 0;
/// One picture of a walk or a run shows for this long at the full pace of
/// the game. The legs go by the ground that was covered, not by the clock:
/// a slow mobile has slow legs, and a mobile that stops has still legs.
const STEP_FRAME_SECONDS: f64 = 0.08;
/// A jump longer than this is a teleport. The camera does not slide over it.
const TELEPORT_TILES: f32 = 12.0;
/// How long a shown action plays, and how long each of its pictures stays.
const SHOW_SECONDS: f64 = 0.9;
const SHOW_FRAME_SECONDS: f64 = 0.1;

/// A steady move from one place to the next. The session tells where a
/// thing is a few times each second. A thing that walks changes its tile at
/// an even pace, so the move to the new tile takes as long as the last tile
/// took. The thing then moves without a stop between two tiles.
#[derive(Clone, Copy, Debug)]
pub(super) struct Glide {
    pub from: [f32; 3],
    pub to: [f32; 3],
    started: f64,
    seconds: f64,
    /// The tiles that came while the move to `to` was under way.
    queue: [Waiting; GLIDE_QUEUE_CAP],
    queued: usize,
    /// How long one tile takes at the pace of the mobile.
    tile_seconds: f64,
    /// How long one tile took of late, by when the news of each tile came.
    /// The news of a live shard comes at uneven times. A move that is a
    /// little faster than the news stops after each tile.
    news_seconds: f64,
    last_news: f64,
    /// The tiles of the moves that ended, for the pictures of the legs.
    tiles_done: f32,
    pub running: bool,
}

/// A tile that waits for the move under way to end.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Waiting {
    at: [f32; 3],
    stride: Option<Stride>,
}

/// The timing of a move the session sent as a step, on the clock of the
/// window: when the move may start, and how long each tile of it lasts. The
/// steps of the character take exactly as long as the session gave them,
/// so they meet end to end at the pace they were sent, as in the classic
/// client. The news of a mobile has no such timing and is only heard.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stride {
    starts: f64,
    tile_seconds: f64,
}

impl Stride {
    /// When the move starts after a move that ends at `ended`: right then
    /// when the step was sent on the cadence of the one before, and at its
    /// own time after a pause.
    fn start_after(self, ended: f64) -> f64 {
        if self.starts <= ended + STRIDE_JOIN_SECONDS {
            ended
        } else {
            self.starts
        }
    }

    /// The timing of a move of `tiles` tiles whose last tile is this step:
    /// its first tile started that many steps before.
    fn over(self, tiles: f32) -> Self {
        Self {
            starts: self.starts - self.tile_seconds * f64::from((tiles - 1.0).max(0.0)),
            ..self
        }
    }
}

/// How long one tile takes. These are the paces of the game itself, so the
/// move on the screen does not depend on when the news of it came.
fn tile_seconds(mounted: bool, running: bool) -> f64 {
    match (mounted, running) {
        (false, false) => STEP_SECONDS_FOOT_WALK,
        (false, true) => STEP_SECONDS_FOOT_RUN,
        (true, false) => STEP_SECONDS_MOUNT_WALK,
        (true, true) => STEP_SECONDS_MOUNT_RUN,
    }
}

fn tiles_between(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}

impl Glide {
    pub fn resting(at: [f32; 3], time: f64) -> Self {
        Self {
            from: at,
            to: at,
            started: time,
            seconds: 0.0,
            queue: [Waiting { at, stride: None }; GLIDE_QUEUE_CAP],
            queued: 0,
            tile_seconds: STEP_SECONDS_FOOT_WALK,
            news_seconds: STEP_SECONDS_FOOT_WALK,
            last_news: time,
            tiles_done: 0.0,
            running: false,
        }
    }

    pub fn at(&self, time: f64) -> [f32; 3] {
        if self.seconds <= 0.0 {
            return self.to;
        }
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * share)
    }

    pub fn moving(&self, time: f64) -> bool {
        self.queued > 0 || (self.from != self.to && time - self.started < self.seconds)
    }

    /// The way the mobile moves now, 0 for north and then clockwise to 7.
    /// None when he rests. The shard turns him when it takes his next step,
    /// but the window still draws the step before it. With the way of the
    /// shard he would slide sideways, as on ice.
    pub fn heading(&self, time: f64) -> Option<u8> {
        /// The ways, by the step south (north, none, south) and then by the
        /// step east (west, none, east). The middle is no move.
        const WAYS: [[u8; 3]; 3] = [[7, 0, 1], [6, 0, 2], [5, 4, 3]];
        let place = |step: f32| match step {
            step if step < 0.0 => 0,
            step if step > 0.0 => 2,
            _ => 1,
        };
        let east = place(self.to[0] - self.from[0]);
        let south = place(self.to[1] - self.from[1]);
        let on_the_move = time - self.started < self.seconds + STEP_LINGER_SECONDS;
        (on_the_move && (east, south) != (1, 1)).then(|| WAYS[south][east])
    }

    /// How many tiles the mobile covered since he last rested.
    fn tiles_covered(&self, time: f64) -> f32 {
        if self.seconds <= 0.0 {
            return self.tiles_done;
        }
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        self.tiles_done + tiles_between(self.from, self.to) * share
    }

    /// What the mobile does now, and which picture of it shows. The legs of
    /// a walk go by the ground he covered. For a short time after a move he
    /// keeps his last picture, so he does not stand up between two tiles.
    pub fn pose(&self, time: f64) -> Pose {
        let stepping = self.queued > 0
            || (self.from != self.to && time - self.started < self.seconds + STEP_LINGER_SECONDS);
        if !stepping {
            return Pose {
                action: Action::Stand,
                tick: (time / STAND_FRAME_SECONDS) as usize,
            };
        }
        let pictures_per_tile = self.tile_seconds / STEP_FRAME_SECONDS;
        Pose {
            action: if self.running {
                Action::Run
            } else {
                Action::Walk
            },
            tick: (f64::from(self.tiles_covered(time)) * pictures_per_tile) as usize,
        }
    }

    /// The last tile the mobile is on his way to.
    pub fn goal(&self) -> [f32; 3] {
        match self.queued {
            0 => self.to,
            queued => self.queue[queued - 1].at,
        }
    }

    /// How long the move to the next tile takes, once it has started.
    ///
    /// A step the session sent takes exactly the time the session gave it,
    /// and a little less while it is late for its slot, so the character
    /// catches up without a jump.
    ///
    /// Heard news has no such time. With nothing queued the move is a little
    /// slow, so the next tile comes before this one ends and the mobile does
    /// not stop between two tiles. With tiles queued it is fast, so the
    /// mobile catches up.
    fn leg_seconds(&self, from: [f32; 3], to: [f32; 3], stride: Option<Stride>) -> f64 {
        let tiles = f64::from(tiles_between(from, to).max(1.0));
        if let Some(stride) = stride {
            let late = ((self.started - stride.starts) / stride.tile_seconds).max(0.0);
            return stride.tile_seconds * tiles / (1.0 + GLIDE_CATCH_UP_PER_TILE * late);
        }
        let pace = match self.queued {
            0 => GLIDE_STRETCH_ALONE,
            queued => 1.0 / (1.0 + GLIDE_CATCH_UP_PER_TILE * queued as f64),
        };
        self.tile_seconds.max(self.news_seconds) * tiles * pace
    }

    /// Takes the tile the mobile is on now, from the news of the shard. A
    /// new tile starts a move, or waits in the queue for the move under way
    /// to end. A jump is not a move.
    pub fn aim(&mut self, goal: [f32; 3], time: f64, mounted: bool, running: bool) {
        self.take(goal, time, mounted, running, None);
    }

    /// Takes the tile of a step the session sent, with the timing it gave
    /// the step.
    fn aim_stride(
        &mut self,
        goal: [f32; 3],
        time: f64,
        mounted: bool,
        running: bool,
        stride: Stride,
    ) {
        self.take(goal, time, mounted, running, Some(stride));
    }

    fn take(
        &mut self,
        goal: [f32; 3],
        time: f64,
        mounted: bool,
        running: bool,
        stride: Option<Stride>,
    ) {
        self.tile_seconds = tile_seconds(mounted, running);
        self.running = running;
        if goal != self.goal() {
            if far_apart(self.goal(), goal) {
                *self = Self::resting(goal, time);
                return;
            }
            let stride = stride.map(|stride| stride.over(tiles_between(self.goal(), goal)));
            if stride.is_none() {
                self.note_news(goal, time);
            }
            let at_rest = self.queued == 0 && time - self.started >= self.seconds;
            if at_rest {
                // A walk after a rest starts its legs from the first picture.
                let rested = time - self.started >= self.seconds + STEP_LINGER_SECONDS;
                self.tiles_done = if rested {
                    0.0
                } else {
                    self.tiles_done + tiles_between(self.from, self.to)
                };
                let ended = self.started + self.seconds;
                self.from = self.to;
                self.to = goal;
                // Late news starts now: a move never jumps ahead.
                self.started = stride.map_or(time, |stride| stride.start_after(ended).max(time));
                self.seconds = self.leg_seconds(self.from, goal, stride);
            } else {
                // A full queue loses its last tile. The move then cuts a corner.
                let place = self.queued.min(GLIDE_QUEUE_CAP - 1);
                self.queue[place] = Waiting { at: goal, stride };
                self.queued = place + 1;
            }
        }
        self.settle(time);
    }

    /// Learns the rhythm of the news. A gap that is much longer than the
    /// pace is a rest, not a rhythm.
    fn note_news(&mut self, goal: [f32; 3], time: f64) {
        let tiles = f64::from(tiles_between(self.goal(), goal).max(1.0));
        let took = (time - self.last_news) / tiles;
        self.last_news = time;
        let in_rhythm = took < self.tile_seconds * NEWS_REST_FACTOR;
        self.news_seconds = if in_rhythm {
            self.news_seconds + (took - self.news_seconds) * NEWS_LEARN_SHARE
        } else {
            self.tile_seconds
        };
    }

    /// Starts the move to the next queued tile at the moment the last move
    /// ended, so no time is lost between two tiles. A step the session sent
    /// after a pause, such as a turn, waits for its own time.
    fn settle(&mut self, time: f64) {
        while self.queued > 0 && time - self.started >= self.seconds {
            let next = self.queue[0];
            self.queue.copy_within(1.., 0);
            self.queued -= 1;
            let ended = self.started + self.seconds;
            self.started = next
                .stride
                .map_or(ended, |stride| stride.start_after(ended));
            self.tiles_done += tiles_between(self.from, self.to);
            self.from = self.to;
            self.to = next.at;
            self.seconds = self.leg_seconds(self.from, next.at, next.stride);
        }
    }
}

/// The look of a mobile turned the way he moves. None when he faces that
/// way already, or when he rests.
pub(super) fn turned_to(look: &WatchLook, heading: Option<u8>) -> Option<WatchLook> {
    let heading = heading.filter(|way| *way != look.direction)?;
    Some(WatchLook {
        direction: heading,
        ..look.clone()
    })
}

/// How long after its slot the character takes a step he was sent, when
/// the window reads the session every `poll_seconds`.
pub(super) fn stride_lag(poll_seconds: f64) -> f64 {
    poll_seconds + STRIDE_READ_MARGIN_SECONDS
}

fn far_apart(a: [f32; 3], b: [f32; 3]) -> bool {
    tiles_between(a, b) > TELEPORT_TILES
}

impl SceneState {
    /// Notes the mobiles and corpses that came into view since the last
    /// frame. The classic client asks for the name of each as it comes.
    pub(super) fn note_arrivals(&mut self, frame: &WatchFrame) {
        let general = &self.look.general;
        let mobiles = frame.mobiles.iter().map(|mobile| (mobile.serial, true));
        let corpses = frame
            .items
            .iter()
            .filter(|item| item.graphic == CORPSE_GRAPHIC)
            .map(|item| (item.serial, false));
        let now: HashMap<u32, bool> = mobiles.chain(corpses).collect();
        if let Some(before) = &self.in_view {
            let wanted = |mobile: bool| {
                if mobile {
                    general.show_incoming_mobiles
                } else {
                    general.show_incoming_corpses
                }
            };
            self.arrivals.extend(
                now.iter()
                    .filter(|(serial, mobile)| !before.contains(serial) && wanted(**mobile))
                    .map(|(serial, _)| *serial),
            );
        }
        self.in_view = Some(now.into_keys().collect());
    }

    /// The mobiles and corpses that came into view, whose names the window
    /// asks for.
    pub fn take_arrivals(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.arrivals)
    }

    /// Moves the camera and every mobile toward where the frame says it is.
    /// While a human steers, the character goes on the steps the session
    /// has sent, ahead of the shard, each over the time the session gave it.
    pub(super) fn follow(&mut self, art: &dyn WorldArt, frame: &WatchFrame, time: f64) -> bool {
        let same_map = self.camera_map == Some(frame.map);
        self.camera_map = Some(frame.map);
        let steered = same_map && frame.human_control && !frame.paralyzed;
        let sent = frame.stepping_to.filter(|_| steered);
        let shown = self.walk.show((frame.x, frame.y, frame.z), sent);
        let goal = [
            f32::from(shown.spot.0),
            f32::from(shown.spot.1),
            f32::from(shown.spot.2),
        ];
        let (mounted, running) = (is_mounted(&frame.look), frame.look.running);
        let glide = match self
            .camera_glide
            .filter(|_| same_map && !shown.snapped_back)
        {
            Some(mut glide) => {
                let stride = frame.stride.filter(|stride| {
                    steered && goal != glide.goal() && self.is_fresh(stride, time)
                });
                match stride {
                    Some(stride) => {
                        self.stride_taken = Some(stride.slot);
                        let stride = self.on_window_clock(stride);
                        glide.aim_stride(goal, time, mounted, running, stride);
                    }
                    None => glide.aim(goal, time, mounted, running),
                }
                glide
            }
            None => Glide::resting(goal, time),
        };
        if self
            .camera_glide
            .is_some_and(|before| before.goal() != glide.goal())
            && !frame.dead
            && !frame.hidden
        {
            self.step(art, SELF_STEP_KEY, &frame.look, &glide, 0.0, time);
        }
        self.camera_glide = Some(glide);
        self.camera = glide.at(time);
        let mut moving = glide.moving(time);
        let mut glides = HashMap::with_capacity(frame.mobiles.len());
        self.actors.clear();
        for mobile in &frame.mobiles {
            let goal = [
                f32::from(mobile.x),
                f32::from(mobile.y),
                f32::from(mobile.z),
            ];
            let glide = match self.glides.get(&mobile.serial).filter(|_| same_map) {
                Some(known) => {
                    let mut glide = *known;
                    glide.aim(goal, time, is_mounted(&mobile.look), mobile.look.running);
                    if glide.goal() != known.goal() {
                        self.step(
                            art,
                            mobile.serial,
                            &mobile.look,
                            &glide,
                            f32::from(mobile.dist),
                            time,
                        );
                    }
                    glide
                }
                None => Glide::resting(goal, time),
            };
            moving |= glide.moving(time);
            self.actors.insert(mobile.serial, glide.at(time));
            glides.insert(mobile.serial, glide);
        }
        self.glides = glides;
        self.last_step
            .retain(|_, at| time - *at < STEP_MEMORY_SECONDS);
        moving
    }

    /// True for the newest step when it is the news of this move: a step
    /// whose move has not started, sent no longer ago than it lasts and the
    /// news of it takes. An older one is not what moved the character.
    /// `time` is now on the clock of the window, the clock the slot is on.
    fn is_fresh(&self, stride: &WatchStride, time: f64) -> bool {
        let fresh_for = stride.lasts + self.stride_lag;
        self.stride_taken != Some(stride.slot) && time - stride.slot <= fresh_for
    }

    /// The timing of a sent step on the clock of the window: the slot is on
    /// that clock already, and the news of it comes the lag later.
    fn on_window_clock(&self, stride: WatchStride) -> Stride {
        Stride {
            starts: stride.slot + self.stride_lag,
            tile_seconds: stride.lasts,
        }
    }

    /// Notes one footstep for the sound, when the walker is a person and his
    /// last footstep is long enough ago. A jump makes no footstep.
    fn step(
        &mut self,
        art: &dyn WorldArt,
        walker: u32,
        look: &WatchLook,
        glide: &Glide,
        tiles_away: f32,
        time: f64,
    ) {
        if !is_person(art, look.body) || glide.from == glide.to {
            return;
        }
        let mounted = is_mounted(look);
        let running = glide.running;
        let gap = match (mounted, running) {
            (true, true) => STEP_GAP_MOUNT_RUN,
            (true, false) => STEP_GAP_MOUNT_WALK,
            (false, _) => STEP_GAP_ON_FOOT,
        };
        if self
            .last_step
            .get(&walker)
            .is_some_and(|at| time - at < gap)
        {
            return;
        }
        self.last_step.insert(walker, time);
        self.steps.push(Step {
            tiles_away,
            mounted,
            running,
        });
    }

    /// Takes the new animation cues. A cue from before the window opened
    /// does not play.
    pub(super) fn take_cues(&mut self, art: &dyn WorldArt, frame: &WatchFrame, time: f64) {
        let newest = frame.cues.iter().map(|cue| cue.seq).max();
        if let Some(seen) = self.last_cue {
            for cue in frame.cues.iter().filter(|cue| cue.seq > seen) {
                if cue.kind == WatchCueKind::DeathScreen {
                    self.died_at = Some(time);
                }
                let action = match cue.kind {
                    WatchCueKind::Animation(group) => u8::try_from(group).ok().map(Action::Shown),
                    WatchCueKind::Deed(kind, action) => {
                        deed_of(art, frame, cue.serial, kind, action)
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.shows.insert(cue.serial, (action, time));
                }
            }
        }
        self.last_cue = newest.or(self.last_cue).or(Some(0));
        self.shows
            .retain(|_, (_, began)| time - *began < SHOW_SECONDS);
    }

    /// The pose of a mobile that shows an action now.
    pub(super) fn shown_pose(&self, serial: u32) -> Option<Pose> {
        let (action, began) = self.shows.get(&serial)?;
        Some(Pose {
            action: *action,
            tick: ((self.now - began) / SHOW_FRAME_SECONDS) as usize,
        })
    }
}

/// The pictures for a deed of the newer animation packet. They depend on
/// the body of the mobile.
fn deed_of(
    art: &dyn WorldArt,
    frame: &WatchFrame,
    serial: u32,
    kind: u16,
    action: u16,
) -> Option<Action> {
    let look = if serial == frame.serial {
        &frame.look
    } else {
        &frame.mobiles.iter().find(|m| m.serial == serial)?.look
    };
    deed_action(art.anim(), look, Deed::from_packet(kind, action)?)
}

#[cfg(test)]
mod tests {
    use super::super::test_art::NoFiles;
    use super::*;
    use crate::frame::WatchCue;

    const ON_FOOT: bool = false;
    const WALKS: bool = false;
    const RUNS: bool = true;
    const CLOSE: f32 = 0.001;

    fn east(tiles: f32) -> [f32; 3] {
        [tiles, 0.0, 0.0]
    }

    #[test]
    fn a_new_animation_cue_plays_once_and_then_ends() {
        const ORC: u32 = 9;
        const SWING: u16 = 9;
        let mut scene = SceneState::new();
        let art = NoFiles::default();
        let mut frame = WatchFrame::default();
        scene.take_cues(&art, &frame, 0.0);
        frame.cues.push(WatchCue {
            seq: 1,
            serial: ORC,
            kind: WatchCueKind::Animation(SWING),
        });
        scene.take_cues(&art, &frame, 1.0);
        scene.now = 1.25;
        let pose = scene.shown_pose(ORC).unwrap();
        assert_eq!((pose.action, pose.tick), (Action::Shown(9), 2));
        scene.take_cues(&art, &frame, 1.0 + SHOW_SECONDS * 2.0);
        assert!(scene.shown_pose(ORC).is_none());
    }

    #[test]
    fn the_death_screen_shows_when_the_shard_says_so() {
        let mut scene = SceneState::new();
        let art = NoFiles::default();
        let mut frame = WatchFrame::default();
        scene.take_cues(&art, &frame, 0.0);
        frame.cues.push(WatchCue {
            seq: 1,
            serial: frame.serial,
            kind: WatchCueKind::DeathScreen,
        });
        scene.take_cues(&art, &frame, 2.0);
        assert_eq!(scene.died_at, Some(2.0));
    }

    #[test]
    fn a_mobile_walks_or_runs_by_what_the_shard_says_and_then_stands() {
        let mut walker = Glide::resting(east(0.0), 0.0);
        assert_eq!(walker.pose(0.0).action, Action::Stand);
        walker.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        assert_eq!(walker.pose(0.1).action, Action::Walk);
        // Five pictures for one tile on foot, spread over the move.
        let end_of_move = walker.seconds;
        assert_eq!(walker.pose(end_of_move / 2.0).tick, 2);
        assert_eq!(walker.pose(end_of_move).tick, 5);
        let end = walker.seconds;
        assert_eq!(
            walker.pose(end + STEP_LINGER_SECONDS / 2.0).action,
            Action::Walk
        );
        assert_eq!(walker.pose(end + 2.0).action, Action::Stand);
        let mut runner = Glide::resting(east(0.0), 0.0);
        runner.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        assert_eq!(runner.pose(0.05).action, Action::Run);
        assert!(runner.seconds < walker.seconds);
    }

    #[test]
    fn tiles_that_come_early_wait_and_the_move_does_not_stop_between_them() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        let first_leg = glide.seconds;
        assert!((first_leg - STEP_SECONDS_FOOT_WALK * GLIDE_STRETCH_ALONE).abs() < 1e-9);
        // The news of the next tile comes at an uneven time. It waits.
        glide.aim(east(2.0), 0.33, ON_FOOT, WALKS);
        assert_eq!(glide.queued, 1);
        assert!(glide.at(0.33)[0] < 1.0);
        // The next move starts at the moment the first one ended.
        glide.aim(east(2.0), first_leg + 0.05, ON_FOOT, WALKS);
        assert_eq!((glide.queued, glide.started), (0, first_leg));
        let speed = 1.0 / glide.seconds as f32;
        assert!((glide.at(first_leg + 0.05)[0] - (1.0 + 0.05 * speed)).abs() < CLOSE);
        assert!(glide.moving(first_leg + 0.05));
    }

    #[test]
    fn news_that_comes_late_each_time_makes_the_move_slower_so_it_does_not_stop() {
        const LATE: f64 = STEP_SECONDS_FOOT_WALK * 1.2;
        let mut glide = Glide::resting(east(0.0), 0.0);
        for tile in 1..=8 {
            glide.aim(east(tile as f32), LATE * f64::from(tile), ON_FOOT, WALKS);
        }
        assert!(glide.news_seconds > STEP_SECONDS_FOOT_WALK * 1.15);
        assert!(glide.seconds >= LATE, "the move lasts until the next news");
        // A rest does not count as a slow rhythm.
        glide.aim(east(9.0), 100.0, ON_FOOT, WALKS);
        assert_eq!(glide.news_seconds, STEP_SECONDS_FOOT_WALK);
    }

    #[test]
    fn uneven_news_gives_a_move_that_never_stops_between_tiles() {
        const FRAME: f64 = 1.0 / 60.0;
        // The session spaces its steps like a person: some early, some late.
        let gaps = [0.46, 0.35, 0.44, 0.37, 0.45, 0.34, 0.46, 0.40, 0.43, 0.36];
        let mut glide = Glide::resting(east(0.0), 0.0);
        let (mut news_at, mut tile, mut gap) = (0.0, 0.0, 0);
        let (mut time, mut before, mut stalls) = (0.0, 0.0, 0);
        while tile < 40.0 {
            if time >= news_at {
                tile += 1.0;
                news_at += gaps[gap % gaps.len()];
                gap += 1;
            }
            glide.aim(east(tile), time, ON_FOOT, WALKS);
            let at = glide.at(time)[0];
            // The first tiles teach the rhythm.
            if tile > 6.0 && at <= before {
                stalls += 1;
            }
            before = at;
            time += FRAME;
        }
        assert_eq!(stalls, 0);
        assert!(tile - before < 2.0, "the move stays near the news");
    }

    #[test]
    fn the_legs_go_by_the_ground_and_stop_when_the_mobile_stops() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        let end = glide.seconds;
        let at_end = glide.pose(end).tick;
        // After the move the legs keep their picture, then he stands.
        assert_eq!(glide.pose(end + STEP_LINGER_SECONDS / 2.0).tick, at_end);
        assert_eq!(glide.pose(end + 1.0).action, Action::Stand);
        // A slow move has the same pictures for each tile as a fast one.
        let mut slow = Glide::resting(east(0.0), 0.0);
        slow.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        slow.seconds *= 3.0;
        assert_eq!(slow.pose(slow.seconds).tick, at_end);
        // The next tile carries the legs on from where they were.
        glide.aim(east(2.0), end, ON_FOOT, RUNS);
        assert!(glide.pose(end + glide.seconds).tick > at_end);
    }

    #[test]
    fn a_mobile_faces_the_way_he_moves_and_not_the_way_of_his_next_step() {
        const NORTH: u8 = 0;
        const EAST: u8 = 2;
        const SOUTH_WEST: u8 = 5;
        let mut glide = Glide::resting(east(0.0), 0.0);
        assert_eq!(glide.heading(0.0), None);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        assert_eq!(glide.heading(0.1), Some(EAST));
        // The shard turned him north for his next step. He still goes east.
        let shard_look = WatchLook {
            direction: NORTH,
            ..WatchLook::default()
        };
        let drawn = turned_to(&shard_look, glide.heading(0.1)).unwrap();
        assert_eq!(drawn.direction, EAST);
        assert!(turned_to(&shard_look, Some(NORTH)).is_none());
        assert_eq!(glide.heading(glide.seconds + 1.0), None);
        let mut back = Glide::resting([5.0, 5.0, 0.0], 0.0);
        back.aim([4.0, 6.0, 0.0], 0.0, ON_FOOT, WALKS);
        assert_eq!(back.heading(0.1), Some(SOUTH_WEST));
    }

    #[test]
    fn a_mobile_that_is_behind_catches_up() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        let alone = glide.seconds;
        for tile in 2..=8 {
            glide.aim(east(tile as f32), 0.01, ON_FOOT, WALKS);
        }
        assert_eq!(glide.queued, GLIDE_QUEUE_CAP);
        assert_eq!(
            glide.goal(),
            east(8.0),
            "a full queue keeps the newest tile"
        );
        glide.aim(east(8.0), alone, ON_FOOT, WALKS);
        assert!(glide.seconds < STEP_SECONDS_FOOT_WALK);
    }

    /// How often the window reads the session in these tests.
    const POLL: f64 = 0.033;
    /// Each read reaches the window this much after the session made it, in
    /// turn: the call, and the wait for the next frame of the window.
    const READ_DELAYS: [f64; 5] = [0.004, 0.019, 0.001, 0.027, 0.012];
    const FRAME: f64 = 1.0 / 60.0;
    /// The pause a turn puts between two steps, as the session times it.
    const TURN: f64 = 0.1;
    /// How far off the pace a frame of a steady walk may move.
    const PACE_TOLERANCE: f64 = 0.01;

    /// One step the session sent east: its slot and how long it lasts.
    type Sent = (f64, f64);

    /// Steps sent one after the other from `first`, each lasting `lasts`.
    fn sent_steps(first: f64, lasts: f64, count: usize) -> Vec<Sent> {
        (0..count)
            .map(|step| (first + lasts * step as f64, lasts))
            .collect()
    }

    /// The character a human steers, drawn at 60 frames a second while the
    /// window reads the session each poll, as the scene takes the reads:
    /// the tile the sent steps lead to, and the newest step once. Gives the
    /// place drawn in each frame, and how long the step under way lasts.
    fn drawn_steps(steps: &[Sent]) -> Vec<(f32, f64)> {
        let lag = stride_lag(POLL);
        let last_ends = steps.last().map_or(0.0, |(slot, lasts)| slot + lasts);
        let mut glide = Glide::resting(east(0.0), 0.0);
        let mut taken = None;
        let (mut poll, mut read, mut shown) = (0.0, 0, (0usize, None::<Sent>));
        let mut drawn = Vec::new();
        let mut time = 0.0;
        while time < last_ends + lag + 1.0 {
            let delay = READ_DELAYS[read % READ_DELAYS.len()];
            if poll + delay <= time {
                let sent = steps.iter().filter(|(slot, _)| *slot <= poll).count();
                shown = (sent, sent.checked_sub(1).map(|newest| steps[newest]));
                poll += POLL;
                read += 1;
            }
            let goal = east(shown.0 as f32);
            match shown
                .1
                .filter(|step| goal != glide.goal() && taken != Some(step.0))
            {
                Some((slot, lasts)) => {
                    taken = Some(slot);
                    let stride = Stride {
                        starts: slot + lag,
                        tile_seconds: lasts,
                    };
                    glide.aim_stride(goal, time, ON_FOOT, WALKS, stride);
                }
                None => glide.aim(goal, time, ON_FOOT, WALKS),
            }
            let lasts = steps
                .iter()
                .rev()
                .find(|(slot, _)| *slot + lag <= time)
                .map_or(STEP_SECONDS_FOOT_WALK, |(_, lasts)| *lasts);
            drawn.push((glide.at(time)[0], lasts));
            time += FRAME;
        }
        drawn
    }

    /// The pace of each frame from the first move to the last: 1 is the
    /// pace the session gave the step, 0 is a stop.
    fn paces(drawn: &[(f32, f64)]) -> Vec<f64> {
        let moves: Vec<f64> = drawn
            .windows(2)
            .map(|pair| f64::from(pair[1].0 - pair[0].0) / FRAME * pair[1].1)
            .collect();
        let first = moves.iter().position(|pace| *pace > 0.0).unwrap_or(0);
        let last = moves.iter().rposition(|pace| *pace > 0.0).unwrap_or(0);
        // The first and the last frame hold only part of a move.
        moves[first + 1..last].to_vec()
    }

    fn is_on_pace(pace: f64) -> bool {
        (pace - 1.0).abs() <= PACE_TOLERANCE
    }

    /// The measured fault: the steps of the character went out on the
    /// session's tick and the window learned their rhythm like news, so his
    /// speed swung from 0.86 to 1.08 of the pace while he ran and stopped
    /// for a frame between steps. Sent steps now meet end to end: every
    /// frame moves him at the pace, however late each read comes.
    #[test]
    fn a_steered_character_moves_at_one_even_pace_over_his_sent_steps() {
        for (lasts, count) in [
            (STEP_SECONDS_FOOT_WALK, 12),
            (STEP_SECONDS_FOOT_RUN, 24),
            (STEP_SECONDS_MOUNT_RUN, 40),
        ] {
            let paces = paces(&drawn_steps(&sent_steps(0.5, lasts, count)));
            let off: Vec<&f64> = paces.iter().filter(|pace| !is_on_pace(**pace)).collect();
            assert!(off.is_empty(), "a step of {lasts} s: {off:?} of {paces:?}");
            let drawn = drawn_steps(&sent_steps(0.5, lasts, count));
            assert_eq!(drawn.last().map(|(at, _)| *at), Some(count as f32));
        }
    }

    /// A turn holds the next step back. The character stands for that
    /// long and no longer, and walks on at the pace.
    #[test]
    fn a_turn_between_two_sent_steps_is_one_short_stop() {
        let lasts = STEP_SECONDS_FOOT_RUN;
        let mut steps = sent_steps(0.5, lasts, 6);
        let after_turn = steps.last().map_or(0.0, |(slot, _)| slot + lasts + TURN);
        steps.extend(sent_steps(after_turn, lasts, 6));
        let paces = paces(&drawn_steps(&steps));
        let stops = paces.iter().filter(|pace| **pace == 0.0).count();
        let stop_frames = (TURN / FRAME).round() as usize;
        assert!(
            stops.abs_diff(stop_frames) <= 1,
            "{stops} frames stood still"
        );
        // Two frames hold part of a move and part of the stop.
        let off = paces
            .iter()
            .filter(|pace| **pace != 0.0 && !is_on_pace(**pace))
            .count();
        assert!(off <= 2, "{paces:?}");
    }

    /// The human lets the mouse go further and the walk becomes a run. The
    /// character walks at the walk and runs at the run, with no stop.
    #[test]
    fn a_walk_that_turns_into_a_run_keeps_each_pace_and_does_not_stop() {
        let mut steps = sent_steps(0.5, STEP_SECONDS_FOOT_WALK, 5);
        let run_from = steps.last().map_or(0.0, |(slot, lasts)| slot + lasts);
        steps.extend(sent_steps(run_from, STEP_SECONDS_FOOT_RUN, 10));
        let paces = paces(&drawn_steps(&steps));
        let off: Vec<&f64> = paces.iter().filter(|pace| !is_on_pace(**pace)).collect();
        // One frame holds the end of the walk and the start of the run.
        assert!(off.len() <= 1, "{off:?} of {paces:?}");
    }

    /// A read that comes later than the lag leaves the character nowhere
    /// ahead of it: he waits for it, then catches up a little on each step,
    /// and never jumps.
    #[test]
    fn a_late_read_is_caught_up_without_a_jump() {
        let lag = stride_lag(POLL);
        let lasts = STEP_SECONDS_FOOT_RUN;
        let mut glide = Glide::resting(east(0.0), 0.0);
        let late = lag + FRAME * 3.0;
        glide.aim_stride(
            east(1.0),
            late,
            ON_FOOT,
            RUNS,
            Stride {
                starts: lag,
                tile_seconds: lasts,
            },
        );
        assert_eq!(glide.at(late)[0], 0.0, "no jump");
        assert!(glide.seconds < lasts, "and a quicker step to catch up");
        assert!(glide.seconds > lasts / (1.0 + GLIDE_CATCH_UP_PER_TILE));
    }

    #[test]
    fn a_step_is_a_steady_move_and_a_teleport_is_not() {
        let leg = STEP_SECONDS_FOOT_WALK * GLIDE_STRETCH_ALONE;
        let art = NoFiles::default();
        let mut scene = SceneState::new();
        let mut frame = WatchFrame {
            x: 100,
            y: 100,
            ..WatchFrame::default()
        };
        scene.follow(&art, &frame, 0.0);
        frame.x += 1;
        assert!(scene.follow(&art, &frame, 1.0));
        assert_eq!(scene.camera[0], 100.0);
        assert!(scene.follow(&art, &frame, 1.0 + leg / 2.0));
        assert!((scene.camera[0] - 100.5).abs() < CLOSE);
        assert!(!scene.follow(&art, &frame, 1.0 + leg * 2.0));
        assert_eq!(scene.camera[0], 101.0);
        frame.x = 900;
        assert!(!scene.follow(&art, &frame, 3.0));
        assert_eq!(scene.camera[0], 900.0);
    }

    /// A step the session sent shows before the shard takes it, and a step
    /// the shard refused puts the character back at once, with no glide.
    #[test]
    fn a_sent_step_shows_before_the_shard_takes_it() {
        let art = NoFiles::default();
        let mut scene = SceneState::new();
        let resting = WatchFrame {
            x: 100,
            y: 100,
            human_control: true,
            ..WatchFrame::default()
        };
        scene.follow(&art, &resting, 0.0);
        let stepping = WatchFrame {
            stepping_to: Some((101, 100, 0)),
            ..resting.clone()
        };
        scene.follow(&art, &stepping, 0.1);
        assert_eq!(scene.camera_glide.unwrap().goal()[0], 101.0);
        scene.follow(&art, &resting, 0.2);
        assert_eq!(scene.camera, [100.0, 100.0, 0.0], "refused: back at once");
    }

    /// The newest step the watch tells times the move of the character
    /// once: the character takes it the lag after its slot, over the time
    /// it lasts. A step from long ago is not the news of a move.
    #[test]
    fn the_newest_sent_step_times_the_move_of_the_character_once() {
        const LASTS: f64 = 0.4;
        /// The time of the window when the step is sent and the frame read.
        const SENT_AT: f64 = 0.0;
        /// How many step lengths before now a step from long ago was sent.
        const LONG_AGO_STEPS: f64 = 10.0;
        /// How near two times must be to count as the same.
        const TIME_SLACK: f64 = 1e-9;
        let art = NoFiles::default();
        let stepping = |slot: f64| WatchFrame {
            x: 100,
            y: 100,
            human_control: true,
            stepping_to: Some((101, 100, 0)),
            stride: Some(WatchStride { slot, lasts: LASTS }),
            ..WatchFrame::default()
        };
        let mut scene = SceneState::new();
        scene.set_poll_every(POLL);
        let lag = scene.stride_lag;
        scene.follow(
            &art,
            &WatchFrame {
                stepping_to: None,
                ..stepping(SENT_AT)
            },
            SENT_AT,
        );
        let sent = stepping(SENT_AT);
        scene.follow(&art, &sent, SENT_AT);
        let glide = scene.camera_glide.unwrap();
        assert!(
            (glide.started - (SENT_AT + lag)).abs() < TIME_SLACK,
            "{}",
            glide.started
        );
        assert!((glide.seconds - LASTS).abs() < TIME_SLACK);
        assert_eq!(scene.stride_taken, sent.stride.map(|stride| stride.slot));

        let long_ago = SENT_AT - LASTS * LONG_AGO_STEPS;
        let mut heard = SceneState::new();
        heard.set_poll_every(POLL);
        heard.follow(
            &art,
            &WatchFrame {
                stepping_to: None,
                ..stepping(long_ago)
            },
            SENT_AT,
        );
        heard.follow(&art, &stepping(long_ago), SENT_AT);
        assert_eq!(
            heard.camera_glide.unwrap().started,
            SENT_AT,
            "heard, not timed"
        );
        assert_eq!(heard.stride_taken, None);
    }

    #[test]
    fn a_stride_is_fresh_only_within_its_length_and_lag() {
        let mut scene = SceneState::new();
        scene.set_poll_every(POLL);
        let stride = WatchStride {
            slot: 5.0,
            lasts: 0.4,
        };
        assert!(scene.is_fresh(&stride, 5.2));
        assert!(!scene.is_fresh(&stride, 6.0));
    }

    /// With client files but no animation files the default tables would
    /// take every body for a person; no footstep is heard then.
    #[test]
    fn a_walker_makes_no_footstep_without_the_real_animation_tables() {
        const MAN: u16 = 0x0190;
        let art = NoFiles::with_files();
        assert!(art.anim().is_person(MAN), "the default tables guess");
        let mut scene = SceneState::new();
        let look = WatchLook {
            body: MAN,
            ..WatchLook::default()
        };
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        scene.step(&art, SELF_STEP_KEY, &look, &glide, 0.0, 0.0);
        assert!(scene.steps.is_empty());
    }
}
