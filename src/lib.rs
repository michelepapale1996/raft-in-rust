use tracing_subscriber::{EnvFilter, fmt};
use crate::raft::broker::append_entries_timeout_emitter::AppendEntriesTimeoutEmitter;
use crate::raft::broker::application_request_acceptor::ApplicationRequestAcceptor;
use crate::raft::broker::broker::RaftBroker;
use crate::raft::broker::leader_election_timeout_emitter::LeaderElectionTimeoutEmitter;
use crate::raft::broker::raft_request_acceptor::RaftRequestAcceptor;
use crate::raft::model::state::{RaftState, RaftNodeConfig};
use crate::raft::rpc::raft::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};
use crate::raft::model::inner_messaging::NodeMessage;

pub mod raft;

pub async fn start(node_config: RaftNodeConfig) {
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
    let heartbeat_timeout_seconds = 4;
    append_entries_timeout_emitter.start(heartbeat_timeout_seconds).await;

    tracing::info!("Starting raft request acceptor...");
    let raft_request_acceptor = RaftRequestAcceptor::new(bus_tx.clone());
    raft_request_acceptor.start_accepting_requests(&node_config).await;

    tracing::info!("Starting application request acceptor...");
    let application_request_acceptor = ApplicationRequestAcceptor::new(bus_tx.clone());
    application_request_acceptor.start_accepting_requests(&node_config).await;

}