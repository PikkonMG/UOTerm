//! Packet length tables, sized by client version.
//!
//! Bit 0x8000 marks a self-describing packet: bytes 1..2 are a big-endian
//! total length that includes the command byte. Length 0 is unknown.

use std::collections::HashMap;

use crate::error::{ProtocolError, Result};
use crate::types::{
    ClientVersion, Era, VARIABLE_LEN_FLAG, WORLD_ITEM_SA_LEN, WORLD_ITEM_SA_LEN_PRE_HIGH_SEAS,
};

pub use crate::types::VARIABLE_LEN_FLAG as VARIABLE_LEN;

const LEN_DROP: u16 = 14;
const LEN_DROP_WITH_GRID: u16 = 15;
const LEN_DAMAGE_OLD: u16 = 0x010A;
const LEN_DAMAGE: u16 = 7;
const LEN_ONE_BYTE: u16 = 1;
const LEN_CONTAINER: u16 = 9;
const LEN_ADD_ITEM_OLD: u16 = 20;
const LEN_ADD_ITEM: u16 = 21;
const LEN_MULTI_PLACEMENT: u16 = 30;
const LEN_FEATURES_U16: u16 = 3;
const LEN_FEATURES_U32: u16 = 5;
const LEN_QUEST_ARROW: u16 = 10;
const LEN_LOGOUT: u16 = 2;
const LEN_MOBILE_UPDATE_NEW: u16 = 25;
const LEN_CLIENT_INFO: u16 = 0x010C;
const LEN_OPL_INFO: u16 = 9;
const LEN_NEW_ANIMATION: u16 = 10;
const LEN_KR_E1_OLD: u16 = 9;
const LEN_KR_E3_OLD: u16 = 0x4D;
const LEN_KR_E6: u16 = 5;
const LEN_KR_E7: u16 = 12;
const LEN_KR_E8: u16 = 13;
const LEN_KR_E9: u16 = 75;
const LEN_KR_EA: u16 = 3;
const LEN_EE: u16 = 10;
const LEN_SEED: u16 = 21;
const LEN_TIME_SYNC: u16 = 9;
/// Classic Client 7.0.9.0+ / 7.0.116 SA world-item size.
/// 7.0.0.0–7.0.8.x used 24. The framer and the decoder must read the same
/// width, so both sizes come from the one pair of constants the decoder uses.
const LEN_WORLD_ITEM_SA: u16 = WORLD_ITEM_SA_LEN as u16;
const LEN_TIME_SYNC_RESP: u16 = 25;
const LEN_NEW_MAP: u16 = 21;
const LEN_CREATE_CHAR_OLD: u16 = 0x68;
const LEN_CREATE_CHAR_70180: u16 = 0x6A;
const LEN_CREATE_CHAR_70160: u16 = 106;
const LEN_STORE_OPEN: u16 = 1;
const LEN_PUBLIC_HOUSE: u16 = 2;
const LEN_D5: u16 = 9;
const LEN_FD: u16 = 2;
const LEN_ASSISTANT_HANDSHAKE: u16 = 8;
const LEN_KR_ACCOUNT_LOGIN: u16 = 78;
const LEN_CD_UNKNOWN: u16 = 1;
const LEN_CONTAINER_PRE_HIGH_SEAS: u16 = 7;
const LEN_MULTI_PLACEMENT_PRE_HIGH_SEAS: u16 = 26;
const LEN_QUEST_ARROW_PRE_HIGH_SEAS: u16 = 6;
/// `0xF3` without the trailing word High Seas added.
const LEN_WORLD_ITEM_SA_PRE_HIGH_SEAS: u16 = WORLD_ITEM_SA_LEN_PRE_HIGH_SEAS as u16;

const V_500A: ClientVersion = ClientVersion::new(5, 0, 0, b'a' as u32);
const V_5090: ClientVersion = ClientVersion::new(5, 0, 9, 0);
const V_6013: ClientVersion = ClientVersion::new(6, 0, 1, 3);
const V_6017: ClientVersion = ClientVersion::new(6, 0, 1, 7);
const V_60142: ClientVersion = ClientVersion::new(6, 0, 14, 2);
const V_7000: ClientVersion = ClientVersion::new(7, 0, 0, 0);
const V_7090: ClientVersion = ClientVersion::new(7, 0, 9, 0);
const V_70180: ClientVersion = ClientVersion::new(7, 0, 18, 0);
const V_70640: ClientVersion = ClientVersion::new(7, 0, 64, 0);
const V_701040: ClientVersion = ClientVersion::new(7, 0, 104, 0);

