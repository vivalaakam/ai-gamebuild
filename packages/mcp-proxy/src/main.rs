use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

const DEFAULT_URL: &str = "http://127.0.0.1:3001/jsonrpc";

#[derive(Debug, Parser)]
#[command(name = "ai-rpg-mcp-proxy")]
struct Cli {
    /// Backend JSON-RPC URL
    #[arg(long, default_value = DEFAULT_URL)]
    url: String,
    /// Enable debug logging to stderr
    #[arg(long)]
    debug: bool,
    /// Optional path to append logs
    #[arg(long)]
    log_file: Option<String>,
}

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

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let url = cli.url;
    set_log_config(cli.debug, cli.log_file);
    let mut stdout = io::stdout();
    let mut reader = io::stdin().lock();

    while let Some((message, framing)) = read_message(&mut reader)? {
        if message.trim().is_empty() {
            continue;
        }
        debug_log(&format!("recv payload={}", message));
        let request: JsonRpcRequest = match serde_json::from_str(&message) {
            Ok(req) => req,
            Err(err) => {
                let response = JsonRpcResponse::error(None, -32700, format!("Parse error: {err}"));
                send_response(&response, &mut stdout, Framing::Line)?;
                continue;
            }
        };
        if request.id.is_none() {
            if let Err(err) = forward_request(&url, &request) {
                debug_log(&format!("forward notification error={err}"));
            }
            continue;
        }

        let response = match forward_request(&url, &request) {
            Ok(resp) => adjust_response(&request, resp),
            Err(err) => {
                debug_log(&format!("forward error={err}"));
                fallback_response(&request, err)
            }
        };
        debug_log(&format!(
            "send payload={}",
            serde_json::to_string(&response).unwrap_or_else(|_| "<invalid>".into())
        ));
        send_response(&response, &mut stdout, framing)?;
    }
    Ok(())
}

fn forward_request(url: &str, request: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
    let payload = serde_json::to_value(request).map_err(|e| e.to_string())?;
    let response = ureq::post(url)
        .send_json(payload)
        .map_err(|e| e.to_string())?;
    let value = response.into_json::<Value>().map_err(|e| e.to_string())?;
    let json = serde_json::from_value::<JsonRpcResponse>(value).map_err(|e| e.to_string())?;
    Ok(json)
}

fn adjust_response(request: &JsonRpcRequest, response: JsonRpcResponse) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => {
            let backend_result = response.result.clone();
            build_initialize_response(request, backend_result)
        }
        "tools/list" => normalize_tools_list(response),
        _ => response,
    }
}

fn fallback_response(request: &JsonRpcRequest, error: String) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => build_initialize_response(request, None),
        "initialized" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(json!({})),
            error: None,
        },
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(json!({ "tools": [] })),
            error: None,
        },
        _ => JsonRpcResponse::error(
            request.id.clone(),
            -32000,
            format!("Backend unavailable: {error}"),
        ),
    }
}

fn build_initialize_response(request: &JsonRpcRequest, backend_result: Option<Value>) -> JsonRpcResponse {
    let protocol_version = request
        .params
        .as_ref()
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or("2024-11-05");

    let mut capabilities = json!({ "tools": {} });
    let mut server_info = json!({
        "name": "ai-rpg-live-proxy",
        "title": "AI RPG Live Proxy",
        "version": "0.1.0"
    });

    if let Some(result) = backend_result {
        if let Some(result_capabilities) = result.get("capabilities") {
            capabilities = result_capabilities.clone();
        }
        if let Some(result_info) = result.get("serverInfo") {
            server_info = result_info.clone();
        }
    }

    if let Some(obj) = capabilities.as_object_mut() {
        let tools_entry = obj.entry("tools".to_string()).or_insert_with(|| json!({}));
        if let Some(tools_obj) = tools_entry.as_object_mut() {
            tools_obj.entry("listChanged".to_string()).or_insert(json!(false));
        }
    }
    if server_info.get("title").is_none() {
        if let Some(obj) = server_info.as_object_mut() {
            obj.insert("title".to_string(), json!("AI RPG Live Proxy"));
        }
    }

    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: request.id.clone(),
        result: Some(json!({
            "protocolVersion": protocol_version,
            "capabilities": capabilities,
            "serverInfo": server_info
        })),
        error: None,
    }
}

fn normalize_tools_list(response: JsonRpcResponse) -> JsonRpcResponse {
    let mut result = match response.result {
        Some(value) => value,
        None => return response,
    };

    if let Some(tools) = result.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools.iter_mut() {
            if let Some(obj) = tool.as_object_mut() {
                if let Some(schema) = obj.remove("input_schema") {
                    obj.insert("inputSchema".to_string(), schema);
                }
            }
        }
    }

    JsonRpcResponse { result: Some(result), ..response }
}

#[derive(Debug, Clone, Copy)]
enum Framing {
    Line,
    Lsp,
}

fn read_message(reader: &mut (impl BufRead + Read)) -> Result<Option<(String, Framing)>, String> {
    let mut headers: Vec<String> = Vec::new();

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if bytes == 0 {
            return Ok(None);
        }
        if line.trim().is_empty() {
            if headers.is_empty() {
                continue;
            }
            break;
        }

        let trimmed = line.trim_start();
        if headers.is_empty() && (trimmed.starts_with('{') || trimmed.starts_with('[')) {
            return Ok(Some((line.trim_end().to_string(), Framing::Line)));
        }

        headers.push(line);
    }

    let mut content_length = None;
    for header in &headers {
        if header.to_ascii_lowercase().starts_with("content-length:") {
            let value = header
                .split(':')
                .nth(1)
                .ok_or("Missing Content-Length value")?
                .trim();
            content_length = Some(value.parse::<usize>().map_err(|e| e.to_string())?);
            break;
        }
    }

    let length = content_length.ok_or("Missing Content-Length header")?;
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer).map_err(|e| e.to_string())?;
    let payload = String::from_utf8(buffer).map_err(|e| e.to_string())?;
    debug_log(&format!("recv headers={:?} bytes={}", headers, length));
    Ok(Some((payload, Framing::Lsp)))
}

fn send_response(
    response: &JsonRpcResponse,
    stdout: &mut io::Stdout,
    framing: Framing,
) -> Result<(), String> {
    let json = serde_json::to_string(response).map_err(|e| e.to_string())?;
    debug_log(&format!("send bytes={}", json.as_bytes().len()));
    match framing {
        Framing::Line => {
            stdout
                .write_all(format!("{json}\n").as_bytes())
                .map_err(|e| e.to_string())?;
        }
        Framing::Lsp => {
            let header = format!("Content-Length: {}\r\n\r\n", json.as_bytes().len());
            stdout
                .write_all(header.as_bytes())
                .map_err(|e| e.to_string())?;
            stdout.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        }
    }
    stdout.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn debug_log(message: &str) {
    if LOG_CONFIG.with(|cfg| cfg.borrow().debug) {
        eprintln!("[mcp-proxy] {message}");
    }
    if let Some(path) = LOG_CONFIG.with(|cfg| cfg.borrow().log_file.clone()) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "[mcp-proxy] {message}");
        }
    }
}

impl JsonRpcResponse {
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

#[derive(Default, Clone)]
struct LogConfig {
    debug: bool,
    log_file: Option<String>,
}

thread_local! {
    static LOG_CONFIG: std::cell::RefCell<LogConfig> = std::cell::RefCell::new(LogConfig::default());
}

fn set_log_config(debug: bool, log_file: Option<String>) {
    LOG_CONFIG.with(|cfg| {
        *cfg.borrow_mut() = LogConfig { debug, log_file };
    });
}
