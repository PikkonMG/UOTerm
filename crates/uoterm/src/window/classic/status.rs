//! The status gump of the character, as the reference client draws it in
//! both its looks, and the small health bar it becomes: the
//! name, the stats with their lock arrows, hit points, mana, stamina, gold,
//! weight, and on the modern gump the stat cap, luck, damage, followers,
//! the resists and, from the gump art of the UOP client on, the combat and
//! casting properties. The General option "Old status gump" picks the old
//! look. The corner button opens the health bar; with "Status and bar are
//! exclusive" on, the status closes then, and a double click on the bar
//! brings it back. A click on either while the shard waits for a target
//! targets the character.

use super::canvas::{ButtonArt, Canvas};
use super::health_bar::{HealthBar, BAR_RULES};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::{top_bar, viewport};
use crate::view::WatchFrame;
use crate::window::control::Act;
use crate::window::model::status::{damage_words, of, stats, StatLocks};
use crate::window::settings::{DEFAULT_GAME_WINDOW_WIDTH, DEFAULT_GAME_WINDOW_X};
use eframe::egui::{Color32, Pos2, Vec2};
use uoterm_nav::TextAlign;

/// The room between the status gump and what it first stands beside.
const FIRST_PLACE_GAP: f32 = 10.0;
/// The status first opens right of the game window as it first stands,
/// under the menu bar, so it lies on neither the world nor the
/// containers, which open over the game window.
const FIRST_PLACE: Pos2 = Pos2::new(
    DEFAULT_GAME_WINDOW_X
        + DEFAULT_GAME_WINDOW_WIDTH.max(viewport::LEAST_SIZE.x)
        + viewport::BORDER
        + FIRST_PLACE_GAP,
    top_bar::STRIP_HEIGHT as f32 + FIRST_PLACE_GAP,
);

pub const STATUS: GumpKind = GumpKind {
    id: well_known::STATUS,
    rules: GumpRules {
        first_place: FIRST_PLACE,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(Status::default()),
};

/// The health bar of the character, as the health bar gumps draw it.
pub const SELF_BAR: GumpKind = GumpKind {
    id: well_known::SELF_BAR,
    rules: BAR_RULES,
    open: |_| Box::new(HealthBar::own()),
};

const OLD_BACKGROUND: u16 = 0x0802;
const MODERN_BACKGROUND: u16 = 0x2A6C;
const LOCK_UP: u16 = 0x0984;
const LOCK_DOWN: u16 = 0x0986;
const LOCK_LOCKED: u16 = 0x082C;
const LOCK_ART: [u16; 3] = [LOCK_UP, LOCK_DOWN, LOCK_LOCKED];
/// The lock arrows sit here on the gump art of the UOP client, and at the
/// `lock_x` of the look on the older art.
const UOP_LOCK_X: i32 = 28;
const BUFF_BUTTON: ButtonArt = ButtonArt::new(0x7538, 0x7539, 0x7539);
const LABEL_FONT: u8 = 1;
const LABEL_HUE: u16 = 0x0386;
const FEMALE: &str = "Female";
const MALE: &str = "Male";
/// The dark lines between current and most values on the modern gump.
const RULE_COLOR: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);
const RULE_HEIGHT: i32 = 1;
/// The corner button that opens the health bar is this big.
const MINIMIZE_SIDE: i32 = 16;
const TIP_MINIMIZE: &str = "Minimize";
const TIP_OPEN_BAR: &str = "Open health bar";
const CENTERED_WIDTH: u32 = 40;
const NAME_WIDTH: u32 = 320;
const NAME_Y: i32 = 50;

/// Where one label goes, and the width and alignment of its box.
#[derive(Clone, Copy)]
struct Spot {
    x: i32,
    y: i32,
    width: u32,
    align: TextAlign,
}

const fn at(x: i32, y: i32) -> Spot {
    Spot {
        x,
        y,
        width: 0,
        align: TextAlign::Left,
    }
}

/// A centered label of the modern gump, forty pixels wide.
const fn centered(x: i32, y: i32) -> Spot {
    Spot {
        x,
        y,
        width: CENTERED_WIDTH,
        align: TextAlign::Center,
    }
}

