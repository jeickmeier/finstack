/**
 * Portfolio-namespace facade runtime contract test.
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
const { portfolio } = facade;

await init({ module_or_path: readFileSync(WASM_BG) });

const EXPORTED_KEYS = [
  'Portfolio',
  'aggregateFullCashflows',
  'aggregateFullCashflowsBuilt',
  'aggregateMetrics',
  'almgrenChrissImpact',
  'amihudIlliquidity',
  'applyScenarioAndRevalue',
  'applyScenarioAndRevalueBuilt',
  'brinsonFachler',
  'buildPortfolioFromSpec',
  'campisiAttribution',
  'campisiCarinoLink',
  'campisiCarinoLinkFromSnapshots',
  'carinoLink',
  'computeFactorSensitivities',
  'computeFactorSensitivitiesWithMarket',
  'computePnlProfiles',
  'computePnlProfilesWithMarket',
  'daysToLiquidate',
  'decomposeFactorRisk',
  'evaluateRiskBudget',
  'historicalVarDecomposition',
  'kyleLambda',
  'liquidityTier',
  'lvarBangia',
  'mwrXirr',
  'optimizePortfolio',
  'parametricEsDecomposition',
  'parametricVarDecomposition',
  'parsePortfolioSpec',
  'portfolioResultGetMetric',
  'portfolioResultTotalValue',
  'replayPortfolio',
  'rollEffectiveSpread',
  'scenarioPnl',
  'scenarioPnlBuilt',
  'twrrLinked',
  'twrrModifiedDietz',
  'valuePortfolio',
  'valuePortfolioBuilt',
];

test('portfolio namespace exposes exactly the pinned contract surface', () => {
  assert.deepEqual(Object.keys(portfolio).sort(), EXPORTED_KEYS);
  for (const key of EXPORTED_KEYS) {
    assert.equal(
      typeof portfolio[key],
      'function',
      `portfolio.${key} must be a function (got ${typeof portfolio[key]})`
    );
  }
});

const campisiSnapshot = (sector, weight, totalReturn, yieldAnnual, modifiedDuration) => ({
  sector,
  weight,
  total_return: totalReturn,
  yield_annual: yieldAnnual,
  modified_duration: modifiedDuration,
  spread_duration: 0.0,
  spread: 0.0,
  delta_treasury_yield: -0.001,
  delta_spread: 0.0,
});

const CAMPISI_PORTFOLIO = JSON.stringify([
  campisiSnapshot('GOVT', 0.6, 0.016, 0.04, 5.0),
  campisiSnapshot('CORP', 0.4, 0.012, 0.06, 4.0),
]);
const CAMPISI_BENCHMARK = JSON.stringify([
  campisiSnapshot('GOVT', 0.5, 0.015, 0.04, 5.5),
  campisiSnapshot('CORP', 0.5, 0.011, 0.055, 4.5),
]);
const campisiConfig = (periodYears) =>
  JSON.stringify({ period_years: periodYears, spread_mode: 'spread_duration' });

test('portfolio.campisiAttribution reconciles the five effects to active return', () => {
  const result = JSON.parse(
    portfolio.campisiAttribution(CAMPISI_PORTFOLIO, CAMPISI_BENCHMARK, campisiConfig(0.25))
  );
  const reconstructed =
    result.total_allocation +
    result.total_active_carry +
    result.total_active_treasury +
    result.total_active_spread +
    result.total_selection;
  assert.ok(Math.abs(reconstructed - result.active_return) < 1e-12);
  assert.equal(result.spread_mode, 'spread_duration');
});

test('portfolio.campisiAttribution fails closed on a non-canonical spread_mode', () => {
  assert.throws(() =>
    portfolio.campisiAttribution(
      CAMPISI_PORTFOLIO,
      CAMPISI_BENCHMARK,
      JSON.stringify({ period_years: 0.25, spread_mode: 'SpreadDuration' })
    )
  );
  // `spread_mode` has no default: omitting it must be rejected, not guessed.
  assert.throws(() =>
    portfolio.campisiAttribution(
      CAMPISI_PORTFOLIO,
      CAMPISI_BENCHMARK,
      JSON.stringify({ period_years: 0.25 })
    )
  );
});

test('portfolio.campisiCarinoLink links precomputed periods of unequal length', () => {
  // 31/365 and 28/365 — an act/365 monthly pair the snapshot-based entry
  // point cannot express, because it applies one shared period_years.
  const jan = portfolio.campisiAttribution(
    CAMPISI_PORTFOLIO,
    CAMPISI_BENCHMARK,
    campisiConfig(31 / 365)
  );
  const feb = portfolio.campisiAttribution(
    CAMPISI_PORTFOLIO,
    CAMPISI_BENCHMARK,
    campisiConfig(28 / 365)
  );
  assert.notEqual(JSON.parse(jan).total_active_carry, JSON.parse(feb).total_active_carry);

  const linked = JSON.parse(portfolio.campisiCarinoLink(`[${jan},${feb}]`));
  const geometric = linked.portfolio_return_compounded - linked.benchmark_return_compounded;
  const reconstructed =
    linked.linked_allocation +
    linked.linked_active_carry +
    linked.linked_active_treasury +
    linked.linked_active_spread +
    linked.linked_selection;
  assert.ok(Math.abs(reconstructed - geometric) < 1e-10);
  assert.equal(linked.periods.length, 2);
});

test('portfolio.campisiCarinoLinkFromSnapshots links raw snapshot periods', () => {
  const period = {
    portfolio: JSON.parse(CAMPISI_PORTFOLIO),
    benchmark: JSON.parse(CAMPISI_BENCHMARK),
  };
  const linked = JSON.parse(
    portfolio.campisiCarinoLinkFromSnapshots(JSON.stringify([period, period]), campisiConfig(0.25))
  );
  const geometric = linked.portfolio_return_compounded - linked.benchmark_return_compounded;
  const reconstructed =
    linked.linked_allocation +
    linked.linked_active_carry +
    linked.linked_active_treasury +
    linked.linked_active_spread +
    linked.linked_selection;
  assert.ok(Math.abs(reconstructed - geometric) < 1e-10);

  // The two link entry points take different payloads and are not aliases.
  assert.throws(() => portfolio.campisiCarinoLink(JSON.stringify([period])));
  assert.throws(() =>
    portfolio.campisiCarinoLinkFromSnapshots(
      `[${portfolio.campisiAttribution(CAMPISI_PORTFOLIO, CAMPISI_BENCHMARK, campisiConfig(0.25))}]`,
      campisiConfig(0.25)
    )
  );
});
