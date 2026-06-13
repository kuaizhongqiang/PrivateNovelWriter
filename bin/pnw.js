#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// 查找二进制文件
function findBinary() {
  // 1. 同目录下的预编译二进制
  const dir = __dirname;
  const binaryName = process.platform === 'win32' ? 'pnw.exe' : 'pnw';
  const localPath = path.join(dir, binaryName);
  if (fs.existsSync(localPath)) return localPath;

  // 2. cargo build --release 的产物
  const cargoPath = path.join(dir, '..', 'target', 'release', binaryName);
  if (fs.existsSync(cargoPath)) return cargoPath;

  // 3. cargo build (debug) 的产物
  const cargoDebugPath = path.join(dir, '..', 'target', 'debug', binaryName);
  if (fs.existsSync(cargoDebugPath)) return cargoDebugPath;

  // 4. 系统 PATH
  return binaryName;
}

const binary = findBinary();
const args = process.argv.slice(2);

const child = spawn(binary, args, {
  stdio: 'inherit',
  env: { ...process.env },
});

child.on('exit', (code) => {
  process.exit(code ?? 1);
});
