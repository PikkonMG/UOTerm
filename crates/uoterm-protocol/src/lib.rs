//! Protocol stack for UOTerm: framing, Huffman, codecs, era tables.

pub mod buf;
pub mod crypto;
pub mod decode;
pub mod encode;
pub mod error;
pub mod frame;
pub mod huffman;
pub mod lengths;
pub mod types;

pub use decode::character_refusal;
pub use decode::{
    parse, parse_with_version, Affix, BoatRider, BookPage, BuffEntry, BulletinEvent, CharacterSlot,
    ChatEvent, ContainerItem, ContextMenuEntry, CustomHouse, DisplayMap, EquipAttribute, EquipInfo,
    EquipItem, GraphicEffect, GroundItem, HealthBarStatus, HousePlane, Inbound, LiveMapDefinition,
    LiveMapEvent, MapChange, MapPatchCount, MemberPosition, MenuEntry, MobileView, ObjectProperty,
    OpenGump, PartyEvent, PromptRequest, SecureTrade, ServerEntry, SkillEntry, SpeechLine,
    StartTown, StatusExtra, TargetCursor, TextEntryDialog, TownPlace, VendorBuyEntry,
    VendorSellEntry, EFFECT_AT_PLACE, EFFECT_LIGHTNING, EFFECT_MOVING, EFFECT_ON_MOBILE,
    EQUIP_NO_CHARGES, LIVE_LAND_BYTES,
};
pub use encode::{HouseEdit, NewCharacter, NewLooks};
pub use error::{ProtocolError, Result};
pub use frame::{compress_packet, FrameDecoder, GameDecoder, RawPacket};
pub use huffman::{Huffman, HuffmanDecoder};
pub use lengths::{PacketTable, VARIABLE_LEN};
pub use types::*;
