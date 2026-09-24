//! The health bars of both styles, apart from how they draw them: what a
//! bar shows of its mobile, when it closes by "Close health bar when",
//! what its clicks, its name and its party buttons do, which mobiles a
//! drag-select on the map takes and where their bars go, and what the map
//! asks of the bars: a bar pulled off a mobile, the box of a drag-select,
//! and the bar that follows the last target.

use crate::view::WatchFrame;
use crate::window::control::{quoted, Act};
use crate::window::scene::MapDrag;
use crate::window::settings::{CloseHealthBar, GeneralOptions, ModifierKey};
use eframe::egui::{Modifiers, Pos2, Rect, Vec2};
use uoterm_assist::mobiles::is_humanoid;
use uoterm_protocol::types::{NOTO_FRIEND, NOTO_INNOCENT, NOTO_INVULNERABLE};
use uoterm_world::is_ghost_body;

/// The spells the party buttons cast on the member: Greater Heal and Cure.
pub const SPELL_GREATER_HEAL: u16 = 29;
pub const SPELL_CURE: u16 = 11;
/// A pet's name has at most this many chars.
pub const NAME_MAX_CHARS: usize = 32;
/// The hits of another mobile come as a share of this.
pub const PERCENT_FULL: i32 = 100;
/// Drag-selected bars go down the screen from the start place, this far
/// apart, to this near the bottom and the right edge.
const SELECT_GAP: f32 = 2.0;
const SELECT_EDGE: f32 = 20.0;
const SELECT_EDGE_PAST_A_BAR: f32 = 100.0;

/// Whose bar it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    Own,
    Mobile(u32),
    /// The last target, whoever it is now.
    LastTarget,
}

impl Subject {
    /// The mobile the bar shows now. None for the target bar while no one
    /// else is the target.
    pub fn serial(self, frame: &WatchFrame) -> Option<u32> {
        match self {
            Subject::Own => Some(frame.serial),
            Subject::Mobile(serial) => Some(serial),
            Subject::LastTarget => last_target(frame),
        }
    }
}

/// The last target, when it is not the character.
pub fn last_target(frame: &WatchFrame) -> Option<u32> {
    frame.last_target.filter(|serial| *serial != frame.serial)
}

/// What one bar shows of its mobile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BarFacts {
    pub name: String,
    /// Hits, mana and stamina as current and most. None when unknown.
    pub hits: Option<(i32, i32)>,
    pub mana: Option<(i32, i32)>,
    pub stam: Option<(i32, i32)>,
    pub notoriety: Option<u8>,
    pub in_range: bool,
    pub dead: bool,
    pub poisoned: bool,
    pub yellow_hits: bool,
    /// A pet of the character: the player may change its name.
    pub renamable: bool,
    pub party: bool,
    pub own: bool,
    pub war: bool,
    /// The mobile is the last target, and a bar marks it.
    pub marked: bool,
}

/// A value as a share of its most, from 0 to 1.
pub fn share(value: Option<(i32, i32)>) -> Option<f32> {
    value.map(|(current, most)| {
        if most <= 0 {
            0.0
        } else {
            (current as f32 / most as f32).clamp(0.0, 1.0)
        }
    })
}

/// What a bar of `serial` shows now.
pub fn facts(frame: &WatchFrame, serial: u32) -> BarFacts {
    let party = frame
        .party_members
        .iter()
        .find(|member| member.serial == serial);
    let base = BarFacts {
        party: party.is_some(),
        marked: frame.last_target == Some(serial) && serial != frame.serial,
        name: party.map(|member| member.name.clone()).unwrap_or_default(),
        ..BarFacts::default()
    };
    if serial == frame.serial {
        return BarFacts {
            name: frame.name.clone(),
            hits: Some((i32::from(frame.hits), i32::from(frame.hits_max))),
            mana: Some((i32::from(frame.mana), i32::from(frame.mana_max))),
            stam: Some((i32::from(frame.stam), i32::from(frame.stam_max))),
            notoriety: Some(frame.notoriety),
            in_range: true,
            dead: frame.dead,
            poisoned: frame.poisoned,
            own: true,
            war: frame.war,
            ..base
        };
    }
    let of_full = |percent: Option<u8>| percent.map(|percent| (i32::from(percent), PERCENT_FULL));
    match frame.mobiles.iter().find(|mobile| mobile.serial == serial) {
        Some(mobile) => BarFacts {
            name: mobile.name.clone(),
            hits: of_full(mobile.hits_percent),
            mana: of_full(mobile.mana_percent),
            stam: of_full(mobile.stam_percent),
            notoriety: Some(mobile.notoriety),
            in_range: true,
            dead: is_ghost_body(mobile.look.body),
            poisoned: mobile.poisoned,
            yellow_hits: mobile.yellow_hits,
            renamable: mobile.follower,
            war: mobile.look.war,
            ..base
        },
        None => base,
    }
}

