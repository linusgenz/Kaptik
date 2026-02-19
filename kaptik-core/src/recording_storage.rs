use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use anyhow::Result;
use crate::domain::recording::RecordingData;

// Storage functions
pub fn save_recording_data<P: AsRef<Path>>(
    data: &RecordingData,
    path: P,
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;
    encode::write(&mut file, data)?;
    Ok(())
}

pub fn load_recording_data<P: AsRef<Path>>(path: P) -> Result<RecordingData> {
    let file = File::open(path)?;
    let data: RecordingData = decode::from_read(file)?;
    Ok(data)
}

pub fn get_recording_path(recording_id: &Uuid) -> Result<PathBuf> {
    let mut dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("No local data dir found"))?;

    dir.push("Kaptik");
    dir.push("recordings");

    fs::create_dir_all(&dir)?;

    Ok(dir.join(format!("{}.msgpack", recording_id)))
}