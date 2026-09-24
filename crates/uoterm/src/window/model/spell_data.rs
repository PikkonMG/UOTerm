//! What a spellbook shows of each spell, as the spell tables and the
//! spellbook of the reference client have it: the picture of each
//! book and of its icons, the icon and the reagents or the mana, skill and
//! tithing each spell asks for, the names of the circles and of the
//! masteries, and the text numbers of the tooltips. The names and the
//! words of power come from the spell table of `uoterm_assist`; only the
//! names the book writes otherwise are kept here. The Classic spellbook
//! gump and the Modern spell tab both read it. No drawing.

use crate::view::{WatchFrame, WatchSpellbook};
use crate::window::actions::ActionId;
use crate::window::settings::MacroStep;
use std::sync::OnceLock;
use uoterm_assist::spells::{School, Spell, SpellBook};

/// A reagent a spell uses up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reagent {
    BlackPearl,
    Bloodmoss,
    Garlic,
    Ginseng,
    MandrakeRoot,
    Nightshade,
    SulfurousAsh,
    SpidersSilk,
    BatWing,
    GraveDust,
    DaemonBlood,
    NoxCrystal,
    PigIron,
    Bone,
    FertileDirt,
    DragonsBlood,
    DemonBone,
}

impl Reagent {
    /// The name of the reagent as the book writes it.
    pub fn words(self) -> &'static str {
        match self {
            Reagent::BlackPearl => "Black Pearl",
            Reagent::Bloodmoss => "Bloodmoss",
            Reagent::Garlic => "Garlic",
            Reagent::Ginseng => "Ginseng",
            Reagent::MandrakeRoot => "Mandrake Root",
            Reagent::Nightshade => "Nightshade",
            Reagent::SulfurousAsh => "Sulfurous Ash",
            Reagent::SpidersSilk => "Spiders Silk",
            Reagent::BatWing => "Bat Wing",
            Reagent::GraveDust => "Grave Dust",
            Reagent::DaemonBlood => "Daemon Blood",
            Reagent::NoxCrystal => "Nox Crystal",
            Reagent::PigIron => "Pig Iron",
            Reagent::Bone => "Bone",
            Reagent::FertileDirt => "Fertile Dirt",
            Reagent::DragonsBlood => "Dragons Blood",
            Reagent::DemonBone => "Demon Bone",
        }
    }
}

/// What the book shows of one spell, apart from its name and words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookSpell {
    pub id: u16,
    /// The small icon, which a spell button on the desktop shows. A book
    /// of masteries shows it too.
    pub small_icon: u16,
    pub mana: u8,
    /// The least skill, in whole points.
    pub skill: u8,
    /// The upkeep in tithing points of a mastery, and the tithing points a
    /// chivalry spell costs.
    pub tithing: u8,
    pub reagents: &'static [Reagent],
}

const fn spell(
    id: u16,
    small_icon: u16,
    mana: u8,
    skill: u8,
    tithing: u8,
    reagents: &'static [Reagent],
) -> BookSpell {
    BookSpell {
        id,
        small_icon,
        mana,
        skill,
        tithing,
        reagents,
    }
}

use Reagent::*;

