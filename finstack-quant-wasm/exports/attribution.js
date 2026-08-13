import * as wasm from '../pkg/finstack_quant_wasm.js';

export const attribution = {
  AttributionParams: wasm.AttributionParams,
  attributePnl: wasm.attributePnl,
  attributePnlJson: wasm.attributePnlJson,
  attributePnlFromSpec: wasm.attributePnlFromSpec,
  validateAttributionJson: wasm.validateAttributionJson,
  defaultWaterfallOrder: wasm.defaultWaterfallOrder,
  defaultAttributionMetrics: wasm.defaultAttributionMetrics,
};
