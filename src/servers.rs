use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Server {
    #[serde(rename = "instanceId")]
    pub identifier: String,
    pub env: String,
    #[serde(rename = "sourcePort")]
    pub host_port: usize,
    pub name: String,
    #[serde(rename = "destPort")]
    pub dest_port: usize,
}

pub fn load(path: impl AsRef<Path>) -> Result<Vec<Server>> {
    let data = std::fs::read_to_string(path)?;
    let entries: Vec<Server> = serde_json::from_str(&data)?;
    Ok(entries)
}

pub fn save(path: impl AsRef<Path>, servers: &[Server]) -> Result<()> {
    let json = serde_json::to_string_pretty(servers)?;
    std::fs::write(path, json)?;
    Ok(())
}
