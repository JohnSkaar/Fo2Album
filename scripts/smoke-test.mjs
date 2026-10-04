#!/usr/bin/env node
// Røyktest: starter den bygde appen og venter på at grensesnittet melder seg klart.
// Appen avslutter selv med kode 0 når P2A_SMOKE_TEST er satt (se src-tauri/src/lib.rs).
//
// Bruk: node scripts/smoke-test.mjs <sti-til-binær> [tidsfrist-sekunder]

import { spawn } from "node:child_process";

const [bin, seconds = "90"] = process.argv.slice(2);
if (!bin) {
  console.error("Bruk: smoke-test.mjs <sti-til-binær> [tidsfrist-sekunder]");
  process.exit(2);
}

const child = spawn(bin, [], {
  env: { ...process.env, P2A_SMOKE_TEST: "1" },
  stdio: "inherit",
});

const timer = setTimeout(
  () => {
    console.error(`Røyktest: appen meldte seg ikke klar innen ${seconds} s`);
    child.kill();
    process.exit(1);
  },
  Number(seconds) * 1000,
);

child.on("error", (err) => {
  console.error(`Røyktest: kunne ikke starte ${bin}: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  clearTimeout(timer);
  if (code === 0) {
    console.log("Røyktest: appen startet og grensesnittet lastet");
    process.exit(0);
  }
  console.error(`Røyktest: appen avsluttet med kode ${code ?? signal}`);
  process.exit(1);
});
