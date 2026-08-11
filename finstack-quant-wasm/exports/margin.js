import * as wasm from '../pkg/finstack_quant_wasm.js';

export const margin = {
  csaUsdRegulatoryJson: wasm.csaUsdRegulatoryJson,
  csaEurRegulatoryJson: wasm.csaEurRegulatoryJson,
  validateCsaJson: wasm.validateCsaJson,
  calculateVm: wasm.calculateVm,
  computeBilateralXva: wasm.computeBilateralXva,
};