#[rustfmt::skip]
const MAGERY: &[BookSpell] = &[
    spell(1, 0x08C0, 0, 0, 0, &[Bloodmoss, Nightshade]),
    spell(2, 0x08C1, 0, 0, 0, &[Garlic, Ginseng, MandrakeRoot]),
    spell(3, 0x08C2, 0, 0, 0, &[Nightshade, Ginseng]),
    spell(4, 0x08C3, 0, 0, 0, &[Garlic, Ginseng, SpidersSilk]),
    spell(5, 0x08C4, 0, 0, 0, &[SulfurousAsh]),
    spell(6, 0x08C5, 0, 0, 0, &[SpidersSilk, SulfurousAsh]),
    spell(7, 0x08C6, 0, 0, 0, &[Garlic, SpidersSilk, SulfurousAsh]),
    spell(8, 0x08C7, 0, 0, 0, &[Garlic, Nightshade]),
    spell(9, 0x08C8, 0, 0, 0, &[Bloodmoss, MandrakeRoot]),
    spell(10, 0x08C9, 0, 0, 0, &[Nightshade, MandrakeRoot]),
    spell(11, 0x08CA, 0, 0, 0, &[Garlic, Ginseng]),
    spell(12, 0x08CB, 0, 0, 0, &[Nightshade, SpidersSilk]),
    spell(13, 0x08CC, 0, 0, 0, &[Garlic, SpidersSilk, SulfurousAsh]),
    spell(14, 0x08CD, 0, 0, 0, &[Bloodmoss, SulfurousAsh]),
    spell(15, 0x08CE, 0, 0, 0, &[Garlic, Ginseng, SulfurousAsh]),
    spell(16, 0x08CF, 0, 0, 0, &[MandrakeRoot, Nightshade]),
    spell(17, 0x08D0, 0, 0, 0, &[Garlic, MandrakeRoot]),
    spell(18, 0x08D1, 0, 0, 0, &[BlackPearl]),
    spell(19, 0x08D2, 0, 0, 0, &[Bloodmoss, Garlic, SulfurousAsh]),
    spell(20, 0x08D3, 0, 0, 0, &[Nightshade]),
    spell(21, 0x08D4, 0, 0, 0, &[Bloodmoss, MandrakeRoot]),
    spell(22, 0x08D5, 0, 0, 0, &[Bloodmoss, MandrakeRoot]),
    spell(23, 0x08D6, 0, 0, 0, &[Bloodmoss, SulfurousAsh]),
    spell(24, 0x08D7, 0, 0, 0, &[Bloodmoss, Garlic]),
    spell(25, 0x08D8, 0, 0, 0, &[Garlic, Ginseng, MandrakeRoot]),
    spell(26, 0x08D9, 0, 0, 0, &[Garlic, Ginseng, MandrakeRoot, SulfurousAsh]),
    spell(27, 0x08DA, 0, 0, 0, &[Garlic, Nightshade, SulfurousAsh]),
    spell(28, 0x08DB, 0, 0, 0, &[BlackPearl, SpidersSilk, SulfurousAsh]),
    spell(29, 0x08DC, 0, 0, 0, &[Garlic, Ginseng, MandrakeRoot, SpidersSilk]),
    spell(30, 0x08DD, 0, 0, 0, &[MandrakeRoot, SulfurousAsh]),
    spell(31, 0x08DE, 0, 0, 0, &[BlackPearl, MandrakeRoot, SpidersSilk]),
    spell(32, 0x08DF, 0, 0, 0, &[BlackPearl, Bloodmoss, MandrakeRoot]),
    spell(33, 0x08E0, 0, 0, 0, &[BlackPearl, MandrakeRoot, Nightshade]),
    spell(34, 0x08E1, 0, 0, 0, &[BlackPearl, Garlic, SpidersSilk, SulfurousAsh]),
    spell(35, 0x08E2, 0, 0, 0, &[Bloodmoss, Garlic, Nightshade]),
    spell(36, 0x08E3, 0, 0, 0, &[Garlic, MandrakeRoot, SpidersSilk]),
    spell(37, 0x08E4, 0, 0, 0, &[BlackPearl, MandrakeRoot, Nightshade, SulfurousAsh]),
    spell(38, 0x08E5, 0, 0, 0, &[Garlic, MandrakeRoot, SpidersSilk]),
    spell(39, 0x08E6, 0, 0, 0, &[BlackPearl, Nightshade, SpidersSilk]),
    spell(40, 0x08E7, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk]),
    spell(41, 0x08E8, 0, 0, 0, &[Garlic, MandrakeRoot, SulfurousAsh]),
    spell(42, 0x08E9, 0, 0, 0, &[BlackPearl, Nightshade]),
    spell(43, 0x08EA, 0, 0, 0, &[Bloodmoss, MandrakeRoot]),
    spell(44, 0x08EB, 0, 0, 0, &[Bloodmoss, Nightshade]),
    spell(45, 0x08EC, 0, 0, 0, &[BlackPearl, Bloodmoss, MandrakeRoot]),
    spell(46, 0x08ED, 0, 0, 0, &[Garlic, MandrakeRoot, Nightshade, SulfurousAsh]),
    spell(47, 0x08EE, 0, 0, 0, &[BlackPearl, Ginseng, SpidersSilk]),
    spell(48, 0x08EF, 0, 0, 0, &[Bloodmoss, SulfurousAsh]),
    spell(49, 0x08F0, 0, 0, 0, &[BlackPearl, Bloodmoss, MandrakeRoot, SulfurousAsh]),
    spell(50, 0x08F1, 0, 0, 0, &[BlackPearl, MandrakeRoot, SpidersSilk, SulfurousAsh]),
    spell(51, 0x08F2, 0, 0, 0, &[SpidersSilk, SulfurousAsh]),
    spell(52, 0x08F3, 0, 0, 0, &[BlackPearl, MandrakeRoot, SulfurousAsh]),
    spell(53, 0x08F4, 0, 0, 0, &[BlackPearl, Bloodmoss, MandrakeRoot, SpidersSilk]),
    spell(54, 0x08F5, 0, 0, 0, &[BlackPearl, Garlic, MandrakeRoot, SulfurousAsh]),
    spell(55, 0x08F6, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk, SulfurousAsh]),
    spell(56, 0x08F7, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk]),
    spell(57, 0x08F8, 0, 0, 0, &[Bloodmoss, Ginseng, MandrakeRoot, SulfurousAsh]),
    spell(58, 0x08F9, 0, 0, 0, &[BlackPearl, Bloodmoss, MandrakeRoot, Nightshade]),
    spell(59, 0x08FA, 0, 0, 0, &[Bloodmoss, Ginseng, Garlic]),
    spell(60, 0x08FB, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk]),
    spell(61, 0x08FC, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk, SulfurousAsh]),
    spell(62, 0x08FD, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk]),
    spell(63, 0x08FE, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk, SulfurousAsh]),
    spell(64, 0x08FF, 0, 0, 0, &[Bloodmoss, MandrakeRoot, SpidersSilk]),
];

