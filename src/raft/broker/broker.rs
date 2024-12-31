use std::cmp::min;
use std::collections::HashMap;
use std::iter::Map;
use std::sync::Arc;
use tracing::Level;
use tokio::sync::oneshot;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use crate::raft::broker::handlers::append_entries::AppendEntriesHandler;
use crate::raft::broker::handlers::leader_election::LeaderElectionHandler;
use crate::raft::broker::handlers::request_vote::RequestVoteHandler;
use crate::raft::broker::raft_request_executor::RaftRequestExecutor;
use crate::raft::model::inner_messaging::{EntryResponse, NodeMessage};
use crate::raft::model::log::LogEntry;
use crate::raft::model::state::{NodeState, RaftNodeConfig, RaftState};
use crate::raft::model::state_machine::StateMachine;
use crate::raft::rpc::raft::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub struct RaftBroker {
    raft_state: RaftState,
    state_machine: StateMachine,
    bus_rx: Receiver<NodeMessage>,
    leader_election_handler: LeaderElectionHandler,
    append_entries_handler: AppendEntriesHandler
}

impl RaftBroker {
    pub fn new(node_config: RaftNodeConfig, bus_rx: Receiver<NodeMessage>) -> RaftBroker {
        let raft_state = RaftState::build(node_config);
        let request_executor = Arc::new(RaftRequestExecutor::new());

        let leader_election_handler = LeaderElectionHandler::new(request_executor.clone());
        let append_entries_handler = AppendEntriesHandler::new(request_executor.clone());

        Self {
            raft_state,
            state_machine: StateMachine::new(),
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

    // todo: this method should be agnostic from the messages
    #[tracing::instrument(level = Level::TRACE, skip(self), fields(state=?self.raft_state, msg=?msg))]
    async fn process_received_message(&mut self, msg: NodeMessage) {
        match msg {
            // local broker timeouts
            NodeMessage::ElectionTimeout =>
                self.leader_election_handler.start_leader_election_process(&mut self.raft_state).await,
            NodeMessage::AppendEntriesTimeout =>
                self.append_entries_handler.start_append_entries_process(&mut self.raft_state, &mut self.state_machine).await,

            // here starts intra-cluster communication requests
            NodeMessage::AppendEntries { payload, reply_channel } =>
                self.append_entries_handler.consume_append_entries(&mut self.raft_state, &mut self.state_machine, payload, reply_channel).await,
            NodeMessage::RequestVote { payload, reply_channel } =>
                RequestVoteHandler::handle_request_vote_request(&mut self.raft_state, payload, reply_channel).await,

            // here starts application requests
            NodeMessage::GetEntryRequest { payload , reply_channel } => {
                let value = self.state_machine.get(&*payload.key).map(|string| {string.to_owned()});
                reply_channel.send(EntryResponse { key: payload.key, value }).unwrap();
            },
            NodeMessage::UpsertEntryRequest { payload, reply_channel} => {
                // todo: handle the case where I'm not the leader!


                self.raft_state.log.append(&payload.key, &payload.value, self.raft_state.current_term);
                tracing::info!("Sending append entries to replicate the log...");
                self.append_entries_handler.start_append_entries_process(&mut self.raft_state, &mut self.state_machine).await;
                // todo: arrived here, I'm still not sure that my entry has been committed!
                reply_channel.send(EntryResponse { key: payload.key, value: Some(payload.value) } ).unwrap()
            }
        }
    }
}