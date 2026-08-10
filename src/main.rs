mod config;
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
        /// Message text (words are joined with spaces)
        #[arg(required = true)]
        message: Vec<String>,
    },
    /// Run as an MCP stdio server (tools: notify, ask)
    Mcp,
    /// Check token / chat_id configuration
    Status,
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::Auth { token, chat_id } => telegram::auth(token, chat_id),
        Command::Send { message } => {
            let receipt = telegram::send(&message.join(" "))?;
            println!("{receipt}");
            Ok(())
        }
        Command::Mcp => {
            mcp::serve();
            Ok(())
        }
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
