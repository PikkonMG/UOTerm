mod art;
mod mcp;
mod remote;
mod view;
mod window;

use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use uoterm_protocol::types::{ClientVersion, Era, Point3, EXIT_OK, EXIT_USAGE};
use uoterm_runtime::config::{
    load_app_config, load_persona, load_profile, password_from_env, ConnectOptions, ScreenLogin,
    DEFAULT_PASSWORD_ENV,
};
use uoterm_runtime::mock;
use uoterm_runtime::persona::Persona;
use uoterm_runtime::tools::{
    TOOL_CANCEL_GOAL, TOOL_MOVE_TO, TOOL_OPEN_DOOR, TOOL_SAY, TOOL_SET_GOAL, TOOL_SET_PERSONA,
    TOOL_WALK,
};
use uoterm_runtime::{
    AppConfig, LoginStore, LoginTarget, Profile, Runtime, RuntimeError, StoredLogin,
};

const ENCRYPTION_NONE: &str = "none";
const ENCRYPTION_OSI: &str = "osi";
const ERA_T2A: &str = "t2a";
const ERA_MODERN: &str = "modern";
const TERM_CLEAR_HOME: &str = "\x1b[2J\x1b[H";

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

impl From<EncryptionMode> for uoterm_runtime::EncryptionMode {
    fn from(mode: EncryptionMode) -> Self {
        match mode {
            EncryptionMode::None => Self::None,
            EncryptionMode::Osi => Self::Osi,
        }
    }
}

impl From<uoterm_runtime::EncryptionMode> for EncryptionMode {
    fn from(mode: uoterm_runtime::EncryptionMode) -> Self {
        match mode {
            uoterm_runtime::EncryptionMode::None => Self::None,
            uoterm_runtime::EncryptionMode::Osi => Self::Osi,
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
        /// The variable that holds the password. Default: the one of --profile, else UO_PASS.
        #[arg(long = "password-env")]
        password_env: Option<String>,
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
        /// none = nocrypt freeshard, the usual private shard default. osi = Classic Client encryption for official/encrypted shards. Default: the one of the saved login, else none.
        #[arg(long, value_enum)]
        encryption: Option<EncryptionMode>,
        #[arg(long)]
        uopath: Option<PathBuf>,
        /// A marker file of named places to travel to (UO Auto Map .map or Ultima Mapper Waypoints.lua).
        #[arg(long)]
        markers: Option<PathBuf>,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        persona: Option<PathBuf>,
        #[arg(long)]
        api_bind: Option<String>,
        /// Open the watch window on this session. `view = true` in uoterm.toml does the same.
        #[arg(long, conflicts_with = "text_view")]
        view: bool,
        /// Print a live radar in this terminal while the API runs.
        #[arg(long = "text-view", conflicts_with = "view")]
        text_view: bool,
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
        #[arg(long, default_value = uoterm_runtime::config::DEFAULT_MOCK_BIND)]
        bind: String,
    },
    /// MCP stdio server (proxies to --api)
    Mcp,
    /// Play by hand: login screens, then the game window with control taken
    Play {
        /// A saved login that fills the form at the start: its name, or its file
        #[arg(long)]
        profile: Option<PathBuf>,
        /// none = nocrypt freeshard, the usual private shard default. osi = Classic Client encryption for official/encrypted shards. Default: the one of the saved login, else none.
        #[arg(long, value_enum)]
        encryption: Option<EncryptionMode>,
        /// The client files for the real map. The default is `uopath` in uoterm.toml.
        #[arg(long)]
        uopath: Option<PathBuf>,
        #[arg(long)]
        api_bind: Option<String>,
        /// Log in at once with the saved login of --profile, with no click on Connect
        #[arg(long, requires = "profile")]
        go: bool,
    },
    /// Watch a running session: a 2D window, or --text for the terminal
    Watch {
        /// Print the radar in the terminal instead of opening a window
        #[arg(long)]
        text: bool,
        /// The client files for the real map. The default is `uopath` in uoterm.toml.
        #[arg(long)]
        uopath: Option<PathBuf>,
        /// Save one picture of the window to this PNG file, then close
        #[arg(long, conflicts_with = "text")]
        snapshot: Option<PathBuf>,
        /// Panels that are open when the window starts. Give it once for each panel
        #[arg(long, value_enum, conflicts_with = "text")]
        open: Vec<window::Panel>,
    },
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
    // The program keeps its files in the person's folders. A test never
    // runs this, so a test never changes them.
    uoterm_runtime::config::use_user_folders();
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
        Commands::Play {
            profile,
            encryption,
            uopath,
            api_bind,
            go,
        } => play(profile, encryption, uopath, api_bind, go),
        Commands::Watch {
            text,
            uopath,
            snapshot,
            open,
        } => {
            let uopath = uopath.or_else(|| load_app_config(None).uopath);
            let shown = window::Shown { snapshot, open };
            watch_session(api, session, text, uopath, shown).await
        }
        Commands::Mcp => mcp::run_stdio(api).await,
    }
}

