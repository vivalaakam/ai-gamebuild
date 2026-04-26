use crate::input::RawInput;
use crate::model::{ScriptUnit, StructUnit};
use crate::renderer::FrameView;
use crate::runtime::Runtime;
use crate::scripting::validate_source;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

// ─── JSON-RPC types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success<T: Serialize>(id: Option<Value>, result: T) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// ─── MCP types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    capabilities: Value,
    #[serde(rename = "serverInfo")]
    server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Clone, Serialize)]
struct McpToolDescription {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct McpToolsResponse {
    tools: Vec<McpToolDescription>,
}

#[derive(Debug, Clone, Serialize)]
struct McpCallResponse {
    content: Vec<McpTextContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}

// ─── Domain response types ──────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct ProjectOverview {
    id: String,
    name: String,
    scripts: Vec<ScriptSummary>,
    structs: Vec<StructSummary>,
    input_actions: Vec<InputActionSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ScriptSummary {
    id: String,
    name: String,
    bindings: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StructSummary {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct InputActionSummary {
    id: String,
    label: String,
    key_code: String,
}

#[derive(Debug, Clone, Serialize)]
struct BuildResult {
    valid: bool,
    scripts_count: usize,
    structs_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SuccessResponse {
    success: bool,
}

// ─── Handler ────────────────────────────────────────────────────

pub struct JsonRpcHandler {
    runtime: Arc<Mutex<Runtime>>,
    initialized: bool,
}

impl JsonRpcHandler {
    pub fn new(runtime: Arc<Mutex<Runtime>>) -> Self {
        Self { runtime, initialized: false }
    }

    pub fn handle_request(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize"   => self.handle_initialize(req),
            "initialized"  => JsonRpcResponse::success(req.id, json!({})),
            "tools/list"   => self.handle_tools_list(req),
            "tools/call"   => self.handle_tools_call(req),

            "get_project"   => self.direct(req, tool_get_project),
            "list_scripts"  => self.direct(req, tool_list_scripts),
            "get_script"    => self.direct_with(req, tool_get_script),
            "update_script" => self.direct_mut(req, tool_update_script),
            "list_structs"  => self.direct(req, tool_list_structs),
            "get_struct"    => self.direct_with(req, tool_get_struct),
            "update_struct" => self.direct_mut(req, tool_update_struct),
            "build_code"    => self.direct(req, tool_build_code),
            "run_frame"     => self.direct_mut(req, tool_run_frame),

            _ => JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method)),
        }
    }

    // ─── Runtime access ─────────────────────────────────────────

    fn with_runtime<F, T>(&self, f: F) -> T
    where F: FnOnce(&Runtime) -> T,
    {
        let rt = self.runtime.lock().expect("runtime poisoned");
        f(&rt)
    }

    fn with_runtime_mut<F, T>(&self, f: F) -> T
    where F: FnOnce(&mut Runtime) -> T,
    {
        let mut rt = self.runtime.lock().expect("runtime poisoned");
        f(&mut rt)
    }

    // ─── MCP protocol ──────────────────────────────────────────

    fn handle_initialize(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;
        JsonRpcResponse::success(req.id, InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: json!({ "tools": {} }),
            server_info: ServerInfo {
                name: "ai-rpg-jsonrpc".into(),
                version: "0.1.0".into(),
            },
        })
    }

