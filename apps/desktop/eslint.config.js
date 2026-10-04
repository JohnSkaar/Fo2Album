import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "src-tauri"] },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: { globals: globals.browser },
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // Appen skal ikke gjøre nettverkskall fra grensesnittet (CLAUDE.md, «Lokalt først»).
      "no-restricted-globals": [
        "error",
        { name: "fetch", message: "Grensesnittet skal ikke gjøre nettverkskall." },
        { name: "XMLHttpRequest", message: "Grensesnittet skal ikke gjøre nettverkskall." },
        { name: "WebSocket", message: "Grensesnittet skal ikke gjøre nettverkskall." },
        { name: "EventSource", message: "Grensesnittet skal ikke gjøre nettverkskall." },
      ],
    },
  },
);
