/**
 * Merton structural-credit facade tests for `valuations.credit`.
 *
 * Covers the measure split (risk-neutral versus KMV/EDF), the three spread
 * conventions, and the day-count argument on the hazard-curve export.
 *
 * Requires the wasm-pack web build: npm run build (mise run wasm-build).
 */

import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_DIR = join(__dirname, '..', '..', 'pkg');
const WASM_BG = join(PKG_DIR, 'finstack_quant_wasm_bg.wasm');

if (!existsSync(WASM_BG)) {
  throw new Error(
    `finstack-quant-wasm web build not found at ${WASM_BG}. Generate it with: npm run build`
  );
}

const facade = await import('../../index.js');
const init = facade.default;
const { valuations } = facade;

await init({ module_or_path: readFileSync(WASM_BG) });

const credit = valuations.credit;
const modelJson = credit.mertonModelJson(100.0, 0.25, 80.0, 0.05);

test('physical-measure default probability sits below the risk-neutral one', () => {
  const riskNeutralDd = credit.mertonDistanceToDefault(modelJson, 1.0);
  assert.ok(
    Math.abs(credit.mertonDistanceToDefaultWithDrift(modelJson, 0.05, 1.0) - riskNeutralDd) < 1e-12
  );
  assert.ok(credit.mertonDistanceToDefaultWithDrift(modelJson, 0.12, 1.0) > riskNeutralDd);
  assert.ok(
    credit.mertonDefaultProbabilityWithDrift(modelJson, 0.12, 1.0) <
      credit.mertonDefaultProbability(modelJson, 1.0)
  );
});

test('kmv default point is short-term debt plus half of long-term debt', () => {
  assert.equal(credit.mertonKmvDefaultPoint(40.0, 60.0), 70.0);
  assert.throws(() => credit.mertonKmvDefaultPoint(-1.0, 60.0));
});

test('the three spread conventions are ordered as their assumptions imply', () => {
  const zeroCoupon = credit.mertonImpliedSpread(modelJson, 5.0, 0.4);
  const endogenous = credit.mertonDebtSpread(modelJson, 5.0);
  const parSpread = credit.mertonCdsParSpread(modelJson, 5.0, 0.4);
  assert.ok(endogenous > 0 && endogenous < zeroCoupon);
  assert.ok(parSpread > zeroCoupon);
});

test('cds calibration round-trips a par spread quote', () => {
  const quoteBp = credit.mertonCdsParSpread(modelJson, 5.0, 0.4) * 10_000;
  const calibrated = credit.mertonFromCdsSpreadJson(quoteBp, 0.4, 80.0, 0.05, 5.0, 100.0, 0.0);
  assert.ok(Math.abs(JSON.parse(calibrated).asset_vol - 0.25) < 1e-6);
});

test('target-pd calibration honours the payout rate', () => {
  const withoutPayout = credit.mertonFromTargetPdJson(100.0, 0.25, 0.05, 0.0, 0.05, 1.0);
  const withPayout = credit.mertonFromTargetPdJson(100.0, 0.25, 0.05, 0.03, 0.05, 1.0);
  assert.ok(JSON.parse(withPayout).debt_barrier < JSON.parse(withoutPayout).debt_barrier);
  assert.ok(Math.abs(credit.mertonDefaultProbability(withPayout, 1.0) - 0.05) < 1e-4);
});

test('hazard-curve export carries the requested day count', () => {
  const curveJson = credit.mertonToHazardCurveJson(
    modelJson,
    'ACME-HZD',
    '2024-01-15',
    [1.0, 3.0, 5.0],
    0.4,
    'act_360'
  );
  const curve = JSON.parse(curveJson);
  assert.equal(curve.id, 'ACME-HZD');
  assert.equal(curve.day_count, 'act_360');
  assert.throws(() =>
    credit.mertonToHazardCurveJson(modelJson, 'ACME-HZD', '2024-01-15', [1.0], 0.4, 'nope')
  );
});
