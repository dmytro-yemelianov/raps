#!/usr/bin/env node

/**
 * RAPS CLI - Platform-specific binary executor
 *
 * This script detects the current platform and executes the appropriate
 * pre-compiled RAPS binary, passing through all command-line arguments.
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Platform to package mapping
const PLATFORMS = {
  'win32-x64': '@dmytro-yemelianov/raps-cli-win32-x64',
  'darwin-x64': '@dmytro-yemelianov/raps-cli-darwin-x64',
  'darwin-arm64': '@dmytro-yemelianov/raps-cli-darwin-arm64',
  'linux-x64': '@dmytro-yemelianov/raps-cli-linux-x64',
  'linux-arm64': '@dmytro-yemelianov/raps-cli-linux-arm64',
};

// Get current platform identifier
function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;

  // Map Node.js arch names to our naming convention
  const archMap = {
    'x64': 'x64',
    'arm64': 'arm64',
    'ia32': 'x86',
  };

  const mappedArch = archMap[arch];
  if (!mappedArch) {
    return null;
  }

  return `${platform}-${mappedArch}`;
}

// Find the binary path for the current platform
function getBinaryPath() {
  const platformKey = getPlatformKey();

  if (!platformKey) {
    console.error(`Error: Unsupported architecture: ${process.arch}`);
    console.error('Supported architectures: x64, arm64');
    process.exit(1);
  }

  const packageName = PLATFORMS[platformKey];

  if (!packageName) {
    console.error(`Error: Unsupported platform: ${platformKey}`);
    console.error(`Supported platforms: ${Object.keys(PLATFORMS).join(', ')}`);
    process.exit(1);
  }

  // Try to find the platform package
  let packagePath;
  try {
    packagePath = require.resolve(packageName);
  } catch (e) {
    console.error(`Error: Platform package not installed: ${packageName}`);
    console.error('');
    console.error('This usually means your platform is not supported or the');
    console.error('optional dependency failed to install.');
    console.error('');
    console.error('Try reinstalling with:');
    console.error('  npm install -g @dmytro-yemelianov/raps-cli');
    console.error('');
    console.error('Or install RAPS directly:');
    console.error('  curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash');
    process.exit(1);
  }

  // The platform package exports the binary path
  const platformModule = require(packageName);
  const binaryPath = platformModule.binaryPath;

  if (!binaryPath || !fs.existsSync(binaryPath)) {
    console.error(`Error: Binary not found at expected path: ${binaryPath}`);
    process.exit(1);
  }

  return binaryPath;
}

// Execute the binary with all arguments
function main() {
  const binaryPath = getBinaryPath();
  const args = process.argv.slice(2);

  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    shell: false,
  });

  child.on('error', (err) => {
    console.error(`Error executing RAPS: ${err.message}`);
    process.exit(1);
  });

  child.on('close', (code) => {
    process.exit(code || 0);
  });
}

main();
