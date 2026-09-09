//! Packet framing on a byte stream.

use crate::error::{ProtocolError, Result};
use crate::lengths::PacketTable;
use crate::types::{
    ClientVersion, MAX_PACKET_LEN, PKT_MOBILE_INCOMING, UNKNOWN_VAR_MAX, UNKNOWN_VAR_MIN,
    VARIABLE_LEN_FLAG,
};

const MOBILE_INCOMING_HEADER: usize = 17;
const EQUIP_SERIAL_LEN: usize = 4;
const EQUIP_GRAPHIC_LAYER: usize = 3;
const EQUIP_HUE_LEN: usize = 2;
const EQUIP_HUE_BIT: u16 = 0x8000;
const MOBILE_INCOMING_MAX: usize = 512;
const PKT_HEALTHBAR_EC: u8 = 0x16;
const HEALTHBAR_EC_HIDE_LEN: usize = 9;
const VARIABLE_LEN_MIN: usize = 3;

/// One framed packet: id plus full wire bytes (id included).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPacket {
    pub id: u8,
    pub bytes: Vec<u8>,
}

/// Incremental framer. Feed decompressed (or login-plain) bytes.
pub struct FrameDecoder {
    table: PacketTable,
    buf: Vec<u8>,
    version: ClientVersion,
}

impl FrameDecoder {
    pub fn new(table: PacketTable) -> Self {
        let version = table.era().default_version();
        Self::with_version(table, version)
    }

    pub fn with_version(table: PacketTable, version: ClientVersion) -> Self {
        Self {
            table,
            buf: Vec::new(),
            version,
        }
    }

    pub fn table(&self) -> &PacketTable {
        &self.table
    }

    pub fn reset(&mut self) {
        self.buf.clear();
    }

    pub fn push(&mut self, data: &[u8]) -> Result<Vec<RawPacket>> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        loop {
            match self.take_one() {
                Ok(Some(pkt)) => out.push(pkt),
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(error = %e, "frame skip one byte");
                    if self.buf.is_empty() {
                        break;
                    }
                    self.buf.drain(..1);
                }
            }
        }
        Ok(out)
    }

    fn take_one(&mut self) -> Result<Option<RawPacket>> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        let id = self.buf[0];
        let slot = self.table.get(id);
        let needed = if id == PKT_MOBILE_INCOMING && !self.version.has_prefixed_mobile_incoming() {
            match mobile_incoming_len(&self.buf) {
                None => return Ok(None),
                Some(n) => n,
            }
        } else if slot == 0 {
            match guess_unknown(&self.buf, id)? {
                None => return Ok(None),
                Some(n) => n,
            }
        } else if slot & VARIABLE_LEN_FLAG != 0 {
            if self.buf.len() < VARIABLE_LEN_MIN {
                return Ok(None);
            }
            let declared = u16::from_be_bytes([self.buf[1], self.buf[2]]) as usize;
            if id == PKT_HEALTHBAR_EC && declared < VARIABLE_LEN_MIN {
                if self.buf.len() < HEALTHBAR_EC_HIDE_LEN {
                    return Ok(None);
                }
                HEALTHBAR_EC_HIDE_LEN
            } else if !(VARIABLE_LEN_MIN..=MAX_PACKET_LEN).contains(&declared) {
                return Err(ProtocolError::InvalidLength {
                    id,
                    length: declared as u16,
                });
            } else {
                declared
            }
        } else {
            slot as usize
        };
        if needed == 0 {
            tracing::warn!(packet = id, "unknown packet with no length; skip one byte");
            self.buf.drain(..1);
            return Ok(Some(RawPacket {
                id,
                bytes: vec![id],
            }));
        }
        if self.buf.len() < needed {
            return Ok(None);
        }
        let bytes: Vec<u8> = self.buf.drain(..needed).collect();
        Ok(Some(RawPacket { id, bytes }))
    }
}

pub fn compress_packet(huffman: &crate::huffman::Huffman, packet: &[u8]) -> Vec<u8> {
    huffman.compress(packet)
}

/// Huffman plus length-table framing for the post-login game stream.
pub struct GameDecoder {
    huffman: crate::huffman::HuffmanDecoder,
    frame: FrameDecoder,
}

impl GameDecoder {
    pub fn new(table: PacketTable) -> Self {
        let version = table.era().default_version();
        Self::with_version(table, version)
    }

    pub fn with_version(table: PacketTable, version: ClientVersion) -> Self {
        let tree = crate::huffman::Huffman::new();
        Self {
            huffman: crate::huffman::HuffmanDecoder::new(tree),
            frame: FrameDecoder::with_version(table, version),
        }
    }

    pub fn push_plain(&mut self, data: &[u8]) -> Result<Vec<RawPacket>> {
        self.frame.push(data)
    }

