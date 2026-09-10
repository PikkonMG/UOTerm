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

/// Text a script line cannot hold: a script has no escapes, so a quote
/// cannot hold both quote marks, and a line cannot hold a line break.
struct Unquotable;

/// Text in the quotes a script needs.
fn quoted(text: &str) -> std::result::Result<String, Unquotable> {
    if text.contains(['\n', '\r']) {
        return Err(Unquotable);
    }
    match (text.contains('\''), text.contains('"')) {
        (true, true) => Err(Unquotable),
        (true, false) => Ok(format!("\"{text}\"")),
        (false, _) => Ok(format!("'{text}'")),
    }
}

/// A note line in a script.
fn note(text: &str) -> String {
    format!("{} {text}", uoterm_script::COMMENT)
}

/// Writes down a tool call that went through, as a script line. A call whose
/// text no script line can hold is written as a note that says so.
pub(super) fn tool_call(inner: &mut Inner, call: &ToolCall, result: &ToolResult) {
    if inner.recording.is_none() || !result.ok {
        return;
    }
    let line = match script_line(inner, call, result) {
        Ok(Some(line)) => line,
        Ok(None) => return,
        Err(Unquotable) => note(&format!(
            "not recorded: the {} text holds both quote marks or a line break",
            call.name
        )),
    };
    push(inner, line);
}

