//! Minimal MCP server speaking newline-delimited JSON-RPC 2.0 over stdio.
//!
//! Exposes `notify(conversation_title, message)` for one-way alerts and
//! `ask(conversation_title, message, choices)` for blocking questions answered
//! from Telegram.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::{conversation, question, telegram};

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
                "description": "Send the user a short Telegram message when work is ready, \
                    you need something from them, or a long task finishes. Write like a \
                    normal person: use plain everyday language and avoid developer jargon, \
                    internal names, and acronyms unless the user needs an exact detail. On \
                    the first TALBot message in a conversation, choose a short title for \
                    that conversation. Reuse the exact same title in every later call.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "One or two plain sentences saying what happened, \
                                where to look, and what the user needs to do"
                        },
                        "conversation_title": {
                            "type": "string",
                            "description": "A short title chosen on the first TALBot use \
                                in this conversation and reused unchanged in every later \
                                ask or notify call, for example Finance Page",
                            "minLength": 1,
                            "maxLength": conversation::MAX_TITLE_CHARS,
                            "pattern": "^[^\\r\\n]+$"
                        },
                        "action_required": {
                            "type": "boolean",
                            "description": "Set this to true only when the user must answer or \
                                do something. TALBot will add the action-needed marker.",
                            "default": false
                        }
                    },
                    "required": ["conversation_title", "message"]
                }
            },
            {
                "name": "ask",
                "description": "Ask the user a question in Telegram and wait for their \
                    answer. Write it like a short message to a person, using plain everyday \
                    language with no developer jargon. TALBot adds an action-needed marker. \
                    On the first TALBot message in a conversation, choose a short title and \
                    reuse the exact same title in every later call. \
                    The user can tap a choice or reply with text. Questions expire after at \
                    most two hours; if that happens, stop work that needs the answer.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "A short, plain-language question"
                        },
                        "conversation_title": {
                            "type": "string",
                            "description": "A short title chosen on the first TALBot use \
                                in this conversation and reused unchanged in every later \
                                ask or notify call, for example Finance Page",
                            "minLength": 1,
                            "maxLength": conversation::MAX_TITLE_CHARS,
                            "pattern": "^[^\\r\\n]+$"
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
                    "required": ["conversation_title", "message", "choices"]
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
    let title = match conversation_title(message) {
        Ok(title) => title,
        Err(error) => return tool_error(&error),
    };
    let Some(text) = message
        .pointer("/params/arguments/message")
        .and_then(Value::as_str)
    else {
        return tool_error("missing required argument: message");
    };
    let action_required = match message.pointer("/params/arguments/action_required") {
        Some(value) => match value.as_bool() {
            Some(value) => value,
            None => return tool_error("action_required must be true or false"),
        },
        None => false,
    };
    let text = notification_text(title, text, action_required);
    match telegram::send(&text) {
        Ok(receipt) => tool_success(&receipt),
        Err(e) => tool_error(&format!("{e:#}")),
    }
}

fn notification_text(title: &str, message: &str, action_required: bool) -> String {
    if action_required {
        question::action_required_text_with_title(title, message)
    } else {
        conversation::titled_text(title, message)
    }
}

fn ask(message: &Value) -> Value {
    let title = match conversation_title(message) {
        Ok(title) => title,
        Err(error) => return tool_error(&error),
    };
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

    match question::ask(question, &choices, timeout_secs, Some(title)) {
        Ok(answer) => tool_success(&format!("User answered: {answer}")),
        Err(e) => tool_error(&format!("{e:#}")),
    }
}

fn conversation_title(message: &Value) -> Result<&str, String> {
    let value = message
        .pointer("/params/arguments/conversation_title")
        .ok_or_else(|| "missing required argument: conversation_title".to_string())?;
    let value = value
        .as_str()
        .ok_or_else(|| "conversation_title must be a string".to_string())?;
    conversation::title(value).map_err(|error| error.to_string())
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
        assert_eq!(
            result.pointer("/tools/0/inputSchema/properties/action_required/default"),
            Some(&json!(false))
        );
        assert_eq!(
            result.pointer("/tools/0/inputSchema/properties/conversation_title/maxLength"),
            Some(&json!(80))
        );
        assert_eq!(
            result.pointer("/tools/1/inputSchema/properties/conversation_title/maxLength"),
            Some(&json!(80))
        );
    }

    #[test]
    fn ask_requires_string_choices() {
        let result = tool_call(&json!({
            "params": {
                "name": "ask",
                "arguments": {
                    "conversation_title": "Finance Page",
                    "message": "Pick one",
                    "choices": [1, 2]
                }
            }
        }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            result.pointer("/content/0/text").and_then(Value::as_str),
            Some("choices must be an array of strings")
        );
    }

    #[test]
    fn marks_notifications_that_need_action() {
        assert_eq!(
            notification_text("Finance Page", "Please choose a release date.", true),
            "🚨 Action needed\n\nFinance Page\n\nPlease choose a release date."
        );
        assert_eq!(
            notification_text("Finance Page", "The update is ready.", false),
            "Finance Page\n\nThe update is ready."
        );
    }

    #[test]
    fn rejects_an_invalid_action_marker_flag() {
        let result = tool_call(&json!({
            "params": {
                "name": "notify",
                "arguments": {
                    "conversation_title": "Finance Page",
                    "message": "Please choose a release date.",
                    "action_required": "yes"
                }
            }
        }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            result.pointer("/content/0/text").and_then(Value::as_str),
            Some("action_required must be true or false")
        );
    }

    #[test]
    fn requires_a_conversation_title() {
        let result = tool_call(&json!({
            "params": {
                "name": "notify",
                "arguments": { "message": "The update is ready." }
            }
        }));
        assert_eq!(result["isError"], true);
        assert_eq!(
            result.pointer("/content/0/text").and_then(Value::as_str),
            Some("missing required argument: conversation_title")
        );
    }
}
