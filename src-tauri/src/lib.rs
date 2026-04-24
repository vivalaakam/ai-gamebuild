mod events;
mod input;
mod model;
mod renderer;
mod rng;
mod runtime;
mod scripting;
mod storage;

use input::RawInput;
use model::Project;
use renderer::FrameView;
use runtime::{Runtime, SaveResult, ValidationResult};
use serde_json::Value;
use std::sync::Mutex;
use tauri::Manager;

struct RuntimeHandle {
    runtime: Mutex<Runtime>,
}

#[tauri::command]
fn load_project(state: tauri::State<'_, RuntimeHandle>) -> Project {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .project()
}

#[tauri::command]
fn run_frame(
    state: tauri::State<'_, RuntimeHandle>,
    pressed_keys: Vec<String>,
    delta: f64,
) -> FrameView {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .frame(RawInput { pressed_keys }, delta)
}

#[tauri::command]
fn update_script(
    state: tauri::State<'_, RuntimeHandle>,
    script_id: String,
    source: String,
) -> Result<Project, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .update_script(script_id, source)
}

#[tauri::command]
fn validate_script(state: tauri::State<'_, RuntimeHandle>, source: String) -> ValidationResult {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .validate_script(source)
}

#[tauri::command]
fn create_script(state: tauri::State<'_, RuntimeHandle>) -> Result<Project, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .create_script()
}

#[tauri::command]
fn create_struct(state: tauri::State<'_, RuntimeHandle>) -> Project {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .create_struct()
}

#[tauri::command]
fn update_struct(
    state: tauri::State<'_, RuntimeHandle>,
    struct_id: String,
    source: String,
) -> Result<Project, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .update_struct(struct_id, source)
}

#[tauri::command]
fn update_input_action(
    state: tauri::State<'_, RuntimeHandle>,
    action_id: String,
    key_code: String,
) -> Result<Project, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .update_input_action(action_id, key_code)
}

#[tauri::command]
fn reset_input_actions(state: tauri::State<'_, RuntimeHandle>) -> Project {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .reset_input_actions()
}

#[tauri::command]
fn emit_event(
    state: tauri::State<'_, RuntimeHandle>,
    name: String,
    payload: Value,
) -> Result<(), String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .emit(name, payload);
    Ok(())
}

#[tauri::command]
fn save_project(state: tauri::State<'_, RuntimeHandle>) -> Result<SaveResult, String> {
    state.runtime.lock().expect("runtime state poisoned").save()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let runtime = Runtime::open(app_dir.join("projects.sqlite"))
                .map_err(|err| Box::<dyn std::error::Error>::from(err))?;
            app.manage(RuntimeHandle {
                runtime: Mutex::new(runtime),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_project,
            run_frame,
            update_script,
            validate_script,
            create_script,
            create_struct,
            update_struct,
            update_input_action,
            reset_input_actions,
            emit_event,
            save_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