/// True when the bar closes now by the "Close health bar when" option. A
/// party member's bar and the character's never close by it, nor a bar
/// joined to others.
pub fn closes_by_rule(
    rule: CloseHealthBar,
    facts: &BarFacts,
    hits_gone: bool,
    anchored: bool,
) -> bool {
    if facts.own || facts.party || anchored {
        return false;
    }
    match rule {
        CloseHealthBar::Never => false,
        CloseHealthBar::OutOfRange => !facts.in_range,
        CloseHealthBar::Dead => facts.dead || (!facts.in_range && hits_gone),
    }
}

/// What a bar does in its first frame.
#[derive(Clone, Debug, PartialEq)]
pub enum FirstFrame {
    /// The character's own bar only shows.
    Stay,
    /// A new bar asks the shard for the status of its mobile.
    Ask(Act),
    /// A bar the profile kept closes, since the General page does not
    /// save health bars.
    Close,
}

/// What a bar of `serial` does in its first frame. `restored` is true for
/// a bar the profile kept open, opened again at the start.
pub fn first_frame(
    frame: &WatchFrame,
    serial: u32,
    restored: bool,
    general: &GeneralOptions,
) -> FirstFrame {
    if serial == frame.serial {
        FirstFrame::Stay
    } else if restored && !general.save_health_bars {
        FirstFrame::Close
    } else {
        FirstFrame::Ask(Act::MobileStatus {
            serial,
            close: false,
        })
    }
}

/// Follows whether the mobile of a bar is in range: the shard is asked for
/// its status when it comes back, and told to stop when it leaves, unless
/// it is the target. Keeps whether its hits had gone to nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct RangeWatch {
    was_in_range: Option<bool>,
    hits_gone: bool,
}

impl RangeWatch {
    /// Takes the facts of one frame. Gives the act for the shard.
    pub fn follow(&mut self, frame: &WatchFrame, serial: u32, facts: &BarFacts) -> Option<Act> {
        if facts.own {
            return None;
        }
        let was = self.was_in_range.replace(facts.in_range);
        if facts.in_range {
            self.hits_gone = facts.hits.is_some_and(|(current, _)| current == 0);
        }
        match (was, facts.in_range) {
            (Some(false), true) if facts.hits.is_none() => Some(Act::MobileStatus {
                serial,
                close: false,
            }),
            (Some(true), false) if frame.last_target != Some(serial) => Some(Act::MobileStatus {
                serial,
                close: true,
            }),
            _ => None,
        }
    }

    /// True when the hits of the mobile were at nothing when it was last
    /// in range.
    pub fn hits_gone(&self) -> bool {
        self.hits_gone
    }
}

/// The act of a click on a bar: it targets the mobile while the shard
/// waits for a target.
pub fn click_act(frame: &WatchFrame, serial: u32) -> Option<Act> {
    frame.target_cursor.then_some(Act::Target(serial))
}

/// The act of a double click on the bar of another mobile: an attack in
/// war, a use in peace. None for the character's own bar, which opens his
/// status.
pub fn double_click_act(frame: &WatchFrame, serial: u32) -> Option<Act> {
    let act = if frame.war {
        Act::Attack(serial)
    } else {
        Act::Use(serial)
    };
    (serial != frame.serial).then_some(act)
}

/// The act of a party button: the spell cast on the member.
pub fn cast_on(spell: u16, serial: u32) -> Act {
    Act::Command(format!("cast '{spell}' {serial:#X}"))
}