#[rustfmt::skip]
const NECROMANCY: &[BookSpell] = &[
    spell(101, 0x5000, 23, 40, 0, &[DaemonBlood, GraveDust]),
    spell(102, 0x5001, 13, 20, 0, &[DaemonBlood]),
    spell(103, 0x5002, 11, 20, 0, &[BatWing, GraveDust]),
    spell(104, 0x5003, 7, 0, 0, &[PigIron]),
    spell(105, 0x5004, 11, 20, 0, &[BatWing, NoxCrystal]),
    spell(106, 0x5005, 11, 40, 0, &[BatWing, DaemonBlood]),
    spell(107, 0x5006, 25, 70, 0, &[DaemonBlood, GraveDust, NoxCrystal]),
    spell(108, 0x5007, 17, 30, 0, &[BatWing, DaemonBlood, PigIron]),
    spell(109, 0x5008, 5, 20, 0, &[GraveDust, PigIron]),
    spell(110, 0x5009, 17, 50, 0, &[NoxCrystal]),
    spell(111, 0x500A, 29, 65, 0, &[DaemonBlood, NoxCrystal]),
    spell(112, 0x500B, 17, 30, 0, &[BatWing, DaemonBlood, GraveDust]),
    spell(113, 0x500C, 25, 99, 0, &[BatWing, NoxCrystal, PigIron]),
    spell(114, 0x500D, 41, 80, 0, &[BatWing, GraveDust, PigIron]),
    spell(115, 0x500E, 23, 60, 0, &[GraveDust, NoxCrystal, PigIron]),
    spell(116, 0x500F, 17, 20, 0, &[NoxCrystal, PigIron]),
    spell(117, 0x5010, 40, 80, 0, &[NoxCrystal, GraveDust]),
];

#[rustfmt::skip]
const CHIVALRY: &[BookSpell] = &[
    spell(201, 0x5100, 10, 5, 10, &[]),
    spell(202, 0x5101, 10, 0, 10, &[]),
    spell(203, 0x5102, 10, 15, 10, &[]),
    spell(204, 0x5103, 10, 35, 10, &[]),
    spell(205, 0x5104, 10, 25, 10, &[]),
    spell(206, 0x5105, 20, 45, 10, &[]),
    spell(207, 0x5106, 20, 55, 10, &[]),
    spell(208, 0x5107, 20, 65, 30, &[]),
    spell(209, 0x5108, 20, 5, 10, &[]),
    spell(210, 0x5109, 20, 5, 10, &[]),
];

#[rustfmt::skip]
const BUSHIDO: &[BookSpell] = &[
    spell(401, 0x5420, 0, 25, 0, &[]),
    spell(402, 0x5421, 10, 25, 0, &[]),
    spell(403, 0x5422, 10, 60, 0, &[]),
    spell(404, 0x5423, 5, 40, 0, &[]),
    spell(405, 0x5424, 10, 50, 0, &[]),
    spell(406, 0x5425, 10, 70, 0, &[]),
];

#[rustfmt::skip]
const NINJITSU: &[BookSpell] = &[
    spell(501, 0x5320, 20, 60, 0, &[]),
    spell(502, 0x5321, 30, 85, 0, &[]),
    spell(503, 0x5322, 0, 10, 0, &[]),
    spell(504, 0x5323, 25, 80, 0, &[]),
    spell(505, 0x5324, 20, 30, 0, &[]),
    spell(506, 0x5325, 30, 20, 0, &[]),
    spell(507, 0x5326, 15, 50, 0, &[]),
    spell(508, 0x5327, 10, 40, 0, &[]),
];

#[rustfmt::skip]
const SPELLWEAVING: &[BookSpell] = &[
    spell(601, 0x59D8, 20, 0, 0, &[]),
    spell(602, 0x59D9, 24, 0, 0, &[]),
    spell(603, 0x59DA, 32, 10, 0, &[]),
    spell(604, 0x59DB, 24, 0, 0, &[]),
    spell(605, 0x59DC, 32, 10, 0, &[]),
    spell(606, 0x59DD, 24, 0, 0, &[]),
    spell(607, 0x59DE, 10, 38, 0, &[]),
    spell(608, 0x59DF, 10, 38, 0, &[]),
    spell(609, 0x59E0, 34, 24, 0, &[]),
    spell(610, 0x59E1, 50, 66, 0, &[]),
    spell(611, 0x59E2, 40, 52, 0, &[]),
    spell(612, 0x59E3, 40, 52, 0, &[]),
    spell(613, 0x59E4, 32, 24, 0, &[]),
    spell(614, 0x59E5, 50, 23, 0, &[]),
    spell(615, 0x59E6, 70, 38, 0, &[]),
    spell(616, 0x59E7, 50, 24, 0, &[]),
];

