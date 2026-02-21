// agent_lookup.rs

use crate::log;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Deserialize)]
struct AgentsResponse {
    data: Vec<ApiAgent>,
}

#[derive(Deserialize)]
struct ApiAgent {
    uuid: String,

    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Serialize, Deserialize, Default)]
struct AgentCache {
    agents: HashMap<String, AgentEntry>,
}

#[derive(Serialize, Deserialize)]
struct AgentEntry {
    name: String,
}

pub async fn agent_id_to_name(uuid: &str) -> Option<String> {
    let uuid = uuid.to_lowercase();

    ensure_cache_exists().await.ok()?;

    if let Some(name) = AGENT_CACHE
        .read()
        .unwrap()
        .agents
        .get(&uuid)
        .map(|a| a.name.clone())
    {
        return Some(name);
    }

    let name = fetch_single_agent(&uuid).await?;

    {
        let mut cache = AGENT_CACHE.write().unwrap();

        cache.agents.insert(
            uuid.clone(),
            AgentEntry {
                name: name.clone(),
            },
        );

        let _ = save_cache(&cache);
    }

    Some(name)
}

static AGENT_CACHE: Lazy<RwLock<AgentCache>> =
    Lazy::new(|| RwLock::new(load_cache().unwrap_or_default()));

fn agents_cache_path() -> PathBuf {
    config_dir().join("agents.toml")
}

fn config_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap();
    dir.push("Kaptik");
    dir
}

fn load_cache() -> Result<AgentCache> {
    let path = agents_cache_path();

    if !path.exists() {
        return Ok(AgentCache::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn save_cache(cache: &AgentCache) -> Result<()> {
    let path = agents_cache_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(cache)?;
    fs::write(path, toml)?;

    Ok(())
}

async fn ensure_cache_exists() -> Result<()> {
    let path = agents_cache_path();

    if path.exists() {
        return Ok(());
    }

    let endpoint = "https://valorant-api.com/v1/agents";

    log!("agents.toml missing → fetching all entries from {}", endpoint);

    let res = reqwest::get(endpoint).await?;
    let data: AgentsResponse = res.json().await?;

    let mut cache = AgentCache::default();

    for agent in data.data {
        cache.agents.insert(
            agent.uuid.to_lowercase(),
            AgentEntry {
                name: agent.display_name,
            },
        );
    }

    save_cache(&cache)?;

    if let Ok(mut guard) = AGENT_CACHE.try_write() {
        *guard = cache;
    } else {
        log!("[WARNING] Failed to acquire write lock, skipping cache update");
    }

    Ok(())
}

async fn fetch_single_agent(uuid: &str) -> Option<String> {
    let url = format!("https://valorant-api.com/v1/agents/{uuid}");

    let res = reqwest::get(url).await.ok()?;
    let data: serde_json::Value = res.json().await.ok()?;

    data["data"]["displayName"]
        .as_str()
        .map(|s| s.to_string())
}