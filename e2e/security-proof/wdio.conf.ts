import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { download as downloadEdgeDriver } from "edgedriver";

const repositoryRoot = resolve(import.meta.dirname, "../..");
const reportPath = resolve(repositoryRoot, ".artifacts/security-proof/wdio/results.jsonl");
const applicationPath = resolve(
  repositoryRoot,
  "src-tauri/target/release/secrets-storage.exe",
);
const forbiddenReportMarkers = ["SECURITY_PROOF_E2E_CANARY", "SECURITY_PROOF_XSS_CANARY"];
const executableSuffix = process.platform === "win32" ? ".exe" : "";

function findTauriDriver() {
  const cargoHome =
    process.env.CARGO_HOME ?? join(process.env.USERPROFILE ?? "", ".cargo");
  const installedPath = join(cargoHome, "bin", `tauri-driver${executableSuffix}`);

  if (!existsSync(installedPath)) {
    execFileSync("cargo", ["install", "tauri-driver", "--locked"], {
      stdio: "inherit",
    });
  }

  const safeDirectory = join(tmpdir(), "security-proof-wdio");
  const safePath = join(safeDirectory, `tauri-driver${executableSuffix}`);
  mkdirSync(safeDirectory, { recursive: true });
  if (!existsSync(safePath)) copyFileSync(installedPath, safePath);
  return safePath;
}

const tauriDriverPath = findTauriDriver();
const nativeDriverPath =
  process.platform === "win32" ? await downloadEdgeDriver() : undefined;
const tauriDriverPort = 4444;
const nativeDriverPort = 4445;
let tauriDriverProcess: ChildProcess | undefined;

async function waitForPort(port: number, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ready = await new Promise<boolean>((resolveReady) => {
      const socket = connect({ host: "127.0.0.1", port });
      socket.once("connect", () => {
        socket.destroy();
        resolveReady(true);
      });
      socket.once("error", () => resolveReady(false));
    });
    if (ready) return;
    await new Promise((resolveWait) => setTimeout(resolveWait, 100));
  }
  throw new Error("tauri-driver did not become ready before the timeout");
}

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: [resolve(import.meta.dirname, "**/*.e2e.ts")],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": {
        application: applicationPath,
      },
    },
  ],
  hostname: "127.0.0.1",
  port: tauriDriverPort,
  path: "/",
  framework: "mocha",
  reporters: ["spec"],
  logLevel: "warn",
  outputDir: resolve(repositoryRoot, ".artifacts/security-proof/wdio/raw"),
  waitforTimeout: 10_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 0,
  mochaOpts: {
    timeout: 60_000,
  },
  async onPrepare() {
    mkdirSync(dirname(reportPath), { recursive: true });
    writeFileSync(
      reportPath,
      `${JSON.stringify({ schema_version: 1, suite: "security-proof-authority" })}\n`,
      "utf8",
    );
    const args = [
      "--port",
      String(tauriDriverPort),
      "--native-port",
      String(nativeDriverPort),
    ];
    if (nativeDriverPath) args.push("--native-driver", nativeDriverPath);
    tauriDriverProcess = spawn(tauriDriverPath, args, {
      shell: false,
      stdio: ["ignore", "ignore", "ignore"],
    });
    await waitForPort(tauriDriverPort, 30_000);
  },
  afterTest(test, _context, result) {
    writeFileSync(
      reportPath,
      `${JSON.stringify({
        title: test.title,
        result: result.passed ? "pass" : "fail",
      })}\n`,
      { encoding: "utf8", flag: "a" },
    );
  },
  onComplete() {
    try {
      const report = readFileSync(reportPath, "utf8");
      for (const marker of forbiddenReportMarkers) {
        if (report.includes(marker)) {
          throw new Error("sanitized WebDriver report contains forbidden proof material");
        }
      }
    } finally {
      tauriDriverProcess?.kill();
    }
  },
};
