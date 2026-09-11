// The tool list is one large json! block; the default macro depth is too
// shallow for it.
#![recursion_limit = "256"]

pub mod api;
pub mod building;
pub mod config;
pub mod deposit;
pub mod error;
pub mod harvest;
pub mod loot;
pub mod manager;
pub mod mcp;
pub mod mock;
pub mod movement;
pub mod persona;
pub mod populate;
pub mod reflex;
pub mod scene;
pub mod session;
pub mod tools;

pub use config::{parse_encryption_mode, AppConfig, ConnectOptions, EncryptionMode, Profile};
pub use error::{Result, RuntimeError};
pub use manager::{FacetCache, Runtime};
pub use persona::Persona;
pub use scene::{look_around, Scene, SceneKind, SceneThing, SCENE_RADIUS};
pub use session::SessionHandle;
pub use tools::{ToolCall, ToolResult};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{
        TOOL_FIND_ITEMS, TOOL_JOURNAL_SEARCH, TOOL_MOVE_TO, TOOL_OBSERVE, TOOL_OPEN_DOOR, TOOL_SAY,
        TOOL_SET_GOAL, TOOL_TARGET, TOOL_UNEQUIP, TOOL_USE, TOOL_WAIT_TARGET, TOOL_WALK,
    };
    use uoterm_protocol::types::{GRAPHIC_HATCHET, GRAPHIC_LOGS};

    async fn connect_mock() -> (SessionHandle, mock::MockServer) {
        let server = mock::MockServer::start().await.unwrap();
        let rt = Runtime::new(4);
        let opts = ConnectOptions {
            host: server.addr.ip().to_string(),
            port: server.addr.port(),
            account: "test".into(),
            password: "test".into(),
            shard: None,
            character: mock::MOCK_CHAR.into(),
            version: uoterm_protocol::types::ClientVersion::T2A,
            era: uoterm_protocol::types::Era::T2a,
            uopath: None,
            persona: None,
            stay_on_socket: true,
            next_login_key: uoterm_protocol::types::LOGIN_NEXT_KEY_DEFAULT,
            encryption: Default::default(),
            obey_shard_rules: crate::config::OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: crate::config::ANSWER_WHEN_NAMED_DEFAULT,
            play_along: crate::config::PLAY_ALONG_DEFAULT,
        };
        let handle = rt.connect(opts).await.unwrap();
        for _ in 0..25 {
            if handle.logged_in() {
                return (handle, server);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("expected in-world after mock login");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mock_login_say_and_observe() {
        let (handle, _server) = connect_mock().await;
        let said = handle
            .call(ToolCall {
                name: TOOL_SAY.into(),
                args: serde_json::json!({ "text": "vendor buy" }),
            })
            .await;
        assert!(said.ok, "{said:?}");
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let obs = handle
            .call(ToolCall {
                name: TOOL_OBSERVE.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(obs.ok);
        let radar = obs
            .result
            .get("radar")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(radar.contains('@'));
        let journal = handle
            .call(ToolCall {
                name: TOOL_JOURNAL_SEARCH.into(),
                args: serde_json::json!({ "query": "vendor buy" }),
            })
            .await;
        assert!(journal.ok);
        let lines = journal.result.to_string();
        assert!(
            lines.contains("vendor buy"),
            "journal missing speech: {lines}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn find_hatchet_in_backpack() {
        let (handle, _server) = connect_mock().await;
        let mut text = String::new();
        for _ in 0..40 {
            let found = handle
                .call(ToolCall {
                    name: TOOL_FIND_ITEMS.into(),
                    args: serde_json::json!({ "graphic": GRAPHIC_HATCHET }),
                })
                .await;
            assert!(found.ok, "{found:?}");
            text = found.result.to_string();
            if text.contains(&format!("{}", mock::MOCK_HATCHET))
                || text.contains(&format!("{GRAPHIC_HATCHET}"))
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("hatchet missing: {text}");
    }

    /// Calls a tool until it answers something the test can use, or gives up.
    /// The mock sends the world in packets, so what the world holds depends on
    /// how much of it has arrived.
    async fn call_until(
        handle: &SessionHandle,
        name: &str,
        args: serde_json::Value,
        ready: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        const TRIES: usize = 40;
        const WAIT_MS: u64 = 50;
        let mut last = serde_json::Value::Null;
        for _ in 0..TRIES {
            let answer = handle
                .call(ToolCall {
                    name: name.into(),
                    args: args.clone(),
                })
                .await;
            assert!(answer.ok, "{answer:?}");
            last = answer.result;
            if ready(&last) {
                return last;
            }
            tokio::time::sleep(std::time::Duration::from_millis(WAIT_MS)).await;
        }
        panic!("{name} never answered what the test needs: {last}");
    }

    /// The pack the character carries holds the hatchet, and an agent that
    /// cannot see into it cannot use anything it owns.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_observation_shows_what_is_inside_a_carried_container() {
        let (handle, _server) = connect_mock().await;
        let pack_serial = uoterm_protocol::types::Serial(mock::MOCK_BACKPACK).to_string();
        let obs = call_until(&handle, TOOL_OBSERVE, serde_json::json!({}), |v| {
            v.get("containers")
                .and_then(|c| c.as_array())
                .is_some_and(|c| !c.is_empty())
        })
        .await;
        let containers = obs["containers"].as_array().expect("containers array");
        let pack = containers
            .iter()
            .find(|c| c["serial"] == serde_json::json!(pack_serial))
            .unwrap_or_else(|| panic!("the pack is missing from {obs}"));
        assert_eq!(pack["total"], serde_json::json!(1));
        let contents = pack["contents"].as_array().expect("contents array");
        assert_eq!(
            contents[0]["graphic"],
            serde_json::json!(GRAPHIC_HATCHET),
            "the hatchet in the pack must be visible: {obs}"
        );
        assert!(
            contents[0].get("name").is_some(),
            "every item shown must carry a name field: {obs}"
        );
    }

    /// "What is in this container" has to be answerable, and the serial an
    /// observation prints has to be the serial the search accepts.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_search_reaches_inside_a_container() {
        let (handle, _server) = connect_mock().await;
        let pack_serial = uoterm_protocol::types::Serial(mock::MOCK_BACKPACK).to_string();
        let found = call_until(
            &handle,
            TOOL_FIND_ITEMS,
            serde_json::json!({
                "container": pack_serial,
                "name": format!("0x{GRAPHIC_HATCHET:04X}"),
            }),
            |v| v.as_array().is_some_and(|a| !a.is_empty()),
        )
        .await;
        let hits = found.as_array().expect("an array of items");
        assert_eq!(hits.len(), 1, "only the hatchet is in the pack: {found}");
        assert_eq!(hits[0]["serial"], serde_json::json!(mock::MOCK_HATCHET));
        assert_eq!(hits[0]["container"], serde_json::json!(mock::MOCK_BACKPACK));

        let elsewhere = handle
            .call(ToolCall {
                name: TOOL_FIND_ITEMS.into(),
                args: serde_json::json!({
                    "container": mock::MOCK_TREE,
                    "graphic": GRAPHIC_HATCHET,
                }),
            })
            .await;
        assert!(elsewhere.ok, "{elsewhere:?}");
        assert_eq!(
            elsewhere.result.as_array().map(Vec::len),
            Some(0),
            "a search of one container must not answer with another's items"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn chop_wood_adds_logs() {
        let (handle, _server) = connect_mock().await;
        assert!(
            handle.world.read().pending_target.is_none(),
            "login must not leave a target cursor"
        );
        let used = handle
            .call(ToolCall {
                name: TOOL_USE.into(),
                args: serde_json::json!({ "serial": mock::MOCK_HATCHET }),
            })
            .await;
        assert!(used.ok, "{used:?}");
        let mut pending = false;
        for _ in 0..40 {
            if handle.world.read().pending_target.is_some() {
                pending = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(pending, "expected a target cursor after using the hatchet");
        let wait = handle
            .call(ToolCall {
                name: TOOL_WAIT_TARGET.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(wait.ok, "{wait:?}");
        assert_eq!(
            wait.result.get("pending").and_then(|v| v.as_bool()),
            Some(true)
        );
        let targeted = handle
            .call(ToolCall {
                name: TOOL_TARGET.into(),
                args: serde_json::json!({ "serial": mock::MOCK_TREE }),
            })
            .await;
        assert!(targeted.ok, "{targeted:?}");
        let mut saw_logs = false;
        for _ in 0..20 {
            let items = handle
                .call(ToolCall {
                    name: TOOL_FIND_ITEMS.into(),
                    args: serde_json::json!({ "graphic": GRAPHIC_LOGS }),
                })
                .await;
            if items
                .result
                .to_string()
                .contains(&format!("{}", mock::MOCK_LOGS))
                || items
                    .result
                    .to_string()
                    .contains(&format!("{GRAPHIC_LOGS}"))
            {
                saw_logs = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(saw_logs, "expected logs after targeting the tree");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn move_to_steps_on_mock_map() {
        let (handle, _server) = connect_mock().await;
        let start = handle.world.read().self_state.location;
        let dest_x = start.x + 2;
        let dest_y = start.y;
        let moved = handle
            .call(ToolCall {
                name: TOOL_MOVE_TO.into(),
                args: serde_json::json!({ "x": dest_x, "y": dest_y, "z": start.z }),
            })
            .await;
        assert!(moved.ok, "{moved:?}");
        let mut progressed = false;
        for _ in 0..20 {
            let loc = handle.world.read().self_state.location;
            if loc.x != start.x || loc.y != start.y {
                progressed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(progressed, "expected predicted walk after move_to");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn walk_south_steps_on_mock_map() {
        let (handle, _server) = connect_mock().await;
        let start = handle.world.read().self_state.location;
        let walked = handle
            .call(ToolCall {
                name: TOOL_WALK.into(),
                args: serde_json::json!({ "direction": "south", "running": true, "hold_ms": 0 }),
            })
            .await;
        assert!(walked.ok, "{walked:?}");
        let mut progressed = false;
        for _ in 0..20 {
            let loc = handle.world.read().self_state.location;
            if loc.y != start.y {
                progressed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(progressed, "expected predicted 0x02 step after walk");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn observe_lists_facing_and_open_door_ok() {
        let (handle, _server) = connect_mock().await;
        let obs = handle
            .call(ToolCall {
                name: TOOL_OBSERVE.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(obs.ok, "{obs:?}");
        assert!(obs.result.get("facing").and_then(|v| v.as_str()).is_some());
        assert!(obs.result.get("x").is_some());
        assert!(obs.result.get("doors").is_some());

        // The macro that opens a door names no door: the server opens whatever
        // door stands in the tile the character faces. So there has to be one
        // beside him, or there is nothing for the macro to aim at and the tool
        // says so instead of asking for a door that is not there.
        const DOOR_SERIAL: uoterm_protocol::Serial = uoterm_protocol::Serial(0x4002_021B);
        const DOOR_GRAPHIC: u16 = 1653;
        let no_door = handle
            .call(ToolCall {
                name: TOOL_OPEN_DOOR.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(!no_door.ok, "{no_door:?}");

        let beside_him = {
            let world = handle.world.read();
            let at = world.self_state.location;
            uoterm_protocol::Point3::new(at.x, at.y.saturating_sub(1), at.z)
        };
        handle
            .world
            .write()
            .note_door(DOOR_SERIAL, DOOR_GRAPHIC, beside_him);

        let opened = handle
            .call(ToolCall {
                name: TOOL_OPEN_DOOR.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(opened.ok, "{opened:?}");
        assert_eq!(opened.action_id.as_deref(), Some(TOOL_OPEN_DOOR));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_goal_gather() {
        let (handle, _server) = connect_mock().await;
        let g = handle
            .call(ToolCall {
                name: TOOL_SET_GOAL.into(),
                args: serde_json::json!({ "goal": "gather" }),
            })
            .await;
        assert!(g.ok, "{g:?}");
        assert_eq!(handle.world.read().goal, "gather");
        let mut saw_logs = false;
        for _ in 0..50 {
            if handle
                .world
                .read()
                .find_item_graphic(GRAPHIC_LOGS)
                .is_some()
            {
                saw_logs = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(saw_logs, "set_goal gather must chop logs on the mock shard");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn asterisk_emote_is_rejected() {
        let (handle, _server) = connect_mock().await;
        let said = handle
            .call(ToolCall {
                name: TOOL_SAY.into(),
                args: serde_json::json!({ "text": "*smiles*" }),
            })
            .await;
        assert!(!said.ok, "persona must reject asterisk emotes");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn move_to_requires_x_and_y() {
        let (handle, _server) = connect_mock().await;
        let missing = handle
            .call(ToolCall {
                name: TOOL_MOVE_TO.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(!missing.ok, "{missing:?}");
        let only_x = handle
            .call(ToolCall {
                name: TOOL_MOVE_TO.into(),
                args: serde_json::json!({ "x": 10 }),
            })
            .await;
        assert!(!only_x.ok, "{only_x:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unequip_requires_occupied_layer() {
        let (handle, _server) = connect_mock().await;
        let missing = handle
            .call(ToolCall {
                name: TOOL_UNEQUIP.into(),
                args: serde_json::json!({}),
            })
            .await;
        assert!(!missing.ok, "{missing:?}");
        let empty = handle
            .call(ToolCall {
                name: TOOL_UNEQUIP.into(),
                args: serde_json::json!({ "layer": 1 }),
            })
            .await;
        assert!(!empty.ok);
        assert_eq!(empty.error.as_deref(), Some("layer empty"));
    }
}
