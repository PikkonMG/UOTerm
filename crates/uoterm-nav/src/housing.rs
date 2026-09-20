//! The parts a player can build his house from: walls, floors, doors,
//! stairs, roofs, archways and teleporters. The client files list them in
//! text files, one line for each style, with the graphics of its pieces.
//!
//! Each file names its columns in a header row. A column that is not the
//! category, the style, the text number or the feature mask is a piece.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The files of the catalog, with the kind of part each one holds.
const FILES: [(&str, HousePartKind); 7] = [
    ("walls.txt", HousePartKind::Wall),
    ("floors.txt", HousePartKind::Floor),
    ("doors.txt", HousePartKind::Door),
    ("stairs.txt", HousePartKind::Stair),
    ("roof.txt", HousePartKind::Roof),
    ("misc.txt", HousePartKind::Misc),
    ("teleprts.txt", HousePartKind::Teleporter),
];
/// The columns that are not pieces.
const NOT_PIECES: [&str; 5] = ["category", "style", "tid", "featuremask", "comment"];
/// A static graphic is under this. A larger number is a text number.
const GRAPHIC_MAX: u32 = 0x4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HousePartKind {
    Wall,
    Floor,
    Door,
    Stair,
    Roof,
    /// Archways, chimneys and the like.
    Misc,
    Teleporter,
}

impl HousePartKind {
    /// The word a tool and a window use for the kind.
    pub fn word(self) -> &'static str {
        match self {
            Self::Wall => "wall",
            Self::Floor => "floor",
            Self::Door => "door",
            Self::Stair => "stair",
            Self::Roof => "roof",
            Self::Misc => "misc",
            Self::Teleporter => "teleporter",
        }
    }
}

/// One style of one kind, such as "Dark Wood Standard Walls", with the
/// graphics of each of its pieces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HousePart {
    pub kind: HousePartKind,
    pub name: String,
    /// The piece graphics, in the order of the columns of the file.
    pub pieces: Vec<u16>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HouseCatalog {
    parts: Vec<HousePart>,
}

/// The places of the columns that hold pieces, from the header row.
fn piece_columns(header: &str) -> Vec<usize> {
    header
        .split('\t')
        .enumerate()
        .filter(|(_, name)| {
            let name = name.trim().to_ascii_lowercase();
            !name.is_empty() && !NOT_PIECES.contains(&name.as_str())
        })
        .map(|(at, _)| at)
        .collect()
}

/// The header row of a file: the first row whose fields are words, not
/// numbers, and that names a category.
fn header_row(text: &str) -> Option<&str> {
    text.lines().find(|line| {
        line.trim_start()
            .to_ascii_lowercase()
            .starts_with("category")
    })
}

fn parse_file(text: &str, kind: HousePartKind) -> Vec<HousePart> {
    let Some(header) = header_row(text) else {
        return Vec::new();
    };
    let columns = piece_columns(header);
    let mut parts = Vec::new();
    for line in text.lines().skip_while(|line| *line != header).skip(1) {
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        // A line of the catalog starts with its category number.
        if fields
            .first()
            .is_none_or(|first| first.parse::<u32>().is_err())
        {
            continue;
        }
        let pieces: Vec<u16> = columns
            .iter()
            .filter_map(|at| fields.get(*at)?.parse::<u32>().ok())
            .filter(|graphic| *graphic > 0 && *graphic < GRAPHIC_MAX)
            .map(|graphic| graphic as u16)
            .collect();
        let name = fields.last().copied().unwrap_or_default().trim();
        if pieces.is_empty() || name.is_empty() {
            continue;
        }
        parts.push(HousePart {
            kind,
            name: name.to_string(),
            pieces,
        });
    }
    parts
}

impl HouseCatalog {
    /// The catalog of the client files. Empty when they hold none.
    pub fn open(uopath: impl AsRef<Path>) -> Self {
        let dir = uopath.as_ref();
        let parts = FILES
            .iter()
            .flat_map(|(name, kind)| {
                std::fs::read(dir.join(name))
                    .map(|raw| parse_file(&String::from_utf8_lossy(&raw), *kind))
                    .unwrap_or_default()
            })
            .collect();
        Self { parts }
    }

    pub fn parts(&self) -> &[HousePart] {
        &self.parts
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// The styles of one kind of part.
    pub fn of_kind(&self, kind: HousePartKind) -> impl Iterator<Item = &HousePart> {
        self.parts.iter().filter(move |part| part.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALLS: &str = "int\tint\tint\tint\tint\tint\tint\tint\tint\tint\tint\tstring\n\
        Category\tStyle\tTID\tSouth1\tSouth2\tSouth3\tCorner\tEast1\tEast2\tEast3\tPost\tFeatureMask\tComment\n\
        0\t0\t1060054\t10\t7\t12\t6\t13\t8\t11\t9\t0\tDark Wood Standard Walls\n\
        1\t1\t1060055\t20\t0\t22\t16\t23\t18\t21\t19\t0\tStone Walls\n";

    #[test]
    fn a_catalog_line_gives_its_name_and_the_graphics_of_its_pieces() {
        let parts = parse_file(WALLS, HousePartKind::Wall);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "Dark Wood Standard Walls");
        assert_eq!(parts[0].pieces, vec![10, 7, 12, 6, 13, 8, 11, 9]);
        assert_eq!(parts[0].kind, HousePartKind::Wall);
        // A piece of zero is no piece.
        assert_eq!(parts[1].pieces, vec![20, 22, 16, 23, 18, 21, 19]);
    }

    #[test]
    fn the_style_the_text_number_and_the_mask_are_not_pieces() {
        let columns = piece_columns("Category\tStyle\tTID\tSouth1\tFeatureMask\tComment");
        assert_eq!(columns, vec![3]);
        let parts = parse_file(WALLS, HousePartKind::Wall);
        assert!(!parts[0].pieces.contains(&0), "the mask is not a piece");
        assert!(
            parts[0]
                .pieces
                .iter()
                .all(|piece| u32::from(*piece) < GRAPHIC_MAX),
            "a text number is not a piece"
        );
    }

    #[test]
    fn a_file_with_no_header_and_a_line_of_words_give_no_parts() {
        assert!(parse_file("no header here\n1 2 3\n", HousePartKind::Floor).is_empty());
        let with_junk = format!("{WALLS}not a number\tat all\t\tComment line\n");
        assert_eq!(parse_file(&with_junk, HousePartKind::Wall).len(), 2);
    }

    #[test]
    fn the_real_catalog_has_walls_and_floors_when_client_files_are_here() {
        let Some(dir) = crate::mul::client_data_dir_from_env() else {
            return;
        };
        let catalog = HouseCatalog::open(&dir);
        assert!(!catalog.is_empty());
        let walls: Vec<&HousePart> = catalog.of_kind(HousePartKind::Wall).collect();
        assert!(walls.len() > 10, "the client lists many wall styles");
        assert!(walls.iter().all(|wall| !wall.pieces.is_empty()));
        assert!(catalog.of_kind(HousePartKind::Floor).count() > 5);
        assert!(catalog.of_kind(HousePartKind::Stair).count() > 5);
    }
}
