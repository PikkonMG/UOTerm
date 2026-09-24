//! The pictures of a paperdoll and the order they go on, as the reference client
//! has them: the body gump of each body, the gump of each worn item from its
//! animation number, and the worn items that hide others. No drawing here.

use crate::view::WatchEquip;
use crate::window::settings::BackpackStyle;
use uoterm_protocol::types::{
    LAYER_ARMS, LAYER_BACKPACK, LAYER_BEARD, LAYER_BRACELET, LAYER_CLOAK, LAYER_EARRINGS,
    LAYER_FACE, LAYER_GLOVES, LAYER_HAIR, LAYER_HELMET, LAYER_LEGS, LAYER_NECKLACE,
    LAYER_ONE_HANDED, LAYER_PANTS, LAYER_RING, LAYER_ROBE, LAYER_SHIRT, LAYER_SHOES, LAYER_SKIRT,
    LAYER_TALISMAN, LAYER_TORSO, LAYER_TUNIC, LAYER_TWO_HANDED, LAYER_WAIST,
};
use uoterm_world::{
    BODY_ELF_FEMALE, BODY_ELF_MALE, BODY_GARGOYLE_FEMALE, BODY_GARGOYLE_MALE, BODY_GHOST_FEMALE,
    BODY_GHOST_GARGOYLE_FEMALE, BODY_GHOST_GARGOYLE_MALE, BODY_HUMAN_FEMALE,
};

/// The gumps of worn items for a man start here, and for a woman here.
pub const MALE_GUMP_OFFSET: u32 = 50_000;
pub const FEMALE_GUMP_OFFSET: u32 = 60_000;
/// The first gump id past the end of the gump files.
const GUMP_COUNT: u32 = 0x1_0000;

// The body gumps of the paperdoll.
const BODY_GUMP_MAN: u16 = 0x000C;
const BODY_GUMP_WOMAN: u16 = 0x000D;
const BODY_GUMP_ELF_MAN: u16 = 0x000E;
const BODY_GUMP_ELF_WOMAN: u16 = 0x000F;
const BODY_GUMP_GARGOYLE_MAN: u16 = 0x029A;
const BODY_GUMP_GARGOYLE_WOMAN: u16 = 0x0299;
/// The body of the Vampire form, and its gump.
const BODY_VAMPIRE: u16 = 0x04E5;
const BODY_GUMP_VAMPIRE: u16 = 0xC835;
/// The body of the Wraith form: the man's gump in a pale hue, with a
/// shroud over it in the hue of the mobile.
const BODY_WRAITH: u16 = 0x03DB;
const WRAITH_HUE: u16 = 0x03EA;
const WRAITH_SHROUD: u16 = 0xC72B;
/// The backpacks of the character's own paperdoll, by the style the
/// Containers page picks.
const OWN_BACKPACK_DEFAULT: u16 = 0xC4F6;
const OWN_BACKPACK_SUEDE: u16 = 0x777B;
const OWN_BACKPACK_POLAR_BEAR: u16 = 0x777C;
const OWN_BACKPACK_GHOUL_SKIN: u16 = 0x777D;
/// The death shroud a gargoyle ghost wears shows its own gump.
const DEATH_SHROUD_ANIM: u16 = 0x03CA;
const GARGOYLE_SHROUD_ANIM: u16 = 0x0223;

/// The layer that is no layer, where the order tables start.
const NO_LAYER: u8 = 0;
/// The order tables hold every layer from none to the talisman.
const ORDER_LEN: usize = 25;
/// The worn items the order reads, from the one-handed weapon to the legs.
const LAYER_SLOTS: usize = LAYER_LEGS as usize + 1;

/// The order most bodies draw in.
const ORDER_PLAIN: [u8; ORDER_LEN] = [
    NO_LAYER,
    LAYER_CLOAK,
    LAYER_SHIRT,
    LAYER_PANTS,
    LAYER_SHOES,
    LAYER_LEGS,
    LAYER_ARMS,
    LAYER_TORSO,
    LAYER_TUNIC,
    LAYER_RING,
    LAYER_BRACELET,
    LAYER_FACE,
    LAYER_GLOVES,
    LAYER_SKIRT,
    LAYER_ROBE,
    LAYER_WAIST,
    LAYER_NECKLACE,
    LAYER_HAIR,
    LAYER_BEARD,
    LAYER_EARRINGS,
    LAYER_HELMET,
    LAYER_ONE_HANDED,
    LAYER_TWO_HANDED,
    LAYER_BACKPACK,
    LAYER_TALISMAN,
];

