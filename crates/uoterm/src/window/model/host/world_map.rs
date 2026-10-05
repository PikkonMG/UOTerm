//! The `map` folder of the config folder: the marker files and zone files
//! of the player, read and written here. Their text is read and made by
//! the rules of `uoterm_view::model::world_map`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uoterm_view::model::world_map::{
    self, csv_line, kept_marker, markers_csv, parse_markers, parse_zones_json, removed_marker,
    MapFile, MapFolder, Marker, MarkerChange, MarkerFile, ZoneFile, USER_MARKERS,
    USER_MARKERS_EXTENSION, USER_MARKERS_MOST,
};

const MAP_DIR: &str = "map";

/// One change of the player's own marker file at a time in this process:
/// the window and each page change it through `change_user_markers`.
static USER_MARKERS_LOCK: Mutex<()> = Mutex::new(());

/// The folder of the marker and zone files.
pub fn map_dir() -> PathBuf {
    map_dir_in(&uoterm_runtime::config::config_dir())
}

/// The folder of the marker and zone files in a config folder.
pub fn map_dir_in(config_dir: &Path) -> PathBuf {
    config_dir.join(MAP_DIR)
}

/// Every marker file and zone file of the folder.
pub fn map_folder(dir: &Path) -> MapFolder {
    MapFolder {
        markers: load_markers(dir),
        zones: load_zones(dir),
    }
}

/// The names of the files of a folder. Empty when it does not read.
fn names_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect()
}

/// The names of every marker file and every zone file of the folder,
/// hidden or not, sorted, for the menu that hides and shows them.
pub fn file_names(dir: &Path) -> (Vec<String>, Vec<String>) {
    let names = names_in(dir);
    world_map::file_names(names.iter().map(String::as_str))
}

/// Every marker file of the folder.
pub fn load_markers(dir: &Path) -> Vec<MarkerFile> {
    let mut files: Vec<MarkerFile> = names_in(dir)
        .iter()
        .filter_map(|name| {
            let stem = MapFile::of(name)?.stem();
            let text = std::fs::read_to_string(dir.join(name)).ok()?;
            Some(MarkerFile {
                name: stem.to_string(),
                markers: parse_markers(name, &text)?,
            })
        })
        .collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

/// The player's own marker file.
fn user_markers_file(dir: &Path) -> PathBuf {
    dir.join(USER_MARKERS)
        .with_extension(USER_MARKERS_EXTENSION)
}

/// Adds a marker to the player's own marker file.
fn add_user_marker(dir: &Path, marker: &Marker) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(user_markers_file(dir))?;
    writeln!(file, "{}", csv_line(marker))
}

/// Writes the player's own marker file again with these markers, after he
/// changed or removed some.
fn save_user_markers(dir: &Path, markers: &[Marker]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(user_markers_file(dir), markers_csv(markers))
}

/// The markers of the player's own file.
pub fn user_markers(dir: &Path) -> Vec<Marker> {
    load_markers(dir)
        .into_iter()
        .find(|file| file.name == USER_MARKERS)
        .map(|file| file.markers)
        .unwrap_or_default()
}

/// Writes a marker to the player's own file: a new one, or in the place of
/// the one at `editing`.
fn keep_user_marker(dir: &Path, editing: Option<usize>, marker: Marker) -> std::io::Result<()> {
    if editing.is_none() {
        return add_user_marker(dir, &marker);
    }
    save_user_markers(dir, &kept_marker(user_markers(dir), editing, marker))
}

/// Takes the marker at a place out of the player's own file.
fn remove_user_marker(dir: &Path, at: usize) -> std::io::Result<()> {
    save_user_markers(dir, &removed_marker(user_markers(dir), at))
}

/// Why a change of the player's own marker file was not made.
#[derive(Debug)]
pub enum MarkerFault {
    /// The marker it writes is not valid.
    Invalid,
    /// The file no longer holds the marker the change expects: the window
    /// or another page changed it.
    Stale,
    /// The file holds `USER_MARKERS_MOST` markers: no more are added.
    Full,
    Write(std::io::Error),
}

impl std::fmt::Display for MarkerFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str(world_map::WORDS_INVALID_MARKER),
            Self::Stale => f.write_str(world_map::WORDS_STALE_MARKERS),
            Self::Full => f.write_str(world_map::WORDS_FULL_MARKERS),
            Self::Write(error) => error.fmt(f),
        }
    }
}

/// Makes a change of the player's own marker file, when the marker it
/// writes is valid, the file holds the marker it expects, and an add finds
/// room. The file is read and written under one lock, so two changes at
/// once do not write over each other.
pub fn change_user_markers(dir: &Path, change: &MarkerChange) -> Result<(), MarkerFault> {
    if !change.is_valid() {
        return Err(MarkerFault::Invalid);
    }
    let _alone = USER_MARKERS_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let markers = user_markers(dir);
    if let Some((at, expected)) = change.expected() {
        if markers.get(at) != Some(expected) {
            return Err(MarkerFault::Stale);
        }
    }
    if matches!(change, MarkerChange::Add(_)) && markers.len() >= USER_MARKERS_MOST {
        return Err(MarkerFault::Full);
    }
    let written = match change {
        MarkerChange::Add(marker) => keep_user_marker(dir, None, marker.clone()),
        MarkerChange::Keep { at, marker, .. } => keep_user_marker(dir, Some(*at), marker.clone()),
        MarkerChange::Remove { at, .. } => remove_user_marker(dir, *at),
    };
    written.map_err(MarkerFault::Write)
}

