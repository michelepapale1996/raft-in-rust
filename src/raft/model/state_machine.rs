use std::collections::HashMap;

pub struct StateMachine {
    state_by_key: HashMap<String, String>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state_by_key: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.state_by_key.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.state_by_key.get(key).map(String::as_str)
    }
}
