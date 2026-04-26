use crate::runtime::Runtime;
use crate::scripting::validate_source;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
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
struct McpTool {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

// ─── Handler ────────────────────────────────────────────────────

pub struct JsonRpcHandler {
    runtime: Arc<Mutex<Runtime>>,
    initialized: bool,
}

impl JsonRpcHandler {
    pub fn new(runtime: Arc<Mutex<Runtime>>) -> Self {
        Self {
            runtime,
            initialized: false,
        }
    }

    pub fn handle_request(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            // MCP protocol methods
            "initialize" => self.handle_initialize(req),
            "initialized" => JsonRpcResponse::success(req.id, json!({})),
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req),

            // Direct project methods (for non-MCP JSON-RPC clients)
            "get_project" => self.read_ok(req, tool_get_project),
            "list_scripts" => self.read_ok(req, tool_list_scripts),
            "get_script" => self.read_ok_with(req, tool_get_script),
            "update_script" => self.write_ok(req, tool_update_script),
            "list_structs" => self.read_ok(req, tool_list_structs),
            "get_struct" => self.read_ok_with(req, tool_get_struct),
            "update_struct" => self.write_ok(req, tool_update_struct),
            "build_code" => self.read_ok(req, tool_build_code),

            _ => {
                JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method))
            }
        }
    }

    // ─── Runtime access helpers ─────────────────────────────────

    fn with_runtime<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&Runtime) -> T,
    {
        let rt = self.runtime.lock().expect("runtime poisoned");
        f(&rt)
    }

    fn with_runtime_mut<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut Runtime) -> T,
    {
        let mut rt = self.runtime.lock().expect("runtime poisoned");
        f(&mut rt)
    }

    // ─── MCP handlers ──────────────────────────────────────────

    fn handle_initialize(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;
        JsonRpcResponse::success(
            req.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "ai-rpg-jsonrpc", "version": "0.1.0" }
            }),
        )
    }

    fn handle_tools_list(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let tools = vec![
            McpTool {
                name: "get_project".into(),
                description: "Get current project info including scripts and structs list".into(),
                input_schema: Some(json!({ "type": "object", "properties": {} })),
            },
            McpTool {
                name: "list_scripts".into(),
                description: "List all scripts in the project with full source".into(),
                input_schema: Some(json!({ "type": "object", "properties": {} })),
            },
            McpTool {
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
            McpTool {
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
            McpTool {
                name: "list_structs".into(),
                description: "List all structs with full source".into(),
                input_schema: Some(json!({ "type": "object", "properties": {} })),
            },
            McpTool {
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
            McpTool {
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
            McpTool {
                name: "build_code".into(),
                description: "Validate and build the entire project code".into(),
                input_schema: Some(json!({ "type": "object", "properties": {} })),
            },
        ];
        JsonRpcResponse::success(req.id, json!({ "tools": tools }))
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

        let result: Result<Vec<McpTextContent>, String> = match name.as_str() {
            "get_project" => self.with_runtime(tool_get_project),
            "list_scripts" => self.with_runtime(tool_list_scripts),
            "get_script" => self.read_with(args, tool_get_script),
            "update_script" => self.write_with(args, tool_update_script),
            "list_structs" => self.with_runtime(tool_list_structs),
            "get_struct" => self.read_with(args, tool_get_struct),
            "update_struct" => self.write_with(args, tool_update_struct),
            "build_code" => self.with_runtime(tool_build_code),
            other => {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    -32602,
                    format!("Unknown tool: {}", other),
                )
            }
        };

        match result {
            Ok(content) => JsonRpcResponse::success(req.id, json!({ "content": content })),
            Err(msg) => JsonRpcResponse::success(
                req.id,
                json!({
                    "content": [{"type": "text", "text": msg}],
                    "isError": true
                }),
            ),
        }
    }

    // ─── Dispatch helpers ──────────────────────────────────────

    fn read_ok<F>(&self, req: JsonRpcRequest, f: F) -> JsonRpcResponse
    where
        F: FnOnce(&Runtime) -> Result<Vec<McpTextContent>, String>,
    {
        match self.with_runtime(f) {
            Ok(content) => JsonRpcResponse::success(req.id, json!({ "content": content })),
            Err(msg) => JsonRpcResponse::error(req.id, -32603, msg),
        }
    }

    fn read_ok_with<F>(&self, req: JsonRpcRequest, f: F) -> JsonRpcResponse
    where
        F: FnOnce(&Runtime, Value) -> Result<Vec<McpTextContent>, String>,
    {
        let args = req.params.clone().unwrap_or(json!({}));
        match self.with_runtime(|rt| f(rt, args)) {
            Ok(content) => JsonRpcResponse::success(req.id, json!({ "content": content })),
            Err(msg) => JsonRpcResponse::error(req.id, -32603, msg),
        }
    }

    fn write_ok<F>(&self, req: JsonRpcRequest, f: F) -> JsonRpcResponse
    where
        F: FnOnce(&mut Runtime, Value) -> Result<Vec<McpTextContent>, String>,
    {
        let args = req.params.clone().unwrap_or(json!({}));
        match self.with_runtime_mut(|rt| f(rt, args)) {
            Ok(content) => JsonRpcResponse::success(req.id, json!({ "content": content })),
            Err(msg) => JsonRpcResponse::error(req.id, -32603, msg),
        }
    }

    fn read_with<F>(&self, args: Value, f: F) -> Result<Vec<McpTextContent>, String>
    where
        F: FnOnce(&Runtime, Value) -> Result<Vec<McpTextContent>, String>,
    {
        self.with_runtime(|rt| f(rt, args))
    }

    fn write_with<F>(&self, args: Value, f: F) -> Result<Vec<McpTextContent>, String>
    where
        F: FnOnce(&mut Runtime, Value) -> Result<Vec<McpTextContent>, String>,
    {
        self.with_runtime_mut(|rt| f(rt, args))
    }
}

// ─── Tool functions (operate on Runtime) ────────────────────────

fn tool_get_project(runtime: &Runtime) -> Result<Vec<McpTextContent>, String> {
    let project = runtime.project();
    let text = serde_json::to_string_pretty(&json!({
        "id": project.id,
        "name": project.name,
        "scripts": project.scripts.iter().map(|s| json!({"id": s.id, "name": s.name, "bindings": s.bindings})).collect::<Vec<_>>(),
        "structs": project.structs.iter().map(|s| json!({"id": s.id, "name": s.name})).collect::<Vec<_>>(),
        "input_actions": project.input_actions.iter().map(|a| json!({"id": a.id, "label": a.label, "key_code": a.key_code})).collect::<Vec<_>>(),
    })).map_err(|e| e.to_string())?;
    Ok(vec![McpTextContent {
        content_type: "text".into(),
        text,
    }])
}

fn tool_list_scripts(runtime: &Runtime) -> Result<Vec<McpTextContent>, String> {
    let project = runtime.project();
    let text = serde_json::to_string_pretty(&project.scripts).map_err(|e| e.to_string())?;
    Ok(vec![McpTextContent {
        content_type: "text".into(),
        text,
    }])
}

fn tool_get_script(runtime: &Runtime, args: Value) -> Result<Vec<McpTextContent>, String> {
    let project = runtime.project();
    let id = args.get("id").and_then(|v| v.as_str());
    let name = args.get("name").and_then(|v| v.as_str());
    let script = project
        .scripts
        .into_iter()
        .find(|s| id.map_or(false, |id| s.id == id) || name.map_or(false, |n| s.name == n));
    match script {
        Some(s) => {
            let text = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
            Ok(vec![McpTextContent {
                content_type: "text".into(),
                text,
            }])
        }
        None => Err("Script not found".into()),
    }
}

fn tool_update_script(runtime: &mut Runtime, args: Value) -> Result<Vec<McpTextContent>, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?
        .to_string();
    let source = args
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or("Missing source")?
        .to_string();
    runtime.update_script(id, source).map(|_| {
        vec![McpTextContent {
            content_type: "text".into(),
            text: "Script updated successfully".into(),
        }]
    })
}

fn tool_list_structs(runtime: &Runtime) -> Result<Vec<McpTextContent>, String> {
    let project = runtime.project();
    let text = serde_json::to_string_pretty(&project.structs).map_err(|e| e.to_string())?;
    Ok(vec![McpTextContent {
        content_type: "text".into(),
        text,
    }])
}

fn tool_get_struct(runtime: &Runtime, args: Value) -> Result<Vec<McpTextContent>, String> {
    let project = runtime.project();
    let id = args.get("id").and_then(|v| v.as_str());
    let name = args.get("name").and_then(|v| v.as_str());
    let unit = project
        .structs
        .into_iter()
        .find(|s| id.map_or(false, |id| s.id == id) || name.map_or(false, |n| s.name == n));
    match unit {
        Some(s) => {
            let text = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
            Ok(vec![McpTextContent {
                content_type: "text".into(),
                text,
            }])
        }
        None => Err("Struct not found".into()),
    }
}

fn tool_update_struct(runtime: &mut Runtime, args: Value) -> Result<Vec<McpTextContent>, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?
        .to_string();
    let source = args
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or("Missing source")?
        .to_string();
    runtime.update_struct(id, source).map(|_| {
        vec![McpTextContent {
            content_type: "text".into(),
            text: "Struct updated successfully".into(),
        }]
    })
}

