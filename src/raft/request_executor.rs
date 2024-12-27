use std::sync::Arc;
use reqwest::Response;
use serde::Serialize;
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
        self.perform_requests(cluster_hosts, "append_entries", &append_entries_request).await
    }

    pub async fn perform_vote_request(&self, cluster_hosts: &[String], request_vote_request: RequestVoteRequest) -> Vec<RequestVoteResponse> {
        self.perform_requests(cluster_hosts, "request_vote", &request_vote_request).await
    }

    async fn perform_requests<T, U>(&self, cluster_hosts: &[String], endpoint_suffix: &str, request_body: &T) -> Vec<U>
    where
        T: serde::Serialize,
        U: serde::de::DeserializeOwned + std::fmt::Debug
    {
        let mut responses: Vec<U> = vec![];
        for cluster_host in cluster_hosts {
            let endpoint = format!("http://{}/{}", cluster_host, endpoint_suffix);

            let res = self.client.post(&endpoint)
                .json(request_body)
                .send()
                .await;

            if let Ok(response) = res {
                let json = response.json().await;
                if let Ok(json) = json {
                    responses.push(json);
                } else {
                    tracing::error!("Error while parsing JSON from {:?}", json);
                }
            } else {
                tracing::error!("Error while calling endpoint {}", endpoint);
            }
        }

        responses
    }
}