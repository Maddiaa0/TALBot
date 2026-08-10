//! Interactive Telegram questions for MCP clients.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use crate::{config, telegram};

const POLL_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_TIMEOUT_SECS: u64 = 2 * 60 * 60;
pub const MAX_TIMEOUT_SECS: u64 = 2 * 60 * 60;
const MAX_CHOICES: usize = 8;
const ACTION_HEADER: &str = "🚨 Action needed";

/// Ask a multiple-choice question in the configured private Telegram chat and
/// wait for the user to tap a button or send a text answer.
pub fn ask(question: &str, choices: &[String], timeout_secs: u64) -> Result<String> {
    let question = question.trim();
    ensure!(!question.is_empty(), "question must not be empty");
    ensure!(
        question.chars().count() <= 3000,
        "question must be at most 3000 characters"
    );
    ensure!(
        (2..=MAX_CHOICES).contains(&choices.len()),
        "choices must contain between 2 and {MAX_CHOICES} items"
    );
    ensure!(
        (30..=MAX_TIMEOUT_SECS).contains(&timeout_secs),
        "timeout_seconds must be between 30 and {MAX_TIMEOUT_SECS}"
    );

    let choices = choices
        .iter()
        .map(|choice| choice.trim().to_string())
        .collect::<Vec<_>>();
    ensure!(
        choices
            .iter()
            .all(|choice| !choice.is_empty() && choice.chars().count() <= 64),
        "each choice must contain between 1 and 64 characters"
    );

    let token = config::read_token()?;
    let chat = telegram::chat_id(&token)?;
    let user_id = ensure_private_chat(&token, &chat)?;
    let lock = AskLock::acquire(&token)?;

    // Ignore updates that arrived before this question was sent.
    let mut offset = pending_offset(&token)?;
    let request_id = request_id();
    let sent = telegram::call(
        &token,
        "sendMessage",
        Some(json!({
            "chat_id": chat,
            "text": pending_text(question),
            "reply_markup": {
                "inline_keyboard": choices.iter().enumerate().map(|(index, choice)| {
                    vec![json!({
                        "text": choice,
                        "callback_data": format!("talbot:{request_id}:{index}")
                    })]
                }).collect::<Vec<_>>()
            }
        })),
    )?;
    let message_id = sent["message_id"]
        .as_i64()
        .context("sendMessage response is missing message_id")?;
    lock.record_question(&chat, message_id)?;
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let wait = wait_label(timeout_secs);
            return match expire_question(&token, &chat, message_id, question, &wait) {
                Ok(()) => Err(anyhow::anyhow!(
                    "Telegram question expired after {wait}; the buttons were removed. \
                     Do not assume an answer or continue work that depends on one"
                )),
                Err(error) => {
                    eprintln!("talbot: could not update the expired Telegram question: {error:#}");
                    Err(anyhow::anyhow!(
                        "Telegram question expired after {wait}, but its Telegram message \
                         could not be updated. The request is inactive; do not assume an \
                         answer or continue work that depends on one"
                    ))
                }
            };
        }
        let poll_secs = remaining.as_secs().clamp(1, POLL_TIMEOUT_SECS);
        let updates = get_updates(&token, offset, poll_secs)?;
        if let Some(next) = next_offset(&updates) {
            offset = Some(next);
        }

        for update in &updates {
            let Some(answer) = parse_answer(update, user_id, &request_id, &choices) else {
                continue;
            };
            acknowledge_answer(&token, &chat, message_id, question, &answer);
            return Ok(answer.text().to_string());
        }
    }
}

fn ensure_private_chat(token: &str, chat: &str) -> Result<i64> {
    let details = telegram::call(token, "getChat", Some(json!({ "chat_id": chat })))?;
    ensure!(
        details["type"].as_str() == Some("private"),
        "interactive questions require a private Telegram chat"
    );
    details["id"]
        .as_i64()
        .context("getChat response is missing a numeric id")
}

fn pending_offset(token: &str) -> Result<Option<i64>> {
    let mut offset = None;
    for _ in 0..100 {
        let updates = get_updates(token, offset, 0)?;
        if updates.is_empty() {
            return Ok(offset);
        }
        offset = next_offset(&updates);
        if updates.len() < 100 {
            return Ok(offset);
        }
    }
    bail!("too many pending Telegram updates; send the question again")
}

