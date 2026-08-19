# TALBot

**T**ake **A** **L**ook bot — a minimal Telegram notifier for coding agents,
usable as a CLI or as an MCP stdio server. Zero config files beyond a token:
`~/.talbot/token`.

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
talbot mcp                             # run as an MCP stdio server (tool: notify)
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
```

## Agent instructions

Paste the block in [SNIPPET.md](SNIPPET.md) into your `CLAUDE.md` /
`AGENTS.md` so agents ping you when they're ready.

## Build

```sh
cargo install --path .   # installs to ~/.cargo/bin/talbot
```