/// What `connect` logs in with, apart from the password and the files.
#[derive(Debug, PartialEq, Eq)]
struct ConnectChoice {
    target: LoginTarget,
    account: String,
    character: String,
    password_env: String,
    shard: Option<String>,
    era: Option<String>,
    version: Option<String>,
}

/// Each part is the one of the command line, else the one of the saved
/// login, else the one of the config file.
fn connect_choice(
    command: &Commands,
    saved: Option<&Profile>,
    cfg: &AppConfig,
) -> Result<ConnectChoice, RuntimeError> {
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
        ..
    } = command
    else {
        return Err(RuntimeError::Usage("connect".into()));
    };
    let from_saved = |pick: fn(&Profile) -> Option<String>| saved.and_then(pick);
    Ok(ConnectChoice {
        target: LoginTarget::choose(host.clone(), *port, encryption.map(Into::into), saved, cfg),
        account: account
            .clone()
            .or_else(|| from_saved(|p| Some(p.account.clone())))
            .ok_or_else(|| RuntimeError::Usage("account is required".into()))?,
        character: character
            .clone()
            .or_else(|| from_saved(|p| Some(p.character.clone())))
            .ok_or_else(|| RuntimeError::Usage("character is required".into()))?,
        password_env: password_env
            .clone()
            .or_else(|| from_saved(|p| p.password_env.clone()))
            .unwrap_or_else(|| DEFAULT_PASSWORD_ENV.into()),
        shard: shard.clone().or_else(|| from_saved(|p| p.shard.clone())),
        era: era.clone().or_else(|| from_saved(|p| p.era.clone())),
        version: version
            .clone()
            .or_else(|| from_saved(|p| p.version.clone())),
    })
}