fn get_updates(token: &str, offset: Option<i64>, timeout_secs: u64) -> Result<Vec<Value>> {
    let mut body = json!({
        "timeout": timeout_secs,
        "limit": 100,
        "allowed_updates": ["message", "callback_query"]
    });
    if let Some(offset) = offset {
        body["offset"] = json!(offset);
    }
    let result = telegram::call(token, "getUpdates", Some(body))?;
    result
        .as_array()
        .cloned()
        .context("getUpdates response is not an array")
}

fn next_offset(updates: &[Value]) -> Option<i64> {
    updates
        .iter()
        .filter_map(|update| update["update_id"].as_i64())
        .max()
        .map(|id| id + 1)
}

fn request_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}{nanos:x}", std::process::id())
}

fn expire_question(
    token: &str,
    chat: &str,
    message_id: i64,
    question: &str,
    wait: &str,
) -> Result<()> {
    telegram::call(
        token,
        "editMessageText",
        Some(json!({
            "chat_id": chat,
            "message_id": message_id,
            "text": expired_text(question, wait),
            "reply_markup": { "inline_keyboard": [] }
        })),
    )?;
    Ok(())
}

fn acknowledge_answer(
    token: &str,
    chat: &str,
    message_id: i64,
    question: &str,
    answer: &IncomingAnswer,
) {
    if let IncomingAnswer::Choice {
        callback_query_id, ..
    } = answer
        && let Err(error) = telegram::call(
            token,
            "answerCallbackQuery",
            Some(json!({
                "callback_query_id": callback_query_id,
                "text": "✅ Received — sending to Codex now."
            })),
        )
    {
        eprintln!("talbot: could not dismiss the Telegram button spinner: {error:#}");
    }

    if let Err(error) = telegram::call(
        token,
        "sendMessage",
        Some(json!({
            "chat_id": chat,
            "text": confirmation_text(answer.text()),
            "reply_parameters": { "message_id": message_id }
        })),
    ) {
        eprintln!("talbot: could not send the Telegram answer receipt: {error:#}");
    }

    if let Err(error) = mark_answered(token, chat, message_id, question, answer.text()) {
        eprintln!("talbot: could not update the answered Telegram question: {error:#}");
    }
}

fn mark_answered(
    token: &str,
    chat: &str,
    message_id: i64,
    question: &str,
    answer: &str,
) -> Result<()> {
    telegram::call(
        token,
        "editMessageText",
        Some(json!({
            "chat_id": chat,
            "message_id": message_id,
            "text": answered_text(question, answer),
            "reply_markup": { "inline_keyboard": [] }
        })),
    )?;
    Ok(())
}

/// Add a consistent marker to Telegram messages that need the user's input.
/// Callers do not need to add the marker themselves.
pub(crate) fn action_required_text(message: &str) -> String {
    let message = message.trim();
    if message.starts_with(ACTION_HEADER) {
        message.to_string()
    } else {
        format!("{ACTION_HEADER}\n\n{message}")
    }
}

fn pending_text(question: &str) -> String {
    format!(
        "{}\n\nTap a button below, or reply with your answer.",
        action_required_text(question)
    )
}

fn answered_text(question: &str, answer: &str) -> String {
    format!(
        "{question}\n\n✅ Received: {}\n\nTALBot is sending this to Codex now.",
        answer_preview(answer)
    )
}

fn confirmation_text(answer: &str) -> String {
    format!(
        "✅ Received: {}\n\nTALBot is sending this to Codex now.",
        answer_preview(answer)
    )
}

fn answer_preview(answer: &str) -> String {
    const MAX_CHARS: usize = 512;
    if answer.chars().count() <= MAX_CHARS {
        answer.to_string()
    } else {
        let suffix = "…";
        let keep = MAX_CHARS - suffix.chars().count();
        format!(
            "{}{}",
            answer.chars().take(keep).collect::<String>(),
            suffix
        )
    }
}

fn expired_text(question: &str, wait: &str) -> String {
    format!(
        "{question}\n\n⌛ No answer after {wait}, so Codex stopped waiting. Send a new message when you're ready."
    )
}

fn wait_label(seconds: u64) -> String {
    if seconds.is_multiple_of(3600) {
        plural(seconds / 3600, "hour")
    } else if seconds.is_multiple_of(60) {
        plural(seconds / 60, "minute")
    } else {
        plural(seconds, "second")
    }
}

