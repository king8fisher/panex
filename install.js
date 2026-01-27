#!/usr/bin/env node

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

  if (!PLATFORMS[platform]) {
    console.warn(`Unsupported platform: ${platform}`);
    return null;
  }

  if (!PLATFORMS[platform][arch]) {
    console.warn(`Unsupported architecture: ${arch} on ${platform}`);
    return null;
  }

  return PLATFORMS[platform][arch];
}

function findBinary() {
  const binaryName = process.platform === "win32" ? "panex.exe" : "panex";
  const packageName = getPlatformPackage();

  // Check for local development binary first
  const localPaths = [
    path.join(__dirname, "panex-rs", "target", "release", binaryName),
    path.join(__dirname, "panex-rs", "target", "debug", binaryName),
  ];

  for (const p of localPaths) {
    if (fs.existsSync(p)) {
      console.log(`Using local development binary: ${p}`);
      return p;
    }
  }

  // Check for platform package
  if (!packageName) {
    return null;
  }

  const paths = [
    path.join(__dirname, "..", packageName, binaryName),
    path.join(__dirname, "node_modules", packageName, binaryName),
  ];

  for (const p of paths) {
    if (fs.existsSync(p)) {
      return p;
    }
  }

  return null;
}

const binaryPath = findBinary();

if (binaryPath) {
  // Make binary executable on Unix
  if (process.platform !== "win32") {
    try {
      fs.chmodSync(binaryPath, 0o755);
    } catch (e) {
      // Ignore chmod errors
    }
  }
  console.log(`Panex ready: ${binaryPath}`);
} else {
  console.warn("Panex binary not found. For development, run: cargo build --release -p panex");
}
