use std::net::SocketAddr;
use std::sync::mpsc::SendError;
use axum::extract::State;
use axum::{Error, Json, Router};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use reqwest::Response;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tracing::error;
use crate::raft::model::inner_messaging::NodeMessage;
use crate::raft::model::state::{RaftNodeConfig};
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub struct RequestAcceptor {
    bus_tx: Sender<NodeMessage>
}

impl RequestAcceptor {
    pub fn new(bus_tx: Sender<NodeMessage>) -> RequestAcceptor {
        Self {
            bus_tx
        }
    }

    pub async fn start_accepting_requests(&self, node_config: &RaftNodeConfig) {
        let app = Router::new()
            .route("/request_vote", post(request_vote))
            .route("/append_entries", post(append_entries))
            .with_state(self.bus_tx.clone());

        let addr = SocketAddr::from(([127, 0, 0, 1], node_config.broker_port));
        tracing::info!("Raft node with id {} listening on {addr}!", node_config.node_id);
        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

async fn request_vote(
    State(sender): State<Sender<NodeMessage>>,
    Json(payload): Json<RequestVoteRequest>
) -> Result<Json<RequestVoteResponse>, StatusCode> {
    let (tx, rx) = oneshot::channel();

    let result = sender.send(NodeMessage::RequestVote {
        payload,
        reply_channel: tx
    }).await;

    if let Err(error) = result {
        tracing::error!("request vote send error: {:?}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    match rx.await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn append_entries(
    State(sender): State<Sender<NodeMessage>>,
    Json(payload): Json<AppendEntriesRequest>
) -> Result<Json<AppendEntriesResponse>, StatusCode> {
    let (tx, rx) = oneshot::channel();

    let result = sender.send(NodeMessage::AppendEntries {
        payload,
        reply_channel: tx
    }).await;

    if let Err(error) = result {
        tracing::error!("request vote send error: {:?}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    match rx.await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}