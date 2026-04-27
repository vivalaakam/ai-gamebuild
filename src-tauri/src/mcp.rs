use ai_rpg_engine::{Project, validate_source};
use crate::storage::ProjectStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpTool {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpTextContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

pub struct McpServer {
    store: ProjectStore,
    project_id: String,
    initialized: bool,
}

impl McpServer {
    pub fn open(db_path: std::path::PathBuf, project_id: String) -> Result<Self, String> {
        let store = ProjectStore::open(db_path)?;
        let _ = store.load_project(&project_id)?;
        Ok(Self {
            store,
            project_id,
            initialized: false,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = stdin.lock();

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                    send_response(&resp, &mut stdout)?;
                    continue;
                }
            };
            let response = self.handle_request(request);
            send_response(&response, &mut stdout)?;
        }
        Ok(())
    }

    fn handle_request(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "initialized" => JsonRpcResponse::success(req.id, json!({})),
            "tools/list" => self.handle_tools_list(req),
            "tools/call" => self.handle_tools_call(req),
            _ => {
                JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method))
            }
        }
    }

    fn handle_initialize(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;
        JsonRpcResponse::success(
            req.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ai-rpg-mcp",
                    "version": "0.1.0"
                }
            }),
        )
    }

    fn handle_tools_list(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let tools = vec![
            McpTool {
                name: "get_project".into(),
                description: "Get current project info including scripts and structs list".into(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {}
                })),
            },
            McpTool {
                name: "list_scripts".into(),
                description: "List all scripts in the project".into(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {}
                })),
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
                description: "Update a script source code by id".into(),
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
                description: "List all structs in the project".into(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {}
                })),
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
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {}
                })),
            },
        ];
        JsonRpcResponse::success(req.id, json!({ "tools": tools }))
    }

    fn handle_tools_call(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params = match &req.params {
            Some(p) => p.clone(),
            None => {
                return JsonRpcResponse::error(req.id.clone(), -32602, "Missing params");
            }
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                return JsonRpcResponse::error(req.id.clone(), -32602, "Missing tool name");
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        let result: Result<Vec<McpTextContent>, String> = match name.as_str() {
            "get_project" => self.tool_get_project(),
            "list_scripts" => self.tool_list_scripts(),
            "get_script" => self.tool_get_script(arguments),
            "update_script" => self.tool_update_script(arguments),
            "list_structs" => self.tool_list_structs(),
            "get_struct" => self.tool_get_struct(arguments),
            "update_struct" => self.tool_update_struct(arguments),
            "build_code" => self.tool_build_code(),
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

    fn load_project(&self) -> Result<Project, String> {
        self.store.load_project(&self.project_id)
    }

    fn tool_get_project(&self) -> Result<Vec<McpTextContent>, String> {
        let project = self.load_project()?;
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

    fn tool_list_scripts(&self) -> Result<Vec<McpTextContent>, String> {
        let project = self.load_project()?;
        let text = serde_json::to_string_pretty(&project.scripts).map_err(|e| e.to_string())?;
        Ok(vec![McpTextContent {
            content_type: "text".into(),
            text,
        }])
    }

    fn tool_get_script(&self, args: Value) -> Result<Vec<McpTextContent>, String> {
        let project = self.load_project()?;
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

    fn tool_update_script(&mut self, args: Value) -> Result<Vec<McpTextContent>, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing id")?;
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or("Missing source")?;

        let mut project = self.load_project()?;
        let script = project
            .scripts
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Script not found")?;
        script.source = source.to_string();
        self.store.save_snapshot(&project)?;
        Ok(vec![McpTextContent {
            content_type: "text".into(),
            text: "Script updated successfully".into(),
        }])
    }

    fn tool_list_structs(&self) -> Result<Vec<McpTextContent>, String> {
        let project = self.load_project()?;
        let text = serde_json::to_string_pretty(&project.structs).map_err(|e| e.to_string())?;
        Ok(vec![McpTextContent {
            content_type: "text".into(),
            text,
        }])
    }

    fn tool_get_struct(&self, args: Value) -> Result<Vec<McpTextContent>, String> {
        let project = self.load_project()?;
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

    fn tool_update_struct(&mut self, args: Value) -> Result<Vec<McpTextContent>, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing id")?;
        let source = args
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or("Missing source")?;

        let mut project = self.load_project()?;
        let unit = project
            .structs
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Struct not found")?;
        unit.source = source.to_string();
        self.store.save_snapshot(&project)?;
        Ok(vec![McpTextContent {
            content_type: "text".into(),
            text: "Struct updated successfully".into(),
        }])
    }

    fn tool_build_code(&self) -> Result<Vec<McpTextContent>, String> {
        let project = self.load_project()?;
        let mut full_source = String::new();
        for unit in &project.structs {
            full_source.push_str(&unit.source);
            full_source.push('\n');
        }
        for script in &project.scripts {
            full_source.push_str(&script.source);
            full_source.push('\n');
        }
        let result = validate_source(&full_source);
        match result {
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
}

fn send_response(response: &JsonRpcResponse, stdout: &mut io::Stdout) -> Result<(), String> {
    let json = serde_json::to_string(response).map_err(|e| e.to_string())?;
    writeln!(stdout, "{}", json).map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())?;
    Ok(())
}