/// The order when the arms go over the chest.
const ORDER_ARMS_OVER: [u8; ORDER_LEN] = [
    NO_LAYER,
    LAYER_CLOAK,
    LAYER_SHIRT,
    LAYER_PANTS,
    LAYER_SHOES,
    LAYER_LEGS,
    LAYER_TORSO,
    LAYER_TUNIC,
    LAYER_RING,
    LAYER_BRACELET,
    LAYER_FACE,
    LAYER_ARMS,
    LAYER_GLOVES,
    LAYER_SKIRT,
    LAYER_ROBE,
    LAYER_WAIST,
    LAYER_NECKLACE,
    LAYER_HAIR,
    LAYER_BEARD,
    LAYER_EARRINGS,
    LAYER_HELMET,
    LAYER_ONE_HANDED,
    LAYER_TWO_HANDED,
    LAYER_BACKPACK,
    LAYER_TALISMAN,
];

/// The order when a woman's chest piece goes under the shirt.
const ORDER_TORSO_FIRST: [u8; ORDER_LEN] = [
    NO_LAYER,
    LAYER_CLOAK,
    LAYER_TORSO,
    LAYER_SHIRT,
    LAYER_PANTS,
    LAYER_SHOES,
    LAYER_LEGS,
    LAYER_TUNIC,
    LAYER_RING,
    LAYER_BRACELET,
    LAYER_FACE,
    LAYER_ARMS,
    LAYER_GLOVES,
    LAYER_SKIRT,
    LAYER_ROBE,
    LAYER_WAIST,
    LAYER_NECKLACE,
    LAYER_HAIR,
    LAYER_BEARD,
    LAYER_EARRINGS,
    LAYER_HELMET,
    LAYER_ONE_HANDED,
    LAYER_TWO_HANDED,
    LAYER_BACKPACK,
    LAYER_TALISMAN,
];

// The animation numbers that change the order, as the reference client names them.
const ARMS_LOW_LIMIT: u16 = 0x03D0;
const ARMS_OVER_LOW: [u16; 3] = [0x03CF, 0x0210, 0x03B3];
const ARMS_OVER_HIGH: u16 = 0x03DD;
const TORSO_ARMS_OVER: u16 = 0x021A;
const TORSO_WOMAN_FIRST: u16 = 0x0399;
const TORSO_WOMAN_COUNT: u16 = 5;
const PANTS_UNDER_SHIRT: u16 = 0x0398;
const PANTS_LOW_LIMIT: u16 = 0x0201;
const PANTS_UNDER_SHOES: [u16; 3] = [0x0200, 0x01EB, 0x01FA];
const PANTS_OVER_SHOES_FIRST: u16 = 0x0513;
const PANTS_OVER_SHOES_COUNT: u16 = 2;
const PANTS_TUCKED: u16 = 0x03E4;
const TUNIC_SURCOAT: u16 = 0x0238;
const ROBES_UNDER_NECK: [u16; 8] = [
    0x04E8, 0x04E9, 0x04EA, 0x04EB, 0x05E2, 0x05E3, 0x05E4, 0x05E5,
];
const CLOAKS_OVER_ROBE: [u16; 2] = [0x0380, 0x05F3];
const HELM_LOW_LIMIT: u16 = 0x0202;
const HELMS_OVER_NECK: [u16; 2] = [0x0201, 0x01A9];
const NECK_UNDER_HELM: u16 = 0x01C8;
const NECK_UNDER_HELM_ABOVE: u16 = 0x01D6;
const NECK_UNDER_HELM_BELOW: u16 = 0x01D9;
const HELMS_UNDER_ROBE_FIRST: u16 = 0x05E9;
const HELMS_UNDER_ROBE_COUNT: u16 = 2;
const ROBES_OVER_HELM_FIRST: u16 = 0x05E2;
const ROBES_OVER_HELM_COUNT: u16 = 4;

