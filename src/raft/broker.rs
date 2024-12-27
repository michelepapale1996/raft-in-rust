use std::sync::Arc;
use tracing::Level;
use tokio::sync::oneshot;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use crate::raft::model::inner_messaging::NodeMessage;
use crate::raft::model::state::{NodeState, RaftNodeConfig, RaftState};
use crate::raft::request_executor;
use crate::raft::request_executor::RequestExecutor;
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub struct RaftBroker {
    raft_state: RaftState,
    bus_rx: Receiver<NodeMessage>,
    leader_election_handler: LeaderElectionHandler,
    append_entries_handler: AppendEntriesHandler
}

impl RaftBroker {
    pub fn new(node_config: RaftNodeConfig, bus_rx: Receiver<NodeMessage>) -> RaftBroker {
        let raft_state = RaftState::build(node_config);
        let request_executor = Arc::new(RequestExecutor::new());

        let leader_election_handler = LeaderElectionHandler::new(request_executor.clone());
        let append_entries_handler = AppendEntriesHandler::new(request_executor.clone());

        Self {
            raft_state,
            bus_rx,
            leader_election_handler,
            append_entries_handler
        }
    }

    pub(crate) fn start(mut self) {
        tokio::spawn(async move {
            loop {
                match self.bus_rx.recv().await {
                    Some(msg) => self.process_received_message(msg).await,
                    None => tracing::error!("Received nothing")
                }
            }
        });
    }

    #[tracing::instrument(level = Level::DEBUG, skip(self), fields(state=?self.raft_state, msg=?msg))]
    async fn process_received_message(&mut self, msg: NodeMessage) {
        match msg {
            NodeMessage::ElectionTimeout =>
                self.leader_election_handler.start_leader_election_process(&mut self.raft_state).await,
            NodeMessage::AppendEntriesTimeout =>
                self.append_entries_handler.start_append_entries_process(&mut self.raft_state).await,
            NodeMessage::AppendEntries { payload, reply_channel } =>
                self.append_entries_handler.consume_append_entries(&mut self.raft_state, payload, reply_channel).await,
            NodeMessage::RequestVote { payload, reply_channel } =>
                RequestVoteHandler::handle_request_vote_request(&mut self.raft_state, payload, reply_channel).await
        }
    }
}

struct LeaderElectionHandler {
    request_executor: Arc<RequestExecutor>
}
impl LeaderElectionHandler {
    fn new(request_executor: Arc<RequestExecutor>) -> Self {
        Self {
            request_executor
        }
    }

    // #[tracing::instrument(skip(self), fields(id=?self.raft_state))]
    async fn start_leader_election_process(&self, raft_state: &mut RaftState) {
        let received_heartbeat = {
            if raft_state.state() == NodeState::Leader {
                return
            }

            let received_heartbeat = raft_state.received_heartbeat();
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
        tracing::info!("Starting election for term {}", raft_state.current_term());

        let cluster_hosts = raft_state.cluster_hosts();
        let request_vote_request = LeaderElectionHandler::build_request_vote_request(raft_state);
        let request_vote_responses = self.request_executor.perform_vote_request(&cluster_hosts, request_vote_request).await;

        tracing::info!("Received {} responses out of {} nodes during the election process", request_vote_responses.len(), cluster_hosts.len());

        if LeaderElectionHandler::gained_quorum(cluster_hosts.len() + 1, request_vote_responses) {
            tracing::info!("I'm the leader since I've gained the quorum!");
            raft_state.switch_to_leader();
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
}

struct AppendEntriesHandler {
    request_executor: Arc<RequestExecutor>
}
impl AppendEntriesHandler {
    fn new(request_executor: Arc<RequestExecutor>) -> Self {
        Self { request_executor }
    }

    // #[tracing::instrument(skip(self), fields(id=?self.raft_state))]
    async fn start_append_entries_process(&self, raft_state: &mut RaftState) {
        if raft_state.state() != NodeState::Leader {
            tracing::debug!("I'm not the leader, no need to send append entries");
            return
        }

        let cluster_hosts = raft_state.cluster_hosts();
        let term_of_current_broker = raft_state.current_term();

        let append_entries_request = AppendEntriesHandler::build_append_entries_request(raft_state).await;
        let append_entries_responses = self.request_executor.perform_append_entries_requests(&cluster_hosts, append_entries_request).await;

        let max_term_in_response = append_entries_responses.iter()
            .map(|response| response.term)
            .max()
            .unwrap_or(term_of_current_broker);

        if max_term_in_response > term_of_current_broker {
            tracing::info!("Received heartbeat response with a higher term. I will become a follower");
            raft_state.switch_to_follower_with_term(max_term_in_response);
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

    async fn consume_append_entries(&self,
                                    raft_state: &mut RaftState,
                                    payload: AppendEntriesRequest,
                                    reply_channel: oneshot::Sender<AppendEntriesResponse>) {
        tracing::info!("Received append entries request {:?}", payload);

        /*
        if raft_node.state() == NodeState::Leader {
            tracing::info!("Received append entries request from another leader. Current leader is {}", raft_node.node_id());
            return Json(AppendEntriesResponse { term: raft_node.current_term(), success: false})
        }
        */

        if payload.term < raft_state.current_term() {
            tracing::info!("Received append entries request with stale term ({}). Current term is {}", payload.term, raft_state.current_term());
            reply_channel.send(AppendEntriesResponse { term: raft_state.current_term(), success: false}).unwrap();
            return
        }

        raft_state.switch_to_follower_with_term(payload.term);
        raft_state.set_received_heartbeat();
        reply_channel.send(AppendEntriesResponse { term: raft_state.current_term(), success: true}).unwrap()
    }
}

struct RequestVoteHandler {}

impl RequestVoteHandler {

    pub async fn handle_request_vote_request(raft_state: &mut RaftState, payload: RequestVoteRequest, reply_channel: oneshot::Sender<RequestVoteResponse>) {
        tracing::info!("Received vote request: {:?}", payload);
        tracing::info!("Current node state: {:?}", raft_state);

        if payload.term < raft_state.current_term() {
            reply_channel.send(RequestVoteResponse {
                term: raft_state.current_term(),
                vote_granted: false,
            }).unwrap();
            return
        }

        if payload.term > raft_state.current_term() {
            raft_state.switch_to_follower_with_term(payload.term);
            raft_state.vote_for(payload.candidate_id);

            reply_channel.send(RequestVoteResponse {
                term: payload.term,
                vote_granted: true,
            }).unwrap();
            return;
        }

        // todo: add the log comparison logic here
        if raft_state.voted_for() != None && raft_state.voted_for() != Some(payload.candidate_id) {
            reply_channel.send(RequestVoteResponse {
                term: raft_state.current_term(),
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
}