#!/usr/bin/env node

/**
 * RAPS CLI - Post-install verification script
 *
 * Verifies that the platform-specific binary was installed correctly.
 */

const { execSync } = require('child_process');
const path = require('path');

// Platform to package mapping
const PLATFORMS = {
  'win32-x64': '@anthropic-ai/raps-cli-win32-x64',
  'darwin-x64': '@anthropic-ai/raps-cli-darwin-x64',
  'darwin-arm64': '@anthropic-ai/raps-cli-darwin-arm64',
  'linux-x64': '@anthropic-ai/raps-cli-linux-x64',
  'linux-arm64': '@anthropic-ai/raps-cli-linux-arm64',
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  const archMap = { 'x64': 'x64', 'arm64': 'arm64' };
  const mappedArch = archMap[arch];
  return mappedArch ? `${platform}-${mappedArch}` : null;
}

function main() {
  const platformKey = getPlatformKey();
  const packageName = PLATFORMS[platformKey];

  if (!packageName) {
    console.warn(`Warning: RAPS does not support your platform (${process.platform}-${process.arch})`);
    console.warn('The CLI may not work correctly.');
    return;
  }

  try {
    // Try to require the platform package
    const platformModule = require(packageName);
    const binaryPath = platformModule.binaryPath;

    if (!binaryPath) {
      console.warn('Warning: Platform package found but binary path not exported.');
      return;
    }

    // Try to run --version to verify binary works
    try {
      const version = execSync(`"${binaryPath}" --version`, {
        encoding: 'utf-8',
        timeout: 5000,
      }).trim();
      console.log(`RAPS installed successfully: ${version}`);
    } catch (e) {
      console.warn('Warning: Binary found but failed to execute.');
      console.warn('You may need to install additional dependencies for your platform.');
    }
  } catch (e) {
    // Optional dependency not installed - this is normal for unsupported platforms
    console.warn(`Note: Platform binary for ${platformKey} not available.`);
    console.warn('This is expected if npm skipped optional dependencies.');
  }
}

main();