// The pictures `IsCovered` knows.
const ROBES_OPEN_CHEST: [u16; 6] = [0x9985, 0x9986, 0xA2CA, 0xA2CB, 0xA412, 0xB1DE];
const ROBE_ANIM_FULL: u16 = 0x0504;
const PANTS_ANIMS_OVER_SHOES: [u16; 2] = [0x0513, 0x0514];
const PANTS_GRAPHIC_SPLIT: u16 = 0xAEB2;
const PANTS_HIDING_SHOES_LOW: [u16; 2] = [0xAEB1, 0x1411];
const PANTS_SHOWING_SHOES_LOW: u16 = 0xAEA2;
const PANTS_HIDING_SHOES_HIGH: u16 = 0xAEC0;
const PANTS_SHOWING_SHOES_HIGH: u16 = 0xAECF;
const PANTS_ANIMS_UNDER_SKIRT: [u16; 3] = [0x01EB, 0x01FA, 0x0200];
const SKIRT_ANIMS_OVER_PANTS: [u16; 2] = [0x01C7, 0x01E4];
const ROBE_ANIM_SPLIT: u16 = 0x04EC;
const ROBE_ANIM_OPEN_ABOVE: u16 = 0x04E7;
const ROBE_ANIM_SHOWING_PANTS: u16 = 0x0229;
const ROBE_ANIMS_OPEN_FIRST: u16 = 0x05E2;
const ROBE_ANIMS_OPEN_COUNT: u16 = 4;
const TORSO_UNDER_TUNIC: [u16; 2] = [0x782A, 0x782B];
const TUNICS_SHOWING_TORSO: [u16; 2] = [0x1541, 0x1542];
const ROBE_ANIMS_SHOWING_NECK: [u16; 2] = [0x05F2, 0x05F5];
const ROBE_ANIMS_SHOWING_NECK_LOW: std::ops::RangeInclusive<u16> = 0x04E8..=0x04EB;
const ROBE_ANIMS_SHOWING_NECK_HIGH: std::ops::RangeInclusive<u16> = 0x05E2..=0x05E5;
const NECK_ANIM_UNDER_ROBE: u16 = 0x05EC;
const BRACELET_UNDER_ARMS: u16 = 0xB1C0;
const HELMS_HIDING_HAIR_FIRST: u16 = 0xA42B;
const HELMS_HIDING_HAIR_COUNT: u16 = 2;
// The hoods that hide the hair and the helmet.
const HOODS_LOW_LIMIT: u16 = 0x4B9E;
const HOOD_WIDE: u16 = 0x4B9D;
const HOODS_MIDDLE_LIMIT: u16 = 0x2FBA;
const HOOD_MIDDLE: u16 = 0x2FB9;
const HOODS_ROW_LAST: u16 = 0x2687;
const HOODS_ROW_FIRST: u16 = 0x2683;
const HOODS_PAIR: std::ops::RangeInclusive<u16> = 0x204E..=0x204F;
const HOOD_RAISED: u16 = 0x3173;
const HOODS_HIGH_LIMIT: u16 = 0xA0B0;
const HOODS_HIGH_FIRST: u16 = 0xA0AB;
const HOOD_OLD: u16 = 0x7816;
const HOOD_LAST: u16 = 0xB2B7;

/// The body picture of a paperdoll: its gump, the hue it takes instead of
/// the mobile's, and a picture over it in the mobile's hue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyGump {
    pub gump: u16,
    pub hue: Option<u16>,
    pub overlay: Option<u16>,
}

/// True for a woman's body, as the reference client reads it from the body.
pub fn is_female_body(body: u16) -> bool {
    matches!(
        body,
        BODY_HUMAN_FEMALE | BODY_GHOST_FEMALE | BODY_ELF_FEMALE | BODY_GARGOYLE_FEMALE
    )
}

/// True for a gargoyle's body, alive or a ghost.
pub fn is_gargoyle_body(body: u16) -> bool {
    matches!(
        body,
        BODY_GARGOYLE_MALE
            | BODY_GARGOYLE_FEMALE
            | BODY_GHOST_GARGOYLE_FEMALE
            | BODY_GHOST_GARGOYLE_MALE
    )
}

