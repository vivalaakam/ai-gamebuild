use crate::events::Event;
use crate::input::InputState;
use crate::model::{Entity, EntityFlags, Project, RenderComponent, ScriptUnit, Transform};
use crate::renderer::DrawCommand;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Scope};
use serde_json::{json, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ScriptHost {
    project: Arc<Mutex<Project>>,
    input: Arc<Mutex<InputState>>,
    outbox: Arc<Mutex<Vec<Event>>>,
    draw_commands: Arc<Mutex<Vec<DrawCommand>>>,
    logs: Arc<Mutex<Vec<String>>>,
}

impl ScriptHost {
    pub fn new(project: Arc<Mutex<Project>>, input: Arc<Mutex<InputState>>) -> Self {
        Self {
            project,
            input,
            outbox: Arc::new(Mutex::new(Vec::new())),
            draw_commands: Arc::new(Mutex::new(Vec::new())),
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn take_events(&self) -> Vec<Event> {
        self.outbox
            .lock()
            .expect("script outbox poisoned")
            .drain(..)
            .collect()
    }

    pub fn take_draw_commands(&self) -> Vec<DrawCommand> {
        self.draw_commands
            .lock()
            .expect("draw command queue poisoned")
            .drain(..)
            .collect()
    }

    pub fn take_logs(&self) -> Vec<String> {
        self.logs
            .lock()
            .expect("script log queue poisoned")
            .drain(..)
            .collect()
    }
}

pub struct ScriptRuntime {
    project: Arc<Mutex<Project>>,
    input: Arc<Mutex<InputState>>,
}

impl ScriptRuntime {
    pub fn new(project: Arc<Mutex<Project>>, input: Arc<Mutex<InputState>>) -> Self {
        Self { project, input }
    }

    pub fn dispatch(&self, event: &Event, script_ids: Vec<String>) -> ScriptDispatchResult {
        let host = ScriptHost::new(self.project.clone(), self.input.clone());
        let engine = build_engine(host.clone());
        let project = self.project.lock().expect("project state poisoned").clone();
        let scripts: BTreeMap<_, _> = project
            .scripts
            .iter()
            .map(|script| (script.id.clone(), script.clone()))
            .collect();

        let mut errors = Vec::new();
        for script_id in script_ids {
            let Some(source) = source_with_dependencies(&script_id, &project, &scripts) else {
                errors.push(format!("missing script '{script_id}'"));
                continue;
            };

            let ast = match engine.compile(source) {
                Ok(ast) => ast,
                Err(err) => {
                    errors.push(format!("compile error in {script_id}: {err}"));
                    continue;
                }
            };

            let fn_name = format!("on_{}", event.name);
            let payload = json_to_dynamic(&event.payload);
            let mut scope = Scope::new();
            let result: Result<Dynamic, Box<EvalAltResult>> =
                engine.call_fn(&mut scope, &ast, &fn_name, (payload,));
            if let Err(err) = result {
                if !err.to_string().contains("Function not found") {
                    errors.push(format!("script error in {script_id}.{fn_name}: {err}"));
                }
            }
        }

        ScriptDispatchResult {
            emitted_events: host.take_events(),
            draw_commands: host.take_draw_commands(),
            logs: host
                .take_logs()
                .into_iter()
                .chain(errors.into_iter())
                .collect(),
        }
    }
}

pub struct ScriptDispatchResult {
    pub emitted_events: Vec<Event>,
    pub draw_commands: Vec<DrawCommand>,
    pub logs: Vec<String>,
}

pub fn validate_source(source: &str) -> Result<(), String> {
    Engine::new()
        .compile(source)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub fn validate_project_source(project: &Project, source: &str) -> Result<(), String> {
    let mut full_source = shared_source(project);
    full_source.push_str(source);
    Engine::new()
        .compile(full_source)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn build_engine(host: ScriptHost) -> Engine {
    let mut engine = Engine::new();

    let h = host.clone();
    engine.register_fn("clear", move || {
        h.draw_commands
            .lock()
            .expect("draw command queue poisoned")
            .push(DrawCommand::Clear);
    });

    let h = host.clone();
    engine.register_fn("draw_tile", move |tile_id: i64, x: i64, y: i64| {
        h.draw_commands
            .lock()
            .expect("draw command queue poisoned")
            .push(DrawCommand::Tile {
                tile_id: tile_id.max(0) as u32,
                x: x as i32,
                y: y as i32,
            });
    });

    let h = host.clone();
    engine.register_fn("draw_entity", move |entity_id: i64| {
        h.draw_commands
            .lock()
            .expect("draw command queue poisoned")
            .push(DrawCommand::Entity {
                entity_id: entity_id.max(0) as u64,
            });
    });

    let h = host.clone();
    engine.register_fn("get_tile", move |x: i64, y: i64| -> i64 {
        h.project
            .lock()
            .expect("project state poisoned")
            .world
            .tilemap
            .get(x, y)
            .unwrap_or(0) as i64
    });

    let h = host.clone();
    engine.register_fn("set_tile", move |x: i64, y: i64, tile_id: i64| -> bool {
        h.project
            .lock()
            .expect("project state poisoned")
            .world
            .tilemap
            .set(x, y, tile_id.max(0) as u32)
    });

    let h = host.clone();
    engine.register_fn(
        "entity_spawn_raw",
        move |kind: String, x: i64, y: i64, tile_id: i64, blocking: bool| -> i64 {
            let mut project = h.project.lock().expect("project state poisoned");
            let id = project.world.next_entity_id;
            project.world.next_entity_id += 1;
            project.world.entities.insert(
                id,
                Entity {
                    id,
                    transform: Transform {
                        x: x as i32,
                        y: y as i32,
                    },
                    render: RenderComponent {
                        tile_id: tile_id.max(0) as u32,
                    },
                    flags: EntityFlags {
                        visible: true,
                        blocking,
                    },
                    script: None,
                    state: json!({ "kind": kind }),
                },
            );
            id as i64
        },
    );

    let h = host.clone();
    engine.register_fn("remove_entity", move |id: i64| -> bool {
        h.project
            .lock()
            .expect("project state poisoned")
            .world
            .entities
            .remove(&(id.max(0) as u64))
            .is_some()
    });

    let h = host.clone();
    engine.register_fn(
        "entity_set_pos_raw",
        move |id: i64, x: i64, y: i64| -> bool {
            let mut project = h.project.lock().expect("project state poisoned");
            let Some(entity) = project.world.entities.get_mut(&(id.max(0) as u64)) else {
                return false;
            };
            entity.transform = Transform {
                x: x as i32,
                y: y as i32,
            };
            true
        },
    );

    let h = host.clone();
    engine.register_fn("entity_next_id", move |current_id: i64| -> i64 {
        let project = h.project.lock().expect("project state poisoned");
        let current_id = current_id.max(0) as u64;
        project
            .world
            .entities
            .keys()
            .copied()
            .find(|id| *id > current_id)
            .or_else(|| project.world.entities.keys().next().copied())
            .unwrap_or(0) as i64
    });

    let h = host.clone();
    engine.register_fn("state_get", move |key: String| -> Dynamic {
        let project = h.project.lock().expect("project state poisoned");
        project
            .runtime_state
            .get(&key)
            .map(json_to_dynamic)
            .unwrap_or(Dynamic::UNIT)
    });

    let h = host.clone();
    engine.register_fn("state_set", move |key: String, value: Dynamic| {
        let mut project = h.project.lock().expect("project state poisoned");
        if !project.runtime_state.is_object() {
            project.runtime_state = json!({});
        }
        if let Some(state) = project.runtime_state.as_object_mut() {
            state.insert(key, dynamic_to_json(&value));
        }
    });

    let h = host.clone();
    engine.register_fn("state_remove", move |key: String| -> bool {
        let mut project = h.project.lock().expect("project state poisoned");
        project
            .runtime_state
            .as_object_mut()
            .and_then(|state| state.remove(&key))
            .is_some()
    });

    let h = host.clone();
    engine.register_fn("entity_ids", move || -> Array {
        h.project
            .lock()
            .expect("project state poisoned")
            .world
            .entities
            .keys()
            .map(|id| Dynamic::from(*id as i64))
            .collect()
    });

    let h = host.clone();
    engine.register_fn("get_entity", move |id: i64| -> Dynamic {
        let project = h.project.lock().expect("project state poisoned");
        project
            .world
            .entities
            .get(&(id.max(0) as u64))
            .map(entity_to_map)
            .map(Dynamic::from_map)
            .unwrap_or(Dynamic::UNIT)
    });

    let h = host.clone();
    engine.register_fn("emit", move |name: String, payload: Dynamic| {
        h.outbox
            .lock()
            .expect("script outbox poisoned")
            .push(Event {
                name,
                payload: dynamic_to_json(&payload),
            });
    });

    let h = host.clone();
    engine.register_fn(
        "is_pressed",
        move |payload: Dynamic, action: String| -> bool {
            if let Some(pressed) = payload_action_pressed(&payload, &action) {
                return pressed;
            }
            h.input
                .lock()
                .expect("input state poisoned")
                .is_pressed(&action)
        },
    );

    let h = host.clone();
    engine.register_fn(
        "is_just_pressed",
        move |payload: Dynamic, action: String| -> bool {
            if let Some(pressed) = payload_action_pressed(&payload, &action) {
                return pressed;
            }
            h.input
                .lock()
                .expect("input state poisoned")
                .is_just_pressed(&action)
        },
    );

    let h = host.clone();
    engine.register_fn("log", move |message: String| {
        h.logs
            .lock()
            .expect("script log queue poisoned")
            .push(message);
    });

    let h = host.clone();
    engine.register_fn("log", move |message: String, payload: Dynamic| {
        let value = dynamic_to_json(&payload);
        let suffix = match value {
            Value::Null => String::new(),
            _ => serde_json::to_string(&value).unwrap_or_else(|_| "<invalid>".into()),
        };
        let output = if suffix.is_empty() {
            message
        } else {
            format!("{message} {suffix}")
        };
        h.logs
            .lock()
            .expect("script log queue poisoned")
            .push(output);
    });

    engine
}

fn source_with_dependencies(
    script_id: &str,
    project: &Project,
    scripts: &BTreeMap<String, crate::model::ScriptUnit>,
) -> Option<String> {
    fn visit(
        id: &str,
        scripts: &BTreeMap<String, crate::model::ScriptUnit>,
        seen: &mut BTreeSet<String>,
        output: &mut String,
    ) -> Option<()> {
        if !seen.insert(id.to_string()) {
            return Some(());
        }
        let script = scripts.get(id)?;
        for dependency in &script.dependencies {
            visit(dependency, scripts, seen, output)?;
        }
        output.push_str(&script.source);
        output.push('\n');
        Some(())
    }

    let mut seen = BTreeSet::new();
    let mut output = shared_source(project);
    for script in project
        .scripts
        .iter()
        .filter(|script| script.bindings.is_empty())
    {
        visit(&script.id, scripts, &mut seen, &mut output)?;
    }
    visit(script_id, scripts, &mut seen, &mut output)?;
    Some(output)
}

fn shared_source(project: &Project) -> String {
    let mut output = String::new();
    for unit in &project.structs {
        output.push_str(&unit.source);
        output.push('\n');
    }
    for builtin in ScriptUnit::builtin_libraries() {
        output.push_str(&builtin.source);
        output.push('\n');
    }
    output
}

fn entity_to_map(entity: &Entity) -> Map {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(entity.id as i64));
    map.insert("x".into(), Dynamic::from(entity.transform.x as i64));
    map.insert("y".into(), Dynamic::from(entity.transform.y as i64));
    map.insert(
        "tile_id".into(),
        Dynamic::from(entity.render.tile_id as i64),
    );
    map.insert("visible".into(), Dynamic::from(entity.flags.visible));
    map.insert("blocking".into(), Dynamic::from(entity.flags.blocking));
    map
}

fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(value) => Dynamic::from(*value),
        Value::Number(value) => Dynamic::from(value.as_f64().unwrap_or_default()),
        Value::String(value) => Dynamic::from(value.clone()),
        Value::Array(values) => Dynamic::from_array(values.iter().map(json_to_dynamic).collect()),
        Value::Object(values) => {
            let mut map = Map::new();
            for (key, value) in values {
                map.insert(key.clone().into(), json_to_dynamic(value));
            }
            Dynamic::from_map(map)
        }
    }
}

fn payload_action_pressed(payload: &Dynamic, action: &str) -> Option<bool> {
    let json = dynamic_to_json(payload);
    let object = json.as_object()?;
    let payload_action = object.get("action")?.as_str()?;
    if payload_action != action {
        return None;
    }
    object.get("pressed").and_then(Value::as_bool)
}

fn dynamic_to_json(value: &Dynamic) -> Value {
    if value.is_unit() {
        return Value::Null;
    }
    if let Some(value) = value.clone().try_cast::<bool>() {
        return Value::Bool(value);
    }
    if let Some(value) = value.clone().try_cast::<i64>() {
        return Value::Number(Number::from(value));
    }
    if let Some(value) = value.clone().try_cast::<f64>() {
        return Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Some(value) = value.clone().try_cast::<String>() {
        return Value::String(value);
    }
    if let Some(values) = value.clone().try_cast::<Array>() {
        return Value::Array(values.iter().map(dynamic_to_json).collect());
    }
    if let Some(values) = value.clone().try_cast::<Map>() {
        let object = values
            .iter()
            .map(|(key, value)| (key.to_string(), dynamic_to_json(value)))
            .collect();
        return Value::Object(object);
    }
    Value::String(value.type_name().to_string())
}
