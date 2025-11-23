#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const VERSION = '0.1.0';
const REPO = 'globalbusinessadvisors/llm-schema-registry';

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin') {
    return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  } else if (platform === 'linux') {
    return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  } else if (platform === 'win32') {
    return 'x86_64-pc-windows-msvc';
  }

  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`Downloading from ${url}...`);
    const file = fs.createWriteStream(dest);

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        return download(response.headers.location, dest).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function install() {
  try {
    const binDir = path.join(__dirname, 'bin');

    // Create bin directory if it doesn't exist
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    const platform = getPlatform();
    const ext = process.platform === 'win32' ? '.exe' : '';
    const binaryName = `llm-schema-cli${ext}`;
    const binaryPath = path.join(binDir, binaryName);

    // Try to install from GitHub releases
    const releaseUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/${binaryName}-${platform}${ext}`;

    console.log(`Installing LLM Schema Registry CLI v${VERSION} for ${platform}...`);

    try {
      await download(releaseUrl, binaryPath);
      fs.chmodSync(binaryPath, 0o755);
      console.log('✅ Installation complete!');
      console.log(`Binary installed at: ${binaryPath}`);
    } catch (error) {
      console.warn('⚠️  Could not download pre-built binary from GitHub releases.');
      console.log('This is expected if the release is not yet published.');
      console.log('You can build from source by running: cargo install llm-schema-cli');

      // Check if cargo is available
      try {
        execSync('cargo --version', { stdio: 'ignore' });
        console.log('\n📦 Attempting to build from source with cargo...');
        execSync('cargo install --git https://github.com/globalbusinessadvisors/llm-schema-registry llm-schema-cli', {
          stdio: 'inherit'
        });
        console.log('✅ Built from source successfully!');
      } catch (cargoError) {
        console.error('\n❌ Installation failed.');
        console.error('Please install Rust and Cargo from https://rustup.rs/');
        console.error('Then run: cargo install llm-schema-cli');
        process.exit(1);
      }
    }
  } catch (error) {
    console.error('❌ Installation failed:', error.message);
    process.exit(1);
  }
}

install();
