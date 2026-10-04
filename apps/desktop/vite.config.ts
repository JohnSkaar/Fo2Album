/// <reference types="vitest/config" />
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const designDir = fileURLToPath(new URL("../../design", import.meta.url));

/**
 * `design/tokens.css` er kilden for designet og henter fonter fra Google.
 * Appen skal aldri gjøre nettverkskall, så vi fjerner `@import url(http…)`
 * her og bruker fontene som følger med appen (se `src/styles/fonts.css`).
 */
function stripRemoteImports(): Plugin {
  return {
    name: "p2a-strip-remote-imports",
    enforce: "pre",
    transform(code, id) {
      if (!id.split("?")[0].endsWith(".css")) return null;
      const stripped = code.replace(/@import\s+url\(\s*['"]?https?:[^)]*\)\s*;?/g, "");
      return stripped === code ? null : { code: stripped, map: null };
    },
  };
}

export default defineConfig({
  plugins: [stripRemoteImports(), react()],
  resolve: {
    alias: { "@design": designDir },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    fs: { allow: [".", designDir] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: ["safari15", "chrome105"],
    sourcemap: false,
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
