use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::step::{LandCorners, TileColumn, TilePiece};
use crate::tiles::{StaticView, TileQuery, TILE_DOOR};
use crate::uop::{
    load_map_entries, UopIndex, UOP_CACHE_CAP, UOP_COMPRESS_NONE, UOP_MAP_BLOCK_MASK,
    UOP_MAP_BLOCK_SHIFT,
};

pub const CELL_BYTES: usize = 3;
pub const BLOCK_CELLS: usize = 64;
pub const BLOCK_HEADER: usize = 4;
pub const BLOCK_BYTES: usize = BLOCK_HEADER + BLOCK_CELLS * CELL_BYTES;
pub const STATIC_RECORD: usize = 7;
pub const STAIDX_RECORD: usize = 12;
pub const LAND_RECORD_OLD: usize = 26;
pub const LAND_RECORD_HS: usize = 30;
pub const STATIC_RECORD_OLD: usize = 37;
pub const STATIC_RECORD_HS: usize = 41;
pub const LAND_COUNT: usize = 0x4000;
pub const DEFAULT_MAP0_BLOCKS_W: u16 = 896;
pub const DEFAULT_MAP0_BLOCKS_H: u16 = 512;
pub const OLD_FELUCCA_BLOCKS_W: u16 = 768;
pub const MAP_ILSHENAR_BLOCKS_W: u16 = 288;
pub const MAP_ILSHENAR_BLOCKS_H: u16 = 200;
pub const MAP_MALAS_BLOCKS_W: u16 = 320;
pub const MAP_MALAS_BLOCKS_H: u16 = 256;
pub const MAP_TOKUNO_BLOCKS: u16 = 181;
pub const MAP_TERMUR_BLOCKS_W: u16 = 160;
pub const MAP_BLOCKS_W_640: u16 = 640;
pub const CELL_PER_BLOCK_EDGE: u16 = 8;
pub const TILEDATA_HS_LEN: usize = 3_188_736;
/// Every UO index file marks a slot it does not fill with this lookup.
pub const IDX_EMPTY: u32 = 0xFFFF_FFFF;
pub const GROUP_HEADER: usize = 4;
pub const LAND_GROUP: usize = 32;
pub const STATIC_GROUP: usize = 32;
/// A static tiledata record is `flags`, then `weight u8`, `layer u8`,
/// `count u32`, `animid u16`, `hue u16`, `lightindex u16`, `height u8`, then
/// the name. So the height byte starts 1+1+4+2+2+2 = 12 bytes after the flags
/// field, on High Seas files (`u64` flags) and older files (`u32` flags) alike.
pub const STATIC_HEIGHT_BYTES_AFTER_FLAGS: usize = 12;
/// Every tiledata record ends with a NUL-padded latin1 name of this length.
pub const TILE_NAME_LEN: usize = 20;
/// A land record puts its texture id between the flags and the name.
pub const LAND_NAME_AFTER_FLAGS: usize = 2;
pub const TILEDATA_FLAGS_OLD: usize = 4;
pub const TILEDATA_FLAGS_HS: usize = 8;
/// The tiledata file every reader in this crate takes its flags from.
pub const TILEDATA_NAME: &str = "tiledata.mul";
/// Environment variable that names a real Ultima Online client data directory.
///
/// This repository ships no client files. Tests that must read real map,
/// statics and tiledata files look here for them.
pub const ENV_TEST_UOPATH: &str = "UOTERM_TEST_UOPATH";

