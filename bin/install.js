#!/usr/bin/env node
const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryName = process.platform === 'win32' ? 'pnw.exe' : 'pnw';
const dest = path.join(__dirname, binaryName);

// 检查是否已有二进制文件
if (fs.existsSync(dest)) {
  console.log(`✓ pnw binary found at ${dest}`);
  process.exit(0);
}

// 检查 cargo 构建产物
const cargoPaths = [
  path.join(__dirname, '..', 'target', 'release', binaryName),
  path.join(__dirname, '..', 'target', 'debug', binaryName),
];

for (const p of cargoPaths) {
  if (fs.existsSync(p)) {
    fs.copyFileSync(p, dest);
    console.log(`✓ pnw binary copied from ${p}`);
    process.exit(0);
  }
}

console.log(`
╔══════════════════════════════════════════════════╗
║ PrivateNovelWriter - npm package                ║
╠══════════════════════════════════════════════════╣
║ Prebuilt binary not found.                      ║
║                                                 ║
║ To build from source:                           ║
║   npm run build                                 ║
║   or                                            ║
║   cargo build -p pnw --release                  ║
╚══════════════════════════════════════════════════╝
`);
