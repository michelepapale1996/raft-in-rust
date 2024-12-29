use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct GetEntryRequest {
    pub key: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpsertEntryRequest {
    pub key: String,
    pub value: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EntryResponse {
    pub key: String,
    pub value: Option<String>
}