/// The client data directory named by [`ENV_TEST_UOPATH`], when it holds
/// client files.
///
/// A build machine has no client data, so the answer is `None` there and every
/// test that needs real files returns early instead of failing.
pub fn client_data_dir_from_env() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os(ENV_TEST_UOPATH)?);
    dir.join(TILEDATA_NAME).exists().then_some(dir)
}
/// How many tiles a map keeps in hand. One step check reads four tiles and
/// each of those reads its three neighbours, so a search that walks from one
/// tile to the next asks again for tiles it has only just read.
pub(crate) const COLUMN_CACHE_CAP: usize = 16;
/// How many 8x8 blocks of the map a map keeps in memory. A path search
/// crosses the same blocks thousands of times; each is read from disk once.
/// A block is a few hundred bytes, so this is a few megabytes at most. When
/// full, the store starts again.
const BLOCK_CACHE_CAP: usize = 4096;
/// The tiles in one block of the map, eight by eight.
const CELLS_PER_BLOCK: usize = (CELL_PER_BLOCK_EDGE as usize) * (CELL_PER_BLOCK_EDGE as usize);

#[derive(Debug, Error)]
pub enum MapError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("client file is truncated")]
    Truncated,
    #[error("uopath is missing {0}")]
    Missing(&'static str),
    #[error("map uop is not a LegacyMUL package")]
    BadUop,
    #[error("map uop compression is not supported")]
    UnsupportedUop,
}

#[derive(Clone, Debug)]
pub struct MapFiles {
    pub map: PathBuf,
    pub statics: PathBuf,
    pub staidx: PathBuf,
    pub tiledata: PathBuf,
    pub blocks_w: u16,
    pub blocks_h: u16,
    pub map_index: u8,
}

impl MapFiles {
    pub fn from_uopath(uopath: &Path, map_index: u8) -> Result<Self, MapError> {
        let (mut blocks_w, mut blocks_h) = map_block_dims(map_index);
        let map = first_existing(
            uopath,
            &[
                &format!("map{map_index}.mul"),
                &format!("map{map_index}LegacyMUL.uop"),
                &format!("map{map_index}xLegacyMUL.uop"),
            ],
        )
        .ok_or(MapError::Missing("map*.mul"))?;
        if !is_uop_path(&map) {
            if let Ok(meta) = std::fs::metadata(&map) {
                let inferred = infer_mul_blocks(meta.len(), blocks_w, blocks_h);
                blocks_w = inferred.0;
                blocks_h = inferred.1;
            }
        }
        let statics = uopath.join(format!("statics{map_index}.mul"));
        if !statics.exists() {
            return Err(MapError::Missing("statics*.mul"));
        }
        let staidx = uopath.join(format!("staidx{map_index}.mul"));
        let tiledata = uopath.join(TILEDATA_NAME);
        if !staidx.exists() {
            return Err(MapError::Missing("staidx*.mul"));
        }
        if !tiledata.exists() {
            return Err(MapError::Missing(TILEDATA_NAME));
        }
        Ok(Self {
            map,
            statics,
            staidx,
            tiledata,
            blocks_w,
            blocks_h,
            map_index,
        })
    }
}

pub fn map_block_dims(map_index: u8) -> (u16, u16) {
    match map_index {
        2 => (MAP_ILSHENAR_BLOCKS_W, MAP_ILSHENAR_BLOCKS_H),
        3 => (MAP_MALAS_BLOCKS_W, MAP_MALAS_BLOCKS_H),
        4 => (MAP_TOKUNO_BLOCKS, MAP_TOKUNO_BLOCKS),
        5 => (MAP_TERMUR_BLOCKS_W, DEFAULT_MAP0_BLOCKS_H),
        _ => (DEFAULT_MAP0_BLOCKS_W, DEFAULT_MAP0_BLOCKS_H),
    }
}

pub fn infer_mul_blocks(file_len: u64, default_w: u16, default_h: u16) -> (u16, u16) {
    if file_len < BLOCK_BYTES as u64 {
        return (default_w, default_h);
    }
    let blocks = file_len / BLOCK_BYTES as u64;
    let widths = [
        default_w,
        OLD_FELUCCA_BLOCKS_W,
        DEFAULT_MAP0_BLOCKS_W,
        MAP_BLOCKS_W_640,
        MAP_MALAS_BLOCKS_W,
        MAP_ILSHENAR_BLOCKS_W,
        MAP_MALAS_BLOCKS_H,
        MAP_TOKUNO_BLOCKS,
        MAP_TERMUR_BLOCKS_W,
    ];
    for w in widths {
        if w > 0 && blocks % u64::from(w) == 0 {
            let h = blocks / u64::from(w);
            if h > 0 && h <= u64::from(u16::MAX) {
                return (w, h as u16);
            }
        }
    }
    (default_w, default_h)
}