/// Interprets one slot from a 256-entry era table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketLen {
    Fixed(u16),
    Variable,
    Unknown,
}

impl PacketLen {
    pub fn from_table(entry: u16) -> Self {
        if entry == 0 {
            Self::Unknown
        } else if entry & VARIABLE_LEN_FLAG != 0 {
            Self::Variable
        } else {
            Self::Fixed(entry)
        }
    }
}

/// 256-slot length table for one era, with optional YAML/JSON overlays.
#[derive(Clone, Debug)]
pub struct PacketTable {
    lengths: [u16; 256],
    era: Era,
}

impl PacketTable {
    /// Table for one era at the version that era defaults to.
    pub fn for_era(era: Era) -> Self {
        Self::for_version(era, era.default_version())
    }

    /// Table for the client version the session speaks.
    ///
    /// A packet's size depends on the client version, not on the era: the
    /// reference client starts from one table and moves each slot at the
    /// version that changed it. A server writes the form that matches the
    /// version the client reported, so framing a stream by any other size
    /// leaves bytes behind and the stream then desynchronises. The era is
    /// kept for the messages that name it.
    pub fn for_version(era: Era, version: ClientVersion) -> Self {
        let mut lengths = T2A_LENGTHS;
        for &(id, len) in LATER_SLOTS {
            lengths[id as usize] = len;
        }
        for gate in VERSION_GATES {
            lengths[gate.id as usize] = if version.at_least(gate.first) {
                gate.from
            } else {
                gate.before
            };
        }
        Self { lengths, era }
    }

    pub fn era(&self) -> Era {
        self.era
    }

    pub fn t2a() -> Self {
        Self::for_era(Era::T2a)
    }

    pub fn modern() -> Self {
        Self::for_era(Era::Modern)
    }

    pub fn get(&self, id: u8) -> u16 {
        self.lengths[id as usize]
    }

    pub fn is_variable(&self, id: u8) -> bool {
        self.lengths[id as usize] & VARIABLE_LEN_FLAG != 0
    }

    pub fn is_known(&self, id: u8) -> bool {
        self.lengths[id as usize] != 0
    }

    pub fn fixed_len(&self, id: u8) -> Option<u16> {
        let v = self.lengths[id as usize];
        if v == 0 || v & VARIABLE_LEN_FLAG != 0 {
            None
        } else {
            Some(v)
        }
    }

    /// Merge operator overlays. Keys may be decimal or `0xNN`.
    pub fn with_overrides(mut self, map: &HashMap<String, u16>) -> Result<Self> {
        for (key, value) in map {
            let id = parse_packet_id(key)?;
            self.lengths[id as usize] = *value;
        }
        Ok(self)
    }
}

fn parse_packet_id(key: &str) -> Result<u8> {
    crate::types::parse_unsigned(key)
        .and_then(|id| u8::try_from(id).ok())
        .ok_or_else(|| ProtocolError::message(format!("bad packet id {key}")))
}

