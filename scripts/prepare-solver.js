import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';

const rustInfo = execSync('rustc -vV').toString();
const target = /host: (\S+)/.exec(rustInfo)[1];
if (!target) {
    throw new Error("Could not determine rustc host target.");
}

const projectRoot = path.resolve(process.cwd());
const binaryName = 'solver';
const binaryExtension = target.includes('windows') ? '.exe' : '';
const sourcePath = path.join(projectRoot, 'solver', 'dist', `${binaryName}${binaryExtension}`);
const destDir = path.join(projectRoot, 'src-tauri', 'binaries');
const destPath = path.join(destDir, `${binaryName}-${target}${binaryExtension}`);

if (!fs.existsSync(destDir)) {
    fs.mkdirSync(destDir, { recursive: true });
}

if (!fs.existsSync(sourcePath)) {
    throw new Error(`Solver binary not found at ${sourcePath}. Please run the PyInstaller build first.`);
}

fs.copyFileSync(sourcePath, destPath);

console.log(`Successfully prepared solver binary for target ${target} at:\n${destPath}`);