pub fn tiledata_is_hs(len: usize) -> bool {
    len == TILEDATA_HS_LEN
}

/// Map, staidx, and UOP block order is column-major: all blocks of column 0
/// come first, then column 1: `(x >> 3) * blocks_height + (y >> 3)`.
pub fn block_index(blocks_h: u16, bx: u16, by: u16) -> u64 {
    u64::from(bx) * u64::from(blocks_h) + u64::from(by)
}

/// Reads a file whole. Every client file this helper serves is small next to a
/// map file, and no reader keeps a handle open after it loads.
pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>, MapError> {
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;
    Ok(data)
}

/// The `len` bytes that start at `at`, or `None` when the file ends first.
/// Neither number can run past the end of the slice or over its own width.
pub(crate) fn slice_at(data: &[u8], at: usize, len: usize) -> Option<&[u8]> {
    data.get(at..).and_then(|rest| rest.get(..len))
}

/// How many bytes a read may take when a file says it wants `want` of them.
pub(crate) fn capped_len(file_len: u64, offset: u64, want: u32) -> usize {
    let avail = file_len.saturating_sub(offset);
    (u64::from(want)).min(avail) as usize
}

pub(crate) fn first_existing(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names.iter().map(|n| dir.join(n)).find(|p| p.exists())
}

pub(crate) fn is_uop_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("uop"))
}

/// Reads the NUL-padded latin1 name that starts at `offset`. A record cut short
/// by a truncated file gives a blank name instead of an error.
fn read_tile_name(data: &[u8], offset: usize) -> String {
    let end = offset.saturating_add(TILE_NAME_LEN).min(data.len());
    if offset >= end {
        return String::new();
    }
    let raw = &data[offset..end];
    let len = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    raw[..len].iter().map(|&b| char::from(b)).collect()
}

#[derive(Clone, Debug)]
pub(crate) struct TileFlags {
    land: Vec<u32>,
    land_name: Vec<String>,
    r#static: Vec<u32>,
    static_height: Vec<u8>,
    static_name: Vec<String>,
}

impl TileFlags {
    pub(crate) fn load(path: &Path) -> Result<Self, MapError> {
        let data = read_file(path)?;
        let hs = tiledata_is_hs(data.len());
        let land_size = if hs { LAND_RECORD_HS } else { LAND_RECORD_OLD };
        let static_size = if hs {
            STATIC_RECORD_HS
        } else {
            STATIC_RECORD_OLD
        };
        let flags_size = if hs {
            TILEDATA_FLAGS_HS
        } else {
            TILEDATA_FLAGS_OLD
        };
        let height_off = flags_size + STATIC_HEIGHT_BYTES_AFTER_FLAGS;
        let land_name_off = flags_size + LAND_NAME_AFTER_FLAGS;
        let static_name_off = static_size - TILE_NAME_LEN;
        let mut land = vec![0u32; LAND_COUNT];
        let mut land_name = vec![String::new(); LAND_COUNT];
        let mut offset = 0usize;
        let mut i = 0usize;
        while i < LAND_COUNT && offset + land_size <= data.len() {
            if i % LAND_GROUP == 0 {
                offset += GROUP_HEADER;
                if offset + land_size > data.len() {
                    break;
                }
            }
            land[i] = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            land_name[i] = read_tile_name(&data, offset + land_name_off);
            offset += land_size;
            i += 1;
        }
        let mut r#static = Vec::new();
        let mut static_height = Vec::new();
        let mut static_name = Vec::new();
        while offset + static_size <= data.len() {
            if r#static.len() % STATIC_GROUP == 0 {
                offset += GROUP_HEADER;
                if offset + static_size > data.len() {
                    break;
                }
            }
            let flags = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            let height = data.get(offset + height_off).copied().unwrap_or(0);
            r#static.push(flags);
            static_height.push(height);
            static_name.push(read_tile_name(&data, offset + static_name_off));
            offset += static_size;
        }
        Ok(Self {
            land,
            land_name,
            r#static,
            static_height,
            static_name,
        })
    }

    fn land(&self, id: u16) -> u32 {
        self.land.get(id as usize).copied().unwrap_or(0)
    }

    fn land_name(&self, id: u16) -> &str {
        self.land_name.get(id as usize).map_or("", String::as_str)
    }

    pub(crate) fn stat(&self, id: u16) -> (u32, u8) {
        (
            self.r#static.get(id as usize).copied().unwrap_or(0),
            self.static_height.get(id as usize).copied().unwrap_or(0),
        )
    }

    fn static_name(&self, id: u16) -> &str {
        self.static_name.get(id as usize).map_or("", String::as_str)
    }
}

