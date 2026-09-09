//! Pluggable stream ciphers.
//!
//! Official-server key material is not shipped as dumped `client.exe` tables.
//! Version keys use the public POL/vCrypter formula every open-source Classic
//! Client uses. The default codec is identity (nocrypt). `OsiCipher` is the
//! Classic Client login + Twofish/MD5 game path the reference client uses when
//! `encryption=1`.
//!
//! # Send / receive order (`OsiCipher`, matching the reference client network
//! loop)
//!
//! The login seed is sent plaintext by the session. This codec never emits it.
//!
//! * **Login (after seed, before [`StreamCipher::reset_for_game`])**
//!   * Outbound: `encrypt` applies the login XOR keyed by seed + version.
//!   * Inbound: the login server sends plaintext. The reference client does not
//!     decrypt it. `decrypt` is the same XOR as `encrypt` so two ciphers
//!     roundtrip; do not call `decrypt` on login-server replies.
//! * **Game (after `reset_for_game`, typically 0x8C reconnect / 0x91)**
//!   * Outbound: the Classic Client does **not** Huffman-encode. Call
//!     `encrypt` on raw packet bytes (Twofish table XOR).
//!   * Inbound: call `decrypt` (repeating 16-byte MD5 XOR) **then** Huffman
//!     decode. Huffman is not this codec's job.
//!
//! Game `encrypt` and `decrypt` are independent keystreams, not inverses.
//! Every Classic Client version has keys via the derivation below; 2.0.4+
//! (including 2.0.7 and 7.0.x) use Twofish+MD5 on the game socket.

use serde::{Deserialize, Serialize};

use crate::types::ClientVersion;

const OSI_NAME: &str = "osi";
const NONE_NAME: &str = "none";

const SEED_XOR_A: u32 = 0x0000_1357;
const SEED_XOR_B: u32 = 0xFFFF_AAAA;
const SEED_XOR_C: u32 = 0x4321_0000;
const SEED_XOR_D: u32 = 0xABCD_FFFF;
const KEY_CONST_LO: u32 = 0x2C13_A5FD;
const KEY_CONST_HI: u32 = 0xA31D_527F;
const KEY_MINOR_MUL_LO: u32 = 0x0B00_0000;
const KEY_BUILD_MUL_LO: u32 = 0x0038_0000;
const KEY_MINOR_MUL_HI: u32 = 0x0680_0000;
const KEY_BUILD_MUL_HI: u32 = 0x001C_0000;
const KEY_TEMP_HI_MUL: u32 = 0x0C00;
const U8_MASK: u32 = 0xFF;
const U16_MASK: u32 = 0xFFFF;
const U16_SHIFT: u32 = 16;

const VER_OLD_LOGIN: ClientVersion = ClientVersion {
    major: 1,
    minor: 25,
    revision: 35,
    patch: 0,
};
const VER_LOGIN_1_25_36: ClientVersion = ClientVersion {
    major: 1,
    minor: 25,
    revision: 36,
    patch: 0,
};

const TWOFISH_BLOCK: usize = 16;
const TWOFISH_TABLE_LEN: usize = 256;
const TWOFISH_KEY_WORDS: usize = 4;
const TWOFISH_ROUNDS: usize = 16;
const TWOFISH_SUBKEYS: usize = 40;
const TWOFISH_SBOX_KEYS: usize = 4;
const TWOFISH_INPUT_WHITEN: usize = 0;
const TWOFISH_OUTPUT_WHITEN: usize = 4;
const TWOFISH_ROUND_SUBKEYS: usize = 8;
const TWOFISH_SK_STEP: u32 = 0x0202_0202;
const TWOFISH_SK_BUMP: u32 = 0x0101_0101;
const TWOFISH_SK_ROTL: u32 = 9;
const TWOFISH_RS_GF: u32 = 0x14D;
const TWOFISH_MDS_GF: u32 = 0x169;
const TWOFISH_MDS_GF_HALF: u32 = TWOFISH_MDS_GF / 2;
const TWOFISH_MDS_GF_QUARTER: u32 = TWOFISH_MDS_GF / 4;
const BYTE_SHIFT_1: u32 = 8;
const BYTE_SHIFT_2: u32 = 16;
const BYTE_SHIFT_3: u32 = 24;
const WORD_BYTES: usize = 4;
const MD5_LEN: usize = 16;
const MD5_BLOCK: usize = 64;
const MD5_POS_MASK: u8 = 0x0F;
const MD5_PAD_TARGET: usize = 56;
const MD5_INIT_A: u32 = 0x6745_2301;
const MD5_INIT_B: u32 = 0xEFCD_AB89;
const MD5_INIT_C: u32 = 0x98BA_DCFE;
const MD5_INIT_D: u32 = 0x1032_5476;
const MD5_PAD_START: u8 = 0x80;
const BITS_PER_BYTE: u64 = 8;

const SPECIAL_12536_C0: u32 = 0x35CE_9581;
const SPECIAL_12536_C1: u32 = 0x07AF_CC37;
const SPECIAL_12536_C2: u32 = 0x4C3A_1353;
const SPECIAL_12536_C3: u32 = 0x16EF_783F;
const SPECIAL_12536_SHIFT_A: u32 = 5;
const SPECIAL_12536_SHIFT_B: u32 = 3;

