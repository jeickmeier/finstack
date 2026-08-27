/** Models-liquidity namespace facade contract. */

import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const wasmBg = join(__dirname, '..', '..', 'pkg', 'finstack_quant_wasm_bg.wasm');
if (!existsSync(wasmBg)) {
  throw new Error(`WASM build not found at ${wasmBg}. Generate it with: npm run build`);
}

const facade = await import('../../index.js');
await facade.default({ module_or_path: readFileSync(wasmBg) });

const { liquidity } = facade.models;
const expected = [
  'almgrenChrissImpact',
  'amihudIlliquidity',
  'daysToLiquidate',
  'kyleLambda',
  'liquidityTier',
  'lvarBangia',
  'rollEffectiveSpread',
];

function structured(value, label) {
  assert.equal(typeof value, 'object', `${label} must be structured`);
  assert.notEqual(value, null);
  assert.ok(JSON.stringify(value).length > 2);
  return value;
}

test('models.liquidity exposes exactly the moved API', () => {
  assert.deepEqual(Object.keys(liquidity).sort(), expected);
  assert.equal(liquidity.kyleLambda.length, 3);
  for (const key of expected) assert.equal(typeof liquidity[key], 'function');
});

test('portfolio retains no liquidity compatibility exports', () => {
  for (const key of expected) assert.equal(key in facade.portfolio, false);
});

test('models.liquidity preserves estimator and risk results', () => {
  assert.equal(liquidity.rollEffectiveSpread('[0.01,-0.01,0.01,-0.01]'), 0.02);
  assert.equal(liquidity.daysToLiquidate(1_000_000, 250_000, 0.2), 20);
  assert.equal(liquidity.liquidityTier(3), 'tier2');
  assert.equal(liquidity.kyleLambda('[100,200]', '[0.01,-0.02]', 50), 0.005);

  const lvar = structured(
    liquidity.lvarBangia(-100_000, 0.002, 0.0005, 0.99, 1_000_000),
    'lvarBangia result'
  );
  assert.deepEqual(Object.keys(lvar).sort(), ['lvar', 'lvar_ratio', 'spread_cost', 'var']);
  assert.ok(lvar.lvar <= lvar.var);

  const impact = structured(
    liquidity.almgrenChrissImpact(10_000, 1_000_000, 0.02, 1, 0, 0.01, 100),
    'almgrenChrissImpact result'
  );
  assert.deepEqual(Object.keys(impact).sort(), [
    'execution_risk',
    'expected_cost_bp',
    'permanent_impact',
    'temporary_impact',
    'total_impact',
  ]);
});