/// The script line for a tool call, or none when the call has no line.
fn script_line(
    inner: &Inner,
    call: &ToolCall,
    result: &ToolResult,
) -> std::result::Result<Option<String>, Unquotable> {
    let args = &call.args;
    let text = || {
        args.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let serial = || arg_serial(args, "serial");
    let line = match call.name.as_str() {
        TOOL_SAY => format!("msg {}", quoted(&text())?),
        TOOL_WHISPER => format!("whispermsg {}", quoted(&text())?),
        TOOL_EMOTE => format!("emotemsg {}", quoted(&text())?),
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
        TOOL_EQUIP => {
            // The last weapon is named by its serial: the tool wore that
            // item, and it is the one the script must wear again.
            let item = match args.get("who").and_then(|v| v.as_str()) {
                Some(WHO_LAST) => match inner.last_weapon {
                    Some(weapon) => weapon,
                    None => return Ok(None),
                },
                _ => serial(),
            };
            let layer = args
                .get("layer")
                .and_then(|v| v.as_u64())
                .unwrap_or(u64::from(LAYER_ONE_HANDED));
            format!("equipitem {} {layer}", hex(item))
        }
        TOOL_UNEQUIP => match args.get("layer").and_then(|v| v.as_u64()) {
            Some(l) if l == u64::from(LAYER_ONE_HANDED) => "clearhands 'right'".into(),
            Some(l) if l == u64::from(LAYER_TWO_HANDED) => "clearhands 'left'".into(),
            Some(l) => note(&format!("took off layer {l}; wear it again with equipitem")),
            None => return Ok(None),
        },
        TOOL_CAST => {
            let Some(id) = arg_number(args, ARG_SPELL) else {
                return Ok(None);
            };
            match inner.scripting.spells.by_id(id as u16) {
                Some(spell) => format!("cast {}", quoted(&spell.name)?),
                None => format!("cast {id}"),
            }
        }
        TOOL_USE_SKILL => {
            let Some(id) = arg_number(args, ARG_SKILL) else {
                return Ok(None);
            };
            match inner.scripting.skills.by_id(id as u16) {
                Some(skill) => format!("useskill {}", quoted(&skill.name)?),
                None => format!("useskill {id}"),
            }
        }
        TOOL_TARGET => {
            if args.get("serial").is_some() {
                format!("target {}", hex(serial()))
            } else if let Some(who) = args.get("who").and_then(|v| v.as_str()) {
                format!("target {}", quoted(who)?)
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
                .map(|d| d.name())
                .unwrap_or_default();
            // The walk's answer says how it went: a held walk is its steps,
            // each written out, since a script walk takes a list of steps.
            let Some(steps) = result.result.get("steps").and_then(|v| v.as_u64()) else {
                return Ok(None);
            };
            let running = result
                .result
                .get("running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let path = vec![dir; steps as usize].join(", ");
            format!("{} {}", if running { "run" } else { "walk" }, quoted(&path)?)
        }
        TOOL_OPEN_DOOR => "opendoor".into(),
        TOOL_CONTEXT_MENU => {
            // Without the client text files the entry is named by its bare
            // number, which a script reads as that client text number.
            let cliloc = arg_u32(args, "cliloc", 0);
            let entry = match inner.cliloc.as_ref().and_then(|t| t.text(cliloc)) {
                Some(words) => quoted(words)?,
                None => cliloc.to_string(),
            };
            format!("contextmenu {} {entry}", hex(serial()))
        }
        TOOL_GUMP_RESPOND | TOOL_GUMP_CLOSE => {
            let Some(gump) = result.result.get("gump").and_then(|v| v.as_u64()) else {
                return Ok(None);
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
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(Some(line))
}

/// Writes down the waits for what the shard opens while it records. A target
/// cursor a queued target has already answered gets no wait: the script
/// queues the same target, and the cursor is answered as it comes.
pub(super) fn shard_opened(inner: &mut Inner, msg: &Inbound) {
    if inner.recording.is_none() {
        return;
    }
    let line = match msg {
        Inbound::Target(_) if inner.world.read().pending_target.is_none() => return,
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
        for (name, args, answer) in [
            (TOOL_SAY, json!({ "text": "japan's melon" }), Value::Null),
            (
                TOOL_WALK,
                json!({ "direction": "n", "running": true }),
                json!({ "running": true, "steps": 1 }),
            ),
            (TOOL_MOVE_TO, json!({ "x": 10, "y": 20 }), Value::Null),
            (TOOL_USE, json!({ "serial": "0x40001234" }), Value::Null),
        ] {
            let c = call(name, args);
            tool_call(&mut inner, &c, &ToolResult::ok(answer));
        }
        let text = recording(&inner).join("\n");
        assert!(uoterm_script::Program::parse(&text).is_ok(), "{text}");
        assert!(text.contains("run 'north'"));
        assert!(text.contains("useobject 0x40001234"));
    }

    /// The text of the first argument of the one command `line` holds.
    fn first_text(line: &str) -> Option<String> {
        let program = uoterm_script::Program::parse(line).ok()?;
        match program.ops.first()? {
            uoterm_script::Op::Command(c) => c.args.first().map(|a| a.text.clone()),
            _ => None,
        }
    }

    #[test]
    fn recorded_speech_reads_back_as_the_same_text() {
        for text in ["japan's melon", "say \"hi\"", "plain words"] {
            let mut inner = armed_session();
            assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "say" })).ok);
            tool_call(
                &mut inner,
                &call(TOOL_SAY, json!({ "text": text })),
                &ToolResult::ok(Value::Null),
            );
            let line = recording(&inner).join("\n");
            assert_eq!(first_text(&line).as_deref(), Some(text), "{line}");
        }
    }

    #[test]
    fn speech_no_quote_can_hold_is_a_note_and_not_a_broken_line() {
        for text in ["it's \"fine\"", "two\nlines"] {
            let mut inner = armed_session();
            assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "say" })).ok);
            tool_call(
                &mut inner,
                &call(TOOL_SAY, json!({ "text": text })),
                &ToolResult::ok(Value::Null),
            );
            let lines = recording(&inner);
            let saved = lines.join("\n");
            assert_eq!(lines.len(), 1, "{saved}");
            assert!(saved.starts_with(uoterm_script::COMMENT), "{saved}");
            assert!(saved.contains(TOOL_SAY), "{saved}");
            assert!(uoterm_script::Program::parse(&saved).is_ok(), "{saved}");
        }
    }

    #[test]
    fn wearing_the_last_weapon_is_recorded_as_wearing_that_weapon() {
        const SWORD: Serial = Serial(0x4000_0B02);
        let mut inner = armed_session();
        inner.last_weapon = Some(SWORD);
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "arm" })).ok);
        tool_call(
            &mut inner,
            &call(TOOL_EQUIP, json!({ "who": WHO_LAST })),
            &ToolResult::ok(Value::Null),
        );
        assert_eq!(
            recording(&inner),
            vec![format!("equipitem 0x40000B02 {LAYER_ONE_HANDED}")]
        );
    }

    #[test]
    fn a_held_run_is_recorded_as_every_step_it_ran() {
        const STEPS: u64 = 3;
        let mut inner = armed_session();
        {
            let mut world = inner.world.write();
            world.logged_in = true;
            world.self_state.location = Point3::new(100, 100, 0);
        }
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "run" })).ok);
        let held = call(
            TOOL_WALK,
            json!({ "direction": "n", "run": true, "hold_ms": STEP_RUN_MS * STEPS }),
        );
        let result = handle_tool(&mut inner, held.clone());
        assert!(result.ok, "{:?}", result.error);
        tool_call(&mut inner, &held, &result);
        assert_eq!(recording(&inner), vec!["run 'north, north, north'"]);
    }

    #[test]
    fn a_menu_entry_with_no_words_is_recorded_as_its_bare_number() {
        let mut inner = armed_session();
        assert!(inner.cliloc.is_none(), "no client text files");
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "bank" })).ok);
        tool_call(
            &mut inner,
            &call(
                TOOL_CONTEXT_MENU,
                json!({ "serial": "0x40000D01", "cliloc": 3_000_489 }),
            ),
            &ToolResult::ok(Value::Null),
        );
        assert_eq!(recording(&inner), vec!["contextmenu 0x40000D01 3000489"]);
    }

    #[test]
    fn a_cursor_a_queued_target_answers_adds_no_wait() {
        let mut inner = armed_session();
        inner.world.write().logged_in = true;
        assert!(record_macro(&mut inner, &json!({ "action": "start", "name": "aim" })).ok);
        let aim = call(TOOL_TARGET, json!({ "serial": "0x40000D01" }));
        let result = handle_tool(&mut inner, aim.clone());
        tool_call(&mut inner, &aim, &result);
        ingest(&mut inner, &target_cursor(A_CURSOR));
        assert!(inner.world.read().pending_target.is_none(), "answered");
        assert_eq!(recording(&inner), vec!["target 0x40000D01"]);
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
