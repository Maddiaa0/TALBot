const agentSummary =
  "An agent completes its work, calls TALBot's notify tool, and TALBot sends a short Telegram message to the user.";

export const AgentLoop = Object.assign(
  function AgentLoop() {
    return (
      <div className="agent-loop" aria-label={agentSummary}>
        <div className="agent-loop__sky" aria-hidden="true">
          <span className="agent-loop__sun" />
          <span className="agent-loop__cloud agent-loop__cloud--one" />
          <span className="agent-loop__cloud agent-loop__cloud--two" />
        </div>

        <div className="agent-loop__run">
          <div className="agent-loop__eyebrow">
            <span className="agent-loop__pulse" /> agent run · finished
          </div>
          <p>Refactor complete. 218 tests passing.</p>
          <code>notify(message)</code>
        </div>

        <div className="agent-loop__path" aria-hidden="true">
          <span />
          <span />
          <span />
          <span />
        </div>

        <div className="agent-loop__notification">
          <div className="agent-loop__telegram" aria-hidden="true">
            T
          </div>
          <div>
            <div className="agent-loop__notification-header">
              <strong>talbot</strong>
              <span>now</span>
            </div>
            <p>PR #128 ready for review.</p>
          </div>
        </div>
      </div>
    );
  },
  {
    toMarkdown: () => ({
      type: "blockquote",
      children: [
        {
          type: "paragraph",
          children: [{ type: "text", value: agentSummary }],
        },
      ],
    }),
  },
);