/// The body gump of a mobile's paperdoll.
pub fn body_gump(body: u16, female: bool) -> BodyGump {
    let plain = |gump| BodyGump {
        gump,
        hue: None,
        overlay: None,
    };
    match body {
        BODY_HUMAN_FEMALE | BODY_GHOST_FEMALE => plain(BODY_GUMP_WOMAN),
        BODY_ELF_MALE => plain(BODY_GUMP_ELF_MAN),
        BODY_ELF_FEMALE => plain(BODY_GUMP_ELF_WOMAN),
        BODY_GARGOYLE_MALE | BODY_GHOST_GARGOYLE_FEMALE => plain(BODY_GUMP_GARGOYLE_MAN),
        BODY_GARGOYLE_FEMALE | BODY_GHOST_GARGOYLE_MALE => plain(BODY_GUMP_GARGOYLE_WOMAN),
        BODY_VAMPIRE => plain(BODY_GUMP_VAMPIRE),
        BODY_WRAITH => BodyGump {
            gump: BODY_GUMP_MAN,
            hue: Some(WRAITH_HUE),
            overlay: Some(WRAITH_SHROUD),
        },
        _ if female => plain(BODY_GUMP_WOMAN),
        _ => plain(BODY_GUMP_MAN),
    }
}

/// What a mobile wears, for the order: the graphic and the animation
/// number of the item on each layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Worn {
    items: [Option<(u16, u16)>; LAYER_SLOTS],
}

impl Worn {
    /// The worn items, with `anim_of` giving the animation number of a
    /// graphic from the tiledata.
    pub fn new(equipment: &[WatchEquip], anim_of: impl Fn(u16) -> u16) -> Self {
        let mut items = [None; LAYER_SLOTS];
        for item in equipment {
            if let Some(slot) = items.get_mut(usize::from(item.layer)) {
                *slot = Some((item.graphic, anim_of(item.graphic)));
            }
        }
        Self { items }
    }

    fn get(&self, layer: u8) -> Option<(u16, u16)> {
        self.items.get(usize::from(layer)).copied().flatten()
    }

    fn has(&self, layer: u8) -> bool {
        self.get(layer).is_some()
    }

    fn graphic(&self, layer: u8) -> u16 {
        self.get(layer).map_or(0, |(graphic, _)| graphic)
    }

    fn anim(&self, layer: u8) -> u16 {
        self.get(layer).map_or(0, |(_, anim)| anim)
    }
}

/// True when `value - first` is below `count`, counted as the classic
/// client does with no sign, so a value under `first` is never in.
fn in_run(value: u16, first: u16, count: u16) -> bool {
    value.wrapping_sub(first) < count
}

fn place(order: &[u8], layer: u8) -> Option<usize> {
    order.iter().position(|each| *each == layer)
}

/// Puts `layer` at the place of `at`, the rest moving up or down one.
fn move_to(order: &mut Vec<u8>, layer: u8, at: u8) {
    let (Some(from), Some(to)) = (place(order, layer), place(order, at)) else {
        return;
    };
    if from != to {
        let moved = order.remove(from);
        order.insert(to, moved);
    }
}

/// Puts `layer` right after `after`.
fn move_after(order: &mut Vec<u8>, layer: u8, after: u8) {
    let (Some(from), Some(to)) = (place(order, layer), place(order, after)) else {
        return;
    };
    if from == to {
        return;
    }
    let moved = order.remove(from);
    let to = if to < from { to + 1 } else { to };
    order.insert(to, moved);
}

/// The worn layers of a paperdoll in the order they are drawn, first to
/// last, without the backpack. `alternate_torso` is set for women and
/// gargoyles.
pub fn paint_order(worn: &Worn, alternate_torso: bool) -> Vec<u8> {
    let anim = |layer| worn.anim(layer);
    let arms = anim(LAYER_ARMS);
    let arms_over = if arms < ARMS_LOW_LIMIT {
        ARMS_OVER_LOW.contains(&arms)
    } else {
        arms == ARMS_OVER_HIGH
    };
    let torso = anim(LAYER_TORSO);
    let table = if arms_over || torso == TORSO_ARMS_OVER {
        &ORDER_ARMS_OVER
    } else if in_run(torso, TORSO_WOMAN_FIRST, TORSO_WOMAN_COUNT) && alternate_torso {
        &ORDER_TORSO_FIRST
    } else {
        &ORDER_PLAIN
    };
    let mut order = table.to_vec();
    reorder(&mut order, &anim);
    order.retain(|layer| *layer != NO_LAYER && *layer != LAYER_BACKPACK);
    order
}

