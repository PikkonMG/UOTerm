//! Static Ultima Online Huffman codec.
//!
//! The table is the public 257-symbol encoding used on the game stream
//! after account login. Symbol 256 is the per-packet byte-align flush.

use crate::error::{ProtocolError, Result};

const SYMBOL_COUNT: usize = 257;
const FLUSH_SYMBOL: u16 = 256;
const CHILD_NONE: i32 = -1;
const LEAF_NONE: i32 = -1;

/// `(bit_length, code)` for symbols `0..=255` plus flush at index 256.
const TABLE: [(u8, u16); SYMBOL_COUNT] = [
    (2, 0x0000),
    (5, 0x001F),
    (6, 0x0022),
    (7, 0x0034),
    (7, 0x0075),
    (6, 0x0028),
    (6, 0x003B),
    (7, 0x0032),
    (8, 0x00E0),
    (8, 0x0062),
    (7, 0x0056),
    (8, 0x0079),
    (9, 0x019D),
    (8, 0x0097),
    (6, 0x002A),
    (7, 0x0057),
    (8, 0x0071),
    (8, 0x005B),
    (9, 0x01CC),
    (8, 0x00A7),
    (7, 0x0025),
    (7, 0x004F),
    (8, 0x0066),
    (8, 0x007D),
    (9, 0x0191),
    (9, 0x01CE),
    (7, 0x003F),
    (9, 0x0090),
    (8, 0x0059),
    (8, 0x007B),
    (8, 0x0091),
    (8, 0x00C6),
    (6, 0x002D),
    (9, 0x0186),
    (8, 0x006F),
    (9, 0x0093),
    (10, 0x01CC),
    (8, 0x005A),
    (10, 0x01AE),
    (10, 0x01C0),
    (9, 0x0148),
    (9, 0x014A),
    (9, 0x0082),
    (10, 0x019F),
    (9, 0x0171),
    (9, 0x0120),
    (9, 0x00E7),
    (10, 0x01F3),
    (9, 0x014B),
    (9, 0x0100),
    (9, 0x0190),
    (6, 0x0013),
    (9, 0x0161),
    (9, 0x0125),
    (9, 0x0133),
    (9, 0x0195),
    (9, 0x0173),
    (9, 0x01CA),
    (9, 0x0086),
    (9, 0x01E9),
    (9, 0x00DB),
    (9, 0x01EC),
    (9, 0x008B),
    (9, 0x0085),
    (5, 0x000A),
    (8, 0x0096),
    (8, 0x009C),
    (9, 0x01C3),
    (9, 0x019C),
    (9, 0x008F),
    (9, 0x018F),
    (9, 0x0091),
    (9, 0x0087),
    (9, 0x00C6),
    (9, 0x0177),
    (9, 0x0089),
    (9, 0x00D6),
    (9, 0x008C),
    (9, 0x01EE),
    (9, 0x01EB),
    (9, 0x0084),
    (9, 0x0164),
    (9, 0x0175),
    (9, 0x01CD),
    (8, 0x005E),
    (9, 0x0088),
    (9, 0x012B),
    (9, 0x0172),
    (9, 0x010A),
    (9, 0x008D),
    (9, 0x013A),
    (9, 0x011C),
    (10, 0x01E1),
    (10, 0x01E0),
    (9, 0x0187),
    (10, 0x01DC),
    (10, 0x01DF),
    (7, 0x0074),
    (9, 0x019F),
    (8, 0x008D),
    (8, 0x00E4),
    (7, 0x0079),
    (9, 0x00EA),
    (9, 0x00E1),
    (8, 0x0040),
    (7, 0x0041),
    (9, 0x010B),
    (9, 0x00B0),
    (8, 0x006A),
    (8, 0x00C1),
    (7, 0x0071),
    (7, 0x0078),
    (8, 0x00B1),
    (9, 0x014C),
    (7, 0x0043),
    (8, 0x0076),
    (7, 0x0066),
    (7, 0x004D),
    (9, 0x008A),
    (6, 0x002F),
    (8, 0x00C9),
    (9, 0x00CE),
    (9, 0x0149),
    (9, 0x0160),
    (10, 0x01BA),
    (10, 0x019E),
    (10, 0x039F),
    (9, 0x00E5),
    (9, 0x0194),
    (9, 0x0184),
    (9, 0x0126),
    (7, 0x0030),
    (8, 0x006C),
    (9, 0x0121),
    (9, 0x01E8),
    (10, 0x01C1),
    (10, 0x011D),
    (10, 0x0163),
    (10, 0x0385),
    (10, 0x03DB),
    (10, 0x017D),
    (10, 0x0106),
    (10, 0x0397),
    (10, 0x024E),
    (7, 0x002E),
    (8, 0x0098),
    (10, 0x033C),
    (10, 0x032E),
    (10, 0x01E9),
    (9, 0x00BF),
    (10, 0x03DF),
    (10, 0x01DD),
    (10, 0x032D),
    (10, 0x02ED),
    (10, 0x030B),
    (10, 0x0107),
    (10, 0x02E8),
    (10, 0x03DE),
    (10, 0x0125),
    (10, 0x01E8),
    (9, 0x00E9),
    (10, 0x01CD),
    (10, 0x01B5),
    (9, 0x0165),
    (10, 0x0232),
    (10, 0x02E1),
    (11, 0x03AE),
    (11, 0x03C6),
    (11, 0x03E2),
    (10, 0x0205),
    (10, 0x029A),
    (10, 0x0248),
    (10, 0x02CD),
    (10, 0x023B),
    (11, 0x03C5),
    (10, 0x0251),
    (10, 0x02E9),
    (10, 0x0252),
    (9, 0x01EA),
    (11, 0x03A0),
    (11, 0x0391),
    (10, 0x023C),
    (11, 0x0392),
    (11, 0x03D5),
    (10, 0x0233),
    (10, 0x02CC),
    (11, 0x0390),
    (10, 0x01BB),
    (11, 0x03A1),
    (11, 0x03C4),
    (10, 0x0211),
    (10, 0x0203),
    (9, 0x012A),
    (10, 0x0231),
    (11, 0x03E0),
    (10, 0x029B),
    (11, 0x03D7),
    (10, 0x0202),
    (11, 0x03AD),
    (10, 0x0213),
    (10, 0x0253),
    (10, 0x032C),
    (10, 0x023D),
    (10, 0x023F),
    (10, 0x032F),
    (10, 0x011C),
    (10, 0x0384),
    (10, 0x031C),
    (10, 0x017C),
    (10, 0x030A),
    (10, 0x02E0),
    (10, 0x0276),
    (10, 0x0250),
    (11, 0x03E3),
    (10, 0x0396),
    (10, 0x018F),
    (10, 0x0204),
    (10, 0x0206),
    (10, 0x0230),
    (10, 0x0265),
    (10, 0x0212),
    (10, 0x023E),
    (11, 0x03AC),
    (11, 0x0393),
    (11, 0x03E1),
    (10, 0x01DE),
    (11, 0x03D6),
    (10, 0x031D),
    (11, 0x03E5),
    (11, 0x03E4),
    (10, 0x0207),
    (11, 0x03C7),
    (10, 0x0277),
    (11, 0x03D4),
    (8, 0x00C0),
    (10, 0x0162),
    (10, 0x03DA),
    (10, 0x0124),
    (10, 0x01B4),
    (10, 0x0264),
    (10, 0x033D),
    (10, 0x01D1),
    (10, 0x01AF),
    (10, 0x039E),
    (10, 0x024F),
    (11, 0x0373),
    (10, 0x0249),
    (11, 0x0372),
    (9, 0x0167),
    (10, 0x0210),
    (10, 0x023A),
    (10, 0x01B8),
    (11, 0x03AF),
    (10, 0x018E),
    (10, 0x02EC),
    (7, 0x0062),
    (4, 0x000D),
];

