use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use flate2::read::ZlibDecoder;

use crate::mul::MapError;

pub const UOP_MAGIC: u32 = 0x0050_594D;
pub const UOP_MAP_BLOCK_SHIFT: u32 = 12;
pub const UOP_MAP_BLOCKS_PER_FILE: u32 = 1 << UOP_MAP_BLOCK_SHIFT;
pub const UOP_MAP_BLOCK_MASK: u32 = UOP_MAP_BLOCKS_PER_FILE - 1;
pub const UOP_COMPRESS_NONE: u16 = 0;
pub const UOP_COMPRESS_ZLIB: u16 = 1;
pub const UOP_COMPRESS_ZLIB_BWT: u16 = 3;
pub const HASH_SEED: u32 = 0xDEAD_BEEF;
pub const UOP_CACHE_CAP: usize = 8;
/// A map package names one file per chunk, and the probe runs this far past
/// the number of files the package holds so a package that also holds a file
/// this reader cannot name still finds every chunk.
pub const MAP_CHUNK_SLACK: u32 = 8;
/// A multi id comes off the wire as a `u16`, so the probe covers that whole
/// range. `MultiCollection.uop` leaves wide gaps between the ids it holds, so
/// there is no shorter honest limit.
pub const MULTI_ID_COUNT: u32 = u16::MAX as u32 + 1;

#[derive(Clone, Copy, Debug)]
pub struct UopIndex {
    pub offset: u64,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub compression: u16,
}

pub fn map_uop_name(map_index: u8, chunk: u32) -> String {
    format!("build/map{map_index}legacymul/{chunk:08}.dat")
}

pub fn multi_uop_name(multi_id: u32) -> String {
    format!("build/multicollection/{multi_id:06}.bin")
}

pub fn hash_filename(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut eax: u32 = 0;
    let mut edx: u32;
    let mut ebx: u32 = (bytes.len() as u32).wrapping_add(HASH_SEED);
    let mut esi: u32 = ebx;
    let mut edi: u32 = ebx;
    let mut i = 0usize;

    while i + 12 < bytes.len() {
        edi = edi.wrapping_add(u32::from_le_bytes([
            bytes[i + 4],
            bytes[i + 5],
            bytes[i + 6],
            bytes[i + 7],
        ]));
        esi = esi.wrapping_add(u32::from_le_bytes([
            bytes[i + 8],
            bytes[i + 9],
            bytes[i + 10],
            bytes[i + 11],
        ]));
        edx = u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
            .wrapping_sub(esi);
        edx = edx.wrapping_add(ebx) ^ (esi >> 28) ^ (esi << 4);
        esi = esi.wrapping_add(edi);
        edi = edi.wrapping_sub(edx) ^ (edx >> 26) ^ (edx << 6);
        edx = edx.wrapping_add(esi);
        esi = esi.wrapping_sub(edi) ^ (edi >> 24) ^ (edi << 8);
        edi = edi.wrapping_add(edx);
        ebx = edx.wrapping_sub(esi) ^ (esi >> 16) ^ (esi << 16);
        esi = esi.wrapping_add(edi);
        edi = edi.wrapping_sub(ebx) ^ (ebx >> 13) ^ (ebx << 19);
        ebx = ebx.wrapping_add(esi);
        esi = esi.wrapping_sub(edi) ^ (edi >> 28) ^ (edi << 4);
        edi = edi.wrapping_add(ebx);
        i += 12;
    }

    let rem = bytes.len() - i;
    if rem == 0 {
        return ((esi as u64) << 32) | u64::from(eax);
    }

    let mut tail = [0u8; 12];
    tail[..rem].copy_from_slice(&bytes[i..i + rem]);
    esi = esi.wrapping_add(u32::from_le_bytes([tail[8], tail[9], tail[10], tail[11]]));
    edi = edi.wrapping_add(u32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]));
    ebx = ebx.wrapping_add(u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]));

    esi = (esi ^ edi).wrapping_sub((edi >> 18) ^ (edi << 14));
    let ecx = (esi ^ ebx).wrapping_sub((esi >> 21) ^ (esi << 11));
    edi = (edi ^ ecx).wrapping_sub((ecx >> 7) ^ (ecx << 25));
    esi = (esi ^ edi).wrapping_sub((edi >> 16) ^ (edi << 16));
    edx = (esi ^ ecx).wrapping_sub((esi >> 28) ^ (esi << 4));
    edi = (edi ^ edx).wrapping_sub((edx >> 18) ^ (edx << 14));
    eax = (esi ^ edi).wrapping_sub((edi >> 8) ^ (edi << 24));
    ((edi as u64) << 32) | u64::from(eax)
}

