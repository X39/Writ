'use strict';
// structural.test.js
// Nyquist gap-closure tests for Phase 57 (EXT-03, EXT-04).
// Uses only Node.js built-in modules (assert, fs, path). No npm install required.
// Exit 0 on full pass, non-zero on any failure.

const assert = require('assert');
const fs = require('fs');
const path = require('path');

// Resolve paths relative to this file's location (writ-vscode/tests/).
const ROOT = path.join(__dirname, '..');

let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

function check(description, fn) {
    totalTests++;
    try {
        fn();
        console.log(`  PASS  ${description}`);
        passedTests++;
    } catch (err) {
        console.error(`  FAIL  ${description}`);
        console.error(`        ${err.message}`);
        failedTests++;
    }
}

function readText(relPath) {
    const abs = path.join(ROOT, relPath);
    assert.ok(fs.existsSync(abs), `File not found: ${relPath}`);
    return fs.readFileSync(abs, 'utf8');
}

function readJson(relPath) {
    return JSON.parse(readText(relPath));
}

// ---------------------------------------------------------------------------
// Gap 1: EXT-03 — extension.ts structural checks
// ---------------------------------------------------------------------------
console.log('\n[Gap 1] EXT-03 — extension.ts bundled binary wiring');

const extSrc = readText('src/extension.ts');

check('extension.ts contains WritDebugAdapterDescriptorFactory class', () => {
    assert.ok(
        extSrc.includes('class WritDebugAdapterDescriptorFactory'),
        'Expected "class WritDebugAdapterDescriptorFactory" in extension.ts'
    );
});

check('extension.ts contains getBinaryPath function', () => {
    assert.ok(
        extSrc.includes('function getBinaryPath'),
        'Expected "function getBinaryPath" in extension.ts'
    );
});

check("extension.ts contains registerDebugAdapterDescriptorFactory('writ'", () => {
    assert.ok(
        extSrc.includes("registerDebugAdapterDescriptorFactory('writ'"),
        'Expected registerDebugAdapterDescriptorFactory(\'writ\') in extension.ts'
    );
});

check('extension.ts contains fs.existsSync for binary check', () => {
    assert.ok(
        extSrc.includes('fs.existsSync'),
        'Expected "fs.existsSync" in extension.ts'
    );
});

check("extension.ts resolves binary from path.join('bin' — not from target/debug", () => {
    assert.ok(
        extSrc.includes("path.join('bin'"),
        "Expected path.join('bin' in extension.ts"
    );
    assert.ok(
        !extSrc.includes('target/debug'),
        'extension.ts must NOT reference target/debug (old dev path)'
    );
});

check("extension.ts uses getConfiguration('writ') for serverPath override", () => {
    assert.ok(
        extSrc.includes("getConfiguration('writ')"),
        "Expected getConfiguration('writ') in extension.ts"
    );
});

// ---------------------------------------------------------------------------
// Gap 2: EXT-03 — build pipeline structural checks
// ---------------------------------------------------------------------------
console.log('\n[Gap 2] EXT-03 — build pipeline (copy-bins.js and smoke-test.sh)');

const copyBinsSrc = readText('scripts/copy-bins.js');

check('copy-bins.js references writ-lsp', () => {
    assert.ok(
        copyBinsSrc.includes('writ-lsp'),
        'Expected "writ-lsp" in copy-bins.js'
    );
});

check('copy-bins.js references writ-dap', () => {
    assert.ok(
        copyBinsSrc.includes('writ-dap'),
        'Expected "writ-dap" in copy-bins.js'
    );
});

check('copy-bins.js uses copyFileSync', () => {
    assert.ok(
        copyBinsSrc.includes('copyFileSync'),
        'Expected "copyFileSync" in copy-bins.js'
    );
});

check("copy-bins.js references target/release path", () => {
    // Accept either the string 'release' appearing inside a path.join call
    // or the full literal 'target/release'.
    const hasReleaseLiteral = copyBinsSrc.includes('target/release') ||
        copyBinsSrc.includes("'release'") ||
        copyBinsSrc.includes('"release"');
    assert.ok(
        hasReleaseLiteral,
        'Expected reference to "target/release" or \'release\' directory in copy-bins.js'
    );
});

const smokeTestSrc = readText('scripts/smoke-test.sh');

check('smoke-test.sh checks for extension/bin/writ-lsp in VSIX', () => {
    assert.ok(
        smokeTestSrc.includes('extension/bin/writ-lsp'),
        'Expected "extension/bin/writ-lsp" in smoke-test.sh'
    );
});

check('smoke-test.sh checks for extension/bin/writ-dap in VSIX', () => {
    assert.ok(
        smokeTestSrc.includes('extension/bin/writ-dap'),
        'Expected "extension/bin/writ-dap" in smoke-test.sh'
    );
});

check('smoke-test.sh checks for extension/out/extension.js in VSIX', () => {
    assert.ok(
        smokeTestSrc.includes('extension/out/extension.js'),
        'Expected "extension/out/extension.js" in smoke-test.sh'
    );
});

check('smoke-test.sh uses unzip to inspect VSIX', () => {
    assert.ok(
        smokeTestSrc.includes('unzip'),
        'Expected "unzip" command in smoke-test.sh'
    );
});

// ---------------------------------------------------------------------------
// Gap 3: EXT-04 — package.json launch.json snippet and configuration checks
// ---------------------------------------------------------------------------
console.log('\n[Gap 3] EXT-04 — package.json debuggers and configuration');

const pkg = readJson('package.json');

check('package.json contributes.debuggers[0].type === "writ"', () => {
    const debuggers = pkg.contributes && pkg.contributes.debuggers;
    assert.ok(Array.isArray(debuggers) && debuggers.length > 0, 'contributes.debuggers must be a non-empty array');
    assert.strictEqual(debuggers[0].type, 'writ', 'debuggers[0].type must be "writ"');
});

check('package.json contributes.debuggers[0].label === "Writ Debug"', () => {
    const debuggers = pkg.contributes.debuggers;
    assert.strictEqual(debuggers[0].label, 'Writ Debug', 'debuggers[0].label must be "Writ Debug"');
});

check('package.json debuggers[0].configurationSnippets[0].label === "Writ: Launch Current File"', () => {
    const snippets = pkg.contributes.debuggers[0].configurationSnippets;
    assert.ok(Array.isArray(snippets) && snippets.length > 0, 'configurationSnippets must be a non-empty array');
    assert.strictEqual(
        snippets[0].label,
        'Writ: Launch Current File',
        'configurationSnippets[0].label must be "Writ: Launch Current File"'
    );
});

check("package.json contributes.configuration.properties['writ.serverPath'] exists", () => {
    const props = pkg.contributes &&
        pkg.contributes.configuration &&
        pkg.contributes.configuration.properties;
    assert.ok(props, 'contributes.configuration.properties must exist');
    assert.ok(
        'writ.serverPath' in props,
        'contributes.configuration.properties must contain "writ.serverPath"'
    );
});

check('package.json has @vscode/vsce in devDependencies', () => {
    const devDeps = pkg.devDependencies || {};
    assert.ok(
        '@vscode/vsce' in devDeps,
        'devDependencies must contain "@vscode/vsce"'
    );
});

check('package.json has "package" script', () => {
    const scripts = pkg.scripts || {};
    assert.ok(
        'package' in scripts,
        'scripts must contain "package"'
    );
});

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
console.log(`\n${passedTests}/${totalTests} tests passed, ${failedTests} failed.\n`);

if (failedTests > 0) {
    process.exit(1);
}
process.exit(0);