#[rustfmt::skip]
const MYSTICISM: &[BookSpell] = &[
    spell(678, 0x5DC0, 4, 0, 0, &[BlackPearl, SulfurousAsh]),
    spell(679, 0x5DC1, 4, 0, 0, &[Bone, Garlic, Ginseng, SpidersSilk]),
    spell(680, 0x5DC2, 6, 8, 0, &[FertileDirt, Garlic, MandrakeRoot, SulfurousAsh]),
    spell(681, 0x5DC3, 6, 8, 0, &[SpidersSilk, MandrakeRoot, SulfurousAsh]),
    spell(682, 0x5DC4, 8, 20, 0, &[Nightshade, SpidersSilk, BlackPearl]),
    spell(683, 0x5DC5, 9, 20, 0, &[Bloodmoss, Bone, MandrakeRoot, SpidersSilk]),
    spell(684, 0x5DC6, 11, 33, 0, &[Bone, BlackPearl, MandrakeRoot, Nightshade]),
    spell(685, 0x5DC7, 11, 33, 0, &[Bloodmoss, FertileDirt, Garlic]),
    spell(686, 0x5DC8, 14, 45, 0, &[DragonsBlood, Garlic, MandrakeRoot, SpidersSilk]),
    spell(687, 0x5DC9, 14, 45, 0, &[Ginseng, Nightshade, SpidersSilk]),
    spell(688, 0x5DCA, 20, 58, 0, &[DragonsBlood, Garlic, Ginseng, MandrakeRoot]),
    spell(689, 0x5DCB, 20, 58, 0, &[Bloodmoss, DragonsBlood, Garlic, SulfurousAsh]),
    spell(690, 0x5DCC, 40, 70, 0, &[DemonBone, DragonsBlood, Nightshade, SulfurousAsh]),
    spell(691, 0x5DCD, 50, 70, 0, &[DragonsBlood, BlackPearl, Bloodmoss, MandrakeRoot]),
    spell(692, 0x5DCE, 50, 83, 0, &[MandrakeRoot, Nightshade, SulfurousAsh, Bloodmoss]),
    spell(693, 0x5DCF, 50, 83, 0, &[DemonBone, DragonsBlood, FertileDirt, Nightshade]),
];

#[rustfmt::skip]
const MASTERY: &[BookSpell] = &[
    spell(701, 0x0945, 16, 90, 4, &[]),
    spell(702, 0x0946, 22, 90, 5, &[]),
    spell(703, 0x0947, 16, 90, 4, &[]),
    spell(704, 0x0948, 18, 90, 5, &[]),
    spell(705, 0x0949, 24, 90, 10, &[]),
    spell(706, 0x094A, 26, 90, 12, &[]),
    spell(707, 0x9B8B, 50, 90, 35, &[BlackPearl, Bloodmoss, SpidersSilk]),
    spell(708, 0x9B8C, 0, 90, 0, &[Bloodmoss, Ginseng, MandrakeRoot]),
    spell(709, 0x9B8D, 40, 90, 0, &[DragonsBlood, DemonBone]),
    spell(710, 0x9B8E, 40, 90, 0, &[FertileDirt, Bone]),
    spell(711, 0x9B8F, 40, 90, 0, &[DaemonBlood, PigIron, BatWing]),
    spell(712, 0x9B90, 40, 90, 0, &[NoxCrystal, BatWing, GraveDust]),
    spell(713, 0x9B91, 40, 90, 0, &[]),
    spell(714, 0x9B92, 50, 90, 0, &[]),
    spell(715, 0x9B93, 0, 90, 0, &[]),
    spell(716, 0x9B94, 10, 90, 0, &[]),
    spell(717, 0x9B95, 40, 90, 0, &[]),
    spell(718, 0x9B96, 0, 90, 0, &[]),
    spell(719, 0x9B97, 10, 90, 35, &[]),
    spell(720, 0x9B98, 50, 90, 35, &[]),
    spell(721, 0x9B99, 10, 90, 4, &[]),
    spell(722, 0x9B9A, 10, 90, 0, &[]),
    spell(723, 0x9B9B, 30, 90, 0, &[]),
    spell(724, 0x9B9C, 25, 90, 0, &[]),
    spell(725, 0x9B9D, 30, 90, 20, &[]),
    spell(726, 0x9B9E, 20, 90, 0, &[]),
    spell(727, 0x9B9F, 20, 90, 0, &[]),
    spell(728, 0x9BA0, 20, 90, 20, &[]),
    spell(729, 0x9BA1, 20, 90, 0, &[]),
    spell(730, 0x9BA2, 20, 90, 20, &[]),
    spell(731, 0x9BA3, 20, 90, 0, &[]),
    spell(732, 0x9BA4, 40, 90, 0, &[]),
    spell(733, 0x9BA5, 50, 90, 0, &[]),
    spell(734, 0x9BA6, 50, 90, 0, &[]),
    spell(735, 0x9BA7, 40, 90, 0, &[]),
    spell(736, 0x9BA8, 10, 90, 10, &[]),
    spell(737, 0x9BA9, 20, 90, 0, &[]),
    spell(738, 0x9BAA, 30, 90, 0, &[]),
    spell(739, 0x9BAB, 0, 90, 0, &[]),
    spell(740, 0x9BAC, 20, 90, 0, &[]),
    spell(741, 0x9BAD, 20, 90, 0, &[]),
    spell(742, 0x9BAE, 0, 90, 0, &[]),
    spell(743, 0x9BAF, 40, 90, 0, &[]),
    spell(744, 0x9BB0, 40, 90, 0, &[]),
    spell(745, 0x9BB1, 0, 90, 0, &[]),
];

