#!/usr/bin/env node
// Bumps the app version across every version file and, optionally, finalizes
// the CHANGELOG "Unreleased" section into a dated release.
//
// Usage:
//   node bump_version.mjs <new-version> [--changelog] [--date YYYY-MM-DD]
//   node bump_version.mjs --current            # prints the canonical version and exits
//
// The canonical source is src-tauri/tauri.conf.json > version (see RELEASES.md §5).
// Edits are surgical regex replacements so the diffs stay minimal and reviewable.

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

// Repo root is two levels up from .claude/skills/release-version/scripts.
const scriptDir = dirname(fileURLToPath(import.meta.url));
const root = resolve(scriptDir, "..", "..", "..", "..");

const rel = (p) => resolve(root, p);
const read = (p) => readFileSync(rel(p), "utf8");
const write = (p, s) => writeFileSync(rel(p), s);

const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

const TAURI_CONF = "src-tauri/tauri.conf.json";
const PACKAGE_JSON = "package.json";
const CARGO_TOML = "src-tauri/Cargo.toml";
const CARGO_LOCK = "src-tauri/Cargo.lock";
const CHANGELOG = "CHANGELOG.md";

function currentVersion() {
  const conf = JSON.parse(read(TAURI_CONF));
  return conf.version;
}

// Replaces exactly one match of `re` in the file at `path`, or throws so a
// silently-missed file can never ship a half-bumped release.
function replaceOne(path, re, replacement, label) {
  const before = read(path);
  const matches = before.match(new RegExp(re.source, re.flags.replace("g", "") + "g"));
  if (!matches || matches.length === 0) {
    throw new Error(`Nenhuma versão encontrada em ${path} (${label}).`);
  }
  if (matches.length > 1) {
    throw new Error(`Padrão ambíguo em ${path} (${label}): ${matches.length} ocorrências.`);
  }
  write(path, before.replace(re, replacement));
}

function bumpVersionFiles(version) {
  // package.json — first "version" key belongs to this package.
  replaceOne(
    PACKAGE_JSON,
    /("version":\s*")[^"]+(")/,
    `$1${version}$2`,
    "package.json > version",
  );

  // tauri.conf.json — top-level "version" key.
  replaceOne(
    TAURI_CONF,
    /("version":\s*")[^"]+(")/,
    `$1${version}$2`,
    "tauri.conf.json > version",
  );

  // Cargo.toml — the version line inside [package]. `\r?\n` tolerates CRLF.
  replaceOne(
    CARGO_TOML,
    /(\[package\][\s\S]*?\r?\nversion\s*=\s*")[^"]+(")/,
    `$1${version}$2`,
    "Cargo.toml [package] version",
  );

  // Cargo.lock — the version line inside the secrets-storage package block.
  replaceOne(
    CARGO_LOCK,
    /(name = "secrets-storage"\r?\nversion = ")[^"]+(")/,
    `$1${version}$2`,
    "Cargo.lock secrets-storage version",
  );
}

// Turns "## [Unreleased]" + its body into a dated release section and seeds a
// fresh empty Unreleased scaffold above it (Keep a Changelog, pt-BR).
function finalizeChangelog(version, date) {
  const text = read(CHANGELOG);
  const marker = "## [Unreleased]";
  const idx = text.indexOf(marker);
  if (idx === -1) {
    throw new Error(`Seção "${marker}" não encontrada em ${CHANGELOG}.`);
  }

  // Everything from the Unreleased heading to the next "## " heading (or EOF).
  const bodyStart = idx + marker.length;
  const nextHeading = text.indexOf("\n## ", bodyStart);
  const end = nextHeading === -1 ? text.length : nextHeading;
  const body = text.slice(bodyStart, end).trim();

  if (!body) {
    throw new Error(
      `A seção Unreleased está vazia. Adicione as entradas do changelog antes de finalizar a versão.`,
    );
  }

  const freshUnreleased = "## [Unreleased]\n";
  const released = `## [${version}] - ${date}\n\n${body}\n`;
  const replacement = `${freshUnreleased}\n${released}`;

  write(CHANGELOG, text.slice(0, idx) + replacement + text.slice(end));
}

function today() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function main() {
  const args = process.argv.slice(2);

  if (args.includes("--current")) {
    process.stdout.write(currentVersion() + "\n");
    return;
  }

  const version = args.find((a) => !a.startsWith("--"));
  if (!version) {
    throw new Error("Informe a nova versão, ex.: node bump_version.mjs 0.2.0 --changelog");
  }
  if (!SEMVER.test(version)) {
    throw new Error(`Versão inválida: "${version}". Use MAJOR.MINOR.PATCH[-pre].`);
  }

  const dateFlag = args.indexOf("--date");
  const date = dateFlag !== -1 ? args[dateFlag + 1] : today();

  for (const f of [TAURI_CONF, PACKAGE_JSON, CARGO_TOML, CARGO_LOCK, CHANGELOG]) {
    if (!existsSync(rel(f))) throw new Error(`Arquivo esperado ausente: ${f}`);
  }

  bumpVersionFiles(version);

  if (args.includes("--changelog")) {
    finalizeChangelog(version, date);
  }

  process.stdout.write(
    `Versão atualizada para ${version} em package.json, tauri.conf.json, Cargo.toml e Cargo.lock` +
      (args.includes("--changelog") ? ` e CHANGELOG.md (${date}).\n` : ".\n"),
  );
}

try {
  main();
} catch (err) {
  process.stderr.write(`erro: ${err.message}\n`);
  process.exit(1);
}
