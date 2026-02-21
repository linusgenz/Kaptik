// map_lookup.rs

use crate::log;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Deserialize)]
struct MapsResponse {
    data: Vec<ApiMap>,
}

#[derive(Deserialize)]
struct ApiMap {
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "mapUrl")]
    map_url: String,
}

#[derive(Serialize, Deserialize, Default)]
struct MapCache {
    maps: HashMap<String, MapEntry>,
}

#[derive(Serialize, Deserialize)]
struct MapEntry {
    name: String,
}

pub async fn map_id_to_name(map_url: &str) -> String {
    let map_url = map_url.to_lowercase();

    ensure_cache_exists().await.ok();

    if let Some(entry) = MAP_CACHE.read().unwrap().maps.get(&map_url) {
        return entry.name.clone();
    }

    if let Some(name) = fetch_map_from_api(&map_url).await {
        let mut cache = MAP_CACHE.write().unwrap();
        cache.maps.insert(map_url.clone(), MapEntry { name: name.clone() });
        let _ = save_cache(&cache);
        return name;
    }

    "Unknown Map".to_string()
}

static MAP_CACHE: Lazy<RwLock<MapCache>> =
    Lazy::new(|| RwLock::new(load_cache().unwrap_or_default()));

fn maps_cache_path() -> PathBuf {
    config_dir().join("maps.toml")
}

fn config_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap();
    dir.push("Kaptik");
    dir
}

fn load_cache() -> Result<MapCache> {
    let path = maps_cache_path();

    if !path.exists() {
        return Ok(MapCache::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn save_cache(cache: &MapCache) -> Result<()> {
    let path = maps_cache_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(cache)?;
    fs::write(path, toml)?;

    Ok(())
}

async fn ensure_cache_exists() -> Result<()> {
    let path = maps_cache_path();

    if path.exists() {
        return Ok(());
    }

    let endpoint = "https://valorant-api.com/v1/maps";
    log!("maps.toml missing → fetching all entries from {}", endpoint);

    let res = reqwest::get(endpoint).await?;
    let data: MapsResponse = res.json().await?;

    let mut cache = MapCache::default();
    for map in data.data {
        cache.maps.insert(map.map_url.to_lowercase(), MapEntry { name: map.display_name });
    }

    save_cache(&cache)?;

    if let Ok(mut guard) = MAP_CACHE.try_write() {
        *guard = cache;
    } else {
        log!("[WARNING] Failed to acquire write lock, skipping cache update");
    }

    Ok(())
}

async fn fetch_map_from_api(map_url: &str) -> Option<String> {
    let endpoint = "https://valorant-api.com/v1/maps";

    let res = reqwest::get(endpoint).await.ok()?;
    let data: MapsResponse = res.json().await.ok()?;

    for map in data.data {
        if map.map_url.to_lowercase() == map_url {
            return Some(map.display_name);
        }
    }

    None
}