async fn connect(cli: Cli) -> Result<u8, RuntimeError> {
    let json = cli.json;
    let cfg = load_app_config(None);
    let saved = match &cli.command {
        Commands::Connect {
            profile: Some(path),
            ..
        } => Some(load_profile(path)?),
        _ => None,
    };
    let choice = connect_choice(&cli.command, saved.as_ref(), &cfg)?;
    let Commands::Connect {
        uopath,
        markers,
        persona,
        api_bind,
        view,
        text_view,
        ..
    } = cli.command
    else {
        return Err(RuntimeError::Usage("connect".into()));
    };
    let ConnectChoice {
        target,
        account,
        character,
        password_env,
        shard,
        era,
        version,
    } = choice;
    let password = password_from_env(&password_env)?;
    let era_raw = era.unwrap_or_else(|| match cfg.era {
        Era::T2a => ERA_T2A.into(),
        Era::Modern => ERA_MODERN.into(),
    });
    let era: Era = era_raw.parse().unwrap_or(cfg.era);
    let persona = match persona {
        Some(p) => load_persona(&p)?,
        None => Persona::lumberjack_yew(),
    };
    let LoginTarget {
        host,
        port,
        encryption,
    } = target;
    let uopath = uopath.or(cfg.uopath);
    let version =
        uoterm_runtime::config::client_version(version.as_deref(), era, uopath.as_deref());
    let view_uopath = uopath.clone();
    let view_shard = window::shard_address(&host, port);
    // The file may ask for the window. The terminal view takes its place.
    let view = view || (cfg.view && !text_view);
    let markers = markers.or(cfg.markers);
    let encryption_name = EncryptionMode::from(encryption).as_str();
    tracing::info!(
        encryption = encryption_name,
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
        markers,
        persona: Some(persona),
        next_login_key: uoterm_protocol::types::LOGIN_NEXT_KEY_DEFAULT,
        encryption,
        obey_shard_rules: cfg.obey_shard_rules,
        answer_when_named: cfg.answer_when_named,
        play_along: cfg.play_along,
        picker: None,
        reconnect: cfg.reconnect,
        proxy: cfg.proxy.clone(),
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
                "encryption": encryption_name,
                "era": match era {
                    Era::T2a => ERA_T2A,
                    Era::Modern => ERA_MODERN,
                },
                "version": version.to_string(),
            })
        );
    } else {
        println!(
            "session {} started; api {bind}; encryption {encryption_name}",
            handle.id
        );
    }
    if text_view {
        let api = remote::normalize_base(&bind);
        let sid = handle.id.clone();
        std::thread::spawn(move || text_watch_loop(api, sid));
    }
    if view {
        start_api(rt.clone(), bind.clone());
        // The window and the session are one program, so the window reaches
        // the session with no delay. The window needs the main thread. The
        // session runs on the other threads. When the window closes, the
        // character stays in the world until Ctrl+C.
        let options = window::WatchOptions {
            link: window::Link::SameProgram {
                runtime: rt.clone(),
                session: handle.id.clone(),
            },
            uopath: view_uopath,
            shown: window::Shown::default(),
            shard: Some(view_shard),
        };
        if let Err(e) = tokio::task::block_in_place(|| window::open(options)) {
            tracing::error!(error = %e, "watch window");
        }
        // Closing the window ends the program. The character leaves the
        // world as she does on Ctrl+C.
        return Ok(EXIT_OK as u8);
    }
    serve_until_ctrl_c(rt, bind).await
}

fn text_watch_loop(api: String, session: String) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let clock = window::Clock::start();
    loop {
        let frame = rt.block_on(async {
            match remote::session_observe(&api, &session, view::WATCH_RADAR_SIZE).await {
                Ok(value) => view::WatchFrame::from_observe(&value, clock.seconds()),
                Err(e) => view::WatchFrame::error_frame(e.to_string()),
            }
        });
        print!("{TERM_CLEAR_HOME}{}", view::text(&frame));
        std::thread::sleep(std::time::Duration::from_millis(view::WATCH_POLL_MS));
    }
}

async fn watch_session(
    api: Option<&str>,
    session: Option<&str>,
    text: bool,
    uopath: Option<PathBuf>,
    shown: window::Shown,
) -> Result<u8, RuntimeError> {
    let base = remote::api_base(api);
    let id = remote::resolve_session(&base, session).await?;
    if text {
        let clock = window::Clock::start();
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                _ = tokio::time::sleep(std::time::Duration::from_millis(view::WATCH_POLL_MS)) => {
                    match remote::session_observe(&base, &id, view::WATCH_RADAR_SIZE).await {
                        Ok(state) => {
                            let frame = view::WatchFrame::from_observe(&state, clock.seconds());
                            print!("{TERM_CLEAR_HOME}{}", view::text(&frame));
                        }
                        Err(e) => eprintln!("{e}"),
                    }
                }
            }
        }
        return Ok(EXIT_OK as u8);
    }
    window::open(window::WatchOptions {
        link: window::Link::Http {
            api: base,
            session: id,
        },
        uopath,
        shown,
        shard: None,
    })
    .map_err(RuntimeError::Network)?;
    Ok(EXIT_OK as u8)
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

const NEEDS_PASSWORD: &str = "Type the password.";

/// The era and the client version of a login: the saved login's, or the
/// era of the config and its version.
fn login_era_version(cfg: &AppConfig, profile: Option<&Profile>) -> (Era, ClientVersion) {
    cfg.era_version(
        profile.and_then(|p| p.era.as_deref()),
        profile.and_then(|p| p.version.as_deref()),
    )
}

