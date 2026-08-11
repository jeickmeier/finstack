import * as wasm from '../pkg/finstack_quant_wasm.js';

export const cashflows = {
  accruedInterest: wasm.accruedInterest,
  buildCashflowScheduleJson: wasm.buildCashflowScheduleJson,
  datedFlowsJson: wasm.datedFlowsJson,
  validateCashflowScheduleJson: wasm.validateCashflowScheduleJson,
};
