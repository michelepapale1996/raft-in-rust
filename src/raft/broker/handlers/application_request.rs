use tokio::sync::oneshot::Sender;
use crate::raft::model::state::RaftState;
use crate::raft::model::state_machine::StateMachine;
use crate::raft::rpc::application::dto::{EntryResponse, GetEntryRequest, UpsertEntryRequest};

pub struct ApplicationRequestHandler {}

impl ApplicationRequestHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle_get_request(&self,
                                    state_machine: &mut StateMachine,
                                    request: GetEntryRequest,
                                    reply_channel: Sender<EntryResponse>) {
        let value = state_machine.get(&*request.key).map(|string| {string.to_owned()});
        let response = EntryResponse { key: request.key, value };
        reply_channel.send(response).unwrap();
    }

    pub async fn handle_upsert_request(&self, raft_state: &mut RaftState, request: UpsertEntryRequest, reply_channel: Sender<EntryResponse>) {
        // todo: handle the case where I'm not the leader!

        let term = raft_state.current_term;
        raft_state.log.append(&request.key, &request.value, term);

        // todo: await till the entry is not persisted & replicated in the log

        reply_channel.send(EntryResponse { key: request.key, value: Some(request.value) } ).unwrap()
    }
}