const KEY_MAJOR_SHIFT: u32 = 9;
const KEY_MID_SHIFT: u32 = 10;
const KEY_SQ_SHIFT: u32 = 5;
const KEY_TEMP_SHIFT: u32 = 4;
const KEY_HI_MUL: u32 = 8;

pub trait StreamCipher: Send {
    fn name(&self) -> &'static str;
    fn encrypt(&mut self, data: &mut [u8]);
    fn decrypt(&mut self, data: &mut [u8]);
    /// Switch from the login XOR to the game Twofish+MD5 streams.
    ///
    /// `seed` is the game-server seed (0x8C / the value sent before 0x91).
    fn reset_for_game(&mut self, _seed: u32) {}
}

/// Stream codec selected by config. Freeshards stay [`None`]; official and
/// encrypted shards use [`Osi`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncryptionMode {
    #[default]
    None,
    Osi,
}

/// Build the codec for `mode`. [`EncryptionMode::None`] ignores seed and version.
pub fn for_mode(mode: EncryptionMode, seed: u32, version: ClientVersion) -> Box<dyn StreamCipher> {
    match mode {
        EncryptionMode::None => Box::new(IdentityCipher),
        EncryptionMode::Osi => Box::new(OsiCipher::new(seed, version)),
    }
}

#[derive(Default, Debug, Clone)]
pub struct IdentityCipher;

impl StreamCipher for IdentityCipher {
    fn name(&self) -> &'static str {
        NONE_NAME
    }

    fn encrypt(&mut self, _data: &mut [u8]) {}

    fn decrypt(&mut self, _data: &mut [u8]) {}
}

/// Classic Client login XOR + Twofish/MD5 game streams.
pub struct OsiCipher {
    keys: [u32; 3],
    login_kind: LoginKind,
    phase: OsiPhase,
}

#[derive(Clone, Copy)]
enum LoginKind {
    Old,
    Special12536,
    Modern,
}

enum OsiPhase {
    Login(LoginState),
    Game(Box<GameState>),
}

struct LoginState {
    key0: u32,
    key1: u32,
}

struct GameState {
    twofish: Twofish,
    table: [u8; TWOFISH_TABLE_LEN],
    table_pos: u16,
    xor_data: [u8; MD5_LEN],
    xor_pos: u8,
}

struct Twofish {
    sbox_keys: [u32; TWOFISH_SBOX_KEYS],
    sub_keys: [u32; TWOFISH_SUBKEYS],
}

impl OsiCipher {
    pub fn new(seed: u32, version: ClientVersion) -> Self {
        Self {
            keys: Self::version_keys(version),
            login_kind: login_kind(version),
            phase: OsiPhase::Login(LoginState::from_seed(seed)),
        }
    }

    /// POL/vCrypter keys `(k1, k2, k3)` for `version`.
    ///
    /// Every Classic Client version has keys. They are derived from
    /// major.minor.revision (patch is unused). 2.0.0x uses the 2.0.0 keys.
    pub fn version_keys(version: ClientVersion) -> [u32; 3] {
        let a = version.major & U8_MASK;
        let b = version.minor & U8_MASK;
        let c = version.revision & U8_MASK;
        let mut temp = ((((a << KEY_MAJOR_SHIFT) | b) << KEY_MID_SHIFT) | c)
            ^ ((c.wrapping_mul(c)) << KEY_SQ_SHIFT);
        let key2 = (temp << KEY_TEMP_SHIFT)
            ^ b.wrapping_mul(b)
            ^ b.wrapping_mul(KEY_MINOR_MUL_LO)
            ^ c.wrapping_mul(KEY_BUILD_MUL_LO)
            ^ KEY_CONST_LO;
        temp = (((((a << KEY_MAJOR_SHIFT) | c) << KEY_MID_SHIFT) | b).wrapping_mul(KEY_HI_MUL))
            ^ c.wrapping_mul(c).wrapping_mul(KEY_TEMP_HI_MUL);
        let key3 = temp
            ^ b.wrapping_mul(b)
            ^ b.wrapping_mul(KEY_MINOR_MUL_HI)
            ^ c.wrapping_mul(KEY_BUILD_MUL_HI)
            ^ KEY_CONST_HI;
        [key2.wrapping_sub(1), key2, key3]
    }

    pub fn reset_for_game(&mut self, seed: u32) {
        self.switch_to_game(seed);
    }

    fn switch_to_game(&mut self, seed: u32) {
        self.phase = OsiPhase::Game(Box::new(GameState::new(seed)));
    }
}

fn login_kind(version: ClientVersion) -> LoginKind {
    if version == VER_LOGIN_1_25_36 {
        LoginKind::Special12536
    } else if !version.at_least(VER_OLD_LOGIN) {
        LoginKind::Old
    } else {
        LoginKind::Modern
    }
}

impl LoginState {
    fn from_seed(seed: u32) -> Self {
        let key0 = ((!seed ^ SEED_XOR_A) << U16_SHIFT) | ((seed ^ SEED_XOR_B) & U16_MASK);
        let key1 = ((seed ^ SEED_XOR_C) >> U16_SHIFT) | ((!seed ^ SEED_XOR_D) & !U16_MASK);
        Self { key0, key1 }
    }

