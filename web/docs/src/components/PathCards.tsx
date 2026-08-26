const paths = [
  {
    label: "01 · install",
    title: "I want TALBot running",
    description:
      "Install the binary, connect Telegram, and register the local MCP server.",
    href: "/getting-started",
  },
  {
    label: "02 · delegate",
    title: "Let my agent set it up",
    description: "Give your coding agent a safe, bounded setup prompt.",
    href: "/agents/setup",
  },
  {
    label: "03 · retrieve",
    title: "I am an agent",
    description:
      "Use Markdown, llms.txt, SKILL.md, or the read-only docs MCP endpoint.",
    href: "/agents/read-the-docs",
  },
] as const;

export const PathCards = Object.assign(
  function PathCards() {
    return (
      <div className="talbot-paths">
        {paths.map((path) => (
          <a className="talbot-path" href={path.href} key={path.href}>
            <span>{path.label}</span>
            <strong>{path.title}</strong>
            <p>{path.description}</p>
            <i aria-hidden="true">↗</i>
          </a>
        ))}
      </div>
    );
  },
  {
    toMarkdown: () => ({
      type: "list",
      ordered: false,
      spread: true,
      children: paths.map((path) => ({
        type: "listItem",
        spread: false,
        children: [
          {
            type: "paragraph",
            children: [
              {
                type: "link",
                url: path.href,
                children: [{ type: "strong", children: [{ type: "text", value: path.title }] }],
              },
              { type: "text", value: ` — ${path.description}` },
            ],
          },
        ],
      })),
    }),
  },
);
