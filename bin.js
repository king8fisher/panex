#!/usr/bin/env node

const { spawn } = require("child_process");
const fs = require("fs");
const path = require("path");

const PLATFORMS = {
  darwin: {
    arm64: "@king8fisher/panex-darwin-arm64",
    x64: "@king8fisher/panex-darwin-x64",
  },
  linux: {
    arm64: "@king8fisher/panex-linux-arm64",
    x64: "@king8fisher/panex-linux-x64",
  },
  win32: {
    arm64: "@king8fisher/panex-win32-arm64",
    x64: "@king8fisher/panex-win32-x64",
  },
};

function getPlatformPackage() {
  const platform = process.platform;
  const arch = process.arch;

  if (!PLATFORMS[platform] || !PLATFORMS[platform][arch]) {
    return null;
  }

  return PLATFORMS[platform][arch];
}

function findBinary() {
  const binaryName = process.platform === "win32" ? "panex.exe" : "panex";
  const packageName = getPlatformPackage();

  const paths = [
    // Local development (release)
    path.join(__dirname, "panex-rs", "target", "release", binaryName),
    // Local development (debug)
    path.join(__dirname, "panex-rs", "target", "debug", binaryName),
  ];

  // Add platform package paths if available
  if (packageName) {
    paths.push(
      // When installed as a dependency
      path.join(__dirname, "..", packageName, binaryName),
      // When installed globally
      path.join(__dirname, "node_modules", packageName, binaryName),
    );
  }

  for (const p of paths) {
    if (fs.existsSync(p)) {
      return p;
    }
  }

  return null;
}

const binaryPath = findBinary();

// Ensure binary is executable on Unix
if (binaryPath && process.platform !== "win32") {
  try {
    fs.chmodSync(binaryPath, 0o755);
  } catch (e) {
    // Ignore chmod errors
  }
}

if (!binaryPath) {
  console.error("Could not find panex binary.");
  console.error("For development: cargo build --release in panex-rs/");
  console.error("For production: platform package should be installed");
  process.exit(1);
}

const args = process.argv.slice(2);

const child = spawn(binaryPath, args, {
  stdio: "inherit",
  shell: false,
});

child.on("error", (err) => {
  console.error(`Failed to start panex: ${err.message}`);
  process.exit(1);
});

child.on("close", (code) => {
  process.exit(code ?? 0);
});