    fn handle_tools_list(&self, _req: JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse::success(_req.id, McpToolsResponse {
            tools: vec![
                McpToolDescription {
                    name: "get_project".into(),
                    description: "Get current project info including scripts and structs list".into(),
                    input_schema: Some(json!({ "type": "object", "properties": {} })),
                },
                McpToolDescription {
                    name: "list_scripts".into(),
                    description: "List all scripts with full source".into(),
                    input_schema: Some(json!({ "type": "object", "properties": {} })),
                },
                McpToolDescription {
                    name: "get_script".into(),
                    description: "Get a single script by id or name".into(),
                    input_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Script id" },
                            "name": { "type": "string", "description": "Script name" }
                        }
                    })),
                },
                McpToolDescription {
                    name: "update_script".into(),
                    description: "Update a script source by id".into(),
                    input_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "source": { "type": "string" }
                        },
                        "required": ["id", "source"]
                    })),
                },
                McpToolDescription {
                    name: "list_structs".into(),
                    description: "List all structs with full source".into(),
                    input_schema: Some(json!({ "type": "object", "properties": {} })),
                },
                McpToolDescription {
                    name: "get_struct".into(),
                    description: "Get a single struct by id or name".into(),
                    input_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Struct id" },
                            "name": { "type": "string", "description": "Struct name" }
                        }
                    })),
                },
                McpToolDescription {
                    name: "update_struct".into(),
                    description: "Update a struct source by id".into(),
                    input_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "source": { "type": "string" }
                        },
                        "required": ["id", "source"]
                    })),
                },
                McpToolDescription {
                    name: "build_code".into(),
                    description: "Validate and build the entire project code".into(),
                    input_schema: Some(json!({ "type": "object", "properties": {} })),
                },
                McpToolDescription {
                    name: "run_frame".into(),
                    description: "Run one game frame and return render data".into(),
                    input_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "pressed_keys": { "type": "array", "items": { "type": "string" } },
                            "delta": { "type": "number" }
                        }
                    })),
                },
            ],
        })
    }

    fn handle_tools_call(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params = match &req.params {
            Some(p) => p.clone(),
            None => return JsonRpcResponse::error(req.id.clone(), -32602, "Missing params"),
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return JsonRpcResponse::error(req.id.clone(), -32602, "Missing tool name"),
        };

        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        // Match each tool, call it inside the runtime lock, and wrap in MCP format
        match name.as_str() {
            "get_project" => self.tool_call(req.id, args, tool_get_project),
            "list_scripts" => self.tool_call(req.id, args, tool_list_scripts),
            "get_script" => self.tool_call_with(req.id, args, tool_get_script),
            "update_script" => self.tool_call_mut_with(req.id, args, tool_update_script),
            "list_structs" => self.tool_call(req.id, args, tool_list_structs),
            "get_struct" => self.tool_call_with(req.id, args, tool_get_struct),
            "update_struct" => self.tool_call_mut_with(req.id, args, tool_update_struct),
            "build_code" => self.tool_call(req.id, args, tool_build_code),
            "run_frame" => self.tool_call_mut_with(req.id, args, tool_run_frame),
            other => JsonRpcResponse::error(req.id, -32602, format!("Unknown tool: {}", other)),
        }
    }

    // ─── Direct dispatch ────────────────────────────────────────

    fn direct<T: Serialize>(&self, req: JsonRpcRequest, f: impl FnOnce(&Runtime) -> Result<T, String>) -> JsonRpcResponse {
        match self.with_runtime(f) {
            Ok(v) => JsonRpcResponse::success(req.id, v),
            Err(m) => JsonRpcResponse::error(req.id, -32603, m),
        }
    }

    fn direct_with<T: Serialize>(&self, req: JsonRpcRequest, f: impl FnOnce(&Runtime, Value) -> Result<T, String>) -> JsonRpcResponse {
        let args = req.params.clone().unwrap_or(json!({}));
        match self.with_runtime(|rt| f(rt, args)) {
            Ok(v) => JsonRpcResponse::success(req.id, v),
            Err(m) => JsonRpcResponse::error(req.id, -32603, m),
        }
    }

    fn direct_mut<T: Serialize>(&self, req: JsonRpcRequest, f: impl FnOnce(&mut Runtime, Value) -> Result<T, String>) -> JsonRpcResponse {
        let args = req.params.clone().unwrap_or(json!({}));
        match self.with_runtime_mut(|rt| f(rt, args)) {
            Ok(v) => JsonRpcResponse::success(req.id, v),
            Err(m) => JsonRpcResponse::error(req.id, -32603, m),
        }
    }

    // ─── MCP tools/call dispatch (wraps result in McpCallResponse) ───

    fn tool_call<T: Serialize>(&self, id: Option<Value>, _args: Value, f: impl FnOnce(&Runtime) -> Result<T, String>) -> JsonRpcResponse {
        let result = self.with_runtime(|rt| {
            f(rt).map(|v| {
                let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into());
                McpCallResponse {
                    content: vec![McpTextContent { content_type: "text".into(), text }],
                    is_error: None,
                }
            })
        });
        match result {
            Ok(resp) => JsonRpcResponse::success(id, resp),
            Err(msg) => JsonRpcResponse::success(id, McpCallResponse {
                content: vec![McpTextContent { content_type: "text".into(), text: msg }],
                is_error: Some(true),
            }),
        }
    }

    fn tool_call_with<T: Serialize>(&self, id: Option<Value>, args: Value, f: impl FnOnce(&Runtime, Value) -> Result<T, String>) -> JsonRpcResponse {
        let result = self.with_runtime(|rt| {
            f(rt, args).map(|v| {
                let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into());
                McpCallResponse {
                    content: vec![McpTextContent { content_type: "text".into(), text }],
                    is_error: None,
                }
            })
        });
        match result {
            Ok(resp) => JsonRpcResponse::success(id, resp),
            Err(msg) => JsonRpcResponse::success(id, McpCallResponse {
                content: vec![McpTextContent { content_type: "text".into(), text: msg }],
                is_error: Some(true),
            }),
        }
    }

    fn tool_call_mut_with<T: Serialize>(&self, id: Option<Value>, args: Value, f: impl FnOnce(&mut Runtime, Value) -> Result<T, String>) -> JsonRpcResponse {
        let result = self.with_runtime_mut(|rt| {
            f(rt, args).map(|v| {
                let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into());
                McpCallResponse {
                    content: vec![McpTextContent { content_type: "text".into(), text }],
                    is_error: None,
                }
            })
        });
        match result {
            Ok(resp) => JsonRpcResponse::success(id, resp),
            Err(msg) => JsonRpcResponse::success(id, McpCallResponse {
                content: vec![McpTextContent { content_type: "text".into(), text: msg }],
                is_error: Some(true),
            }),
        }
    }
}