/// One static of the map files: what the movement rules read, plus the
/// graphic that names it to an agent.
#[derive(Clone, Copy, Debug)]
struct StaticPiece {
    graphic: u16,
    piece: TilePiece,
}

struct MulHandles {
    map: File,
    statics: File,
    staidx: File,
    uop_cache: Vec<(u32, Vec<u8>)>,
    /// The last tiles read, oldest first, each under its packed x,y.
    columns: Vec<(u32, TileColumn)>,
    /// Whole blocks read from disk, under their block index.
    blocks: HashMap<u64, MapBlock>,
}

/// One 8x8 block of the map in memory: the ground of each tile, and the
/// statics standing on each tile, both in row order.
struct MapBlock {
    cells: Vec<(u16, i8)>,
    statics: Vec<Vec<StaticPiece>>,
}

pub struct MulMap {
    inner: std::sync::Mutex<MulHandles>,
    flags: TileFlags,
    blocks_w: u16,
    blocks_h: u16,
    uop: Option<Vec<Option<UopIndex>>>,
}

impl MulMap {
    pub fn open(uopath: impl AsRef<Path>, map_index: u8) -> Result<Self, MapError> {
        let files = MapFiles::from_uopath(uopath.as_ref(), map_index)?;
        Self::from_files(&files)
    }

    pub fn from_files(files: &MapFiles) -> Result<Self, MapError> {
        let uop = if is_uop_path(&files.map) {
            Some(load_map_entries(&files.map, files.map_index)?)
        } else {
            None
        };
        Ok(Self {
            inner: std::sync::Mutex::new(MulHandles {
                map: File::open(&files.map)?,
                statics: File::open(&files.statics)?,
                staidx: File::open(&files.staidx)?,
                uop_cache: Vec::new(),
                columns: Vec::new(),
                blocks: HashMap::new(),
            }),
            flags: TileFlags::load(&files.tiledata)?,
            blocks_w: files.blocks_w,
            blocks_h: files.blocks_h,
            uop,
        })
    }

    fn read_cell(
        handles: &mut MulHandles,
        uop: Option<&[Option<UopIndex>]>,
        blocks_h: u16,
        x: u16,
        y: u16,
    ) -> Result<(u16, i8), MapError> {
        let mut buf = [0u8; CELL_BYTES];
        if let Some(entries) = uop {
            read_uop_cell(handles, entries, blocks_h, x, y, &mut buf)?;
        } else {
            let offset = mul_cell_offset(blocks_h, x, y);
            handles.map.seek(SeekFrom::Start(offset))?;
            handles.map.read_exact(&mut buf)?;
        }
        let land = u16::from_le_bytes([buf[0], buf[1]]);
        Ok((land, buf[2] as i8))
    }

