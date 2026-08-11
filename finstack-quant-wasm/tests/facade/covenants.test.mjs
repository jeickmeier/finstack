/**
 * Covenants-namespace facade smoke tests.
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
const { covenants } = facade;

await init({ module_or_path: readFileSync(WASM_BG) });

test('covenants namespace exposes JSON bridge functions', () => {
  assert.equal(typeof covenants.validateCovenantSpecJson, 'function');
  assert.equal(typeof covenants.validateCovenantReportJson, 'function');
  assert.equal(typeof covenants.validateCovenantEngineJson, 'function');
  assert.equal(typeof covenants.evaluateEngine, 'function');
  assert.equal(typeof covenants.lboStandardJson, 'function');
  assert.equal(typeof covenants.covLiteJson, 'function');
  assert.equal(typeof covenants.realEstateJson, 'function');
  assert.equal(typeof covenants.projectFinanceJson, 'function');
});

test('covenants facade generates and evaluates template JSON', () => {
  const specs = JSON.parse(covenants.lboStandardJson(5.0, 1.5, 1.2, 10_000_000.0));
  const engine = JSON.stringify({
    specs: [specs[0]],
    breach_history: [],
    windows: [],
    waivers: [],
  });
  const canonical = covenants.validateCovenantEngineJson(engine);
  const reports = covenants.evaluateEngine(
    canonical,
    JSON.stringify({ debt_to_ebitda: 4.0 }),
    '2026-03-31'
  );

  // Structured object, not a JSON string and not an ES2015 Map: property
  // reads must resolve directly.
  assert.ok(!(reports instanceof Map), 'evaluateEngine must not return an ES2015 Map');
  assert.equal(typeof reports, 'object');
  const report = reports.max_debt_ebitda;
  assert.equal(report.passed, true);
  assert.equal(typeof report.covenant_type, 'string');
  assert.equal(typeof report.actual_value, 'number');
  assert.equal(typeof report.threshold, 'number');
  assert.ok(report.meta, 'report carries the results meta stamp');

  // Round-trips through JSON without collapsing to `{}` (the ES-Map regression).
  const roundTripped = JSON.stringify(reports);
  assert.ok(roundTripped.length > 2, `evaluateEngine result must serialize: ${roundTripped}`);
  assert.equal(JSON.parse(roundTripped).max_debt_ebitda.passed, true);
});

test('covenants facade rejects unknown validation fields', () => {
  const engine = JSON.stringify({
    specs: [],
    breach_history: [],
    windows: [],
    waviers: [],
  });

  assert.throws(() => covenants.validateCovenantEngineJson(engine), /unknown field/);
});
