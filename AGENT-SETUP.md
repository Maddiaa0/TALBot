# Agent-assisted setup

Copy the block below and paste it into your coding agent (Claude Code, Codex,
etc.) from a checkout of this repo. It will build talbot, wire it into your
agent config, and walk you through connecting your Telegram bot.

> Note: this is a **one-time setup prompt** to paste into a chat — don't put
> it in `CLAUDE.md`/`AGENTS.md`. The instructions that belong in those files
> (telling agents *when to notify you*) are in [SNIPPET.md](SNIPPET.md);
> step 3 below installs them for you.

```text
Set up talbot (a Telegram notifier for coding agents) from this repo:

1. Build and install the binary: run `cargo install --path .` in the repo
   root. The binary lands in ~/.cargo/bin/talbot; verify with
   `talbot --help`.

2. Register the MCP server (do whichever apply to tools I use):
   - Claude Code: `claude mcp add --scope user talbot -- ~/.cargo/bin/talbot mcp`
     (use the absolute expanded path), then confirm it shows as connected in
     `claude mcp list`.
   - Codex: add to ~/.codex/config.toml (create the file if missing,
     append without clobbering existing entries):
       [mcp_servers.talbot]
       command = "<absolute path to ~/.cargo/bin/talbot>"
       args = ["mcp"]

3. Add the agent instructions: append the "Notifying the user (talbot)"
   block from SNIPPET.md in this repo to my global agent instructions —
   ~/.claude/CLAUDE.md for Claude Code and/or ~/.codex/AGENTS.md for Codex.
   Create the file(s) if they don't exist; don't duplicate the block if it's
   already there.

4. Help me connect my bot:
   - Tell me to create a bot via @BotFather on Telegram (/newbot) if I don't
     have one.
   - Do NOT ask me to paste the token into the chat. Have me run
     `talbot auth` myself in a terminal (it prompts for the token, verifies
     it with Telegram, and stores it in ~/.talbot/token).
   - Remind me to send my new bot any message on Telegram, then confirm the
     pipeline works by running `talbot send "talbot setup complete"` and
     checking `talbot status`.

Report what you changed at each step.
```