/// The pictures and the spells of one kind of book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookInfo {
    pub school: School,
    /// The name of the school in `watch` and in the open command.
    pub name: &'static str,
    /// The name of the school as a book title.
    pub title: &'static str,
    pub book: u16,
    pub minimized: u16,
    /// The icon of the first spell; the others follow it.
    pub first_icon: u16,
    /// The text number of the tooltip of the first spell; the others
    /// follow it. Zero for the masteries, which have their own.
    pub first_tooltip: u32,
    /// The item graphics of the books of this school.
    pub graphics: &'static [u16],
    pub spells: &'static [BookSpell],
}

impl BookInfo {
    /// How many spells a page of the index lists.
    pub fn spells_on_page(&self) -> usize {
        (self.spells.len() / HALF).min(MOST_ON_INDEX_PAGE)
    }

    /// How many pages the index takes, an even number.
    pub fn index_pages(&self) -> usize {
        let pages = self.spells.len().div_ceil(MOST_ON_INDEX_PAGE);
        pages + pages % HALF
    }

    /// The spell at a place of the book, from zero.
    pub fn spell_at(&self, index: usize) -> Option<&'static BookSpell> {
        self.spells.get(index)
    }

    /// The icon a page of the book shows for the spell at `index`.
    pub fn icon(&self, index: usize) -> u16 {
        match self.school {
            School::Mastery => self.spells.get(index).map_or(0, |spell| spell.small_icon),
            _ => self.first_icon + index as u16,
        }
    }

    /// The text number of the tooltip of the icon of the spell at `index`.
    pub fn tooltip(&self, index: usize) -> Option<u32> {
        let index = index as u32;
        match self.school {
            School::Mastery if index < FIRST_MASTERY_GROUP => Some(MASTERY_FIRST_TOOLTIP + index),
            School::Mastery => Some(MASTERY_LATER_TOOLTIP - FIRST_MASTERY_GROUP + index),
            _ => (self.first_tooltip != 0).then_some(self.first_tooltip + index),
        }
    }

    /// The places in the school, from zero and in order, of the spells a
    /// book holds. A book the shard has not filled holds none.
    pub fn held_places(&self, contents: Option<&WatchSpellbook>) -> Vec<usize> {
        let first = self.spells.first().map_or(0, |spell| spell.id);
        let mut held: Vec<usize> = contents
            .map(|contents| {
                contents
                    .spells
                    .iter()
                    .filter_map(|(number, _)| number.checked_sub(first).map(usize::from))
                    .filter(|place| *place < self.spells.len())
                    .collect()
            })
            .unwrap_or_default();
        held.sort_unstable();
        held.dedup();
        held
    }

    /// The circle of a spell of magery, or the group of a mastery, by its
    /// place in the school. The other schools have none.
    pub fn spell_group(&self, place: usize) -> Option<&'static str> {
        match self.school {
            School::Magery => CIRCLE_NAMES.get(place / SPELLS_PER_CIRCLE).copied(),
            School::Mastery => Some(mastery_group(place + 1)),
            _ => None,
        }
    }
}

/// A circle of magery holds this many spells.
pub const SPELLS_PER_CIRCLE: usize = 8;

/// What a book of mana and skill asks for a spell, in words.
pub fn needs_words(mana: u8, skill: u8, upkeep: u8) -> String {
    if upkeep > 0 {
        format!("Upkeep Cost: {upkeep}\nMana cost: {mana}\nMin. Skill: {skill}")
    } else {
        format!("Mana cost: {mana}\nMin. Skill: {skill}")
    }
}

/// The upkeep in tithing points a mastery asks for; other spells have none.
pub fn upkeep(book: &BookInfo, spell: &BookSpell) -> u8 {
    if book.school == School::Mastery {
        spell.tithing
    } else {
        0
    }
}

/// The name and the steps of the macro that casts a spell, as "Fast spell
/// assign" makes it.
pub fn spell_macro(spell: u16) -> (String, Vec<MacroStep>) {
    let name = book_name(spell);
    let steps = vec![MacroStep::new(ActionId::CastSpell.spec().id, &name)];
    (name, steps)
}

