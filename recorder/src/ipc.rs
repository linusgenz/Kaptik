use serde::{Deserialize, Serialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSetting {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[repr(u8)]
#[derive(Debug, Serialize_repr, Deserialize_repr)]
pub enum CommandType {
    StartRecording = 0,
    StopRecording = 1,
    UpdateSetting = 2,
    Shutdown = 67,
    ShutdownUI = 68,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    #[serde(rename = "type")]
    pub(crate) type_: CommandType,
    pub update: Option<UpdateSetting>,
}