/// The act that gives a pet a new name. None for no name.
pub fn rename(serial: u32, name: &str) -> Option<Act> {
    let name = name.trim();
    (!name.is_empty()).then(|| Act::Command(format!("rename {serial:#X} {}", quoted(name))))
}

/// True when a pet's name may be written over now: in range, and while no
/// target waits.
pub fn name_editable(frame: &WatchFrame, facts: &BarFacts) -> bool {
    facts.renamable && facts.in_range && !frame.target_cursor
}

/// True when the keys held let a drag on the map select health bars.
/// Ctrl and Shift together never do: they show the name plates.
pub fn drag_select_allowed(general: &GeneralOptions, modifiers: Modifiers) -> bool {
    general.drag_select_health_bars
        && !(modifiers.ctrl && modifiers.shift)
        && (general.drag_select_key == ModifierKey::None
            || general.drag_select_key.is_held(modifiers))
}

/// True when a drag-select takes this mobile, by the options.
fn selectable(general: &GeneralOptions, body: u16, notoriety: u8, follower: bool) -> bool {
    let friendly = follower || [NOTO_FRIEND, NOTO_INNOCENT, NOTO_INVULNERABLE].contains(&notoriety);
    (!general.drag_select_humanoids_only || is_humanoid(body))
        && !(general.drag_select_hostiles_only && friendly)
}

/// The mobiles in the box of a drag-select that get a bar: not the
/// character, none that has a bar, and only those the options take.
pub fn selected_mobiles(
    in_box: Vec<u32>,
    frame: &WatchFrame,
    general: &GeneralOptions,
    has_bar: impl Fn(u32) -> bool,
) -> Vec<u32> {
    in_box
        .into_iter()
        .filter(|serial| *serial != frame.serial && !has_bar(*serial))
        .filter(|serial| {
            frame
                .mobiles
                .iter()
                .find(|mobile| mobile.serial == *serial)
                .is_some_and(|mobile| {
                    selectable(general, mobile.look.body, mobile.notoriety, mobile.follower)
                })
        })
        .collect()
}

/// Where the first drag-selected bar goes, from the General page.
pub fn select_start(screen: Rect, general: &GeneralOptions) -> Pos2 {
    screen.min + Vec2::new(general.drag_select_start_x, general.drag_select_start_y)
}

