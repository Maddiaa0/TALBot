# TALBot

**T**ake **A** **L**ook bot — a minimal Telegram notifier and question bridge
for coding agents, usable as a CLI or as an MCP stdio server. Zero config
files beyond a token: `~/.talbot/token`.

Two copy-paste files in this repo — they are **not** interchangeable:

- **[AGENT-SETUP.md](AGENT-SETUP.md)** — a one-time prompt. Paste it into a
  *chat* with your coding agent and it performs the installation below for
  you. It does not belong in any config file.
- **[SNIPPET.md](SNIPPET.md)** — permanent instructions. Paste it into your
  `CLAUDE.md` / `AGENTS.md` so agents know to ping you when they're ready.
  It is not a setup script and does nothing on its own.

## Setup

1. Create a bot: message [@BotFather](https://t.me/BotFather) on Telegram,
   send `/newbot`, copy the token.
2. Store it (verifies the token with Telegram, then writes
   `~/.talbot/token`):
   ```sh
   talbot auth            # prompts for the token
   ```
3. Open a chat with your new bot and send it any message (this is how it
   learns your chat id — bots can't message you first).
4. Test:
   ```sh
   talbot send "hello from talbot"
   ```
   The discovered chat id is cached in `~/.talbot/chat_id`.

## Usage

```sh
talbot auth [TOKEN] [--chat-id <id>]   # store (and verify) the bot token
talbot send <message...>               # send a Telegram message
talbot status                          # check token / chat_id configuration
talbot mcp                             # run as an MCP stdio server (notify, ask)
```

## MCP registration

Claude Code (user scope, exposes a `notify(message)` tool to every session):

```sh
claude mcp add --scope user talbot -- ~/.cargo/bin/talbot mcp
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.talbot]
command = "/path/to/.cargo/bin/talbot"
args = ["mcp"]
tool_timeout_sec = 7260
```

The longer tool timeout lets the interactive `ask` tool wait up to two hours
for a Telegram response. Configure an equivalent timeout in other MCP clients
if you use interactive questions there.

## MCP tools

- `notify(message)` sends a one-way Telegram alert.
- `ask(message, choices, timeout_seconds?)` sends two to eight choice buttons
  and waits for the user to tap one or send a text answer. Only replies from
  the configured private chat are accepted. One question may be active at a
  time; the default and maximum wait are two hours. A shorter wait can be
  requested. When a question expires, TALBot marks the Telegram message as
  expired and removes its buttons so a late tap cannot be mistaken for an
  answer.

## Agent instructions

Paste the block in [SNIPPET.md](SNIPPET.md) into your `CLAUDE.md` /
`AGENTS.md` so agents ask through Telegram when they need a decision and ping
you when they're ready.

## Build

```sh
cargo install --path .   # installs to ~/.cargo/bin/talbot
```