/// The name of the modern gump, centered over the stats.
const fn name_spot(x: i32) -> Spot {
    Spot {
        x,
        y: NAME_Y,
        width: NAME_WIDTH,
        align: TextAlign::Center,
    }
}

/// A box with a tooltip: its place and size, and the text number and the
/// words the tooltip shows.
type Tip = (i32, i32, i32, i32, u32, &'static str);

/// The three looks of the status gump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Old,
    Modern,
    /// The modern gump on the gump art of the UOP client, with the combat
    /// and casting properties.
    ModernUop,
}

/// The parts of a look that are not labels.
struct Look {
    background: u16,
    lock_x: i32,
    lock_rows: [i32; 3],
    /// The dark rules between current and most values: left, top, width.
    rules: &'static [(i32, i32, i32)],
    tips: &'static [Tip],
    /// The corner that opens the health bar.
    minimize: (i32, i32),
    /// The button that opens the buff window.
    buffs: (i32, i32),
}

const OLD_TIPS: [Tip; 10] = [
    (86, 61, 34, 12, 3_000_077, "Strength"),
    (86, 73, 34, 12, 3_000_078, "Dex"),
    (86, 85, 34, 12, 3_000_079, "Intelligence"),
    (86, 97, 34, 12, 3_000_076, "Sex"),
    (86, 109, 34, 12, 1_062_760, "Armor"),
    (171, 61, 66, 12, 3_000_080, "Hits"),
    (171, 73, 66, 12, 1_061_151, "Mana"),
    (171, 85, 66, 12, 1_061_150, "Stamina"),
    (171, 97, 66, 12, 1_061_156, "Gold"),
    (171, 109, 66, 12, 1_061_154, "Weight"),
];

const OLD: Look = Look {
    background: OLD_BACKGROUND,
    lock_x: 40,
    lock_rows: [62, 74, 86],
    rules: &[],
    tips: &OLD_TIPS,
    minimize: (244, 112),
    buffs: (20, 42),
};

const MODERN_TIPS: [Tip; 17] = [
    (58, 70, 59, 24, 1_061_146, "Strength"),
    (58, 98, 59, 24, 1_061_147, "Dexterity"),
    (58, 126, 59, 24, 1_061_148, "Intelligence"),
    (124, 70, 59, 24, 1_061_149, "Hit Points"),
    (124, 98, 59, 24, 1_061_150, "Stamina"),
    (124, 126, 59, 24, 1_061_151, "Mana"),
    (188, 70, 65, 24, 1_061_152, "Maximum Stats"),
    (188, 98, 65, 24, 1_061_153, "Luck"),
    (188, 126, 65, 24, 1_061_154, "Weight"),
    (260, 98, 69, 24, 1_061_156, "Gold"),
    (260, 70, 69, 24, 1_061_155, "Damage"),
    (260, 126, 69, 24, 1_061_157, "Followers"),
    (334, 76, 40, 14, 1_061_158, "Physical Resistance"),
    (334, 92, 40, 14, 1_061_159, "Fire Resistance"),
    (334, 106, 40, 14, 1_061_160, "Cold Resistance"),
    (334, 120, 40, 14, 1_061_161, "Poison Resistance"),
    (334, 134, 40, 14, 1_061_162, "Energy Resistance"),
];

const MODERN: Look = Look {
    background: MODERN_BACKGROUND,
    lock_x: 40,
    lock_rows: [76, 102, 132],
    rules: &[
        (146, 138, 39),
        (146, 110, 39),
        (146, 82, 39),
        (216, 138, 34),
    ],
    tips: &MODERN_TIPS,
    minimize: (389, 152),
    buffs: (40, 50),
};