/// True when two boxes share more than an edge.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// Where drag-selected bars go, as the reference client lays them out: down from the
/// start place, past each bar in the way, to the next column at the
/// bottom. Each gives the bar it joins when they join. `bars` are the bars
/// open now, by the ids of the style.
pub fn select_layout<Id: Copy>(
    count: usize,
    start: Pos2,
    size: Vec2,
    screen: Rect,
    joined: bool,
    mut bars: Vec<(Id, Rect)>,
    ids: impl Fn(usize) -> Id,
) -> Vec<(Pos2, Option<Id>)> {
    let offset = if joined { 0.0 } else { SELECT_GAP };
    let mut next = start;
    let mut out = Vec::with_capacity(count);
    for at in 0..count {
        if next.y >= screen.bottom() - SELECT_EDGE {
            next.y = start.y;
            next.x += size.x + SELECT_GAP;
        }
        if next.x >= screen.right() - SELECT_EDGE {
            next.x = start.x;
        }
        bars.sort_by(|a, b| {
            (a.1.min.x, a.1.min.y)
                .partial_cmp(&(b.1.min.x, b.1.min.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut join = None;
        for (id, bar) in &bars {
            if overlaps(*bar, Rect::from_min_size(next, size)) {
                next.y = bar.bottom() + offset;
                if next.y >= screen.bottom() - SELECT_EDGE_PAST_A_BAR {
                    next.y = start.y;
                    next.x = bar.right() + offset;
                }
                if next.x >= screen.right() - SELECT_EDGE_PAST_A_BAR {
                    next.x = start.x;
                }
                if joined {
                    join = Some(*id);
                }
            }
        }
        let place = next;
        if !joined {
            next.y += size.y + SELECT_GAP;
        }
        bars.push((ids(at), Rect::from_min_size(place, size)));
        out.push((place, join));
    }
    out
}

/// What the map asks of the health bars in one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapAsk {
    /// A bar pulled off this mobile, with its middle at the mouse.
    Pull { serial: u32, mouse: Pos2 },
    /// The box of a drag-select while the button is held, to be drawn.
    Selecting(Rect),
    /// The box of a drag-select when the button came up: a bar opens for
    /// each mobile in it.
    Selected(Rect),
}

/// What the target bar does as the last target changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetBar {
    /// A new last target: the bar opens.
    Open,
    /// The new target system is off: the bar closes.
    Close,
}

/// The mouse and the keys the map bars read each frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pointer {
    pub at: Option<Pos2>,
    pub down: bool,
    pub modifiers: Modifiers,
}

/// Follows the map for the health bars: the drags the map hands over,
/// the box of a drag-select, and the last target the target bar follows.
#[derive(Default)]
pub struct MapBars {
    /// Where the box of a drag-select started, while the button is down.
    selecting: Option<Pos2>,
    target_seen: Option<u32>,
}

impl MapBars {
    /// Takes the drag the map handed over this frame, when there is one.
    pub fn follow(
        &mut self,
        drag: Option<MapDrag>,
        pointer: Pointer,
        general: &GeneralOptions,
    ) -> Option<MapAsk> {
        if let Some(drag) = drag {
            match drag.mobile {
                Some(serial) => {
                    return Some(MapAsk::Pull {
                        serial,
                        mouse: pointer.at.unwrap_or(drag.from),
                    })
                }
                None if drag_select_allowed(general, pointer.modifiers) => {
                    self.selecting = Some(drag.from);
                }
                None => {}
            }
        }
        let (Some(from), Some(at)) = (self.selecting, pointer.at) else {
            return None;
        };
        let area = Rect::from_two_pos(from, at);
        if pointer.down {
            return Some(MapAsk::Selecting(area));
        }
        self.selecting = None;
        (area.width() > 0.0 || area.height() > 0.0).then_some(MapAsk::Selected(area))
    }

    /// Forgets a drag-select that has begun.
    pub fn stop_selecting(&mut self) {
        self.selecting = None;
    }

    /// Opens the target bar for each new last target while the new target
    /// system is on, and closes it when the option is off.
    pub fn follow_target(
        &mut self,
        frame: &WatchFrame,
        new_target_system: bool,
    ) -> Option<TargetBar> {
        let target = last_target(frame);
        let seen = std::mem::replace(&mut self.target_seen, target);
        if !new_target_system {
            return Some(TargetBar::Close);
        }
        (target.is_some() && target != seen).then_some(TargetBar::Open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchLook, WatchMobile, WatchPartyMember};
    use uoterm_protocol::types::NOTO_GREY;

    const ORC: u32 = 0x0000_0B01;
    const FRIEND: u32 = 0x0000_0B02;
    const ME: u32 = 0x0000_0001;

    fn frame() -> WatchFrame {
        WatchFrame {
            serial: ME,
            name: "Mara".into(),
            hits: 40,
            hits_max: 50,
            mana: 10,
            mana_max: 20,
            war: true,
            last_target: Some(ORC),
            mobiles: vec![WatchMobile {
                serial: ORC,
                name: "an orc".into(),
                hits_percent: Some(0),
                notoriety: NOTO_GREY,
                look: WatchLook {
                    body: 0x0011,
                    ..WatchLook::default()
                },
                ..WatchMobile::default()
            }],
            party_members: vec![WatchPartyMember {
                serial: FRIEND,
                name: "Bob".into(),
                hits_percent: Some(80),
                ..WatchPartyMember::default()
            }],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_bar_knows_its_own_the_mobiles_in_range_and_the_party_out_of_it() {
        let frame = frame();
        let own = facts(&frame, ME);
        assert!(own.own && own.war && own.in_range);
        assert_eq!((own.hits, own.mana), (Some((40, 50)), Some((10, 20))));
        assert_eq!(share(own.hits), Some(0.8));
        let orc = facts(&frame, ORC);
        assert!(orc.in_range && orc.marked && !orc.party);
        assert_eq!(orc.hits, Some((0, PERCENT_FULL)));
        let bob = facts(&frame, FRIEND);
        assert!(bob.party && !bob.in_range);
        assert_eq!(bob.name, "Bob");
        assert_eq!(bob.hits, None, "a member out of range shows no hits");
        let mut near = frame.clone();
        near.mobiles.push(WatchMobile {
            serial: FRIEND,
            name: "Bob".into(),
            hits_percent: Some(80),
            mana_percent: Some(40),
            stam_percent: Some(100),
            ..WatchMobile::default()
        });
        let bob = facts(&near, FRIEND);
        assert!(bob.party && bob.in_range);
        assert_eq!(
            (bob.hits, bob.mana, bob.stam),
            (Some((80, 100)), Some((40, 100)), Some((100, 100)))
        );
        assert_eq!(share(Some((5, 0))), Some(0.0));
    }

    #[test]
    fn bars_close_by_the_rule_but_never_the_party_the_own_or_the_joined() {
        let frame = frame();
        let mut orc = facts(&frame, ORC);
        orc.in_range = false;
        assert!(closes_by_rule(
            CloseHealthBar::OutOfRange,
            &orc,
            false,
            false
        ));
        assert!(!closes_by_rule(
            CloseHealthBar::OutOfRange,
            &orc,
            false,
            true
        ));
        assert!(!closes_by_rule(CloseHealthBar::Never, &orc, false, false));
        assert!(closes_by_rule(CloseHealthBar::Dead, &orc, true, false));
        assert!(!closes_by_rule(CloseHealthBar::Dead, &orc, false, false));
        let bob = facts(&frame, FRIEND);
        assert!(!closes_by_rule(
            CloseHealthBar::OutOfRange,
            &bob,
            false,
            false
        ));
        let own = facts(&frame, ME);
        assert!(!closes_by_rule(
            CloseHealthBar::OutOfRange,
            &own,
            false,
            false
        ));
    }

    #[test]
    fn a_new_bar_asks_for_the_status_and_a_kept_one_needs_the_option() {
        let frame = frame();
        let mut general = GeneralOptions::default();
        assert_eq!(first_frame(&frame, ME, true, &general), FirstFrame::Stay);
        assert_eq!(first_frame(&frame, ORC, true, &general), FirstFrame::Close);
        general.save_health_bars = true;
        assert_eq!(
            first_frame(&frame, ORC, true, &general),
            FirstFrame::Ask(Act::MobileStatus {
                serial: ORC,
                close: false
            })
        );
    }

    #[test]
    fn the_range_watch_asks_again_on_return_and_stops_on_leaving() {
        let mut frame = frame();
        frame.last_target = None;
        let mut watch = RangeWatch::default();
        let seen = facts(&frame, ORC);
        assert_eq!(watch.follow(&frame, ORC, &seen), None);
        assert!(watch.hits_gone());
        let gone = BarFacts {
            in_range: false,
            hits: None,
            ..seen.clone()
        };
        assert_eq!(
            watch.follow(&frame, ORC, &gone),
            Some(Act::MobileStatus {
                serial: ORC,
                close: true
            })
        );
        assert_eq!(
            watch.follow(
                &frame,
                ORC,
                &BarFacts {
                    in_range: true,
                    ..gone
                }
            ),
            Some(Act::MobileStatus {
                serial: ORC,
                close: false
            })
        );
    }

    #[test]
    fn clicks_target_attack_or_use_and_names_need_words() {
        let mut frame = frame();
        assert_eq!(click_act(&frame, ORC), None);
        frame.target_cursor = true;
        assert_eq!(click_act(&frame, ORC), Some(Act::Target(ORC)));
        assert_eq!(double_click_act(&frame, ORC), Some(Act::Attack(ORC)));
        frame.war = false;
        assert_eq!(double_click_act(&frame, ORC), Some(Act::Use(ORC)));
        assert_eq!(double_click_act(&frame, ME), None);
        assert_eq!(rename(ORC, "  "), None);
        assert_eq!(
            rename(0x0B01, "Rex"),
            Some(Act::Command("rename 0xB01 'Rex'".into()))
        );
        assert_eq!(
            cast_on(SPELL_CURE, 0x0B02),
            Act::Command("cast '11' 0xB02".into())
        );
    }

    #[test]
    fn the_keys_and_the_filters_of_drag_select_follow_the_options() {
        let mut general = GeneralOptions {
            drag_select_health_bars: true,
            ..GeneralOptions::default()
        };
        assert!(drag_select_allowed(&general, Modifiers::NONE));
        assert!(!drag_select_allowed(
            &general,
            Modifiers::CTRL | Modifiers::SHIFT
        ));
        general.drag_select_key = ModifierKey::Ctrl;
        assert!(!drag_select_allowed(&general, Modifiers::NONE));
        assert!(drag_select_allowed(&general, Modifiers::CTRL));
        general.drag_select_key = ModifierKey::Alt;
        assert!(drag_select_allowed(&general, Modifiers::ALT));
        assert!(!drag_select_allowed(&general, Modifiers::CTRL));
        general.drag_select_health_bars = false;
        assert!(!drag_select_allowed(&general, Modifiers::CTRL));
        const HUMAN: u16 = 0x0190;
        const ORC_BODY: u16 = 0x0011;
        general.drag_select_humanoids_only = true;
        assert!(!selectable(&general, ORC_BODY, NOTO_GREY, false));
        assert!(selectable(&general, HUMAN, NOTO_INNOCENT, false));
        general.drag_select_hostiles_only = true;
        assert!(!selectable(&general, HUMAN, NOTO_INNOCENT, false));
        assert!(
            !selectable(&general, HUMAN, NOTO_GREY, true),
            "a pet is no foe"
        );
        assert!(selectable(&general, HUMAN, NOTO_GREY, false));
        let frame = frame();
        let general = GeneralOptions::default();
        let chosen = selected_mobiles(vec![ME, ORC, FRIEND], &frame, &general, |_| false);
        assert_eq!(chosen, vec![ORC], "not the character, only mobiles in view");
        assert!(selected_mobiles(vec![ORC], &frame, &general, |_| true).is_empty());
    }

    #[test]
    fn drag_selected_bars_go_down_the_screen_and_join_when_asked() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 240.0));
        let size = Vec2::new(100.0, 60.0);
        let start = Pos2::new(100.0, 100.0);
        let id = |at: usize| at as u32;
        let loose = select_layout(3, start, size, screen, false, Vec::new(), id);
        assert_eq!(loose[0], (start, None));
        assert_eq!(loose[1].0, Pos2::new(100.0, 162.0));
        assert_eq!(
            loose[2].0,
            Pos2::new(202.0, 100.0),
            "the bottom edge starts a new column"
        );
        let tall = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 800.0));
        let joined = select_layout(2, start, size, tall, true, Vec::new(), id);
        assert_eq!(joined[0], (start, None));
        assert_eq!(joined[1], (Pos2::new(100.0, 160.0), Some(id(0))));
        let open = vec![(9, Rect::from_min_size(start, size))];
        let past = select_layout(1, start, size, tall, false, open, id);
        assert_eq!(past[0].0, Pos2::new(100.0, 162.0), "an open bar is passed");
    }

    #[test]
    fn the_map_pulls_bars_draws_the_box_and_follows_the_target() {
        let general = GeneralOptions {
            drag_select_health_bars: true,
            ..GeneralOptions::default()
        };
        let mut bars = MapBars::default();
        let mouse = Pos2::new(50.0, 60.0);
        let held = Pointer {
            at: Some(mouse),
            down: true,
            modifiers: Modifiers::NONE,
        };
        let pulled = MapDrag {
            from: Pos2::ZERO,
            mobile: Some(ORC),
        };
        assert_eq!(
            bars.follow(Some(pulled), held, &general),
            Some(MapAsk::Pull { serial: ORC, mouse })
        );
        let boxed = MapDrag {
            from: Pos2::new(10.0, 10.0),
            mobile: None,
        };
        let area = Rect::from_two_pos(Pos2::new(10.0, 10.0), mouse);
        assert_eq!(
            bars.follow(Some(boxed), held, &general),
            Some(MapAsk::Selecting(area))
        );
        let up = Pointer {
            down: false,
            ..held
        };
        assert_eq!(
            bars.follow(None, up, &general),
            Some(MapAsk::Selected(area))
        );
        assert_eq!(bars.follow(None, up, &general), None, "the box is gone");
        let frame = frame();
        assert_eq!(bars.follow_target(&frame, true), Some(TargetBar::Open));
        assert_eq!(bars.follow_target(&frame, true), None, "the same target");
        assert_eq!(bars.follow_target(&frame, false), Some(TargetBar::Close));
    }
}
