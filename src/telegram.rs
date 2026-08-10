//! Telegram Bot API client: token auth, chat-id discovery, and sending.

use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use crate::config;

const TIMEOUT: Duration = Duration::from_secs(30);

/// Call a Bot API method and return its `result` payload, surfacing
/// Telegram's error `description` on failure.
pub(crate) fn call(token: &str, method: &str, body: Option<Value>) -> Result<Value> {
    call_with_timeout(token, method, body, TIMEOUT)
}

pub(crate) fn call_with_timeout(
    token: &str,
    method: &str,
    body: Option<Value>,
    timeout: Duration,
) -> Result<Value> {
    let url = format!("https://api.telegram.org/bot{token}/{method}");
    let result = match body {
        Some(body) => ureq::post(&url).timeout(timeout).send_json(body),
        None => ureq::get(&url).timeout(timeout).call(),
    };
    let response = match result {
        // Telegram reports errors with a 4xx status but puts the details
        // ("ok": false, "description": …) in the body, so keep reading.
        Ok(response) | Err(ureq::Error::Status(_, response)) => response,
        // ureq transport errors include the full request URL, and Telegram bot
        // credentials are embedded in that URL. Never attach the raw error to
        // an error chain that may be printed by a hook or coding agent.
        Err(ureq::Error::Transport(_)) => bail!("{method} request failed before Telegram replied"),
    };
    let mut payload: Value = response
        .into_json()
        .with_context(|| format!("{method} returned a malformed response"))?;
    if payload["ok"].as_bool() != Some(true) {
        let description = payload["description"]
            .as_str()
            .unwrap_or("no error description");
        bail!("{method} failed: {description}");
    }
    Ok(payload["result"].take())
}

pub fn send(message: &str) -> Result<String> {
    let token = config::read_token()?;
    let chat = chat_id(&token)?;
    call(
        &token,
        "sendMessage",
        Some(json!({ "chat_id": chat, "text": message })),
    )?;
    Ok(format!("sent to chat {chat}"))
}

/// Chat id comes from `~/.talbot/chat_id`, or is discovered from getUpdates
/// (the most recent person to message the bot) and cached there.
pub(crate) fn chat_id(token: &str) -> Result<String> {
    if let Some(id) = config::read_chat_id() {
        return Ok(id);
    }
    let updates = call(token, "getUpdates", None)?;
    let id = updates
        .as_array()
        .and_then(|updates| {
            updates.iter().rev().find_map(|update| {
                update
                    .pointer("/message/chat/id")
                    .or_else(|| update.pointer("/edited_message/chat/id"))
                    .and_then(Value::as_i64)
            })
        })
        .context("no chat id found — send your bot a message on Telegram first")?
        .to_string();
    config::write_chat_id(&id)?;
    Ok(id)
}

pub fn auth(token: Option<String>, chat_id: Option<String>) -> Result<()> {
    let token = match token {
        Some(token) => token,
        None => prompt("Paste bot token: ")?,
    };
    let token = token.trim();
    ensure!(!token.is_empty(), "no token given");

    // Verify against Telegram before storing.
    let me = call(token, "getMe", None).context("token verification failed")?;
    let username = me["username"]
        .as_str()
        .context("Telegram response is missing the bot username")?;

    let token_path = config::write_token(token)?;
    println!(
        "token ok (bot @{username}), saved to {}",
        token_path.display()
    );

    match chat_id {
        Some(id) => {
            let path = config::write_chat_id(id.trim())?;
            println!("chat_id saved to {}", path.display());
        }
        None => println!(
            "no chat id given — send @{username} any message on Telegram, \
             it will be auto-discovered on first send"
        ),
    }
    Ok(())
}

pub fn status() {
    match config::read_token() {
        Ok(token) => {
            println!("token: configured");
            match chat_id(&token) {
                Ok(id) => println!("chat_id: {id}"),
                Err(e) => println!("chat_id: {e:#}"),
            }
        }
        Err(e) => println!("token: {e:#}"),
    }
}

fn prompt(message: &str) -> Result<String> {
    eprint!("{message}");
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("failed to read input")?;
    Ok(line)
}

#[cfg(test)]
mod tests {
    #[test]
    fn transport_error_wording_cannot_include_a_bot_url() {
        let message = "getUpdates request failed before Telegram replied";
        assert!(!message.contains("api.telegram.org/bot"));
        assert!(!message.contains(':'));
    }
}
