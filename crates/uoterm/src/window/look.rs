//! How the world looks by the choices of the Options screen: the hue a
//! mobile or an item takes, the ring under the feet, the hit points over a
//! head, the circle of transparency and the fading of things. The rules are
//! those of the reference client. The profile is read once for each frame.

use super::settings::{
    AuraRule, CircleStyle, CombatOptions, GeneralOptions, HpShowWhen, HpStyle, NameplateFilter,
    NameplateOptions, Profile, UiStyle, VideoOptions,
};
use crate::view::WatchLook;
use uoterm_protocol::types::{
    NOTO_CRIMINAL, NOTO_ENEMY, NOTO_FRIEND, NOTO_GREY, NOTO_INNOCENT, NOTO_INVULNERABLE,
    NOTO_MURDERER,
};

/// The hue of the thing under the mouse.
pub const HIGHLIGHT_HUE: u16 = 0x0014;
/// The hue of a thing out of range.
pub const OUT_OF_RANGE_HUE: u16 = 0x038B;
/// The hue of the whole world while the character is dead.
pub const DEAD_WORLD_HUE: u16 = 0x038E;
/// The hue of a hidden mobile.
pub const HIDDEN_HUE: u16 = 0x038E;
/// The hue of a dead creature that is no person.
pub const DEAD_CREATURE_HUE: u16 = 0x0386;
/// The hue of the invulnerable.
pub const INVULNERABLE_NOTORIETY_HUE: u16 = 0x0034;
/// Farther than this many tiles a thing is out of range.
pub const VIEW_RANGE: u16 = 18;

const FULL_PERCENT: u8 = 100;
/// The hues of the hit points over a head, from low to full.
const HITS_LOW_PERCENT: u8 = 30;
const HITS_HALF_PERCENT: u8 = 50;
const HITS_HIGH_PERCENT: u8 = 80;
const HITS_LOW_HUE: u16 = 0x0021;
const HITS_HALF_HUE: u16 = 0x0030;
const HITS_HIGH_HUE: u16 = 0x0058;
const HITS_FULL_HUE: u16 = 0x0044;

/// A circle of transparency hides all inside this share of its radius, and
/// fades things out between it and the edge.
const CIRCLE_SOLID_SHARE: f32 = 0.85;
/// How fast a thing fades in or out: the classic client steps its alpha by
/// 25 of 255 each 20 milliseconds.
const FADE_PER_SECOND: f32 = 25.0 / 255.0 / 0.020;
/// A translucent static shows at this share.
pub const TRANSLUCENT_ALPHA: f32 = 178.0 / 255.0;
/// Things this much taller than a static may be seen through.
const SEE_THROUGH_HEIGHT: u8 = 5;

/// The choices of the profile that change how the world is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldLook {
    pub style: UiStyle,
    pub general: GeneralOptions,
    pub video: VideoOptions,
    pub combat: CombatOptions,
    pub nameplates: NameplateOptions,
}

impl WorldLook {
    pub fn of(profile: &Profile) -> Self {
        Self {
            style: profile.interface.ui_style,
            general: profile.general.clone(),
            video: profile.video.clone(),
            combat: profile.combat.clone(),
            nameplates: profile.nameplates.clone(),
        }
    }

    /// The look of the official client, not the glass of the command deck.
    pub fn classic(&self) -> bool {
        self.style == UiStyle::Classic
    }

    /// The hue of the name and of the ring of a mobile of this notoriety.
    pub fn notoriety_hue(&self, notoriety: u8) -> u16 {
        notoriety_hue(&self.combat, notoriety)
    }
}

/// The hue of the name and of the ring of a mobile of this notoriety, from
/// the Combat & Spells page. Zero for one of no notoriety.
pub fn notoriety_hue(combat: &CombatOptions, notoriety: u8) -> u16 {
    match notoriety {
        NOTO_INNOCENT => combat.innocent_hue,
        NOTO_FRIEND => combat.friend_hue,
        NOTO_GREY => combat.can_attack_hue,
        NOTO_CRIMINAL => combat.criminal_hue,
        NOTO_ENEMY => combat.enemy_hue,
        NOTO_MURDERER => combat.murderer_hue,
        NOTO_INVULNERABLE => INVULNERABLE_NOTORIETY_HUE,
        _ => 0,
    }
}

/// The look of a mobile without the worn layers the player hides, or None
/// when every layer shows. `own` is the character himself.
pub fn without_hidden_layers(
    general: &GeneralOptions,
    look: &WatchLook,
    own: bool,
) -> Option<WatchLook> {
    let hidden = |layer: u8| general.hides_layer(layer, own);
    if !look.equipment.iter().any(|item| hidden(item.layer)) {
        return None;
    }
    Some(WatchLook {
        equipment: look
            .equipment
            .iter()
            .filter(|item| !hidden(item.layer))
            .cloned()
            .collect(),
        ..look.clone()
    })
}

