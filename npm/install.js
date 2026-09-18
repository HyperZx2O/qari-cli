#!/usr/bin/env node

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
  "win32-x64": ["x86_64-pc-windows-msvc", "zip"]
};

const entry = platforms[`${os.platform()}-${os.arch()}`];
if (!entry) throw new Error(`Unsupported platform: ${os.platform()}-${os.arch()}`);
const [target, extension] = entry;
const binDir = path.join(__dirname, "bin");
const binary = path.join(binDir, os.platform() === "win32" ? "qari.exe" : "qari");

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const request = current => https.get(current, { headers: { "User-Agent": "qari-cli-npm" } }, response => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) return request(response.headers.location);
      if (response.statusCode !== 200) return reject(new Error(`Download failed: HTTP ${response.statusCode}`));
      const file = fs.createWriteStream(destination);
      response.pipe(file);
      file.on("close", resolve);
      file.on("error", reject);
    }).on("error", reject);
    request(url);
  });
}

async function install() {
  fs.mkdirSync(binDir, { recursive: true });
  const archive = path.join(binDir, `qari.${extension}`);
  const url = `https://github.com/${REPO}/releases/download/v${version}/qari-${target}.${extension}`;
  console.log(`Installing qari-cli v${version} for ${target}...`);
  await download(url, archive);
  execFileSync("tar", [extension === "zip" ? "-xf" : "-xzf", archive, "-C", binDir]);
  fs.unlinkSync(archive);
  if (os.platform() !== "win32") fs.chmodSync(binary, 0o755);
}

(async () => {
  if (process.env.npm_lifecycle_event === "postinstall") {
    await install();
    return;
  }
  if (!fs.existsSync(binary)) await install();
  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
  process.exit(result.status ?? 1);
})().catch(error => {
  console.error(`qari-cli: ${error.message}`);
  process.exit(1);
});
