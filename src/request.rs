use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Disconnect,
    GetFileCount,
    DownloadFileByIndex(u64),
    DownloadFileByName(String),
    DownloadAllFiles,
    // UploadFile(u64),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RequestResult {
    Ok,
    ErrUnauthorizedAccess,
    ErrIndexOutOfBounds,
}

impl RequestResult {
    pub fn naturalize(&self) -> Result<()> {
        match self {
            RequestResult::Ok => Ok(()),
            RequestResult::ErrUnauthorizedAccess => Err(anyhow!("Unauthorized access")),
            RequestResult::ErrIndexOutOfBounds => Err(anyhow!("Index out of bounds")),
        }
    }
}
