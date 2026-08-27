import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const testDir = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(testDir, '..', '..', 'pkg', 'finstack_quant_wasm_bg.wasm');

if (!existsSync(wasmPath)) {
  throw new Error(`WASM build not found at ${wasmPath}. Run mise run wasm-build.`);
}

const facade = await import('../../index.js');
await facade.default({ module_or_path: readFileSync(wasmPath) });

test('DTSM is exposed only under models.rates.dtsm', () => {
  assert.equal('nelsonSiegelYields' in facade.core, false);
  assert.equal(typeof facade.models.rates.dtsm.nelsonSiegelYields, 'function');

  const yields = facade.models.rates.dtsm.nelsonSiegelYields(
    0.7308,
    0.03,
    -0.01,
    0.005,
    [1, 5, 10]
  );
  assert.equal(yields.length, 3);
  assert.ok(Array.from(yields).every(Number.isFinite));
});