    fn xor_byte(&mut self, kind: LoginKind, keys: [u32; 3], b: &mut u8) {
        *b ^= self.key0 as u8;
        let table0 = self.key0;
        let table1 = self.key1;
        match kind {
            LoginKind::Old => {
                self.key0 = ((table0 >> 1) | (table1 << 31)) ^ keys[1];
                self.key1 = ((table1 >> 1) | (table0 << 31)) ^ keys[0];
            }
            LoginKind::Modern => {
                self.key1 = (((((table1 >> 1) | (table0 << 31)) ^ keys[0]) >> 1) | (table0 << 31))
                    ^ keys[1];
                self.key0 = ((table0 >> 1) | (table1 << 31)) ^ keys[2];
            }
            LoginKind::Special12536 => {
                self.key0 = ((table0 >> 1) | (table1 << 31)) ^ keys[1];
                self.key1 = ((table1 >> 1) | (table0 << 31)) ^ keys[0];
                let k1 = keys[0];
                let k2 = keys[1];
                let sh1 = SPECIAL_12536_SHIFT_A
                    .wrapping_mul(table1)
                    .wrapping_mul(table1)
                    & U8_MASK;
                self.key1 = k1
                    .wrapping_shr(sh1)
                    .wrapping_add(table1.wrapping_mul(k1))
                    .wrapping_add(table0.wrapping_mul(table0).wrapping_mul(SPECIAL_12536_C0))
                    .wrapping_add(SPECIAL_12536_C1);
                let sh2 = SPECIAL_12536_SHIFT_B
                    .wrapping_mul(table0)
                    .wrapping_mul(table0)
                    & U8_MASK;
                self.key0 = k2
                    .wrapping_shr(sh2)
                    .wrapping_add(table0.wrapping_mul(k2))
                    .wrapping_add(
                        self.key1
                            .wrapping_mul(self.key1)
                            .wrapping_mul(SPECIAL_12536_C2),
                    )
                    .wrapping_add(SPECIAL_12536_C3);
            }
        }
    }
}

impl StreamCipher for OsiCipher {
    fn name(&self) -> &'static str {
        OSI_NAME
    }

    fn encrypt(&mut self, data: &mut [u8]) {
        match &mut self.phase {
            OsiPhase::Login(login) => {
                let kind = self.login_kind;
                let keys = self.keys;
                for b in data.iter_mut() {
                    login.xor_byte(kind, keys, b);
                }
            }
            OsiPhase::Game(game) => game.encrypt(data),
        }
    }

    fn decrypt(&mut self, data: &mut [u8]) {
        match &mut self.phase {
            OsiPhase::Login(login) => {
                let kind = self.login_kind;
                let keys = self.keys;
                for b in data.iter_mut() {
                    login.xor_byte(kind, keys, b);
                }
            }
            OsiPhase::Game(game) => game.decrypt(data),
        }
    }

    fn reset_for_game(&mut self, seed: u32) {
        self.switch_to_game(seed);
    }
}

impl GameState {
    fn new(seed: u32) -> Self {
        let word = seed.swap_bytes();
        let twofish = Twofish::with_key(&[word, word, word, word]);
        let mut table = [0u8; TWOFISH_TABLE_LEN];
        for (i, slot) in table.iter_mut().enumerate() {
            *slot = i as u8;
        }
        let mut game = Self {
            twofish,
            table,
            table_pos: 0,
            xor_data: [0; MD5_LEN],
            xor_pos: 0,
        };
        game.refresh_cipher_table();
        game.xor_data = md5_hash(&game.table);
        game
    }

    fn refresh_cipher_table(&mut self) {
        let mut off = 0;
        while off < TWOFISH_TABLE_LEN {
            let mut block = [
                load_u32_le(&self.table, off),
                load_u32_le(&self.table, off + WORD_BYTES),
                load_u32_le(&self.table, off + WORD_BYTES * 2),
                load_u32_le(&self.table, off + WORD_BYTES * 3),
            ];
            self.twofish.encrypt_words(&mut block);
            store_u32_le(&mut self.table, off, block[0]);
            store_u32_le(&mut self.table, off + WORD_BYTES, block[1]);
            store_u32_le(&mut self.table, off + WORD_BYTES * 2, block[2]);
            store_u32_le(&mut self.table, off + WORD_BYTES * 3, block[3]);
            off += TWOFISH_BLOCK;
        }
        self.table_pos = 0;
    }

    fn encrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            if self.table_pos as usize >= TWOFISH_TABLE_LEN {
                self.refresh_cipher_table();
            }
            *b ^= self.table[self.table_pos as usize];
            self.table_pos += 1;
        }
    }

    fn decrypt(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            *b ^= self.xor_data[self.xor_pos as usize];
            self.xor_pos = (self.xor_pos + 1) & MD5_POS_MASK;
        }
    }
}

