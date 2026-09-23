#!/usr/bin/env node

const crypto = require("crypto");
const fs = require("fs");
const https = require("https");
const os = require("os");
const path = require("path");
const { execFileSync, spawnSync } = require("child_process");

const REPO = "HyperZx2O/qari-cli";
const version = require("./package.json").version;
const platforms = {
  "darwin-x64": ["x86_64-apple-darwin", "tar.gz"],
  "darwin-arm64": ["aarch64-apple-darwin", "tar.gz"],
  "linux-x64": ["x86_64-unknown-linux-gnu", "tar.gz"],
  "linux-arm64": ["aarch64-unknown-linux-gnu", "tar.gz"],
  "win32-x64": ["x86_64-pc-windows-msvc", "zip"],
  "win32-arm64": ["aarch64-pc-windows-msvc", "zip"],
};

function resolveTarget() {
  const key = `${os.platform()}-${os.arch()}`;
  const entry = platforms[key];
  if (!entry) {
    const supported = Object.keys(platforms).join(", ");
    throw new Error(
      `Unsupported platform: ${key}. Supported: ${supported}. ` +
        `Download a binary manually from https://github.com/${REPO}/releases`
    );
  }
  return entry;
}

function releaseBaseUrl() {
  if (process.env.QARI_RELEASE_BASE_URL) {
    return process.env.QARI_RELEASE_BASE_URL.replace(/\/$/, "");
  }
  return `https://github.com/${REPO}/releases/download/v${version}`;
}

function download(url, destination, redirects = 5) {
  const headers = { "User-Agent": "qari-cli-npm" };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  return new Promise((resolve, reject) => {
    const request = (current, left) =>
      https.get(current, { headers }, (response) => {
        if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          if (left <= 0) return reject(new Error("Too many redirects"));
          response.resume();
          return request(response.headers.location, left - 1);
        }
        if (response.statusCode !== 200) {
          response.resume();
          return reject(new Error(`Download failed: HTTP ${response.statusCode} for ${current}`));
        }
        const file = fs.createWriteStream(destination);
        response.pipe(file);
        file.on("finish", () => file.close(resolve));
        file.on("error", reject);
      }).on("error", reject);
    request(url, redirects);
  });
}

async function downloadWithRetry(url, destination, attempts = 3) {
  let lastError;
  for (let i = 1; i <= attempts; i++) {
    try {
      await download(url, destination);
      return;
    } catch (error) {
      lastError = error;
      if (i < attempts) await new Promise((r) => setTimeout(r, 500 * i));
    }
  }
  throw lastError;
}

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash("sha256");
    fs.createReadStream(filePath).on("data", (d) => hash.update(d)).on("end", () => resolve(hash.digest("hex"))).on("error", reject);
  });
}

async function verifyChecksum(archivePath, archiveName) {
  try {
    const sumsUrl = `${releaseBaseUrl()}/SHA256SUMS.txt`;
    const sumsPath = `${archivePath}.SHA256SUMS.txt`;
    await downloadWithRetry(sumsUrl, sumsPath, 2);
    const sums = fs.readFileSync(sumsPath, "utf8");
    fs.unlinkSync(sumsPath);
    const line = sums.split("\n").find((l) => l.trim().endsWith(archiveName));
    if (!line) {
      console.warn(`qari-cli: no checksum entry for ${archiveName}, skipping verification`);
      return;
    }
    const expected = line.split(/\s+/)[0];
    const actual = await sha256File(archivePath);
    if (actual !== expected.toLowerCase()) {
      throw new Error(`Checksum mismatch for ${archiveName}: expected ${expected}, got ${actual}`);
    }
  } catch (error) {
    if (/Checksum mismatch/.test(error.message)) throw error;
    console.warn(`qari-cli: checksum verification skipped (${error.message})`);
  }
}

function extract(archive, binDir, extension) {
  if (extension === "zip" && os.platform() === "win32") {
    try {
      execFileSync("tar", ["-xf", archive, "-C", binDir], { stdio: "inherit" });
      return;
    } catch {
      execFileSync(
        "powershell",
        ["-NoProfile", "-Command", `Expand-Archive -Force '${archive}' -DestinationPath '${binDir}'`],
        { stdio: "inherit" }
      );
      return;
    }
  }
  try {
    execFileSync("tar", [extension === "zip" ? "-xf" : "-xzf", archive, "-C", binDir], { stdio: "inherit" });
  } catch (error) {
    throw new Error(
      `'tar' extraction failed (${error.message}). Install bsdtar/GNU tar, or manually download from https://github.com/${REPO}/releases`
    );
  }
}

async function install() {
  const [target, extension] = resolveTarget();
  const binDir = path.join(__dirname, "bin");
  const binary = path.join(binDir, os.platform() === "win32" ? "qari.exe" : "qari");
  fs.mkdirSync(binDir, { recursive: true });
  const archiveName = `qari-${target}.${extension}`;
  const archive = path.join(binDir, archiveName);
  const url = `${releaseBaseUrl()}/${archiveName}`;
  console.log(`Installing qari-cli v${version} for ${target}...`);
  await downloadWithRetry(url, archive);
  await verifyChecksum(archive, archiveName);
  extract(archive, binDir, extension);
  fs.unlinkSync(archive);
  if (os.platform() !== "win32") fs.chmodSync(binary, 0o755);
  if (!fs.existsSync(binary)) throw new Error(`Install finished but ${binary} is missing`);
}

function isPostinstall() {
  const event = process.env.npm_lifecycle_event || process.env.BUN_LIFECYCLE_EVENT || "";
  if (["postinstall", "install", "preinstall"].includes(event)) return true;
  // pnpm/yarn set npm_config_argv with the lifecycle event.
  try {
    const argv = JSON.parse(process.env.npm_config_argv || "{}");
    if (argv.cooked && argv.cooked.includes("postinstall")) return true;
  } catch {}
  return false;
}

(async () => {
  if (isPostinstall() || process.argv.includes("--postinstall")) {
    try {
      await install();
    } catch (error) {
      // Never break `npm install` for an optional binary fetch; proxy will retry on first run.
      console.error(`qari-cli postinstall skipped: ${error.message}`);
    }
    return;
  }
  try {
    resolveTarget();
  } catch (error) {
    console.error(`qari-cli: ${error.message}`);
    process.exit(1);
  }
  const binDir = path.join(__dirname, "bin");
  const binary = path.join(binDir, os.platform() === "win32" ? "qari.exe" : "qari");
  if (!fs.existsSync(binary)) await install();
  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
  process.exit(result.status ?? 1);
})().catch((error) => {
  console.error(`qari-cli: ${error.message}`);
  process.exit(1);
});