/// T2A 2.0.7 `g_PacketLengthTable`. 0x8000 = variable. One change: `0xF0`
/// is marked variable, because shards of every era can send the assistant
/// handshake.
pub const T2A_LENGTHS: [u16; 256] = [
    0x0068, 0x0005, 0x0007, 0x8000, 0x0002, 0x0005, 0x0005, 0x0007, 0x000E, 0x0005, 0x000B, 0x010A,
    0x8000, 0x0003, 0x8000, 0x003D, 0x00D7, 0x8000, 0x8000, 0x000A, 0x0006, 0x0009, 0x0001, 0x8000,
    0x8000, 0x8000, 0x8000, 0x0025, 0x8000, 0x0005, 0x0004, 0x0008, 0x0013, 0x0008, 0x0003, 0x001A,
    0x0007, 0x0014, 0x0005, 0x0002, 0x0005, 0x0001, 0x0005, 0x0002, 0x0002, 0x0011, 0x000F, 0x000A,
    0x0005, 0x0001, 0x0002, 0x0002, 0x000A, 0x028D, 0x8000, 0x0008, 0x0007, 0x0009, 0x8000, 0x8000,
    0x8000, 0x0002, 0x0025, 0x8000, 0x00C9, 0x8000, 0x8000, 0x0229, 0x02C9, 0x0005, 0x8000, 0x000B,
    0x0049, 0x005D, 0x0005, 0x0009, 0x8000, 0x8000, 0x0006, 0x0002, 0x8000, 0x8000, 0x8000, 0x0002,
    0x000C, 0x0001, 0x000B, 0x006E, 0x006A, 0x8000, 0x8000, 0x0004, 0x0002, 0x0049, 0x8000, 0x0031,
    0x0005, 0x0009, 0x000F, 0x000D, 0x0001, 0x0004, 0x8000, 0x0015, 0x8000, 0x8000, 0x0003, 0x0009,
    0x0013, 0x0003, 0x000E, 0x8000, 0x001C, 0x8000, 0x0005, 0x0002, 0x8000, 0x0023, 0x0010, 0x0011,
    0x8000, 0x0009, 0x8000, 0x0002, 0x8000, 0x000D, 0x0002, 0x8000, 0x003E, 0x8000, 0x0002, 0x0027,
    0x0045, 0x0002, 0x8000, 0x8000, 0x0042, 0x8000, 0x8000, 0x8000, 0x000B, 0x8000, 0x8000, 0x8000,
    0x0013, 0x0041, 0x8000, 0x0063, 0x8000, 0x0009, 0x8000, 0x0002, 0x8000, 0x001A, 0x8000, 0x0102,
    0x0135, 0x0033, 0x8000, 0x8000, 0x0003, 0x0009, 0x0009, 0x0009, 0x0095, 0x8000, 0x8000, 0x0004,
    0x8000, 0x8000, 0x0005, 0x8000, 0x8000, 0x8000, 0x8000, 0x000D, 0x8000, 0x8000, 0x8000, 0x8000,
    0x8000, 0x0040, 0x0009, 0x8000, 0x8000, 0x0003, 0x0006, 0x0009, 0x0003, 0x8000, 0x8000, 0x8000,
    0x0024, 0x8000, 0x8000, 0x8000, 0x0006, 0x00CB, 0x0001, 0x0031, 0x0002, 0x0006, 0x0006, 0x0007,
    0x8000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x8000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000,
];

/// Slots the 2.0.7 table leaves empty that later clients frame at one size
/// in every version. `0xFE` is the assistant handshake a shard sends at eight
/// bytes; the reference client leaves it self-describing, but the server
/// writes a fixed packet.
const LATER_SLOTS: &[(u8, u16)] = &[
    (0xCD, LEN_CD_UNKNOWN),
    (0xCE, VARIABLE_LEN_FLAG),
    (0xCF, LEN_KR_ACCOUNT_LOGIN),
    (0xD0, VARIABLE_LEN_FLAG),
    (0xD1, LEN_LOGOUT),
    (0xD2, LEN_MOBILE_UPDATE_NEW),
    (0xD3, VARIABLE_LEN_FLAG),
    (0xD4, VARIABLE_LEN_FLAG),
    (0xD6, VARIABLE_LEN_FLAG),
    (0xD7, VARIABLE_LEN_FLAG),
    (0xD8, VARIABLE_LEN_FLAG),
    (0xD9, LEN_CLIENT_INFO),
    (0xDA, VARIABLE_LEN_FLAG),
    (0xDB, VARIABLE_LEN_FLAG),
    (0xDC, LEN_OPL_INFO),
    (0xDD, VARIABLE_LEN_FLAG),
    (0xDE, VARIABLE_LEN_FLAG),
    (0xDF, VARIABLE_LEN_FLAG),
    (0xE0, VARIABLE_LEN_FLAG),
    (0xE2, LEN_NEW_ANIMATION),
    (0xE4, VARIABLE_LEN_FLAG),
    (0xE5, VARIABLE_LEN_FLAG),
    (0xEB, VARIABLE_LEN_FLAG),
    (0xEC, VARIABLE_LEN_FLAG),
    (0xED, VARIABLE_LEN_FLAG),
    (0xEF, LEN_SEED),
    (0xF0, VARIABLE_LEN_FLAG),
    (0xF4, VARIABLE_LEN_FLAG),
    (0xF5, LEN_NEW_MAP),
    (0xF6, VARIABLE_LEN_FLAG),
    (0xF7, VARIABLE_LEN_FLAG),
    (0xF8, LEN_CREATE_CHAR_70160),
    (0xF9, VARIABLE_LEN_FLAG),
    (0xFC, VARIABLE_LEN_FLAG),
    (0xFE, LEN_ASSISTANT_HANDSHAKE),
];

