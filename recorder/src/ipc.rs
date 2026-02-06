use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSetting {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandType {
    StartRecording = 0,
    StopRecording = 1,
    UpdateSetting = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    pub(crate) type_: CommandType,
    pub update: Option<UpdateSetting>,
}
