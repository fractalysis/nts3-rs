import { copyFile, mkdir, stat } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const workspace = path.resolve(here, '..');
const website = path.resolve(workspace, '..', '..');
const outputDirectory = path.join(website, 'assets');
const target = 'wasm32-unknown-unknown';
const effects = [
  { package: 'smooth-echo-nts3-plug', library: 'smooth_echo', output: 'nts3-smooth-echo.wasm' },
  { package: 'diopser-nts3-plug', library: 'diopser_nts3', output: 'nts3-diopser.wasm' },
  { package: 'ott-nts3-plug', library: 'ott_nts3', output: 'nts3-ott.wasm' },
];

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: workspace, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`)));
  });
}

await mkdir(outputDirectory, { recursive: true });
for (const effect of effects) {
  await run('cargo', [
    'build',
    '--release',
    '--target', target,
    '--package', effect.package,
    '--features', 'web',
    '--config', 'profile.release.strip="symbols"',
  ]);
  const source = path.join(workspace, 'target', target, 'release', `${effect.library}.wasm`);
  const destination = path.join(outputDirectory, effect.output);
  await copyFile(source, destination);
  const { size } = await stat(destination);
  console.log(`wrote ${path.relative(website, destination)} (${size.toLocaleString()} bytes)`);
}
