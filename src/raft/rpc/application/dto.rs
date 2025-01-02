use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ValueInformation {
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ValuesInformation {
    pub values: Vec<ValueInformation>,
}
