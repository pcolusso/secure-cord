use anyhow::Result;
use clap::Parser;
use ssm::Session;
use std::path::PathBuf;

mod ssm;
mod ui;
mod servers;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Config {
    #[arg(short, long, default_value = "/Users/paulcolusso/Documents/connections.json")]
    connections_file: PathBuf
}


#[tokio::main]
async fn main() -> Result<()> {
    let Config { connections_file } = Config::parse();
    let servers = servers::load(connections_file).await?;

    ui::run(servers).await?;

    Ok(())
}