const HALF: usize = 2;
const MOST_ON_INDEX_PAGE: usize = 8;
/// The first six masteries have tooltips of their own.
const FIRST_MASTERY_GROUP: u32 = 6;
const MASTERY_FIRST_TOOLTIP: u32 = 1_115_689;
const MASTERY_LATER_TOOLTIP: u32 = 1_155_938;

pub const BOOKS: [BookInfo; 8] = [
    BookInfo {
        school: School::Magery,
        name: "magery",
        title: "Magery",
        book: 0x08AC,
        minimized: 0x08BA,
        first_icon: 0x08C0,
        first_tooltip: 1_061_290,
        graphics: &[0x0EFA],
        spells: MAGERY,
    },
    BookInfo {
        school: School::Necromancy,
        name: "necromancy",
        title: "Necromancy",
        book: 0x2B00,
        minimized: 0x2B03,
        first_icon: 0x5000,
        first_tooltip: 1_061_390,
        graphics: &[0x2253],
        spells: NECROMANCY,
    },
    BookInfo {
        school: School::Chivalry,
        name: "chivalry",
        title: "Chivalry",
        book: 0x2B01,
        minimized: 0x2B04,
        first_icon: 0x5100,
        first_tooltip: 1_061_490,
        graphics: &[0x2252],
        spells: CHIVALRY,
    },
    BookInfo {
        school: School::Bushido,
        name: "bushido",
        title: "Bushido",
        book: 0x2B07,
        minimized: 0x2B09,
        first_icon: 0x5400,
        first_tooltip: 1_063_263,
        graphics: &[0x238C],
        spells: BUSHIDO,
    },
    BookInfo {
        school: School::Ninjitsu,
        name: "ninjitsu",
        title: "Ninjitsu",
        book: 0x2B06,
        minimized: 0x2B08,
        first_icon: 0x5300,
        first_tooltip: 1_063_279,
        graphics: &[0x23A0],
        spells: NINJITSU,
    },
    BookInfo {
        school: School::Spellweaving,
        name: "spellweaving",
        title: "Spellweaving",
        book: 0x2B2F,
        minimized: 0x2B2D,
        first_icon: 0x59D8,
        first_tooltip: 1_072_042,
        graphics: &[0x2D50],
        spells: SPELLWEAVING,
    },
    BookInfo {
        school: School::Mysticism,
        name: "mysticism",
        title: "Mysticism",
        book: 0x2B32,
        minimized: 0x2B30,
        first_icon: 0x5DC0,
        first_tooltip: 1_095_193,
        graphics: &[0x2D9D],
        spells: MYSTICISM,
    },
    BookInfo {
        school: School::Mastery,
        name: "mastery",
        title: "Masteries",
        book: 0x08AC,
        minimized: 0x08BA,
        first_icon: 0x0945,
        first_tooltip: 0,
        graphics: &[0x225A, 0x225B],
        spells: MASTERY,
    },
];

/// The kind of a book: by the school `watch` names, else by its graphic,
/// else a book of magery, as the classic client falls back.
pub fn book_info(school: &str, graphic: u16) -> &'static BookInfo {
    BOOKS
        .iter()
        .find(|book| book.name.eq_ignore_ascii_case(school))
        .or_else(|| BOOKS.iter().find(|book| book.graphics.contains(&graphic)))
        .unwrap_or(&BOOKS[0])
}

/// The kind of book of a school; a school the client has no book for has
/// none.
pub fn book_of(school: School) -> Option<&'static BookInfo> {
    BOOKS.iter().find(|book| book.school == school)
}

/// The book data of a spell number, of any school.
pub fn book_spell(id: u16) -> Option<(&'static BookInfo, &'static BookSpell)> {
    BOOKS.iter().find_map(|book| {
        book.spells
            .iter()
            .find(|spell| spell.id == id)
            .map(|spell| (book, spell))
    })
}

/// The names the book writes that differ from the spell table.
const BOOK_NAMES: [(u16, &str); 8] = [
    (51, "Flamestrike"),
    (60, "Air Elemental"),
    (62, "Earth Elemental"),
    (63, "Fire Elemental"),
    (64, "Water Elemental"),
    (708, "Ethereal Burst"),
    (733, "Warrior's Gifts"),
    (741, "Fists of Fury"),
];

/// The spell table, made once.
fn spell_table() -> &'static SpellBook {
    static TABLE: OnceLock<SpellBook> = OnceLock::new();
    TABLE.get_or_init(SpellBook::standard)
}

/// The spell of a number in the spell table.
pub fn known_spell(id: u16) -> Option<&'static Spell> {
    spell_table().by_id(id)
}

/// The spell whose words of power these are, in any case.
pub fn spell_by_words(words: &str) -> Option<&'static Spell> {
    let words = words.trim();
    spell_table()
        .iter()
        .find(|spell| !spell.words.is_empty() && spell.words.eq_ignore_ascii_case(words))
}