/// What the window knows of a mobile for the hue it is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MobileState {
    pub own: bool,
    pub hovered: bool,
    /// The character fights him, or points a target cursor at him.
    pub marked: bool,
    pub out_of_range: bool,
    pub hidden: bool,
    pub dead: bool,
    pub person: bool,
    pub poisoned: bool,
    pub paralyzed: bool,
    pub yellow_hits: bool,
    pub notoriety: u8,
}

/// The hue that covers the whole of a mobile, or None for his own colors.
/// The order is the classic client's: the mouse, the range, a dead world,
/// hiding, death, then poison, paralysis and the yellow bar. The foe and
/// the mobile under a target cursor show their notoriety over all of it.
pub fn mobile_hue(look: &WorldLook, dead_world: bool, state: MobileState) -> Option<u16> {
    let general = &look.general;
    if !state.own && state.marked {
        return Some(look.notoriety_hue(state.notoriety)).filter(|hue| *hue != 0);
    }
    if general.highlight_objects && state.hovered {
        return Some(HIGHLIGHT_HUE);
    }
    if general.out_of_range_no_color && state.out_of_range {
        return Some(OUT_OF_RANGE_HUE);
    }
    if dead_world && look.video.black_and_white_when_dead {
        return Some(DEAD_WORLD_HUE);
    }
    if state.hidden {
        return Some(HIDDEN_HUE);
    }
    if state.dead {
        return (!state.person).then_some(DEAD_CREATURE_HUE);
    }
    let invulnerable = state.notoriety == NOTO_INVULNERABLE;
    let mut hue = None;
    if general.highlight_poisoned && state.poisoned {
        hue = Some(general.poisoned_hue);
    }
    if general.highlight_paralyzed && state.paralyzed && !invulnerable {
        hue = Some(general.paralyzed_hue);
    }
    if general.highlight_invulnerable && state.yellow_hits && !invulnerable {
        hue = Some(general.invulnerable_hue);
    }
    hue
}

/// The hue that covers the whole of an item or a static, or None for its
/// own colors.
pub fn thing_hue(
    look: &WorldLook,
    dead_world: bool,
    hovered: bool,
    out_of_range: bool,
) -> Option<u16> {
    if look.general.highlight_objects && hovered {
        Some(HIGHLIGHT_HUE)
    } else if look.general.out_of_range_no_color && out_of_range {
        Some(OUT_OF_RANGE_HUE)
    } else if dead_world && look.video.black_and_white_when_dead {
        Some(DEAD_WORLD_HUE)
    } else {
        None
    }
}

/// True when the rings of color under the feet of mobiles show.
pub fn aura_shows(rule: AuraRule, war: bool, ctrl_shift: bool) -> bool {
    match rule {
        AuraRule::Never => false,
        AuraRule::WarMode => war,
        AuraRule::CtrlShift => ctrl_shift,
        AuraRule::Always => true,
    }
}

/// What shows of the hit points of one mobile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitsShown {
    /// The share in words over his head.
    pub percent: bool,
    /// The line under his feet.
    pub line: bool,
}

/// What shows of the hit points of a mobile with `percent` left. A mobile
/// the character fights shows his line whatever the choices say.
pub fn hits_shown(
    general: &GeneralOptions,
    percent: Option<u8>,
    fought: bool,
    dead: bool,
) -> HitsShown {
    let Some(percent) = percent else {
        return HitsShown::default();
    };
    let full = percent >= FULL_PERCENT;
    if !general.show_mobile_hp {
        return HitsShown {
            percent: false,
            line: fought,
        };
    }
    if general.mobile_hp_when == HpShowWhen::BelowFull && full {
        return HitsShown::default();
    }
    let wants_percent = general.mobile_hp_style != HpStyle::Line;
    let wants_line = general.mobile_hp_style != HpStyle::Percentage;
    let percent_now = match general.mobile_hp_when {
        HpShowWhen::Smart => !full,
        HpShowWhen::Always | HpShowWhen::BelowFull => true,
    };
    HitsShown {
        percent: wants_percent && percent_now && percent > 0 && !dead,
        line: wants_line || fought,
    }
}

/// The hue of the hit points in words over a head.
pub fn hits_hue(percent: u8) -> u16 {
    match percent {
        p if p < HITS_LOW_PERCENT => HITS_LOW_HUE,
        p if p < HITS_HALF_PERCENT => HITS_HALF_HUE,
        p if p < HITS_HIGH_PERCENT => HITS_HIGH_HUE,
        _ => HITS_FULL_HUE,
    }
}

