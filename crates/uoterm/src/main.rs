mod mcp;
mod remote;

use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use uoterm_protocol::types::{ClientVersion, Era, Point3, EXIT_OK, EXIT_USAGE};
use uoterm_runtime::config::{
    load_app_config, load_persona, load_profile, password_from_env, stay_on_socket_for_era,
    ConnectOptions,
};
use uoterm_runtime::mock;
use uoterm_runtime::persona::Persona;
use uoterm_runtime::tools::{
    TOOL_CANCEL_GOAL, TOOL_MOVE_TO, TOOL_OPEN_DOOR, TOOL_SAY, TOOL_SET_GOAL, TOOL_SET_PERSONA,
    TOOL_WALK,
};
use uoterm_runtime::{Runtime, RuntimeError};

const ENCRYPTION_NONE: &str = "none";
const ENCRYPTION_OSI: &str = "osi";
const ERA_T2A: &str = "t2a";
const ERA_MODERN: &str = "modern";

/// Stream codec for the login and game sockets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum EncryptionMode {
    /// Nocrypt freeshard (the usual private shard default).
    #[default]
    #[value(
        name = "none",
        help = "nocrypt freeshard (the usual private shard default)"
    )]
    None,
    /// Classic Client encryption for official/encrypted shards.
    #[value(
        name = "osi",
        help = "Classic Client encryption for official/encrypted shards"
    )]
    Osi,
}

impl EncryptionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => ENCRYPTION_NONE,
            Self::Osi => ENCRYPTION_OSI,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "uoterm",
    version,
    about = "Protocol-native headless Classic Client and agent runtime",
    long_about = "UOTerm is a protocol-native headless Classic Client. It works with private server shards. Use --encryption none for nocrypt freeshards, the usual private shard default, and --encryption osi for Classic Client encryption. It is not a cheat overlay."
)]
struct Cli {
    /// Print machine JSON on stdout
    #[arg(long, global = true)]
    json: bool,
    /// HTTP API of a running connect/populate process (or UOTERM_API)
    #[arg(long, global = true)]
    api: Option<String>,
    /// Session id (or UOTERM_SESSION)
    #[arg(long, global = true)]
    session: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Log in and keep the HTTP API up
    Connect {
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long, required_unless_present = "profile")]
        account: Option<String>,
        #[arg(long = "password-env", default_value = "UO_PASS")]
        password_env: String,
        #[arg(long)]
        shard: Option<String>,
        #[arg(long, required_unless_present = "profile")]
        character: Option<String>,
        /// Client version string sent as 0xBD (for example 7.0.102.3).
        #[arg(long)]
        version: Option<String>,
        /// Packet era: t2a or modern
        #[arg(long, value_parser = ["t2a", "modern"])]
        era: Option<String>,
        /// none = nocrypt freeshard, the usual private shard default. osi = Classic Client encryption for official/encrypted shards.
        #[arg(long, value_enum, default_value_t = EncryptionMode::None)]
        encryption: EncryptionMode,
        #[arg(long)]
        uopath: Option<PathBuf>,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        persona: Option<PathBuf>,
        #[arg(long)]
        api_bind: Option<String>,
    },
    /// List or inspect sessions on a running process
    #[command(subcommand)]
    Session(SessionCmd),
    /// Speak (remote)
    Say { text: String },
    /// Pathfind to x,y,z (remote)
    Move {
        #[arg(long)]
        to: String,
    },
    /// Send 0x02 walk/run steps (Classic Client hold-right-click analog)
    Walk {
        #[arg(long)]
        dir: String,
        #[arg(long)]
        run: bool,
        #[arg(long = "hold-ms", default_value_t = 0)]
        hold_ms: u64,
    },
    /// Open the door you face (must stand next to it)
    OpenDoor,
    /// Compact look / radar (remote)
    Look,
    /// Full world state (remote)
    State,
    /// Start or stop a high-level goal (remote)
    #[command(subcommand)]
    Agent(AgentCmd),
    /// Harvest event log
    #[command(subcommand)]
    Harvest(HarvestCmd),
    /// Launch many personas and keep the API up
    Populate {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        api_bind: Option<String>,
    },
    /// Local unencrypted demo shard
    #[command(name = "mock-shard")]
    MockShard {
        #[arg(long, default_value = "127.0.0.1:2593")]
        bind: String,
    },
    /// MCP stdio server (proxies to --api)
    Mcp,
}

#[derive(Subcommand, Debug)]
enum SessionCmd {
    List,
    Attach { id: String },
}