/// A saved login as the login screen lists it. The host, the port and the
/// encryption are the ones of the command line, else of the saved login,
/// else of the config file.
fn saved_login_row(
    cfg: &AppConfig,
    encryption: Option<EncryptionMode>,
    stored: &StoredLogin,
) -> window::SavedLogin {
    let profile = &stored.profile;
    let target = LoginTarget::choose(None, None, encryption.map(Into::into), Some(profile), cfg);
    window::SavedLogin {
        name: stored.name.clone(),
        host: target.host,
        port: target.port,
        encryption: target.encryption,
        account: profile.account.clone(),
        character: profile.character.clone(),
        shard: profile.shard.clone().unwrap_or_default(),
        password_env: profile.password_env.clone(),
        version: login_era_version(cfg, Some(profile)).1,
        deletable: stored.kept,
    }
}

/// Saves or deletes a saved login as the login screen asks, and gives the
/// list after. A save over a saved login keeps the era and the version it
/// names, which the screen does not show, and its password variable when
/// the screen names none.
fn keep_login(
    store: &LoginStore,
    row: impl Fn(&StoredLogin) -> window::SavedLogin,
    asked: window::KeepLogin,
) -> Result<Vec<window::SavedLogin>, String> {
    match asked {
        window::KeepLogin::Save { name, profile } => {
            let old = store.load(&name).ok().unwrap_or_default();
            let profile = Profile {
                password_env: profile.password_env.or(old.password_env),
                era: old.era,
                version: old.version,
                ..profile
            };
            store.save(&name, &profile).map(drop)
        }
        window::KeepLogin::Delete(name) => store.delete(&name),
    }
    .map_err(|e| e.to_string())?;
    Ok(store.list().iter().map(row).collect())
}

/// The options of a login from what the human typed. The password comes
/// from the form, or from the environment variable of the saved login.
fn play_options(
    cfg: &AppConfig,
    form: &window::LoginForm,
    profile: Option<&Profile>,
    picker: uoterm_runtime::LoginPicker,
) -> Result<ConnectOptions, String> {
    let account = form.account_name()?;
    let host = form.host_name()?;
    let port = form.port_number()?;
    let password = if form.password.is_empty() {
        profile
            .and_then(|p| p.password_env.as_deref())
            .and_then(|var| password_from_env(var).ok())
            .ok_or(NEEDS_PASSWORD)?
    } else {
        form.password.clone()
    };
    let login = ScreenLogin {
        host: host.to_string(),
        port,
        account: account.to_string(),
        password,
        shard: Some(form.shard.clone()),
        character: Some(form.character.clone()),
        era: profile.and_then(|p| p.era.clone()),
        version: profile.and_then(|p| p.version.clone()),
        encryption: form.encryption,
    };
    Ok(ConnectOptions::for_screen(login, cfg, picker))
}

/// The form `play` starts with: the saved login `--profile` names, by its
/// name or its file, else the config file and the encryption of the
/// command line.
fn start_form(
    cfg: &AppConfig,
    encryption: Option<EncryptionMode>,
    profile: Option<&std::path::Path>,
    rows: &[window::SavedLogin],
) -> window::LoginForm {
    let blank = window::LoginForm {
        host: cfg.host.clone(),
        port: cfg.port.to_string(),
        encryption: encryption.unwrap_or_default().into(),
        ..window::LoginForm::default()
    };
    profile
        .and_then(|path| path.file_stem()?.to_str())
        .and_then(|name| rows.iter().find(|row| row.name == name))
        .map_or_else(|| blank.clone(), |row| blank.filled_from(row))
}

