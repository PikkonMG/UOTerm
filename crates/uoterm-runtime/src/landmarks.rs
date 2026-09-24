//! Named places the agent can travel to, read from a marker file the user
//! supplies. Nothing here ships with the client: the user points the config
//! at their own file, the same way they point it at the client data files.
//!
//! Two marker formats are read, both of which number facets the way the game
//! numbers its maps (0 Felucca, 1 Trammel, 2 Ilshenar, 3 Malas, 4 Tokuno,
//! 5 Ter Mur):
//!
//! * UO Auto Map `.map`: one marker a line, `[+-]TYPE: x y facet name`.
//! * Ultima Mapper `Waypoints.lua`: a Lua table per facet, each row
//!   `{x="..", y="..", z="..", ..., Name=".."}`.
//!
//! A moongate, for one, is not in the map files: the shard drops it in as a
//! live item. So the agent walks to the marker, then finds the gate item and
//! steps on it.

use std::collections::BTreeMap;
use std::path::Path;

use uoterm_protocol::Point3;

/// The smallest static Z and the largest, so a marker Z that will not fit the
/// map coordinate is held at the nearest edge instead of wrapping.
const Z_MIN: i32 = i8::MIN as i32;
const Z_MAX: i32 = i8::MAX as i32;

/// One named place: where it is, on which map, and the marker word that
/// labelled it (`MOONGATE`, `BANK`, ...), blank when the format carries none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Landmark {
    pub name: String,
    pub map: u8,
    pub at: Point3,
    pub kind: String,
}

/// Every landmark read from one marker file.
#[derive(Clone, Debug, Default)]
pub struct Landmarks {
    list: Vec<Landmark>,
}

impl Landmarks {
    /// Reads a marker file, choosing the format by its name: `.lua` is read as
    /// an Ultima Mapper waypoint table, anything else as a UO Auto Map file.
    /// Lines that do not parse are skipped, so a stray header or comment does
    /// no harm.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let is_lua = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lua"));
        Ok(if is_lua {
            Self::parse_waypoints_lua(&text)
        } else {
            Self::parse_uoam_map(&text)
        })
    }

    /// Parses the UO Auto Map text format.
    pub fn parse_uoam_map(text: &str) -> Self {
        let list = text.lines().filter_map(parse_uoam_line).collect();
        Self { list }
    }

    /// Parses the Ultima Mapper `Waypoints.lua` text format.
    pub fn parse_waypoints_lua(text: &str) -> Self {
        let mut list = Vec::new();
        let mut facet: Option<u8> = None;
        for line in text.lines() {
            if let Some(f) = parse_lua_facet_header(line) {
                facet = Some(f);
                continue;
            }
            let Some(map) = facet else { continue };
            if let Some(mark) = parse_lua_entry(line, map) {
                list.push(mark);
            }
        }
        Self { list }
    }

    /// The landmarks whose name holds `name` (ignoring case) and, when `map`
    /// is given, that stand on that map. An empty `name` matches every one.
    /// The result keeps the file order.
    pub fn find(&self, name: Option<&str>, map: Option<u8>) -> Vec<&Landmark> {
        let needle = name.map(str::to_ascii_lowercase);
        self.list
            .iter()
            .filter(|m| map.is_none_or(|want| m.map == want))
            .filter(|m| {
                needle
                    .as_deref()
                    .is_none_or(|n| n.is_empty() || m.name.to_ascii_lowercase().contains(n))
            })
            .collect()
    }

    /// [`Landmarks::find`], narrowed to a kind: a landmark whose marker word
    /// holds `kind`, or, for one the file gave no word, whose name holds it.
    pub fn find_of_kind(
        &self,
        name: Option<&str>,
        kind: Option<&str>,
        map: Option<u8>,
    ) -> Vec<&Landmark> {
        let wanted = kind
            .map(|k| k.trim().to_ascii_lowercase())
            .filter(|k| !k.is_empty());
        self.find(name, map)
            .into_iter()
            .filter(|m| {
                wanted.as_deref().is_none_or(|k| {
                    if m.kind.is_empty() {
                        m.name.to_ascii_lowercase().contains(k)
                    } else {
                        m.kind.to_ascii_lowercase().contains(k)
                    }
                })
            })
            .collect()
    }

    /// How many landmarks of each marker word were read, the blank word
    /// among them.
    pub fn kinds(&self) -> BTreeMap<String, usize> {
        let mut kinds = BTreeMap::new();
        for mark in &self.list {
            *kinds.entry(mark.kind.to_ascii_lowercase()).or_insert(0) += 1;
        }
        kinds
    }

    /// How many landmarks were read on each map.
    pub fn per_map(&self) -> BTreeMap<u8, usize> {
        let mut maps = BTreeMap::new();
        for mark in &self.list {
            *maps.entry(mark.map).or_insert(0) += 1;
        }
        maps
    }

    /// How many landmarks were read.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// True when no landmark was read.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// Fits a marker's Z, which a file may give wider than a map coordinate, into
