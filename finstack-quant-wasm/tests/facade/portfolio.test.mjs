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
const { portfolio, analytics, core } = facade;

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
  'campisiReconciliationCheck',
  'carinoLink',
  'cellReturnsFromCurves',
  'cellReturnsFromReference',
  'computeFactorSensitivities',
  'computeFactorSensitivitiesWithMarket',
  'computePnlProfiles',
  'computePnlProfilesWithMarket',
  'daysToLiquidate',
  'decomposeFactorRisk',
  'evaluateRiskBudget',
  'excessReturns',
  'factorBrinsonAttribution',
  'gridAttribution',
  'gridCarinoLink',
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

// Runtime arity gate. `index.d.ts` is hand-maintained, so a declaration with
// the wrong argument count would compile clean for a TypeScript caller while
// the extra argument is silently discarded at the JS boundary. Pinning
// Function.length here forces the real exports to stay at the arity the
// declarations promise; `dts_contract.rs` pins the declarations themselves.
test('portfolio Campisi exports keep their declared arity', () => {
  assert.equal(portfolio.campisiAttribution.length, 3);
  assert.equal(portfolio.campisiCarinoLink.length, 1);
  assert.equal(portfolio.campisiCarinoLinkFromSnapshots.length, 2);
  assert.equal(portfolio.campisiReconciliationCheck.length, 2);
});

// Same lesson as the Campisi arity gate above, applied to the credit
// excess-return / grid-attribution / factor-Brinson surfaces landed here:
// `index.d.ts` is hand-maintained, so a declaration with the wrong argument
// count would compile clean for a TypeScript caller while an extra argument
// is silently discarded at the JS boundary. `dts_contract.rs` pins the
// declared signatures themselves.
test('portfolio credit excess / grid / factor-Brinson exports keep their declared arity', () => {
  assert.equal(portfolio.cellReturnsFromReference.length, 3);
  assert.equal(portfolio.cellReturnsFromCurves.length, 6);
  assert.equal(portfolio.excessReturns.length, 2);
  assert.equal(portfolio.gridAttribution.length, 2);
  assert.equal(portfolio.gridCarinoLink.length, 1);
  assert.equal(portfolio.factorBrinsonAttribution.length, 2);
});

test('analytics.constrainedLeastSquares keeps its declared arity', () => {
  assert.equal(analytics.constrainedLeastSquares.length, 4);
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
const campisiConfig = (periodYears) => JSON.stringify({ period_years: periodYears });

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
  assert.equal('spread_mode' in result, false);
});