// ─── Tool functions ─────────────────────────────────────────────

fn tool_get_project(runtime: &Runtime) -> Result<ProjectOverview, String> {
    let p = runtime.project();
    Ok(ProjectOverview {
        id: p.id,
        name: p.name,
        scripts: p.scripts.into_iter().map(|s| ScriptSummary {
            id: s.id, name: s.name, bindings: s.bindings,
        }).collect(),
        structs: p.structs.into_iter().map(|s| StructSummary {
            id: s.id, name: s.name,
        }).collect(),
        input_actions: p.input_actions.into_iter().map(|a| InputActionSummary {
            id: a.id, label: a.label, key_code: a.key_code,
        }).collect(),
    })
}

fn tool_list_scripts(runtime: &Runtime) -> Result<Vec<ScriptUnit>, String> {
    Ok(runtime.project().scripts)
}

fn tool_get_script(runtime: &Runtime, args: Value) -> Result<ScriptUnit, String> {
    let project = runtime.project();
    let id = args.get("id").and_then(|v| v.as_str());
    let name = args.get("name").and_then(|v| v.as_str());
    project.scripts.into_iter()
        .find(|s| id.map_or(false, |id| s.id == id) || name.map_or(false, |n| s.name == n))
        .ok_or_else(|| "Script not found".into())
}

fn tool_update_script(runtime: &mut Runtime, args: Value) -> Result<SuccessResponse, String> {
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?.to_string();
    let source = args.get("source").and_then(|v| v.as_str()).ok_or("Missing source")?.to_string();
    runtime.update_script(id, source)?;
    Ok(SuccessResponse { success: true })
}

fn tool_list_structs(runtime: &Runtime) -> Result<Vec<StructUnit>, String> {
    Ok(runtime.project().structs)
}

fn tool_get_struct(runtime: &Runtime, args: Value) -> Result<StructUnit, String> {
    let project = runtime.project();
    let id = args.get("id").and_then(|v| v.as_str());
    let name = args.get("name").and_then(|v| v.as_str());
    project.structs.into_iter()
        .find(|s| id.map_or(false, |id| s.id == id) || name.map_or(false, |n| s.name == n))
        .ok_or_else(|| "Struct not found".into())
}

fn tool_update_struct(runtime: &mut Runtime, args: Value) -> Result<SuccessResponse, String> {
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?.to_string();
    let source = args.get("source").and_then(|v| v.as_str()).ok_or("Missing source")?.to_string();
    runtime.update_struct(id, source)?;
    Ok(SuccessResponse { success: true })
}

fn tool_build_code(runtime: &Runtime) -> Result<BuildResult, String> {
    let p = runtime.project();
    let mut full_source = String::new();
    for unit in &p.structs {
        full_source.push_str(&unit.source);
        full_source.push('\n');
    }
    for script in &p.scripts {
        full_source.push_str(&script.source);
        full_source.push('\n');
    }
    match validate_source(&full_source) {
        Ok(()) => Ok(BuildResult {
            valid: true,
            scripts_count: p.scripts.len(),
            structs_count: p.structs.len(),
            error: None,
        }),
        Err(error) => Ok(BuildResult {
            valid: false,
            scripts_count: p.scripts.len(),
            structs_count: p.structs.len(),
            error: Some(error),
        }),
    }
}

fn tool_run_frame(runtime: &mut Runtime, args: Value) -> Result<FrameView, String> {
    let pressed_keys = args
        .get("pressed_keys")
        .or_else(|| args.get("pressedKeys"))
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|entry| entry.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let delta = args.get("delta").and_then(|value| value.as_f64()).unwrap_or(0.016);
    Ok(runtime.frame(RawInput { pressed_keys }, delta))
}

// ─── Axum state & handler ───────────────────────────────────────

#[derive(Clone)]
pub struct AxumState {
    pub handler: Arc<Mutex<JsonRpcHandler>>,
}

pub async fn jsonrpc_handler(
    State(state): State<AxumState>,
    Json(body): Json<Value>,
) -> Json<JsonRpcResponse> {
    let request: JsonRpcRequest = match serde_json::from_value(body) {
        Ok(req) => req,
        Err(e) => return Json(JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e))),
    };

    let response = {
        let mut handler = state.handler.lock().expect("jsonrpc handler poisoned");
        handler.handle_request(request)
    };

    Json(response)
}

pub fn router(handler: Arc<Mutex<JsonRpcHandler>>) -> Router {
    let state = AxumState { handler };
    Router::new()
        .route("/jsonrpc", post(jsonrpc_handler))
        .with_state(state)
}
