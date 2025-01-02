use crate::raft::model::inner_messaging::{
    EntryResponse, GetEntryRequest, NodeMessage, UpsertEntryRequest,
};
use crate::raft::model::state::RaftNodeConfig;
use crate::raft::rpc::application::dto::{ValueInformation, ValuesInformation};
use crate::raft::rpc::raft::dto::{RequestVoteRequest, RequestVoteResponse};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

pub struct ApplicationRequestAcceptor {
    bus_tx: Sender<NodeMessage>,
}

impl ApplicationRequestAcceptor {
    pub fn new(bus_tx: Sender<NodeMessage>) -> ApplicationRequestAcceptor {
        ApplicationRequestAcceptor { bus_tx }
    }

    pub async fn start_accepting_requests(&self, node_config: &RaftNodeConfig) {
        let app = Router::new()
            // .route("/v1/kv", get(get_all_values)) // todo!
            .route(
                "/v1/kv/:key",
                get(get_value_by_key).put(upsert_value_by_key),
            )
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
) -> Result<Json<ValueInformation>, StatusCode> {
    let (tx, rx) = oneshot::channel();

    let result = sender
        .send(NodeMessage::GetEntryRequest {
            payload: GetEntryRequest { key },
            reply_channel: tx,
        })
        .await;

    if let Err(error) = result {
        tracing::error!("Get entry request send error: {:?}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    match rx.await {
        Ok(response) => match response.value {
            Some(value) => Ok(Json(ValueInformation { value })),
            None => Err(StatusCode::NOT_FOUND),
        },
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn upsert_value_by_key(
    State(sender): State<Sender<NodeMessage>>,
    Path(key): Path<String>,
    Json(payload): Json<ValueInformation>,
) -> Result<Json<ValueInformation>, StatusCode> {
    let (tx, rx) = oneshot::channel();

    let result = sender
        .send(NodeMessage::UpsertEntryRequest {
            payload: UpsertEntryRequest {
                key,
                value: payload.value,
            },
            reply_channel: tx,
        })
        .await;

    if let Err(error) = result {
        tracing::error!("Upsert entry request send error: {:?}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    match rx.await {
        Ok(response) => match response.value {
            Some(value) => Ok(Json(ValueInformation { value })),
            None => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