/// the map coordinate's range.
fn clamp_z(z: i32) -> i8 {
    z.clamp(Z_MIN, Z_MAX) as i8
}

/// Reads one UO Auto Map line, `[+-]TYPE: x y facet name`, or `None` when the
/// line is a header, blank, or otherwise not a marker.
fn parse_uoam_line(line: &str) -> Option<Landmark> {
    let line = line.trim();
    let line = line.strip_prefix(['+', '-']).unwrap_or(line);
    let (kind, rest) = line.split_once(':')?;
    let kind = kind.trim();
    if kind.is_empty() || kind.contains(char::is_whitespace) {
        return None;
    }
    let mut parts = rest.split_whitespace();
    let x: u16 = parts.next()?.parse().ok()?;
    let y: u16 = parts.next()?.parse().ok()?;
    let map: u8 = parts.next()?.parse().ok()?;
    let name = parts.collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return None;
    }
    Some(Landmark {
        name,
        map,
        at: Point3 { x, y, z: 0 },
        kind: kind.to_string(),
    })
}

/// Reads the facet a `Waypoints.Facet[N] = {` line opens.
fn parse_lua_facet_header(line: &str) -> Option<u8> {
    let rest = line.trim().strip_prefix("Waypoints.Facet[")?;
    let (digits, _) = rest.split_once(']')?;
    digits.trim().parse().ok()
}

/// Reads one `Waypoints.lua` table row into a landmark on the given map.
fn parse_lua_entry(line: &str, map: u8) -> Option<Landmark> {
    let name = lua_attr(line, "Name")?;
    let x: u16 = lua_attr(line, "x")?.parse().ok()?;
    let y: u16 = lua_attr(line, "y")?.parse().ok()?;
    let z = lua_attr(line, "z")
        .and_then(|v| v.parse::<i32>().ok())
        .map(clamp_z)
        .unwrap_or(0);
    Some(Landmark {
        name,
        map,
        at: Point3 { x, y, z },
        kind: String::new(),
    })
}