/// Every zone file of the folder.
pub fn load_zones(dir: &Path) -> Vec<ZoneFile> {
    names_in(dir)
        .iter()
        .filter_map(|name| {
            let MapFile::Zones { stem } = MapFile::of(name)? else {
                return None;
            };
            parse_zones_json(stem, &std::fs::read_to_string(dir.join(name)).ok()?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_view::model::world_map::NEW_MARKER_COLOR;

    fn named(name: &str) -> Marker {
        Marker {
            name: name.into(),
            map: 0,
            x: 0,
            y: 0,
            icon: String::new(),
            color: String::new(),
        }
    }

    #[test]
    fn the_user_file_is_kept_changed_and_cut() {
        let dir =
            std::env::temp_dir().join(format!("uoterm-user-markers-{}", uuid::Uuid::new_v4()));
        assert!(user_markers(&dir).is_empty());
        keep_user_marker(&dir, None, named("Camp")).unwrap();
        keep_user_marker(&dir, None, named("Mine")).unwrap();
        std::fs::write(dir.join("towns.csv"), "1,1,0,Town\n").unwrap();
        keep_user_marker(&dir, Some(1), named("Cave")).unwrap();
        let names: Vec<String> = user_markers(&dir).into_iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["Camp".to_string(), "Cave".to_string()]);
        remove_user_marker(&dir, 0).unwrap();
        assert_eq!(user_markers(&dir).len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_change_is_made_only_when_valid_and_when_the_file_holds_what_it_expects() {
        let dir =
            std::env::temp_dir().join(format!("uoterm-change-markers-{}", uuid::Uuid::new_v4()));
        let camp = Marker {
            color: NEW_MARKER_COLOR.into(),
            ..named("Camp")
        };
        change_user_markers(&dir, &MarkerChange::Add(camp.clone())).unwrap();
        let bad = MarkerChange::Add(named(""));
        assert!(matches!(
            change_user_markers(&dir, &bad),
            Err(MarkerFault::Invalid)
        ));
        let mine = Marker {
            name: "Mine".into(),
            ..camp.clone()
        };
        let stale = MarkerChange::Keep {
            at: 0,
            marker: mine.clone(),
            expected: mine.clone(),
        };
        assert!(matches!(
            change_user_markers(&dir, &stale),
            Err(MarkerFault::Stale)
        ));
        let keep = MarkerChange::Keep {
            at: 0,
            marker: mine.clone(),
            expected: camp,
        };
        change_user_markers(&dir, &keep).unwrap();
        assert_eq!(user_markers(&dir), vec![mine.clone()]);
        let remove = MarkerChange::Remove {
            at: 0,
            expected: mine,
        };
        change_user_markers(&dir, &remove).unwrap();
        assert!(user_markers(&dir).is_empty());
        let full = vec![named_marker("Spot"); USER_MARKERS_MOST];
        save_user_markers(&dir, &full).unwrap();
        let one_more = MarkerChange::Add(named_marker("More"));
        assert!(matches!(
            change_user_markers(&dir, &one_more),
            Err(MarkerFault::Full)
        ));
        let line_break = MarkerChange::Add(named_marker("Camp\nMine"));
        assert!(matches!(
            change_user_markers(&dir, &line_break),
            Err(MarkerFault::Invalid)
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A valid marker of the own file named `name`.
    fn named_marker(name: &str) -> Marker {
        Marker {
            color: NEW_MARKER_COLOR.into(),
            ..named(name)
        }
    }

    #[test]
    fn the_marker_files_of_the_folder_read_and_the_user_file_grows() {
        let dir = std::env::temp_dir().join(format!("uoterm-map-{}", uuid::Uuid::new_v4()));
        let place = Marker {
            name: "My, camp".into(),
            map: 1,
            x: 10,
            y: 20,
            icon: String::new(),
            color: NEW_MARKER_COLOR.into(),
        };
        add_user_marker(&dir, &place).unwrap();
        std::fs::write(
            dir.join("towns.map"),
            "3\n+BANK: 1434 1699 1 Britain Bank\n",
        )
        .unwrap();
        std::fs::write(dir.join("old.csv"), "1,1,0,Hidden\n").unwrap();
        let files = load_markers(&dir);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["old", "towns", "user-markers"]);
        assert_eq!(files[2].markers[0].name, "My  camp");
        assert_eq!(files[2].name, USER_MARKERS);
        assert_eq!(files[1].markers[0].x, 1434);
        let mut kept = files[2].markers.clone();
        kept[0].color = "red".into();
        kept.push(Marker {
            name: "Mine".into(),
            icon: "exit".into(),
            ..kept[0].clone()
        });
        save_user_markers(&dir, &kept).unwrap();
        assert_eq!(user_markers(&dir), kept, "the file keeps each field");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_zone_files_of_the_folder_read() {
        let text = r#"{ "MapIndex": 1, "Zones": [ { "Label": "Britain", "Color": "red",
            "Polygon": [[0, 0], [10, 0], [10, 10], [0, 10]] } ] }"#;
        let dir = std::env::temp_dir().join(format!("uoterm-zones-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("towns.zones.json"), text).unwrap();
        std::fs::write(dir.join("camps.csv"), "1,1,0,Camp\n").unwrap();
        assert_eq!(
            file_names(&dir),
            (vec!["camps".to_string()], vec!["towns".to_string()])
        );
        assert_eq!(load_zones(&dir).len(), 1);
        assert_eq!(map_folder(&dir).markers.len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
