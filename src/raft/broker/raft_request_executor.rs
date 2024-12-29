use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Response;
use serde::Serialize;
use crate::raft::rpc::raft::dto::{AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse};

pub struct RaftRequestExecutor {
    client: reqwest::Client
}

impl RaftRequestExecutor {
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        Self {
            client
        }
    }

    pub async fn perform_append_entries_requests(&self, append_entries_requests_by_host: &HashMap<String, AppendEntriesRequest>)
        -> HashMap<String, AppendEntriesResponse> {
        self.perform_requests("append_entries", append_entries_requests_by_host).await
    }

    pub async fn perform_vote_request(&self, request_vote_request: &HashMap<String, RequestVoteRequest>)
        -> HashMap<String, RequestVoteResponse> {
        self.perform_requests("request_vote", request_vote_request).await
    }

    async fn perform_requests<T, U>(&self, endpoint_suffix: &str, request_bodies_by_host: &HashMap<String, T>)
        -> HashMap<String, U>
    where
        T: serde::Serialize,
        U: serde::de::DeserializeOwned + std::fmt::Debug
    {
        let mut responses: HashMap<String, U> = HashMap::new();
        for (cluster_host, request_body) in request_bodies_by_host {
            let endpoint = format!("http://{}/{}", cluster_host, endpoint_suffix);

            let res = self.client.post(&endpoint)
                .json(&request_body)
                .send()
                .await;

            if let Ok(response) = res {
                let json = response.json().await;
                if let Ok(json) = json {
                    responses.insert(cluster_host.to_string(), json);
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