impl Twofish {
    fn with_key(key32: &[u32; TWOFISH_KEY_WORDS]) -> Self {
        let mut sbox_keys = [0u32; TWOFISH_SBOX_KEYS];
        let mut sub_keys = [0u32; TWOFISH_SUBKEYS];
        let mut k32e = [0u32; TWOFISH_SBOX_KEYS];
        let mut k32o = [0u32; TWOFISH_SBOX_KEYS];
        let k64_cnt = TWOFISH_KEY_WORDS / 2;
        for i in 0..k64_cnt {
            k32e[i] = key32[2 * i];
            k32o[i] = key32[2 * i + 1];
            sbox_keys[k64_cnt - 1 - i] = rs_mds_encode(k32e[i], k32o[i]);
        }
        for i in 0..(TWOFISH_SUBKEYS / 2) {
            let a = twofish_f32((i as u32).wrapping_mul(TWOFISH_SK_STEP), &k32e);
            let mut b = twofish_f32(
                (i as u32)
                    .wrapping_mul(TWOFISH_SK_STEP)
                    .wrapping_add(TWOFISH_SK_BUMP),
                &k32o,
            );
            b = b.rotate_left(BYTE_SHIFT_1);
            sub_keys[2 * i] = a.wrapping_add(b);
            sub_keys[2 * i + 1] = a
                .wrapping_add(b.wrapping_mul(2))
                .rotate_left(TWOFISH_SK_ROTL);
        }
        Self {
            sbox_keys,
            sub_keys,
        }
    }

    fn encrypt_words(&self, x: &mut [u32; TWOFISH_KEY_WORDS]) {
        for (i, word) in x.iter_mut().enumerate() {
            *word ^= self.sub_keys[TWOFISH_INPUT_WHITEN + i];
        }
        for r in 0..TWOFISH_ROUNDS {
            let t0 = twofish_f32(x[0], &self.sbox_keys);
            let t1 = twofish_f32(x[1].rotate_left(BYTE_SHIFT_1), &self.sbox_keys);
            x[3] = x[3].rotate_left(1);
            x[2] ^= t0
                .wrapping_add(t1)
                .wrapping_add(self.sub_keys[TWOFISH_ROUND_SUBKEYS + 2 * r]);
            x[3] ^= t0
                .wrapping_add(t1.wrapping_mul(2))
                .wrapping_add(self.sub_keys[TWOFISH_ROUND_SUBKEYS + 2 * r + 1]);
            x[2] = x[2].rotate_right(1);
            if r + 1 < TWOFISH_ROUNDS {
                x.swap(0, 2);
                x.swap(1, 3);
            }
        }
        for (i, word) in x.iter_mut().enumerate() {
            *word ^= self.sub_keys[TWOFISH_OUTPUT_WHITEN + i];
        }
    }
}

fn twofish_f32(x: u32, k32: &[u32; TWOFISH_SBOX_KEYS]) -> u32 {
    // 128-bit keyed S-boxes: P_00=1,P_01=0,P_02=0 and the matching P_1x/P_2x/P_3x.
    let b0 = pbox(1, pbox(0, pbox(0, b0(x)) ^ b0(k32[1])) ^ b0(k32[0]));
    let b1 = pbox(0, pbox(0, pbox(1, b1(x)) ^ b1(k32[1])) ^ b1(k32[0]));
    let b2 = pbox(1, pbox(1, pbox(0, b2(x)) ^ b2(k32[1])) ^ b2(k32[0]));
    let b3 = pbox(0, pbox(1, pbox(1, b3(x)) ^ b3(k32[1])) ^ b3(k32[0]));
    let m0 = mx_1(b0) ^ mx_y(b1) ^ mx_x(b2) ^ mx_x(b3);
    let m1 = mx_x(b0) ^ mx_y(b1) ^ mx_y(b2) ^ mx_1(b3);
    let m2 = mx_y(b0) ^ mx_x(b1) ^ mx_1(b2) ^ mx_y(b3);
    let m3 = mx_y(b0) ^ mx_1(b1) ^ mx_y(b2) ^ mx_x(b3);
    m0 ^ (m1 << BYTE_SHIFT_1) ^ (m2 << BYTE_SHIFT_2) ^ (m3 << BYTE_SHIFT_3)
}

fn pbox(which: u8, x: u8) -> u8 {
    if which == 0 {
        Q0[x as usize]
    } else {
        Q1[x as usize]
    }
}

fn b0(x: u32) -> u8 {
    x as u8
}

fn b1(x: u32) -> u8 {
    (x >> BYTE_SHIFT_1) as u8
}

fn b2(x: u32) -> u8 {
    (x >> BYTE_SHIFT_2) as u8
}

fn b3(x: u32) -> u8 {
    (x >> BYTE_SHIFT_3) as u8
}

fn lfsr1(x: u32) -> u32 {
    (x >> 1) ^ if x & 1 == 1 { TWOFISH_MDS_GF_HALF } else { 0 }
}

fn lfsr2(x: u32) -> u32 {
    (x >> 2)
        ^ if x & 2 == 2 { TWOFISH_MDS_GF_HALF } else { 0 }
        ^ if x & 1 == 1 {
            TWOFISH_MDS_GF_QUARTER
        } else {
            0
        }
}

fn mx_1(x: u8) -> u32 {
    u32::from(x)
}

fn mx_x(x: u8) -> u32 {
    let v = u32::from(x);
    (v ^ lfsr2(v)) & U8_MASK
}

fn mx_y(x: u8) -> u32 {
    let v = u32::from(x);
    (v ^ lfsr1(v) ^ lfsr2(v)) & U8_MASK
}

fn rs_mds_encode(k0: u32, k1: u32) -> u32 {
    let mut r = 0u32;
    for i in 0..2u32 {
        r ^= if i > 0 { k0 } else { k1 };
        for _ in 0..WORD_BYTES {
            rs_rem(&mut r);
        }
    }
    r
}

