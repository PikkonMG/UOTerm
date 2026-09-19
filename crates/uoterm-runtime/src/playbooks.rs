//! Agent playbooks shipped as MCP resources (`uo://playbook/{name}`).

use serde_json::{json, Value};

pub const PLAYBOOK_URI_PREFIX: &str = "uo://playbook/";
pub const PLAYBOOK_MIME: &str = "text/markdown";
pub const SESSION_MIME: &str = "application/json";
const SESSION_URI_PREFIX: &str = "uo://session/";
const SESSION_URI_SUFFIX: &str = "/state";

#[derive(Clone, Copy, Debug)]
pub struct Playbook {
    pub name: &'static str,
    pub body: &'static str,
}

const PLAYBOOKS: &[Playbook] = &[
    Playbook {
        name: "bank",
        body: include_str!("../../../docs/playbooks/bank.md"),
    },
    Playbook {
        name: "buy",
        body: include_str!("../../../docs/playbooks/buy.md"),
    },
    Playbook {
        name: "containers",
        body: include_str!("../../../docs/playbooks/containers.md"),
    },
    Playbook {
        name: "death",
        body: include_str!("../../../docs/playbooks/death.md"),
    },
    Playbook {
        name: "driver",
        body: include_str!("../../../docs/playbooks/driver.md"),
    },
    Playbook {
        name: "dungeon",
        body: include_str!("../../../docs/playbooks/dungeon.md"),
    },
    Playbook {
        name: "equip",
        body: include_str!("../../../docs/playbooks/equip.md"),
    },
    Playbook {
        name: "hunt",
        body: include_str!("../../../docs/playbooks/hunt.md"),
    },
    Playbook {
        name: "inspect",
        body: include_str!("../../../docs/playbooks/inspect.md"),
    },
    Playbook {
        name: "login",
        body: include_str!("../../../docs/playbooks/login.md"),
    },
    Playbook {
        name: "loot",
        body: include_str!("../../../docs/playbooks/loot.md"),
    },
    Playbook {
        name: "moongate",
        body: include_str!("../../../docs/playbooks/moongate.md"),
    },
    Playbook {
        name: "mounts",
        body: include_str!("../../../docs/playbooks/mounts.md"),
    },
    Playbook {
        name: "navigation",
        body: include_str!("../../../docs/playbooks/navigation.md"),
    },
    Playbook {
        name: "runebook",
        body: include_str!("../../../docs/playbooks/runebook.md"),
    },
    Playbook {
        name: "sell",
        body: include_str!("../../../docs/playbooks/sell.md"),
    },
    Playbook {
        name: "talk",
        body: include_str!("../../../docs/playbooks/talk.md"),
    },
    Playbook {
        name: "walk",
        body: include_str!("../../../docs/playbooks/walk.md"),
    },
];

pub fn all() -> &'static [Playbook] {
    PLAYBOOKS
}

pub fn get(name: &str) -> Option<&'static Playbook> {
    PLAYBOOKS.iter().find(|p| p.name == name)
}

pub fn uri(name: &str) -> String {
    format!("{PLAYBOOK_URI_PREFIX}{name}")
}

pub fn parse_uri(uri: &str) -> Option<&str> {
    let name = uri.strip_prefix(PLAYBOOK_URI_PREFIX)?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub fn list_resources() -> Vec<Value> {
    PLAYBOOKS
        .iter()
        .map(|p| {
            json!({
                "uri": uri(p.name),
                "name": format!("playbook {}", p.name),
                "mimeType": PLAYBOOK_MIME
            })
        })
        .collect()
}

pub fn mcp_resources(session_ids: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<Value> {
    let mut resources = list_resources();
    for sid in session_ids {
        let sid = sid.as_ref();
        resources.push(json!({
            "uri": format!("{SESSION_URI_PREFIX}{sid}{SESSION_URI_SUFFIX}"),
            "name": format!("session {sid} state"),
            "mimeType": SESSION_MIME
        }));
    }
    resources
}

pub fn read_playbook(uri: &str) -> Option<(&'static str, &'static str)> {
    let name = parse_uri(uri)?;
    let book = get(name)?;
    Some((PLAYBOOK_MIME, book.body))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIRED: &[&str] = &[
        "bank",
        "buy",
        "containers",
        "death",
        "driver",
        "dungeon",
        "equip",
        "hunt",
        "inspect",
        "login",
        "loot",
        "moongate",
        "mounts",
        "navigation",
        "runebook",
        "sell",
        "talk",
        "walk",
    ];

    #[test]
    fn playbooks_cover_the_agent_loop() {
        let names: Vec<&str> = all().iter().map(|p| p.name).collect();
        assert_eq!(names.len(), REQUIRED.len());
        for name in REQUIRED {
            assert!(names.contains(name), "missing playbook {name}");
            let body = get(name).expect(name).body;
            assert!(body.contains('#'), "{name} is empty");
        }
        assert!(get("hunt").unwrap().body.contains("job_start"));
        assert!(get("walk").unwrap().body.contains("job_start"));
        assert!(get("driver").unwrap().body.contains("next_event"));
        assert!(get("inspect").unwrap().body.contains("observe"));
        assert!(!get("inspect").unwrap().body.contains("`look` or"));
        assert!(get("hunt").unwrap().body.contains("replace"));
        assert!(get("walk").unwrap().body.contains("hostile"));
        assert_eq!(parse_uri("uo://playbook/hunt"), Some("hunt"));
        assert!(parse_uri("uo://playbook/").is_none());
        assert!(parse_uri("uo://session/s1/state").is_none());
        assert!(get("nope").is_none());
        let (mime, text) = read_playbook("uo://playbook/login").unwrap();
        assert_eq!(mime, PLAYBOOK_MIME);
        assert!(text.contains("UO_PASS"));
        assert!(text.contains("--era modern"));
        let listed = mcp_resources(["s1"]);
        assert!(listed
            .iter()
            .any(|r| r["uri"] == "uo://playbook/hunt" && r["mimeType"] == PLAYBOOK_MIME));
        assert!(listed
            .iter()
            .any(|r| r["uri"] == "uo://session/s1/state" && r["mimeType"] == SESSION_MIME));
    }
}
