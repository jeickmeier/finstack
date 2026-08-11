import * as wasm from '../pkg/finstack_quant_wasm.js';

export const covenants = {
  validateCovenantSpecJson: wasm.validateCovenantSpecJson,
  validateCovenantReportJson: wasm.validateCovenantReportJson,
  validateCovenantEngineJson: wasm.validateCovenantEngineJson,
  evaluateEngine: wasm.evaluateEngine,
  lboStandardJson: wasm.lboStandardJson,
  covLiteJson: wasm.covLiteJson,
  realEstateJson: wasm.realEstateJson,
  projectFinanceJson: wasm.projectFinanceJson,
};
