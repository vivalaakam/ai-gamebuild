use crate::events::{Event, EventDispatcher};
use crate::input::{InputState, RawInput};
use crate::model::{InputAction, Project, ScriptUnit, StructUnit};
use crate::renderer::{build_frame, DrawCommand, FrameView};
use crate::scripting::{validate_project_source, validate_source, ScriptRuntime};
use crate::storage::ProjectStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Runtime {
    project: Arc<Mutex<Project>>,
    input: Arc<Mutex<InputState>>,
    dispatcher: EventDispatcher,
    store: ProjectStore,
    logs: Vec<String>,
    draw_commands: Vec<DrawCommand>,
}

impl Runtime {
    pub fn open(db_path: PathBuf) -> Result<Self, String> {
        let store = ProjectStore::open(db_path)?;
        let mut project = store.load_or_seed()?;
        project.normalize_demo_scripts();
        let input_actions = project.input_actions.clone();
        let mut runtime = Self {
            project: Arc::new(Mutex::new(project)),
            input: Arc::new(Mutex::new(InputState::from_actions(&input_actions))),
            dispatcher: EventDispatcher::default(),
            store,
            logs: Vec::new(),
            draw_commands: Vec::new(),
        };
        runtime.rebuild_bindings();
        runtime.dispatch_now(Event {
            name: "project_load".into(),
            payload: json!({}),
        });
        runtime.dispatch_now(Event {
            name: "init".into(),
            payload: json!({}),
        });
        Ok(runtime)
    }

    pub fn project(&self) -> Project {
        self.project.lock().expect("project state poisoned").clone()
    }

    pub fn frame(&mut self, raw_input: RawInput, delta: f64) -> FrameView {
        let input_events = self
            .input
            .lock()
            .expect("input state poisoned")
            .update(raw_input);
        for event in input_events {
            self.dispatch_now(Event {
                name: "input".into(),
                payload: json!({
                    "action": event.action,
                    "pressed": event.pressed,
                }),
            });
        }

        self.dispatch_now(Event {
            name: "update".into(),
            payload: json!({ "delta": delta }),
        });
        self.dispatch_now(Event {
            name: "render".into(),
            payload: json!({}),
        });
        self.dispatch_queued_events();

        let project = self.project();
        let draw_commands = self.take_draw_commands_from_scripts();
        build_frame(&project, draw_commands, std::mem::take(&mut self.logs))
    }

    pub fn emit(&mut self, name: String, payload: serde_json::Value) {
        self.dispatcher.emit(name, payload);
        self.dispatch_queued_events();
    }

    pub fn update_script(&mut self, script_id: String, source: String) -> Result<Project, String> {
        {
            let project = self.project.lock().expect("project state poisoned");
            validate_project_source(&project, &source)?;
        }
        let mut project = self.project.lock().expect("project state poisoned");
        let Some(script) = project
            .scripts
            .iter_mut()
            .find(|script| script.id == script_id)
        else {
            return Err(format!("unknown script '{script_id}'"));
        };
        script.source = source;
        drop(project);
        self.rebuild_bindings();
        Ok(self.project())
    }

    pub fn validate_script(&self, source: String) -> ValidationResult {
        match validate_source(&source) {
            Ok(()) => ValidationResult {
                valid: true,
                error: None,
            },
            Err(error) => ValidationResult {
                valid: false,
                error: Some(error),
            },
        }
    }

    pub fn create_script(&mut self) -> Result<Project, String> {
        let mut project = self.project.lock().expect("project state poisoned");
        let mut index = project.scripts.len() + 1;
        let id = loop {
            let candidate = format!("script_{index}");
            if !project.scripts.iter().any(|script| script.id == candidate) {
                break candidate;
            }
            index += 1;
        };
        let name = format!("{id}.rhai");
        project.scripts.push(ScriptUnit::new(
            &id,
            name,
            r#"fn name_fn(payload) {
    
}"#,
            ["custom"],
        ));
        drop(project);
        self.rebuild_bindings();
        Ok(self.project())
    }

    pub fn create_struct(&mut self) -> Project {
        let mut project = self.project.lock().expect("project state poisoned");
        let mut index = project.structs.len() + 1;
        let id = loop {
            let candidate = format!("struct_{index}");
            if !project.structs.iter().any(|unit| unit.id == candidate) {
                break candidate;
            }
            index += 1;
        };
        project.structs.push(StructUnit::new(
            &id,
            format!("{id}.rhai"),
            r#"fn make_name(payload) {
    return #{
        
    };
}"#,
        ));
        drop(project);
        self.project()
    }

    pub fn update_struct(&mut self, struct_id: String, source: String) -> Result<Project, String> {
        validate_source(&source)?;
        let mut project = self.project.lock().expect("project state poisoned");
        let Some(unit) = project.structs.iter_mut().find(|unit| unit.id == struct_id) else {
            return Err(format!("unknown struct '{struct_id}'"));
        };
        unit.source = source;
        drop(project);
        Ok(self.project())
    }

    pub fn update_input_action(
        &mut self,
        action_id: String,
        key_code: String,
    ) -> Result<Project, String> {
        let mut project = self.project.lock().expect("project state poisoned");
        let Some(action) = project
            .input_actions
            .iter_mut()
            .find(|action| action.id == action_id)
        else {
            return Err(format!("unknown input action '{action_id}'"));
        };
        action.key_code = key_code;
        let actions = project.input_actions.clone();
        drop(project);
        self.input
            .lock()
            .expect("input state poisoned")
            .set_actions(&actions);
        Ok(self.project())
    }

    pub fn reset_input_actions(&mut self) -> Project {
        let mut project = self.project.lock().expect("project state poisoned");
        project.input_actions = InputAction::defaults();
        let actions = project.input_actions.clone();
        drop(project);
        self.input
            .lock()
            .expect("input state poisoned")
            .set_actions(&actions);
        self.project()
    }

    pub fn save(&self) -> Result<SaveResult, String> {
        let project = self.project.lock().expect("project state poisoned");
        let snapshot = self.store.save_snapshot(&project)?;
        Ok(SaveResult {
            snapshot_bytes: snapshot.len(),
        })
    }

    fn rebuild_bindings(&mut self) {
        self.dispatcher.clear_bindings();
        let project = self.project.lock().expect("project state poisoned");
        for script in &project.scripts {
            for binding in &script.bindings {
                self.dispatcher.bind_script(binding, &script.id);
            }
        }
    }

    fn dispatch_now(&mut self, event: Event) {
        let script_ids = self.dispatcher.bindings_for(&event.name);
        let runtime = ScriptRuntime::new(self.project.clone(), self.input.clone());
        let result = runtime.dispatch(&event, script_ids);
        for event in result.emitted_events {
            self.dispatcher.emit(event.name, event.payload);
        }
        self.logs.extend(result.logs);
        self.draw_commands.extend(result.draw_commands);
    }

    fn dispatch_queued_events(&mut self) {
        let mut guard = 0;
        while guard < 64 {
            guard += 1;
            let events = self.dispatcher.drain();
            if events.is_empty() {
                break;
            }
            for event in events {
                self.dispatch_now(event);
            }
        }
        if guard >= 64 {
            self.logs.push("event queue limit reached".into());
        }
    }

    fn take_draw_commands_from_scripts(&mut self) -> Vec<DrawCommand> {
        std::mem::take(&mut self.draw_commands)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveResult {
    pub snapshot_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
}