fn rs_rem(x: &mut u32) {
    let b = *x >> BYTE_SHIFT_3;
    let g2 = ((b << 1) ^ if b & 0x80 == 0x80 { TWOFISH_RS_GF } else { 0 }) & U8_MASK;
    let g3 = ((b >> 1) & 0x7F) ^ if b & 1 == 1 { TWOFISH_RS_GF >> 1 } else { 0 } ^ g2;
    *x = (*x << BYTE_SHIFT_1)
        ^ (g3 << BYTE_SHIFT_3)
        ^ (g2 << BYTE_SHIFT_2)
        ^ (g3 << BYTE_SHIFT_1)
        ^ b;
}

fn load_u32_le(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

fn store_u32_le(bytes: &mut [u8], off: usize, value: u32) {
    let raw = value.to_le_bytes();
    bytes[off] = raw[0];
    bytes[off + 1] = raw[1];
    bytes[off + 2] = raw[2];
    bytes[off + 3] = raw[3];
}

fn md5_hash(data: &[u8]) -> [u8; MD5_LEN] {
    let mut state = [MD5_INIT_A, MD5_INIT_B, MD5_INIT_C, MD5_INIT_D];
    let bit_len = (data.len() as u64).wrapping_mul(BITS_PER_BYTE);
    let mut padded = Vec::with_capacity(data.len() + MD5_BLOCK);
    padded.extend_from_slice(data);
    padded.push(MD5_PAD_START);
    while padded.len() % MD5_BLOCK != MD5_PAD_TARGET {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_le_bytes());
    for chunk in padded.chunks_exact(MD5_BLOCK) {
        md5_step(&mut state, chunk);
    }
    let mut out = [0u8; MD5_LEN];
    for (i, word) in state.iter().enumerate() {
        store_u32_le(&mut out, i * WORD_BYTES, *word);
    }
    out
}

fn md5_step(state: &mut [u32; 4], chunk: &[u8]) {
    let mut input = [0u32; 16];
    for (i, slot) in input.iter_mut().enumerate() {
        *slot = load_u32_le(chunk, i * WORD_BYTES);
    }
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    for i in 0..64 {
        let (f, g) = match i / 16 {
            0 => ((b & c) | (!b & d), i),
            1 => ((b & d) | (c & !d), (5 * i + 1) % 16),
            2 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | !d), (7 * i) % 16),
        };
        let f = f
            .wrapping_add(a)
            .wrapping_add(MD5_K[i])
            .wrapping_add(input[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(f.rotate_left(MD5_S[i]));
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const MD5_K: [u32; 64] = [
    0xD76A_A478,
    0xE8C7_B756,
    0x2420_70DB,
    0xC1BD_CEEE,
    0xF57C_0FAF,
    0x4787_C62A,
    0xA830_4613,
    0xFD46_9501,
    0x6980_98D8,
    0x8B44_F7AF,
    0xFFFF_5BB1,
    0x895C_D7BE,
    0x6B90_1122,
    0xFD98_7193,
    0xA679_438E,
    0x49B4_0821,
    0xF61E_2562,
    0xC040_B340,
    0x265E_5A51,
    0xE9B6_C7AA,
    0xD62F_105D,
    0x0244_1453,
    0xD8A1_E681,
    0xE7D3_FBC8,
    0x21E1_CDE6,
    0xC337_07D6,
    0xF4D5_0D87,
    0x455A_14ED,
    0xA9E3_E905,
    0xFCEF_A3F8,
    0x676F_02D9,
    0x8D2A_4C8A,
    0xFFFA_3942,
    0x8771_F681,
    0x6D9D_6122,
    0xFDE5_380C,
    0xA4BE_EA44,
    0x4BDE_CFA9,
    0xF6BB_4B60,
    0xBEBF_BC70,
    0x289B_7EC6,
    0xEAA1_27FA,
    0xD4EF_3085,
    0x0488_1D05,
    0xD9D4_D039,
    0xE6DB_99E5,
    0x1FA2_7CF8,
    0xC4AC_5665,
    0xF429_2244,
    0x432A_FF97,
    0xAB94_23A7,
    0xFC93_A039,
    0x655B_59C3,
    0x8F0C_CC92,
    0xFFEF_F47D,
    0x8584_5DD1,
    0x6FA8_7E4F,
    0xFE2C_E6E0,
    0xA301_4314,
    0x4E08_11A1,
    0xF753_7E82,
    0xBD3A_F235,
    0x2AD7_D2BB,
    0xEB86_D391,
];

/// Twofish q0 permutation (public AES submission table).
const Q0: [u8; 256] = [
    0xA9, 0x67, 0xB3, 0xE8, 0x04, 0xFD, 0xA3, 0x76, 0x9A, 0x92, 0x80, 0x78, 0xE4, 0xDD, 0xD1, 0x38,
    0x0D, 0xC6, 0x35, 0x98, 0x18, 0xF7, 0xEC, 0x6C, 0x43, 0x75, 0x37, 0x26, 0xFA, 0x13, 0x94, 0x48,
    0xF2, 0xD0, 0x8B, 0x30, 0x84, 0x54, 0xDF, 0x23, 0x19, 0x5B, 0x3D, 0x59, 0xF3, 0xAE, 0xA2, 0x82,
    0x63, 0x01, 0x83, 0x2E, 0xD9, 0x51, 0x9B, 0x7C, 0xA6, 0xEB, 0xA5, 0xBE, 0x16, 0x0C, 0xE3, 0x61,
    0xC0, 0x8C, 0x3A, 0xF5, 0x73, 0x2C, 0x25, 0x0B, 0xBB, 0x4E, 0x89, 0x6B, 0x53, 0x6A, 0xB4, 0xF1,
    0xE1, 0xE6, 0xBD, 0x45, 0xE2, 0xF4, 0xB6, 0x66, 0xCC, 0x95, 0x03, 0x56, 0xD4, 0x1C, 0x1E, 0xD7,
    0xFB, 0xC3, 0x8E, 0xB5, 0xE9, 0xCF, 0xBF, 0xBA, 0xEA, 0x77, 0x39, 0xAF, 0x33, 0xC9, 0x62, 0x71,
    0x81, 0x79, 0x09, 0xAD, 0x24, 0xCD, 0xF9, 0xD8, 0xE5, 0xC5, 0xB9, 0x4D, 0x44, 0x08, 0x86, 0xE7,
    0xA1, 0x1D, 0xAA, 0xED, 0x06, 0x70, 0xB2, 0xD2, 0x41, 0x7B, 0xA0, 0x11, 0x31, 0xC2, 0x27, 0x90,
    0x20, 0xF6, 0x60, 0xFF, 0x96, 0x5C, 0xB1, 0xAB, 0x9E, 0x9C, 0x52, 0x1B, 0x5F, 0x93, 0x0A, 0xEF,
    0x91, 0x85, 0x49, 0xEE, 0x2D, 0x4F, 0x8F, 0x3B, 0x47, 0x87, 0x6D, 0x46, 0xD6, 0x3E, 0x69, 0x64,
    0x2A, 0xCE, 0xCB, 0x2F, 0xFC, 0x97, 0x05, 0x7A, 0xAC, 0x7F, 0xD5, 0x1A, 0x4B, 0x0E, 0xA7, 0x5A,
    0x28, 0x14, 0x3F, 0x29, 0x88, 0x3C, 0x4C, 0x02, 0xB8, 0xDA, 0xB0, 0x17, 0x55, 0x1F, 0x8A, 0x7D,
    0x57, 0xC7, 0x8D, 0x74, 0xB7, 0xC4, 0x9F, 0x72, 0x7E, 0x15, 0x22, 0x12, 0x58, 0x07, 0x99, 0x34,
    0x6E, 0x50, 0xDE, 0x68, 0x65, 0xBC, 0xDB, 0xF8, 0xC8, 0xA8, 0x2B, 0x40, 0xDC, 0xFE, 0x32, 0xA4,
    0xCA, 0x10, 0x21, 0xF0, 0xD3, 0x5D, 0x0F, 0x00, 0x6F, 0x9D, 0x36, 0x42, 0x4A, 0x5E, 0xC1, 0xE0,
];

/// Twofish q1 permutation (public AES submission table).
const Q1: [u8; 256] = [
    0x75, 0xF3, 0xC6, 0xF4, 0xDB, 0x7B, 0xFB, 0xC8, 0x4A, 0xD3, 0xE6, 0x6B, 0x45, 0x7D, 0xE8, 0x4B,
    0xD6, 0x32, 0xD8, 0xFD, 0x37, 0x71, 0xF1, 0xE1, 0x30, 0x0F, 0xF8, 0x1B, 0x87, 0xFA, 0x06, 0x3F,
    0x5E, 0xBA, 0xAE, 0x5B, 0x8A, 0x00, 0xBC, 0x9D, 0x6D, 0xC1, 0xB1, 0x0E, 0x80, 0x5D, 0xD2, 0xD5,
    0xA0, 0x84, 0x07, 0x14, 0xB5, 0x90, 0x2C, 0xA3, 0xB2, 0x73, 0x4C, 0x54, 0x92, 0x74, 0x36, 0x51,
    0x38, 0xB0, 0xBD, 0x5A, 0xFC, 0x60, 0x62, 0x96, 0x6C, 0x42, 0xF7, 0x10, 0x7C, 0x28, 0x27, 0x8C,
    0x13, 0x95, 0x9C, 0xC7, 0x24, 0x46, 0x3B, 0x70, 0xCA, 0xE3, 0x85, 0xCB, 0x11, 0xD0, 0x93, 0xB8,
    0xA6, 0x83, 0x20, 0xFF, 0x9F, 0x77, 0xC3, 0xCC, 0x03, 0x6F, 0x08, 0xBF, 0x40, 0xE7, 0x2B, 0xE2,
    0x79, 0x0C, 0xAA, 0x82, 0x41, 0x3A, 0xEA, 0xB9, 0xE4, 0x9A, 0xA4, 0x97, 0x7E, 0xDA, 0x7A, 0x17,
    0x66, 0x94, 0xA1, 0x1D, 0x3D, 0xF0, 0xDE, 0xB3, 0x0B, 0x72, 0xA7, 0x1C, 0xEF, 0xD1, 0x53, 0x3E,
    0x8F, 0x33, 0x26, 0x5F, 0xEC, 0x76, 0x2A, 0x49, 0x81, 0x88, 0xEE, 0x21, 0xC4, 0x1A, 0xEB, 0xD9,
    0xC5, 0x39, 0x99, 0xCD, 0xAD, 0x31, 0x8B, 0x01, 0x18, 0x23, 0xDD, 0x1F, 0x4E, 0x2D, 0xF9, 0x48,
    0x4F, 0xF2, 0x65, 0x8E, 0x78, 0x5C, 0x58, 0x19, 0x8D, 0xE5, 0x98, 0x57, 0x67, 0x7F, 0x05, 0x64,
    0xAF, 0x63, 0xB6, 0xFE, 0xF5, 0xB7, 0x3C, 0xA5, 0xCE, 0xE9, 0x68, 0x44, 0xE0, 0x4D, 0x43, 0x69,
    0x29, 0x2E, 0xAC, 0x15, 0x59, 0xA8, 0x0A, 0x9E, 0x6E, 0x47, 0xDF, 0x34, 0x35, 0x6A, 0xCF, 0xDC,
    0x22, 0xC9, 0xC0, 0x9B, 0x89, 0xD4, 0xED, 0xAB, 0x12, 0xA2, 0x0D, 0x52, 0xBB, 0x02, 0x2F, 0xA9,
    0xD7, 0x61, 0x1E, 0xB4, 0x50, 0x04, 0xF6, 0xC2, 0x16, 0x25, 0x86, 0x56, 0x55, 0x09, 0xBE, 0x91,
];

#[cfg(test)]
fn twofish_encrypt_block(key_words: &[u32; TWOFISH_KEY_WORDS], block: &mut [u8; TWOFISH_BLOCK]) {
    let fish = Twofish::with_key(key_words);
    let mut words = [
        load_u32_le(block, 0),
        load_u32_le(block, WORD_BYTES),
        load_u32_le(block, WORD_BYTES * 2),
        load_u32_le(block, WORD_BYTES * 3),
    ];
    fish.encrypt_words(&mut words);
    store_u32_le(block, 0, words[0]);
    store_u32_le(block, WORD_BYTES, words[1]);
    store_u32_le(block, WORD_BYTES * 2, words[2]);
    store_u32_le(block, WORD_BYTES * 3, words[3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: u32 = 0xDEAD_BEEF;
    const GAME_SEED: u32 = 0xA1B2_C3D4;
    const SWITCH_SEED: u32 = 0x1111_2222;
    const GAME_BUF_LEN: usize = 64;
    const GAME_REFRESH_LEN: usize = 300;
    const KEY_2_0_0: [u32; 3] = [0x2D13_A5FC, 0x2D13_A5FD, 0xA39D_527F];
    const KEY_2_0_3: [u32; 3] = [0x2DBB_B7CC, 0x2DBB_B7CD, 0xA3C9_5E7F];
    const KEY_7_0_73: [u32; 3] = [0x2042_036C, 0x2042_036D, 0xA5D1_BE7F];
    const KEY_7_0_102: [u32; 3] = [0x3992_EB9C, 0x3992_EB9D, 0xA81E_227F];
    const TWOFISH_ZERO_CT: [u8; TWOFISH_BLOCK] = [
        0x9F, 0x58, 0x9F, 0x5C, 0xF6, 0x12, 0x2C, 0x32, 0xB6, 0xBF, 0xEC, 0x2F, 0x2A, 0xE8, 0xC3,
        0x5A,
    ];
    const MD5_EMPTY: [u8; MD5_LEN] = [
        0xD4, 0x1D, 0x8C, 0xD9, 0x8F, 0x00, 0xB2, 0x04, 0xE9, 0x80, 0x09, 0x98, 0xEC, 0xF8, 0x42,
        0x7E,
    ];
    const MD5_ABC: [u8; MD5_LEN] = [
        0x90, 0x01, 0x50, 0x98, 0x3C, 0xD2, 0x4F, 0xB0, 0xD6, 0x96, 0x3F, 0x7D, 0x28, 0xE1, 0x7F,
        0x72,
    ];

    #[test]
    fn identity_is_noop() {
        let mut c = IdentityCipher;
        let mut d = *b"abc";
        c.encrypt(&mut d);
        assert_eq!(&d, b"abc");
    }

    #[test]
    fn encryption_mode_default_is_none() {
        assert_eq!(EncryptionMode::default(), EncryptionMode::None);
    }

    #[test]
    fn encryption_mode_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&EncryptionMode::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&EncryptionMode::Osi).unwrap(),
            "\"osi\""
        );
        assert_eq!(
            serde_json::from_str::<EncryptionMode>("\"none\"").unwrap(),
            EncryptionMode::None
        );
        assert_eq!(
            serde_json::from_str::<EncryptionMode>("\"osi\"").unwrap(),
            EncryptionMode::Osi
        );
    }

    #[test]
    fn for_mode_none_roundtrip() {
        let mut e = for_mode(EncryptionMode::None, TEST_SEED, ClientVersion::MODERN);
        let mut d = for_mode(EncryptionMode::None, TEST_SEED, ClientVersion::MODERN);
        let mut buf = *b"abcXYZ12";
        let original = buf;
        e.encrypt(&mut buf);
        assert_eq!(buf, original);
        d.decrypt(&mut buf);
        assert_eq!(buf, original);
        assert_eq!(e.name(), NONE_NAME);
        e.reset_for_game(TEST_SEED);
        e.encrypt(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn for_mode_osi_login_roundtrip() {
        let mut e = for_mode(EncryptionMode::Osi, TEST_SEED, ClientVersion::MODERN);
        let mut d = for_mode(EncryptionMode::Osi, TEST_SEED, ClientVersion::MODERN);
        let mut buf = *b"password-bytes!!";
        let original = buf;
        e.encrypt(&mut buf);
        assert_ne!(buf, original);
        d.decrypt(&mut buf);
        assert_eq!(buf, original);
        assert_eq!(e.name(), OSI_NAME);
        let mut game = for_mode(EncryptionMode::Osi, TEST_SEED, ClientVersion::MODERN);
        let mut game_buf = original;
        game.reset_for_game(TEST_SEED);
        game.encrypt(&mut game_buf);
        assert_ne!(game_buf, original);
        assert_ne!(game_buf, buf);
    }

    #[test]
    fn osi_game_encrypt_and_decrypt_are_involute() {
        let mut e1 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        let mut e2 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        e1.reset_for_game(GAME_SEED);
        e2.reset_for_game(GAME_SEED);
        let mut enc_buf = [0u8; GAME_BUF_LEN];
        for (i, b) in enc_buf.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7);
        }
        let original = enc_buf;
        e1.encrypt(&mut enc_buf);
        assert_ne!(enc_buf, original);
        e2.encrypt(&mut enc_buf);
        assert_eq!(enc_buf, original);

        let mut d1 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        let mut d2 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        d1.reset_for_game(GAME_SEED);
        d2.reset_for_game(GAME_SEED);
        let mut dec_buf = original;
        d1.decrypt(&mut dec_buf);
        assert_ne!(dec_buf, original);
        d2.decrypt(&mut dec_buf);
        assert_eq!(dec_buf, original);

        let mut enc_once = original;
        let mut dec_once = original;
        let mut e3 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        let mut d3 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        e3.reset_for_game(GAME_SEED);
        d3.reset_for_game(GAME_SEED);
        e3.encrypt(&mut enc_once);
        d3.decrypt(&mut dec_once);
        assert_ne!(enc_once, dec_once);
    }

    #[test]
    fn osi_game_encrypt_refreshes_after_256_bytes() {
        let mut e1 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        let mut e2 = OsiCipher::new(GAME_SEED, ClientVersion::MODERN);
        e1.reset_for_game(GAME_SEED);
        e2.reset_for_game(GAME_SEED);
        let mut buf = vec![0xA5u8; GAME_REFRESH_LEN];
        let original = buf.clone();
        e1.encrypt(&mut buf);
        assert_ne!(buf, original);
        e2.encrypt(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn reset_for_game_switches_cipher() {
        let mut login = OsiCipher::new(SWITCH_SEED, ClientVersion::T2A);
        let mut game = OsiCipher::new(SWITCH_SEED, ClientVersion::T2A);
        game.reset_for_game(SWITCH_SEED);
        let mut a = *b"0123456789abcdef";
        let mut b = a;
        login.encrypt(&mut a);
        game.encrypt(&mut b);
        assert_ne!(a, b);
    }

    #[test]
    fn osi_version_keys_match_published_mapping() {
        assert_eq!(
            OsiCipher::version_keys(ClientVersion {
                major: 2,
                minor: 0,
                revision: 0,
                patch: 0
            }),
            KEY_2_0_0
        );
        assert_eq!(
            OsiCipher::version_keys(ClientVersion {
                major: 2,
                minor: 0,
                revision: 3,
                patch: 0
            }),
            KEY_2_0_3
        );
        assert_eq!(
            OsiCipher::version_keys(ClientVersion {
                major: 7,
                minor: 0,
                revision: 73,
                patch: 0
            }),
            KEY_7_0_73
        );
        assert_eq!(OsiCipher::version_keys(ClientVersion::MODERN), KEY_7_0_102);
        assert_eq!(
            OsiCipher::version_keys(ClientVersion {
                major: 7,
                minor: 0,
                revision: 102,
                patch: 3
            }),
            KEY_7_0_102
        );
    }

    #[test]
    fn twofish_ecb_matches_published_128_bit_vector() {
        let mut block = [0u8; TWOFISH_BLOCK];
        twofish_encrypt_block(&[0u32; TWOFISH_KEY_WORDS], &mut block);
        assert_eq!(block, TWOFISH_ZERO_CT);
    }

    #[test]
    fn md5_matches_rfc_1321() {
        assert_eq!(md5_hash(b""), MD5_EMPTY);
        assert_eq!(md5_hash(b"abc"), MD5_ABC);
    }

    #[test]
    fn osi_login_old_and_modern_differ() {
        let old = ClientVersion {
            major: 1,
            minor: 25,
            revision: 34,
            patch: 0,
        };
        let mut a = OsiCipher::new(TEST_SEED, old);
        let mut b = OsiCipher::new(TEST_SEED, ClientVersion::MODERN);
        let mut x = *b"login-request!!!!";
        let mut y = x;
        a.encrypt(&mut x);
        b.encrypt(&mut y);
        assert_ne!(x, y);
    }

    #[test]
    fn identity_reset_for_game_is_noop() {
        let mut c = IdentityCipher;
        c.reset_for_game(1);
        let mut d = *b"abc";
        c.encrypt(&mut d);
        assert_eq!(&d, b"abc");
    }
}
