use serde::Deserialize;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Deserialize, Clone)]
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

pub async fn load(path: impl AsRef<Path>) -> Result<Vec<Server>> {
    let data = tokio::fs::read_to_string(path).await?;
    let entries: Vec<Server> = serde_json::from_str(&data)?;
    Ok(entries)
}
