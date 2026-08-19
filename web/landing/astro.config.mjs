import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://talbot.maddiaa.com",
  output: "static",
  trailingSlash: "never",
  build: {
    inlineStylesheets: "never",
  },
  vite: {
    build: {
      assetsInlineLimit: 0,
    },
  },
});