const MODERN_UOP_TIPS: [Tip; 26] = [
    (58, 154, 59, 24, 1_075_616, "Hit Chance Increase"),
    (58, 70, 59, 24, 1_061_146, "Strength"),
    (58, 98, 59, 24, 1_061_147, "Dexterity"),
    (58, 126, 59, 24, 1_061_148, "Intelligence"),
    (124, 154, 59, 24, 1_075_620, "Defense Chance Increase"),
    (124, 70, 59, 24, 1_061_149, "Hit Points"),
    (124, 98, 59, 24, 1_061_150, "Stamina"),
    (124, 126, 59, 24, 1_061_151, "Mana"),
    (205, 154, 65, 24, 1_075_621, "Lower Mana Cost"),
    (205, 70, 65, 24, 1_061_152, "Maximum Stats"),
    (205, 98, 65, 24, 1_061_153, "Luck"),
    (205, 126, 65, 24, 1_061_154, "Weight"),
    (285, 98, 69, 24, 1_075_619, "Weapon Damage Increase"),
    (285, 154, 69, 24, 1_075_629, "Swing Speed Increase"),
    (285, 70, 69, 24, 1_061_155, "Damage"),
    (285, 126, 69, 24, 1_061_157, "Followers"),
    (365, 70, 55, 24, 1_075_625, "Lower Reagent Cost"),
    (365, 98, 55, 24, 1_075_628, "Spell Damage Increase"),
    (365, 126, 55, 24, 1_075_617, "Faster Casting"),
    (365, 154, 55, 24, 1_075_618, "Faster Cast Recovery"),
    (445, 154, 55, 24, 1_061_156, "Gold"),
    (445, 76, 40, 14, 1_061_158, "Physical Resistance"),
    (445, 92, 40, 14, 1_061_159, "Fire Resistance"),
    (445, 106, 40, 14, 1_061_160, "Cold Resistance"),
    (445, 120, 40, 14, 1_061_161, "Poison Resistance"),
    (445, 134, 40, 14, 1_061_162, "Energy Resistance"),
];

const MODERN_UOP: Look = Look {
    background: MODERN_BACKGROUND,
    lock_x: UOP_LOCK_X,
    lock_rows: [76, 102, 132],
    rules: &[
        (150, 138, 35),
        (150, 110, 35),
        (150, 82, 35),
        (236, 138, 34),
    ],
    tips: &MODERN_UOP_TIPS,
    minimize: (540, 180),
    buffs: (40, 50),
};

impl Kind {
    fn look(self) -> &'static Look {
        match self {
            Kind::Old => &OLD,
            Kind::Modern => &MODERN,
            Kind::ModernUop => &MODERN_UOP,
        }
    }
}