    /// The statics of one whole block, each under the tile it stands on.
    fn read_block_statics(
        handles: &mut MulHandles,
        flags: &TileFlags,
        block: u64,
    ) -> Result<Vec<Vec<StaticPiece>>, MapError> {
        let mut out = vec![Vec::new(); CELLS_PER_BLOCK];
        handles
            .staidx
            .seek(SeekFrom::Start(block * STAIDX_RECORD as u64))?;
        let mut idx = [0u8; STAIDX_RECORD];
        handles.staidx.read_exact(&mut idx)?;
        let lookup = u32::from_le_bytes([idx[0], idx[1], idx[2], idx[3]]);
        let length = u32::from_le_bytes([idx[4], idx[5], idx[6], idx[7]]);
        if lookup == IDX_EMPTY || length == 0 || length == IDX_EMPTY {
            return Ok(out);
        }
        let offset = u64::from(lookup);
        let file_len = handles.statics.metadata()?.len();
        let n = capped_len(file_len, offset, length);
        if n == 0 {
            return Ok(out);
        }
        handles.statics.seek(SeekFrom::Start(offset))?;
        let mut data = vec![0u8; n];
        handles.statics.read_exact(&mut data)?;
        let edge = usize::from(CELL_PER_BLOCK_EDGE);
        for chunk in data.chunks(STATIC_RECORD) {
            if chunk.len() < STATIC_RECORD {
                break;
            }
            let graphic = u16::from_le_bytes([chunk[0], chunk[1]]);
            let (sx, sy) = (usize::from(chunk[2]), usize::from(chunk[3]));
            if sx >= edge || sy >= edge {
                continue;
            }
            let (st_flags, height) = flags.stat(graphic);
            out[sy * edge + sx].push(StaticPiece {
                graphic,
                piece: TilePiece {
                    z: chunk[4] as i8,
                    height,
                    flags: st_flags,
                },
            });
        }
        Ok(out)
    }

    /// The block that holds tile `x`, `y`, read from disk the first time it
    /// is asked for, and the tile's place in it.
    fn block_of<'h>(
        &self,
        handles: &'h mut MulHandles,
        x: u16,
        y: u16,
    ) -> Result<(&'h MapBlock, usize), MapError> {
        let edge = CELL_PER_BLOCK_EDGE;
        let (bx, by) = (x / edge, y / edge);
        let key = block_index(self.blocks_h, bx, by);
        if !handles.blocks.contains_key(&key) {
            let uop = self.uop.as_deref();
            let mut cells = Vec::with_capacity(CELLS_PER_BLOCK);
            for cy in 0..edge {
                for cx in 0..edge {
                    let at = (bx * edge + cx, by * edge + cy);
                    cells.push(Self::read_cell(handles, uop, self.blocks_h, at.0, at.1)?);
                }
            }
            let statics = Self::read_block_statics(handles, &self.flags, key)?;
            if handles.blocks.len() >= BLOCK_CACHE_CAP {
                handles.blocks.clear();
            }
            handles.blocks.insert(key, MapBlock { cells, statics });
        }
        let i = usize::from(y % edge) * usize::from(edge) + usize::from(x % edge);
        Ok((&handles.blocks[&key], i))
    }

    /// The ground graphic and height of one tile.
    fn cell(&self, handles: &mut MulHandles, x: u16, y: u16) -> Result<(u16, i8), MapError> {
        let (block, i) = self.block_of(handles, x, y)?;
        Ok(block.cells[i])
    }

    /// The statics standing on one tile.
    fn statics(
        &self,
        handles: &mut MulHandles,
        x: u16,
        y: u16,
    ) -> Result<Vec<StaticPiece>, MapError> {
        let (block, i) = self.block_of(handles, x, y)?;
        Ok(block.statics[i].clone())
    }

    /// The column of one tile, off the last few read when it is one of them.
    ///
    /// One step check reads the tile a person is on, the tile ahead of him and
    /// the two beside the corner, and each of those reads its own three
    /// neighbours to average the ground. A search then asks the same question
    /// of the next tile along, whose neighbours it has just read. Without this
    /// the same cell of the map file is read many times over for one step.
    fn cached_column(&self, x: u16, y: u16) -> Result<TileColumn, MapError> {
        let mut handles = self.inner.lock().map_err(|_| MapError::Truncated)?;
        let key = (u32::from(x) << u16::BITS) | u32::from(y);
        if let Some(i) = handles.columns.iter().position(|(k, _)| *k == key) {
            let hit = handles.columns.remove(i);
            let column = hit.1.clone();
            handles.columns.push(hit);
            return Ok(column);
        }
        let column = self.read_column(&mut handles, x, y)?;
        if handles.columns.len() >= COLUMN_CACHE_CAP {
            handles.columns.remove(0);
        }
        handles.columns.push((key, column.clone()));
        Ok(column)
    }

    fn read_column(
        &self,
        handles: &mut MulHandles,
        x: u16,
        y: u16,
    ) -> Result<TileColumn, MapError> {
        let (land_id, land_z) = self.cell(handles, x, y)?;
        let statics = self.statics(handles, x, y)?;
        // The height of a corner cell. A cell off the south or east edge of
        // the map has no corner of its own, so the cell that asked for it
        // lends its own height and the four corners come out flat.
        let corner = |handles: &mut MulHandles, dx: u16, dy: u16| {
            let (cx, cy) = (x.saturating_add(dx), y.saturating_add(dy));
            if !self.in_bounds(cx, cy) {
                return land_z;
            }
            self.cell(handles, cx, cy).map_or(land_z, |(_, z)| z)
        };
        Ok(TileColumn {
            land_id,
            land_flags: self.flags.land(land_id),
            land: LandCorners {
                north_west: land_z,
                north_east: corner(handles, 1, 0),
                south_west: corner(handles, 0, 1),
                south_east: corner(handles, 1, 1),
            },
            pieces: statics.into_iter().map(|s| s.piece).collect(),
        })
    }
}

