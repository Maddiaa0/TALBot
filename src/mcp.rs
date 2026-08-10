//! Minimal MCP server speaking newline-delimited JSON-RPC 2.0 over stdio.
//!
//! Exposes `notify(message)` for one-way alerts and `ask(message, choices)`
//! for blocking questions answered from Telegram.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::{question, telegram};

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
        "tools/list" => Ok(json!({ "tools": [
            {
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
            },
            {
                "name": "ask",
                "description": "Ask the user a blocking question in Telegram and wait for \
                    their answer. Prefer this over a local-only question prompt when the \
                    user may be away. The user can tap a choice or send a text answer. \
                    Questions expire after at most two hours; on expiry, stop work that \
                    depends on the answer.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The concise question to send"
                        },
                        "choices": {
                            "type": "array",
                            "description": "Two to eight short button labels",
                            "items": { "type": "string", "minLength": 1, "maxLength": 64 },
                            "minItems": 2,
                            "maxItems": 8
                        },
                        "timeout_seconds": {
                            "type": "integer",
                            "description": "How long to wait for an answer",
                            "minimum": 30,
                            "maximum": 7200,
                            "default": 7200
                        }
                    },
                    "required": ["message", "choices"]
                }
            }
        ] })),
        "tools/call" => Ok(tool_call(message)),
        other => Err(json!({
            "code": -32601,
            "message": format!("method not found: {other}")
        })),
    }
}

fn tool_call(message: &Value) -> Value {
    let name = message.pointer("/params/name").and_then(Value::as_str);
    match name {
        Some("notify") => notify(message),
        Some("ask") => ask(message),
        _ => tool_error(&format!("unknown tool: {name:?}")),
    }
}

fn notify(message: &Value) -> Value {
    let Some(text) = message
        .pointer("/params/arguments/message")
        .and_then(Value::as_str)
    else {
        return tool_error("missing required argument: message");
    };
    match telegram::send(text) {
        Ok(receipt) => tool_success(&receipt),
        Err(e) => tool_error(&format!("{e:#}")),
    }
}

fn ask(message: &Value) -> Value {
    let Some(question) = message
        .pointer("/params/arguments/message")
        .and_then(Value::as_str)
    else {
        return tool_error("missing required argument: message");
    };
    let Some(choices) = message
        .pointer("/params/arguments/choices")
        .and_then(Value::as_array)
        .and_then(|choices| {
            choices
                .iter()
                .map(|choice| choice.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        })
    else {
        return tool_error("choices must be an array of strings");
    };
    let timeout_secs = match message.pointer("/params/arguments/timeout_seconds") {
        Some(value) => match value.as_u64() {
            Some(value) => value,
            None => return tool_error("timeout_seconds must be a positive integer"),
        },
        None => question::DEFAULT_TIMEOUT_SECS,
    };

    match question::ask(question, &choices, timeout_secs) {
        Ok(answer) => tool_success(&format!("User answered: {answer}")),
        Err(e) => tool_error(&format!("{e:#}")),
    }
}

fn tool_success(message: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": message }] })
}

fn tool_error(message: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_notify_and_ask_tools() {
        let result = handle(&json!({ "method": "tools/list" })).unwrap();
        let names = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["notify", "ask"]);
        assert_eq!(
            result.pointer("/tools/1/inputSchema/properties/timeout_seconds/default"),
            Some(&json!(7200))
        );
        assert_eq!(
            result.pointer("/tools/1/inputSchema/properties/timeout_seconds/maximum"),
            Some(&json!(7200))
        );
    }

    #[test]
    fn ask_requires_string_choices() {
        let result = tool_call(&json!({
            "params": {
                "name": "ask",
                "arguments": { "message": "Pick one", "choices": [1, 2] }
            }
        }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            result.pointer("/content/0/text").and_then(Value::as_str),
            Some("choices must be an array of strings")
        );
    }
}