/// What a name plate is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlateOf {
    Mobile,
    Item,
    Corpse,
}

/// True when a name plate shows over a thing. The plates show while they
/// are on, or while Ctrl and Shift are held, as the classic client shows
/// the names over heads.
pub fn plate_shows(
    plates: &NameplateOptions,
    of: PlateOf,
    ctrl_shift: bool,
    full_health: bool,
) -> bool {
    if !plates.enabled && !ctrl_shift {
        return false;
    }
    if plates.hide_at_full_health && full_health && of == PlateOf::Mobile {
        return false;
    }
    match plates.filter {
        NameplateFilter::All => true,
        NameplateFilter::Mobiles => of == PlateOf::Mobile,
        NameplateFilter::Items => of == PlateOf::Item,
        NameplateFilter::Corpses => of == PlateOf::Corpse,
        NameplateFilter::MobilesAndCorpses => of != PlateOf::Item,
    }
}

/// How much of a thing shows at `ratio` of the radius of the circle of
/// transparency from its middle: nothing inside, all of it outside.
pub fn circle_alpha(style: CircleStyle, ratio: f32) -> f32 {
    match style {
        CircleStyle::Gradient => ratio.clamp(0.0, 1.0).powi(3),
        CircleStyle::Full if ratio < CIRCLE_SOLID_SHARE => 0.0,
        CircleStyle::Full if ratio < 1.0 => {
            ((ratio - CIRCLE_SOLID_SHARE) / (1.0 - CIRCLE_SOLID_SHARE)).powi(3)
        }
        CircleStyle::Full => 1.0,
    }
}

/// True when the circle of transparency may show through a static of this
/// height and these flags at `z`, for a character at `own_z`. A low thing
/// under his feet, or a small thing over his head, stays.
pub fn see_through(
    z: i8,
    height: u8,
    surface: bool,
    background: bool,
    roof_or_wall: bool,
    own_z: i8,
) -> bool {
    let test_z = i16::from(own_z) + i16::from(SEE_THROUGH_HEIGHT);
    let can_be_transparent = height > SEE_THROUGH_HEIGHT
        || height == 0
        || roof_or_wall
        || surface && (background || height == SEE_THROUGH_HEIGHT);
    if i16::from(z) <= test_z - i16::from(height) {
        return false;
    }
    test_z >= i16::from(z) || can_be_transparent
}

