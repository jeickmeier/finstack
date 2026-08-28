import * as wasm from '../pkg/finstack_quant_wasm.js';

const jsonInput = (value) => (typeof value === 'string' ? value : JSON.stringify(value));

/** Quote ingestion, market construction, and explicit model calibration. */
export const calibration = {
  calibrate: (envelope) => wasm.calibrate(jsonInput(envelope)),
  validateCalibrationJson: (envelope) => wasm.validateCalibrationJson(jsonInput(envelope)),
  dryRun: (envelope) => wasm.dryRun(jsonInput(envelope)),
  dependencyGraphJson: (envelope) => wasm.dependencyGraphJson(jsonInput(envelope)),
  calibrateBermudanLmmBaseVol: (instrument, market, asOf) =>
    wasm.calibrateBermudanLmmBaseVol(jsonInput(instrument), market, asOf),
};
