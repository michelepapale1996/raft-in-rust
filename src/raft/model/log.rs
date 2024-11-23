use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LogEntry {
    term: u64,
    command: Command
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    data: Vec<u8>
}

#[derive(Debug)]
pub struct Log {
    entries: Vec<LogEntry>
}

impl Log {
    pub fn new() -> Log {
        Log {
            entries: vec![]
        }
    }
}