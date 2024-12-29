use std::net::SocketAddr;
use axum::extract::{Path, State};
use axum::{Json, Router};
use axum::http::StatusCode;
use axum::routing::{get, post};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use crate::raft::model::inner_messaging::NodeMessage;
use crate::raft::model::state::RaftNodeConfig;
use crate::raft::model::state_machine::StateMachineCommand;
use crate::raft::rpc::application::dto::{EntryResponse, GetEntryRequest, UpsertEntryRequest};
use crate::raft::rpc::raft::dto::{RequestVoteRequest, RequestVoteResponse};

pub struct ApplicationRequestAcceptor {
    bus_tx: Sender<NodeMessage>
}

impl ApplicationRequestAcceptor {
    pub fn new(bus_tx: Sender<NodeMessage>) -> ApplicationRequestAcceptor {
        ApplicationRequestAcceptor {
            bus_tx
        }
    }

    pub async fn start_accepting_requests(&self, node_config: &RaftNodeConfig) {
        let app = Router::new()
            .route("/:key", get(get_value_by_key))
            .route("/:key", post(upsert_value_by_key))
            .with_state(self.bus_tx.clone());

        let addr = SocketAddr::from(([127, 0, 0, 1], node_config.application_port));
        tracing::info!("Raft node ready to listen for application requests on {addr}!");

        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

async fn get_value_by_key(
    State(sender): State<Sender<NodeMessage>>,
    Path(key): Path<String>,
) -> Result<Json<EntryResponse>, StatusCode> {
    let (tx, rx) = oneshot::channel();

    let result = sender.send(NodeMessage::GetEntryRequest {
        payload: GetEntryRequest { key },
        reply_channel: tx
    }).await;

    if let Err(error) = result {
        tracing::error!("Get entry request send error: {:?}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // todo: in case it does not exist, return proper http status code
    match rx.await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// todo: here the body should not contain the key - decouple the http layer from the model
async fn upsert_value_by_key(
    State(sender): State<Sender<NodeMessage>>,
    Path(key): Path<String>,
    Json(payload): Json<UpsertEntryRequest>
) -> Result<Json<EntryResponse>, StatusCode> {
    let (tx, rx) = oneshot::channel();

    let result = sender.send(NodeMessage::UpsertEntryRequest {
        payload,
        reply_channel: tx
    }).await;

    if let Err(error) = result {
        tracing::error!("Upsert entry request send error: {:?}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    match rx.await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}