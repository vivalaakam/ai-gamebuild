use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub payload: Value,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EventDispatcher {
    script_bindings: BTreeMap<String, BTreeSet<String>>,
    queue: VecDeque<Event>,
}

impl EventDispatcher {
    pub fn bind_script(&mut self, event_name: impl Into<String>, script_id: impl Into<String>) {
        self.script_bindings
            .entry(event_name.into())
            .or_default()
            .insert(script_id.into());
    }

    pub fn bindings_for(&self, event_name: &str) -> Vec<String> {
        self.script_bindings
            .get(event_name)
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn emit(&mut self, name: impl Into<String>, payload: Value) {
        self.queue.push_back(Event {
            name: name.into(),
            payload,
        });
    }

    pub fn drain(&mut self) -> Vec<Event> {
        self.queue.drain(..).collect()
    }

    pub fn clear_bindings(&mut self) {
        self.script_bindings.clear();
    }
}
