use std::sync::Arc;
use tokio::sync::Mutex;
use crate::raft::model::node::{RaftNode, RaftNodeConfig};
use crate::raft::request_acceptor::start_accepting_requests;
use crate::raft::scheduler::Scheduler;

pub mod raft;

pub async fn start(node_config: RaftNodeConfig) {
    let raft_node = RaftNode::build(node_config.clone());
    let raft_node = Arc::new(Mutex::new(raft_node));

    let mut scheduler = Scheduler::new();
    let election_timeout_seconds = (5, 15);
    scheduler.schedule_leader_election_process(election_timeout_seconds, raft_node.clone()).await;

    let heartbeat_timeout_seconds = 1;
    scheduler.schedule_heartbeats(heartbeat_timeout_seconds, raft_node.clone()).await;

    start_accepting_requests(&node_config, raft_node.clone()).await;
}