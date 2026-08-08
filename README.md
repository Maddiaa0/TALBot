# readybot

Minimal Telegram notifier for coding agents, usable as a CLI or as an MCP
stdio server. Zero config files beyond a token: `~/.readybot/token`.

## Setup

1. Create a bot: message [@BotFather](https://t.me/BotFather) on Telegram,
   send `/newbot`, copy the token.
2. Store it (verifies the token with Telegram, then writes
   `~/.readybot/token`):
   ```sh
   readybot auth            # prompts for the token
   ```
3. Open a chat with your new bot and send it any message (this is how it
   learns your chat id — bots can't message you first).
4. Test:
   ```sh
   readybot send "hello from readybot"
   ```
   The discovered chat id is cached in `~/.readybot/chat_id`.

## Usage

```sh
readybot auth [TOKEN] [--chat-id <id>]   # store (and verify) the bot token
readybot send <message...>               # send a Telegram message
readybot status                          # check token / chat_id configuration
readybot mcp                             # run as an MCP stdio server (tool: notify)
```

## MCP registration

Already registered at user scope in this environment:

```sh
claude mcp add --scope user readybot -- /root/.cargo/bin/readybot mcp
```

This exposes one tool, `notify(message)`, to every Claude Code session.

## Agent instructions

Paste the block in [SNIPPET.md](SNIPPET.md) into your `CLAUDE.md` /
`AGENTS.md` so agents ping you when they're ready.

## Build

```sh
cargo install --path .   # installs to ~/.cargo/bin/readybot
```