/// The moves of the order that single pieces make.
fn reorder(order: &mut Vec<u8>, anim: &impl Fn(u8) -> u16) {
    if anim(LAYER_SHIRT) != 0 && anim(LAYER_PANTS) == PANTS_UNDER_SHIRT {
        move_to(order, LAYER_PANTS, LAYER_SHIRT);
    }
    let pants = anim(LAYER_PANTS);
    let shoes = anim(LAYER_SHOES);
    let mut pants_settled = false;
    if pants < PANTS_LOW_LIMIT {
        if PANTS_UNDER_SHOES.contains(&pants) {
            if let (Some(at_shoes), Some(at_pants)) =
                (place(order, LAYER_SHOES), place(order, LAYER_PANTS))
            {
                if at_pants < at_shoes {
                    order.swap(at_shoes, at_pants);
                }
            }
        }
    } else if in_run(pants, PANTS_OVER_SHOES_FIRST, PANTS_OVER_SHOES_COUNT) {
        if shoes != 0 {
            move_to(order, LAYER_PANTS, LAYER_SHOES);
        }
        pants_settled = true;
    }
    if !pants_settled && shoes != 0 && pants == PANTS_TUCKED {
        move_to(order, LAYER_PANTS, LAYER_SHOES);
    }
    if anim(LAYER_TUNIC) == TUNIC_SURCOAT {
        move_after(order, LAYER_TUNIC, LAYER_WAIST);
        if ROBES_UNDER_NECK.contains(&anim(LAYER_ROBE)) {
            move_to(order, LAYER_ROBE, LAYER_NECKLACE);
        }
    }
    if CLOAKS_OVER_ROBE.contains(&anim(LAYER_CLOAK)) {
        move_after(order, LAYER_CLOAK, LAYER_ROBE);
    }
    let helm = anim(LAYER_HELMET);
    let robe = anim(LAYER_ROBE);
    if helm < HELM_LOW_LIMIT {
        let neck = anim(LAYER_NECKLACE);
        let neck_under = neck == NECK_UNDER_HELM
            || (neck > NECK_UNDER_HELM_ABOVE && neck < NECK_UNDER_HELM_BELOW);
        if HELMS_OVER_NECK.contains(&helm) && neck_under {
            move_after(order, LAYER_NECKLACE, LAYER_HELMET);
        }
    } else if in_run(helm, HELMS_UNDER_ROBE_FIRST, HELMS_UNDER_ROBE_COUNT)
        && robe != 0
        && in_run(robe, ROBES_OVER_HELM_FIRST, ROBES_OVER_HELM_COUNT)
    {
        move_to(order, LAYER_ROBE, LAYER_HELMET);
    }
}