#[derive(Clone, Debug)]
struct Node {
    child: [i32; 2],
    value: i32,
}

#[derive(Clone, Debug)]
pub struct Huffman {
    nodes: Vec<Node>,
}

impl Default for Huffman {
    fn default() -> Self {
        Self::new()
    }
}

impl Huffman {
    pub fn new() -> Self {
        let mut nodes = Vec::with_capacity(512);
        nodes.push(Node {
            child: [CHILD_NONE, CHILD_NONE],
            value: LEAF_NONE,
        });
        for (sym, &(bits, code)) in TABLE.iter().enumerate() {
            let mut node = 0i32;
            for shift in (0..bits).rev() {
                let bit = ((code >> shift) & 1) as usize;
                if nodes[node as usize].child[bit] == CHILD_NONE {
                    let created = nodes.len() as i32;
                    nodes.push(Node {
                        child: [CHILD_NONE, CHILD_NONE],
                        value: LEAF_NONE,
                    });
                    nodes[node as usize].child[bit] = created;
                }
                node = nodes[node as usize].child[bit];
            }
            nodes[node as usize].value = sym as i32;
        }
        Self { nodes }
    }

    pub fn compress(&self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len() + 2);
        let mut acc: u32 = 0;
        let mut nbits: u32 = 0;
        for &b in input {
            emit_symbol(&mut out, &mut acc, &mut nbits, b as usize);
        }
        emit_symbol(&mut out, &mut acc, &mut nbits, FLUSH_SYMBOL as usize);
        if nbits > 0 {
            acc <<= 8 - nbits;
            out.push(acc as u8);
        }
        out
    }

    pub fn decompress(&self, input: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = HuffmanDecoder::new(self.clone());
        decoder.push(input)
    }
}

