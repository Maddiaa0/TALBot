<!-- Paste the block below into CLAUDE.md / AGENTS.md. These are permanent
     behavioral instructions for agents, not a setup script — for one-time
     installation, paste AGENT-SETUP.md into a chat with your agent instead. -->


## Contacting the user (talbot)

When you need a decision that blocks further work, call the MCP tool `ask`
from the `talbot` server with a concise question and two to eight short
choices. Wait for the Telegram answer and continue the same turn with it.
Prefer this over a local-only question prompt when the user may be away.
Questions expire after at most two hours. If `ask` reports that a question
expired, do not assume an answer or continue work that depends on one; end the
current turn and let the user ask again when ready.

If `ask` is unavailable, send the question with `notify` and use the client's
normal question flow.

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
