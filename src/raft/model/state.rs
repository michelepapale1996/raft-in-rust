use std::collections::HashMap;
use crate::raft::model::log::Log;

#[derive(Debug, Clone)]
pub struct RaftNodeConfig {
    pub node_id: u32,
    pub raft_port: u16,
    pub cluster_hosts: Vec<String>,
    pub application_port: u16
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader
}

#[derive(Debug)]
pub struct RaftState {
    raft_node_config: RaftNodeConfig,
    pub current_term: u64,
    pub voted_for: Option<u32>,
    pub log: Log,
    pub state: NodeState,
    pub received_heartbeat: bool,
    // volatile state on all servers
    pub commit_index: i64,
    pub last_applied: i64,
    // volatile state on leaders
    next_index_by_host: HashMap<String, i64>,
    match_index_by_host: HashMap<String, i64>,
}

impl RaftState {
    pub fn build(raft_node_config: RaftNodeConfig) -> RaftState {
        RaftState {
            raft_node_config,
            current_term: 0,
            voted_for: None,
            log: Log::new(),
            state: NodeState::Follower,
            received_heartbeat: false,
            commit_index: -1,
            last_applied: -1,
            next_index_by_host: HashMap::new(),
            match_index_by_host: HashMap::new()
        }
    }

    pub fn cluster_hosts(&self) -> Vec<String> {
        self.raft_node_config.cluster_hosts.clone()
    }
    pub fn node_id(&self) -> u32 {
        self.raft_node_config.node_id
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

    pub fn initialize_next_index_by_host(&mut self) {
        self.next_index_by_host = HashMap::new();
        self.cluster_hosts().iter().for_each(|host| {
            let last_index = self.log.last_log_entry().map_or(-1, |l| l.index);
            self.next_index_by_host.insert(host.to_owned(), last_index + 1);
        })
    }

    pub fn get_next_index_for_host(&self, host: &str) -> Option<i64> {
        self.next_index_by_host.get(host).cloned()
    }

    pub fn set_next_index_for_host(&mut self, host: &str, next_index: i64) {
        self.next_index_by_host.insert(host.to_owned(), next_index);
    }

    pub fn initialize_match_index_by_host(&mut self) {
        self.match_index_by_host = HashMap::new();
        self.cluster_hosts().iter().for_each(|host| {
            self.match_index_by_host.insert(host.to_owned(), -1);
        })
    }

    pub fn get_match_index_by_host(&self) -> &HashMap<String, i64> {
        &self.match_index_by_host
    }

    pub fn set_match_index_for_host(&mut self, host: &str, match_index: i64) {
        self.match_index_by_host.insert(host.to_owned(), match_index);
    }
}