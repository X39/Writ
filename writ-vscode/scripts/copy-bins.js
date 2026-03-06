// scripts/copy-bins.js — copies cargo release binaries to bin/ for VSIX bundling.
// Called by: npm run copy-binaries (after cargo build --release)
'use strict';
const fs = require('fs');
const path = require('path');

const isWin = process.platform === 'win32';
const ext = isWin ? '.exe' : '';
const targetDir = path.join(__dirname, '..', '..', 'target', 'release');
const binDir = path.join(__dirname, '..', 'bin');

fs.mkdirSync(binDir, { recursive: true });

for (const name of ['writ-lsp', 'writ-dap']) {
    const src = path.join(targetDir, name + ext);
    const dst = path.join(binDir, name + ext);
    if (!fs.existsSync(src)) {
        console.error(`ERROR: ${src} not found. Run 'cargo build --release -p ${name}' first.`);
        process.exit(1);
    }
    fs.copyFileSync(src, dst);
    console.log(`Copied ${src} -> ${dst}`);
}
