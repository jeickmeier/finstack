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

test('volatility engines are exposed only under models.volatility', () => {
  assert.equal('SabrParameters' in facade.models, false);
  assert.equal('SabrModel' in facade.models, false);
  assert.equal('SabrSmile' in facade.models, false);
  assert.equal('SabrCalibrator' in facade.models, false);

  const volatility = facade.models.volatility;
  for (const name of [
    'SabrParameters',
    'SabrModel',
    'SabrSmile',
    'SabrCalibrator',
    'getCubeVol',
    'getCubeVolClamped',
    'getCubeNormalVol',
    'getCubeNormalVolClamped',
    'getFxDeltaPillarVols',
    'getFxDeltaVol',
    'deltaToStrike',
    'strikeToDelta',
  ]) {
    assert.ok(name in volatility, `missing models.volatility.${name}`);
  }
});

test('models.volatility evaluates core data artifacts', () => {
  const cube = new facade.core.VolCube(
    'USD-SWAPTION',
    [1],
    [5],
    [0.03, 0.5, -0.2, 0.4, Number.NaN],
    [0.03]
  );
  const vol = facade.models.volatility.getCubeVol(cube, 1, 5, 0.03);
  assert.ok(Number.isFinite(vol));
  assert.ok(vol > 0);

  const surface = new facade.core.FxDeltaVolSurface('EURUSD-VOL', [1], [0.12], [0.01], [0.002]);
  const pillars = facade.models.volatility.getFxDeltaPillarVols(surface, 0);
  assert.deepEqual(
    Array.from(pillars).map((value) => Number(value.toFixed(6))),
    [0.12, 0.117, 0.127]
  );

  surface.free();
  cube.free();
});