fn mul_cell_offset(blocks_h: u16, x: u16, y: u16) -> u64 {
    let edge = u64::from(CELL_PER_BLOCK_EDGE);
    let cx = u64::from(x) % edge;
    let cy = u64::from(y) % edge;
    let block = block_index(blocks_h, x / CELL_PER_BLOCK_EDGE, y / CELL_PER_BLOCK_EDGE);
    block * BLOCK_BYTES as u64 + BLOCK_HEADER as u64 + (cy * edge + cx) * CELL_BYTES as u64
}

fn read_uop_cell(
    handles: &mut MulHandles,
    entries: &[Option<UopIndex>],
    blocks_h: u16,
    x: u16,
    y: u16,
    buf: &mut [u8; CELL_BYTES],
) -> Result<(), MapError> {
    let edge = CELL_PER_BLOCK_EDGE;
    let block = block_index(blocks_h, x / edge, y / edge);
    let file_no = u32::try_from(block >> UOP_MAP_BLOCK_SHIFT).map_err(|_| MapError::Truncated)?;
    let block_in_file = block & u64::from(UOP_MAP_BLOCK_MASK);
    let entry = entries
        .get(file_no as usize)
        .and_then(|e| e.as_ref())
        .ok_or(MapError::Truncated)?;
    let cx = u64::from(x % edge);
    let cy = u64::from(y % edge);
    let local = block_in_file * BLOCK_BYTES as u64
        + BLOCK_HEADER as u64
        + (cy * u64::from(edge) + cx) * CELL_BYTES as u64;
    if entry.compression == UOP_COMPRESS_NONE {
        handles.map.seek(SeekFrom::Start(entry.offset + local))?;
        handles.map.read_exact(buf)?;
        return Ok(());
    }
    let chunk = load_uop_chunk(handles, file_no, entry)?;
    let start = local as usize;
    if start + CELL_BYTES > chunk.len() {
        return Err(MapError::Truncated);
    }
    buf.copy_from_slice(&chunk[start..start + CELL_BYTES]);
    Ok(())
}

