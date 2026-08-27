import * as wasm from '../pkg/finstack_quant_wasm.js';

const fromMaterializationWithCache = wasm.Portfolio.fromMaterialization.bind(wasm.Portfolio);
const validateMaterializationWithCache = wasm.Portfolio.validateMaterialization.bind(
  wasm.Portfolio
);

wasm.Portfolio.fromMaterialization = (bundle, cache = undefined) => {
  if (cache !== undefined) {
    return fromMaterializationWithCache(bundle, cache);
  }
  const ephemeral = new wasm.InstrumentArtifactCache();
  try {
    return fromMaterializationWithCache(bundle, ephemeral);
  } finally {
    ephemeral.free();
  }
};

wasm.Portfolio.validateMaterialization = (bundle, cache = undefined) => {
  if (cache !== undefined) {
    return validateMaterializationWithCache(bundle, cache);
  }
  const ephemeral = new wasm.InstrumentArtifactCache();
  try {
    return validateMaterializationWithCache(bundle, ephemeral);
  } finally {
    ephemeral.free();
  }
};

export const portfolio = {
  InstrumentArtifactCache: wasm.InstrumentArtifactCache,
  Portfolio: wasm.Portfolio,
  parsePortfolioSpecJson: wasm.parsePortfolioSpecJson,
  brinsonFachler: wasm.brinsonFachler,
  carinoLink: wasm.carinoLink,
  campisiAttribution: wasm.campisiAttribution,
  // ⚠️ campisiCarinoLink links precomputed results and carries no shared
  // period_years, so it is the correct entry point for unequal-length
  // periods (e.g. act/365 months). campisiCarinoLinkFromSnapshots applies one
  // config to every period and is only correct for equal-length periods.
  campisiCarinoLink: wasm.campisiCarinoLink,
  campisiCarinoLinkFromSnapshots: wasm.campisiCarinoLinkFromSnapshots,
  campisiReconciliationCheck: wasm.campisiReconciliationCheck,
  cellReturnsFromReference: wasm.cellReturnsFromReference,
  cellReturnsFromCurves: wasm.cellReturnsFromCurves,
  excessReturns: wasm.excessReturns,
  gridAttribution: wasm.gridAttribution,
  gridCarinoLink: wasm.gridCarinoLink,
  factorBrinsonAttribution: wasm.factorBrinsonAttribution,
  twrrModifiedDietz: wasm.twrrModifiedDietz,
  twrrLinked: wasm.twrrLinked,
  mwrXirr: wasm.mwrXirr,
  buildPortfolioFromSpecJson: wasm.buildPortfolioFromSpecJson,
  portfolioResultTotalValue: wasm.portfolioResultTotalValue,
  portfolioResultGetMetric: wasm.portfolioResultGetMetric,
  aggregateMetrics: wasm.aggregateMetrics,
  valuePortfolio: wasm.valuePortfolio,
  valuePortfolioBuilt: wasm.valuePortfolioBuilt,
  aggregateFullCashflows: wasm.aggregateFullCashflows,
  aggregateFullCashflowsBuilt: wasm.aggregateFullCashflowsBuilt,
  applyScenarioAndRevalue: wasm.applyScenarioAndRevalue,
  applyScenarioAndRevalueBuilt: wasm.applyScenarioAndRevalueBuilt,
  scenarioPnl: wasm.scenarioPnl,
  scenarioPnlBuilt: wasm.scenarioPnlBuilt,
  optimizePortfolio: wasm.optimizePortfolio,
  replayPortfolio: wasm.replayPortfolio,
  // ⚠️ BLOCKING: prefer computeFactorSensitivitiesWithMarket for repeated calls
  // so large MarketContext JSON is parsed once into Market.
  computeFactorSensitivities: wasm.computeFactorSensitivities,
  computeFactorSensitivitiesWithMarket: wasm.computeFactorSensitivitiesWithMarket,
  computePnlProfiles: wasm.computePnlProfiles,
  computePnlProfilesWithMarket: wasm.computePnlProfilesWithMarket,
  // ⚠️ BLOCKING: validate sensitivity/covariance dimensions before calling;
  // malformed matrices throw instead of returning partial decompositions.
  decomposeFactorRisk: wasm.decomposeFactorRisk,
  rollEffectiveSpread: wasm.rollEffectiveSpread,
  amihudIlliquidity: wasm.amihudIlliquidity,
  daysToLiquidate: wasm.daysToLiquidate,
  liquidityTier: wasm.liquidityTier,
  lvarBangia: wasm.lvarBangia,
  almgrenChrissImpact: wasm.almgrenChrissImpact,
  kyleLambda: wasm.kyleLambda,
};
