use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct APMData {
    #[serde(rename = "series")]
    pub series: Vec<(f64, u32)>, // (second, APM)
}

pub fn save_apm_msgpack<P: AsRef<Path>>(
    apm: &APMData,
    path: P,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;
    encode::write(&mut file, apm)?;
    Ok(())
}

pub fn load_apm_msgpack<P: AsRef<Path>>(path: P) -> Result<APMData, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let apm: APMData = decode::from_read(file)?;
    Ok(apm)
}

