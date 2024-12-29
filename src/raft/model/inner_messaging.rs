use tokio::sync::oneshot;
use crate::raft::rpc::application::dto::{EntryResponse, GetEntryRequest, UpsertEntryRequest};
use crate::raft::rpc::raft::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

#[derive(Debug)]
pub enum NodeMessage {
    ElectionTimeout,
    AppendEntriesTimeout,
    RequestVote {
        payload: RequestVoteRequest,
        reply_channel: oneshot::Sender<RequestVoteResponse>
    },
    AppendEntries {
        payload: AppendEntriesRequest,
        reply_channel: oneshot::Sender<AppendEntriesResponse>
    },
    GetEntryRequest {
        payload: GetEntryRequest,
        reply_channel: oneshot::Sender<EntryResponse>
    },
    UpsertEntryRequest {
        payload: UpsertEntryRequest,
        reply_channel: oneshot::Sender<EntryResponse>
    }
}