/// Every label of a look, with its words, as the reference client places them.
fn labels(kind: Kind, frame: &WatchFrame) -> Vec<(Spot, String)> {
    let s = &frame.status;
    let stats = stats(frame);
    let damage = damage_words(frame);
    match kind {
        Kind::Old => vec![
            (at(86, 42), frame.name.clone()),
            (at(86, 62), stats[0].to_string()),
            (at(86, 74), stats[1].to_string()),
            (at(86, 86), stats[2].to_string()),
            (
                at(86, 98),
                if frame.female { FEMALE } else { MALE }.to_string(),
            ),
            (at(86, 110), s.physical_resist.to_string()),
            (at(171, 62), of(frame.hits, frame.hits_max)),
            (at(171, 74), of(frame.mana, frame.mana_max)),
            (at(171, 86), of(frame.stam, frame.stam_max)),
            (at(171, 98), frame.gold.to_string()),
            (at(171, 110), of(frame.weight, frame.weight_max)),
        ],
        Kind::Modern => vec![
            (name_spot(58), frame.name.clone()),
            (at(88, 77), stats[0].to_string()),
            (at(88, 105), stats[1].to_string()),
            (at(88, 133), stats[2].to_string()),
            (centered(141, 70), frame.hits.to_string()),
            (centered(141, 83), frame.hits_max.to_string()),
            (centered(141, 98), frame.stam.to_string()),
            (centered(141, 111), frame.stam_max.to_string()),
            (centered(141, 126), frame.mana.to_string()),
            (centered(141, 139), frame.mana_max.to_string()),
            (at(220, 77), s.stat_cap.to_string()),
            (at(220, 105), s.luck.to_string()),
            (centered(210, 126), frame.weight.to_string()),
            (centered(210, 139), frame.weight_max.to_string()),
            (at(280, 105), frame.gold.to_string()),
            (at(280, 77), damage),
            (at(280, 133), of(s.followers, s.followers_max)),
            (at(354, 76), s.physical_resist.to_string()),
            (at(354, 92), s.fire_resist.to_string()),
            (at(354, 106), s.cold_resist.to_string()),
            (at(354, 120), s.poison_resist.to_string()),
            (at(354, 134), s.energy_resist.to_string()),
        ],
        Kind::ModernUop => vec![
            (name_spot(90), frame.name.clone()),
            (at(80, 161), s.hit_chance_increase.to_string()),
            (at(80, 77), stats[0].to_string()),
            (at(80, 105), stats[1].to_string()),
            (at(80, 133), stats[2].to_string()),
            (
                at(150, 161),
                of(s.defense_chance_increase, s.max_defense_chance_increase),
            ),
            (centered(145, 70), frame.hits.to_string()),
            (centered(145, 83), frame.hits_max.to_string()),
            (centered(145, 98), frame.stam.to_string()),
            (centered(145, 111), frame.stam_max.to_string()),
            (centered(145, 126), frame.mana.to_string()),
            (centered(145, 139), frame.mana_max.to_string()),
            (at(240, 162), s.lower_mana_cost.to_string()),
            (at(240, 77), s.stat_cap.to_string()),
            (at(240, 105), s.luck.to_string()),
            (centered(230, 126), frame.weight.to_string()),
            (centered(230, 139), frame.weight_max.to_string()),
            (at(320, 105), s.damage_increase.to_string()),
            (at(320, 161), s.swing_speed_increase.to_string()),
            (at(320, 77), damage),
            (at(320, 133), of(s.followers, s.followers_max)),
            (at(400, 77), s.lower_reagent_cost.to_string()),
            (at(400, 105), s.spell_damage_increase.to_string()),
            (at(400, 133), s.faster_casting.to_string()),
            (at(400, 161), s.faster_cast_recovery.to_string()),
            (at(480, 161), frame.gold.to_string()),
            (at(475, 74), of(s.physical_resist, s.max_physical_resist)),
            (at(475, 92), of(s.fire_resist, s.max_fire_resist)),
            (at(475, 106), of(s.cold_resist, s.max_cold_resist)),
            (at(475, 120), of(s.poison_resist, s.max_poison_resist)),
            (at(475, 134), of(s.energy_resist, s.max_energy_resist)),
        ],
    }
}

/// The status gump.
#[derive(Default)]
pub struct Status {
    locks: StatLocks,
}

/// Clicks on a gump of the character while the shard waits for a target
/// target the character.
fn target_self(g: &Canvas<'_>, cx: &GumpContext<'_>) -> bool {
    if g.body_click().is_some() && cx.frame.target_cursor {
        cx.act(Act::Target(cx.frame.serial));
        return true;
    }
    false
}

fn label(g: &mut Canvas<'_>, spot: Spot, words: &str) {
    let look = TextLook::ascii(LABEL_FONT, LABEL_HUE).aligned(spot.align);
    let look = if spot.width > 0 {
        look.wrap(spot.width)
    } else {
        look
    };
    g.label(spot.x, spot.y, words, &look);
}

impl Status {
    /// The lock arrows: the lock the shard told, or the one the player just
    /// asked for. A click turns up to down to locked and asks the shard.
    fn locks(&mut self, g: &mut Canvas<'_>, cx: &GumpContext<'_>, rows: [i32; 3], lock_x: i32) {
        for (index, row) in rows.into_iter().enumerate() {
            let art = LOCK_ART[usize::from(self.locks.shown(cx.frame, index))];
            let clicked = g.button(("lock", index), lock_x, row, ButtonArt::new(art, art, art));
            if clicked && cx.live() {
                cx.act(Act::Command(self.locks.turn(cx.frame, index)));
            }
        }
    }
}

