use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub term: u64,
    pub index: i64,
    pub entry: Entry
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub value: String
}

#[derive(Debug)]
pub struct Log {
    entries: Vec<LogEntry>
}

impl Log {
    pub fn new() -> Log {
        Log { entries: vec![] }
    }

    pub fn last_log_entry(&self) -> Option<&LogEntry> {
        self.entries.last()
    }

    pub fn entry_at(&self, index: i64) -> Option<&LogEntry> {
        for entry in self.entries.iter() {
            if entry.index == index {
                return Some(entry);
            }
        }
        None
    }

    pub fn entries_starting_from_index(&self, index: i64) -> Vec<&LogEntry> {
        let mut entries = vec![];
        for entry in self.entries.iter() {
            if entry.index >= index {
                entries.push(entry);
            }
        }
        entries
    }

    pub fn append(&mut self, key: &str, value: &str, term: u64) {
        let index = self.last_log_entry().map_or(0, |entry| entry.index + 1);

        let entry = Entry { key: key.to_owned(), value: value.to_owned() };
        let entry = LogEntry { term, index, entry};
        self.entries.push(entry);
    }
}