/// The alpha of a fading thing one step of `seconds` after `now`, toward
/// `target`. With fading off it is at the target at once.
pub fn faded(now: f32, target: f32, seconds: f32, fading: bool) -> f32 {
    if !fading {
        return target;
    }
    let step = FADE_PER_SECOND * seconds;
    if now > target {
        (now - step).max(target)
    } else {
        (now + step).min(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> WorldLook {
        WorldLook::of(&Profile::default())
    }

    #[test]
    fn a_hidden_layer_leaves_the_figure_and_the_rest_stays() {
        use crate::view::WatchEquip;
        const CLOAK: u8 = 0x14;
        const HELMET: u8 = 0x06;
        let worn = |layer: u8| WatchEquip {
            layer,
            ..WatchEquip::default()
        };
        let mobile = WatchLook {
            equipment: vec![worn(CLOAK), worn(HELMET)],
            ..WatchLook::default()
        };
        let mut general = look().general;
        general.set_layer_hidden(CLOAK, true);
        assert_eq!(without_hidden_layers(&general, &mobile, true), None);
        general.hidden_layers_enabled = true;
        let shown = without_hidden_layers(&general, &mobile, true).unwrap();
        assert_eq!(shown.equipment, [worn(HELMET)]);
        assert_eq!(without_hidden_layers(&general, &mobile, false), None);
    }

    #[test]
    fn the_mouse_the_range_and_death_come_before_poison() {
        let mut look = look();
        look.general.highlight_objects = true;
        look.general.out_of_range_no_color = true;
        let poisoned = MobileState {
            poisoned: true,
            person: true,
            ..MobileState::default()
        };
        assert_eq!(
            mobile_hue(&look, false, poisoned),
            Some(look.general.poisoned_hue)
        );
        let hovered = MobileState {
            hovered: true,
            ..poisoned
        };
        assert_eq!(mobile_hue(&look, false, hovered), Some(HIGHLIGHT_HUE));
        let far = MobileState {
            out_of_range: true,
            ..poisoned
        };
        assert_eq!(mobile_hue(&look, false, far), Some(OUT_OF_RANGE_HUE));
        assert_eq!(mobile_hue(&look, true, poisoned), Some(DEAD_WORLD_HUE));
        look.general.highlight_poisoned = false;
        assert_eq!(mobile_hue(&look, false, poisoned), None);
    }

    #[test]
    fn the_foe_shows_his_notoriety_and_the_invulnerable_keep_their_look() {
        let look = look();
        let foe = MobileState {
            marked: true,
            hovered: true,
            notoriety: NOTO_MURDERER,
            ..MobileState::default()
        };
        assert_eq!(
            mobile_hue(&look, false, foe),
            Some(look.combat.murderer_hue)
        );
        let guard = MobileState {
            paralyzed: true,
            yellow_hits: true,
            notoriety: NOTO_INVULNERABLE,
            ..MobileState::default()
        };
        assert_eq!(mobile_hue(&look, false, guard), None);
        let dead_dog = MobileState {
            dead: true,
            ..MobileState::default()
        };
        assert_eq!(mobile_hue(&look, false, dead_dog), Some(DEAD_CREATURE_HUE));
    }

    #[test]
    fn the_aura_follows_its_rule() {
        assert!(!aura_shows(AuraRule::Never, true, true));
        assert!(aura_shows(AuraRule::WarMode, true, false));
        assert!(!aura_shows(AuraRule::WarMode, false, true));
        assert!(aura_shows(AuraRule::CtrlShift, false, true));
        assert!(aura_shows(AuraRule::Always, false, false));
    }

    #[test]
    fn hit_points_show_by_style_and_by_when() {
        let mut general = GeneralOptions {
            show_mobile_hp: true,
            mobile_hp_style: HpStyle::Both,
            ..GeneralOptions::default()
        };
        let both = HitsShown {
            percent: true,
            line: true,
        };
        assert_eq!(hits_shown(&general, Some(40), false, false), both);
        general.mobile_hp_when = HpShowWhen::BelowFull;
        assert_eq!(
            hits_shown(&general, Some(100), false, false),
            HitsShown::default()
        );
        general.mobile_hp_when = HpShowWhen::Smart;
        let smart_full = hits_shown(&general, Some(100), false, false);
        assert!(smart_full.line && !smart_full.percent);
        general.show_mobile_hp = false;
        assert!(hits_shown(&general, Some(40), true, false).line);
        assert_eq!(
            hits_shown(&general, None, true, false),
            HitsShown::default()
        );
        assert_eq!(hits_hue(10), HITS_LOW_HUE);
        assert_eq!(hits_hue(100), HITS_FULL_HUE);
    }

    #[test]
    fn plates_show_while_on_or_while_ctrl_and_shift_are_held() {
        let mut plates = NameplateOptions::default();
        assert!(!plate_shows(&plates, PlateOf::Mobile, false, false));
        assert!(plate_shows(&plates, PlateOf::Mobile, true, false));
        plates.enabled = true;
        plates.filter = NameplateFilter::MobilesAndCorpses;
        assert!(plate_shows(&plates, PlateOf::Corpse, false, false));
        assert!(!plate_shows(&plates, PlateOf::Item, false, false));
        plates.hide_at_full_health = true;
        assert!(!plate_shows(&plates, PlateOf::Mobile, false, true));
    }

    #[test]
    fn the_circle_hides_its_middle_and_fades_at_its_edge() {
        assert_eq!(circle_alpha(CircleStyle::Full, 0.5), 0.0);
        let edge = circle_alpha(CircleStyle::Full, 0.95);
        assert!(edge > 0.0 && edge < 1.0);
        assert_eq!(circle_alpha(CircleStyle::Full, 1.5), 1.0);
        assert_eq!(circle_alpha(CircleStyle::Gradient, 0.5), 0.125);
    }

    #[test]
    fn walls_over_the_head_may_be_seen_through_and_the_floor_may_not() {
        const OWN_Z: i8 = 0;
        const WALL_HEIGHT: u8 = 20;
        assert!(see_through(0, WALL_HEIGHT, false, false, true, OWN_Z));
        // A rug at the feet stays.
        assert!(!see_through(0, 0, true, true, false, OWN_Z));
        // A small box high over the head stays.
        assert!(!see_through(30, 2, false, false, false, OWN_Z));
    }

    #[test]
    fn a_thing_fades_in_steps_or_at_once() {
        let half = faded(1.0, 0.0, 0.1, true);
        assert!((0.0..1.0).contains(&half));
        assert_eq!(faded(1.0, 0.0, 10.0, true), 0.0);
        assert_eq!(faded(1.0, 0.0, 0.0, false), 0.0);
        assert_eq!(faded(0.0, TRANSLUCENT_ALPHA, 10.0, true), TRANSLUCENT_ALPHA);
    }
}