/// True when another worn item hides the item on `layer`, so it is not
/// drawn.
pub fn is_covered(worn: &Worn, layer: u8, gargoyle: bool) -> bool {
    let robe_graphic = worn.graphic(LAYER_ROBE);
    let robe_anim = worn.anim(LAYER_ROBE);
    let robe_open = ROBES_OPEN_CHEST.contains(&robe_graphic);
    match layer {
        LAYER_SHOES => shoes_covered(worn, robe_anim),
        LAYER_PANTS => pants_covered(worn, robe_anim),
        LAYER_TUNIC => {
            worn.anim(LAYER_TUNIC) == TUNIC_SURCOAT && worn.has(LAYER_ROBE) && !robe_open
        }
        LAYER_TORSO => {
            if robe_graphic != 0 && !robe_open {
                return true;
            }
            worn.has(LAYER_TUNIC)
                && !TUNICS_SHOWING_TORSO.contains(&worn.graphic(LAYER_TUNIC))
                && TORSO_UNDER_TUNIC.contains(&worn.graphic(LAYER_TORSO))
        }
        LAYER_ARMS => robe_graphic != 0 && !robe_open,
        LAYER_NECKLACE => {
            let neck_showing = ROBE_ANIMS_SHOWING_NECK.contains(&robe_anim)
                || ROBE_ANIMS_SHOWING_NECK_LOW.contains(&robe_anim)
                || ROBE_ANIMS_SHOWING_NECK_HIGH.contains(&robe_anim);
            worn.has(LAYER_ROBE)
                && !neck_showing
                && worn.anim(LAYER_NECKLACE) == NECK_ANIM_UNDER_ROBE
        }
        LAYER_BRACELET => {
            worn.graphic(LAYER_BRACELET) == BRACELET_UNDER_ARMS && worn.has(LAYER_ARMS)
        }
        LAYER_HAIR => {
            in_run(
                worn.graphic(LAYER_HELMET),
                HELMS_HIDING_HAIR_FIRST,
                HELMS_HIDING_HAIR_COUNT,
            ) || under_hood(robe_graphic, gargoyle)
        }
        LAYER_HELMET => under_hood(robe_graphic, gargoyle),
        LAYER_SKIRT => {
            SKIRT_ANIMS_OVER_PANTS.contains(&worn.anim(LAYER_SKIRT))
                && PANTS_ANIMS_UNDER_SKIRT.contains(&worn.anim(LAYER_PANTS))
        }
        _ => false,
    }
}

fn shoes_covered(worn: &Worn, robe_anim: u16) -> bool {
    let pants_anim = worn.anim(LAYER_PANTS);
    if robe_anim == ROBE_ANIM_FULL || PANTS_ANIMS_OVER_SHOES.contains(&pants_anim) {
        return true;
    }
    let pants = worn.graphic(LAYER_PANTS);
    if pants < PANTS_GRAPHIC_SPLIT {
        if PANTS_HIDING_SHOES_LOW.contains(&pants) {
            return true;
        }
        if pants != PANTS_SHOWING_SHOES_LOW {
            return worn.has(LAYER_LEGS);
        }
    } else {
        if pants == PANTS_HIDING_SHOES_HIGH {
            return true;
        }
        if pants != PANTS_SHOWING_SHOES_HIGH {
            return worn.has(LAYER_LEGS);
        }
    }
    true
}

fn pants_covered(worn: &Worn, robe_anim: u16) -> bool {
    if worn.has(LAYER_LEGS) || robe_anim == ROBE_ANIM_FULL {
        return true;
    }
    if !PANTS_ANIMS_UNDER_SKIRT.contains(&worn.anim(LAYER_PANTS)) {
        return false;
    }
    if worn.has(LAYER_SKIRT) && !SKIRT_ANIMS_OVER_PANTS.contains(&worn.anim(LAYER_SKIRT)) {
        return true;
    }
    if !worn.has(LAYER_ROBE) {
        return false;
    }
    if robe_anim < ROBE_ANIM_SPLIT {
        robe_anim <= ROBE_ANIM_OPEN_ABOVE && robe_anim != ROBE_ANIM_SHOWING_PANTS
    } else {
        !in_run(robe_anim, ROBE_ANIMS_OPEN_FIRST, ROBE_ANIMS_OPEN_COUNT)
    }
}

/// True when the robe is a hood that hides the head. The hoods of the
/// middle rows hide it on every body, the rest on all but gargoyles.
fn under_hood(robe: u16, gargoyle: bool) -> bool {
    if robe < HOODS_LOW_LIMIT {
        if robe != HOOD_WIDE {
            if robe < HOODS_MIDDLE_LIMIT {
                let in_row = robe == HOOD_MIDDLE
                    || (robe <= HOODS_ROW_LAST
                        && (robe >= HOODS_ROW_FIRST || HOODS_PAIR.contains(&robe)));
                return in_row;
            }
            return robe == HOOD_RAISED;
        }
    } else if robe < HOODS_HIGH_LIMIT {
        if robe < HOODS_HIGH_FIRST && robe != HOOD_OLD {
            return false;
        }
    } else if robe != HOOD_LAST {
        return false;
    }
    !gargoyle
}

