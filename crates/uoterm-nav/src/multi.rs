//! Player houses and boats.
//!
//! A house or a boat is a "multi": the shard sends an ordinary item whose
//! graphic names a multi, and the client reads the shape of that multi out of
//! its own files. The map and statics files hold none of it, so a client that
//! skips these files sees open ground where a building stands and walks into
//! it.
//!
//! `MultiCollection.uop` is read when the client ships one, because it holds
//! multis the older `multi.mul` and `multi.idx` pair does not.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mul::{
    capped_len, first_existing, is_uop_path, read_file, MapError, TileFlags, IDX_EMPTY,
    TILEDATA_NAME,
};
use crate::step::is_standing_surface;
use crate::tiles::TILE_IMPASSABLE;
use crate::uop::{decompress, load_multi_entries, UopIndex, MULTI_ID_COUNT};

/// `multi.idx` holds the same record every UO index file holds: a lookup, a
/// length, and an extra field this reader has no use for.
pub const MULTI_IDX_RECORD: usize = 12;
/// A component record before High Seas: graphic `u16`, x `i16`, y `i16`,
/// z `i16`, flags `u32`.
pub const MULTI_RECORD_OLD: usize = 12;
/// A High Seas component record adds a trailing field to the old record.
pub const MULTI_RECORD_HS: usize = 16;
/// A `multi.mul` component with a zero flags field is not part of the
/// building. It is the `nodraw` anchor, or a door or a sign the shard places
/// as a loose item of its own.
pub const MULTI_MUL_LOOSE: u32 = 0;
/// A `MultiCollection.uop` entry starts with the multi id and the number of
/// components that follow.
pub const MULTI_UOP_HEADER: usize = 8;
/// The fixed part of a UOP component: graphic `u16`, x `i16`, y `i16`,
/// z `i16`, flags `u16`, then the number of cliloc ids that follow it.
pub const MULTI_UOP_RECORD: usize = 14;
/// Every cliloc id a UOP component carries is this wide.
pub const MULTI_UOP_CLILOC: usize = 4;
/// The UOP flags field turns the `multi.mul` rule around: this bit marks the
/// loose item, and a component that is part of the building has it clear.
pub const MULTI_UOP_LOOSE: u16 = 0x0001;

pub(crate) const MULTI_MUL_NAME: &str = "multi.mul";
pub(crate) const MULTI_IDX_NAME: &str = "multi.idx";
/// Client directories do not agree on the case of this name.
pub(crate) const MULTI_UOP_NAMES: [&str; 2] = ["MultiCollection.uop", "multicollection.uop"];

/// Where a component of a multi sits, and what a person can do with it.
///
/// The offsets are counted from the position of the multi item itself, so a
/// piece stands at `multi.x + dx`, `multi.y + dy`, `multi.z + dz`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiPiece {
    pub graphic: u16,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
    /// The tiledata height of the graphic, as `StaticView` reports it.
    pub height: u8,
    /// The tiledata flags of the graphic.
    pub flags: u32,
}

impl MultiPiece {
    /// A person cannot walk through this piece.
    pub fn blocks(&self) -> bool {
        self.flags & TILE_IMPASSABLE != 0
    }

    /// A person can stand on this piece.
    pub fn surface(&self) -> bool {
        is_standing_surface(self.flags)
    }
}

/// The client files that describe multis.
#[derive(Clone, Debug)]
pub struct MultiFiles {
    /// `MultiCollection.uop` when the client ships one, else `multi.mul`.
    pub multi: PathBuf,
    /// `multi.idx`. A UOP package carries its own directory, so this is
    /// `None` whenever `multi` is one.
    pub index: Option<PathBuf>,
    pub tiledata: PathBuf,
}

