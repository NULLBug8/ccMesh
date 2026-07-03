#!/usr/bin/env node
// 统一版本管理脚本（跨平台）
// 用法: node scripts/update-version.mjs [new_version | patch | minor | major]
// 示例:
//   node scripts/update-version.mjs 0.5.0   指定版本号
//   node scripts/update-version.mjs patch   补丁号 +1（不传参数时的默认行为）
//   node scripts/update-version.mjs minor    次版本号 +1，补丁号归零
//   node scripts/update-version.mjs major    主版本号 +1，其余归零
//   node scripts/update-version.mjs          等价于 patch，读取 package.json 自增

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, '..');

const VERSION_RE = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const BUMP_TYPES = ['major', 'minor', 'patch'];

/** 从 package.json 读取当前版本号。 */
function readCurrentVersion() {
  const pkgPath = resolve(projectRoot, 'package.json');
  const match = readFileSync(pkgPath, 'utf8').match(/"version"\s*:\s*"([^"]*)"/);
  if (!match) {
    console.error('✗ package.json: 未找到 version 字段');
    process.exit(1);
  }
  return match[1];
}

/**
 * 按 bump 类型对当前版本号自增（忽略预发布后缀）。
 * @param {string} current 形如 0.1.3
 * @param {'major'|'minor'|'patch'} type
 */
function bumpVersion(current, type) {
  const core = current.split('-')[0];
  const parts = core.split('.').map(Number);
  if (parts.length !== 3 || parts.some(Number.isNaN)) {
    console.error(`✗ 无法解析当前版本号: ${current}`);
    process.exit(1);
  }
  let [major, minor, patch] = parts;
  if (type === 'major') {
    major += 1;
    minor = 0;
    patch = 0;
  } else if (type === 'minor') {
    minor += 1;
    patch = 0;
  } else {
    patch += 1;
  }
  return `${major}.${minor}.${patch}`;
}

const arg = process.argv[2];
let newVersion;

if (!arg || BUMP_TYPES.includes(arg)) {
  // 不传参数时默认 patch 自增
  const bumpType = arg || 'patch';
  const current = readCurrentVersion();
  newVersion = bumpVersion(current, bumpType);
  console.log(`当前版本 ${current}，按 ${bumpType} 自增 -> ${newVersion}`);
} else if (VERSION_RE.test(arg)) {
  newVersion = arg;
} else {
  console.error(`✗ 非法参数: ${arg}`);
  console.error('用法: node scripts/update-version.mjs [new_version | patch | minor | major]');
  console.error('示例: node scripts/update-version.mjs 0.5.0');
  process.exit(1);
}

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


console.log('');
console.log(`版本已更新到 ${newVersion}`);
