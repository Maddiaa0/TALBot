<!-- Paste the block below into CLAUDE.md / AGENTS.md. These are permanent
     behavioral instructions for agents, not a setup script — for one-time
     installation, paste AGENT-SETUP.md into a chat with your agent instead. -->


## Notifying the user (talbot)

When you finish a task, are ready for the user to review something, or become
blocked on input only the user can provide, notify them on Telegram before
ending your turn:

- Preferred: call the MCP tool `notify` from the `talbot` server with a short
  message saying what is ready and where, e.g.
  `"auth refactor done — branch fix/auth, tests passing, ready for review"`.
- Fallback (if the MCP tool is unavailable): run
  `talbot send "<same message>"` in the shell.

Keep messages to one or two sentences: what happened, where to look, and what
you need from the user. Do not notify for routine intermediate steps — only
when you are done, blocked, or need a decision.
