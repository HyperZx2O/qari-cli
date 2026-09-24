const test = require("node:test");
const assert = require("node:assert/strict");
const { assertDownloadUrl, expectedChecksum, releaseBaseUrl } = require("./install.js");

test("expectedChecksum only returns the packaged archive hash", () => {
  assert.match(expectedChecksum("qari-x86_64-unknown-linux-gnu.tar.gz"), /^[a-f0-9]{64}$/);
  assert.throws(() => expectedChecksum("other.tar.gz"));
  assert.throws(() => expectedChecksum("../outside.tar.gz"));
});

test("release URL is immutable and versioned", () => {
  assert.equal(releaseBaseUrl(), "https://github.com/HyperZx2O/qari-cli/releases/download/v0.1.2");
});

test("download URLs must use HTTPS and an allowed host", () => {
  assert.equal(
    assertDownloadUrl("https://github.com/HyperZx2O/qari-cli/releases/download/v0.1.0/qari").hostname,
    "github.com"
  );
  assert.throws(() => assertDownloadUrl("http://github.com/example"));
  assert.throws(() => assertDownloadUrl("https://evil.example/qari"));
});
