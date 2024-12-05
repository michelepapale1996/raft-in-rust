use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub async fn perform_append_entries_requests(cluster_hosts: &[String], append_entries_request: AppendEntriesRequest) -> Vec<AppendEntriesResponse> {
    let client = reqwest::Client::new(); // todo: do not create at each iteration

    let mut responses: Vec<AppendEntriesResponse> = vec![];
    for (_cluster_host_index, cluster_host) in cluster_hosts.iter().enumerate() {
        let endpoint = format!("http://{}/append_entries", cluster_host);

        let res = client.post(&endpoint)
            .json(&append_entries_request)
            .send()
            .await;

        if res.is_ok() {
            responses.push(res.unwrap().json().await.unwrap())
        } else {
            tracing::error!("Error while calling endpoint {}", endpoint);
        }
    }

    responses
}

pub async fn perform_vote_request(cluster_hosts: &[String], request_vote_request: RequestVoteRequest) -> Vec<RequestVoteResponse> {
    let client = reqwest::Client::new(); // todo: do not create at each iteration

    let mut responses: Vec<RequestVoteResponse> = vec![];
    for (_cluster_host_index, cluster_host) in cluster_hosts.iter().enumerate() {
        let endpoint = format!("http://{}/request_vote", cluster_host);

        let res = client.post(&endpoint)
            .json(&request_vote_request)
            .send()
            .await;

        if res.is_ok() {
            responses.push(res.unwrap().json().await.unwrap())
        } else {
            tracing::error!("Error while calling endpoint {}", endpoint);
        }
    }

    responses
}