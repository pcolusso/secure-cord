use anyhow::Result;
use clap::Parser;
use ssm::Session;

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
    let sess = Session::new(instance_id, host_port, dest_port);
    sess.start().await;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        eprintln!("{}", sess.healthy().await);
        eprintln!("{:?}", sess.stdout().await);
        eprintln!("{:?}", sess.stderr().await);
    }
}
