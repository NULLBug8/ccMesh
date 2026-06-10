#!/usr/bin/env node
// 统一版本管理脚本（跨平台）
// 用法: node scripts/update-version.mjs <new_version>
// 示例: node scripts/update-version.mjs 0.5.0

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const newVersion = process.argv[2];

if (!newVersion) {
  console.error('用法: node scripts/update-version.mjs <new_version>');
  console.error('示例: node scripts/update-version.mjs 0.5.0');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(newVersion)) {
  console.error(`✗ 非法版本号: ${newVersion}（应形如 0.5.0 或 0.5.0-beta.1）`);
  process.exit(1);
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, '..');

console.log(`更新版本到 ${newVersion}...`);

/**
 * 读取文件，用正则替换后写回（保留原有格式）。
 * @param {string} relPath 相对项目根目录的路径
 * @param {RegExp} pattern 必须含一个捕获组用于拼接前缀
 * @param {string} replacement 替换字符串
 */
function patchFile(relPath, pattern, replacement) {
  const filePath = resolve(projectRoot, relPath);
  const content = readFileSync(filePath, 'utf8');
  if (!pattern.test(content)) {
    console.error(`✗ ${relPath}: 未匹配到 version 字段`);
    process.exit(1);
  }
  writeFileSync(filePath, content.replace(pattern, replacement), 'utf8');
  console.log(`✓ ${relPath}`);
}

// package.json —— 顶层 "version"（只替换首个，避免命中依赖项）
patchFile(
  'package.json',
  /("version"\s*:\s*")[^"]*(")/,
  `$1${newVersion}$2`,
);

// src-tauri/Cargo.toml —— [package] 下的首个 version
patchFile(
  'src-tauri/Cargo.toml',
  /^(version\s*=\s*")[^"]*(")/m,
  `$1${newVersion}$2`,
);

// src-tauri/tauri.conf.json —— 顶层 "version"
patchFile(
  'src-tauri/tauri.conf.json',
  /("version"\s*:\s*")[^"]*(")/,
  `$1${newVersion}$2`,
);

console.log('');
console.log(`版本已更新到 ${newVersion}`);
