use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use raft_in_rust::raft::model::state::RaftNodeConfig;

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

    init_tracing();
    raft_in_rust::start(node_config).await;
}


fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}