fn emit_symbol(out: &mut Vec<u8>, acc: &mut u32, nbits: &mut u32, symbol: usize) {
    let (bits, code) = TABLE[symbol];
    *acc = (*acc << bits) | u32::from(code);
    *nbits += u32::from(bits);
    while *nbits >= 8 {
        *nbits -= 8;
        out.push((*acc >> *nbits) as u8);
        *acc &= (1u32 << *nbits) - 1;
    }
}

/// Streaming decoder. Leftover bits stay in `bit_pos` across `push` calls.
#[derive(Clone, Debug)]
pub struct HuffmanDecoder {
    tree: Huffman,
    buf: Vec<u8>,
    bit_pos: usize,
}

impl HuffmanDecoder {
    pub fn new(tree: Huffman) -> Self {
        Self {
            tree,
            buf: Vec::new(),
            bit_pos: 0,
        }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.bit_pos = 0;
    }

    pub fn push(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.buf.extend_from_slice(data);
        let total_bits = self.buf.len() * 8;
        let mut pos = self.bit_pos;
        let mut out = Vec::new();
        loop {
            let mut node = 0i32;
            let mut p = pos;
            loop {
                if p >= total_bits {
                    let consumed = pos / 8;
                    if consumed > 0 {
                        self.buf.drain(..consumed);
                    }
                    self.bit_pos = pos % 8;
                    return Ok(out);
                }
                let byte = self.buf[p / 8];
                let bit = ((byte >> (7 - (p % 8))) & 1) as usize;
                p += 1;
                let next = self.tree.nodes[node as usize].child[bit];
                if next == CHILD_NONE {
                    self.reset();
                    return Err(ProtocolError::Huffman);
                }
                node = next;
                if self.tree.nodes[node as usize].value != LEAF_NONE {
                    break;
                }
            }
            let value = self.tree.nodes[node as usize].value;
            if value == FLUSH_SYMBOL as i32 {
                pos = (p + 7) & !7;
            } else {
                out.push(value as u8);
                pos = p;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ascii_payload() {
        let h = Huffman::new();
        let src = b"\x55\x00\x1Bhello ultima online journal";
        let enc = h.compress(src);
        let dec = h.decompress(&enc).unwrap();
        assert_eq!(dec, src);
        assert!(enc.len() <= src.len() + 4);
    }

    #[test]
    fn roundtrip_all_bytes() {
        let h = Huffman::new();
        let src: Vec<u8> = (0u8..=255).collect();
        let dec = h.decompress(&h.compress(&src)).unwrap();
        assert_eq!(dec, src);
    }

    #[test]
    fn empty_compress_is_flush_only() {
        let h = Huffman::new();
        let enc = h.compress(&[]);
        assert!(!enc.is_empty());
        assert!(h.decompress(&enc).unwrap().is_empty());
    }

    #[test]
    fn streaming_decoder_splits_across_pushes() {
        let h = Huffman::new();
        let src = vec![0x1C, 0x00, 0x0A, b'h', b'i'];
        let enc = h.compress(&src);
        let mid = enc.len() / 2;
        let mut d = HuffmanDecoder::new(h);
        let mut out = d.push(&enc[..mid]).unwrap();
        out.extend(d.push(&enc[mid..]).unwrap());
        assert_eq!(out, src);
    }

    #[test]
    fn two_packets_each_with_flush() {
        let h = Huffman::new();
        let a = h.compress(&[0x55]);
        let b = h.compress(&[0x73, 0x01]);
        let mut joined = a.clone();
        joined.extend_from_slice(&b);
        let dec = h.decompress(&joined).unwrap();
        assert_eq!(dec, vec![0x55, 0x73, 0x01]);
    }

    #[test]
    fn golden_ping_bytes() {
        let h = Huffman::new();
        let enc = h.compress(&[0x73, 0x01]);
        assert_eq!(enc, vec![0x76, 0xFE, 0x80]);
        assert_eq!(h.decompress(&enc).unwrap(), vec![0x73, 0x01]);
    }
}
