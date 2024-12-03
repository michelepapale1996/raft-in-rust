use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::FmtSubscriber;
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

// todo: move in a separate file!
#[derive(Debug)]
enum NodeMessage {
    ElectionTimeout,
    AppendEntriesTimeout,
    RequestVote {
        payload: RequestVoteRequest,
        reply_channel: oneshot::Sender<RequestVoteResponse>
    },
    AppendEntries {
        payload: AppendEntriesRequest,
        reply_channel: oneshot::Sender<AppendEntriesResponse>
    }
}

fn init_tracing() {
    let subscriber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).expect("error setting global tracing subscriber");
}