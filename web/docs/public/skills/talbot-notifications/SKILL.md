---
name: talbot-notifications
description: Install, configure, or use TALBot so coding agents notify a user on Telegram when work finishes, needs review, or is blocked. Use for TALBot CLI setup, local notify MCP registration, Telegram connection, and repository notification-policy requests.
---

# TALBot notifications

Keep TALBot's capability narrow: notify the user when their attention is useful.

## Set up

1. Install from a trusted TALBot checkout with `cargo install --path .`, or from
   `https://github.com/Maddiaa0/TALBot` with `cargo install --git <url>`.
2. Verify the installed binary with `talbot --help`.
3. Register `<absolute path to ~/.cargo/bin/talbot> mcp` in the user's coding
   client without replacing existing configuration.
4. Append the repository's `SNIPPET.md` notification policy to the user's global
   agent instructions. Preserve existing instructions and do not duplicate it.
5. Ask the user to run `talbot auth` themselves in a private terminal. Never ask
   them to paste or reveal their Telegram bot token in chat.
6. Ask the user to message their new bot once, then verify with
   `talbot send "talbot setup complete"` and `talbot status`.
7. Report every file and setting changed.

## Notify

At the end of a task, call `notify(message)` from the local `talbot` MCP server.
If it is unavailable, run `talbot send "<message>"`.

Notify only when work is complete or ready for review, or when progress is
blocked on user input. Keep the message to one or two sentences: say what
happened, where to look, and what the user needs to do.

Do not treat a notification as authorization to merge, deploy, delete, or
modify external systems.

## Read more

Use `https://docs.talbot.maddiaa.com/llms.txt` to discover focused reference
pages. Use `https://docs.talbot.maddiaa.com/api/mcp` only to list, read, or
search the public documentation; it is separate from the local notification
MCP server.
