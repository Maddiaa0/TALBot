<!-- Paste the block below into CLAUDE.md / AGENTS.md. These are permanent
     behavioral instructions for agents, not a setup script — for one-time
     installation, paste AGENT-SETUP.md into a chat with your agent instead. -->


## Contacting the user (talbot)

When you need a decision that blocks further work, call the MCP tool `ask`
from the `talbot` server with a concise question and two to eight short
choices. Wait for the Telegram answer and continue the same turn with it.
Prefer this over a local-only question prompt when the user may be away.
Write the question like a short message to a person: use plain everyday
language and avoid developer jargon, internal names, and acronyms unless the
user needs an exact detail to answer. TALBot adds the `🚨 Action needed`
marker, so do not add it yourself.
Questions expire after at most two hours. If `ask` reports that a question
expired, do not assume an answer or continue work that depends on one; end the
current turn and let the user ask again when ready.

While a TALBot question or Codex permission request is waiting, keep its
underlying tool call or background wait alive. Use bounded wait calls so you
can still post occasional status updates, but do not terminate the wait merely
because the user has not answered yet. Cancelling that wait disconnects the
Telegram buttons from Codex. If a wait was interrupted, treat the old request
as inactive and issue a fresh request when the decision is still needed.

If `ask` is unavailable, send the question with `notify`, set
`action_required` to `true`, and use the client's normal question flow.

When you finish a task, are ready for the user to review something, or become
blocked on input only the user can provide, notify them on Telegram before
ending your turn:

- Preferred: call the MCP tool `notify` from the `talbot` server with a short
  message saying what is ready and where, e.g.
  `"The login fix is ready in PR #42. Tests passed and it's ready for you to review."`.
- Fallback (if the MCP tool is unavailable): run
  `talbot send "<same message>"` in the shell.

Keep messages to one or two sentences: what happened, where to look, and what
you need from the user. Write like a normal person and avoid jargon. When the
message requires the user to answer or do something, set `action_required` to
`true`; TALBot will add the `🚨 Action needed` marker. Leave it false for
ordinary completion messages. Do not notify for routine intermediate steps —
only when you are done, blocked, or need a decision.
