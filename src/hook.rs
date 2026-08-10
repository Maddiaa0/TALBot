//! Coding-agent hook integrations.

use std::io::{Read as _, Write as _};

use anyhow::{Context, Result, ensure};
use serde_json::{Map, Value, json};

use crate::question;

const ALLOW_LABEL: &str = "Allow once";
const DENY_LABEL: &str = "Don't allow";
const MAX_QUESTION_CHARS: usize = 3000;
const HIDDEN_INPUT: &str =
    "[Hidden because this may contain a password or other private information]";

/// Read a Codex `PermissionRequest` hook event from stdin, collect a Telegram
/// decision, and write the hook decision to stdout. Any missing or expired
/// approval is denied so Codex never falls back to an unattended local prompt.
pub fn permission_request(timeout_secs: u64) -> Result<()> {
    let response = match read_and_collect_permission(timeout_secs) {
        Ok(response) => response,
        Err(error) => {
            eprintln!("talbot permission hook: {error:#}");
            deny_response("You did not approve this in Telegram, so TALBot blocked it.")
        }
    };

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer(&mut stdout, &response)
        .context("cannot write the PermissionRequest hook decision")?;
    writeln!(stdout).context("cannot finish the PermissionRequest hook decision")
}

fn read_and_collect_permission(timeout_secs: u64) -> Result<Value> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("cannot read the PermissionRequest hook event")?;
    collect_permission(&input, timeout_secs)
}

fn collect_permission(input: &str, timeout_secs: u64) -> Result<Value> {
    let event: Value = serde_json::from_str(input).context("invalid hook event JSON")?;
    ensure!(
        event["hook_event_name"].as_str() == Some("PermissionRequest"),
        "expected a PermissionRequest hook event"
    );

    let message = permission_message(&event)?;
    let choices = vec![ALLOW_LABEL.to_string(), DENY_LABEL.to_string()];
    let answer = question::ask(&message, &choices, timeout_secs)?;

    if answer == ALLOW_LABEL {
        Ok(allow_response())
    } else {
        Ok(deny_response("You chose not to allow this in Telegram."))
    }
}

fn permission_message(event: &Value) -> Result<String> {
    let tool_name = event["tool_name"]
        .as_str()
        .context("PermissionRequest is missing tool_name")?;
    let cwd = event["cwd"].as_str().unwrap_or("unknown");
    let tool_input = &event["tool_input"];

    let mut sections = vec![
        "Codex needs your okay before it can continue.".to_string(),
        format!("What it wants to do: {}", action_name(tool_name)),
        format!("Folder: {}", truncate_chars(cwd, 500)),
    ];

    if let Some(description) = tool_input.get("description").and_then(Value::as_str) {
        sections.push(format!(
            "Why: {}",
            truncate_chars(&safe_text(description), 500)
        ));
    }

    let header = sections.join("\n\n");
    let input_label = "\n\nDetails:\n";
    let footer = "\n\nDo you want to allow this once?";
    let fixed_chars = header.chars().count() + input_label.chars().count() + footer.chars().count();
    let available_input_chars = MAX_QUESTION_CHARS.saturating_sub(fixed_chars);
    let preview = truncate_chars(&input_preview(tool_name, tool_input), available_input_chars);

    Ok(format!("{header}{input_label}{preview}{footer}"))
}

fn action_name(tool_name: &str) -> String {
    match tool_name {
        "Bash" => "Run a terminal command".to_string(),
        "apply_patch" => "Change files".to_string(),
        name if name.starts_with("mcp__") => {
            let mut parts = name.split("__").skip(1);
            let service = parts.next().unwrap_or("another app");
            let action = parts.next().unwrap_or("use a tool");
            format!(
                "Use {} to {}",
                humanize_name(service),
                humanize_name(action)
            )
        }
        name => humanize_name(name),
    }
}

fn humanize_name(name: &str) -> String {
    name.replace(['_', '-'], " ")
}

fn input_preview(tool_name: &str, tool_input: &Value) -> String {
    if matches!(tool_name, "Bash" | "apply_patch")
        && let Some(command) = tool_input.get("command").and_then(Value::as_str)
    {
        return safe_text(command);
    }

    let sanitized = sanitize_value(tool_input, None);
    serde_json::to_string_pretty(&sanitized).unwrap_or_else(|_| "[unavailable]".to_string())
}

