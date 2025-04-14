use std::collections::HashMap;

use anyhow::Result;
use aws_sdk_ssm as ssm;
use clap::Parser;

mod ssm;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Config {
    instance_id: String,
    host_port: usize,
    dest_port: usize
}


#[tokio::main]
async fn main() -> Result<()> {
    let Config { instance_id, host_port, dest_port } = Config::parse();
    let parameters = HashMap::from([
        ("portNumber".into(), vec![dest_port.to_string()]),
        ("localPortNumber".into(), vec![host_port.to_string()])
    ]);

    let config = aws_config::load_from_env().await;
    let client = ssm::Client::new(&config);

    Ok(())
}