fn tool_build_code(runtime: &Runtime) -> Result<Vec<McpTextContent>, String> {
    let project = runtime.project();
    let mut full_source = String::new();
    for unit in &project.structs {
        full_source.push_str(&unit.source);
        full_source.push('\n');
    }
    for script in &project.scripts {
        full_source.push_str(&script.source);
        full_source.push('\n');
    }
    match validate_source(&full_source) {
        Ok(()) => {
            let text = serde_json::to_string_pretty(&json!({
                "valid": true,
                "scripts_count": project.scripts.len(),
                "structs_count": project.structs.len(),
            }))
            .map_err(|e| e.to_string())?;
            Ok(vec![McpTextContent {
                content_type: "text".into(),
                text,
            }])
        }
        Err(error) => {
            let text = serde_json::to_string_pretty(&json!({
                "valid": false,
                "error": error,
            }))
            .map_err(|e| e.to_string())?;
            Ok(vec![McpTextContent {
                content_type: "text".into(),
                text,
            }])
        }
    }
}

// ─── Axum state ─────────────────────────────────────────────────

#[derive(Clone)]
pub struct AxumState {
    pub handler: Arc<Mutex<JsonRpcHandler>>,
}

// ─── Axum route handler ─────────────────────────────────────────

pub async fn jsonrpc_handler(
    State(state): State<AxumState>,
    Json(body): Json<Value>,
) -> Json<JsonRpcResponse> {
    let request: JsonRpcRequest = match serde_json::from_value(body) {
        Ok(req) => req,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                None,
                -32700,
                format!("Parse error: {}", e),
            ));
        }
    };

    let response = {
        let mut handler = state.handler.lock().expect("jsonrpc handler poisoned");
        handler.handle_request(request)
    };

    Json(response)
}

/// Build the axum Router for JSON-RPC endpoints
pub fn router(handler: Arc<Mutex<JsonRpcHandler>>) -> Router {
    let state = AxumState { handler };
    Router::new()
        .route("/jsonrpc", post(jsonrpc_handler))
        .with_state(state)
}
