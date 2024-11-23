use std::collections::HashMap;
use crate::raft::model::log::Log;

#[derive(Debug, Clone)]
pub struct RaftNodeConfig {
    pub node_id: u32,
    pub broker_port: u16,
    pub cluster_hosts: Vec<String>
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader
}

#[derive(Debug)]
pub struct RaftNode {
    raft_node_config: RaftNodeConfig,
    current_term: u64,
    voted_for: Option<u32>,
    log: Log,
    state: NodeState,
    received_heartbeat: bool,
    // volatile state on all servers
    commit_index: u64,
    last_applied: u64,
    // volatile state on leaders
    next_index: HashMap<u64, u64>,
    match_index: HashMap<u64, u64>,
}

impl RaftNode {
    pub fn build(raft_node_config: RaftNodeConfig) -> RaftNode {
        RaftNode {
            raft_node_config,
            current_term: 0,
            voted_for: None,
            log: Log::new(),
            state: NodeState::Follower,
            received_heartbeat: false,
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new()
        }
    }

    pub fn cluster_hosts(&self) -> Vec<String> {
        self.raft_node_config.cluster_hosts.clone()
    }

    pub fn node_id(&self) -> u32 {
        self.raft_node_config.node_id
    }

    pub fn state(&self) -> NodeState {
        self.state
    }

    pub fn current_term(&self) -> u64 {
        self.current_term
    }

    pub fn voted_for(&self) -> Option<u32> {
        self.voted_for
    }

    pub fn received_heartbeat(&self) -> bool {
        self.received_heartbeat
    }

    pub fn set_received_heartbeat(&mut self) {
        self.received_heartbeat = true
    }

    pub fn reset_heartbeat(&mut self) {
        self.received_heartbeat = false
    }

    pub fn trigger_new_election(&mut self) {
        self.state = NodeState::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.node_id())
    }

    pub fn vote_for(&mut self, candidate_id: u32) {
        self.voted_for = Some(candidate_id);
    }

    pub fn switch_to_follower_with_term(&mut self, term: u64) {
        self.state = NodeState::Follower;
        self.current_term = term;
    }

    pub fn switch_to_leader(&mut self) {
        self.state = NodeState::Leader
    }
}