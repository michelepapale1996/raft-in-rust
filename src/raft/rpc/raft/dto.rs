use crate::raft::model::log::LogEntry;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: u32,
    pub prev_log_index: i64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestVoteRequest {
    pub term: u64,
    pub candidate_id: u32,
    pub last_log_index: i64,
    pub last_log_term: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}