/// One slot the reference client sizes by version: `before` below `first`,
/// `from` at `first` and up.
struct VersionGate {
    first: ClientVersion,
    id: u8,
    before: u16,
    from: u16,
}

const fn gate(first: ClientVersion, id: u8, before: u16, from: u16) -> VersionGate {
    VersionGate {
        first,
        id,
        before,
        from,
    }
}

/// Every slot the reference client moves by version, with the version that
/// moved it.
const VERSION_GATES: &[VersionGate] = &[
    gate(V_500A, 0x0B, LEN_DAMAGE_OLD, LEN_DAMAGE),
    gate(V_500A, 0x16, LEN_ONE_BYTE, VARIABLE_LEN_FLAG),
    gate(V_500A, 0x31, LEN_ONE_BYTE, VARIABLE_LEN_FLAG),
    gate(V_5090, 0xE1, LEN_KR_E1_OLD, VARIABLE_LEN_FLAG),
    gate(V_6013, 0xE3, LEN_KR_E3_OLD, VARIABLE_LEN_FLAG),
    gate(V_6013, 0xE6, VARIABLE_LEN_FLAG, LEN_KR_E6),
    gate(V_6013, 0xE7, VARIABLE_LEN_FLAG, LEN_KR_E7),
    gate(V_6013, 0xE8, VARIABLE_LEN_FLAG, LEN_KR_E8),
    gate(V_6013, 0xE9, VARIABLE_LEN_FLAG, LEN_KR_E9),
    gate(V_6013, 0xEA, VARIABLE_LEN_FLAG, LEN_KR_EA),
    gate(V_6017, 0x08, LEN_DROP, LEN_DROP_WITH_GRID),
    gate(V_6017, 0x25, LEN_ADD_ITEM_OLD, LEN_ADD_ITEM),
    gate(V_60142, 0xB9, LEN_FEATURES_U16, LEN_FEATURES_U32),
    gate(V_7000, 0xEE, VARIABLE_LEN_FLAG, LEN_EE),
    gate(V_7090, 0x24, LEN_CONTAINER_PRE_HIGH_SEAS, LEN_CONTAINER),
    gate(
        V_7090,
        0x99,
        LEN_MULTI_PLACEMENT_PRE_HIGH_SEAS,
        LEN_MULTI_PLACEMENT,
    ),
    gate(V_7090, 0xBA, LEN_QUEST_ARROW_PRE_HIGH_SEAS, LEN_QUEST_ARROW),
    gate(
        V_7090,
        0xF3,
        LEN_WORLD_ITEM_SA_PRE_HIGH_SEAS,
        LEN_WORLD_ITEM_SA,
    ),
    gate(V_7090, 0xF1, VARIABLE_LEN_FLAG, LEN_TIME_SYNC),
    gate(V_7090, 0xF2, VARIABLE_LEN_FLAG, LEN_TIME_SYNC_RESP),
    gate(V_70180, 0x00, LEN_CREATE_CHAR_OLD, LEN_CREATE_CHAR_70180),
    gate(V_70640, 0xFA, VARIABLE_LEN_FLAG, LEN_STORE_OPEN),
    gate(V_70640, 0xFB, VARIABLE_LEN_FLAG, LEN_PUBLIC_HOUSE),
    gate(V_701040, 0xD5, VARIABLE_LEN_FLAG, LEN_D5),
    gate(V_701040, 0xFD, VARIABLE_LEN_FLAG, LEN_FD),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        PKT_ASSISTANT, PKT_BATCH_QUERY_PROPERTIES, PKT_BOOK_CONTENT, PKT_BOOK_HEADER,
        PKT_BOOK_HEADER_OLD, PKT_BUFF_DEBUFF, PKT_CHARACTER_ANIMATION, PKT_COMBATANT, PKT_EXTENDED,
        PKT_HEALTH_BAR_STATUS, PKT_LIFT_REJECT, PKT_MENU_RESPONSE, PKT_MUSIC, PKT_OPEN_MENU,
        PKT_OPL_INFO, PKT_SECURE_TRADE, PKT_SINGLE_CLICK, PKT_SKILLS, PKT_SOUND_EFFECT, PKT_SWING,
        PKT_UPDATE_STAM, PKT_VENDOR_BUY, PKT_VENDOR_BUY_LIST, PKT_VENDOR_SELL,
        PKT_VENDOR_SELL_LIST,
    };

    #[test]
    fn the_assistant_handshake_is_variable_in_every_era() {
        assert!(PacketTable::t2a().is_variable(PKT_ASSISTANT));
        assert!(PacketTable::modern().is_variable(PKT_ASSISTANT));
    }

    #[test]
    fn t2a_move_is_seven() {
        let t = PacketTable::t2a();
        assert_eq!(t.fixed_len(0x02), Some(7));
        assert!(t.is_variable(0x03));
        assert_eq!(t.fixed_len(0x80), Some(0x3E));
        assert_eq!(t.fixed_len(0x1B), Some(0x25));
    }

    #[test]
    fn t2a_does_not_use_modern_sizes() {
        let t = PacketTable::t2a();
        assert_eq!(t.fixed_len(0x08), Some(14));
        assert_eq!(t.fixed_len(0x0B), Some(0x010A));
        assert_eq!(t.fixed_len(0x24), Some(7));
        assert_eq!(t.fixed_len(0x25), Some(20));
        assert_eq!(t.fixed_len(0xB9), Some(3));
        assert!(t.is_variable(0xD6));
        assert_eq!(t.fixed_len(0xDC), Some(LEN_OPL_INFO));
        assert!(t.is_variable(0xDD));
        assert!(t.is_variable(0xDF));
        assert_eq!(t.fixed_len(0xF3), Some(LEN_WORLD_ITEM_SA_PRE_HIGH_SEAS));
    }

    #[test]
    fn modern_features_is_five() {
        let t = PacketTable::modern();
        assert_eq!(t.fixed_len(0xB9), Some(LEN_FEATURES_U32));
        assert!(t.is_variable(0xD6));
        assert_eq!(t.fixed_len(0xF3), Some(LEN_WORLD_ITEM_SA));
        assert_eq!(t.fixed_len(0x08), Some(LEN_DROP_WITH_GRID));
        assert_eq!(t.fixed_len(0xEF), Some(LEN_SEED));
        assert_eq!(t.fixed_len(0xDC), Some(LEN_OPL_INFO));
    }

    #[test]
    fn modern_overrides_match_table() {
        let t = PacketTable::modern();
        assert!(!LATER_SLOTS.is_empty());
        for &(id, len) in LATER_SLOTS {
            assert_eq!(t.get(id), len, "modern 0x{id:02X}");
            match PacketLen::from_table(len) {
                PacketLen::Fixed(fixed) => {
                    assert_eq!(t.fixed_len(id), Some(fixed), "fixed 0x{id:02X}");
                    assert!(!t.is_variable(id), "not variable 0x{id:02X}");
                }
                PacketLen::Variable => {
                    assert!(t.is_variable(id), "variable 0x{id:02X}");
                    assert_eq!(t.fixed_len(id), None, "no fixed 0x{id:02X}");
                }
                PacketLen::Unknown => panic!("override 0x{id:02X} must not be unknown"),
            }
        }
    }

    #[test]
    fn modern_known_overrides_are_correct() {
        let t = PacketTable::modern();
        assert_eq!(t.fixed_len(0x08), Some(15));
        assert_eq!(t.fixed_len(0x0B), Some(7));
        assert_eq!(t.fixed_len(0x24), Some(9));
        assert_eq!(t.fixed_len(0x25), Some(21));
        assert_eq!(t.fixed_len(0xB9), Some(5));
        assert_eq!(t.fixed_len(0xD1), Some(2));
        assert!(t.is_variable(0xD6));
        assert_eq!(t.get(0xD6), VARIABLE_LEN_FLAG);
        assert_eq!(t.fixed_len(0xDC), Some(9));
        assert!(t.is_variable(0xDD));
        assert_eq!(t.fixed_len(0xEF), Some(21));
        assert!(t.is_variable(0xF0));
        assert_eq!(t.fixed_len(0xF1), Some(LEN_TIME_SYNC));
        assert_eq!(t.fixed_len(0xF3), Some(26));
    }

    #[test]
    fn modern_login_world_dump_lengths() {
        let t = PacketTable::modern();
        assert_eq!(t.fixed_len(0x1B), Some(37));
        assert_eq!(t.fixed_len(0x53), Some(2));
        assert_eq!(t.fixed_len(0x55), Some(1));
        assert_eq!(t.fixed_len(0x8C), Some(11));
        assert!(t.is_variable(0x78));
        assert_eq!(t.get(0x78), VARIABLE_LEN_FLAG);
        assert!(t.is_variable(0xA9));
        assert!(t.is_variable(0xBD));
        assert!(t.is_variable(0xBF));
        assert!(t.is_variable(0xE1));
        assert_eq!(t.fixed_len(0xD9), Some(LEN_CLIENT_INFO));
        assert!(t.is_variable(0x16));
        assert_eq!(t.fixed_len(0xBA), Some(LEN_QUEST_ARROW));
        assert_eq!(t.fixed_len(0x99), Some(LEN_MULTI_PLACEMENT));
        assert!(t.is_variable(0xDF));
        assert_eq!(t.fixed_len(0xE2), Some(LEN_NEW_ANIMATION));
        assert_eq!(t.fixed_len(0xF5), Some(LEN_NEW_MAP));
        assert_eq!(t.fixed_len(0xF8), Some(LEN_CREATE_CHAR_70160));
    }

    #[test]
    fn combat_and_lift_packets_use_named_ids() {
        const LEN_COMBATANT: u16 = 5;
        const LEN_LIFT_REJECT: u16 = 2;
        const LEN_SWING: u16 = 10;
        for table in [PacketTable::t2a(), PacketTable::modern()] {
            let era = table.era();
            assert_eq!(
                table.fixed_len(PKT_COMBATANT),
                Some(LEN_COMBATANT),
                "0xAA {era}"
            );
            assert_eq!(
                table.fixed_len(PKT_LIFT_REJECT),
                Some(LEN_LIFT_REJECT),
                "0x27 {era}"
            );
            assert_eq!(table.fixed_len(PKT_SWING), Some(LEN_SWING), "0x2F {era}");
        }
    }

    #[test]
    fn object_property_packets_use_named_ids() {
        const LEN_SINGLE_CLICK: u16 = 5;
        let t = PacketTable::modern();
        assert_eq!(t.fixed_len(PKT_SINGLE_CLICK), Some(LEN_SINGLE_CLICK));
        assert_eq!(t.get(PKT_BATCH_QUERY_PROPERTIES), VARIABLE_LEN_FLAG);
        assert!(t.is_variable(PKT_BATCH_QUERY_PROPERTIES));
        assert_eq!(t.fixed_len(PKT_OPL_INFO), Some(LEN_OPL_INFO));
    }

    #[test]
    fn status_and_effect_packets_use_named_ids() {
        const LEN_SOUND_EFFECT: u16 = 12;
        const LEN_MUSIC: u16 = 3;
        const LEN_CHARACTER_ANIMATION: u16 = 14;
        const LEN_UPDATE_STAM: u16 = 9;
        for table in [PacketTable::t2a(), PacketTable::modern()] {
            let era = table.era();
            assert!(table.is_variable(PKT_HEALTH_BAR_STATUS), "0x17 {era}");
            assert_eq!(
                table.fixed_len(PKT_SOUND_EFFECT),
                Some(LEN_SOUND_EFFECT),
                "0x54 {era}"
            );
            assert_eq!(table.fixed_len(PKT_MUSIC), Some(LEN_MUSIC), "0x6D {era}");
            assert_eq!(
                table.fixed_len(PKT_CHARACTER_ANIMATION),
                Some(LEN_CHARACTER_ANIMATION),
                "0x6E {era}"
            );
            assert_eq!(
                table.fixed_len(PKT_UPDATE_STAM),
                Some(LEN_UPDATE_STAM),
                "0xA3 {era}"
            );
        }
        assert!(PacketTable::modern().is_variable(PKT_BUFF_DEBUFF));
    }

    /// Every packet a shopkeeper needs is self-describing on both eras, so a
    /// shard that predates the modern table still frames them.
    #[test]
    fn vendor_packets_use_named_ids() {
        for table in [PacketTable::t2a(), PacketTable::modern()] {
            let era = table.era();
            for id in [
                PKT_VENDOR_BUY,
                PKT_VENDOR_BUY_LIST,
                PKT_VENDOR_SELL,
                PKT_VENDOR_SELL_LIST,
            ] {
                assert_eq!(table.get(id), VARIABLE_LEN_FLAG, "0x{id:02X} {era}");
                assert_eq!(table.fixed_len(id), None, "0x{id:02X} {era}");
            }
        }
    }

    /// The old-style menu is framed, its answer is a fixed thirteen bytes, and
    /// context menus ride on the framed extended packet.
    #[test]
    fn menu_packets_use_named_ids() {
        const LEN_MENU_RESPONSE: u16 = 13;
        for table in [PacketTable::t2a(), PacketTable::modern()] {
            let era = table.era();
            assert!(table.is_variable(PKT_OPEN_MENU), "0x7C {era}");
            assert_eq!(
                table.fixed_len(PKT_MENU_RESPONSE),
                Some(LEN_MENU_RESPONSE),
                "0x7D {era}"
            );
            assert!(table.is_variable(PKT_EXTENDED), "0xBF {era}");
        }
    }

    /// Skills and secure trades are framed on both eras. The old book cover is
    /// a fixed ninety-nine bytes; the counted one is framed on both.
    #[test]
    fn skill_trade_and_book_packets_use_named_ids() {
        const LEN_BOOK_HEADER_OLD: u16 = 99;
        for table in [PacketTable::t2a(), PacketTable::modern()] {
            let era = table.era();
            assert!(table.is_variable(PKT_SKILLS), "0x3A {era}");
            assert!(table.is_variable(PKT_SECURE_TRADE), "0x6F {era}");
            assert!(table.is_variable(PKT_BOOK_CONTENT), "0x66 {era}");
            assert_eq!(
                table.fixed_len(PKT_BOOK_HEADER_OLD),
                Some(LEN_BOOK_HEADER_OLD),
                "0x93 {era}"
            );
        }
        assert!(PacketTable::modern().is_variable(PKT_BOOK_HEADER));
        assert!(PacketTable::t2a().is_variable(PKT_BOOK_HEADER));
    }

    #[test]
    fn packet_len_from_table() {
        assert_eq!(PacketLen::from_table(0), PacketLen::Unknown);
        assert_eq!(PacketLen::from_table(21), PacketLen::Fixed(21));
        assert_eq!(
            PacketLen::from_table(VARIABLE_LEN_FLAG),
            PacketLen::Variable
        );
    }

    #[test]
    fn modern_unknown_ids() {
        let t = PacketTable::modern();
        let unknown: Vec<u8> = (0u8..=255).filter(|&id| !t.is_known(id)).collect();
        assert_eq!(unknown, vec![0xFF], "only 0xFF stays unknown on modern");
    }

    /// A server writes `0xF3` two bytes shorter to a session it holds as
    /// Stygian Abyss but not High Seas, and the reference client sizes the
    /// same four packets by that one 7.0.9.0 boundary.
    #[test]
    fn high_seas_gate_picks_the_world_item_size() {
        const VERSION_PRE_HIGH_SEAS: &str = "7.0.8.2";
        const VERSION_HIGH_SEAS: &str = "7.0.9.0";
        let older: ClientVersion = VERSION_PRE_HIGH_SEAS.parse().unwrap();
        let newer: ClientVersion = VERSION_HIGH_SEAS.parse().unwrap();
        assert!(!older.has_high_seas());
        assert!(newer.has_high_seas());

        let pre = PacketTable::for_version(Era::Modern, older);
        assert_eq!(
            pre.fixed_len(0xF3),
            Some(LEN_WORLD_ITEM_SA_PRE_HIGH_SEAS),
            "0xF3 below 7.0.9.0"
        );
        assert_eq!(pre.fixed_len(0x24), Some(LEN_CONTAINER_PRE_HIGH_SEAS));
        assert_eq!(pre.fixed_len(0x99), Some(LEN_MULTI_PLACEMENT_PRE_HIGH_SEAS));
        assert_eq!(pre.fixed_len(0xBA), Some(LEN_QUEST_ARROW_PRE_HIGH_SEAS));

        let hs = PacketTable::for_version(Era::Modern, newer);
        assert_eq!(hs.fixed_len(0xF3), Some(LEN_WORLD_ITEM_SA));
        assert_eq!(hs.fixed_len(0x24), Some(LEN_CONTAINER));
        assert_eq!(hs.fixed_len(0x99), Some(LEN_MULTI_PLACEMENT));
        assert_eq!(hs.fixed_len(0xBA), Some(LEN_QUEST_ARROW));
    }

    /// The High Seas gate moves six slots and nothing else, and both era
    /// defaults equal the table of their own version.
    #[test]
    fn high_seas_gate_leaves_every_other_slot_alone() {
        const VERSION_PRE_HIGH_SEAS: &str = "7.0.8.2";
        const VERSION_HIGH_SEAS: &str = "7.0.9.0";
        let older: ClientVersion = VERSION_PRE_HIGH_SEAS.parse().unwrap();
        let newer: ClientVersion = VERSION_HIGH_SEAS.parse().unwrap();
        let pre = PacketTable::for_version(Era::Modern, older);
        let hs = PacketTable::for_version(Era::Modern, newer);
        let gated: Vec<u8> = (0u8..=255)
            .filter(|&id| pre.get(id) != hs.get(id))
            .collect();
        assert_eq!(gated, vec![0x24, 0x99, 0xBA, 0xF1, 0xF2, 0xF3]);

        for era in [Era::T2a, Era::Modern] {
            let by_era = PacketTable::for_era(era);
            let by_version = PacketTable::for_version(era, era.default_version());
            for id in 0u8..=255 {
                assert_eq!(by_era.get(id), by_version.get(id), "0x{id:02X} {era}");
            }
        }
    }

    /// The version, not the era, sizes a packet: an old client on the modern
    /// era gets the old sizes, and a newer client on the T2A era the new ones.
    #[test]
    fn the_version_sizes_packets_in_both_eras() {
        let v4: ClientVersion = "4.0.11".parse().unwrap();
        let v5: ClientVersion = "5.0.9".parse().unwrap();
        let v6: ClientVersion = "6.0.1.3".parse().unwrap();
        let v6_grid: ClientVersion = "6.0.1.7".parse().unwrap();
        let v6_features: ClientVersion = "6.0.14.2".parse().unwrap();
        for era in [Era::T2a, Era::Modern] {
            let old = PacketTable::for_version(era, v4);
            assert_eq!(old.fixed_len(0x0B), Some(LEN_DAMAGE_OLD), "{era}");
            assert_eq!(old.fixed_len(0x16), Some(LEN_ONE_BYTE), "{era}");
            assert_eq!(old.fixed_len(0x31), Some(LEN_ONE_BYTE), "{era}");

            let five = PacketTable::for_version(era, v5);
            assert_eq!(five.fixed_len(0x0B), Some(LEN_DAMAGE), "{era}");
            assert!(five.is_variable(0x16), "{era}");
            assert!(five.is_variable(0x31), "{era}");

            let six = PacketTable::for_version(era, v6);
            assert_eq!(six.fixed_len(0x08), Some(LEN_DROP), "{era}");
            assert_eq!(six.fixed_len(0x25), Some(LEN_ADD_ITEM_OLD), "{era}");
            assert_eq!(six.fixed_len(0xB9), Some(LEN_FEATURES_U16), "{era}");

            let grid = PacketTable::for_version(era, v6_grid);
            assert_eq!(grid.fixed_len(0x08), Some(LEN_DROP_WITH_GRID), "{era}");
            assert_eq!(grid.fixed_len(0x25), Some(LEN_ADD_ITEM), "{era}");
            assert_eq!(grid.fixed_len(0xB9), Some(LEN_FEATURES_U16), "{era}");

            let features = PacketTable::for_version(era, v6_features);
            assert_eq!(features.fixed_len(0xB9), Some(LEN_FEATURES_U32), "{era}");
        }
    }

    /// Each gate names one slot, so no later gate can undo an earlier one.
    #[test]
    fn each_gate_names_its_own_slot() {
        let mut ids: Vec<u8> = VERSION_GATES.iter().map(|g| g.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), VERSION_GATES.len());
        for gate in VERSION_GATES {
            assert!(
                !LATER_SLOTS.iter().any(|&(id, _)| id == gate.id),
                "0x{:02X} is both gated and fixed",
                gate.id
            );
        }
    }

    #[test]
    fn yaml_override() {
        let mut map = HashMap::new();
        map.insert("0xB9".into(), 5);
        let t = PacketTable::t2a().with_overrides(&map).unwrap();
        assert_eq!(t.fixed_len(0xB9), Some(5));
    }
}
