//! The macro recorder: what the character does, written down as a script.
//!
//! While it records, each tool call that acts and each hotkey pressed turns
//! into a script line. When the shard opens a target cursor, a gump or a
//! prompt, a wait for it goes in by itself, so the script waits where the
//! player waited. Stopping saves the lines as a script file.

use std::path::PathBuf;

use super::*;

/// The extension recorded scripts are saved with.
const SCRIPT_EXT: &str = "txt";
/// How long a recorded wait waits.
const RECORDED_WAIT_MS: u32 = 5000;

/// A recording in progress.
pub(super) struct Recording {
    name: String,
    lines: Vec<String>,
}

const ARG_ACTION: &str = "action";

/// `record_macro`: start, stop (and save), or cancel a recording.
pub(super) fn record_macro(inner: &mut Inner, args: &Value) -> ToolResult {
    let action = args.get(ARG_ACTION).and_then(|v| v.as_str()).unwrap_or("");
    match action {
        "start" => {
            let Some(name) = args
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|n| !n.is_empty() && is_file_name(n))
            else {
                return ToolResult::err(
                    "record_macro start needs a name of letters, digits, - and _",
                );
            };
            inner.recording = Some(Recording {
                name: name.into(),
                lines: Vec::new(),
            });
            ToolResult::ok(json!({ "recording": name }))
        }
        "stop" => {
            let Some(rec) = inner.recording.take() else {
                return ToolResult::err("nothing is being recorded");
            };
            let text = rec.lines.join("\n") + "\n";
            match save(&rec.name, &text) {
                Ok(path) => ToolResult::ok(json!({
                    "script": rec.name,
                    "file": path,
                    "lines": rec.lines,
                })),
                Err(e) => ToolResult::err(e),
            }
        }
        "cancel" => {
            let had = inner.recording.take().is_some();
            ToolResult::ok(json!({ "cancelled": had }))
        }
        other => ToolResult::err(format!("'{other}' is not start, stop or cancel")),
    }
}

