use crate::raft::rpc::raft::dto::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct GetEntryRequest {
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertEntryRequest {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EntryResponse {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug)]
pub enum NodeMessage {
    ElectionTimeout,
    AppendEntriesTimeout,
    RequestVote {
        payload: RequestVoteRequest,
        reply_channel: oneshot::Sender<RequestVoteResponse>,
    },
    AppendEntries {
        payload: AppendEntriesRequest,
        reply_channel: oneshot::Sender<AppendEntriesResponse>,
    },
    GetEntryRequest {
        payload: GetEntryRequest,
        reply_channel: oneshot::Sender<EntryResponse>,
    },
    UpsertEntryRequest {
        payload: UpsertEntryRequest,
        reply_channel: oneshot::Sender<EntryResponse>,
    },
}
