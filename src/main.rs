use anyhow::Result;
use clap::Parser;
use servers::Server;
use ssm::Session;
use std::path::PathBuf;

mod ssm;
mod ui;
mod servers;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Config {
    #[arg(short, long, default_value = "/Users/paulcolusso/Documents/jobs.json")]
    connections_file: PathBuf
}

type Uhh = (Session, Server, bool);


#[tokio::main]
async fn main() -> Result<()> {
    let Config { connections_file } = Config::parse();
    let servers = servers::load(connections_file).await?;
    let mapped: Vec<Uhh> = servers
        .into_iter()
        .map(|s| (Session::new(s.identifier.clone(), s.env.clone(),  s.host_port.clone(), s.dest_port.clone()), s, false)).collect();

    ui::run(mapped).await?;

    Ok(())
}
