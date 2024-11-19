use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::State;
use axum::{Json, Router};
use axum::routing::post;
use log::info;
use tokio::sync::Mutex;
use crate::raft::node::{NodeState, RaftNode, RaftNodeConfig};
use crate::raft::rpc::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};
use crate::raft::scheduler::Scheduler;

pub mod raft;

pub async fn start(node_config: RaftNodeConfig) {
    let raft_node = RaftNode::build(node_config.clone()); // todo: do not use clone!
    let raft_node = Arc::new(Mutex::new(raft_node));

    let mut scheduler = Scheduler::new();
    let leader_election_frequency_millis = 15000;
    scheduler.schedule_leader_election_process(leader_election_frequency_millis, raft_node.clone()).await;

    let heartbeat_millis = 1000;
    scheduler.schedule_heartbeats(heartbeat_millis, raft_node.clone()).await;

    start_accepting_requests(node_config, raft_node).await;
}

async fn start_accepting_requests(node_config: RaftNodeConfig, raft_node: Arc<Mutex<RaftNode>>) {
    // Create the Axum router and define routes
    let app = Router::new()
        .route("/request_vote", post(request_vote))
        .route("/append_entries", post(append_entries))
        .with_state(raft_node);

    let addr = SocketAddr::from(([127, 0, 0, 1], node_config.broker_port));
    info!("Raft node with id {} listening on {addr}!", node_config.node_id);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn request_vote(
    State(state): State<Arc<Mutex<RaftNode>>>,
    Json(payload): Json<RequestVoteRequest>
) -> Json<RequestVoteResponse> {
    info!("Received vote request: {:?}", payload);
    info!("Current node state: {:?}", state.lock().await);

    let mut raft_node = state.lock().await;

    if payload.term < raft_node.current_term() {
        return Json(RequestVoteResponse {
            term: raft_node.current_term(),
            vote_granted: false,
        });
    }

    // todo: add the log comparison logic here
    if raft_node.voted_for() != None && raft_node.voted_for() != Some(payload.candidate_id) {
        return Json(RequestVoteResponse {
            term: raft_node.current_term(),
            vote_granted: false,
        });
    }

    // in all other cases, I'm a follower!
    raft_node.switch_to_follower_with_term(payload.term);
    raft_node.vote_for(payload.candidate_id);

    Json(RequestVoteResponse {
        term: payload.term,
        vote_granted: true,
    })
}

async fn append_entries(
    State(state): State<Arc<Mutex<RaftNode>>>,
    Json(payload): Json<AppendEntriesRequest>
) -> Json<AppendEntriesResponse> {
    info!("Received append entries request {:?}", payload);

    let mut raft_node = state.lock().await;
    if raft_node.state() == NodeState::Leader {
        info!("Received append entries request from another leader. Current leader is {}", raft_node.node_id());
        return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
    }

    if payload.term < raft_node.current_term() {
        info!("Received append entries request with stale term ({}). Current term is {}", payload.term, raft_node.current_term());
        return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
    }

    raft_node.switch_to_follower_with_term(payload.term);
    raft_node.set_received_heartbeat();
    Json(AppendEntriesResponse { term: raft_node.current_term(), success: true})
}