impl MultiFiles {
    pub fn from_uopath(uopath: &Path) -> Result<Self, MapError> {
        let tiledata = uopath.join(TILEDATA_NAME);
        if !tiledata.exists() {
            return Err(MapError::Missing(TILEDATA_NAME));
        }
        if let Some(collection) = first_existing(uopath, &MULTI_UOP_NAMES) {
            return Ok(Self {
                multi: collection,
                index: None,
                tiledata,
            });
        }
        let multi = uopath.join(MULTI_MUL_NAME);
        if !multi.exists() {
            return Err(MapError::Missing(MULTI_MUL_NAME));
        }
        let index = uopath.join(MULTI_IDX_NAME);
        if !index.exists() {
            return Err(MapError::Missing(MULTI_IDX_NAME));
        }
        Ok(Self {
            multi,
            index: Some(index),
            tiledata,
        })
    }
}

/// A High Seas component record is 16 bytes and an older one is 12, and both
/// sizes divide some block lengths, so the index decides: the file is High
/// Seas only when every block it names divides by 16. An index that names no
/// block at all reads as High Seas, the layout every maintained client ships.
pub fn multi_is_hs(lengths: &[u32]) -> bool {
    lengths
        .iter()
        .all(|len| *len as usize % MULTI_RECORD_HS == 0)
}

/// Where the pieces of one multi sit in the flat piece list.
#[derive(Clone, Copy, Debug, Default)]
struct PieceSpan {
    start: usize,
    len: usize,
}

/// The shape of every multi the client files describe.
///
/// The whole set is read once and never changes after that, so one instance
/// serves every character on a shard. Read it through the same cache the
/// runtime shares a `MulMap` with.
pub struct MultiData {
    pieces: Vec<MultiPiece>,
    spans: Vec<PieceSpan>,
}

impl MultiData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let files = MultiFiles::from_uopath(uopath.as_ref())?;
        Self::from_files(&files)
    }

    pub fn from_files(files: &MultiFiles) -> Result<Self, MapError> {
        let flags = TileFlags::load(&files.tiledata)?;
        let (pieces, spans) = if is_uop_path(&files.multi) {
            load_uop(&files.multi, &flags)?
        } else {
            let index = files
                .index
                .as_deref()
                .ok_or(MapError::Missing(MULTI_IDX_NAME))?;
            load_mul(&files.multi, index, &flags)?
        };
        Ok(Self { pieces, spans })
    }

    /// Every piece of the building that multi id names. An id the client
    /// files do not describe has no pieces.
    pub fn pieces(&self, multi_id: u16) -> &[MultiPiece] {
        let Some(span) = self.spans.get(multi_id as usize) else {
            return &[];
        };
        let end = span.start.saturating_add(span.len);
        self.pieces.get(span.start..end).unwrap_or(&[])
    }

    /// How many multi ids the client files describe.
    pub fn multi_count(&self) -> usize {
        self.spans.iter().filter(|span| span.len > 0).count()
    }
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let raw = data.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let raw = data.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    read_u16(data, offset).map(|v| v as i16)
}

/// Turns one component into a piece, with the height and the flags the
/// tiledata gives its graphic.
fn piece(flags: &TileFlags, graphic: u16, dx: i16, dy: i16, dz: i16) -> MultiPiece {
    let (tile_flags, height) = flags.stat(graphic);
    MultiPiece {
        graphic,
        dx,
        dy,
        dz,
        height,
        flags: tile_flags,
    }
}

/// Reads the components of one multi out of `multi.mul`. A record cut short by
/// a damaged file ends the multi instead of failing the load.
fn read_mul_block(block: &[u8], record: usize, flags: &TileFlags, out: &mut Vec<MultiPiece>) {
    for chunk in block.chunks(record) {
        if chunk.len() < record {
            break;
        }
        let (Some(graphic), Some(dx), Some(dy), Some(dz), Some(loose)) = (
            read_u16(chunk, 0),
            read_i16(chunk, 2),
            read_i16(chunk, 4),
            read_i16(chunk, 6),
            read_u32(chunk, 8),
        ) else {
            break;
        };
        if loose == MULTI_MUL_LOOSE {
            continue;
        }
        out.push(piece(flags, graphic, dx, dy, dz));
    }
}

