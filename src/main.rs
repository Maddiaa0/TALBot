use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Telegram notifier for coding agents.
///
/// Setup: create a bot with @BotFather, `echo '<token>' > ~/.readybot/token`,
/// then send your bot any message so it can discover your chat id.
#[derive(Parser)]
#[command(version, about, verbatim_doc_comment)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Send a Telegram message
    Send {
        /// Message text (words are joined with spaces)
        #[arg(required = true)]
        message: Vec<String>,
    },
    /// Run as an MCP stdio server (tool: notify)
    Mcp,
    /// Check token / chat_id configuration
    Status,
}

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".readybot")
}

fn read_token() -> Result<String, String> {
    let path = config_dir().join("token");
    let token = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    Ok(token)
}

fn api(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}

/// Chat id comes from ~/.readybot/chat_id, or is discovered from getUpdates
/// (the most recent person to message the bot) and cached there.
fn chat_id(token: &str) -> Result<String, String> {
    let path = config_dir().join("chat_id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return Ok(id);
        }
    }
    let resp: Value = ureq::get(&api(token, "getUpdates"))
        .call()
        .map_err(|e| format!("getUpdates failed: {e}"))?
        .into_json()
        .map_err(|e| format!("getUpdates bad response: {e}"))?;
    let id = resp["result"]
        .as_array()
        .and_then(|updates| {
            updates.iter().rev().find_map(|u| {
                u.pointer("/message/chat/id")
                    .or_else(|| u.pointer("/edited_message/chat/id"))
                    .and_then(Value::as_i64)
            })
        })
        .ok_or("no chat id found — send your bot a message on Telegram first")?;
    let id = id.to_string();
    std::fs::write(&path, &id).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(id)
}

fn send(message: &str) -> Result<String, String> {
    let token = read_token()?;
    let chat = chat_id(&token)?;
    let resp = ureq::post(&api(&token, "sendMessage"))
        .send_json(json!({ "chat_id": chat, "text": message }))
        .map_err(|e| format!("sendMessage failed: {e}"))?;
    let body: Value = resp
        .into_json()
        .map_err(|e| format!("sendMessage bad response: {e}"))?;
    if body["ok"].as_bool() != Some(true) {
        return Err(format!("Telegram error: {body}"));
    }
    Ok(format!("sent to chat {chat}"))
}

fn status() {
    match read_token() {
        Ok(t) => {
            println!("token: ok ({}…)", &t[..t.len().min(8)]);
            match chat_id(&t) {
                Ok(id) => println!("chat_id: {id}"),
                Err(e) => println!("chat_id: {e}"),
            }
        }
        Err(e) => println!("token: {e}"),
    }
}

// ---- MCP stdio server (newline-delimited JSON-RPC 2.0) ----

fn mcp_serve() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");
        // Notifications (no id) need no reply
        let Some(id) = id else { continue };
        let result = match method {
            "initialize" => json!({
                "protocolVersion": msg.pointer("/params/protocolVersion")
                    .and_then(Value::as_str).unwrap_or("2024-11-05"),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "readybot", "version": env!("CARGO_PKG_VERSION") }
            }),
            "ping" => json!({}),
            "tools/list" => json!({
                "tools": [{
                    "name": "notify",
                    "description": "Send a Telegram message to the user. Use this to ping them when work is ready for review, when you are blocked, or when a long task finishes.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "Short message: what is ready and where (e.g. 'PR #42 ready for review')"
                            }
                        },
                        "required": ["message"]
                    }
                }]
            }),
            "tools/call" => {
                let name = msg.pointer("/params/name").and_then(Value::as_str);
                if name != Some("notify") {
                    tool_error(format!("unknown tool: {name:?}"))
                } else {
                    let text = msg
                        .pointer("/params/arguments/message")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    match send(text) {
                        Ok(ok) => json!({ "content": [{ "type": "text", "text": ok }] }),
                        Err(e) => tool_error(e),
                    }
                }
            }
            other => {
                write_msg(&mut stdout, &json!({
                    "jsonrpc": "2.0", "id": id,
                    "error": { "code": -32601, "message": format!("method not found: {other}") }
                }));
                continue;
            }
        };
        write_msg(&mut stdout, &json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    }
}

fn tool_error(msg: String) -> Value {
    json!({ "content": [{ "type": "text", "text": msg }], "isError": true })
}

fn write_msg(out: &mut impl Write, v: &Value) {
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

fn main() {
    let _ = std::fs::create_dir_all(config_dir());
    match Cli::parse().command {
        Command::Mcp => mcp_serve(),
        Command::Send { message } => match send(&message.join(" ")) {
            Ok(ok) => println!("{ok}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        Command::Status => status(),
    }
}