test('portfolio.campisiAttribution fails closed on unknown and missing config fields', () => {
  // `period_years` is the config's only field and has no default: omitting it
  // must be rejected, not guessed.
  assert.throws(() =>
    portfolio.campisiAttribution(CAMPISI_PORTFOLIO, CAMPISI_BENCHMARK, JSON.stringify({}))
  );
  // The retired `spread_mode` key is now an unknown field. A stale caller must
  // be told, not silently served a result computed without it.
  assert.throws(() =>
    portfolio.campisiAttribution(
      CAMPISI_PORTFOLIO,
      CAMPISI_BENCHMARK,
      JSON.stringify({ period_years: 0.25, spread_mode: 'dts' })
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

test('portfolio.campisiAttribution fails closed on a zero-net-weight sector', () => {
  // A long/short pair (or a CDS hedge against a cash bond) inside one bucket
  // nets to exactly zero weight. Its real contribution stays in the side
  // return while every per-sector effect would be forced to zero, so the
  // decomposition must be rejected with the offending sector named.
  const core = campisiSnapshot('CORE', 1.0, 0.015, 0.048, 5.0);
  const cleanBenchmark = JSON.stringify([campisiSnapshot('CORE', 1.0, 0.014, 0.044, 5.5)]);

  const hedgedPortfolio = JSON.stringify([
    core,
    campisiSnapshot('HEDGE', 0.5, 0.04, 0.06, 3.0),
    campisiSnapshot('HEDGE', -0.5, 0.01, 0.02, 1.0),
  ]);
  assert.throws(
    () => portfolio.campisiAttribution(hedgedPortfolio, cleanBenchmark, campisiConfig(0.25)),
    (error) => /HEDGE/.test(String(error)) && /Portfolio/.test(String(error))
  );

  const hedgedBenchmark = JSON.stringify([
    campisiSnapshot('CORE', 1.0, 0.014, 0.044, 5.5),
    campisiSnapshot('HEDGE', 0.4, 0.03, 0.055, 4.0),
    campisiSnapshot('HEDGE', -0.4, 0.005, 0.015, 1.0),
  ]);
  assert.throws(
    () =>
      portfolio.campisiAttribution(JSON.stringify([core]), hedgedBenchmark, campisiConfig(0.25)),
    (error) => /HEDGE/.test(String(error)) && /Benchmark/.test(String(error))
  );

  // A sector genuinely absent from one side is the legitimate zero-weight
  // case and must keep working.
  const oneSided = JSON.stringify([
    campisiSnapshot('CORE', 0.8, 0.015, 0.048, 5.0),
    campisiSnapshot('EXTRA', 0.2, 0.021, 0.07, 3.0),
  ]);
  const result = JSON.parse(
    portfolio.campisiAttribution(oneSided, cleanBenchmark, campisiConfig(0.25))
  );
  assert.deepEqual(
    result.sectors.map((s) => s.sector),
    ['CORE', 'EXTRA']
  );
  assert.equal(result.sectors[1].benchmark_weight, 0.0);
  const reconstructed =
    result.total_allocation +
    result.total_active_carry +
    result.total_active_treasury +
    result.total_active_spread +
    result.total_selection;
  assert.ok(Math.abs(reconstructed - result.active_return) < 1e-12);
});

test('portfolio.campisiReconciliationCheck reports the residual and fails closed', () => {
  const resultJson = portfolio.campisiAttribution(
    CAMPISI_PORTFOLIO,
    CAMPISI_BENCHMARK,
    campisiConfig(0.25)
  );
  const report = JSON.parse(portfolio.campisiReconciliationCheck(resultJson, 1e-10));
  assert.equal(report.is_reconciled, true);
  assert.equal(report.tolerance, 1e-10);
  assert.ok(Math.abs(report.total_residual) <= 1e-10);

  // The tolerance argument is load-bearing, not decorative: a result whose
  // active_return has been tampered with breaks the identity at 1e-10 and
  // passes only under an absurdly loose tolerance.
  const tampered = JSON.stringify({
    ...JSON.parse(resultJson),
    active_return: JSON.parse(resultJson).active_return + 0.01,
  });
  assert.equal(
    JSON.parse(portfolio.campisiReconciliationCheck(tampered, 1e-10)).is_reconciled,
    false
  );
  assert.equal(JSON.parse(portfolio.campisiReconciliationCheck(tampered, 1.0)).is_reconciled, true);

  // `FiAttributionResult` denies unknown fields, so a stale key fails closed
  // instead of being silently dropped into a report that still "reconciles".
  const withBogus = JSON.stringify({ ...JSON.parse(resultJson), bogus_field: 1.0 });
  assert.throws(() => portfolio.campisiReconciliationCheck(withBogus, 1e-10));
  assert.throws(() => portfolio.campisiCarinoLink(`[${withBogus}]`));
});

// ---------------------------------------------------------------------------
// Credit excess returns, hierarchical grid attribution, and factor-Brinson
// (Dynkin, Hyman & Vankudre 1998; Carino 1999; Jeet & Partani 2023).
// ---------------------------------------------------------------------------

// Lehman Brothers Fixed Income Research (1998), Figure B-1: populated
// duration cells of May 1997 Treasury returns by duration, width 0.5y.
const LEHMAN_B1_REFERENCE = JSON.stringify(
  [
    [0.25, 0.0054],
    [0.75, 0.0059],
    [1.25, 0.0066],
    [1.75, 0.0071],
    [2.25, 0.0073],
    [2.75, 0.0075],
    [3.25, 0.0078],
    [3.75, 0.0078],
    [4.25, 0.008],
    [4.75, 0.0092],
    [5.25, 0.0097],
    [5.75, 0.0103],
    [6.25, 0.0111],
    [6.75, 0.0105],
    [7.25, 0.0105],
    [9.25, 0.011],
    [9.75, 0.0111],
    [10.25, 0.0111],
    [10.75, 0.0115],
    [11.25, 0.0118],
    [11.75, 0.0121],
    [12.25, 0.0116],
  ].map(([duration, total_return]) => ({ duration, total_return }))
);
const lehmanCellConfig = JSON.stringify({ width: 0.5 });

test('portfolio.cellReturnsFromReference + excessReturns reproduce the Lehman Figure B-2 golden', () => {
  const tableJson = portfolio.cellReturnsFromReference(
    LEHMAN_B1_REFERENCE,
    'UST',
    lehmanCellConfig
  );
  const table = JSON.parse(tableJson);
  assert.equal(table.base_label, 'UST');
  assert.equal(table.cells[0].label, '0.0-0.5');

  const positions = JSON.stringify([
    { id: 'Colombia', weight: 0.2, duration: 5.16, total_return: 0.0225 },
    { id: 'RiteAid', weight: 0.2, duration: 6.71, total_return: 0.0172 },
    { id: 'NewsAM', weight: 0.2, duration: 9.58, total_return: 0.0182 },
    { id: 'Delta', weight: 0.2, duration: 9.81, total_return: 0.0102 },
    { id: 'Quebec', weight: 0.2, duration: 11.08, total_return: 0.0185 },
  ]);
  const result = JSON.parse(portfolio.excessReturns(positions, tableJson));
  assert.ok(Math.abs(result.portfolio_excess_return - 0.00648) < 1e-9);
  assert.equal(result.positions[0].cell, '5.0-5.5');
  assert.ok(
    Math.abs(
      result.portfolio_excess_return -
        (result.portfolio_total_return - result.portfolio_base_return)
    ) < 1e-9
  );
});

test('portfolio.excessReturns fails closed on a duration outside the table range, naming the position', () => {
  const tableJson = portfolio.cellReturnsFromReference(
    LEHMAN_B1_REFERENCE,
    'UST',
    lehmanCellConfig
  );
  const farPosition = JSON.stringify([
    { id: 'X', weight: 1.0, duration: 99.0, total_return: 0.01 },
  ]);
  assert.throws(() => portfolio.excessReturns(farPosition, tableJson), /X/);
});

test('portfolio.cellReturnsFromReference rejects an empty reference universe', () => {
  assert.throws(() => portfolio.cellReturnsFromReference('[]', 'UST', lehmanCellConfig));
});

// FFI-level regression for the Minor-2 finding: a units mistake (e.g. days
// instead of years) producing an astronomically large `duration` must fail
// closed with a clear error, not crash the WASM module with an unrecoverable
// panic ("capacity overflow") while allocating the duration grid.
test('portfolio.cellReturnsFromReference rejects an astronomically large duration, not a panic', () => {
  const reference = JSON.stringify([{ duration: 1e30, total_return: 0.01 }]);
  assert.throws(
    () => portfolio.cellReturnsFromReference(reference, 'UST', JSON.stringify({ width: 1.0 })),
    (error) => /sanity bound/.test(String(error))
  );
});

const flatCurveKnots = (id) =>
  new core.DiscountCurve(
    id,
    '2024-01-01',
    [0.0, 1.0, 0.25, 0.99004983, 0.5, 0.98019867, 1.25, 0.95122942, 1.5, 0.94176453]
  );

test('portfolio.cellReturnsFromCurves reproduces the flat-curve pure-carry golden', () => {
  const start = flatCurveKnots('UST');
  const end = flatCurveKnots('UST');
  const table = JSON.parse(
    portfolio.cellReturnsFromCurves(start, end, 0.25, 2.0, 'UST', JSON.stringify({ width: 1.0 }))
  );
  assert.equal(table.cells.length, 2);
  for (const cell of table.cells) {
    assert.ok(Math.abs(cell.base_return - 0.01005017) < 1e-6);
    assert.equal(cell.observed, true);
  }
});

// The mutation-catching argument-order test the task brief calls for:
// distinct start/end curves (4% -> 5%, rising rates) so swapping the two
// `DiscountCurve` arguments is not a no-op, unlike the pure-carry golden
// above whose start === end.
test('portfolio.cellReturnsFromCurves distinguishes start and end curves under rising rates', () => {
  const start = new core.DiscountCurve(
    'UST',
    '2024-01-01',
    [0.0, 1.0, 0.5, 0.98019867, 1.5, 0.94176453]
  );
  const end = new core.DiscountCurve(
    'UST',
    '2024-01-01',
    [0.0, 1.0, 0.25, 0.9875778, 1.25, 0.93941306]
  );
  const table = JSON.parse(
    portfolio.cellReturnsFromCurves(start, end, 0.25, 2.0, 'UST', JSON.stringify({ width: 1.0 }))
  );
  assert.ok(Math.abs(table.cells[0].base_return - 0.0075281983) < 1e-8);
  assert.ok(Math.abs(table.cells[1].base_return - -0.00249688) < 1e-8);
});

test('portfolio.cellReturnsFromCurves fails closed when a cell matures inside the holding period', () => {
  const knots = [0.0, 1.0, 0.25, 0.99004983, 0.5, 0.98019867];
  const start = new core.DiscountCurve('UST', '2024-01-01', knots);
  const end = new core.DiscountCurve('UST', '2024-01-01', knots);
  assert.throws(() =>
    portfolio.cellReturnsFromCurves(start, end, 0.25, 0.5, 'UST', JSON.stringify({ width: 0.25 }))
  );
});

// Hand-derived golden fixture (finstack-quant/portfolio/src/grid_attribution.rs
// `grid_attribution_matches_hand_derived_golden`): r^P=0.0293, r^B=0.0245,
// active=0.0048, curve=0.0021, sector=0.0004, selection=0.0023.
const gridPos = (cell, sector, weight, total_return) => ({ cell, sector, weight, total_return });
const GRID_PORTFOLIO = JSON.stringify([
  gridPos('0.0-3.0', 'GOVT', 0.2, 0.012),
  gridPos('0.0-3.0', 'CORP', 0.2, 0.025),
  gridPos('3.0-6.0', 'GOVT', 0.3, 0.028),
  gridPos('3.0-6.0', 'CORP', 0.3, 0.045),
]);
const GRID_BENCHMARK = JSON.stringify([
  gridPos('0.0-3.0', 'GOVT', 0.3, 0.01),
  gridPos('0.0-3.0', 'CORP', 0.2, 0.02),
  gridPos('3.0-6.0', 'GOVT', 0.25, 0.03),
  gridPos('3.0-6.0', 'CORP', 0.25, 0.04),
]);

test('portfolio.gridAttribution matches the hand-derived golden and reconciles', () => {
  const result = JSON.parse(portfolio.gridAttribution(GRID_PORTFOLIO, GRID_BENCHMARK));
  const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-12, `${a} vs ${b}`);
  close(result.portfolio_return, 0.0293);
  close(result.benchmark_return, 0.0245);
  close(result.active_return, 0.0048);
  close(result.total_curve, 0.0021);
  close(result.total_sector, 0.0004);
  close(result.total_selection, 0.0023);
  close(result.total_curve + result.total_sector + result.total_selection, result.active_return);
});

// Argument-order mutation gate: `portfolio`/`benchmark` are same-typed JSON
// arrays, and the golden above has portfolio_return != benchmark_return, so
// swapping them changes every total.
test('portfolio.gridAttribution is not invariant under swapping portfolio and benchmark', () => {
  const swapped = JSON.parse(portfolio.gridAttribution(GRID_BENCHMARK, GRID_PORTFOLIO));
  assert.notEqual(swapped.active_return, 0.0048);
  assert.ok(Math.abs(swapped.active_return - -0.0048) < 1e-12);
});

test('portfolio.gridAttribution fails closed on a zero-net-weight bucket, naming cell/sector/side', () => {
  const portfolioPositions = JSON.stringify([
    gridPos('0.0-3.0', 'GOVT', 0.5, 0.01),
    gridPos('0.0-3.0', 'GOVT', -0.5, 0.02),
    gridPos('0.0-3.0', 'CORP', 1.0, 0.03),
  ]);
  const benchmarkPositions = JSON.stringify([gridPos('0.0-3.0', 'CORP', 1.0, 0.025)]);
  assert.throws(
    () => portfolio.gridAttribution(portfolioPositions, benchmarkPositions),
    (error) =>
      /0\.0-3\.0/.test(String(error)) &&
      /GOVT/.test(String(error)) &&
      /Portfolio/.test(String(error))
  );
});

// FFI-level regression for the whole-branch review's Important-1 finding: an
// exact-equality-only guard would let a benchmark cell netting to `eps`
// (not `0.0`) through, and `weighted_return / weight` would then blow up
// without bound as `eps` shrinks. Re-pins the Rust
// `near_zero_net_weight_bucket_fails_closed_before_rate_blows_up` fixture
// through the WASM facade: `eps = 1e-8` (ratio ~1e-8, far below the 1e-6
// relative bound) must be rejected, naming the cell and the side.
test('portfolio.gridAttribution fails closed on a near-zero (not just exact-zero) net weight', () => {
  const eps = 1e-8;
  const portfolioPositions = JSON.stringify([
    gridPos('X', 'GOVT', 0.5, 0.018),
    gridPos('Y', 'GOVT', 0.5, 0.02),
  ]);
  const benchmarkPositions = JSON.stringify([
    gridPos('X', 'GOVT', 0.5, 0.02),
    gridPos('X', 'CORP', -(0.5 - eps), 0.01),
    gridPos('Y', 'GOVT', 1.0 - eps, 0.015),
  ]);
  assert.throws(
    () => portfolio.gridAttribution(portfolioPositions, benchmarkPositions),
    (error) => /bucket 'X'/.test(String(error)) && /Benchmark/.test(String(error))
  );
});

test('portfolio.gridCarinoLink reconstructs the geometrically compounded active return over two periods', () => {
  const period = portfolio.gridAttribution(GRID_PORTFOLIO, GRID_BENCHMARK);
  const linked = JSON.parse(portfolio.gridCarinoLink(`[${period},${period}]`));
  const close = (a, b, tol) => assert.ok(Math.abs(a - b) < tol, `${a} vs ${b}`);
  close(linked.portfolio_return_compounded, 0.05945849, 1e-8);
  close(linked.benchmark_return_compounded, 0.04960025, 1e-8);
  const sum = linked.linked_curve + linked.linked_sector + linked.linked_selection;
  close(sum, linked.portfolio_return_compounded - linked.benchmark_return_compounded, 1e-12);
  assert.equal(linked.periods.length, 2);
});

test('portfolio.gridCarinoLink rejects an empty period array', () => {
  assert.throws(() => portfolio.gridCarinoLink('[]'), /at least one period/);
});

// Jeet & Partani (2023) Exhibits 1-2, binary sector-indicator exposures:
// A -> Healthcare, B -> Energy, C -> Healthcare.
const FACTOR_BRINSON_INPUT = JSON.stringify({
  asset_ids: ['A', 'B', 'C'],
  asset_returns: [0.05, 0.02, 0.01],
  exposures: [0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
  factor_names: ['Energy', 'Healthcare'],
  portfolio_weights: [1.25, -0.3, 0.05],
  benchmark_weights: [0.6, 0.3, 0.1],
});
const FACTOR_BRINSON_F_B = [0.02, 0.31 / 7.0];

test('portfolio.factorBrinsonAttribution matches the binary Jeet-Partani golden', () => {
  const result = JSON.parse(
    portfolio.factorBrinsonAttribution(FACTOR_BRINSON_INPUT, FACTOR_BRINSON_F_B)
  );
  const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-12, `${a} vs ${b}`);
  close(result.active_return, 0.02);
  close(result.allocation, 0.0145714285714286);
  close(result.selection, 0.0054285714285714);
  close(result.allocation + result.selection, result.active_return);
});

test('portfolio.factorBrinsonAttribution fails closed when factor_returns do not explain the benchmark', () => {
  assert.throws(
    () => portfolio.factorBrinsonAttribution(FACTOR_BRINSON_INPUT, [0.05, 0.01]),
    /constrained_least_squares/
  );
});

// Jeet & Partani (2023) Exhibit 1 hand-derivation:
// f = (0.02 + 0.3*lambda, 0.03 + 0.35*lambda) = (0.0289552239, 0.0404477612).
test('analytics.constrainedLeastSquares matches the binary hand-derived golden', () => {
  const f = analytics.constrainedLeastSquares(
    [0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
    2,
    [0.05, 0.02, 0.01],
    [0.6, 0.3, 0.1]
  );
  assert.ok(f instanceof Float64Array);
  assert.equal(f.length, 2);
  assert.ok(Math.abs(f[0] - 0.0289552239) < 1e-9);
  assert.ok(Math.abs(f[1] - 0.0404477612) < 1e-9);
});

test('analytics.constrainedLeastSquares fails closed on orthogonal weights', () => {
  assert.throws(
    () => analytics.constrainedLeastSquares([1.0, -1.0], 1, [0.02, 0.01], [0.5, 0.5]),
    /constraint/i
  );
});

test('analytics.constrainedLeastSquares accepts Float64Array inputs, matching the plain-array golden', () => {
  const f = analytics.constrainedLeastSquares(
    Float64Array.from([0.0, 1.0, 1.0, 0.0, 0.0, 1.0]),
    2,
    Float64Array.from([0.05, 0.02, 0.01]),
    Float64Array.from([0.6, 0.3, 0.1])
  );
  assert.ok(Math.abs(f[0] - 0.0289552239) < 1e-9);
  assert.ok(Math.abs(f[1] - 0.0404477612) < 1e-9);
});

// End-to-end workflow: fit f_b with `analytics.constrainedLeastSquares`
// against benchmark weights, then feed it into
// `portfolio.factorBrinsonAttribution` and confirm the completeness
// condition holds (no rejection) on the continuous (non-binary) exposures
// fixture — the workflow the brief's own doc comments describe.
test('constrainedLeastSquares output satisfies factorBrinsonAttribution completeness', () => {
  const exposures = [1.2, -0.8, 0.5, 1.2, -0.7, 0.7];
  const returns = [0.05, 0.02, 0.01];
  const benchmarkWeights = [0.6, 0.3, 0.1];
  const fB = analytics.constrainedLeastSquares(exposures, 2, returns, benchmarkWeights);

  const input = JSON.stringify({
    asset_ids: ['A', 'B', 'C'],
    asset_returns: returns,
    exposures,
    factor_names: ['Energy', 'Healthcare'],
    portfolio_weights: [1.25, -0.3, 0.05],
    benchmark_weights: benchmarkWeights,
  });
  const result = JSON.parse(portfolio.factorBrinsonAttribution(input, Array.from(fB)));
  assert.ok(Math.abs(result.allocation - 0.00977) < 1e-4);
  assert.ok(Math.abs(result.selection - 0.010231) < 1e-4);
  assert.ok(Math.abs(result.allocation + result.selection - result.active_return) < 1e-12);
});