fn load_mul(
    multi: &Path,
    index: &Path,
    flags: &TileFlags,
) -> Result<(Vec<MultiPiece>, Vec<PieceSpan>), MapError> {
    let idx = read_file(index)?;
    let data = read_file(multi)?;
    let file_len = data.len() as u64;
    let count = (idx.len() / MULTI_IDX_RECORD).min(MULTI_ID_COUNT as usize);
    let mut blocks = Vec::with_capacity(count);
    for id in 0..count {
        let offset = id * MULTI_IDX_RECORD;
        let (Some(lookup), Some(length)) = (read_u32(&idx, offset), read_u32(&idx, offset + 4))
        else {
            break;
        };
        if lookup == IDX_EMPTY || length == 0 || length == IDX_EMPTY {
            blocks.push(None);
            continue;
        }
        blocks.push(Some((lookup, length)));
    }
    let lengths: Vec<u32> = blocks.iter().flatten().map(|(_, length)| *length).collect();
    let record = if multi_is_hs(&lengths) {
        MULTI_RECORD_HS
    } else {
        MULTI_RECORD_OLD
    };
    let mut pieces = Vec::new();
    let mut spans = vec![PieceSpan::default(); blocks.len()];
    for (id, block) in blocks.iter().enumerate() {
        let Some((lookup, length)) = *block else {
            continue;
        };
        let offset = u64::from(lookup);
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let n = capped_len(file_len, offset, length);
        let Some(bytes) = data.get(start..start.saturating_add(n)) else {
            continue;
        };
        let begin = pieces.len();
        read_mul_block(bytes, record, flags, &mut pieces);
        spans[id] = PieceSpan {
            start: begin,
            len: pieces.len() - begin,
        };
    }
    Ok((pieces, spans))
}

/// Reads the components of one multi out of a `MultiCollection.uop` entry. The
/// component count and every cliloc count come from the file, so both are
/// capped against the bytes that are really there.
fn read_uop_entry(blob: &[u8], flags: &TileFlags, out: &mut Vec<MultiPiece>) {
    let Some(count) = read_u32(blob, 4) else {
        return;
    };
    let room = blob.len().saturating_sub(MULTI_UOP_HEADER) / MULTI_UOP_RECORD;
    let count = (count as usize).min(room);
    out.reserve(count);
    let mut offset = MULTI_UOP_HEADER;
    for _ in 0..count {
        let (Some(graphic), Some(dx), Some(dy), Some(dz), Some(loose), Some(clilocs)) = (
            read_u16(blob, offset),
            read_i16(blob, offset + 2),
            read_i16(blob, offset + 4),
            read_i16(blob, offset + 6),
            read_u16(blob, offset + 8),
            read_u32(blob, offset + 10),
        ) else {
            break;
        };
        offset = offset
            .saturating_add(MULTI_UOP_RECORD)
            .saturating_add((clilocs as usize).saturating_mul(MULTI_UOP_CLILOC));
        if loose & MULTI_UOP_LOOSE != 0 {
            continue;
        }
        out.push(piece(flags, graphic, dx, dy, dz));
    }
}

/// Pulls one entry out of the package. A damaged entry costs its own multi
/// and no other.
fn uop_entry_bytes(data: &[u8], entry: &UopIndex) -> Option<Vec<u8>> {
    let start = usize::try_from(entry.offset).ok()?;
    let n = capped_len(data.len() as u64, entry.offset, entry.compressed_len);
    let raw = data.get(start..start.saturating_add(n))?;
    decompress(raw, entry.compression, entry.decompressed_len).ok()
}

fn load_uop(path: &Path, flags: &TileFlags) -> Result<(Vec<MultiPiece>, Vec<PieceSpan>), MapError> {
    let entries = load_multi_entries(path)?;
    let data = read_file(path)?;
    let mut pieces = Vec::new();
    let mut spans = vec![PieceSpan::default(); entries.len()];
    for (id, entry) in entries.iter().enumerate() {
        let Some(entry) = entry else {
            continue;
        };
        let Some(blob) = uop_entry_bytes(&data, entry) else {
            continue;
        };
        let begin = pieces.len();
        read_uop_entry(&blob, flags, &mut pieces);
        spans[id] = PieceSpan {
            start: begin,
            len: pieces.len() - begin,
        };
    }
    Ok((pieces, spans))
}