#[derive(Subcommand, Debug)]
enum AgentCmd {
    Run {
        #[arg(long)]
        persona: PathBuf,
        #[arg(long)]
        goal: Option<String>,
    },
    Stop,
}

#[derive(Subcommand, Debug)]
enum HarvestCmd {
    Log {
        #[arg(long, default_value = "1h")]
        since: String,
        #[arg(long)]
        jsonl: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            let code = match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    EXIT_OK
                }
                _ => EXIT_USAGE,
            };
            return ExitCode::from(code as u8);
        }
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init();
    match run(cli).await {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

async fn run(cli: Cli) -> Result<u8, RuntimeError> {
    let json = cli.json;
    let api = cli.api.as_deref();
    let session = remote::session_hint(cli.session.as_deref());
    let session = session.as_deref();
    match cli.command {
        Commands::Connect { .. } => connect(cli).await,
        Commands::Session(SessionCmd::List) => {
            let base = remote::api_base(api);
            let ids = remote::list_sessions(&base).await?;
            if json {
                println!("{}", json!({ "sessions": ids }));
            } else if ids.is_empty() {
                println!("(no sessions)");
            } else {
                for id in ids {
                    println!("{id}");
                }
            }
            Ok(EXIT_OK as u8)
        }
        Commands::Session(SessionCmd::Attach { id }) => {
            let base = remote::api_base(api);
            let state = remote::session_state(&base, &id).await?;
            emit_state(json, &state)?;
            Ok(EXIT_OK as u8)
        }
        Commands::Say { text } => {
            remote_tool(api, session, json, TOOL_SAY, json!({ "text": text })).await
        }
        Commands::Move { to } => {
            let dest: Point3 = to
                .parse()
                .map_err(|e: uoterm_protocol::ProtocolError| RuntimeError::Usage(e.to_string()))?;
            remote_tool(
                api,
                session,
                json,
                TOOL_MOVE_TO,
                json!({ "x": dest.x, "y": dest.y, "z": dest.z, "to": to }),
            )
            .await
        }
        Commands::Walk { dir, run, hold_ms } => {
            remote_tool(
                api,
                session,
                json,
                TOOL_WALK,
                json!({ "direction": dir, "running": run, "hold_ms": hold_ms }),
            )
            .await
        }
        Commands::OpenDoor => remote_tool(api, session, json, TOOL_OPEN_DOOR, json!({})).await,
        Commands::Look => {
            let base = remote::api_base(api);
            let id = remote::resolve_session(&base, session).await?;
            let state = remote::session_state(&base, &id).await?;
            if json {
                println!("{state}");
            } else if let Some(radar) = state.get("radar").and_then(|r| r.as_str()) {
                print!("{radar}");
            } else {
                emit_state(false, &state)?;
            }
            Ok(EXIT_OK as u8)
        }
        Commands::State => {
            let base = remote::api_base(api);
            let id = remote::resolve_session(&base, session).await?;
            let state = remote::session_state(&base, &id).await?;
            emit_state(json, &state)?;
            Ok(EXIT_OK as u8)
        }
        Commands::Agent(AgentCmd::Run { persona, goal }) => {
            let p = load_persona(&persona)?;
            let g = goal.unwrap_or_else(|| remote::goal_for_class(&p.class).to_string());
            let base = remote::api_base(api);
            let id = remote::resolve_session(&base, session).await?;
            let persona_json =
                serde_json::to_value(&p).map_err(|e| RuntimeError::Protocol(e.to_string()))?;
            remote::call_tool(&base, &id, TOOL_SET_PERSONA, persona_json).await?;
            let v = remote::call_tool(&base, &id, TOOL_SET_GOAL, json!({ "goal": g })).await?;
            emit(json, v);
            Ok(EXIT_OK as u8)
        }
        Commands::Agent(AgentCmd::Stop) => {
            remote_tool(api, session, json, TOOL_CANCEL_GOAL, json!({})).await
        }
        Commands::Harvest(HarvestCmd::Log { since, jsonl }) => {
            let rows = remote::read_harvest(&since)?;
            if rows.is_empty() && !json && !jsonl {
                println!("(no harvest lines)");
            } else {
                for row in rows {
                    println!("{row}");
                }
            }
            Ok(EXIT_OK as u8)
        }
        Commands::Populate { manifest, api_bind } => {
            let cfg = load_app_config(None);
            let rt = Runtime::new(cfg.max_sessions);
            let ids = uoterm_runtime::populate::run(&rt, &manifest).await?;
            let bind = api_bind.unwrap_or(cfg.api_bind);
            if json {
                println!("{}", json!({ "sessions": ids, "api": bind }));
            } else {
                println!("started {} sessions; api {bind}", ids.len());
            }
            serve_until_ctrl_c(rt, bind).await
        }
        Commands::MockShard { bind } => {
            let addr: SocketAddr = bind
                .parse()
                .map_err(|e| RuntimeError::Usage(format!("{e}")))?;
            let local = mock::serve(addr)
                .await
                .map_err(|e| RuntimeError::Network(e.to_string()))?;
            if json {
                println!("{}", json!({ "bind": local.to_string() }));
            } else {
                eprintln!("mock shard listening on {local}");
            }
            tokio::signal::ctrl_c()
                .await
                .map_err(|e| RuntimeError::Network(e.to_string()))?;
            Ok(EXIT_OK as u8)
        }
        Commands::Mcp => mcp::run_stdio(api).await,
    }
}

async fn connect(cli: Cli) -> Result<u8, RuntimeError> {
    let json = cli.json;
    let Commands::Connect {
        host,
        port,
        account,
        password_env,
        shard,
        character,
        version,
        era,
        encryption,
        uopath,
        profile,
        persona,
        api_bind,
        ..
    } = cli.command
    else {
        return Err(RuntimeError::Usage("connect".into()));
    };
    let cfg = load_app_config(None);
    let (account, character, password_env, shard, profile_era, profile_version) =
        if let Some(p) = profile {
            let pr = load_profile(&p)?;
            (
                pr.account,
                pr.character,
                pr.password_env,
                pr.shard.or(shard),
                pr.era,
                pr.version,
            )
        } else {
            (
                account.ok_or_else(|| RuntimeError::Usage("account is required".into()))?,
                character.ok_or_else(|| RuntimeError::Usage("character is required".into()))?,
                password_env,
                shard,
                None,
                None,
            )
        };
    let password = password_from_env(&password_env)?;
    let era_raw = era.or(profile_era).unwrap_or_else(|| match cfg.era {
        Era::T2a => ERA_T2A.into(),
        Era::Modern => ERA_MODERN.into(),
    });
    let era: Era = era_raw.parse().unwrap_or(cfg.era);
    let version_raw = version.or(profile_version);
    let version: ClientVersion = version_raw
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| era.default_version());
    let persona = match persona {
        Some(p) => load_persona(&p)?,
        None => Persona::lumberjack_yew(),
    };
    let host = host.unwrap_or(cfg.host);
    let port = port.unwrap_or(cfg.port);
    let uopath = uopath.or(cfg.uopath);
    tracing::info!(
        encryption = encryption.as_str(),
        era = ?era,
        version = %version,
        "connect"
    );
    let opts = ConnectOptions {
        host,
        port,
        account,
        password,
        shard,
        character,
        version,
        era,
        uopath,
        persona: Some(persona),
        stay_on_socket: stay_on_socket_for_era(era, cfg.stay_on_socket),
        next_login_key: uoterm_protocol::types::LOGIN_NEXT_KEY_DEFAULT,
        encryption: match encryption {
            EncryptionMode::None => uoterm_runtime::EncryptionMode::None,
            EncryptionMode::Osi => uoterm_runtime::EncryptionMode::Osi,
        },
        obey_shard_rules: cfg.obey_shard_rules,
    };
    let rt = Runtime::new(cfg.max_sessions);
    let handle = rt.connect(opts).await?;
    let bind = api_bind.unwrap_or(cfg.api_bind);
    if json {
        println!(
            "{}",
            json!({
                "id": handle.id,
                "logged_in": handle.world.read().logged_in,
                "api": bind,
                "encryption": encryption.as_str(),
                "era": match era {
                    Era::T2a => ERA_T2A,
                    Era::Modern => ERA_MODERN,
                },
                "version": version.to_string(),
            })
        );
    } else {
        println!(
            "session {} started; api {bind}; encryption {}",
            handle.id,
            encryption.as_str()
        );
    }
    serve_until_ctrl_c(rt, bind).await
}

