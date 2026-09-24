//! The version of the client program in a client folder. A shard that
//! checks versions reads the same number from its own copy of the program,
//! so a client that draws with these files says this version.

use std::path::Path;
use uoterm_protocol::types::ClientVersion;

pub const CLIENT_PROGRAM_NAME: &str = "client.exe";
/// The key of the version block of a Windows program.
const VERSION_INFO_KEY: &str = "VS_VERSION_INFO";
/// From the start of the key to the file version of the fixed block.
const FILE_VERSION_OFFSET: usize = 42;
const WORD_BYTES: usize = 2;
/// The order of the four words: each half of the file version keeps its
/// low word first.
const MINOR_WORD: usize = 0;
const MAJOR_WORD: usize = 1;
const PATCH_WORD: usize = 2;
const REVISION_WORD: usize = 3;

/// The version of the client program in `dir`, or None when the folder has
/// no program or the program has no version block.
pub fn client_program_version(dir: &Path) -> Option<ClientVersion> {
    let bytes = std::fs::read(dir.join(CLIENT_PROGRAM_NAME)).ok()?;
    version_in(&bytes)
}

fn version_in(bytes: &[u8]) -> Option<ClientVersion> {
    let key: Vec<u8> = VERSION_INFO_KEY
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let start = bytes.windows(key.len()).position(|w| w == key)? + FILE_VERSION_OFFSET;
    let word = |index: usize| {
        let at = start + index * WORD_BYTES;
        bytes
            .get(at..at + WORD_BYTES)
            .map(|pair| u32::from(u16::from_le_bytes([pair[0], pair[1]])))
    };
    Some(ClientVersion::new(
        word(MAJOR_WORD)?,
        word(MINOR_WORD)?,
        word(REVISION_WORD)?,
        word(PATCH_WORD)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mul::client_data_dir_from_env;

    /// A program with a version block for `major.minor.revision.patch`.
    fn program(major: u16, minor: u16, revision: u16, patch: u16) -> Vec<u8> {
        let mut bytes = vec![0xCC; 64];
        let key_at = bytes.len();
        bytes.extend(VERSION_INFO_KEY.encode_utf16().flat_map(u16::to_le_bytes));
        bytes.resize(key_at + FILE_VERSION_OFFSET, 0);
        for word in [minor, major, patch, revision] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend([0xCC; 16]);
        bytes
    }

    #[test]
    fn the_version_block_gives_the_program_version() {
        assert_eq!(
            version_in(&program(7, 0, 116, 0)),
            Some(ClientVersion::new(7, 0, 116, 0))
        );
        assert_eq!(
            version_in(&program(5, 0, 9, 1)),
            Some(ClientVersion::new(5, 0, 9, 1))
        );
    }

    #[test]
    fn a_program_with_no_version_block_has_no_version() {
        assert_eq!(version_in(&[0xCC; 128]), None);
    }

    #[test]
    fn a_cut_version_block_has_no_version() {
        let mut bytes = program(7, 0, 116, 0);
        bytes.truncate(bytes.len() - 16 - WORD_BYTES);
        assert_eq!(version_in(&bytes), None);
    }

    #[test]
    fn a_folder_with_no_program_has_no_version() {
        let dir = std::env::temp_dir().join("uoterm-no-client-program");
        assert_eq!(client_program_version(&dir), None);
    }

    #[test]
    fn the_client_files_name_a_modern_version() {
        let Some(dir) = client_data_dir_from_env() else {
            eprintln!("skipped: UOTERM_TEST_UOPATH is not set");
            return;
        };
        if !dir.join(CLIENT_PROGRAM_NAME).exists() {
            eprintln!("skipped: the client folder has no {CLIENT_PROGRAM_NAME}");
            return;
        }
        let version = client_program_version(&dir).expect("a version block");
        assert!(version.major >= 7, "{}", version.as_string());
    }
}
