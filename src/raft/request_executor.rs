use std::sync::Arc;
use crate::raft::rpc::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub struct RequestExecutor {
    client: reqwest::Client
}

impl RequestExecutor {
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        Self {
            client
        }
    }

    pub async fn perform_append_entries_requests(&self, cluster_hosts: &[String], append_entries_request: AppendEntriesRequest) -> Vec<AppendEntriesResponse> {
        let mut responses: Vec<AppendEntriesResponse> = vec![];
        for (_cluster_host_index, cluster_host) in cluster_hosts.iter().enumerate() {
            let endpoint = format!("http://{}/append_entries", cluster_host);

            let res = self.client.post(&endpoint)
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

    pub async fn perform_vote_request(&self, cluster_hosts: &[String], request_vote_request: RequestVoteRequest) -> Vec<RequestVoteResponse> {
        let mut responses: Vec<RequestVoteResponse> = vec![];
        for (_cluster_host_index, cluster_host) in cluster_hosts.iter().enumerate() {
            let endpoint = format!("http://{}/request_vote", cluster_host);

            let res = self.client.post(&endpoint)
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
}