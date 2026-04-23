use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInput {
    pub pressed_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    pub action: String,
    pub pressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputState {
    keymap: BTreeMap<String, String>,
    pressed: BTreeSet<String>,
    previous: BTreeSet<String>,
}

impl Default for InputState {
    fn default() -> Self {
        let mut keymap = BTreeMap::new();
        keymap.insert("ArrowUp".into(), "up".into());
        keymap.insert("ArrowDown".into(), "down".into());
        keymap.insert("ArrowLeft".into(), "left".into());
        keymap.insert("ArrowRight".into(), "right".into());
        keymap.insert("Space".into(), "paint".into());
        keymap.insert("Enter".into(), "spawn".into());

        Self {
            keymap,
            pressed: BTreeSet::new(),
            previous: BTreeSet::new(),
        }
    }
}

impl InputState {
    pub fn update(&mut self, raw: RawInput) -> Vec<InputEvent> {
        self.previous = self.pressed.clone();
        self.pressed = raw
            .pressed_keys
            .iter()
            .filter_map(|key| self.keymap.get(key).cloned())
            .collect();

        let mut events = Vec::new();
        for action in self.pressed.difference(&self.previous) {
            events.push(InputEvent {
                action: action.clone(),
                pressed: true,
            });
        }
        for action in self.previous.difference(&self.pressed) {
            events.push(InputEvent {
                action: action.clone(),
                pressed: false,
            });
        }
        events
    }

    pub fn is_pressed(&self, action: &str) -> bool {
        self.pressed.contains(action)
    }

    pub fn is_just_pressed(&self, action: &str) -> bool {
        self.pressed.contains(action) && !self.previous.contains(action)
    }
}
