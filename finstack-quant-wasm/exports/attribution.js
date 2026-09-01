import * as wasm from '../pkg/finstack_quant_wasm.js';

export const attribution = {
  AttributionParams: wasm.AttributionParams,
  attributePnl: wasm.attributePnl,
  attributePnlJson: wasm.attributePnlJson,
  attributePnlEnvelopeJson: wasm.attributePnlEnvelopeJson,
  validateAttributionJson: wasm.validateAttributionJson,
  defaultWaterfallOrder: wasm.defaultWaterfallOrder,
  defaultAttributionMetrics: wasm.defaultAttributionMetrics,
};
