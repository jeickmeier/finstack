import * as wasm from '../pkg/finstack_quant_wasm.js';

export const cashflows = {
  accruedInterest: wasm.accruedInterest,
  buildCashflowScheduleJson: wasm.buildCashflowScheduleJson,
  cdrToMdr: wasm.cdrToMdr,
  cprToSmm: wasm.cprToSmm,
  datedFlowsJson: wasm.datedFlowsJson,
  mdrToCdr: wasm.mdrToCdr,
  smmToCpr: wasm.smmToCpr,
  validateCashflowScheduleJson: wasm.validateCashflowScheduleJson,
};
