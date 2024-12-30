use std::cmp::min;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use crate::raft::broker::raft_request_executor::RaftRequestExecutor;
use crate::raft::model::state::{NodeState, RaftState};
use crate::raft::model::state_machine::StateMachine;
use crate::raft::rpc::raft::dto::{AppendEntriesRequest, AppendEntriesResponse};

pub struct AppendEntriesHandler {
    request_executor: Arc<RaftRequestExecutor>
}

impl AppendEntriesHandler {
    pub(crate) fn new(request_executor: Arc<RaftRequestExecutor>) -> Self {
        Self { request_executor }
    }

    // #[tracing::instrument(skip(self), fields(id=?self.raft_state))]
    pub async fn start_append_entries_process(&self, raft_state: &mut RaftState, state_machine: &mut StateMachine) {
        if raft_state.state != NodeState::Leader {
            tracing::trace!("I'm not the leader, no need to send append entries");
            return
        }

        let append_entries_requests_by_host = AppendEntriesHandler::build_append_entries_request(raft_state);
        let append_entries_responses_by_host = self.request_executor.perform_append_entries_requests(&append_entries_requests_by_host).await;
        let append_entries_responses = append_entries_responses_by_host.values().collect::<Vec<&AppendEntriesResponse>>();

        tracing::info!("Raft state: {:?}", raft_state);
        tracing::info!("Append entries requests sent: {:?}", append_entries_requests_by_host);
        tracing::info!("Append entries responses received: {:?}", append_entries_responses_by_host);

        let term_of_current_broker = raft_state.current_term;
        let max_term_in_response = append_entries_responses.iter()
            .map(|response| response.term)
            .max()
            .unwrap_or(term_of_current_broker);

        if max_term_in_response > term_of_current_broker {
            tracing::info!("Received heartbeat response with a higher term. I will become a follower");
            raft_state.switch_to_follower_with_term(max_term_in_response);
        } else {
            for (cluster_host, append_entry_response) in append_entries_responses_by_host.iter() {
                if append_entry_response.success {
                    let request_sent = append_entries_requests_by_host.get(cluster_host);

                    if let Some(request_sent) = request_sent {
                        let last_accepted_entry = request_sent.entries.last();

                        // in case last_accepted_entry is none, it means nothing must be done since it was just an heartbeat
                        if let Some(last_accepted_entry) = last_accepted_entry {
                            let next_index = last_accepted_entry.index + 1;
                            let match_index = last_accepted_entry.index;

                            raft_state.set_next_index_for_host(cluster_host, next_index);
                            raft_state.set_match_index_for_host(cluster_host, match_index);
                        }
                    }
                } else {
                    let next_index = raft_state.get_next_index_for_host(cluster_host);
                    if let Some(next_index) = next_index {
                        raft_state.set_next_index_for_host(cluster_host, next_index - 1)
                    }
                }
            }

            let my_last_index = raft_state.log.last_log_entry().map_or(-1, |it| it.index);

            let mut values = raft_state.get_match_index_by_host().values().collect::<Vec<_>>();
            values.sort();
            let n = *values.get(values.len() / 2).unwrap();

            if *n > raft_state.commit_index && *n <= my_last_index && raft_state.log.entry_at(*n).unwrap().term == raft_state.current_term {
                raft_state.commit_index = *n
            }

            self.apply_command_to_state_machine(raft_state, state_machine);
        }
    }

    fn build_append_entries_request_for_host(raft_state: &RaftState, host: &str) -> AppendEntriesRequest {
        let last_log_entry = raft_state.log.last_log_entry();

        match last_log_entry {
            Some(last_log_entry) => {
                let last_log_index = last_log_entry.index;
                let last_log_term = last_log_entry.term;

                let next_index_for_host = raft_state.get_next_index_for_host(host);
                if let Some(next_index_for_host) = next_index_for_host {
                    if last_log_index >= next_index_for_host {

                        AppendEntriesRequest {
                            term: raft_state.current_term,
                            leader_id: raft_state.node_id(),
                            prev_log_index: raft_state.log.entry_at(next_index_for_host - 1).map_or(-1, |it| it.index),
                            prev_log_term: raft_state.log.entry_at(next_index_for_host - 1).map_or(0, |it| it.term),
                            entries: raft_state.log.entries_starting_from_index(next_index_for_host).into_iter().cloned().collect::<Vec<_>>(),
                            leader_commit: raft_state.commit_index
                        }
                    } else {
                        // host is up to date, send just an heartbeat
                        AppendEntriesRequest {
                            term: raft_state.current_term,
                            leader_id: raft_state.node_id(),
                            prev_log_index: last_log_index,
                            prev_log_term: last_log_term,
                            entries: vec![],
                            leader_commit: raft_state.commit_index
                        }
                    }
                } else {
                    tracing::error!("No next index for host '{}'", host);
                    panic!()
                }
            },
            // the log is empty, send just an heartbeat
            None => {
                AppendEntriesRequest {
                    term: raft_state.current_term,
                    leader_id: raft_state.node_id(),
                    prev_log_index: -1,
                    prev_log_term: 0,
                    entries: vec![],
                    leader_commit: raft_state.commit_index
                }
            }
        }
    }