fn read_u16(file: &mut File) -> Result<u16, MapError> {
    let mut buf = [0u8; 2];
    file.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(file: &mut File) -> Result<u32, MapError> {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i32(file: &mut File) -> Result<i32, MapError> {
    Ok(read_u32(file)? as i32)
}

fn read_u64(file: &mut File) -> Result<u64, MapError> {
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i64(file: &mut File) -> Result<i64, MapError> {
    Ok(read_u64(file)? as i64)
}

/// Walks the block chain of a UOP package and returns every file it holds,
/// by the hash of its name. A package names no file in the clear, so a reader
/// finds a file only by hashing the name it expects.
fn read_directory(path: &Path) -> Result<HashMap<u64, UopIndex>, MapError> {
    let mut file = File::open(path)?;
    let magic = read_u32(&mut file)?;
    if magic != UOP_MAGIC {
        return Err(MapError::BadUop);
    }
    let _version = read_u32(&mut file)?;
    let _timestamp = read_u32(&mut file)?;
    let mut next_block = read_i64(&mut file)?;
    let _block_size = read_u32(&mut file)?;
    let _count = read_i32(&mut file)?;

    let mut hashes: HashMap<u64, UopIndex> = HashMap::new();
    let mut visited = HashSet::new();
    while next_block != 0 {
        if !visited.insert(next_block) {
            return Err(MapError::BadUop);
        }
        file.seek(SeekFrom::Start(next_block as u64))?;
        let files_count = read_i32(&mut file)?;
        next_block = read_i64(&mut file)?;
        for _ in 0..files_count {
            let offset = read_i64(&mut file)?;
            let header_len = read_i32(&mut file)?;
            let compressed_len = read_i32(&mut file)?;
            let decompressed_len = read_i32(&mut file)?;
            let hash = read_u64(&mut file)?;
            let _data_hash = read_u32(&mut file)?;
            let compression = read_u16(&mut file)?;
            if offset == 0 {
                continue;
            }
            hashes.insert(
                hash,
                UopIndex {
                    offset: (offset as u64).saturating_add(header_len as u64),
                    compressed_len: compressed_len as u32,
                    decompressed_len: decompressed_len as u32,
                    compression,
                },
            );
        }
    }

    if hashes.is_empty() {
        return Err(MapError::BadUop);
    }
    Ok(hashes)
}

/// Puts the files of a package in the order of the index each name carries.
/// The walk stops early once every file of the package has been named.
fn index_by_name(
    hashes: &HashMap<u64, UopIndex>,
    count: u32,
    name: impl Fn(u32) -> String,
) -> Vec<Option<UopIndex>> {
    let mut entries = Vec::new();
    let mut found = 0usize;
    for i in 0..count {
        if found == hashes.len() {
            break;
        }
        if let Some(entry) = hashes.get(&hash_filename(&name(i))).copied() {
            if i as usize >= entries.len() {
                entries.resize(i as usize + 1, None);
            }
            entries[i as usize] = Some(entry);
            found += 1;
        }
    }
    entries
}

pub fn load_map_entries(path: &Path, map_index: u8) -> Result<Vec<Option<UopIndex>>, MapError> {
    let hashes = read_directory(path)?;
    let count = hashes.len() as u32 + MAP_CHUNK_SLACK;
    let entries = index_by_name(&hashes, count, |chunk| map_uop_name(map_index, chunk));
    if entries.is_empty() {
        return Err(MapError::BadUop);
    }
    Ok(entries)
}

pub fn load_multi_entries(path: &Path) -> Result<Vec<Option<UopIndex>>, MapError> {
    let hashes = read_directory(path)?;
    let entries = index_by_name(&hashes, MULTI_ID_COUNT, multi_uop_name);
    if entries.is_empty() {
        return Err(MapError::BadUop);
    }
    Ok(entries)
}

pub fn decompress(raw: &[u8], compression: u16, dest_len: u32) -> Result<Vec<u8>, MapError> {
    match compression {
        UOP_COMPRESS_NONE => {
            let n = (dest_len as usize).min(raw.len());
            Ok(raw[..n].to_vec())
        }
        UOP_COMPRESS_ZLIB => {
            let mut out = Vec::new();
            let mut dec = ZlibDecoder::new(raw).take(u64::from(dest_len));
            dec.read_to_end(&mut out)?;
            Ok(out)
        }
        UOP_COMPRESS_ZLIB_BWT => Err(MapError::UnsupportedUop),
        _ => Err(MapError::UnsupportedUop),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_hash_is_seed() {
        assert_eq!(hash_filename(""), (u64::from(HASH_SEED) << 32));
    }

    #[test]
    fn map_chunk_name_is_stable() {
        assert_eq!(map_uop_name(0, 0), "build/map0legacymul/00000000.dat");
        assert_eq!(map_uop_name(1, 12), "build/map1legacymul/00000012.dat");
    }

    #[test]
    fn block_chain_cycle_is_bad_uop() {
        let dir = std::env::temp_dir().join(format!(
            "uoterm-uop-cycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("map0LegacyMUL.uop");
        let header_len: i64 = 28;
        let mut data = Vec::new();
        data.extend_from_slice(&UOP_MAGIC.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&header_len.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&header_len.to_le_bytes());
        std::fs::write(&path, data).unwrap();
        let err = load_map_entries(&path, 0).unwrap_err();
        assert!(matches!(err, MapError::BadUop));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
