use std::{sync::Arc, time::Duration};
use log::{error, info};
use rand::{thread_rng, Rng};
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;
use tokio::time;
use crate::raft::node::{NodeState, RaftNode};
use crate::raft::rpc::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

#[derive(Debug)]
pub struct Scheduler { }

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler { }
    }

    pub async fn schedule_leader_election_process(&mut self, millis: u64, raft_node: Arc<Mutex<RaftNode>>) {
        tokio::spawn(async move {
            loop {
                time::sleep(Duration::from_millis(millis)).await;

                let mut raft_node_mutex = raft_node.lock().await;

                if raft_node_mutex.state() == NodeState::Leader {
                    // info!("I'm the leader, no need to trigger an election!");
                    continue;
                }

                let received_heartbeat = raft_node_mutex.received_heartbeat();
                raft_node_mutex.reset_heartbeat();
                drop(raft_node_mutex); // todo: is there a better way?

                if !received_heartbeat {
                    info!("I've not received an heartbeat in a timely fashion, I'll trigger a new leader election!");
                    trigger_leader_election(raft_node.clone()).await
                } else {
                    // info!("I'm not the leader, but I received an heartbeat in a timely fashion, no need to trigger an election!");
                }
            }
        });
    }

    pub async fn schedule_heartbeats(&self, millis: u64, raft_node: Arc<Mutex<RaftNode>>) {
        let mut rng = thread_rng();
        let random = rng.gen_range(1..5);

        let sleep_time_millis = random * millis;
        info!("I'll try to send heartbeats every {sleep_time_millis} seconds");
        tokio::spawn(async move {
            loop {

                time::sleep(Duration::from_millis(sleep_time_millis)).await;

                let raft_node_mutex = raft_node.lock().await;

                if raft_node_mutex.state() != NodeState::Leader {
                    // info!("I'm not the leader, no need to send heartbeats to other brokers!");
                    continue;
                }

                let cluster_hosts = raft_node_mutex.cluster_hosts();
                let term_of_current_broker = raft_node_mutex.current_term();

                drop(raft_node_mutex);

                let append_entries_request = build_append_entries_request(raft_node.clone()).await;
                let append_entries_responses = perform_append_entries_requests(cluster_hosts, append_entries_request).await;

                let max_term_in_response = append_entries_responses.iter()
                    .map(|response| response.term)
                    .max()
                    .unwrap_or(term_of_current_broker);

                if max_term_in_response > term_of_current_broker {
                    info!("Received heartbeat response with a higher term. I will become a follower");
                    raft_node.lock().await.switch_to_follower_with_term(max_term_in_response);
                }
            }
        });
    }
}

async fn trigger_leader_election(raft_node: Arc<Mutex<RaftNode>>) {
    let mut raft_node_mutex = raft_node.lock().await;
    raft_node_mutex.trigger_new_election();

    info!("Starting election for term {}", raft_node_mutex.current_term());

    let cluster_hosts = raft_node_mutex.cluster_hosts(); // understand if there exist another way
    let request_vote_request = build_request_vote_request(raft_node_mutex);
    let request_vote_responses = perform_vote_request(cluster_hosts, request_vote_request).await;

    // todo: parametrize cluster size
    info!("Received {} responses out of {} nodes during the election process", request_vote_responses.len(), 2);

    if gained_quorum(3, request_vote_responses) {
        info!("I'm the leader since I've gained the quorum!");
        raft_node.lock().await.switch_to_leader();
        // todo: initialize nextIndexByHost and matchIndexByHost
        /*
        for (String serverId: clusterState.getOtherClusterNodes()) {
            nextIndexByHost.put(serverId, log.size());
            matchIndexByHost.put(serverId, -1);
        }
         */
    } else {
        info!("I'm not the leader since I've not gained the quorum!");
    }
}

// todo: performing the API calls are identical -> generify the code!
async fn perform_append_entries_requests(cluster_hosts: Vec<String>, append_entries_request: AppendEntriesRequest) -> Vec<AppendEntriesResponse> {
    let client = reqwest::Client::new(); // todo: do not create at each iteration

    let mut responses: Vec<AppendEntriesResponse> = vec![];
    for (cluster_host_index, cluster_host) in cluster_hosts.iter().enumerate() {
        let endpoint = format!("http://{}/append_entries", cluster_host);

        let res = client.post(&endpoint)
            .json(&append_entries_request)
            .send()
            .await;

        if res.is_ok() {
            responses.push(res.unwrap().json().await.unwrap())
        } else {
            error!("Error while calling endpoint {}", endpoint);
            error!("{:?}", res.err().unwrap())
        }
    }

    responses
}

async fn perform_vote_request(cluster_hosts: Vec<String>, request_vote_request: RequestVoteRequest) -> Vec<RequestVoteResponse> {
    let client = reqwest::Client::new(); // todo: do not create at each iteration

    let mut responses: Vec<RequestVoteResponse> = vec![];
    for (cluster_host_index, cluster_host) in cluster_hosts.iter().enumerate() {
        let endpoint = format!("http://{}/request_vote", cluster_host);

        let res = client.post(&endpoint)
            .json(&request_vote_request)
            .send()
            .await;

        if res.is_ok() {
            responses.push(res.unwrap().json().await.unwrap())
        } else {
            error!("Error while calling endpoint {}", endpoint);
            error!("{:?}", res.err().unwrap())
        }
    }

    responses
}

fn gained_quorum(cluster_size: usize, request_vote_responses: Vec<RequestVoteResponse>) -> bool {
    request_vote_responses.iter()
        .filter(|response| { response.vote_granted })
        .count() > (cluster_size / 2)
}


fn build_request_vote_request(raft_node_mutex: MutexGuard<RaftNode>) -> RequestVoteRequest {
    RequestVoteRequest {
        term: raft_node_mutex.current_term(),
        candidate_id: raft_node_mutex.node_id(),
        last_log_index: 0, // todo
        last_log_term: 0 // todo
    }
}

async fn build_append_entries_request(raft_node: Arc<Mutex<RaftNode>>) -> AppendEntriesRequest {
    let raft_node_mutex = raft_node.lock().await;

    AppendEntriesRequest {
        term: raft_node_mutex.current_term(),
        leader_id: raft_node_mutex.node_id(),
        prev_log_index: 0,
        prev_log_term: 0,
        entries: vec![],
        leader_commit: 0
    }
}