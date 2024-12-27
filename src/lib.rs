use tracing_subscriber::{EnvFilter, fmt};
use crate::raft::broker::RaftBroker;
use crate::raft::leader_election_timeout_emitter::LeaderElectionTimeoutEmitter;
use crate::raft::model::state::{RaftState, RaftNodeConfig};
use crate::raft::request_acceptor::RequestAcceptor;
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};
use crate::raft::append_entries_timeout_emitter::AppendEntriesTimeoutEmitter;

pub mod raft;

pub async fn start(node_config: RaftNodeConfig) {
    // todo: infinite queue?
    let (bus_tx, bus_rx) = tokio::sync::mpsc::channel(32);

    tracing::info!("Starting broker...");
    let mut broker = RaftBroker::new(node_config.clone(), bus_rx);
    broker.start();

    tracing::info!("Starting leader election handler...");
    let leader_election_timeout_emitter = LeaderElectionTimeoutEmitter::new(bus_tx.clone());
    let election_timeout_seconds = (5, 15);
    leader_election_timeout_emitter.start(election_timeout_seconds).await;

    tracing::info!("Starting heartbeat handler...");
    let append_entries_timeout_emitter = AppendEntriesTimeoutEmitter::new(bus_tx.clone());
    let heartbeat_timeout_seconds = 1;
    append_entries_timeout_emitter.start(heartbeat_timeout_seconds).await;

    tracing::info!("Starting request acceptor...");
    let request_acceptor = RequestAcceptor::new(bus_tx.clone());
    request_acceptor.start_accepting_requests(&node_config).await;
}