impl GumpBody for Status {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let kind = if cx.profile.general.old_status_gump {
            Kind::Old
        } else if g.text.uop_gumps {
            Kind::ModernUop
        } else {
            Kind::Modern
        };
        let look = kind.look();
        let size = g.pic(0, 0, look.background, 0);
        for (spot, words) in labels(kind, cx.frame) {
            label(g, spot, &words);
        }
        if cx.has_kind(well_known::BUFFS)
            && g.button("buffs", look.buffs.0, look.buffs.1, BUFF_BUTTON)
        {
            cx.open(GumpId::one(well_known::BUFFS));
        }
        let lock_x = if g.text.uop_gumps {
            UOP_LOCK_X
        } else {
            look.lock_x
        };
        self.locks(g, cx, look.lock_rows, lock_x);
        for (x, y, width) in look.rules {
            g.fill(*x, *y, *width, RULE_HEIGHT, RULE_COLOR);
        }
        for (x, y, w, h, number, fallback) in look.tips {
            // The words are read only for the box under the pointer.
            if g.hovered(*x, *y, *w, *h) {
                let words = g.words(*number, fallback);
                g.tooltip_at(*x, *y, *w, *h, &words);
            }
        }
        let exclusive = cx.profile.general.status_and_bar_exclusive;
        let (corner_x, corner_y) = look.minimize;
        if kind != Kind::Old {
            let tip = if exclusive {
                TIP_MINIMIZE
            } else {
                TIP_OPEN_BAR
            };
            g.tooltip_at(corner_x, corner_y, MINIMIZE_SIDE, MINIMIZE_SIDE, tip);
        }
        if target_self(g, cx) {
            return;
        }
        let corner_hit = g.body_click().is_some_and(|click| {
            let far = size + Vec2::splat(MINIMIZE_SIDE as f32);
            click.x >= corner_x as f32
                && click.y >= corner_y as f32
                && click.x <= far.x
                && click.y <= far.y
        });
        if corner_hit {
            cx.open_at(GumpId::one(well_known::SELF_BAR), g.at(0, 0));
            if exclusive {
                cx.close(cx.me);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words_at(labels: &[(Spot, String)], x: i32, y: i32) -> &str {
        labels
            .iter()
            .find(|(spot, _)| spot.x == x && spot.y == y)
            .map(|(_, words)| words.as_str())
            .unwrap()
    }

    #[test]
    fn the_status_first_opens_beside_the_game_window_under_the_menu_bar() {
        use crate::view::{WINDOW_HEIGHT, WINDOW_WIDTH};
        use crate::window::settings::VideoOptions;
        let screen =
            eframe::egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let game_window =
            viewport::view_rect(&VideoOptions::default(), screen).expand(viewport::BORDER);
        assert!(FIRST_PLACE.x > game_window.right());
        assert!(FIRST_PLACE.y > top_bar::STRIP_HEIGHT as f32);
        assert_eq!(STATUS.rules.first_place, FIRST_PLACE);
    }

    #[test]
    fn each_look_shows_the_status_where_the_classic_client_does() {
        let mut frame = WatchFrame {
            name: "Mara".into(),
            hits: 50,
            hits_max: 60,
            gold: 1234,
            female: true,
            ..WatchFrame::default()
        };
        frame.status.luck = 250;
        frame.status.physical_resist = 40;
        frame.status.max_physical_resist = 70;
        frame.status.followers = 1;
        frame.status.followers_max = 5;
        let old = labels(Kind::Old, &frame);
        assert_eq!(words_at(&old, 171, 62), "50/60");
        assert_eq!(words_at(&old, 86, 98), FEMALE);
        assert_eq!(words_at(&old, 86, 110), "40");
        let modern = labels(Kind::Modern, &frame);
        assert_eq!(words_at(&modern, 141, 70), "50");
        assert_eq!(words_at(&modern, 220, 105), "250");
        assert_eq!(words_at(&modern, 280, 133), "1/5");
        let uop = labels(Kind::ModernUop, &frame);
        assert_eq!(words_at(&uop, 475, 74), "40/70");
        assert_eq!(words_at(&uop, 480, 161), "1234");
        assert_eq!(words_at(&uop, 90, 50), "Mara");
    }
}