/// `uoterm play`: the login screens, then the game window. The window has
/// the main thread. The logins and the sessions run on the other threads.
fn play(
    profile: Option<PathBuf>,
    encryption: Option<EncryptionMode>,
    uopath: Option<PathBuf>,
    api_bind: Option<String>,
    go: bool,
) -> Result<u8, RuntimeError> {
    let mut cfg = load_app_config(None);
    cfg.uopath = uopath.or(cfg.uopath);
    let bind = api_bind.unwrap_or_else(|| cfg.api_bind.clone());
    let store = LoginStore::standard();
    let row_cfg = cfg.clone();
    let row = move |stored: &StoredLogin| saved_login_row(&row_cfg, encryption, stored);
    let listed: Vec<_> = store.list().iter().map(&row).collect();
    let form = start_form(&cfg, encryption, profile.as_deref(), &listed);
    let version = listed
        .iter()
        .find(|saved| form.profile.as_ref() == Some(&saved.name))
        .map_or_else(|| login_era_version(&cfg, None).1, |saved| saved.version);
    let keep_store = store.clone();
    let keep: window::KeepLogins =
        std::sync::Arc::new(move |asked| keep_login(&keep_store, &row, asked));
    let rt = Runtime::new(cfg.max_sessions);
    let tokio = tokio::runtime::Handle::current();
    let api_started = std::sync::atomic::AtomicBool::new(false);
    let uopath = cfg.uopath.clone();
    let connect: window::Connect = std::sync::Arc::new(move |form, picker| {
        let profile = form
            .profile
            .as_deref()
            .and_then(|name| store.load(name).ok());
        let opts = play_options(&cfg, &form, profile.as_ref(), picker)?;
        let handle = tokio
            .block_on(rt.connect(opts))
            .map_err(|e| e.to_string())?;
        if !api_started.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let _guard = tokio.enter();
            start_api(rt.clone(), bind.clone());
        }
        Ok(window::Link::SameProgram {
            runtime: rt.clone(),
            session: handle.id.clone(),
        })
    });
    let options = window::PlayOptions {
        form,
        saved: listed,
        connect,
        keep,
        uopath,
        connect_at_once: go,
        version,
    };
    tokio::task::block_in_place(|| window::play(options)).map_err(RuntimeError::Network)?;
    Ok(EXIT_OK as u8)
}

fn start_api(rt: Runtime, bind: String) {
    tokio::spawn(async move {
        if let Err(e) = uoterm_runtime::api::serve(&bind, rt).await {
            tracing::error!(error = %e, "api ended");
        }
    });
}

async fn wait_ctrl_c() -> Result<u8, RuntimeError> {
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    Ok(EXIT_OK as u8)
}

