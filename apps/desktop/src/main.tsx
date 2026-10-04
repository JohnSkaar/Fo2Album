import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { invoke, isTauri } from "@tauri-apps/api/core";
import "./styles/fonts.css";
import "@design/tokens.css";
import "./styles/app.css";
import { App } from "./App";

const root = document.getElementById("root");
if (!root) throw new Error("Fant ikke #root");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// Si fra til Rust-kjernen at grensesnittet har lastet (brukes av røyktesten i CI).
if (isTauri()) {
  requestAnimationFrame(() => void invoke("frontend_ready"));
}