/// The name of a spell as the book writes it.
pub fn book_name(id: u16) -> String {
    BOOK_NAMES
        .iter()
        .find(|(number, _)| *number == id)
        .map(|(_, name)| name.to_string())
        .or_else(|| known_spell(id).map(|spell| spell.name.clone()))
        .unwrap_or_default()
}

/// The words of power of a spell.
pub fn power_words(id: u16) -> String {
    known_spell(id)
        .map(|spell| spell.words.clone())
        .unwrap_or_default()
}

/// The first letters of the words of power, as the index of a book of
/// magery shows them: "Uus Jux" is "U J".
pub fn words_letters(words: &str) -> String {
    words
        .chars()
        .filter(|c| c.is_uppercase() || *c == ' ')
        .collect()
}

/// The reagents of a spell, one on each line.
pub fn reagent_lines(spell: &BookSpell) -> String {
    spell
        .reagents
        .iter()
        .map(|reagent| reagent.words())
        .collect::<Vec<_>>()
        .join("\n")
}

pub const CIRCLE_NAMES: [&str; 8] = [
    "First Circle",
    "Second Circle",
    "Third Circle",
    "Fourth Circle",
    "Fifth Circle",
    "Sixth Circle",
    "Seventh Circle",
    "Eighth Circle",
];

/// The spells of each page of the index of a book of masteries, by their
/// place in the school from one.
pub const MASTERY_INDEX_PAGES: [&[usize]; 6] = [
    &[1, 2, 3, 4, 5, 6, 7, 8],
    &[9, 10, 11, 12, 13, 14, 19, 20],
    &[17, 21, 22, 25, 26, 34, 35, 36],
    &[27, 28, 29, 32, 37, 38, 40, 41],
    &[23, 24, 30, 31, 43, 44],
    &[15, 16, 18, 33, 39, 42, 45],
];

/// The group of a mastery by its place in the school from one.
pub fn mastery_group(place: usize) -> &'static str {
    match place {
        3 | 4 => "Peacemaking",
        5 | 6 => "Discordance",
        7 | 8 => "Magery",
        9 | 10 => "Mysticism",
        11 | 12 => "Necromancy",
        13 | 14 => "Spellweaving",
        16 | 17 => "Bushido",
        19 | 20 => "Chivalry",
        21 | 22 => "Ninjitsu",
        23 | 24 => "Archery",
        25 | 26 => "Fencing",
        27 | 28 => "Mace Fighting",
        29 | 30 => "Swordmanship",
        31 | 32 => "Throwing",
        34..=36 => "Parrying",
        37..=39 => "Poisoning",
        40..=42 => "Wrestling",
        43..=45 => "Animal Taming",
        15 | 18 | 33 => "Passive",
        _ => "Provocation",
    }
}

/// The property of a book of masteries that names its active mastery, and
/// the masteries the "Abilities" page shows for it, by their place.
const ACTIVE_MASTERIES: [(u32, &[usize]); 19] = [
    (0x1193CA, &[1, 2]),
    (0x1193CB, &[3, 4]),
    (0x1193C9, &[5, 6]),
    (0x11A2BB, &[15, 7, 8]),
    (0x11A2BC, &[15, 9, 10]),
    (0x11A2BD, &[15, 11, 12]),
    (0x11A2BE, &[15, 13, 14]),
    (0x11A2BF, &[18, 16, 17]),
    (0x11A2C0, &[18, 19, 20]),
    (0x11A2C1, &[18, 21, 22]),
    (0x11A2C2, &[33, 25, 26]),
    (0x11A2C3, &[33, 27, 28]),
    (0x11A2C4, &[33, 29, 30]),
    (0x11A2C5, &[33, 31, 32]),
    (0x11A2C6, &[36, 34, 35]),
    (0x11A2C7, &[39, 37, 38]),
    (0x11A2C8, &[42, 40, 41]),
    (0x11A2C9, &[45, 43, 44]),
    (0x11A2CA, &[33, 23, 24]),
];

/// The masteries the "Abilities" page shows, from the text numbers of the
/// properties of the book: `has` says whether the book has one.
pub fn active_masteries(has: impl Fn(u32) -> bool) -> &'static [usize] {
    ACTIVE_MASTERIES
        .iter()
        .find(|(cliloc, _)| has(*cliloc))
        .map_or(&[], |(_, places)| places)
}

/// The hue of the icon of a spell that stays on, as a stance.
pub const ACTIVE_SPELL_HUE: u16 = 38;

/// True when the shard says the spell stays on now.
pub fn is_active(frame: &WatchFrame, spell: u16) -> bool {
    frame
        .abilities
        .spells
        .iter()
        .any(|(number, _)| *number == spell)
}

/// The hue of the icon of a spell: red while it stays on.
pub fn icon_hue(frame: &WatchFrame, spell: u16) -> u16 {
    if is_active(frame, spell) {
        ACTIVE_SPELL_HUE
    } else {
        0
    }
}