async fn serve_until_ctrl_c(rt: Runtime, bind: String) -> Result<u8, RuntimeError> {
    start_api(rt, bind);
    wait_ctrl_c().await
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
                view,
                text_view,
                ..
            } => {
                assert_eq!(encryption, None, "the saved login or none decides");
                assert!(era.is_none());
                assert!(version.is_none());
                assert!(!view);
                assert!(!text_view);
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
                assert_eq!(encryption, Some(EncryptionMode::Osi));
                assert_eq!(EncryptionMode::Osi.as_str(), ENCRYPTION_OSI);
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
                assert_eq!(encryption, Some(EncryptionMode::None));
                assert_eq!(era.as_deref(), Some(ERA_T2A));
            }
            _ => panic!("expected connect"),
        }
    }

    #[test]
    fn parses_connect_view() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--view",
        ])
        .unwrap();
        match cli.command {
            Commands::Connect {
                view, text_view, ..
            } => {
                assert!(view);
                assert!(!text_view);
            }
            _ => panic!("expected connect"),
        }
    }

    #[test]
    fn parses_connect_text_view() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--text-view",
        ])
        .unwrap();
        match cli.command {
            Commands::Connect {
                view, text_view, ..
            } => {
                assert!(!view);
                assert!(text_view);
            }
            _ => panic!("expected connect"),
        }
    }

    #[test]
    fn rejects_connect_view_and_text_view() {
        let err = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--account",
            "test",
            "--character",
            "Mara",
            "--view",
            "--text-view",
        ])
        .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    fn typed(account: &str, port: &str, password: &str) -> window::LoginForm {
        window::LoginForm {
            host: "127.0.0.1".into(),
            port: port.into(),
            account: account.into(),
            password: password.into(),
            character: "Mara".into(),
            ..window::LoginForm::default()
        }
    }

    fn no_screen() -> uoterm_runtime::LoginPicker {
        uoterm_runtime::LoginPicker(tokio::sync::mpsc::unbounded_channel().0)
    }

    #[test]
    fn a_login_form_needs_an_account_a_host_a_port_and_a_password() {
        let cfg = AppConfig::default();
        let options = |form: &window::LoginForm| play_options(&cfg, form, None, no_screen());
        assert_eq!(
            options(&typed("", "2593", "pw")).unwrap_err(),
            window::NEEDS_ACCOUNT
        );
        assert_eq!(
            options(&typed("acct", "port", "pw")).unwrap_err(),
            window::BAD_PORT
        );
        assert_eq!(
            options(&typed("acct", "0", "pw")).unwrap_err(),
            window::BAD_PORT
        );
        let no_host = window::LoginForm {
            host: " ".into(),
            ..typed("acct", "2593", "pw")
        };
        assert_eq!(options(&no_host).unwrap_err(), window::NEEDS_HOST);
        assert_eq!(
            options(&typed("acct", "2593", "")).unwrap_err(),
            NEEDS_PASSWORD
        );
        let ready = options(&typed(" acct ", "2593", "pw")).unwrap();
        assert_eq!((ready.account.as_str(), ready.port), ("acct", 2593));
        assert_eq!(ready.shard, None);
        assert!(ready.picker.is_some());
        assert_eq!(ready.encryption, uoterm_runtime::EncryptionMode::None);
    }

    /// A saved login in folders of the test's own.
    fn test_store() -> (LoginStore, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("uoterm-main-logins-{}", uuid::Uuid::new_v4()));
        (
            LoginStore::at(root.join("logins"), root.join("profiles")),
            root,
        )
    }

    fn saved_mara() -> Profile {
        Profile {
            account: "mara_acct".into(),
            host: Some("saved.example.com".into()),
            port: Some(2600),
            encryption: Some(uoterm_runtime::EncryptionMode::Osi),
            password_env: Some("MARA_PASS".into()),
            character: "Mara".into(),
            shard: Some("Saved Shard".into()),
            version: Some("7.0.50.0".into()),
            era: Some(ERA_MODERN.into()),
        }
    }

    #[test]
    fn connect_takes_the_command_line_then_the_saved_login_then_the_config() {
        let cfg = AppConfig::default();
        let saved = saved_mara();
        let bare = Cli::try_parse_from(["uoterm", "connect", "--profile", "mara"]).unwrap();
        let choice = connect_choice(&bare.command, Some(&saved), &cfg).unwrap();
        assert_eq!(
            choice.target,
            LoginTarget {
                host: "saved.example.com".into(),
                port: 2600,
                encryption: uoterm_runtime::EncryptionMode::Osi,
            }
        );
        assert_eq!(
            (choice.account.as_str(), choice.character.as_str()),
            ("mara_acct", "Mara")
        );
        assert_eq!(choice.password_env, "MARA_PASS");
        assert_eq!(choice.shard.as_deref(), Some("Saved Shard"));
        assert_eq!(choice.version.as_deref(), Some("7.0.50.0"));
        let named = Cli::try_parse_from([
            "uoterm",
            "connect",
            "--profile",
            "mara",
            "--host",
            "cli.example.com",
            "--port",
            "3000",
            "--encryption",
            ENCRYPTION_NONE,
            "--account",
            "cli_acct",
            "--password-env",
            "CLI_PASS",
            "--shard",
            "Cli Shard",
        ])
        .unwrap();
        let choice = connect_choice(&named.command, Some(&saved), &cfg).unwrap();
        assert_eq!(
            choice.target,
            LoginTarget {
                host: "cli.example.com".into(),
                port: 3000,
                encryption: uoterm_runtime::EncryptionMode::None,
            }
        );
        assert_eq!(choice.account, "cli_acct");
        assert_eq!(choice.password_env, "CLI_PASS");
        assert_eq!(choice.shard.as_deref(), Some("Cli Shard"));
        assert_eq!(choice.character, "Mara", "the saved login fills the rest");
        let plain =
            Cli::try_parse_from(["uoterm", "connect", "--account", "a", "--character", "b"])
                .unwrap();
        let choice = connect_choice(&plain.command, None, &cfg).unwrap();
        assert_eq!(
            choice.target,
            LoginTarget {
                host: cfg.host.clone(),
                port: cfg.port,
                encryption: uoterm_runtime::EncryptionMode::None,
            }
        );
        assert_eq!(choice.password_env, DEFAULT_PASSWORD_ENV);
    }

    #[test]
    fn play_takes_the_command_line_then_the_saved_login_then_the_config() {
        let cfg = AppConfig::default();
        let stored = StoredLogin {
            name: "mara".into(),
            profile: saved_mara(),
            kept: true,
        };
        let row = saved_login_row(&cfg, None, &stored);
        assert_eq!((row.host.as_str(), row.port), ("saved.example.com", 2600));
        assert_eq!(row.encryption, uoterm_runtime::EncryptionMode::Osi);
        assert!(row.deletable);
        let forced = saved_login_row(&cfg, Some(EncryptionMode::None), &stored);
        assert_eq!(forced.encryption, uoterm_runtime::EncryptionMode::None);
        let old = StoredLogin {
            name: "old".into(),
            profile: Profile {
                account: "old".into(),
                ..Profile::default()
            },
            kept: false,
        };
        let row_old = saved_login_row(&cfg, None, &old);
        assert_eq!(
            (row_old.host.as_str(), row_old.port),
            (cfg.host.as_str(), cfg.port)
        );
        assert!(!row_old.deletable);
        let rows = [row, row_old];
        let form = start_form(&cfg, None, Some(std::path::Path::new("mara")), &rows);
        assert_eq!(form.host, "saved.example.com");
        assert_eq!(form.profile.as_deref(), Some("mara"));
        let form = start_form(&cfg, Some(EncryptionMode::Osi), None, &rows);
        assert_eq!(form.host, cfg.host);
        assert_eq!(form.encryption, uoterm_runtime::EncryptionMode::Osi);
    }

    #[test]
    fn a_save_from_the_screen_keeps_what_the_screen_does_not_show() {
        let (store, root) = test_store();
        let cfg = AppConfig::default();
        store.save("mara", &saved_mara()).unwrap();
        let row = |stored: &StoredLogin| saved_login_row(&cfg, None, stored);
        let from_screen = Profile {
            account: "mara_acct".into(),
            host: Some("new.example.com".into()),
            port: Some(2601),
            ..Profile::default()
        };
        let listed = keep_login(
            &store,
            row,
            window::KeepLogin::Save {
                name: "mara".into(),
                profile: from_screen,
            },
        )
        .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].host, "new.example.com");
        let text = std::fs::read_to_string(store.path("mara")).unwrap();
        assert!(!text.contains("password ="), "{text}");
        let kept = store.load("mara").unwrap();
        assert_eq!(kept.password_env.as_deref(), Some("MARA_PASS"));
        assert_eq!(kept.version.as_deref(), Some("7.0.50.0"));
        assert_eq!(kept.encryption, None, "the screen chose none");
        let listed = keep_login(&store, row, window::KeepLogin::Delete("mara".into())).unwrap();
        assert!(listed.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parses_play() {
        let cli = Cli::try_parse_from(["uoterm", "play", "--profile", "profiles/cedric.toml"]);
        assert!(matches!(
            cli.unwrap().command,
            Commands::Play {
                profile: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_watch_snapshot() {
        let cli = Cli::try_parse_from([
            "uoterm",
            "watch",
            "--open",
            "map",
            "--snapshot",
            "out.png",
            "--uopath",
            "/uo",
        ])
        .unwrap();
        match cli.command {
            Commands::Watch {
                snapshot,
                uopath,
                open,
                ..
            } => {
                assert_eq!(snapshot, Some(PathBuf::from("out.png")));
                assert_eq!(open, vec![window::Panel::Map]);
                assert_eq!(uopath, Some(PathBuf::from("/uo")));
            }
            _ => panic!("expected watch"),
        }
        assert!(Cli::try_parse_from(["uoterm", "watch", "--text", "--snapshot", "o.png"]).is_err());
    }

    #[test]
    fn parses_watch_text() {
        let cli = Cli::try_parse_from(["uoterm", "watch", "--text"]).unwrap();
        match cli.command {
            Commands::Watch { text, .. } => assert!(text),
            _ => panic!("expected watch"),
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
