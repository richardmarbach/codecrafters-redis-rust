use std::collections::HashMap;

use crate::resp::Value;

pub struct Store {
    data: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Value {
        self.data
            .get(key)
            .map(|v| Value::BulkString(v.into()))
            .unwrap_or(Value::Null)
    }
}
