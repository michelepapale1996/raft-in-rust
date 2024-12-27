use tracing::Level;
use tokio::sync::oneshot;
use tokio::sync::mpsc::Receiver;
use crate::raft::model::inner_messaging::NodeMessage;
use crate::raft::model::state::{NodeState, RaftNodeConfig, RaftState};
use crate::raft::request_executor;
use crate::raft::request_executor::RequestExecutor;
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub struct RaftBroker {
    raft_state: RaftState,
    bus_rx: Receiver<NodeMessage>,
    request_executor: RequestExecutor
}

impl RaftBroker {
    pub fn new(node_config: RaftNodeConfig, bus_rx: Receiver<NodeMessage>) -> RaftBroker {
        let raft_state = RaftState::build(node_config);
        let request_executor = RequestExecutor::new();
        Self {
            raft_state,
            bus_rx,
            request_executor
        }
    }

    pub(crate) fn start(mut self) {
        tokio::spawn(async move { self.main_loop().await });
    }

    pub async fn main_loop(&mut self) {
        loop {
            match self.bus_rx.recv().await {
                Some(msg) => self.process_received_message(msg).await,
                None => tracing::error!("Received nothing")
            }
        }
    }

    #[tracing::instrument(level = Level::DEBUG, skip(self), fields(state=?self.raft_state, msg=?msg))]
    async fn process_received_message(&mut self, msg: NodeMessage) {
        match msg {
            NodeMessage::ElectionTimeout => self.start_leader_election_process().await,
            NodeMessage::AppendEntriesTimeout => self.start_append_entries_process().await,
            NodeMessage::AppendEntries { payload, reply_channel} =>
                self.append_entries_handler(payload, reply_channel).await,
            NodeMessage::RequestVote { payload, reply_channel} =>
                self.request_vote_handler(payload, reply_channel).await
        }
    }

    // #[tracing::instrument(skip(self), fields(id=?self.raft_state))]
    async fn start_leader_election_process(&mut self) {
        let received_heartbeat = {
            if self.raft_state.state() == NodeState::Leader {
                return
            }

            let received_heartbeat = self.raft_state.received_heartbeat();
            self.raft_state.reset_heartbeat();
            received_heartbeat
        };

        if !received_heartbeat {
            tracing::info!("I've not received an heartbeat in a timely fashion, I'll trigger a new leader election!");
            self.trigger_leader_election().await
        }
    }

    // #[tracing::instrument(skip(self), fields(id=?self.raft_state))]
    async fn start_append_entries_process(&mut self) {
        if self.raft_state.state() != NodeState::Leader {
            tracing::info!("I'm not the leader, no need to send append entries");
            return
        }

        let cluster_hosts = self.raft_state.cluster_hosts();
        let term_of_current_broker = self.raft_state.current_term();

        let append_entries_request = build_append_entries_request(&self.raft_state).await;
        let append_entries_responses = self.request_executor.perform_append_entries_requests(&cluster_hosts, append_entries_request).await;

        let max_term_in_response = append_entries_responses.iter()
            .map(|response| response.term)
            .max()
            .unwrap_or(term_of_current_broker);

        if max_term_in_response > term_of_current_broker {
            tracing::info!("Received heartbeat response with a higher term. I will become a follower");
            self.raft_state.switch_to_follower_with_term(max_term_in_response);
        }
    }

    async fn request_vote_handler(&mut self, payload: RequestVoteRequest, reply_channel: oneshot::Sender<RequestVoteResponse>) {
        tracing::info!("Received vote request: {:?}", payload);
        tracing::info!("Current node state: {:?}", self.raft_state);

        if payload.term < self.raft_state.current_term() {
            reply_channel.send(RequestVoteResponse {
                term: self.raft_state.current_term(),
                vote_granted: false,
            }).unwrap();
            return
        }

        if payload.term > self.raft_state.current_term() {
            self.raft_state.switch_to_follower_with_term(payload.term);
            self.raft_state.vote_for(payload.candidate_id);

            reply_channel.send(RequestVoteResponse {
                term: payload.term,
                vote_granted: true,
            }).unwrap();
            return;
        }

        // todo: add the log comparison logic here
        if self.raft_state.voted_for() != None && self.raft_state.voted_for() != Some(payload.candidate_id) {
            reply_channel.send(RequestVoteResponse {
                term: self.raft_state.current_term(),
                vote_granted: false,
            }).unwrap();
            return
        }
        // nothing to do: I'm in the same term and I already voted for him!
        reply_channel.send(RequestVoteResponse {
            term: payload.term,
            vote_granted: true,
        }).unwrap()
    }

    async fn append_entries_handler(&mut self,
                                    payload: AppendEntriesRequest,
                                    reply_channel: oneshot::Sender<AppendEntriesResponse>) {
        tracing::info!("Received append entries request {:?}", payload);

        /*
        if raft_node.state() == NodeState::Leader {
            tracing::info!("Received append entries request from another leader. Current leader is {}", raft_node.node_id());
            return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
        }
        */

        if payload.term < self.raft_state.current_term() {
            tracing::info!("Received append entries request with stale term ({}). Current term is {}", payload.term, self.raft_state.current_term());
            reply_channel.send(AppendEntriesResponse { term: self.raft_state.current_term(), success: false}).unwrap();
            return
        }

        self.raft_state.switch_to_follower_with_term(payload.term);
        self.raft_state.set_received_heartbeat();
        reply_channel.send(AppendEntriesResponse { term: self.raft_state.current_term(), success: true}).unwrap()
    }

    async fn trigger_leader_election(&mut self) {
        self.raft_state.trigger_new_election();
        tracing::info!("Starting election for term {}", self.raft_state.current_term());

        let cluster_hosts = self.raft_state.cluster_hosts();
        let request_vote_request = build_request_vote_request(&self.raft_state);
        let request_vote_responses = self.request_executor.perform_vote_request(&cluster_hosts, request_vote_request).await;

        tracing::info!("Received {} responses out of {} nodes during the election process", request_vote_responses.len(), cluster_hosts.len());

        if gained_quorum(cluster_hosts.len() + 1, request_vote_responses) {
            tracing::info!("I'm the leader since I've gained the quorum!");
            self.raft_state.switch_to_leader();
            // todo: initialize nextIndexByHost and matchIndexByHost
            /*
            for (String serverId: clusterState.getOtherClusterNodes()) {
                nextIndexByHost.put(serverId, log.size());
                matchIndexByHost.put(serverId, -1);
            }
             */
        } else {
            tracing::info!("I'm not the leader since I've not gained the quorum!");
        }
    }
}

fn gained_quorum(cluster_size: usize, request_vote_responses: Vec<RequestVoteResponse>) -> bool {
    let number_of_approvals = request_vote_responses.iter()
        .filter(|response| { response.vote_granted })
        .count();
    tracing::info!("Number of approvals: {number_of_approvals}");
    number_of_approvals > (cluster_size / 2)
}


fn build_request_vote_request(raft_node: &RaftState) -> RequestVoteRequest {
    RequestVoteRequest {
        term: raft_node.current_term(),
        candidate_id: raft_node.node_id(),
        last_log_index: 0, // todo
        last_log_term: 0 // todo
    }
}

async fn build_append_entries_request(raft_node: &RaftState) -> AppendEntriesRequest {
    AppendEntriesRequest {
        term: raft_node.current_term(),
        leader_id: raft_node.node_id(),
        prev_log_index: 0,
        prev_log_term: 0,
        entries: vec![],
        leader_commit: 0
    }
}