fn plural(value: u64, unit: &str) -> String {
    let suffix = if value == 1 { "" } else { "s" };
    format!("{value} {unit}{suffix}")
}

#[derive(Debug, PartialEq, Eq)]
enum IncomingAnswer {
    Choice {
        text: String,
        callback_query_id: String,
    },
    Text(String),
}

impl IncomingAnswer {
    fn text(&self) -> &str {
        match self {
            Self::Choice { text, .. } | Self::Text(text) => text,
        }
    }
}

fn parse_answer(
    update: &Value,
    user_id: i64,
    request_id: &str,
    choices: &[String],
) -> Option<IncomingAnswer> {
    if id_at(update, "/callback_query/message/chat/id") == Some(user_id)
        && id_at(update, "/callback_query/from/id") == Some(user_id)
    {
        let data = update.pointer("/callback_query/data")?.as_str()?;
        let index = data
            .strip_prefix(&format!("talbot:{request_id}:"))?
            .parse::<usize>()
            .ok()?;
        let text = choices.get(index)?.clone();
        let callback_query_id = update.pointer("/callback_query/id")?.as_str()?.to_string();
        return Some(IncomingAnswer::Choice {
            text,
            callback_query_id,
        });
    }

    if id_at(update, "/message/chat/id") == Some(user_id)
        && id_at(update, "/message/from/id") == Some(user_id)
    {
        let text = update.pointer("/message/text")?.as_str()?.trim();
        if !text.is_empty() {
            return Some(IncomingAnswer::Text(text.to_string()));
        }
    }
    None
}

fn id_at(value: &Value, pointer: &str) -> Option<i64> {
    value.pointer(pointer)?.as_i64()
}

struct AskLock {
    path: PathBuf,
}

impl AskLock {
    fn acquire(token: &str) -> Result<Self> {
        let path = config::dir()?.join("ask.lock");
        for attempt in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    writeln!(file, "{}", std::process::id())?;
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt == 0 && is_stale(&path) {
                        mark_abandoned(token, &path);
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    bail!("another Telegram question is already waiting for an answer");
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("cannot create {}", path.display()));
                }
            }
        }
        unreachable!()
    }

    fn record_question(&self, chat: &str, message_id: i64) -> Result<()> {
        fs::write(
            &self.path,
            json!({
                "pid": process::id(),
                "chat_id": chat,
                "message_id": message_id
            })
            .to_string(),
        )
        .with_context(|| format!("cannot update {}", self.path.display()))
    }
}

impl Drop for AskLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn is_stale(path: &Path) -> bool {
    if let Ok(owner) = fs::read_to_string(path)
        && let Some(pid) = lock_owner_pid(&owner)
        && !process_is_alive(pid)
    {
        return true;
    }

    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .and_then(|modified| modified.elapsed().map_err(std::io::Error::other))
        .is_ok_and(|age| age > Duration::from_secs(MAX_TIMEOUT_SECS + 300))
}

fn lock_owner_pid(contents: &str) -> Option<u32> {
    contents.trim().parse::<u32>().ok().or_else(|| {
        serde_json::from_str::<Value>(contents)
            .ok()?
            .get("pid")?
            .as_u64()?
            .try_into()
            .ok()
    })
}

fn mark_abandoned(token: &str, path: &Path) {
    let Some((chat, message_id)) = fs::read_to_string(path)
        .ok()
        .and_then(|contents| abandoned_message_target(&contents))
    else {
        return;
    };
    if let Err(error) = telegram::call(
        token,
        "editMessageText",
        Some(json!({
            "chat_id": chat,
            "message_id": message_id,
            "text": abandoned_text(),
            "reply_markup": { "inline_keyboard": [] }
        })),
    ) {
        eprintln!("talbot: could not close an abandoned Telegram question: {error:#}");
    }
}

fn abandoned_message_target(contents: &str) -> Option<(String, i64)> {
    let value = serde_json::from_str::<Value>(contents).ok()?;
    Some((
        value.get("chat_id")?.as_str()?.to_string(),
        value.get("message_id")?.as_i64()?,
    ))
}

