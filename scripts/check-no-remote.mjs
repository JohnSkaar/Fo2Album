#!/usr/bin/env node
// Sjekker at det bygde grensesnittet ikke laster noe fra nettet (CLAUDE.md, «Lokalt først»).
// CSP-en i tauri.conf.json blokkerer dette uansett; denne sjekken fanger feilen ved bygging
// i stedet for som en tom font eller et manglende bilde i appen.
//
// Bruk: node scripts/check-no-remote.mjs <dist-mappe>

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname } from "node:path";

const dir = process.argv[2];
if (!dir) {
  console.error("Bruk: check-no-remote.mjs <dist-mappe>");
  process.exit(2);
}

// Referanser som får nettleseren til å hente noe: CSS url()/@import og HTML src/href.
const PATTERNS = [
  /url\(\s*['"]?(https?:)?\/\/[^)'"]+/gi,
  /@import\s+['"](https?:)?\/\/[^'"]+/gi,
  /\b(?:src|href)\s*=\s*['"](https?:)?\/\/[^'"]+/gi,
  /fonts\.(googleapis|gstatic)\.com/gi,
];

function* files(d) {
  for (const name of readdirSync(d)) {
    const p = join(d, name);
    if (statSync(p).isDirectory()) yield* files(p);
    else if ([".html", ".css", ".js"].includes(extname(p))) yield p;
  }
}

const problems = [];
for (const file of files(dir)) {
  const text = readFileSync(file, "utf8");
  for (const re of PATTERNS) {
    for (const m of text.matchAll(re)) problems.push(`${file}: ${m[0].slice(0, 120)}`);
  }
}

if (problems.length) {
  console.error("Grensesnittet refererer til ressurser på nettet:\n  " + problems.join("\n  "));
  process.exit(1);
}
console.log(`check-no-remote: ingen eksterne ressurser i ${dir}`);
