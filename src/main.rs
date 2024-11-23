use clap::Parser;
use log::{info, LevelFilter};
use chrono::Local;
use env_logger::Builder;
use std::io::Write;
use raft_in_rust::raft::model::node::RaftNodeConfig;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[arg(short, long)]
    cluster_hosts: String,

    #[arg(short, long)]
    broker_port: u16,
}

fn build_raft_node_config(cli_args: CliArgs) -> RaftNodeConfig {
    let cluster_hosts = cli_args.cluster_hosts
        .split(",")
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    RaftNodeConfig { node_id: cli_args.broker_port as u32, broker_port: cli_args.broker_port, cluster_hosts }
}

#[tokio::main]
async fn main() {
    let cli_args = CliArgs::parse();
    let node_config = build_raft_node_config(cli_args);

    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] - {}",
                     Local::now().format("%Y-%m-%dT%H:%M:%S"),
                     record.level(),
                     record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    info!("Starting server...");

    raft_in_rust::start(node_config).await;
}