fn load_uop_chunk<'a>(
    handles: &'a mut MulHandles,
    file_no: u32,
    entry: &UopIndex,
) -> Result<&'a [u8], MapError> {
    if let Some(i) = handles.uop_cache.iter().position(|(n, _)| *n == file_no) {
        let rec = handles.uop_cache.remove(i);
        handles.uop_cache.push(rec);
        return Ok(&handles.uop_cache[handles.uop_cache.len() - 1].1);
    }
    handles.map.seek(SeekFrom::Start(entry.offset))?;
    let file_len = handles.map.metadata()?.len();
    let n = capped_len(file_len, entry.offset, entry.compressed_len);
    let mut raw = vec![0u8; n];
    handles.map.read_exact(&mut raw)?;
    let data = crate::uop::decompress(&raw, entry.compression, entry.decompressed_len)?;
    if handles.uop_cache.len() >= UOP_CACHE_CAP {
        handles.uop_cache.remove(0);
    }
    handles.uop_cache.push((file_no, data));
    Ok(&handles.uop_cache[handles.uop_cache.len() - 1].1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorTile {
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub graphic: u16,
    pub dx: i32,
    pub dy: i32,
}

impl MulMap {
    pub fn is_door_graphic(&self, graphic: u16) -> bool {
        self.flags.stat(graphic).0 & TILE_DOOR != 0
    }

    fn static_pieces_at(&self, x: u16, y: u16) -> Vec<StaticPiece> {
        if !self.in_bounds(x, y) {
            return Vec::new();
        }
        let mut handles = match self.inner.lock() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
        self.statics(&mut handles, x, y).unwrap_or_default()
    }

    pub fn doors_near(&self, cx: u16, cy: u16, radius: u16) -> Vec<DoorTile> {
        let r = i32::from(radius);
        let mut out = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let x = i32::from(cx) + dx;
                let y = i32::from(cy) + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as u16;
                let y = y as u16;
                for s in self.static_pieces_at(x, y) {
                    if s.piece.flags & TILE_DOOR != 0 {
                        out.push(DoorTile {
                            x,
                            y,
                            z: s.piece.z,
                            graphic: s.graphic,
                            dx,
                            dy,
                        });
                    }
                }
            }
        }
        out.sort_by_key(|d| d.dx.abs() + d.dy.abs());
        out
    }
}

impl TileQuery for MulMap {
    fn column(&self, x: u16, y: u16) -> TileColumn {
        if !self.in_bounds(x, y) {
            return TileColumn::off_map();
        }
        self.cached_column(x, y)
            .unwrap_or_else(|_| TileColumn::off_map())
    }

    fn statics_at(&self, x: u16, y: u16) -> Vec<StaticView> {
        let mut out: Vec<StaticView> = self
            .static_pieces_at(x, y)
            .into_iter()
            .map(|s| StaticView {
                graphic: s.graphic,
                name: self.flags.static_name(s.graphic).to_string(),
                z: s.piece.z,
                height: s.piece.height,
                flags: s.piece.flags,
            })
            .collect();
        out.sort_by_key(|s| s.z);
        out
    }

    fn land_name(&self, x: u16, y: u16) -> String {
        if !self.in_bounds(x, y) {
            return String::new();
        }
        let Ok(mut handles) = self.inner.lock() else {
            return String::new();
        };
        match self.cell(&mut handles, x, y) {
            Ok((land, _)) => self.flags.land_name(land).to_string(),
            Err(_) => String::new(),
        }
    }

    fn item_name(&self, graphic: u16) -> String {
        self.flags.static_name(graphic).to_string()
    }

    fn width(&self) -> u16 {
        self.blocks_w.saturating_mul(CELL_PER_BLOCK_EDGE)
    }

    fn height(&self) -> u16 {
        self.blocks_h.saturating_mul(CELL_PER_BLOCK_EDGE)
    }
}