fn sanitize_value(value: &Value, key: Option<&str>) -> Value {
    if key.is_some_and(is_sensitive) {
        return Value::String("[HIDDEN]".to_string());
    }

    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| key.as_str() != "description")
                .map(|(key, value)| (key.clone(), sanitize_value(value, Some(key))))
                .collect::<Map<_, _>>(),
        ),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| sanitize_value(value, key))
                .collect(),
        ),
        Value::String(text) if looks_sensitive(text) => Value::String(HIDDEN_INPUT.to_string()),
        _ => value.clone(),
    }
}

fn safe_text(text: &str) -> String {
    if looks_sensitive(text) {
        HIDDEN_INPUT.to_string()
    } else {
        text.to_string()
    }
}

fn is_sensitive(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '.'], "_");
    [
        "secret",
        "token",
        "password",
        "passwd",
        "cookie",
        "authorization",
        "api_key",
        "private_key",
        "credential",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn looks_sensitive(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    [
        "secret",
        "token",
        "password",
        "passwd",
        "cookie",
        "authorization",
        "api_key",
        "api-key",
        "apikey",
        "private_key",
        "private-key",
        "credential",
        "bearer ",
        "ghp_",
        "github_pat_",
        "sk-",
        "xoxb-",
        "akia",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let suffix = "\n… [truncated]";
    let keep = max_chars.saturating_sub(suffix.chars().count());
    format!("{}{}", text.chars().take(keep).collect::<String>(), suffix)
}

fn allow_response() -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": { "behavior": "allow" }
        }
    })
}

fn deny_response(message: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": {
                "behavior": "deny",
                "message": message
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_codex_allow_decision() {
        assert_eq!(
            allow_response(),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PermissionRequest",
                    "decision": { "behavior": "allow" }
                }
            })
        );
    }

    #[test]
    fn builds_the_codex_deny_decision() {
        assert_eq!(
            deny_response("No approval received."),
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PermissionRequest",
                    "decision": {
                        "behavior": "deny",
                        "message": "No approval received."
                    }
                }
            })
        );
    }

    #[test]
    fn formats_a_shell_permission_request() {
        let event = json!({
            "hook_event_name": "PermissionRequest",
            "cwd": "/workspace/project",
            "tool_name": "Bash",
            "tool_input": {
                "command": "rm -f /tmp/result.txt",
                "description": "Remove a disposable test file"
            }
        });
        let message = permission_message(&event).unwrap();
        assert!(message.contains("Codex needs your okay before it can continue."));
        assert!(message.contains("What it wants to do: Run a terminal command"));
        assert!(message.contains("Folder: /workspace/project"));
        assert!(message.contains("Why: Remove a disposable test file"));
        assert!(message.contains("rm -f /tmp/result.txt"));
        assert!(message.ends_with("Do you want to allow this once?"));
    }

    #[test]
    fn hides_sensitive_shell_input() {
        let event = json!({
            "hook_event_name": "PermissionRequest",
            "cwd": "/workspace/project",
            "tool_name": "Bash",
            "tool_input": {
                "command": "API_TOKEN=do-not-leak curl https://example.test"
            }
        });
        let message = permission_message(&event).unwrap();
        assert!(message.contains(HIDDEN_INPUT));
        assert!(!message.contains("do-not-leak"));
    }

    #[test]
    fn redacts_sensitive_structured_arguments() {
        let input = json!({
            "path": "/tmp/result.txt",
            "access_token": "do-not-leak",
            "nested": { "password": "also-secret" }
        });
        let preview = input_preview("mcp__example__write", &input);
        assert!(preview.contains("/tmp/result.txt"));
        assert!(!preview.contains("do-not-leak"));
        assert!(!preview.contains("also-secret"));
        assert_eq!(preview.matches("[HIDDEN]").count(), 2);
    }

    #[test]
    fn gives_tools_plain_names() {
        assert_eq!(action_name("Bash"), "Run a terminal command");
        assert_eq!(action_name("apply_patch"), "Change files");
        assert_eq!(
            action_name("mcp__github__create_issue"),
            "Use github to create issue"
        );
    }

    #[test]
    fn keeps_telegram_questions_within_the_limit() {
        let event = json!({
            "hook_event_name": "PermissionRequest",
            "cwd": "/workspace/project",
            "tool_name": "Bash",
            "tool_input": { "command": "x".repeat(4000) }
        });
        let message = permission_message(&event).unwrap();
        assert_eq!(message.chars().count(), MAX_QUESTION_CHARS);
        assert!(message.contains("… [truncated]"));
        assert!(message.ends_with("Do you want to allow this once?"));
    }
}
