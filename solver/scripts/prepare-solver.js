import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const solverRoot = path.resolve(scriptDir, '..');
const repoRoot = path.resolve(solverRoot, '..');
const binaryName = process.platform === 'win32' ? 'solver.exe' : 'solver';
const sourcePath = path.join(solverRoot, 'dist', binaryName);
const staleSidecarDir = path.join(repoRoot, 'build', 'sidecar');

if (!fs.existsSync(sourcePath)) {
    throw new Error(`Solver binary not found at ${sourcePath}. Please run the PyInstaller build first.`);
}

if (fs.existsSync(staleSidecarDir)) {
    fs.rmSync(staleSidecarDir, { recursive: true, force: true });
}

console.log(`Solver binary is ready at:\n${sourcePath}`);