/// The text number of the tooltip of a spell button.
pub fn button_tooltip(id: u16) -> Option<u32> {
    let id = u32::from(id);
    let from = |first_id: u32, first_cliloc: u32| first_cliloc + (id - first_id);
    Some(match id {
        1..=64 => from(1, 3_002_011),
        101..=117 => from(101, 1_060_509),
        201..=210 => from(201, 1_060_585),
        401..=406 => from(401, 1_060_595),
        501..=508 => from(501, 1_060_610),
        601..=616 => from(601, 1_071_026),
        678..=693 => from(678, 1_031_678),
        701..=706 => from(701, 1_115_612),
        707..=745 => from(707, 1_155_896),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_school_has_its_count_of_spells_in_order() {
        let counts: Vec<usize> = BOOKS.iter().map(|book| book.spells.len()).collect();
        assert_eq!(counts, vec![64, 17, 10, 6, 8, 16, 16, 45]);
        for book in &BOOKS {
            for pair in book.spells.windows(2) {
                assert_eq!(pair[0].id + 1, pair[1].id, "{}", book.name);
            }
        }
    }

    #[test]
    fn a_book_is_found_by_school_then_graphic_then_magery() {
        assert_eq!(book_info("necromancy", 0).school, School::Necromancy);
        assert_eq!(book_info("", 0x2D50).school, School::Spellweaving);
        assert_eq!(book_info("", 0x1234).school, School::Magery);
        assert_eq!(
            book_spell(203).map(|(book, _)| book.school),
            Some(School::Chivalry)
        );
    }

    #[test]
    fn the_index_pages_and_icons_follow_the_classic_book() {
        let magery = &BOOKS[0];
        assert_eq!(magery.spells_on_page(), 8);
        assert_eq!(magery.index_pages(), 8);
        assert_eq!(magery.icon(3), 0x08C3);
        assert_eq!(magery.tooltip(3), Some(1_061_293));
        let bushido = &BOOKS[3];
        assert_eq!(bushido.spells_on_page(), 3);
        assert_eq!(bushido.index_pages(), 2);
        let mastery = &BOOKS[7];
        assert_eq!(mastery.icon(0), 0x0945);
        assert_eq!(mastery.tooltip(1), Some(MASTERY_FIRST_TOOLTIP + 1));
        assert_eq!(mastery.tooltip(6), Some(MASTERY_LATER_TOOLTIP));
    }

    #[test]
    fn names_words_and_reagents_read_as_the_book_writes_them() {
        assert_eq!(book_name(51), "Flamestrike");
        assert_eq!(book_name(1), "Clumsy");
        assert_eq!(words_letters(&power_words(1)), "U J");
        let (_, heal) = book_spell(4).unwrap();
        assert_eq!(reagent_lines(heal), "Garlic\nGinseng\nSpiders Silk");
        assert_eq!(mastery_group(8), "Magery");
        assert_eq!(mastery_group(1), "Provocation");
        assert_eq!(active_masteries(|cliloc| cliloc == 0x11A2BF), &[18, 16, 17]);
        assert!(active_masteries(|_| false).is_empty());
        assert_eq!(button_tooltip(2), Some(3_002_012));
        assert_eq!(button_tooltip(707), Some(1_155_896));
        assert_eq!(button_tooltip(900), None);
    }

    #[test]
    fn a_book_holds_only_the_spells_the_shard_names_and_each_has_its_group() {
        let magery = &BOOKS[0];
        let contents = WatchSpellbook {
            spells: vec![(18, String::new()), (1, String::new()), (1, String::new())],
            ..WatchSpellbook::default()
        };
        assert_eq!(magery.held_places(Some(&contents)), vec![0, 17]);
        assert!(magery.held_places(None).is_empty());
        assert_eq!(magery.spell_group(17), Some("Third Circle"));
        assert_eq!(BOOKS[7].spell_group(7), Some("Magery"));
        assert_eq!(BOOKS[2].spell_group(0), None);
        assert_eq!(BOOKS[7].title, "Masteries");
    }

    #[test]
    fn needs_a_macro_and_the_active_hue_read_as_the_book_has_them() {
        assert_eq!(needs_words(10, 25, 0), "Mana cost: 10\nMin. Skill: 25");
        assert!(needs_words(16, 90, 4).starts_with("Upkeep Cost: 4\n"));
        let (mastery, inspire) = book_spell(701).unwrap();
        assert_eq!(upkeep(mastery, inspire), 4);
        let (chivalry, cleanse) = book_spell(201).unwrap();
        assert_eq!(upkeep(chivalry, cleanse), 0);
        let (name, steps) = spell_macro(51);
        assert_eq!(name, "Flamestrike");
        assert_eq!(steps, vec![MacroStep::new("cast", "Flamestrike")]);
        let mut frame = WatchFrame::default();
        assert_eq!(icon_hue(&frame, 401), 0);
        frame
            .abilities
            .spells
            .push((401, "Honorable Execution".into()));
        assert_eq!(icon_hue(&frame, 401), ACTIVE_SPELL_HUE);
    }
}