/// The gump of a worn item on a paperdoll: its animation number, turned by
/// `Equipconv.def` when the body has a conversion (`converted` is the gump
/// column there), from the gumps of the mobile's sex, or of the other sex
/// when the files lack them. None when neither is in the files.
pub fn equipment_gump(
    body: u16,
    anim: u16,
    female: bool,
    converted: Option<u16>,
    mut exists: impl FnMut(u16) -> bool,
) -> Option<u16> {
    let mut anim = u32::from(anim);
    if anim == u32::from(DEATH_SHROUD_ANIM)
        && matches!(body, BODY_GHOST_GARGOYLE_FEMALE | BODY_GHOST_GARGOYLE_MALE)
    {
        anim = u32::from(GARGOYLE_SHROUD_ANIM);
    }
    if let Some(gump) = converted.map(u32::from) {
        anim = if gump >= FEMALE_GUMP_OFFSET {
            gump - FEMALE_GUMP_OFFSET
        } else if gump > MALE_GUMP_OFFSET {
            gump - MALE_GUMP_OFFSET
        } else {
            gump
        };
    }
    let (own, other) = if female {
        (FEMALE_GUMP_OFFSET, MALE_GUMP_OFFSET)
    } else {
        (MALE_GUMP_OFFSET, FEMALE_GUMP_OFFSET)
    };
    let mut found = |offset: u32| {
        u16::try_from(anim + offset)
            .ok()
            .filter(|gump| u32::from(*gump) < GUMP_COUNT && exists(*gump))
    };
    match found(own) {
        Some(gump) => Some(gump),
        None => found(other),
    }
}

/// The gump of the backpack on a paperdoll, from its animation number.
pub fn backpack_gump(anim: u16) -> Option<u16> {
    u16::try_from(u32::from(anim) + MALE_GUMP_OFFSET).ok()
}

