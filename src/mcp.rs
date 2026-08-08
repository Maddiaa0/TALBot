//! Minimal MCP server speaking newline-delimited JSON-RPC 2.0 over stdio.
//!
//! Exposes a single tool, `notify(message)`, that sends a Telegram message.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::telegram;

pub fn serve() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // Notifications (no id) need no reply.
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        let reply = match handle(&message) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
        };
        let _ = writeln!(stdout, "{reply}");
        let _ = stdout.flush();
    }
}

/// Dispatch one request, returning either a JSON-RPC `result` or `error`.
fn handle(message: &Value) -> Result<Value, Value> {
    match message["method"].as_str().unwrap_or_default() {
        "initialize" => Ok(json!({
            "protocolVersion": message.pointer("/params/protocolVersion")
                .and_then(Value::as_str).unwrap_or("2024-11-05"),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "talbot", "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({
            "tools": [{
                "name": "notify",
                "description": "Send a Telegram message to the user. Use this to ping them \
                    when work is ready for review, when you are blocked, or when a long \
                    task finishes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Short message: what is ready and where \
                                (e.g. 'PR #42 ready for review')"
                        }
                    },
                    "required": ["message"]
                }
            }]
        })),
        "tools/call" => Ok(tool_call(message)),
        other => Err(json!({
            "code": -32601,
            "message": format!("method not found: {other}")
        })),
    }
}

fn tool_call(message: &Value) -> Value {
    let name = message.pointer("/params/name").and_then(Value::as_str);
    if name != Some("notify") {
        return tool_error(&format!("unknown tool: {name:?}"));
    }
    let Some(text) = message
        .pointer("/params/arguments/message")
        .and_then(Value::as_str)
    else {
        return tool_error("missing required argument: message");
    };
    match telegram::send(text) {
        Ok(receipt) => json!({ "content": [{ "type": "text", "text": receipt }] }),
        Err(e) => tool_error(&format!("{e:#}")),
    }
}

fn tool_error(message: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}
