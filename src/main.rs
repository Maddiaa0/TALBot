mod config;
mod conversation;
mod hook;
mod mcp;
mod question;
mod telegram;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// talbot (Take A Look bot) — Telegram notifier for coding agents.
///
/// Setup: create a bot with @BotFather, run `talbot auth`,
/// then send your bot any message so it can discover your chat id.
#[derive(Parser)]
#[command(version, about, verbatim_doc_comment)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Store the bot token (and optionally chat id)
    Auth {
        /// Bot token from @BotFather (prompted for if omitted)
        token: Option<String>,
        /// Chat id to send to (auto-discovered on first send if omitted)
        #[arg(long)]
        chat_id: Option<String>,
    },
    /// Send a Telegram message
    Send {
        /// Stable title for the coding-agent conversation
        #[arg(long)]
        conversation_title: Option<String>,
        /// Message text (words are joined with spaces)
        #[arg(required = true)]
        message: Vec<String>,
    },
    /// Run as an MCP stdio server (tools: notify, ask)
    Mcp,
    /// Run a coding-agent integration hook
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },
    /// Check token / chat_id configuration
    Status,
}

#[derive(Subcommand)]
enum HookCommand {
    /// Handle a Codex PermissionRequest event through Telegram
    PermissionRequest {
        /// Maximum time to wait for the Telegram answer
        #[arg(long, default_value_t = question::DEFAULT_TIMEOUT_SECS)]
        timeout_seconds: u64,
    },
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::Auth { token, chat_id } => telegram::auth(token, chat_id),
        Command::Send {
            conversation_title,
            message,
        } => {
            let message = message.join(" ");
            let message = match conversation_title.as_deref() {
                Some(title) => conversation::titled_text(conversation::title(title)?, &message),
                None => message,
            };
            let receipt = telegram::send(&message)?;
            println!("{receipt}");
            Ok(())
        }
        Command::Mcp => {
            mcp::serve();
            Ok(())
        }
        Command::Hook {
            command: HookCommand::PermissionRequest { timeout_seconds },
        } => hook::permission_request(timeout_seconds),
        Command::Status => {
            telegram::status();
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = run(Cli::parse().command) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