/// The gump of the backpack on the character's own paperdoll in a style.
/// The caller uses it when the files hold it.
pub fn own_backpack_gump(style: BackpackStyle) -> u16 {
    match style {
        BackpackStyle::Default => OWN_BACKPACK_DEFAULT,
        BackpackStyle::Suede => OWN_BACKPACK_SUEDE,
        BackpackStyle::PolarBear => OWN_BACKPACK_POLAR_BEAR,
        BackpackStyle::GhoulSkin => OWN_BACKPACK_GHOUL_SKIN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worn(pieces: &[(u8, u16, u16)]) -> Worn {
        let equipment: Vec<WatchEquip> = pieces
            .iter()
            .map(|(layer, graphic, _)| WatchEquip {
                layer: *layer,
                graphic: *graphic,
                ..WatchEquip::default()
            })
            .collect();
        Worn::new(&equipment, |graphic| {
            pieces
                .iter()
                .find(|(_, g, _)| *g == graphic)
                .map_or(0, |(_, _, anim)| *anim)
        })
    }

    fn before(order: &[u8], first: u8, second: u8) -> bool {
        place(order, first) < place(order, second)
    }

    #[test]
    fn bodies_take_their_gumps() {
        assert_eq!(body_gump(BODY_HUMAN_FEMALE, true).gump, BODY_GUMP_WOMAN);
        assert_eq!(body_gump(BODY_ELF_MALE, false).gump, BODY_GUMP_ELF_MAN);
        assert_eq!(
            body_gump(BODY_GARGOYLE_FEMALE, true).gump,
            BODY_GUMP_GARGOYLE_WOMAN
        );
        let wraith = body_gump(BODY_WRAITH, false);
        assert_eq!(
            (wraith.gump, wraith.hue, wraith.overlay),
            (BODY_GUMP_MAN, Some(WRAITH_HUE), Some(WRAITH_SHROUD))
        );
        assert_eq!(body_gump(0x0190, true).gump, BODY_GUMP_WOMAN);
        assert!(is_female_body(BODY_ELF_FEMALE) && !is_female_body(BODY_ELF_MALE));
        assert!(is_gargoyle_body(BODY_GHOST_GARGOYLE_MALE));
    }

    #[test]
    fn the_plain_order_leaves_out_the_backpack_and_puts_weapons_on_top() {
        let order = paint_order(&Worn::default(), false);
        assert_eq!(order.first(), Some(&LAYER_CLOAK));
        assert!(!order.contains(&LAYER_BACKPACK) && !order.contains(&NO_LAYER));
        assert!(before(&order, LAYER_HELMET, LAYER_ONE_HANDED));
        assert!(before(&order, LAYER_ARMS, LAYER_TORSO));
    }

    #[test]
    fn some_pieces_move_in_the_order() {
        let arms_over = paint_order(&worn(&[(LAYER_ARMS, 1, 0x03CF)]), false);
        assert!(before(&arms_over, LAYER_TORSO, LAYER_ARMS));
        let woman = paint_order(&worn(&[(LAYER_TORSO, 1, 0x0399)]), true);
        assert!(before(&woman, LAYER_TORSO, LAYER_SHIRT));
        let tucked = paint_order(
            &worn(&[(LAYER_SHOES, 1, 5), (LAYER_PANTS, 2, 0x03E4)]),
            false,
        );
        assert!(before(&tucked, LAYER_SHOES, LAYER_PANTS));
        let surcoat = paint_order(&worn(&[(LAYER_TUNIC, 1, TUNIC_SURCOAT)]), false);
        assert_eq!(
            place(&surcoat, LAYER_TUNIC),
            place(&surcoat, LAYER_WAIST).map(|at| at + 1)
        );
        let cloak = paint_order(&worn(&[(LAYER_CLOAK, 1, 0x0380)]), false);
        assert_eq!(
            place(&cloak, LAYER_CLOAK),
            place(&cloak, LAYER_ROBE).map(|at| at + 1)
        );
    }

    #[test]
    fn a_robe_hides_the_chest_and_legs_hide_the_pants() {
        let robed = worn(&[(LAYER_ROBE, 0x1F03, 0x0229), (LAYER_TORSO, 0x1415, 1)]);
        assert!(is_covered(&robed, LAYER_TORSO, false));
        assert!(is_covered(&robed, LAYER_ARMS, false));
        let open = worn(&[(LAYER_ROBE, 0x9985, 1)]);
        assert!(!is_covered(&open, LAYER_TORSO, false));
        let legs = worn(&[(LAYER_LEGS, 0x1411, 1), (LAYER_PANTS, 0x152E, 2)]);
        assert!(is_covered(&legs, LAYER_PANTS, false));
        assert!(is_covered(&legs, LAYER_SHOES, false));
        assert!(!is_covered(&worn(&[]), LAYER_SHOES, false));
        let hood = worn(&[(LAYER_ROBE, HOOD_RAISED, 1)]);
        assert!(is_covered(&hood, LAYER_HAIR, true));
        let high_hood = worn(&[(LAYER_ROBE, HOOD_LAST, 1)]);
        assert!(is_covered(&high_hood, LAYER_HELMET, false));
        assert!(!is_covered(&high_hood, LAYER_HELMET, true));
    }

    #[test]
    fn a_worn_gump_falls_back_to_the_other_sex_and_follows_conversions() {
        let all = |_: u16| true;
        assert_eq!(
            equipment_gump(0x0190, 0x0200, false, None, all),
            Some(50_512)
        );
        assert_eq!(
            equipment_gump(0x0191, 0x0200, true, None, all),
            Some(60_512)
        );
        let only_male = |gump: u16| u32::from(gump) < FEMALE_GUMP_OFFSET;
        assert_eq!(
            equipment_gump(0x0191, 0x0200, true, None, only_male),
            Some(50_512)
        );
        assert_eq!(
            equipment_gump(0x0190, 7, false, Some(61_250), all),
            Some(51_250)
        );
        assert_eq!(
            equipment_gump(0x0190, 7, false, Some(12), all),
            Some(50_012)
        );
        assert_eq!(equipment_gump(0x0190, 7, false, None, |_| false), None);
        assert_eq!(
            equipment_gump(
                BODY_GHOST_GARGOYLE_MALE,
                DEATH_SHROUD_ANIM,
                false,
                None,
                all
            ),
            Some(50_000 + GARGOYLE_SHROUD_ANIM)
        );
        assert_eq!(backpack_gump(0x03E7), Some(50_999));
        assert_eq!(
            own_backpack_gump(BackpackStyle::Default),
            OWN_BACKPACK_DEFAULT
        );
        assert_eq!(own_backpack_gump(BackpackStyle::Suede), OWN_BACKPACK_SUEDE);
    }
}