    fn build_append_entries_request(raft_state: &RaftState) -> HashMap<String, AppendEntriesRequest> {
        let mut append_entries_requests_by_host = HashMap::<String, AppendEntriesRequest>::new();
        for host in raft_state.cluster_hosts() {
            let append_entries = AppendEntriesHandler::build_append_entries_request_for_host(raft_state, &host);
            append_entries_requests_by_host.insert(host, append_entries);
        }

        append_entries_requests_by_host
    }

    pub async fn consume_append_entries(&self,
                                               raft_state: &mut RaftState,
                                               state_machine: &mut StateMachine,
                                               payload: AppendEntriesRequest,
                                               reply_channel: oneshot::Sender<AppendEntriesResponse>) {
        // tracing::info!("Received append entries request {:?}", payload);

        /*
        if raft_node.state() == NodeState::Leader {
            tracing::info!("Received append entries request from another leader. Current leader is {}", raft_node.node_id());
            return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
        }
        */

        if payload.term < raft_state.current_term {
            tracing::info!("Received append entries request with stale term ({}). Current term is {}", payload.term, raft_state.current_term);
            reply_channel.send(AppendEntriesResponse { term: raft_state.current_term, success: false}).unwrap();
            return
        }

        // in all other cases, I'm a follower that has received either a heartbeat or an append entries request
        raft_state.switch_to_follower_with_term(payload.term);
        raft_state.set_received_heartbeat();

        let last_log_entry = raft_state.log.last_log_entry();

        match last_log_entry {
            Some(last_log_entry) => {
                if payload.prev_log_index < last_log_entry.index &&
                    raft_state.log.entry_at(payload.prev_log_index).is_some_and(|it| it.term != payload.prev_log_term) {

                    reply_channel.send(AppendEntriesResponse { term: raft_state.current_term, success: false}).unwrap();
                } else {
                    for entry in payload.entries.iter() {
                        tracing::info!("Appending entry {:?} to the log", entry);
                        raft_state.log.append(&*entry.entry.key, &*entry.entry.value, entry.term);
                    }

                    if payload.leader_commit > raft_state.commit_index {
                        let last_index = raft_state.log.last_log_entry().map_or(0, |it| it.index);
                        raft_state.commit_index = min(payload.leader_commit, last_index)
                    }

                    self.apply_command_to_state_machine(raft_state, state_machine);

                    reply_channel.send(AppendEntriesResponse { term: raft_state.current_term, success: true}).unwrap();
                }
            },
            None => {
                // the log is empty
                if payload.prev_log_index == -1 {
                    for entry in payload.entries {
                        tracing::info!("Appending first command {:?} on follower log", entry.entry);
                        raft_state.log.append(&*entry.entry.key, &*entry.entry.value, entry.term);
                    }
                    reply_channel.send(AppendEntriesResponse { term: raft_state.current_term, success: true}).unwrap();
                } else {
                    // the log is empty and the prevLogIndex is not -1 - I'm not in sync!
                    reply_channel.send(AppendEntriesResponse { term: raft_state.current_term, success: false}).unwrap();
                }
            }
        }
    }

    fn apply_command_to_state_machine(&self, raft_state: &mut RaftState, state_machine: &mut StateMachine) {
        let last_applied = raft_state.last_applied;
        let commit_index = raft_state.commit_index;

        if commit_index > last_applied {
            for i in last_applied + 1..commit_index + 1 {
                let log_entry = raft_state.log.entry_at(i);

                match log_entry {
                    Some(log_entry) => {
                        tracing::info!("Applying {:?} on this server...", log_entry);
                        state_machine.insert(log_entry.entry.key.as_str(), log_entry.entry.value.as_str());
                    },
                    None => {
                        tracing::error!("Error: inconsistent!");
                    }
                }
            }
            raft_state.last_applied = commit_index
        }
    }
}
