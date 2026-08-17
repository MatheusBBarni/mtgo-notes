import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://mtgo-notes.pages.dev",
  output: "static",
  trailingSlash: "never",
  build: {
    format: "directory",
  },
});
