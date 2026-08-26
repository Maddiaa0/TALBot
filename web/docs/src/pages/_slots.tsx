export function SidebarHeader() {
  return (
    <div className="talbot-sidebar-note">
      <span aria-hidden="true" />
      built for agents + humans
    </div>
  );
}

export function OutlineFooter() {
  return (
    <div className="talbot-outline-note">
      <strong>Agent entry points</strong>
      <a href="/llms.txt">llms.txt</a>
      <a href="/skills/talbot-notifications/SKILL.md">SKILL.md</a>
      <a href="/agents/read-the-docs#docs-mcp">docs MCP</a>
    </div>
  );
}

export function Footer() {
  return (
    <div className="talbot-footer">
      Open source, MIT licensed, and deliberately small.
    </div>
  );
}
