//! The `map` folder of the config folder: the marker files and zone files
//! of the player, read and written here. Their text is read and made by
//! the rules of `uoterm_view::model::world_map`.

use std::path::{Path, PathBuf};
use uoterm_view::model::world_map::{
    self, csv_line, kept_marker, markers_csv, parse_markers, parse_zones_json, removed_marker,
    MapFile, Marker, MarkerFile, ZoneFile, USER_MARKERS, USER_MARKERS_EXTENSION,
};

const MAP_DIR: &str = "map";

/// The folder of the marker and zone files.
pub fn map_dir() -> PathBuf {
    uoterm_runtime::config::config_dir().join(MAP_DIR)
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

/// Every marker file of the folder, less those the World Map page hides.
pub fn load_markers(dir: &Path, hidden: &[String]) -> Vec<MarkerFile> {
    let mut files: Vec<MarkerFile> = names_in(dir)
        .iter()
        .filter_map(|name| {
            let stem = MapFile::of(name)?.stem();
            if world_map::is_hidden(hidden, stem) {
                return None;
            }
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
pub fn add_user_marker(dir: &Path, marker: &Marker) -> std::io::Result<()> {
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
pub fn save_user_markers(dir: &Path, markers: &[Marker]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(user_markers_file(dir), markers_csv(markers))
}

/// The markers of the player's own file.
pub fn user_markers(dir: &Path) -> Vec<Marker> {
    load_markers(dir, &[])
        .into_iter()
        .find(|file| file.name == USER_MARKERS)
        .map(|file| file.markers)
        .unwrap_or_default()
}

/// Writes a marker to the player's own file: a new one, or in the place of
/// the one at `editing`.
pub fn keep_user_marker(dir: &Path, editing: Option<usize>, marker: Marker) -> std::io::Result<()> {
    if editing.is_none() {
        return add_user_marker(dir, &marker);
    }
    save_user_markers(dir, &kept_marker(user_markers(dir), editing, marker))
}

/// Takes the marker at a place out of the player's own file.
pub fn remove_user_marker(dir: &Path, at: usize) -> std::io::Result<()> {
    save_user_markers(dir, &removed_marker(user_markers(dir), at))
}

/// Every zone file of the folder, less those the World Map page hides.
pub fn load_zones(dir: &Path, hidden: &[String]) -> Vec<ZoneFile> {
    names_in(dir)
        .iter()
        .filter_map(|name| {
            let MapFile::Zones { stem } = MapFile::of(name)? else {
                return None;
            };
            if world_map::is_hidden(hidden, stem) {
                return None;
            }
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
        let files = load_markers(&dir, &["old".into()]);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["towns", "user-markers"]);
        assert_eq!(files[1].markers[0].name, "My  camp");
        assert_eq!(files[1].name, USER_MARKERS);
        assert_eq!(files[0].markers[0].x, 1434);
        let mut kept = files[1].markers.clone();
        kept[0].color = "red".into();
        kept.push(Marker {
            name: "Mine".into(),
            icon: "exit".into(),
            ..kept[0].clone()
        });
        save_user_markers(&dir, &kept).unwrap();
        let again = load_markers(&dir, &["old".into(), "towns".into()]);
        assert_eq!(again[0].markers, kept, "the file keeps each field");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_zone_files_of_the_folder_read_and_hide() {
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
        assert_eq!(load_zones(&dir, &[]).len(), 1);
        assert!(load_zones(&dir, &["towns".into()]).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
