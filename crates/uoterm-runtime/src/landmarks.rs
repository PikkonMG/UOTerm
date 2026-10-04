//! Reads the marker file the user points the config at. The places, and the
//! two formats a file may hold, are in `uoterm_world::landmarks`.

use std::path::Path;

pub use uoterm_world::landmarks::{Landmark, Landmarks};

/// Reads a marker file. The format comes from the file name.
pub fn load(path: &Path) -> std::io::Result<Landmarks> {
    let text = std::fs::read_to_string(path)?;
    Ok(Landmarks::from_text(&path.to_string_lossy(), &text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_file_is_read_by_the_format_of_its_name() {
        let folder = std::env::temp_dir().join(format!("uoterm-landmarks-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let uoam = folder.join("markers.map");
        std::fs::write(&uoam, "3\n+BANK: 3486 2571 1 New Haven Bank \n").unwrap();
        let lua = folder.join("Waypoints.lua");
        std::fs::write(
            &lua,
            "Waypoints.Facet[0] = {\n{x=\"1434\", y=\"1699\", z=\"0\", Name=\"Britain Bank\"},\n}",
        )
        .unwrap();
        assert_eq!(load(&uoam).unwrap().find(Some("bank"), Some(1)).len(), 1);
        assert_eq!(load(&lua).unwrap().find(Some("bank"), Some(0)).len(), 1);
        assert!(load(&folder.join("missing.map")).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
