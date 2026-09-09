//! Append-only event log for `uoterm harvest log`.

use crate::config::{data_dir, JOURNAL_HARVEST_NAME};
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use uoterm_world::Event;

pub fn append(session_id: &str, event: &Event) {
    let dir = data_dir();
    if create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(JOURNAL_HARVEST_NAME);
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let at = Utc
        .timestamp_millis_opt(event.unix_ms as i64)
        .single()
        .unwrap_or_else(Utc::now);
    let kind = serde_json::to_value(event.kind).unwrap_or(serde_json::Value::Null);
    let line = json!({
        "at": at.to_rfc3339(),
        "session": session_id,
        "kind": kind,
        "serial": event.serial.map(|s| s.0),
        "text": event.text,
    });
    let _ = writeln!(file, "{line}");
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uoterm_world::EventKind;

    #[test]
    fn event_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_value(EventKind::LoggedIn).unwrap(),
            json!("logged_in")
        );
        assert_eq!(
            serde_json::to_value(EventKind::ItemAdded).unwrap(),
            json!("item_added")
        );
        assert_eq!(
            serde_json::to_value(EventKind::PkFlag).unwrap(),
            json!("pk_flag")
        );
        assert_eq!(
            serde_json::to_value(EventKind::PathFailed).unwrap(),
            json!("path_failed")
        );
        assert_eq!(
            serde_json::to_value(EventKind::CombatantChanged).unwrap(),
            json!("combatant_changed")
        );
        assert_eq!(
            serde_json::to_value(EventKind::LiftRejected).unwrap(),
            json!("lift_rejected")
        );
    }
}
