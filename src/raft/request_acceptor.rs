use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::State;
use axum::{Json, Router};
use axum::routing::post;
use log::info;
use tokio::sync::Mutex;
use crate::raft::model::node::{NodeState, RaftNode, RaftNodeConfig};
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};


pub async fn start_accepting_requests(node_config: &RaftNodeConfig, raft_node: Arc<Mutex<RaftNode>>) {
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

    if payload.term > raft_node.current_term() {
        raft_node.switch_to_follower_with_term(payload.term);
        raft_node.vote_for(payload.candidate_id);

        return Json(RequestVoteResponse {
            term: payload.term,
            vote_granted: true,
        })
    }

    // todo: add the log comparison logic here
    if raft_node.voted_for() != None && raft_node.voted_for() != Some(payload.candidate_id) {
        return Json(RequestVoteResponse {
            term: raft_node.current_term(),
            vote_granted: false,
        });
    }
    // nothing to do: I'm in the same term and I already voted for him!
    return Json(RequestVoteResponse {
        term: payload.term,
        vote_granted: true,
    });

}

async fn append_entries(
    State(state): State<Arc<Mutex<RaftNode>>>,
    Json(payload): Json<AppendEntriesRequest>
) -> Json<AppendEntriesResponse> {
    info!("Received append entries request {:?}", payload);

    let mut raft_node = state.lock().await;
    /*
    if raft_node.state() == NodeState::Leader {
        info!("Received append entries request from another leader. Current leader is {}", raft_node.node_id());
        return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
    }
    */

    if payload.term < raft_node.current_term() {
        info!("Received append entries request with stale term ({}). Current term is {}", payload.term, raft_node.current_term());
        return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
    }

    raft_node.switch_to_follower_with_term(payload.term);
    raft_node.set_received_heartbeat();
    Json(AppendEntriesResponse { term: raft_node.current_term(), success: true})
}