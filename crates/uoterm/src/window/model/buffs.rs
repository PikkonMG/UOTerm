//! The improved buff bar: the gump picture of each buff icon, and the time
//! it has left in words.

use crate::view::WatchBuff;

/// The first buff icon number of the older table, and of the newer one.
/// A newer icon comes after the older ones in the picture table.
const FIRST_ICON: u16 = 0x03E9;
const FIRST_NEWER_ICON: u16 = 0x0466;
const NEWER_ICONS_AT: u16 = 125;
const NO_PICTURE: u16 = 0;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
/// A buff with less time than this left is about to end.
pub const ENDING_SECONDS: u64 = 10;

/// The gump picture of each buff icon, in the order of the icon numbers,
/// as the official client has them.
#[rustfmt::skip]
const BUFF_GUMPS: [u16; 189] = [
    0x754C, 0x754A, 0x0000, 0x0000, 0x755E, 0x7549, 0x7551, 0x7556, 0x753A, 0x754D,
    0x754E, 0x7565, 0x753B, 0x7543, 0x7544, 0x7546, 0x755C, 0x755F, 0x7566, 0x7554,
    0x7540, 0x7568, 0x754F, 0x7550, 0x7553, 0x753E, 0x755D, 0x7563, 0x7562, 0x753F,
    0x7559, 0x7557, 0x754B, 0x753D, 0x7561, 0x7558, 0x755B, 0x7560, 0x7541, 0x7545,
    0x7552, 0x7569, 0x7548, 0x755A, 0x753C, 0x7547, 0x7567, 0x7542, 0x758A, 0x758B,
    0x758C, 0x758D, 0x0000, 0x758E, 0x094B, 0x094C, 0x094D, 0x094E, 0x094F, 0x0950,
    0x753E, 0x5011, 0x7590, 0x7591, 0x7592, 0x7593, 0x7594, 0x7595, 0x7596, 0x7598,
    0x7599, 0x759B, 0x759C, 0x759E, 0x759F, 0x75A0, 0x75A1, 0x75A3, 0x75A4, 0x75A5,
    0x75A6, 0x75A7, 0x75C0, 0x75C1, 0x75C2, 0x75C3, 0x75C4, 0x75F2, 0x75F3, 0x75F4,
    0x75F5, 0x75F6, 0x75F7, 0x75F8, 0x75F9, 0x75FA, 0x75FB, 0x75FC, 0x75FD, 0x75FE,
    0x75FF, 0x7600, 0x7601, 0x7602, 0x7603, 0x7604, 0x7605, 0x7606, 0x7607, 0x7608,
    0x7609, 0x760A, 0x760B, 0x760C, 0x760D, 0x760E, 0x760F, 0x7610, 0x7611, 0x7612,
    0x7613, 0x7614, 0x7615, 0x75C5, 0x75F6, 0x761B, 0x9BC9, 0x9BB5, 0x9BDD, 0x9BC6,
    0x9BCC, 0x9BBE, 0x9BBD, 0x9BCB, 0x9BC8, 0x9BBF, 0x9BCD, 0x9BC0, 0x9BCE, 0x9BC1,
    0x9BC7, 0x9BC2, 0x9BB7, 0x9BCA, 0x9BB6, 0x9BB8, 0x9BB9, 0x9BBA, 0x9BBB, 0x9BBC,
    0x9BC3, 0x9BC4, 0x9BC5, 0x9BD2, 0x9BD3, 0x9BD4, 0x9BD5, 0x9BD1, 0x9BD6, 0x9BD7,
    0x9BCF, 0x9BD8, 0x9BD9, 0x9BDB, 0x9BDC, 0x9BDA, 0x9BD0, 0x9BDE, 0x9BDF, 0xC349,
    0xC34D, 0xC34E, 0xC34C, 0xC34B, 0xC34A, 0xC343, 0xC345, 0xC346, 0xC347, 0xC348,
    0x9CDE, 0x5DE1, 0x5DDF, 0x5DE3, 0x5DE5, 0x5DE4, 0x5DE6, 0x5D51, 0x0951,
];

/// The gump picture of a buff icon. None for an icon with no picture.
pub fn gump_of(icon: u16) -> Option<u16> {
    let index = if icon >= FIRST_NEWER_ICON {
        icon - FIRST_NEWER_ICON + NEWER_ICONS_AT
    } else {
        icon.checked_sub(FIRST_ICON)?
    };
    BUFF_GUMPS
        .get(usize::from(index))
        .copied()
        .filter(|gump| *gump != NO_PICTURE)
}

/// The time left in words: "45s", "3m", "1h". Empty for a buff with no end.
pub fn time_words(buff: &WatchBuff) -> String {
    match buff.remaining_secs {
        None => String::new(),
        Some(secs) if secs >= SECONDS_PER_HOUR => format!("{}h", secs / SECONDS_PER_HOUR),
        Some(secs) if secs >= SECONDS_PER_MINUTE => format!("{}m", secs / SECONDS_PER_MINUTE),
        Some(secs) => format!("{secs}s"),
    }
}

/// True when the buff is about to end.
pub fn is_ending(buff: &WatchBuff) -> bool {
    buff.remaining_secs
        .is_some_and(|secs| secs < ENDING_SECONDS)
}

/// The buffs, those that end first on the left and those with no end last.
pub fn in_order(buffs: &[WatchBuff]) -> Vec<&WatchBuff> {
    let mut ordered: Vec<&WatchBuff> = buffs.iter().collect();
    ordered.sort_by_key(|buff| {
        (
            buff.remaining_secs.is_none(),
            buff.remaining_secs,
            buff.icon,
        )
    });
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    const NIGHT_SIGHT: u16 = 0x03ED;

    fn buff(icon: u16, remaining_secs: Option<u64>) -> WatchBuff {
        WatchBuff {
            icon,
            remaining_secs,
            ..WatchBuff::default()
        }
    }

    #[test]
    fn each_icon_has_its_picture_and_a_gap_has_none() {
        assert_eq!(gump_of(FIRST_ICON), Some(0x754C));
        assert_eq!(gump_of(NIGHT_SIGHT), Some(0x755E));
        assert_eq!(gump_of(FIRST_ICON + 2), None, "no icon 0x3EB");
        assert_eq!(
            gump_of(FIRST_NEWER_ICON),
            Some(BUFF_GUMPS[usize::from(NEWER_ICONS_AT)])
        );
        assert_eq!(gump_of(0x0001), None);
        assert_eq!(gump_of(u16::MAX), None);
    }

    #[test]
    fn time_left_reads_short_and_the_ending_come_first() {
        assert_eq!(time_words(&buff(1, Some(45))), "45s");
        assert_eq!(time_words(&buff(1, Some(200))), "3m");
        assert_eq!(time_words(&buff(1, Some(7300))), "2h");
        assert_eq!(time_words(&buff(1, None)), "");
        assert!(is_ending(&buff(1, Some(3))) && !is_ending(&buff(1, None)));
        let buffs = [buff(1, None), buff(2, Some(90)), buff(3, Some(5))];
        let icons: Vec<u16> = in_order(&buffs).iter().map(|b| b.icon).collect();
        assert_eq!(icons, vec![3, 2, 1]);
    }
}
