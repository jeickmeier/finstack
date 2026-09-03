import * as wasm from '../pkg/finstack_quant_wasm.js';

export const analytics = {
  Performance: wasm.Performance,
  constrainedLeastSquares: wasm.constrainedLeastSquares,
  maxDrawdown: wasm.maxDrawdown,
  sharpe: wasm.sharpe,
  sortino: wasm.sortino,
  volatility: wasm.volatility,
};
