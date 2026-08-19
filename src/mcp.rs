//! Minimal MCP server speaking newline-delimited JSON-RPC 2.0 over stdio.
//!
//! Exposes `notify(conversation_title, message)` for one-way alerts and
//! `ask(conversation_title, message, choices)` for blocking questions answered
//! from Telegram.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use serde_json::{Value, json};

use crate::{conversation, question, telegram};

pub fn serve() {
    let stdin = std::io::stdin();
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let active_requests = ActiveRequests::default();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message["method"].as_str() == Some("notifications/cancelled") {
            active_requests.cancel(&message);
            continue;
        }
        // Notifications (no id) need no reply.
        let Some(id) = message.get("id").cloned() else {
            continue;
        };

        if is_blocking_ask(&message) {
            let cancelled = active_requests.register(&id);
            let active_requests = active_requests.clone();
            let stdout = Arc::clone(&stdout);
            std::thread::spawn(move || {
                let reply = reply(&message, &id, &cancelled);
                active_requests.finish(&id, &cancelled);

                // A cancelled request's result is no longer useful to the MCP client.
                if !cancelled.load(Ordering::Acquire) {
                    write_reply(&stdout, &reply);
                }
            });
            continue;
        }

        let reply = reply(&message, &id, &AtomicBool::new(false));
        write_reply(&stdout, &reply);
    }
}

fn is_blocking_ask(message: &Value) -> bool {
    message["method"].as_str() == Some("tools/call")
        && message.pointer("/params/name").and_then(Value::as_str) == Some("ask")
}

fn reply(message: &Value, id: &Value, cancelled: &AtomicBool) -> Value {
    match handle_with_cancellation(message, cancelled) {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    }
}

fn write_reply<W: Write>(stdout: &Arc<Mutex<W>>, reply: &Value) {
    let mut stdout = stdout.lock().unwrap_or_else(|error| error.into_inner());
    let _ = writeln!(stdout, "{reply}");
    let _ = stdout.flush();
}

#[derive(Clone, Default)]
struct ActiveRequests {
    requests: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl ActiveRequests {
    fn register(&self, id: &Value) -> Arc<AtomicBool> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(previous) = requests.insert(request_key(id), Arc::clone(&cancelled)) {
            previous.store(true, Ordering::Release);
        }
        cancelled
    }

    fn cancel(&self, message: &Value) {
        let Some(id) = message.pointer("/params/requestId") else {
            return;
        };
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(cancelled) = requests.get(&request_key(id)) {
            cancelled.store(true, Ordering::Release);
        }
    }

    fn finish(&self, id: &Value, completed: &Arc<AtomicBool>) {
        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let key = request_key(id);
        if requests
            .get(&key)
            .is_some_and(|active| Arc::ptr_eq(active, completed))
        {
            requests.remove(&key);
        }
    }
}

fn request_key(id: &Value) -> String {
    serde_json::to_string(id).expect("JSON-RPC request ids are valid JSON")
}

/// Dispatch one request, returning either a JSON-RPC `result` or `error`.
#[cfg(test)]
fn handle(message: &Value) -> Result<Value, Value> {
    handle_with_cancellation(message, &AtomicBool::new(false))
}

fn handle_with_cancellation(message: &Value, cancelled: &AtomicBool) -> Result<Value, Value> {
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
        "tools/call" => Ok(tool_call_with_cancellation(message, cancelled)),
        other => Err(json!({
            "code": -32601,
            "message": format!("method not found: {other}")
        })),
    }
}

#[cfg(test)]
fn tool_call(message: &Value) -> Value {
    tool_call_with_cancellation(message, &AtomicBool::new(false))
}

fn tool_call_with_cancellation(message: &Value, cancelled: &AtomicBool) -> Value {
    let name = message.pointer("/params/name").and_then(Value::as_str);
    match name {
        Some("notify") => notify(message),
        Some("ask") => ask(message, cancelled),
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

fn ask(message: &Value, cancelled: &AtomicBool) -> Value {
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

    match question::ask_with_cancellation(question, &choices, timeout_secs, Some(title), || {
        cancelled.load(Ordering::Acquire)
    }) {
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

    #[test]
    fn only_ask_calls_are_dispatched_as_blocking_work() {
        assert!(is_blocking_ask(&json!({
            "method": "tools/call",
            "params": { "name": "ask" }
        })));
        assert!(!is_blocking_ask(&json!({
            "method": "tools/call",
            "params": { "name": "notify" }
        })));
        assert!(!is_blocking_ask(&json!({ "method": "tools/list" })));
    }

    #[test]
    fn cancellation_targets_the_matching_in_flight_request() {
        let active = ActiveRequests::default();
        let first = active.register(&json!(1));
        let second = active.register(&json!("2"));

        active.cancel(&json!({
            "method": "notifications/cancelled",
            "params": { "requestId": 1, "reason": "Codex moved on" }
        }));

        assert!(first.load(Ordering::Acquire));
        assert!(!second.load(Ordering::Acquire));
    }

    #[test]
    fn finishing_an_old_duplicate_id_does_not_remove_its_replacement() {
        let active = ActiveRequests::default();
        let first = active.register(&json!(1));
        let replacement = active.register(&json!(1));

        assert!(first.load(Ordering::Acquire));
        active.finish(&json!(1), &first);
        active.cancel(&json!({
            "method": "notifications/cancelled",
            "params": { "requestId": 1 }
        }));

        assert!(replacement.load(Ordering::Acquire));
    }
}