    pub fn push_compressed(&mut self, data: &[u8]) -> Result<Vec<RawPacket>> {
        let decoded = self.huffman.push(data)?;
        self.frame.push(&decoded)
    }

    pub fn push(&mut self, data: &[u8]) -> Result<Vec<RawPacket>> {
        self.push_compressed(data)
    }

    pub fn reset(&mut self) {
        self.huffman.reset();
        self.frame.reset();
    }
}

fn capped_scan(buf_len: usize, pos: usize) -> Option<usize> {
    if buf_len >= MOBILE_INCOMING_MAX {
        Some(pos.max(MOBILE_INCOMING_HEADER))
    } else {
        None
    }
}

fn mobile_incoming_len(buf: &[u8]) -> Option<usize> {
    if buf.len() < MOBILE_INCOMING_HEADER + EQUIP_SERIAL_LEN {
        return None;
    }
    let mut i = MOBILE_INCOMING_HEADER;
    loop {
        if buf.len() < i + EQUIP_SERIAL_LEN {
            return capped_scan(buf.len(), i);
        }
        let serial = u32::from_be_bytes(buf[i..i + EQUIP_SERIAL_LEN].try_into().ok()?);
        i += EQUIP_SERIAL_LEN;
        if serial == 0 {
            return Some(i);
        }
        if buf.len() < i + EQUIP_GRAPHIC_LAYER {
            return capped_scan(buf.len(), i);
        }
        let graphic = u16::from_be_bytes([buf[i], buf[i + 1]]);
        i += EQUIP_GRAPHIC_LAYER;
        if graphic & EQUIP_HUE_BIT != 0 {
            if buf.len() < i + EQUIP_HUE_LEN {
                return capped_scan(buf.len(), i);
            }
            i += EQUIP_HUE_LEN;
        }
    }
}