async fn remote_tool(
    api: Option<&str>,
    session: Option<&str>,
    as_json: bool,
    name: &str,
    args: Value,
) -> Result<u8, RuntimeError> {
    let base = remote::api_base(api);
    let id = remote::resolve_session(&base, session).await?;
    let v = remote::call_tool(&base, &id, name, args).await?;
    emit(as_json, v);
    Ok(EXIT_OK as u8)
}

async fn serve_until_ctrl_c(rt: Runtime, bind: String) -> Result<u8, RuntimeError> {
    let rt2 = rt.clone();
    let bind2 = bind.clone();
    tokio::spawn(async move {
        if let Err(e) = uoterm_runtime::api::serve(&bind2, rt2).await {
            tracing::error!(error = %e, "api ended");
        }
    });
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    Ok(EXIT_OK as u8)
}

fn emit(as_json: bool, value: Value) {
    if as_json {
        println!("{value}");
    } else if let Some(s) = value.as_str() {
        println!("{s}");
    } else {
        println!("{value}");
    }
}

fn emit_state(as_json: bool, state: &Value) -> Result<(), RuntimeError> {
    if as_json {
        println!("{state}");
    } else {
        let yaml =
            serde_yaml::to_string(state).map_err(|e| RuntimeError::Protocol(e.to_string()))?;
        print!("{yaml}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    const VERSION_MODERN_EXAMPLE: &str = "7.0.102.3";

    #[test]
    fn parses_say_json() {
        let cli = Cli::try_parse_from(["uoterm", "--json", "say", "vendor buy"]).unwrap();
        assert!(cli.json);
        match cli.command {
            Commands::Say { text } => assert_eq!(text, "vendor buy"),
            _ => panic!("expected say"),
        }
    }

    #[test]
    fn parses_harvest_log() {
        let cli =
            Cli::try_parse_from(["uoterm", "harvest", "log", "--since", "2h", "--jsonl"]).unwrap();
        match cli.command {
            Commands::Harvest(HarvestCmd::Log { since, jsonl }) => {
                assert_eq!(since, "2h");
                assert!(jsonl);
            }
            _ => panic!("expected harvest log"),
        }
    }

    #[test]
    fn parses_move_to() {
        let cli = Cli::try_parse_from(["uoterm", "move", "--to", "1425,1680,0"]).unwrap();
        match cli.command {
            Commands::Move { to } => assert_eq!(to, "1425,1680,0"),
            _ => panic!("expected move"),
        }
    }

    #[test]
    fn parses_walk_hold_run() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "walk",
            "--dir",
            "south",
            "--run",
            "--hold-ms",
            "2000",
        ])
        .unwrap();
        match cli.command {
            Commands::Walk { dir, run, hold_ms } => {
                assert_eq!(dir, "south");
                assert!(run);
                assert_eq!(hold_ms, 2000);
            }
            _ => panic!("expected walk"),
        }
    }

    #[test]
    fn parses_open_door() {
        let cli = Cli::try_parse_from(["uoterm", "open-door"]).unwrap();
        match cli.command {
            Commands::OpenDoor => {}
            _ => panic!("expected open-door"),
        }
    }

    #[test]
    fn parses_connect_encryption_default_none() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
        ])
        .unwrap();
        match cli.command {
            Commands::Connect {
                encryption,
                era,
                version,
                ..
            } => {
                assert_eq!(encryption, EncryptionMode::None);
                assert_eq!(encryption.as_str(), ENCRYPTION_NONE);
                assert!(era.is_none());
                assert!(version.is_none());
            }
            _ => panic!("expected connect"),
        }
    }

    #[test]
    fn parses_connect_encryption_osi_era_modern() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--encryption",
            ENCRYPTION_OSI,
            "--era",
            ERA_MODERN,
            "--version",
            VERSION_MODERN_EXAMPLE,
        ])
        .unwrap();
        match cli.command {
            Commands::Connect {
                encryption,
                era,
                version,
                ..
            } => {
                assert_eq!(encryption, EncryptionMode::Osi);
                assert_eq!(encryption.as_str(), ENCRYPTION_OSI);
                assert_eq!(era.as_deref(), Some(ERA_MODERN));
                assert_eq!(version.as_deref(), Some(VERSION_MODERN_EXAMPLE));
            }
            _ => panic!("expected connect"),
        }
    }

    #[test]
    fn parses_connect_era_t2a() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--era",
            ERA_T2A,
            "--encryption",
            ENCRYPTION_NONE,
        ])
        .unwrap();
        match cli.command {
            Commands::Connect {
                encryption, era, ..
            } => {
                assert_eq!(encryption, EncryptionMode::None);
                assert_eq!(era.as_deref(), Some(ERA_T2A));
            }
            _ => panic!("expected connect"),
        }
    }

    #[test]
    fn rejects_unknown_encryption() {
        const ENCRYPTION_INVALID: &str = "blowfish";
        let err = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--encryption",
            ENCRYPTION_INVALID,
        ])
        .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn rejects_unknown_era() {
        const ERA_INVALID: &str = "aos";
        let err = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--era",
            ERA_INVALID,
        ])
        .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }
}