/// Reads the value of a `key="value"` pair from a Lua table row. The key is
/// matched only where a non-word character comes before it, so `x` does not
/// match inside `Icon` or `Scale`.
fn lua_attr(line: &str, key: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(rel) = line[from..].find(key) {
        let start = from + rel;
        let before = start.checked_sub(1).map(|i| bytes[i]);
        let boundary = before.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_');
        let after = line[start + key.len()..].strip_prefix("=\"");
        match (boundary, after) {
            (true, Some(tail)) => return tail.split_once('"').map(|(v, _)| v.to_string()),
            _ => from = start + key.len(),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ground truth from a UO Auto Map marker set: the New Haven moongate on
    /// Trammel (map 1). The town moongate is not in the map files, so this
    /// marker is how the agent learns where to walk.
    #[test]
    fn uoam_reads_the_new_haven_moongate_on_trammel() {
        let text = "3\n\
            -EXIT: 5904 16 0 Hythloth \n\
            +MOONGATE: 3450 2677 1 New Haven Moongate \n\
            -BANK: 3486 2571 1 New Haven Bank \n";
        let marks = Landmarks::parse_uoam_map(text);
        let gate = marks.find(Some("new haven moongate"), None);
        assert_eq!(gate.len(), 1, "one gate: {marks:?}");
        assert_eq!(gate[0].map, 1);
        assert_eq!(
            gate[0].at,
            Point3 {
                x: 3450,
                y: 2677,
                z: 0
            }
        );
        assert_eq!(gate[0].kind, "MOONGATE");
    }

    #[test]
    fn a_kind_is_the_marker_word_or_else_part_of_the_name() {
        let text = "3\n\
            +MOONGATE: 3450 2677 1 New Haven Moongate \n\
            -BANK: 3486 2571 1 New Haven Bank \n\
            -BANK: 1434 1699 0 Britain Bank \n";
        let marks = Landmarks::parse_uoam_map(text);
        assert_eq!(marks.find_of_kind(None, Some("bank"), None).len(), 2);
        assert_eq!(marks.find_of_kind(None, Some("Bank"), Some(1)).len(), 1);
        assert_eq!(
            marks
                .find_of_kind(Some("haven"), Some("moongate"), None)
                .len(),
            1
        );
        assert_eq!(marks.kinds().get("bank"), Some(&2));
        assert_eq!(marks.per_map().get(&1), Some(&2));
        let lua = Landmarks::parse_waypoints_lua(
            "Waypoints.Facet[0] = {\n{x=\"1434\", y=\"1699\", z=\"0\", Name=\"Britain Bank\"},\n}",
        );
        assert_eq!(lua.find_of_kind(None, Some("bank"), None).len(), 1);
    }

    #[test]
    fn uoam_skips_the_header_and_blank_lines() {
        let text = "3\n\n+BANK: 3486 2571 1 New Haven Bank \n";
        let marks = Landmarks::parse_uoam_map(text);
        assert_eq!(marks.len(), 1);
    }

    /// Ground truth from Ultima Mapper's `Waypoints.lua`: the same gate, same
    /// map, read from the maintained format. Its facet numbering matches the
    /// game map index, so no remap is needed.
    #[test]
    fn waypoints_lua_reads_the_new_haven_moongate_on_trammel() {
        let text = "Waypoints.Facet[0] = {\n\
            \t{x=\"1336\", y=\"1997\", z=\"5\", type=\"15\", Name=\"Britain Moongate\", Icon=\"100053\", Scale=0.69};\n\
            }\n\
            Waypoints.Facet[1] = {\n\
            \t{x=\"3450\", y=\"2677\", z=\"26\", type=\"15\", Name=\"New Haven Moongate\", Icon=\"100053\", Scale=0.69};\n\
            }\n";
        let marks = Landmarks::parse_waypoints_lua(text);
        let gate = marks.find(Some("New Haven Moongate"), Some(1));
        assert_eq!(gate.len(), 1, "one gate on Trammel: {marks:?}");
        assert_eq!(gate[0].map, 1);
        assert_eq!(
            gate[0].at,
            Point3 {
                x: 3450,
                y: 2677,
                z: 26
            }
        );
    }

    #[test]
    fn waypoints_lua_keeps_the_britain_gate_on_felucca() {
        let text = "Waypoints.Facet[0] = {\n\
            \t{x=\"1336\", y=\"1997\", z=\"5\", type=\"15\", Name=\"Britain Moongate\", Icon=\"100053\"};\n\
            }\n";
        let marks = Landmarks::parse_waypoints_lua(text);
        let gate = marks.find(Some("britain"), None);
        assert_eq!(gate.len(), 1);
        assert_eq!(gate[0].map, 0);
        assert_eq!(
            gate[0].at,
            Point3 {
                x: 1336,
                y: 1997,
                z: 5
            }
        );
    }

    #[test]
    fn a_negative_z_is_read() {
        let text = "Waypoints.Facet[0] = {\n\
            \t{x=\"1828\", y=\"2948\", z=\"-20\", Name=\"Trinsic Moongate\"};\n\
            }\n";
        let marks = Landmarks::parse_waypoints_lua(text);
        assert_eq!(marks.find(None, None)[0].at.z, -20);
    }

    #[test]
    fn the_map_filter_holds_off_other_maps() {
        let text = "+MOONGATE: 3450 2677 1 New Haven Moongate \n\
            +MOONGATE: 1336 1997 0 Britain Moongate \n";
        let marks = Landmarks::parse_uoam_map(text);
        assert_eq!(marks.find(Some("moongate"), Some(1)).len(), 1);
        assert_eq!(marks.find(Some("moongate"), None).len(), 2);
    }

    #[test]
    fn an_empty_name_matches_every_landmark() {
        let text = "+BANK: 3486 2571 1 New Haven Bank \n";
        let marks = Landmarks::parse_uoam_map(text);
        assert_eq!(marks.find(Some(""), None).len(), 1);
        assert_eq!(marks.find(None, None).len(), 1);
    }
}
