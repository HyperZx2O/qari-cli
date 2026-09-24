#!/usr/bin/env node

const crypto = require("crypto");
const fs = require("fs");
const https = require("https");
const os = require("os");
const path = require("path");
const { execFileSync, spawnSync } = require("child_process");

const REPO = "HyperZx2O/qari-cli";
const version = require("./package.json").version;
const checksums = require("./checksums.json");
const releaseTag = `v${version}`;
const MAX_DOWNLOAD_BYTES = 128 * 1024 * 1024;
const REQUEST_TIMEOUT_MS = 30_000;
const ALLOWED_HOSTS = new Set([
  "github.com",
  "release-assets.githubusercontent.com",
  "objects.githubusercontent.com",
]);
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
  return `https://github.com/${REPO}/releases/download/${releaseTag}`;
}

function assertDownloadUrl(value) {
  const url = new URL(value);
  if (url.protocol !== "https:") {
    throw new Error(`Refusing non-HTTPS download URL: ${url.protocol}`);
  }
  if (!ALLOWED_HOSTS.has(url.hostname)) {
    throw new Error(`Refusing download from untrusted host: ${url.hostname}`);
  }
  return url;
}

function download(url, destination, redirects = 5, maxBytes = MAX_DOWNLOAD_BYTES) {
  const parsed = assertDownloadUrl(url);
  return new Promise((resolve, reject) => {
    let settled = false;
    const fail = (error) => {
      if (settled) return;
      settled = true;
      try {
        fs.rmSync(destination, { force: true });
      } catch {}
      reject(error instanceof Error ? error : new Error(String(error)));
    };
    const succeed = () => {
      if (settled) return;
      settled = true;
      resolve();
    };
    const request = (current, left) => {
      let target;
      try {
        target = assertDownloadUrl(current);
      } catch (error) {
        fail(error);
        return;
      }
      const req = https.get(
        target,
        { headers: { "User-Agent": "qari-cli-npm" }, timeout: REQUEST_TIMEOUT_MS },
        (response) => {
          if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
            if (left <= 0) {
              response.resume();
              fail(new Error("Too many redirects"));
              return;
            }
            response.resume();
            let next;
            try {
              next = new URL(response.headers.location, target).toString();
            } catch (error) {
              fail(error);
              return;
            }
            request(next, left - 1);
            return;
          }
          if (response.statusCode !== 200) {
            response.resume();
            fail(new Error(`Download failed: HTTP ${response.statusCode} for ${target}`));
            return;
          }

          const declaredLength = Number(response.headers["content-length"] || 0);
          if (Number.isFinite(declaredLength) && declaredLength > maxBytes) {
            response.resume();
            fail(new Error(`Download exceeds the ${maxBytes}-byte limit`));
            return;
          }

          const file = fs.createWriteStream(destination, { flags: "w" });
          let received = 0;
          response.on("data", (chunk) => {
            received += chunk.length;
            if (received > maxBytes) {
              const error = new Error(`Download exceeds the ${maxBytes}-byte limit`);
              response.destroy(error);
              file.destroy(error);
            }
          });
          response.on("error", fail);
          file.on("error", fail);
          file.on("finish", () => {
            file.close((error) => (error ? fail(error) : succeed()));
          });
          response.pipe(file);
        }
      );
      req.on("timeout", () => req.destroy(new Error("Download request timed out")));
      req.on("error", fail);
    };
    request(parsed.toString(), redirects);
  });
}

async function downloadWithRetry(url, destination, attempts = 3, maxBytes = MAX_DOWNLOAD_BYTES) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await download(url, destination, 5, maxBytes);
      return;
    } catch (error) {
      lastError = error;
      try {
        fs.rmSync(destination, { force: true });
      } catch {}
      if (attempt < attempts) {
        await new Promise((resolve) => setTimeout(resolve, 500 * attempt));
      }
    }
  }
  throw lastError;
}

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash("sha256");
    fs.createReadStream(filePath)
      .on("data", (data) => hash.update(data))
      .on("end", () => resolve(hash.digest("hex")))
      .on("error", reject);
  });
}

function expectedChecksum(archiveName) {
  const expected = checksums[archiveName];
  if (typeof expected !== "string" || !/^[a-f0-9]{64}$/.test(expected)) {
    throw new Error(`No trusted checksum for ${archiveName}`);
  }
  return expected;
}

