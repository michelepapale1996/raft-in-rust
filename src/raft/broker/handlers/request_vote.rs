use tokio::sync::oneshot;
use crate::raft::model::state::RaftState;
use crate::raft::rpc::raft::dto::{RequestVoteRequest, RequestVoteResponse};

pub struct RequestVoteHandler {}

impl RequestVoteHandler {

    pub async fn handle_request_vote_request(raft_state: &mut RaftState, payload: RequestVoteRequest, reply_channel: oneshot::Sender<RequestVoteResponse>) {
        tracing::info!("Received vote request: {:?}", payload);
        tracing::info!("Current node state: {:?}", raft_state);

        if payload.term < raft_state.current_term {
            reply_channel.send(RequestVoteResponse {
                term: raft_state.current_term,
                vote_granted: false,
            }).unwrap();
            return
        }

        if payload.term > raft_state.current_term {
            raft_state.switch_to_follower_with_term(payload.term);
            raft_state.vote_for(payload.candidate_id);

            reply_channel.send(RequestVoteResponse {
                term: payload.term,
                vote_granted: true,
            }).unwrap();
            return;
        }

        // check if I already voted for another member!
        if raft_state.voted_for != None && raft_state.voted_for != Some(payload.candidate_id) {
            reply_channel.send(RequestVoteResponse {
                term: raft_state.current_term,
                vote_granted: false,
            }).unwrap();
            return
        }

        let last_log_entry = raft_state.log.last_log_entry();
        let my_last_log_index = last_log_entry.map_or(0, |it| it.index);
        if payload.last_log_index < my_last_log_index {
            let entry_at_requested_index = raft_state.log.entries_starting_from_index(payload.last_log_index);
            if entry_at_requested_index.is_empty() {
                tracing::error!("Unable to find entry in the log, this should not happen");
                // todo, understand how to terminate gracefully
                return
            }

            let entry = entry_at_requested_index.first().unwrap();
            if entry.term != payload.last_log_term {
                reply_channel.send(RequestVoteResponse {
                    term: raft_state.current_term,
                    vote_granted: false,
                }).unwrap();
                return
            } else {
                reply_channel.send(RequestVoteResponse {
                    term: payload.term,
                    vote_granted: true,
                }).unwrap();
                return
            }
        } else {
            reply_channel.send(RequestVoteResponse {
                term: raft_state.current_term,
                vote_granted: false,
            }).unwrap();
            return
        }
    }
}