fn guess_unknown(buf: &[u8], id: u8) -> Result<Option<usize>> {
    if buf.len() < UNKNOWN_VAR_MIN {
        return Ok(None);
    }
    let declared = u16::from_be_bytes([buf[1], buf[2]]) as usize;
    if (UNKNOWN_VAR_MIN..=UNKNOWN_VAR_MAX).contains(&declared) {
        if declared > buf.len() {
            return Ok(None);
        }
        tracing::warn!(packet = id, declared, "unknown packet treated as variable");
        Ok(Some(declared))
    } else {
        Ok(Some(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lengths::PacketTable;

    #[test]
    fn frames_fixed_and_variable() {
        let mut d = FrameDecoder::new(PacketTable::t2a());
        let mut speech = vec![0x03, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x03, b'h', b'i', 0];
        speech[1] = 0x00;
        speech[2] = speech.len() as u8;
        let ping = vec![0x73, 0x01];
        let mut bytes = ping.clone();
        bytes.extend_from_slice(&speech);
        let pkts = d.push(&bytes).unwrap();
        assert_eq!(pkts.len(), 2);
        assert_eq!(pkts[0].id, 0x73);
        assert_eq!(pkts[1].id, 0x03);
    }

    #[test]
    fn waits_for_full_packet() {
        let mut d = FrameDecoder::new(PacketTable::t2a());
        assert!(d.push(&[0x73]).unwrap().is_empty());
        let pkts = d.push(&[0x04]).unwrap();
        assert_eq!(pkts[0].bytes, vec![0x73, 0x04]);
    }

    #[test]
    fn game_decoder_splits_compressed_packets() {
        let h = crate::huffman::Huffman::new();
        let mut stream = h.compress(&[0x73, 0x01]);
        stream.extend_from_slice(&h.compress(&[0x55]));
        let mut g = GameDecoder::new(PacketTable::t2a());
        let pkts = g.push_compressed(&stream).unwrap();
        assert_eq!(pkts.len(), 2);
        assert_eq!(pkts[0].bytes, vec![0x73, 0x01]);
        assert_eq!(pkts[1].bytes, vec![0x55]);
    }

    fn mobile_incoming_packet() -> Vec<u8> {
        let mut p = vec![PKT_MOBILE_INCOMING];
        p.extend_from_slice(&0x0000_1234u32.to_be_bytes());
        p.extend_from_slice(&0x0190u16.to_be_bytes());
        p.extend_from_slice(&100u16.to_be_bytes());
        p.extend_from_slice(&200u16.to_be_bytes());
        p.push(0);
        p.push(0);
        p.extend_from_slice(&0u16.to_be_bytes());
        p.push(0);
        p.push(1);
        p.extend_from_slice(&0x4000_0001u32.to_be_bytes());
        p.extend_from_slice(&0x8001u16.to_be_bytes());
        p.push(1);
        p.extend_from_slice(&0x0021u16.to_be_bytes());
        p.extend_from_slice(&0u32.to_be_bytes());
        p
    }

    #[test]
    fn frames_mobile_incoming_without_length_prefix() {
        let pkt = mobile_incoming_packet();
        let expected = pkt.len();
        let mut d = FrameDecoder::new(PacketTable::t2a());
        let mut bytes = vec![0x73, 0x01];
        bytes.extend_from_slice(&pkt);
        let pkts = d.push(&bytes).unwrap();
        assert_eq!(pkts.len(), 2);
        assert_eq!(pkts[0].id, 0x73);
        assert_eq!(pkts[1].id, PKT_MOBILE_INCOMING);
        assert_eq!(pkts[1].bytes.len(), expected);
    }

    #[test]
    fn unprefixed_mobile_incoming_ignores_serial_bytes_as_length() {
        let mut pkt = vec![PKT_MOBILE_INCOMING];
        pkt.extend_from_slice(&0x0040_0001u32.to_be_bytes());
        pkt.extend_from_slice(&0x0190u16.to_be_bytes());
        pkt.extend_from_slice(&100u16.to_be_bytes());
        pkt.extend_from_slice(&200u16.to_be_bytes());
        pkt.push(0);
        pkt.push(0);
        pkt.extend_from_slice(&0u16.to_be_bytes());
        pkt.push(0);
        pkt.push(1);
        pkt.extend_from_slice(&0u32.to_be_bytes());
        let mut d = FrameDecoder::new(PacketTable::t2a());
        let pkts = d.push(&pkt).unwrap();
        assert_eq!(pkts.len(), 1);
        assert_eq!(pkts[0].bytes.len(), pkt.len());
    }

    #[test]
    fn mobile_incoming_len_waits_for_terminator() {
        let pkt = mobile_incoming_packet();
        assert_eq!(mobile_incoming_len(&pkt[..pkt.len() - 1]), None);
        assert_eq!(mobile_incoming_len(&pkt), Some(pkt.len()));
    }

    #[test]
    fn unknown_opcode_skips_one_byte() {
        let mut d = FrameDecoder::new(PacketTable::t2a());
        let pkts = d.push(&[0xEE, 0x73, 0x02]).unwrap();
        assert!(pkts.iter().any(|p| p.id == 0x73));
    }

    #[test]
    fn modern_mobile_incoming_uses_length_prefix() {
        let mut pkt = vec![PKT_MOBILE_INCOMING, 0, 0];
        pkt.extend_from_slice(&0x0000_00AAu32.to_be_bytes());
        pkt.extend_from_slice(&0x0190u16.to_be_bytes());
        pkt.extend_from_slice(&100u16.to_be_bytes());
        pkt.extend_from_slice(&200u16.to_be_bytes());
        pkt.push(0);
        pkt.push(0);
        pkt.extend_from_slice(&0u16.to_be_bytes());
        pkt.push(0);
        pkt.push(1);
        pkt.extend_from_slice(&0u32.to_be_bytes());
        let n = pkt.len() as u16;
        pkt[1..3].copy_from_slice(&n.to_be_bytes());
        let mut d = FrameDecoder::new(PacketTable::modern());
        let pkts = d.push(&pkt).unwrap();
        assert_eq!(pkts.len(), 1);
        assert_eq!(pkts[0].bytes.len(), pkt.len());
    }

    #[test]
    fn healthbar_ec_split_chunks_wait() {
        let mut d = FrameDecoder::new(PacketTable::modern());
        assert!(d.push(&[PKT_HEALTHBAR_EC, 0x00, 0x00]).unwrap().is_empty());
        let rest = [0u8; HEALTHBAR_EC_HIDE_LEN - VARIABLE_LEN_MIN];
        let pkts = d.push(&rest).unwrap();
        assert_eq!(pkts.len(), 1);
        assert_eq!(pkts[0].id, PKT_HEALTHBAR_EC);
        assert_eq!(pkts[0].bytes.len(), HEALTHBAR_EC_HIDE_LEN);
    }

    #[test]
    fn invalid_variable_length_keeps_prior_packet() {
        let mut d = FrameDecoder::new(PacketTable::modern());
        let pkts = d.push(&[0x55, 0x16, 0x00, 0x00, 0x73, 0x01]).unwrap();
        assert!(
            pkts.iter().any(|p| p.id == 0x55),
            "ids: {:?}",
            pkts.iter().map(|p| p.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unknown_variable_waits_for_full_length() {
        let mut d = FrameDecoder::new(PacketTable::t2a());
        assert!(d.push(&[0xF3, 0x00, 0x18]).unwrap().is_empty());
        let mut rest = vec![0u8; 21];
        rest[0] = 1;
        let pkts = d.push(&rest).unwrap();
        assert_eq!(pkts.len(), 1);
        assert_eq!(pkts[0].id, 0xF3);
        assert_eq!(pkts[0].bytes.len(), 24);
    }
}