async function verifyChecksum(archivePath, archiveName) {
  const expected = expectedChecksum(archiveName);
  const actual = await sha256File(archivePath);
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${archiveName}: expected ${expected}, got ${actual}`);
  }
}

function expectedBinaryName() {
  return os.platform() === "win32" ? "qari.exe" : "qari";
}

function archiveMembers(archive, extension) {
  const args = extension === "zip" ? ["-tf", archive] : ["-tzf", archive];
  let output;
  try {
    output = execFileSync("tar", args, {
      encoding: "utf8",
      maxBuffer: 1024 * 1024,
      stdio: ["ignore", "pipe", "inherit"],
    });
  } catch (error) {
    if (extension === "zip" && os.platform() === "win32") return;
    throw error;
  }
  const members = output
    .split(/\r?\n/)
    .map((member) => member.trim())
    .filter(Boolean);
  const expected = expectedBinaryName();
  if (members.length !== 1 || members[0] !== expected) {
    throw new Error(`Archive contains unexpected members: ${members.join(", ")}`);
  }
}

function extract(archive, binDir, extension) {
  archiveMembers(archive, extension);
  try {
    const args = extension === "zip" ? ["-xf", archive, "-C", binDir] : ["-xzf", archive, "-C", binDir];
    if (os.platform() !== "win32") args.push("--no-same-owner", "--no-same-permissions");
    execFileSync("tar", args, { stdio: "inherit" });
    return;
  } catch (error) {
    if (os.platform() !== "win32" || extension !== "zip") {
      throw new Error(
        `'tar' extraction failed (${error.message}). Install bsdtar/GNU tar, or manually download from https://github.com/${REPO}/releases`
      );
    }
  }

  const env = {
    ...process.env,
    QARI_ARCHIVE: archive,
    QARI_BIN_DIR: binDir,
    QARI_BINARY: expectedBinaryName(),
  };
  const script = [
    "Add-Type -AssemblyName System.IO.Compression.FileSystem",
    "$archive = [IO.Compression.ZipFile]::OpenRead($env:QARI_ARCHIVE)",
    "try {",
    "  $entries = @($archive.Entries | Where-Object { $_.FullName })",
    "  if ($entries.Count -ne 1 -or $entries[0].FullName -ne $env:QARI_BINARY) { throw 'Archive contains unexpected members' }",
    "} finally { $archive.Dispose() }",
    "Expand-Archive -Force -LiteralPath $env:QARI_ARCHIVE -DestinationPath $env:QARI_BIN_DIR",
  ].join("\n");
  execFileSync(
    "powershell",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { stdio: "inherit", env }
  );
}

async function install() {
  const [target, extension] = resolveTarget();
  const binDir = path.join(__dirname, "bin");
  fs.mkdirSync(binDir, { recursive: true });
  const staging = fs.mkdtempSync(path.join(binDir, ".staging-"));
  const archiveName = `qari-${target}.${extension}`;
  const archive = path.join(staging, archiveName);
  const stagedBinary = path.join(staging, expectedBinaryName());
  const binary = path.join(binDir, expectedBinaryName());

  try {
    const url = `${releaseBaseUrl()}/${archiveName}`;
    console.log(`Installing qari-cli v${version} for ${target}...`);
    await downloadWithRetry(url, archive);
    await verifyChecksum(archive, archiveName);
    extract(archive, staging, extension);
    const extractedMembers = fs
      .readdirSync(staging)
      .filter((member) => member !== archiveName);
    if (extractedMembers.length !== 1 || extractedMembers[0] !== expectedBinaryName()) {
      throw new Error(`Archive contains unexpected extracted members: ${extractedMembers.join(", ")}`);
    }
    if (!fs.existsSync(stagedBinary)) {
      throw new Error(`Install finished but ${stagedBinary} is missing`);
    }
    const stagedMetadata = fs.lstatSync(stagedBinary);
    if (!stagedMetadata.isFile() || stagedMetadata.isSymbolicLink()) {
      throw new Error("The extracted qari binary is not a regular file");
    }
    if (os.platform() !== "win32") fs.chmodSync(stagedBinary, 0o755);
    try {
      const metadata = fs.lstatSync(binary);
      if (metadata.isSymbolicLink()) throw new Error("Refusing to replace a symlinked qari binary");
      fs.rmSync(binary, { force: true });
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    fs.renameSync(stagedBinary, binary);
  } finally {
    fs.rmSync(staging, { recursive: true, force: true });
  }
}

function isPostinstall() {
  const event = process.env.npm_lifecycle_event || process.env.BUN_LIFECYCLE_EVENT || "";
  if (["postinstall", "install", "preinstall"].includes(event)) return true;
  try {
    const argv = JSON.parse(process.env.npm_config_argv || "{}");
    if (argv.cooked && argv.cooked.includes("postinstall")) return true;
  } catch {}
  return false;
}

async function main() {
  if (isPostinstall() || process.argv.includes("--postinstall")) {
    await install();
    return;
  }
  try {
    resolveTarget();
  } catch (error) {
    console.error(`qari-cli: ${error.message}`);
    process.exitCode = 1;
    return;
  }
  const binDir = path.join(__dirname, "bin");
  const binary = path.join(binDir, expectedBinaryName());
  if (!fs.existsSync(binary)) await install();
  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`qari-cli: ${error.message}`);
    process.exitCode = 1;
  });
}

module.exports = {
  assertDownloadUrl,
  expectedChecksum,
  isPostinstall,
  resolveTarget,
  releaseBaseUrl,
};