/// A script name that is safe as a file name.
fn is_file_name(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Saves a script: in the working directory's scripts folder when there is
/// one, and in the user's config folder when not.
fn save(name: &str, text: &str) -> std::result::Result<PathBuf, String> {
    let local = PathBuf::from(scripting::SCRIPTS_DIR);
    let dir = if local.is_dir() {
        local
    } else {
        crate::config::config_dir().join(scripting::SCRIPTS_DIR)
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.{SCRIPT_EXT}"));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

fn push(inner: &mut Inner, line: String) {
    if let Some(rec) = inner.recording.as_mut() {
        rec.lines.push(line);
    }
}

/// A serial as a script writes it.
fn hex(serial: Serial) -> String {
    format!("0x{:08X}", serial.0)
}

/// Text in the quotes a script needs.
fn quoted(text: &str) -> String {
    if text.contains('\'') {
        format!("\"{text}\"")
    } else {
        format!("'{text}'")
    }
}

/// Writes down a tool call that went through, as a script line.
pub(super) fn tool_call(inner: &mut Inner, call: &ToolCall, result: &ToolResult) {
    if inner.recording.is_none() || !result.ok {
        return;
    }
    let args = &call.args;
    let text = || {
        args.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let serial = || arg_serial(args, "serial");
    let line = match call.name.as_str() {
        TOOL_SAY => format!("msg {}", quoted(&text())),
        TOOL_WHISPER => format!("whispermsg {}", quoted(&text())),
        TOOL_EMOTE => format!("emotemsg {}", quoted(&text())),
        TOOL_USE | TOOL_OPEN_CONTAINER => match args.get("who").and_then(|v| v.as_str()) {
            Some(WHO_LAST) => "useobject 'lastobject'".into(),
            _ => format!("useobject {}", hex(serial())),
        },
        TOOL_SINGLE_CLICK => format!("clickobject {}", hex(serial())),
        TOOL_ATTACK => format!("attack {}", hex(serial())),
        TOOL_WAR_MODE => {
            let on = args.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
            format!("warmode '{}'", if on { "on" } else { "off" })
        }
        TOOL_DROP => match drop_destination(args) {
            Some(dest) => format!("moveitem {} {}", hex(serial()), hex(dest)),
            None => format!("moveitemoffset {} 'ground' 0 0 0", hex(serial())),
        },
        TOOL_EQUIP => match args.get("who").and_then(|v| v.as_str()) {
            Some(WHO_LAST) => "togglehands 'right'".into(),
            _ => {
                let layer = args
                    .get("layer")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(u64::from(LAYER_ONE_HANDED));
                format!("equipitem {} {layer}", hex(serial()))
            }
        },
        TOOL_UNEQUIP => match args.get("layer").and_then(|v| v.as_u64()) {
            Some(l) if l == u64::from(LAYER_ONE_HANDED) => "clearhands 'right'".into(),
            Some(l) if l == u64::from(LAYER_TWO_HANDED) => "clearhands 'left'".into(),
            Some(l) => format!("// took off layer {l}; wear it again with equipitem"),
            None => return,
        },
        TOOL_CAST => {
            let Some(id) = arg_number(args, ARG_SPELL) else {
                return;
            };
            match inner.scripting.spells.by_id(id as u16) {
                Some(spell) => format!("cast {}", quoted(&spell.name)),
                None => format!("cast {id}"),
            }
        }
        TOOL_USE_SKILL => {
            let Some(id) = arg_number(args, ARG_SKILL) else {
                return;
            };
            match inner.scripting.skills.by_id(id as u16) {
                Some(skill) => format!("useskill {}", quoted(&skill.name)),
                None => format!("useskill {id}"),
            }
        }
        TOOL_TARGET => {
            if args.get("serial").is_some() {
                format!("target {}", hex(serial()))
            } else if let Some(who) = args.get("who").and_then(|v| v.as_str()) {
                format!("target {}", quoted(who))
            } else if let (Some(x), Some(y)) = (
                args.get("x").and_then(|v| v.as_u64()),
                args.get("y").and_then(|v| v.as_u64()),
            ) {
                match args.get("z").and_then(|v| v.as_i64()) {
                    Some(z) => format!("targettile {x} {y} {z}"),
                    None => format!("targettile {x} {y}"),
                }
            } else {
                "canceltarget".into()
            }
        }
        TOOL_MOVE_TO => {
            let x = args.get("x").and_then(|v| v.as_u64()).unwrap_or(0);
            let y = args.get("y").and_then(|v| v.as_u64()).unwrap_or(0);
            format!("pathfindto {x} {y}")
        }
        TOOL_WALK => {
            let dir = args
                .get("direction")
                .or_else(|| args.get("dir"))
                .and_then(|v| v.as_str())
                .and_then(Direction::from_name)
                .map(|d| d.name().to_string())
                .unwrap_or_default();
            let running = args
                .get("running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            format!("{} {}", if running { "run" } else { "walk" }, quoted(&dir))
        }
        TOOL_OPEN_DOOR => "opendoor".into(),
        TOOL_CONTEXT_MENU => {
            let cliloc = arg_u32(args, "cliloc", 0);
            let words = inner
                .cliloc
                .as_ref()
                .and_then(|t| t.text(cliloc))
                .map(str::to_string)
                .unwrap_or_else(|| cliloc.to_string());
            format!("contextmenu {} {}", hex(serial()), quoted(&words))
        }
        TOOL_GUMP_RESPOND | TOOL_GUMP_CLOSE => {
            let Some(gump) = result.result.get("gump").and_then(|v| v.as_u64()) else {
                return;
            };
            let button = if call.name == TOOL_GUMP_CLOSE {
                0
            } else {
                args.get("button").and_then(|v| v.as_u64()).unwrap_or(0)
            };
            let switches: Vec<String> = args
                .get("switches")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_u64())
                        .map(|n| n.to_string())
                        .collect()
                })
                .unwrap_or_default();
            format!("replygump 0x{gump:08X} {button} {}", switches.join(" "))
                .trim_end()
                .to_string()
        }
        TOOL_HOTKEY => match result.result.get("lines").and_then(|v| v.as_str()) {
            Some(lines) => lines.to_string(),
            None => return,
        },
        _ => return,
    };
    push(inner, line);
}

/// Writes down the waits for what the shard opens while it records.
pub(super) fn shard_opened(inner: &mut Inner, msg: &Inbound) {
    if inner.recording.is_none() {
        return;
    }
    let line = match msg {
        Inbound::Target(_) => format!("waitfortarget {RECORDED_WAIT_MS}"),
        Inbound::Gump(g) => format!("waitforgump 0x{:08X} {RECORDED_WAIT_MS}", g.gump_id),
        Inbound::Prompt(_) => format!("waitforprompt {RECORDED_WAIT_MS}"),
        _ => return,
    };
    push(inner, line);
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::{armed_session, ready_to_act, target_cursor, A_CURSOR};
    use super::*;

    fn call(name: &str, args: Value) -> ToolCall {
        ToolCall {
            name: name.into(),
            args,
        }
    }

    fn recording(inner: &Inner) -> Vec<String> {
        inner
            .recording
            .as_ref()
            .map(|r| r.lines.clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_recording_writes_a_cast_the_cursor_wait_and_the_target() {
        const GREATER_HEAL: u64 = 29;
        let mut inner = armed_session();
        inner.world.write().logged_in = true;
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "heal" })).ok);
        let cast = call(TOOL_CAST, json!({ "spell": GREATER_HEAL }));
        let result = handle_tool(&mut inner, cast.clone());
        tool_call(&mut inner, &cast, &result);
        ingest(&mut inner, &target_cursor(A_CURSOR));
        let aim = call(TOOL_TARGET, json!({ "who": "self" }));
        let result = handle_tool(&mut inner, aim.clone());
        tool_call(&mut inner, &aim, &result);
        assert_eq!(
            recording(&inner),
            vec!["cast 'Greater Heal'", "waitfortarget 5000", "target 'self'"]
        );
    }

    #[test]
    fn a_recorded_line_reads_as_a_script() {
        let mut inner = armed_session();
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "walk" })).ok);
        for (name, args) in [
            (TOOL_SAY, json!({ "text": "japan's melon" })),
            (TOOL_WALK, json!({ "direction": "n", "running": true })),
            (TOOL_MOVE_TO, json!({ "x": 10, "y": 20 })),
            (TOOL_USE, json!({ "serial": "0x40001234" })),
        ] {
            let c = call(name, args);
            let ok = ToolResult::ok(Value::Null);
            tool_call(&mut inner, &c, &ok);
        }
        let text = recording(&inner).join("\n");
        assert!(uoterm_script::Program::parse(&text).is_ok(), "{text}");
        assert!(text.contains("run 'north'"));
        assert!(text.contains("useobject 0x40001234"));
    }

    #[test]
    fn a_failed_call_is_not_recorded() {
        let mut inner = armed_session();
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "x" })).ok);
        let c = call(TOOL_CAST, json!({}));
        let result = handle_tool(&mut inner, c.clone());
        tool_call(&mut inner, &c, &result);
        assert!(recording(&inner).is_empty());
    }

    #[test]
    fn a_name_that_is_not_a_plain_file_name_is_refused() {
        let mut inner = armed_session();
        assert!(!record_macro(&mut inner, &json!({ "action": "start", "name": "../evil" })).ok);
        ready_to_act(&mut inner);
    }
}
