use std::collections::HashMap;
use std::sync::Arc;
use crate::raft::broker::raft_request_executor::RaftRequestExecutor;
use crate::raft::model::state::{NodeState, RaftState};
use crate::raft::rpc::raft::dto::{RequestVoteRequest, RequestVoteResponse};

pub struct LeaderElectionHandler {
    request_executor: Arc<RaftRequestExecutor>
}

impl LeaderElectionHandler {
    pub(crate) fn new(request_executor: Arc<RaftRequestExecutor>) -> Self {
        Self {
            request_executor
        }
    }

    // #[tracing::instrument(skip(self), fields(id=?self.raft_state))]
    pub(crate) async fn start_leader_election_process(&self, raft_state: &mut RaftState) {
        let received_heartbeat = {
            if raft_state.state == NodeState::Leader {
                return
            }

            let received_heartbeat = raft_state.received_heartbeat;
            raft_state.reset_heartbeat();
            received_heartbeat
        };

        if !received_heartbeat {
            tracing::info!("I've not received an heartbeat in a timely fashion, I'll trigger a new leader election!");
            self.trigger_leader_election(raft_state).await
        }
    }

    async fn trigger_leader_election(&self, raft_state: &mut RaftState) {
        raft_state.trigger_new_election();
        tracing::info!("Starting election for term {}", raft_state.current_term);

        let cluster_hosts = raft_state.cluster_hosts();
        let request_vote_request = LeaderElectionHandler::build_request_vote_requests(raft_state);
        let request_vote_responses_by_hosts = self.request_executor.perform_vote_request(&request_vote_request).await;
        let request_vote_responses = request_vote_responses_by_hosts.values().collect::<Vec<&RequestVoteResponse>>();

        tracing::info!("Received {} responses out of {} nodes during the election process", request_vote_responses.len(), cluster_hosts.len());

        if LeaderElectionHandler::gained_quorum(cluster_hosts.len() + 1, request_vote_responses) {
            tracing::info!("I'm the leader since I've gained the quorum!");
            raft_state.switch_to_leader();

            raft_state.initialize_next_index_by_host();
            raft_state.initialize_match_index_by_host();
        } else {
            tracing::info!("I'm not the leader since I've not gained the quorum!");
        }
    }

    fn gained_quorum(cluster_size: usize, request_vote_responses: Vec<&RequestVoteResponse>) -> bool {
        let number_of_approvals = request_vote_responses.iter()
            .filter(|response| { response.vote_granted })
            .count();
        tracing::info!("Number of approvals: {number_of_approvals}");
        number_of_approvals > (cluster_size / 2)
    }

    fn build_request_vote_requests(raft_state: &mut RaftState) -> HashMap<String, RequestVoteRequest> {
        let mut request_vote_requests = HashMap::new();

        let last_log_entry = raft_state.log.last_log_entry();
        let last_log_index = last_log_entry.map_or(0, |it| it.index);
        let last_log_term = last_log_entry.map_or(0, |it| it.term);

        for host in raft_state.cluster_hosts() {
            let request = RequestVoteRequest {
                term: raft_state.current_term,
                candidate_id: raft_state.node_id(),
                last_log_index,
                last_log_term
            };
            request_vote_requests.insert(host, request);
        }

        request_vote_requests
    }
}