fn abandoned_text() -> &'static str {
    "⚠️ Codex stopped waiting, so this request is no longer active and nothing was approved. Ask Codex to try again if you still want to allow it."
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(unix))]
fn process_is_alive(_pid: u32) -> bool {
    // Preserve the age-based fallback on platforms where this tiny CLI does
    // not yet have a portable process-liveness probe.
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choices() -> Vec<String> {
        vec!["First".to_string(), "Second".to_string()]
    }

    #[test]
    fn advances_past_the_latest_update() {
        let updates = vec![json!({ "update_id": 7 }), json!({ "update_id": 11 })];
        assert_eq!(next_offset(&updates), Some(12));
        assert_eq!(next_offset(&[]), None);
    }

    #[test]
    fn parses_matching_button_answer() {
        let update = json!({
            "callback_query": {
                "id": "callback-1",
                "from": { "id": 42 },
                "message": { "chat": { "id": 42 } },
                "data": "talbot:request-1:1"
            }
        });
        assert_eq!(
            parse_answer(&update, 42, "request-1", &choices()),
            Some(IncomingAnswer::Choice {
                text: "Second".to_string(),
                callback_query_id: "callback-1".to_string()
            })
        );
    }

    #[test]
    fn rejects_button_from_another_user_or_request() {
        let wrong_user = json!({
            "callback_query": {
                "id": "callback-1",
                "from": { "id": 99 },
                "message": { "chat": { "id": 42 } },
                "data": "talbot:request-1:0"
            }
        });
        let wrong_request = json!({
            "callback_query": {
                "id": "callback-1",
                "from": { "id": 42 },
                "message": { "chat": { "id": 42 } },
                "data": "talbot:old-request:0"
            }
        });
        assert_eq!(parse_answer(&wrong_user, 42, "request-1", &choices()), None);
        assert_eq!(
            parse_answer(&wrong_request, 42, "request-1", &choices()),
            None
        );
    }

    #[test]
    fn parses_text_from_the_configured_private_chat() {
        let update = json!({
            "message": {
                "from": { "id": 42 },
                "chat": { "id": 42 },
                "text": "A custom answer"
            }
        });
        assert_eq!(
            parse_answer(&update, 42, "request-1", &choices()),
            Some(IncomingAnswer::Text("A custom answer".to_string()))
        );
    }

    #[test]
    fn formats_waits_for_expiry_messages() {
        assert_eq!(wait_label(30), "30 seconds");
        assert_eq!(wait_label(60), "1 minute");
        assert_eq!(wait_label(15 * 60), "15 minutes");
        assert_eq!(wait_label(2 * 60 * 60), "2 hours");
    }

    #[test]
    fn marks_messages_that_need_an_answer() {
        assert_eq!(
            pending_text("Deploy the update now?"),
            "🚨 Action needed\n\nDeploy the update now?\n\nTap a button below, or reply with your answer."
        );
        assert_eq!(
            action_required_text("🚨 Action needed\n\nAlready marked"),
            "🚨 Action needed\n\nAlready marked"
        );
    }

    #[test]
    fn removes_the_action_marker_after_an_answer() {
        assert_eq!(
            answered_text("Deploy the update now?", "Yes"),
            "Deploy the update now?\n\n✅ Received: Yes\n\nTALBot is sending this to Codex now."
        );
        assert_eq!(
            confirmation_text("Yes"),
            "✅ Received: Yes\n\nTALBot is sending this to Codex now."
        );
    }

    #[test]
    fn uses_ten_second_telegram_long_polls() {
        assert_eq!(POLL_TIMEOUT_SECS, 10);
    }

    #[test]
    fn recognizes_the_current_process_as_a_live_lock_owner() {
        assert!(process_is_alive(process::id()));
    }

    #[test]
    fn reads_legacy_and_structured_lock_owners() {
        assert_eq!(lock_owner_pid("42\n"), Some(42));
        assert_eq!(
            lock_owner_pid(r#"{"pid":42,"chat_id":"7","message_id":9}"#),
            Some(42)
        );
        assert_eq!(lock_owner_pid("not a lock"), None);
    }

    #[test]
    fn recovers_an_abandoned_question_target() {
        assert_eq!(
            abandoned_message_target(r#"{"pid":42,"chat_id":"7","message_id":9}"#),
            Some(("7".to_string(), 9))
        );
        assert!(abandoned_message_target("42").is_none());
        assert!(abandoned_text().contains("nothing was approved"));
    }

    #[test]
    fn expired_message_is_unambiguous() {
        assert_eq!(
            expired_text("Deploy now?", "2 hours"),
            "Deploy now?\n\n⌛ No answer after 2 hours, so Codex stopped waiting. Send a new message when you're ready."
        );
    }
}
