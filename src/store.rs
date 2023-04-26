use std::{collections::HashMap, time::Duration};

use tokio::time::Instant;

use crate::resp::Value;

pub struct Store {
    data: HashMap<String, Entry>,
}

pub struct Entry {
    pub value: String,
    pub t: Option<Instant>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, Entry { value, t: None });
    }

    pub fn set_px(&mut self, key: String, value: String, px: u64) {
        let entry = Entry {
            value,
            t: Some(Instant::now() + Duration::from_millis(px)),
        };
        self.data.insert(key, entry);
    }

    pub fn get(&mut self, key: &str) -> Value {
        let Some(entry) = self.data.get(key) else {return Value::Null; };

        if let Some(t) = entry.t {
            if t < Instant::now() {
                self.data.remove(key);
                return Value::Null;
            }
        }

        return Value::BulkString(entry.value.clone());
    }
}
