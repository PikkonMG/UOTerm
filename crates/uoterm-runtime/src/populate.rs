use crate::config::{
    era_from_str, load_persona, load_profile, password_from_env, version_from_str, ConnectOptions,
};
use crate::error::Result;
use crate::manager::Runtime;
use crate::tools::{Goal, ToolCall, TOOL_SET_GOAL};
use chrono::Timelike;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use uoterm_protocol::types::Era;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub era: Option<String>,
    #[serde(default)]
    pub uopath: Option<PathBuf>,
    pub agents: Vec<ManifestAgent>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestAgent {
    pub profile: PathBuf,
    pub persona: PathBuf,
}

pub async fn run(runtime: &Runtime, path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| crate::error::RuntimeError::Usage(format!("manifest: {e}")))?;
    let man: Manifest = toml::from_str(&text)
        .map_err(|e| crate::error::RuntimeError::Usage(format!("manifest parse: {e}")))?;
    let era: Era = era_from_str(man.era.as_deref().unwrap_or("modern"));
    let now = chrono::Local::now();
    let mut ids = Vec::new();
    for agent in man.agents {
        let persona = load_persona(&agent.persona)?;
        if !persona.is_active_now(now.hour(), now.minute()) {
            tracing::info!(name = %persona.name, "outside active hours; skip");
            continue;
        }
        let profile = load_profile(&agent.profile)?;
        let password = password_from_env(&profile.password_env)?;
        let version = version_from_str(profile.version.as_deref(), era);
        let goal = Goal::for_class(&persona.class).name();
        let opts = ConnectOptions {
            host: man.host.clone(),
            port: man.port,
            account: profile.account,
            password,
            shard: profile.shard,
            character: profile.character,
            version,
            era,
            uopath: man.uopath.clone(),
            persona: Some(persona),
            stay_on_socket: crate::config::stay_on_socket_for_era(era, true),
            next_login_key: uoterm_protocol::types::LOGIN_NEXT_KEY_DEFAULT,
            encryption: Default::default(),
            obey_shard_rules: crate::config::OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: crate::config::ANSWER_WHEN_NAMED_DEFAULT,
            play_along: crate::config::PLAY_ALONG_DEFAULT,
        };
        let handle = runtime.connect(opts).await?;
        let _ = handle
            .call(ToolCall {
                name: TOOL_SET_GOAL.into(),
                args: serde_json::json!({ "goal": goal }),
            })
            .await;
        ids.push(handle.id);
    }
    Ok(ids)
}
