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

pub use decode::{
    parse, parse_with_version, BookPage, BuffEntry, CharacterSlot, ContainerItem, ContextMenuEntry,
    EquipItem, GroundItem, HealthBarStatus, Inbound, MenuEntry, MobileView, ObjectProperty,
    OpenGump, PartyEvent, PromptRequest, SecureTrade, ServerEntry, SkillEntry, SpeechLine,
    StatusExtra, TargetCursor, TextEntryDialog, VendorBuyEntry, VendorSellEntry,
};
pub use error::{ProtocolError, Result};
pub use frame::{compress_packet, FrameDecoder, GameDecoder, RawPacket};
pub use huffman::{Huffman, HuffmanDecoder};
pub use lengths::{PacketTable, VARIABLE_LEN};
pub use types::*;
