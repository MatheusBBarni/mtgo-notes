import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://matheusbarni.github.io",
  base: "/mtgo-notes",
  output: "static",
  trailingSlash: "never",
  build: {
    format: "directory",
  },
});
