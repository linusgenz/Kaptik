use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KDA {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
}