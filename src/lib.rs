use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use crate::raft::broker::RaftBroker;
use crate::raft::model::state::{RaftState, RaftNodeConfig};
use crate::raft::request_acceptor::RequestAcceptor;
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};
use crate::raft::scheduler::Scheduler;

pub mod raft;

pub async fn start(node_config: RaftNodeConfig) {
    init_tracing();

    // todo: infinite queue?
    let (bus_tx, bus_rx) = mpsc::channel(32);

    tracing::info!("Starting broker...");
    // bus reader logic
    let raft_state = RaftState::build(node_config.clone());
    let mut broker = RaftBroker::new(raft_state, bus_rx);

    tokio::spawn(async move { broker.main_loop().await });

    tracing::info!("Starting scheduler...");
    // bus writers logic
    let scheduler = Scheduler::new(bus_tx.clone());
    let election_timeout_seconds = (5, 15);
    scheduler.schedule_leader_election_process(election_timeout_seconds).await;
    let heartbeat_timeout_seconds = 1;
    scheduler.schedule_heartbeats(heartbeat_timeout_seconds).await;

    tracing::info!("Starting accepting requests...");
    let request_acceptor = RequestAcceptor::new(bus_tx.clone());
    request_acceptor.start_accepting_requests(&node_config).await;
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}