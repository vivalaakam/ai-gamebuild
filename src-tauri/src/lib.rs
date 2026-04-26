mod events;
mod input;
pub mod jsonrpc;
pub mod mcp;
pub mod model;
mod renderer;
mod rng;
mod runtime;
pub mod scripting;
pub mod storage;

use input::RawInput;
use model::Project;
use renderer::FrameView;
use runtime::{ProjectInfo, Runtime, SaveResult, ValidationResult};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_cli::CliExt;

struct RuntimeHandle {
    runtime: Arc<Mutex<Runtime>>,
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

#[tauri::command]
fn list_projects(state: tauri::State<'_, RuntimeHandle>) -> Result<Vec<ProjectInfo>, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .list_projects()
}

#[tauri::command]
fn switch_project(
    state: tauri::State<'_, RuntimeHandle>,
    project_id: String,
) -> Result<Project, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .switch_project(project_id)
}

#[tauri::command]
fn create_project_cmd(
    state: tauri::State<'_, RuntimeHandle>,
    name: String,
) -> Result<Project, String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .create_project(name)
}

#[tauri::command]
fn delete_project(
    state: tauri::State<'_, RuntimeHandle>,
    project_id: String,
) -> Result<(), String> {
    state
        .runtime
        .lock()
        .expect("runtime state poisoned")
        .delete_project(project_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let default_project_id = args.get(1).cloned();

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_cli::init())
        .setup(move |app| {
            // Check for MCP subcommand before initializing GUI
            let matches = app.cli().matches()?;
            if let Some(subcommand) = matches.subcommand {
                if subcommand.name == "mcp" {
                    let db_path = match subcommand.matches.args.get("db-path") {
                        Some(data) => match &data.value {
                            serde_json::Value::String(s) => std::path::PathBuf::from(s),
                            _ => app.path().app_data_dir()?.join("projects.sqlite"),
                        },
                        None => app.path().app_data_dir()?.join("projects.sqlite"),
                    };
                    let project_id = match subcommand.matches.args.get("project-id") {
                        Some(data) => match &data.value {
                            serde_json::Value::String(s) => s.clone(),
                            _ => {
                                eprintln!("Missing --project-id");
                                std::process::exit(1);
                            }
                        },
                        None => {
                            eprintln!("Missing --project-id");
                            std::process::exit(1);
                        }
                    };

                    let mut server = mcp::McpServer::open(db_path, project_id)
                        .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
                    server
                        .run()
                        .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
                    std::process::exit(0);
                }
            }

            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let runtime =
                Runtime::open(app_dir.join("projects.sqlite"), default_project_id.clone())
                    .map_err(|err| Box::<dyn std::error::Error>::from(err))?;

            // Share the same runtime between GUI and JSON-RPC
            let runtime = Arc::new(Mutex::new(runtime));

            app.manage(RuntimeHandle {
                runtime: runtime.clone(),
            });

            // Start JSON-RPC server via axum on localhost:3001
            // Uses the same application runtime state as the GUI
            tauri::async_runtime::spawn(async move {
                let handler = jsonrpc::JsonRpcHandler::new(runtime);
                let handler = Arc::new(Mutex::new(handler));
                let router = jsonrpc::router(handler);

                match tokio::net::TcpListener::bind("0.0.0.0:3001").await {
                    Ok(listener) => {
                        eprintln!("JSON-RPC server listening on http://0.0.0.0:3001/jsonrpc");
                        axum::serve(listener, router).await.ok();
                    }
                    Err(e) => {
                        eprintln!("Failed to bind JSON-RPC server: {}", e);
                    }
                }
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
            save_project,
            list_projects,
            switch_project,
            create_project_cmd,
            delete_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
