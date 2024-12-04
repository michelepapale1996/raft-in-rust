use tokio::sync::oneshot;
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

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
    }
}