import { defineConfig } from "vocs/config";

const repositoryUrl = "https://github.com/Maddiaa0/TALBot";

export default defineConfig({
  accentColor: "light-dark(#4f873f, #91cf78)",
  baseUrl: "https://docs.talbot.maddiaa.com",
  checkDeadlinks: true,
  codeHighlight: {
    themes: {
      light: "github-light",
      dark: "github-dark-dimmed",
    },
  },
  colorScheme: "light dark",
  description:
    "Install TALBot and let coding agents notify you on Telegram when work is ready or blocked.",
  editLink: {
    link: `${repositoryUrl}/edit/master/web/docs/src/pages/:path`,
  },
  head: {
    meta: {
      themeColor: "#fdfdfb",
      twitterCard: "summary",
    },
  },
  iconUrl: "/favicon.svg",
  logoUrl: {
    light: "/logo-light.svg",
    dark: "/logo-dark.svg",
  },
  mcp: {
    enabled: true,
  },
  sidebar: [
    {
      text: "Start here",
      collapsed: false,
      items: [
        { text: "What is TALBot?", link: "/" },
        { text: "Quickstart", link: "/getting-started" },
      ],
    },
    {
      text: "For agents",
      collapsed: false,
      items: [
        { text: "Agent setup", link: "/agents/setup" },
        { text: "Read these docs", link: "/agents/read-the-docs" },
      ],
    },
    {
      text: "Reference",
      collapsed: false,
      items: [
        { text: "CLI & MCP", link: "/reference/cli-and-mcp" },
        { text: "Security model", link: "/reference/security" },
      ],
    },
  ],
  socials: [{ icon: "github", link: repositoryUrl }],
  title: "TALBot docs",
  titleTemplate: "%s · TALBot docs",
  topNav: [
    { text: "Docs", link: "/", match: "/" },
    { text: "talbot.maddiaa.com", link: "https://talbot.maddiaa.com" },
  ],
});
